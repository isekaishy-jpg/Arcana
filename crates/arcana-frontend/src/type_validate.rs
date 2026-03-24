use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use arcana_hir::{
    HirForewordApp, HirResolvedModule, HirResolvedWorkspace, HirSymbol, HirSymbolBody,
    HirSymbolKind, HirType, HirWorkspaceSummary,
};
use arcana_syntax::{Span, builtin_type_info, is_builtin_boundary_unsafe_type_name};

use crate::surface::{
    ResolvedSymbolRef, SurfaceSymbolUse, lookup_symbol_path, surface_use_name,
    symbol_matches_surface_use,
};
use crate::{Diagnostic, TypeScope};

pub(crate) fn validate_type_surface(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &TypeScope,
    ty: &HirType,
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let refs = arcana_hir::collect_hir_type_refs(ty);
    validate_surface_refs(
        workspace,
        resolved_module,
        module_path,
        scope,
        &refs,
        span,
        context,
        SurfaceSymbolUse::TypeLike,
        diagnostics,
    );
}

pub(crate) fn validate_trait_surface(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &TypeScope,
    trait_ref: &arcana_hir::HirTraitRef,
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_surface_path_kind(
        workspace,
        resolved_module,
        module_path,
        scope,
        &trait_ref.path.segments,
        span,
        context,
        SurfaceSymbolUse::Trait,
        diagnostics,
    );
    for arg in &trait_ref.args {
        validate_type_surface(
            workspace,
            resolved_module,
            module_path,
            scope,
            arg,
            span,
            context,
            diagnostics,
        );
    }
}

pub(crate) fn validate_boundary_symbol_contract(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    scope: &TypeScope,
    self_type: Option<HirType>,
    assoc_bindings: &BTreeMap<String, HirType>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(target) = boundary_target_from_forewords(&symbol.forewords) else {
        return;
    };

    for param in &symbol.params {
        let mut visited = BTreeSet::new();
        if !boundary_type_is_safe(
            workspace,
            resolved_workspace,
            resolved_module,
            scope,
            &param.ty,
            self_type.as_ref(),
            assoc_bindings,
            &mut visited,
        ) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: symbol.span.line,
                column: symbol.span.column,
                message: format!(
                    "type `{}` is not boundary-safe for target `{target}`",
                    param.ty
                ),
            });
        }
    }

    if let Some(return_type) = &symbol.return_type {
        let mut visited = BTreeSet::new();
        if !boundary_type_is_safe(
            workspace,
            resolved_workspace,
            resolved_module,
            scope,
            return_type,
            self_type.as_ref(),
            assoc_bindings,
            &mut visited,
        ) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: symbol.span.line,
                column: symbol.span.column,
                message: format!(
                    "type `{}` is not boundary-safe for target `{target}`",
                    return_type
                ),
            });
        }
    }
}

fn boundary_target_from_forewords(forewords: &[HirForewordApp]) -> Option<String> {
    forewords
        .iter()
        .find(|foreword| foreword.name == "boundary")
        .and_then(|foreword| {
            foreword
                .args
                .iter()
                .find(|arg| arg.name.as_deref() == Some("target"))
        })
        .and_then(|arg| parse_symbol_or_string_literal(&arg.value))
}

fn boundary_type_is_safe(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    ty: &HirType,
    self_type: Option<&HirType>,
    assoc_bindings: &BTreeMap<String, HirType>,
    visited_symbols: &mut BTreeSet<String>,
) -> bool {
    match &ty.kind {
        arcana_hir::HirTypeKind::Path(path) => boundary_path_is_safe(
            workspace,
            resolved_workspace,
            resolved_module,
            scope,
            &path.segments,
            self_type,
            assoc_bindings,
            visited_symbols,
        ),
        arcana_hir::HirTypeKind::Apply { base, args } => {
            boundary_path_is_safe(
                workspace,
                resolved_workspace,
                resolved_module,
                scope,
                &base.segments,
                self_type,
                assoc_bindings,
                visited_symbols,
            ) && args.iter().all(|arg| {
                boundary_type_is_safe(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    scope,
                    arg,
                    self_type,
                    assoc_bindings,
                    visited_symbols,
                )
            })
        }
        arcana_hir::HirTypeKind::Ref { inner, .. } => boundary_type_is_safe(
            workspace,
            resolved_workspace,
            resolved_module,
            scope,
            inner,
            self_type,
            assoc_bindings,
            visited_symbols,
        ),
        arcana_hir::HirTypeKind::Tuple(items) => items.iter().all(|item| {
            boundary_type_is_safe(
                workspace,
                resolved_workspace,
                resolved_module,
                scope,
                item,
                self_type,
                assoc_bindings,
                visited_symbols,
            )
        }),
        arcana_hir::HirTypeKind::Projection(projection) => {
            boundary_path_is_safe(
                workspace,
                resolved_workspace,
                resolved_module,
                scope,
                &projection.trait_ref.path.segments,
                self_type,
                assoc_bindings,
                visited_symbols,
            ) && projection.trait_ref.args.iter().all(|arg| {
                boundary_type_is_safe(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    scope,
                    arg,
                    self_type,
                    assoc_bindings,
                    visited_symbols,
                )
            })
        }
    }
}

