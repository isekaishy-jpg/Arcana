use std::collections::BTreeMap;

pub use arcana_syntax::{
    SurfaceLifetime as HirLifetime, SurfacePath as HirPath, SurfacePredicate as HirPredicate,
    SurfaceProjection as HirProjection, SurfaceRefs as HirSurfaceRefs,
    SurfaceTraitRef as HirTraitRef, SurfaceType as HirType, SurfaceTypeKind as HirTypeKind,
    SurfaceWhereClause as HirWhereClause, collect_surface_type_refs as collect_hir_type_refs,
    collect_surface_where_clause_refs as collect_hir_where_clause_refs,
    parse_surface_type as parse_hir_type, surface_type_is_boundary_safe as hir_type_is_boundary_safe,
    validate_tuple_type_contract as validate_hir_tuple_contract,
};

pub fn render_hir_type(ty: &HirType) -> String {
    ty.render()
}

pub fn render_hir_trait_ref(trait_ref: &HirTraitRef) -> String {
    trait_ref.render()
}

pub fn render_hir_where_clause(where_clause: &HirWhereClause) -> String {
    where_clause.render()
}

pub fn hir_type_base_path(ty: &HirType) -> Option<Vec<String>> {
    match &ty.kind {
        HirTypeKind::Path(path) | HirTypeKind::Apply { base: path, .. } => Some(path.segments.clone()),
        HirTypeKind::Projection(projection) => Some(projection.trait_ref.path.segments.clone()),
        HirTypeKind::Ref { inner, .. } => hir_type_base_path(inner),
        HirTypeKind::Tuple(_) => None,
    }
}

pub fn hir_strip_reference_type<'a>(ty: &'a HirType) -> &'a HirType {
    match &ty.kind {
        HirTypeKind::Ref { inner, .. } => hir_strip_reference_type(inner),
        _ => ty,
    }
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
            lifetime,
            mutable,
            inner,
        } => HirType {
            kind: HirTypeKind::Ref {
                lifetime: lifetime.clone(),
                mutable: *mutable,
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
    if let HirTypeKind::Path(path) = &declared.kind {
        if path.segments.len() == 1 && hir_simple_placeholder(&path.segments[0]) {
            let Some(key) = bindings.binding_id(&path.segments[0]) else {
                return false;
            };
            if let Some(existing) = substitutions.get(&key) {
                return existing == actual;
            }
            substitutions.insert(key, actual.clone());
            return true;
        }
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
