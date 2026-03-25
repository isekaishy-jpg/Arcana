use std::fmt;

use crate::{Span, is_builtin_boundary_unsafe_type_name};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SurfaceRefs {
    pub paths: Vec<Vec<String>>,
    pub lifetimes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfacePath {
    pub segments: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceLifetime {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceTraitRef {
    pub path: SurfacePath,
    pub args: Vec<SurfaceType>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceProjection {
    pub trait_ref: SurfaceTraitRef,
    pub assoc: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceType {
    pub kind: SurfaceTypeKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceTypeKind {
    Path(SurfacePath),
    Apply {
        base: SurfacePath,
        args: Vec<SurfaceType>,
    },
    Ref {
        lifetime: Option<SurfaceLifetime>,
        mutable: bool,
        inner: Box<SurfaceType>,
    },
    Tuple(Vec<SurfaceType>),
    Projection(SurfaceProjection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWhereClause {
    pub predicates: Vec<SurfacePredicate>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfacePredicate {
    TraitBound {
        trait_ref: SurfaceTraitRef,
        span: Span,
    },
    ProjectionEq {
        projection: SurfaceProjection,
        value: SurfaceType,
        span: Span,
    },
    LifetimeOutlives {
        longer: SurfaceLifetime,
        shorter: SurfaceLifetime,
        span: Span,
    },
    TypeOutlives {
        ty: SurfaceType,
        lifetime: SurfaceLifetime,
        span: Span,
    },
}

impl SurfacePath {
    pub fn render(&self) -> String {
        self.segments.join(".")
    }

    pub fn collect_refs(&self, refs: &mut SurfaceRefs) {
        refs.paths.push(self.segments.clone());
    }
}

impl SurfaceLifetime {
    pub fn render(&self) -> String {
        self.name.clone()
    }

    pub fn collect_refs(&self, refs: &mut SurfaceRefs) {
        refs.lifetimes.push(self.name.clone());
    }
}

impl SurfaceTraitRef {
    pub fn render(&self) -> String {
        if self.args.is_empty() {
            self.path.render()
        } else {
            format!(
                "{}[{}]",
                self.path.render(),
                self.args
                    .iter()
                    .map(SurfaceType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    pub fn collect_refs(&self, refs: &mut SurfaceRefs) {
        self.path.collect_refs(refs);
        for arg in &self.args {
            arg.collect_refs(refs);
        }
    }
}

impl SurfaceProjection {
    pub fn render(&self) -> String {
        format!("{}.{}", self.trait_ref.render(), self.assoc)
    }

    pub fn collect_refs(&self, refs: &mut SurfaceRefs) {
        self.trait_ref.collect_refs(refs);
    }
}

impl SurfaceType {
    pub fn render(&self) -> String {
        match &self.kind {
            SurfaceTypeKind::Path(path) => path.render(),
            SurfaceTypeKind::Apply { base, args } => format!(
                "{}[{}]",
                base.render(),
                args.iter()
                    .map(SurfaceType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            SurfaceTypeKind::Ref {
                lifetime,
                mutable,
                inner,
            } => {
                let mut rendered = String::from("&");
                if let Some(lifetime) = lifetime {
                    rendered.push_str(&lifetime.render());
                    rendered.push(' ');
                }
                if *mutable {
                    rendered.push_str("mut ");
                }
                rendered.push_str(&inner.render());
                rendered
            }
            SurfaceTypeKind::Tuple(items) => format!(
                "({})",
                items
                    .iter()
                    .map(SurfaceType::render)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            SurfaceTypeKind::Projection(projection) => projection.render(),
        }
    }

    pub fn collect_refs(&self, refs: &mut SurfaceRefs) {
        match &self.kind {
            SurfaceTypeKind::Path(path) => path.collect_refs(refs),
            SurfaceTypeKind::Apply { base, args } => {
                base.collect_refs(refs);
                for arg in args {
                    arg.collect_refs(refs);
                }
            }
            SurfaceTypeKind::Ref {
                lifetime, inner, ..
            } => {
                if let Some(lifetime) = lifetime {
                    lifetime.collect_refs(refs);
                }
                inner.collect_refs(refs);
            }
            SurfaceTypeKind::Tuple(items) => {
                for item in items {
                    item.collect_refs(refs);
                }
            }
            SurfaceTypeKind::Projection(projection) => projection.collect_refs(refs),
        }
    }

    pub fn refs(&self) -> SurfaceRefs {
        let mut refs = SurfaceRefs::default();
        self.collect_refs(&mut refs);
        refs
    }

    pub fn is_ref(&self) -> bool {
        matches!(self.kind, SurfaceTypeKind::Ref { .. })
    }

    pub fn is_mut_ref(&self) -> bool {
        matches!(self.kind, SurfaceTypeKind::Ref { mutable: true, .. })
    }
}

impl SurfaceWhereClause {
    pub fn render(&self) -> String {
        self.predicates
            .iter()
            .map(SurfacePredicate::render)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn collect_refs(&self, refs: &mut SurfaceRefs) {
        for predicate in &self.predicates {
            predicate.collect_refs(refs);
        }
    }

    pub fn refs(&self) -> SurfaceRefs {
        let mut refs = SurfaceRefs::default();
        self.collect_refs(&mut refs);
        refs
    }
}

impl SurfacePredicate {
    pub fn render(&self) -> String {
        match self {
            SurfacePredicate::TraitBound { trait_ref, .. } => trait_ref.render(),
            SurfacePredicate::ProjectionEq {
                projection, value, ..
            } => format!("{} = {}", projection.render(), value.render()),
            SurfacePredicate::LifetimeOutlives {
                longer, shorter, ..
            } => format!("{}: {}", longer.render(), shorter.render()),
            SurfacePredicate::TypeOutlives { ty, lifetime, .. } => {
                format!("{}: {}", ty.render(), lifetime.render())
            }
        }
    }

    pub fn collect_refs(&self, refs: &mut SurfaceRefs) {
        match self {
            SurfacePredicate::TraitBound { trait_ref, .. } => trait_ref.collect_refs(refs),
            SurfacePredicate::ProjectionEq {
                projection, value, ..
            } => {
                projection.collect_refs(refs);
                value.collect_refs(refs);
            }
            SurfacePredicate::LifetimeOutlives {
                longer, shorter, ..
            } => {
                longer.collect_refs(refs);
                shorter.collect_refs(refs);
            }
            SurfacePredicate::TypeOutlives { ty, lifetime, .. } => {
                ty.collect_refs(refs);
                lifetime.collect_refs(refs);
            }
        }
    }
}

impl fmt::Display for SurfacePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for SurfaceLifetime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for SurfaceTraitRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for SurfaceProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for SurfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for SurfaceWhereClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl fmt::Display for SurfacePredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

pub fn parse_surface_path(text: &str) -> Result<SurfacePath, String> {
    let trimmed = text.trim();
    let Some(segments) = split_simple_path(trimmed) else {
        return Err(format!("malformed path `{trimmed}`"));
    };
    Ok(SurfacePath {
        segments,
        span: Span::default(),
    })
}

pub fn parse_surface_trait_ref(text: &str) -> Result<SurfaceTraitRef, String> {
    let trimmed = text.trim();
    if let Some((base, inside)) = split_trailing_bracket_suffix(trimmed) {
        let path = parse_surface_path(base)?;
        let args = split_top_level_surface_items(inside, ',')
            .into_iter()
            .map(|arg| parse_surface_type(&arg))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(SurfaceTraitRef {
            path,
            args,
            span: Span::default(),
        });
    }
    Ok(SurfaceTraitRef {
        path: parse_surface_path(trimmed)?,
        args: Vec::new(),
        span: Span::default(),
    })
}

pub fn parse_surface_type(text: &str) -> Result<SurfaceType, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty type surface".to_string());
    }
    parse_surface_type_inner(trimmed)
}

pub fn parse_surface_where_clause(text: &str) -> Result<SurfaceWhereClause, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(SurfaceWhereClause {
            predicates: Vec::new(),
            span: Span::default(),
        });
    }

    let predicates = split_top_level_surface_items(trimmed, ',')
        .into_iter()
        .map(|item| parse_surface_predicate(&item))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SurfaceWhereClause {
        predicates,
        span: Span::default(),
    })
}

pub fn surface_type_is_boundary_safe(ty: &SurfaceType) -> bool {
    let refs = ty.refs();
    !refs.paths.into_iter().any(|path| {
        path.last()
            .is_some_and(|name| is_builtin_boundary_unsafe_type_name(name))
    })
}

pub fn validate_tuple_type_contract(
    ty: &SurfaceType,
    span: Span,
    context: &str,
) -> Result<(), String> {
    validate_tuple_type_contract_inner(ty, span, context)
}

fn validate_tuple_type_contract_inner(
    ty: &SurfaceType,
    span: Span,
    context: &str,
) -> Result<(), String> {
    match &ty.kind {
        SurfaceTypeKind::Tuple(items) => {
            if items.len() != 2 {
                return Err(format!(
                    "{}:{}: {context} tuples are not part of v1 except pairs",
                    span.line, span.column
                ));
            }
            for item in items {
                validate_tuple_type_contract_inner(item, span, context)?;
            }
        }
        SurfaceTypeKind::Apply { args, .. } => {
            for arg in args {
                validate_tuple_type_contract_inner(arg, span, context)?;
            }
        }
        SurfaceTypeKind::Ref { inner, .. } => {
            validate_tuple_type_contract_inner(inner, span, context)?;
        }
        SurfaceTypeKind::Projection(_) | SurfaceTypeKind::Path(_) => {}
    }
    Ok(())
}

pub fn collect_surface_type_refs(ty: &SurfaceType) -> SurfaceRefs {
    ty.refs()
}

pub fn collect_surface_where_clause_refs(where_clause: &SurfaceWhereClause) -> SurfaceRefs {
    where_clause.refs()
}

fn parse_surface_predicate(text: &str) -> Result<SurfacePredicate, String> {
    if let Some(index) = find_top_level_surface_char(text, '=') {
        let projection = parse_surface_projection(text[..index].trim())?;
        let value = parse_surface_type(text[index + 1..].trim())?;
        return Ok(SurfacePredicate::ProjectionEq {
            projection,
            value,
            span: Span::default(),
        });
    }
    if let Some(index) = find_top_level_surface_char(text, ':') {
        let left = text[..index].trim();
        let right = text[index + 1..].trim();
        if left.starts_with('\'') && right.starts_with('\'') {
            return Ok(SurfacePredicate::LifetimeOutlives {
                longer: parse_surface_lifetime(left)?,
                shorter: parse_surface_lifetime(right)?,
                span: Span::default(),
            });
        }
        if right.starts_with('\'') {
            return Ok(SurfacePredicate::TypeOutlives {
                ty: parse_surface_type(left)?,
                lifetime: parse_surface_lifetime(right)?,
                span: Span::default(),
            });
        }
        return Err(format!("unsupported where predicate `{}`", text.trim()));
    }
    Ok(SurfacePredicate::TraitBound {
        trait_ref: parse_surface_trait_ref(text)?,
        span: Span::default(),
    })
}

fn parse_surface_projection(text: &str) -> Result<SurfaceProjection, String> {
    let Some(index) = find_projection_split(text) else {
        return Err(format!(
            "projection-equality predicate `{}` must use `<trait-like>.Assoc` on the left",
            text.trim()
        ));
    };
    let base = text[..index].trim();
    let assoc = text[index + 1..].trim();
    if assoc.is_empty() || !is_identifier_text(assoc) {
        return Err(format!("malformed projection `{}`", text.trim()));
    }
    Ok(SurfaceProjection {
        trait_ref: parse_surface_trait_ref(base)?,
        assoc: assoc.to_string(),
        span: Span::default(),
    })
}

fn parse_surface_type_inner(text: &str) -> Result<SurfaceType, String> {
    if let Some(rest) = text.strip_prefix('&') {
        let mut rest = rest.trim_start();
        let lifetime = if rest.starts_with('\'') {
            let (lifetime, consumed) = parse_lifetime_prefix(rest)?;
            rest = rest[consumed..].trim_start();
            Some(lifetime)
        } else {
            None
        };
        let mutable = if let Some(stripped) = rest.strip_prefix("mut") {
            rest = stripped.trim_start();
            true
        } else {
            false
        };
        let inner = parse_surface_type(rest)?;
        return Ok(SurfaceType {
            kind: SurfaceTypeKind::Ref {
                lifetime,
                mutable,
                inner: Box::new(inner),
            },
            span: Span::default(),
        });
    }

    if text.starts_with('(') {
        let Some(close_idx) = find_matching_delim(text, 0, '(', ')') else {
            return Err(format!("malformed tuple type `{text}`"));
        };
        if close_idx == text.len() - 1 {
            let inside = text[1..close_idx].trim();
            let items = if inside.is_empty() {
                Vec::new()
            } else {
                split_top_level_surface_items(inside, ',')
                    .into_iter()
                    .map(|item| parse_surface_type(&item))
                    .collect::<Result<Vec<_>, _>>()?
            };
            return Ok(SurfaceType {
                kind: SurfaceTypeKind::Tuple(items),
                span: Span::default(),
            });
        }
    }

    if let Some(index) = find_projection_split(text) {
        let base = text[..index].trim();
        let assoc = text[index + 1..].trim();
        if !assoc.is_empty() && is_identifier_text(assoc) {
            return Ok(SurfaceType {
                kind: SurfaceTypeKind::Projection(SurfaceProjection {
                    trait_ref: parse_surface_trait_ref(base)?,
                    assoc: assoc.to_string(),
                    span: Span::default(),
                }),
                span: Span::default(),
            });
        }
    }

    if let Some((base, inside)) = split_trailing_bracket_suffix(text) {
        let args = split_top_level_surface_items(inside, ',')
            .into_iter()
            .map(|arg| parse_surface_type(&arg))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(SurfaceType {
            kind: SurfaceTypeKind::Apply {
                base: parse_surface_path(base)?,
                args,
            },
            span: Span::default(),
        });
    }

    Ok(SurfaceType {
        kind: SurfaceTypeKind::Path(parse_surface_path(text)?),
        span: Span::default(),
    })
}

fn parse_surface_lifetime(text: &str) -> Result<SurfaceLifetime, String> {
    let trimmed = text.trim();
    let Some(stripped) = trimmed.strip_prefix('\'') else {
        return Err(format!("malformed lifetime `{trimmed}`"));
    };
    if stripped.is_empty() || !stripped.chars().all(is_ident_continue) {
        return Err(format!("malformed lifetime `{trimmed}`"));
    }
    Ok(SurfaceLifetime {
        name: trimmed.to_string(),
        span: Span::default(),
    })
}

fn parse_lifetime_prefix(text: &str) -> Result<(SurfaceLifetime, usize), String> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.first().copied() != Some('\'') {
        return Err(format!("malformed lifetime prefix `{text}`"));
    }
    let mut index = 1usize;
    while index < chars.len() && is_ident_continue(chars[index]) {
        index += 1;
    }
    let lifetime = chars[..index].iter().collect::<String>();
    Ok((parse_surface_lifetime(&lifetime)?, lifetime.len()))
}

fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut segments = Vec::new();
    for segment in trimmed.split('.') {
        let segment = segment.trim();
        if segment.is_empty() || !is_identifier_text(segment) {
            return None;
        }
        segments.push(segment.to_string());
    }
    (!segments.is_empty()).then_some(segments)
}

fn split_trailing_bracket_suffix(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    if !trimmed.ends_with(']') {
        return None;
    }
    let mut depth = 0usize;
    let mut open = None;
    for (index, ch) in trimmed.char_indices() {
        match ch {
            '[' if depth == 0 => {
                open = Some(index);
                break;
            }
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let open = open?;
    Some((&trimmed[..open], &trimmed[open + 1..trimmed.len() - 1]))
}

fn find_projection_split(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut split_index = None;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            '.' if depth == 0 => split_index = Some(index),
            _ => {}
        }
    }
    let index = split_index?;
    let before = text[..index].chars().next_back()?;
    if before != ']' && before != ')' {
        return None;
    }
    Some(index)
}

fn split_top_level_surface_items(text: &str, separator: char) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for ch in text.chars() {
        if in_string {
            current.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '[' => {
                square_depth += 1;
                current.push(ch);
            }
            ']' => {
                square_depth = square_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == separator && square_depth == 0 && paren_depth == 0 => {
                let item = current.trim();
                if !item.is_empty() {
                    items.push(item.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let tail = current.trim();
    if !tail.is_empty() {
        items.push(tail.to_string());
    }
    items
}

fn find_top_level_surface_char(text: &str, wanted: char) -> Option<usize> {
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (index, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => square_depth += 1,
            ']' => square_depth = square_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            _ if ch == wanted && square_depth == 0 && paren_depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn find_matching_delim(text: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices().skip(start) {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_ident_start(first) && chars.all(is_ident_continue)
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ref_projection_and_where_clause() {
        let ty = parse_surface_type("&'a mut std.iter.Iterator[I].Item").expect("type");
        assert_eq!(ty.render(), "&'a mut std.iter.Iterator[I].Item");
        let refs = ty.refs();
        assert!(refs.paths.iter().any(|path| path
            == &[
                "std".to_string(),
                "iter".to_string(),
                "Iterator".to_string()
            ]));
        assert!(refs.lifetimes.iter().any(|lifetime| lifetime == "'a"));

        let where_clause =
            parse_surface_where_clause("Iterator[I], Iterator[I].Item = U, U: 'a").expect("where");
        assert_eq!(
            where_clause.render(),
            "Iterator[I], Iterator[I].Item = U, U: 'a"
        );
    }

    #[test]
    fn tuple_contract_rejects_non_pair() {
        let ty = parse_surface_type("(Int, Bool, Str)").expect("tuple");
        let err = validate_tuple_type_contract(&ty, Span::new(1, 1), "test type")
            .expect_err("tuple contract should fail");
        assert!(err.contains("pairs"));
    }

    #[test]
    fn boundary_safe_checks_builtin_tokens() {
        let safe = parse_surface_type("List[Int]").expect("safe");
        let unsafe_ty = parse_surface_type("Task[Int]").expect("unsafe");
        assert!(surface_type_is_boundary_safe(&safe));
        assert!(!surface_type_is_boundary_safe(&unsafe_ty));
        assert!(crate::builtin_type_info("Task").is_some());
    }
}