fn boundary_path_is_safe(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    path: &[String],
    self_type: Option<&HirType>,
    assoc_bindings: &BTreeMap<String, HirType>,
    visited_symbols: &mut BTreeSet<String>,
) -> bool {
    if path.len() == 1 {
        let name = &path[0];
        if name == "Self" {
            return self_type.is_none_or(|self_type| {
                boundary_type_is_safe(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    scope,
                    self_type,
                    None,
                    assoc_bindings,
                    visited_symbols,
                )
            });
        }
        if let Some(value_ty) = assoc_bindings.get(name) {
            return boundary_type_is_safe(
                workspace,
                resolved_workspace,
                resolved_module,
                scope,
                value_ty,
                self_type,
                assoc_bindings,
                visited_symbols,
            );
        }
        if scope.allows_type_name(name) || is_boundary_safe_builtin_name(name) {
            return true;
        }
        if is_boundary_unsafe_builtin_name(name) {
            return false;
        }
    }

    let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, path) else {
        return true;
    };
    boundary_symbol_is_safe(
        workspace,
        resolved_workspace,
        resolved_module,
        scope,
        &symbol_ref,
        self_type,
        assoc_bindings,
        visited_symbols,
    )
}

fn boundary_symbol_is_safe(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    symbol_ref: &ResolvedSymbolRef<'_>,
    self_type: Option<&HirType>,
    assoc_bindings: &BTreeMap<String, HirType>,
    visited_symbols: &mut BTreeSet<String>,
) -> bool {
    let visit_key = format!(
        "{}::{}::{}",
        symbol_ref.package_name, symbol_ref.module_id, symbol_ref.symbol.name
    );
    if !visited_symbols.insert(visit_key) {
        return true;
    }

    let nested_scope = TypeScope::default().with_params(&symbol_ref.symbol.type_params);
    let owner_module = resolved_workspace
        .package(symbol_ref.package_name)
        .and_then(|package| package.module(symbol_ref.module_id))
        .unwrap_or(resolved_module);
    match &symbol_ref.symbol.body {
        HirSymbolBody::Record { fields } | HirSymbolBody::Object { fields, .. } => {
            fields.iter().all(|field| {
                boundary_type_is_safe(
                    workspace,
                    resolved_workspace,
                    owner_module,
                    &nested_scope,
                    &field.ty,
                    self_type,
                    assoc_bindings,
                    visited_symbols,
                )
            })
        }
        HirSymbolBody::Enum { variants } => variants.iter().all(|variant| {
            variant.payload.as_ref().is_none_or(|payload| {
                boundary_type_is_safe(
                    workspace,
                    resolved_workspace,
                    owner_module,
                    &nested_scope,
                    payload,
                    self_type,
                    assoc_bindings,
                    visited_symbols,
                )
            })
        }),
        _ if symbol_ref.symbol.kind == HirSymbolKind::OpaqueType => {
            !crate::opaque_symbol_is_boundary_unsafe(symbol_ref.symbol)
        }
        _ => scope.allows_type_name(&symbol_ref.symbol.name),
    }
}

fn is_boundary_safe_builtin_name(name: &str) -> bool {
    builtin_type_info(name).is_some_and(|info| !info.boundary_unsafe)
}

fn is_boundary_unsafe_builtin_name(name: &str) -> bool {
    is_builtin_boundary_unsafe_type_name(name)
}

fn validate_surface_refs(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &TypeScope,
    refs: &arcana_hir::HirSurfaceRefs,
    span: Span,
    context: &str,
    expected_use: SurfaceSymbolUse,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen_lifetimes = BTreeSet::new();
    for lifetime in &refs.lifetimes {
        if seen_lifetimes.insert(lifetime.clone()) && !scope.lifetime_declared(lifetime) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: span.line,
                column: span.column,
                message: format!("undeclared lifetime `{lifetime}` in {context}"),
            });
        }
    }

    let mut seen_paths = BTreeSet::new();
    for path in &refs.paths {
        let path_key = path.join(".");
        if !seen_paths.insert(path_key) {
            continue;
        }
        validate_surface_path_kind(
            workspace,
            resolved_module,
            module_path,
            scope,
            path,
            span,
            context,
            expected_use,
            diagnostics,
        );
    }
}

pub(crate) fn validate_surface_path_kind(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &TypeScope,
    path: &[String],
    span: Span,
    context: &str,
    expected_use: SurfaceSymbolUse,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path_key = path.join(".");
    if path.len() == 1 && scope.allows_type_name(&path[0]) {
        return;
    }
    if path.len() == 1 && crate::is_builtin_type_name(&path[0]) {
        return;
    }
    let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, path) else {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message: format!("unresolved type reference `{path_key}` in {context}"),
        });
        return;
    };
    if !symbol_matches_surface_use(symbol_ref.symbol.kind, expected_use) {
        diagnostics.push(Diagnostic {
            path: module_path.to_path_buf(),
            line: span.line,
            column: span.column,
            message: format!(
                "`{path_key}` does not resolve to a valid {} in {context}",
                surface_use_name(expected_use)
            ),
        });
    }
}

fn parse_symbol_or_string_literal(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if let Some(unquoted) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return Some(unquoted.to_string());
    }
    crate::surface::split_simple_path(trimmed)
        .filter(|path| path.len() == 1)
        .map(|path| path[0].clone())
}
