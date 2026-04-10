use std::{collections::BTreeMap, fmt};

use arcana_syntax::{self, Span};

use crate::HirParamMode;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirSurfaceRefs {
    pub paths: Vec<Vec<String>>,
    pub lifetimes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirPath {
    pub segments: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirLifetime {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirTraitRef {
    pub path: HirPath,
    pub args: Vec<HirType>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirProjection {
    pub trait_ref: HirTraitRef,
    pub assoc: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirType {
    pub kind: HirTypeKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirTypeKind {
    Path(HirPath),
    Apply {
        base: HirPath,
        args: Vec<HirType>,
    },
    Ref {
        mode: HirParamMode,
        lifetime: Option<HirLifetime>,
        inner: Box<HirType>,
    },
    Tuple(Vec<HirType>),
    Projection(HirProjection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirWhereClause {
    pub predicates: Vec<HirPredicate>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirPredicate {
    TraitBound {
        trait_ref: HirTraitRef,
        span: Span,
    },
    ProjectionEq {
        projection: HirProjection,
        value: HirType,
        span: Span,
    },
    LifetimeOutlives {
        longer: HirLifetime,
        shorter: HirLifetime,
        span: Span,
    },
    TypeOutlives {
        ty: HirType,
        lifetime: HirLifetime,
        span: Span,
    },
}

impl HirPath {
    pub fn render(&self) -> String {
        self.segments.join(".")
    }

    pub fn collect_refs(&self, refs: &mut HirSurfaceRefs) {
        refs.paths.push(self.segments.clone());
    }
}

impl HirLifetime {
    pub fn render(&self) -> String {
        self.name.clone()
    }

    pub fn collect_refs(&self, refs: &mut HirSurfaceRefs) {
        refs.lifetimes.push(self.name.clone());
    }
}

impl HirTraitRef {
    pub fn render(&self) -> String {
        if self.args.is_empty() {
            self.path.render()
        } else {
            format!(
                "{}[{}]",
                self.path.render(),
                self.args
                    .iter()
                    .map(HirType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    pub fn collect_refs(&self, refs: &mut HirSurfaceRefs) {
        self.path.collect_refs(refs);
        for arg in &self.args {
            arg.collect_refs(refs);
        }
    }
}

impl HirProjection {
    pub fn render(&self) -> String {
        format!("{}.{}", self.trait_ref.render(), self.assoc)
    }

    pub fn collect_refs(&self, refs: &mut HirSurfaceRefs) {
        self.trait_ref.collect_refs(refs);
    }
}

impl HirType {
    pub fn render(&self) -> String {
        match &self.kind {
            HirTypeKind::Path(path) => path.render(),
            HirTypeKind::Apply { base, args } => format!(
                "{}[{}]",
                base.render(),
                args.iter()
                    .map(HirType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            HirTypeKind::Ref {
                mode,
                lifetime,
                inner,
            } => {
                let mut args = vec![inner.render()];
                if let Some(lifetime) = lifetime {
                    args.push(lifetime.render());
                }
                format!("&{}[{}]", mode.as_str(), args.join(", "))
            }
            HirTypeKind::Tuple(items) => format!(
                "({})",
                items
                    .iter()
                    .map(HirType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            HirTypeKind::Projection(projection) => projection.render(),
        }
    }

    pub fn collect_refs(&self, refs: &mut HirSurfaceRefs) {
        match &self.kind {
            HirTypeKind::Path(path) => path.collect_refs(refs),
            HirTypeKind::Apply { base, args } => {
                base.collect_refs(refs);
                for arg in args {
                    arg.collect_refs(refs);
                }
            }
            HirTypeKind::Ref {
                lifetime, inner, ..
            } => {
                if let Some(lifetime) = lifetime {
                    lifetime.collect_refs(refs);
                }
                inner.collect_refs(refs);
            }
            HirTypeKind::Tuple(items) => {
                for item in items {
                    item.collect_refs(refs);
                }
            }
            HirTypeKind::Projection(projection) => projection.collect_refs(refs),
        }
    }

    pub fn refs(&self) -> HirSurfaceRefs {
        let mut refs = HirSurfaceRefs::default();
        self.collect_refs(&mut refs);
        refs
    }
}

impl HirWhereClause {
    pub fn render(&self) -> String {
        self.predicates
            .iter()
            .map(HirPredicate::render)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn collect_refs(&self, refs: &mut HirSurfaceRefs) {
        for predicate in &self.predicates {
            predicate.collect_refs(refs);
        }
    }

    pub fn refs(&self) -> HirSurfaceRefs {
        let mut refs = HirSurfaceRefs::default();
        self.collect_refs(&mut refs);
        refs
    }
}

impl HirPredicate {
    pub fn render(&self) -> String {
        match self {
            Self::TraitBound { trait_ref, .. } => trait_ref.render(),
            Self::ProjectionEq {
                projection, value, ..
            } => format!("{} = {}", projection.render(), value.render()),
            Self::LifetimeOutlives {
                longer, shorter, ..
            } => format!("{}: {}", longer.render(), shorter.render()),
            Self::TypeOutlives { ty, lifetime, .. } => {
                format!("{}: {}", ty.render(), lifetime.render())
            }
        }
    }

    pub fn collect_refs(&self, refs: &mut HirSurfaceRefs) {
        match self {
            Self::TraitBound { trait_ref, .. } => trait_ref.collect_refs(refs),
            Self::ProjectionEq {
                projection, value, ..
            } => {
                projection.collect_refs(refs);
                value.collect_refs(refs);
            }
            Self::LifetimeOutlives {
                longer, shorter, ..
            } => {
                longer.collect_refs(refs);
                shorter.collect_refs(refs);
            }
            Self::TypeOutlives { ty, lifetime, .. } => {
                ty.collect_refs(refs);
                lifetime.collect_refs(refs);
            }
        }
    }
}

impl fmt::Display for HirPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for HirLifetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for HirTraitRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for HirProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for HirType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for HirWhereClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for HirPredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

pub(crate) fn lower_surface_path(path: &arcana_syntax::SurfacePath) -> HirPath {
    HirPath {
        segments: path.segments.clone(),
        span: path.span,
    }
}

pub(crate) fn lower_surface_lifetime(lifetime: &arcana_syntax::SurfaceLifetime) -> HirLifetime {
    HirLifetime {
        name: lifetime.name.clone(),
        span: lifetime.span,
    }
}

pub(crate) fn lower_surface_trait_ref(trait_ref: &arcana_syntax::SurfaceTraitRef) -> HirTraitRef {
    HirTraitRef {
        path: lower_surface_path(&trait_ref.path),
        args: trait_ref.args.iter().map(lower_surface_type).collect(),
        span: trait_ref.span,
    }
}

pub(crate) fn lower_surface_projection(
    projection: &arcana_syntax::SurfaceProjection,
) -> HirProjection {
    HirProjection {
        trait_ref: lower_surface_trait_ref(&projection.trait_ref),
        assoc: projection.assoc.clone(),
        span: projection.span,
    }
}

pub(crate) fn lower_surface_type(ty: &arcana_syntax::SurfaceType) -> HirType {
    HirType {
        kind: match &ty.kind {
            arcana_syntax::SurfaceTypeKind::Path(path) => {
                HirTypeKind::Path(lower_surface_path(path))
            }
            arcana_syntax::SurfaceTypeKind::Apply { base, args } => HirTypeKind::Apply {
                base: lower_surface_path(base),
                args: args.iter().map(lower_surface_type).collect(),
            },
            arcana_syntax::SurfaceTypeKind::Ref {
                mode,
                lifetime,
                inner,
            } => HirTypeKind::Ref {
                mode: lower_hir_param_mode(*mode),
                lifetime: lifetime.as_ref().map(lower_surface_lifetime),
                inner: Box::new(lower_surface_type(inner)),
            },
            arcana_syntax::SurfaceTypeKind::Tuple(items) => {
                HirTypeKind::Tuple(items.iter().map(lower_surface_type).collect())
            }
            arcana_syntax::SurfaceTypeKind::Projection(projection) => {
                HirTypeKind::Projection(lower_surface_projection(projection))
            }
        },
        span: ty.span,
    }
}

pub(crate) fn lower_surface_where_clause(
    where_clause: &arcana_syntax::SurfaceWhereClause,
) -> HirWhereClause {
    HirWhereClause {
        predicates: where_clause
            .predicates
            .iter()
            .map(lower_surface_predicate)
            .collect(),
        span: where_clause.span,
    }
}

pub(crate) fn lower_surface_predicate(predicate: &arcana_syntax::SurfacePredicate) -> HirPredicate {
    match predicate {
        arcana_syntax::SurfacePredicate::TraitBound { trait_ref, span } => {
            HirPredicate::TraitBound {
                trait_ref: lower_surface_trait_ref(trait_ref),
                span: *span,
            }
        }
        arcana_syntax::SurfacePredicate::ProjectionEq {
            projection,
            value,
            span,
        } => HirPredicate::ProjectionEq {
            projection: lower_surface_projection(projection),
            value: lower_surface_type(value),
            span: *span,
        },
        arcana_syntax::SurfacePredicate::LifetimeOutlives {
            longer,
            shorter,
            span,
        } => HirPredicate::LifetimeOutlives {
            longer: lower_surface_lifetime(longer),
            shorter: lower_surface_lifetime(shorter),
            span: *span,
        },
        arcana_syntax::SurfacePredicate::TypeOutlives { ty, lifetime, span } => {
            HirPredicate::TypeOutlives {
                ty: lower_surface_type(ty),
                lifetime: lower_surface_lifetime(lifetime),
                span: *span,
            }
        }
    }
}

pub fn parse_hir_type(text: &str) -> Result<HirType, String> {
    arcana_syntax::parse_surface_type(text).map(|ty| lower_surface_type(&ty))
}

pub fn parse_hir_where_clause(text: &str) -> Result<HirWhereClause, String> {
    arcana_syntax::parse_surface_where_clause(text)
        .map(|where_clause| lower_surface_where_clause(&where_clause))
}

pub fn render_hir_type(ty: &HirType) -> String {
    ty.render()
}

pub fn render_hir_trait_ref(trait_ref: &HirTraitRef) -> String {
    trait_ref.render()
}

pub fn render_hir_where_clause(where_clause: &HirWhereClause) -> String {
    where_clause.render()
}

pub fn collect_hir_type_refs(ty: &HirType) -> HirSurfaceRefs {
    ty.refs()
}

pub fn collect_hir_where_clause_refs(where_clause: &HirWhereClause) -> HirSurfaceRefs {
    where_clause.refs()
}

pub fn hir_type_is_boundary_safe(ty: &HirType) -> bool {
    let refs = ty.refs();
    !refs.paths.into_iter().any(|path| {
        path.last()
            .is_some_and(|name| arcana_syntax::is_builtin_boundary_unsafe_type_name(name))
    })
}

pub fn validate_hir_tuple_contract(ty: &HirType, span: Span, context: &str) -> Result<(), String> {
    validate_hir_tuple_contract_inner(ty, span, context)
}

fn validate_hir_tuple_contract_inner(
    ty: &HirType,
    span: Span,
    context: &str,
) -> Result<(), String> {
    match &ty.kind {
        HirTypeKind::Tuple(items) => {
            if items.len() != 2 {
                return Err(format!(
                    "{}:{}: {context} tuples are not part of v1 except pairs",
                    span.line, span.column
                ));
            }
            for item in items {
                validate_hir_tuple_contract_inner(item, span, context)?;
            }
        }
        HirTypeKind::Apply { args, .. } => {
            for arg in args {
                validate_hir_tuple_contract_inner(arg, span, context)?;
            }
        }
        HirTypeKind::Ref { inner, .. } => {
            validate_hir_tuple_contract_inner(inner, span, context)?;
        }
        HirTypeKind::Projection(_) | HirTypeKind::Path(_) => {}
    }
    Ok(())
}

pub fn hir_type_base_path(ty: &HirType) -> Option<Vec<String>> {
    match &ty.kind {
        HirTypeKind::Path(path) | HirTypeKind::Apply { base: path, .. } => {
            Some(path.segments.clone())
        }
        HirTypeKind::Projection(projection) => Some(projection.trait_ref.path.segments.clone()),
        HirTypeKind::Ref { inner, .. } => hir_type_base_path(inner),
        HirTypeKind::Tuple(_) => None,
    }
}

pub fn hir_strip_reference_type(mut ty: &HirType) -> &HirType {
    while let HirTypeKind::Ref { inner, .. } = &ty.kind {
        ty = inner;
    }
    ty
}

pub fn hir_type_app_args(ty: &HirType) -> Option<&[HirType]> {
    match &ty.kind {
        HirTypeKind::Apply { args, .. } => Some(args),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HirTypeBindingId(pub u32);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirTypeBindingScope {
    bindings: BTreeMap<String, HirTypeBindingId>,
    next_id: u32,
}

impl HirTypeBindingScope {
    pub fn insert(&mut self, name: impl Into<String>) -> HirTypeBindingId {
        let name = name.into();
        if let Some(existing) = self.bindings.get(&name) {
            return *existing;
        }
        let id = HirTypeBindingId(self.next_id);
        self.next_id += 1;
        self.bindings.insert(name, id);
        id
    }

    pub fn from_names<I>(names: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut scope = Self::default();
        for name in names {
            scope.insert(name);
        }
        scope
    }

    pub fn binding_id(&self, name: &str) -> Option<HirTypeBindingId> {
        self.bindings.get(name).copied()
    }
}

pub type HirTypeSubstitutions = BTreeMap<HirTypeBindingId, HirType>;

pub fn substitute_hir_type(
    ty: &HirType,
    bindings: &HirTypeBindingScope,
    substitutions: &HirTypeSubstitutions,
) -> HirType {
    match &ty.kind {
        HirTypeKind::Path(path) if path.segments.len() == 1 => bindings
            .binding_id(&path.segments[0])
            .and_then(|id| substitutions.get(&id))
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        HirTypeKind::Path(_) => ty.clone(),
        HirTypeKind::Apply { base, args } => HirType {
            kind: HirTypeKind::Apply {
                base: base.clone(),
                args: args
                    .iter()
                    .map(|arg| substitute_hir_type(arg, bindings, substitutions))
                    .collect(),
            },
            span: ty.span,
        },
        HirTypeKind::Ref {
            mode,
            lifetime,
            inner,
        } => HirType {
            kind: HirTypeKind::Ref {
                mode: *mode,
                lifetime: lifetime.clone(),
                inner: Box::new(substitute_hir_type(inner, bindings, substitutions)),
            },
            span: ty.span,
        },
        HirTypeKind::Tuple(items) => HirType {
            kind: HirTypeKind::Tuple(
                items
                    .iter()
                    .map(|item| substitute_hir_type(item, bindings, substitutions))
                    .collect(),
            ),
            span: ty.span,
        },
        HirTypeKind::Projection(projection) => HirType {
            kind: HirTypeKind::Projection(HirProjection {
                trait_ref: HirTraitRef {
                    path: projection.trait_ref.path.clone(),
                    args: projection
                        .trait_ref
                        .args
                        .iter()
                        .map(|arg| substitute_hir_type(arg, bindings, substitutions))
                        .collect(),
                    span: projection.trait_ref.span,
                },
                assoc: projection.assoc.clone(),
                span: projection.span,
            }),
            span: ty.span,
        },
    }
}

fn lower_hir_param_mode(mode: arcana_syntax::ParamMode) -> HirParamMode {
    match mode {
        arcana_syntax::ParamMode::Read => HirParamMode::Read,
        arcana_syntax::ParamMode::Edit => HirParamMode::Edit,
        arcana_syntax::ParamMode::Take => HirParamMode::Take,
        arcana_syntax::ParamMode::Hold => HirParamMode::Hold,
    }
}

pub fn hir_type_matches(
    declared: &HirType,
    actual: &HirType,
    bindings: &HirTypeBindingScope,
    substitutions: &mut HirTypeSubstitutions,
) -> bool {
    let declared = hir_strip_reference_type(declared);
    let actual = hir_strip_reference_type(actual);
    if declared == actual {
        return true;
    }
    if let HirTypeKind::Path(path) = &declared.kind
        && path.segments.len() == 1
        && hir_simple_placeholder(&path.segments[0])
    {
        let Some(key) = bindings.binding_id(&path.segments[0]) else {
            return false;
        };
        if let Some(existing) = substitutions.get(&key) {
            return existing == actual;
        }
        substitutions.insert(key, actual.clone());
        return true;
    }
    match (&declared.kind, &actual.kind) {
        (
            HirTypeKind::Apply {
                base: declared_base,
                args: declared_args,
            },
            HirTypeKind::Apply {
                base: actual_base,
                args: actual_args,
            },
        ) => {
            declared_base.segments == actual_base.segments
                && declared_args.len() == actual_args.len()
                && declared_args
                    .iter()
                    .zip(actual_args.iter())
                    .all(|(declared_arg, actual_arg)| {
                        hir_type_matches(declared_arg, actual_arg, bindings, substitutions)
                    })
        }
        (HirTypeKind::Path(declared_path), HirTypeKind::Path(actual_path)) => {
            declared_path.segments == actual_path.segments
        }
        _ => false,
    }
}

fn hir_simple_placeholder(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        && arcana_syntax::builtin_type_info(text).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ref_projection_and_where_clause() {
        let ty = parse_hir_type("&edit[std.iter.Iterator[I].Item, 'a]").expect("type");
        assert_eq!(ty.render(), "&edit[std.iter.Iterator[I].Item, 'a]");
        let refs = ty.refs();
        assert!(refs.paths.iter().any(|path| path
            == &[
                "std".to_string(),
                "iter".to_string(),
                "Iterator".to_string()
            ]));
        assert!(refs.lifetimes.iter().any(|lifetime| lifetime == "'a"));

        let where_clause =
            parse_hir_where_clause("Iterator[I], Iterator[I].Item = U, U: 'a").expect("where");
        assert_eq!(
            where_clause.render(),
            "Iterator[I], Iterator[I].Item = U, U: 'a"
        );
    }

    #[test]
    fn tuple_contract_rejects_non_pair() {
        let ty = parse_hir_type("(Int, Bool, Str)").expect("tuple");
        let err = validate_hir_tuple_contract(&ty, Span::new(1, 1), "test type")
            .expect_err("tuple contract should fail");
        assert!(err.contains("pairs"));
    }

    #[test]
    fn boundary_safe_checks_builtin_tokens() {
        let safe = parse_hir_type("List[Int]").expect("safe");
        let unsafe_ty = parse_hir_type("Task[Int]").expect("unsafe");
        assert!(hir_type_is_boundary_safe(&safe));
        assert!(!hir_type_is_boundary_safe(&unsafe_ty));
        assert!(arcana_syntax::builtin_type_info("Task").is_some());
    }

    #[test]
    fn lowers_surface_nodes_into_owned_hir_nodes() {
        let parsed = arcana_syntax::parse_surface_where_clause("Iterator[I].Item = U")
            .expect("surface where clause should parse");
        let lowered = lower_surface_where_clause(&parsed);
        assert_eq!(lowered.render(), "Iterator[I].Item = U");
        assert!(matches!(
            lowered.predicates.as_slice(),
            [HirPredicate::ProjectionEq { .. }]
        ));
    }
}
