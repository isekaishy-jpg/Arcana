use std::path::Path;

use arcana_hir::{
    HirImplDecl, HirPredicate, HirResolvedModule, HirResolvedSymbolRef, HirResolvedWorkspace,
    HirSymbolKind, HirTraitRef, HirType, HirTypeBindingScope, HirTypeSubstitutions,
    HirWorkspaceSummary, current_workspace_package_for_module, impl_target_is_public_from_package,
    lookup_symbol_path, substitute_hir_type, visible_method_package_ids_for_module,
};

use crate::semantic_types::SemanticArena;
use crate::type_validate::validate_trait_surface;
use crate::{Diagnostic, TypeScope};

pub(crate) type ResolvedSymbolRef<'a> = HirResolvedSymbolRef<'a>;

pub(crate) fn resolve_trait_symbol_from_trait_ref<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    trait_ref: &HirTraitRef,
) -> Option<ResolvedSymbolRef<'a>> {
    let resolved = lookup_symbol_path(workspace, resolved_module, &trait_ref.path.segments)?;
    (resolved.symbol.kind == HirSymbolKind::Trait).then_some(resolved)
}

pub(crate) fn substitute_trait_ref(
    trait_ref: &HirTraitRef,
    bindings: &HirTypeBindingScope,
    replacements: &HirTypeSubstitutions,
) -> HirTraitRef {
    HirTraitRef {
        path: trait_ref.path.clone(),
        args: trait_ref
            .args
            .iter()
            .map(|arg| substitute_hir_type(arg, bindings, replacements))
            .collect(),
        span: trait_ref.span,
    }
}

pub(crate) fn workspace_has_trait_impl(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    expected_trait_ref: &HirTraitRef,
    expected_target_type: &HirType,
    scope: &TypeScope,
) -> bool {
    let visible_package_ids = visible_method_package_ids_for_module(workspace, resolved_module);
    let current_package_id = current_workspace_package_for_module(workspace, resolved_module)
        .map(|package| package.package_id.as_str());
    let mut semantics = SemanticArena::default();
    let expected_trait_id =
        semantics.trait_ref_id_for_hir(workspace, resolved_module, scope, expected_trait_ref);
    let expected_target_id =
        semantics.type_id_for_hir(workspace, resolved_module, scope, expected_target_type);
    for package in workspace.packages.values() {
        if !visible_package_ids.contains(&package.package_id) {
            continue;
        }
        let Some(resolved_package) = resolved_workspace.package_by_id(&package.package_id) else {
            continue;
        };
        let foreign_package = current_package_id
            .map(|id| id != package.package_id)
            .unwrap_or(false);
        for module in &package.summary.modules {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                continue;
            };
            for impl_decl in &module.impls {
                if foreign_package
                    && !impl_target_is_public_from_package(
                        workspace,
                        package,
                        module,
                        &impl_decl.target_type,
                    )
                {
                    continue;
                }
                let Some(trait_path) = &impl_decl.trait_path else {
                    continue;
                };
                let scope = TypeScope::default()
                    .with_params(&impl_decl.type_params)
                    .with_assoc_types(
                        impl_decl
                            .assoc_types
                            .iter()
                            .map(|assoc_type| assoc_type.name.clone()),
                    )
                    .with_self();
                let impl_trait_id =
                    semantics.trait_ref_id_for_hir(workspace, resolved_module, &scope, trait_path);
                let impl_target_id = semantics.type_id_for_hir(
                    workspace,
                    resolved_module,
                    &scope,
                    &impl_decl.target_type,
                );
                if impl_trait_id == expected_trait_id && impl_target_id == expected_target_id {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn validate_impl_trait_where_requirements_structured(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    impl_decl: &HirImplDecl,
    scope: &TypeScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(trait_path) = &impl_decl.trait_path else {
        return;
    };
    let Some(trait_symbol_ref) =
        resolve_trait_symbol_from_trait_ref(workspace, resolved_module, trait_path)
    else {
        return;
    };
    let Some(where_clause) = &trait_symbol_ref.symbol.where_clause else {
        return;
    };
    let mut bindings = HirTypeBindingScope::default();
    bindings.insert("Self".to_string());
    let mut replacements = HirTypeSubstitutions::new();
    if let Some(self_id) = bindings.binding_id("Self") {
        replacements.insert(self_id, impl_decl.target_type.clone());
    }
    for (formal, actual) in trait_symbol_ref
        .symbol
        .type_params
        .iter()
        .zip(trait_path.args.iter())
    {
        let id = bindings.insert(formal.clone());
        replacements.insert(id, actual.clone());
    }
    for predicate in &where_clause.predicates {
        if let HirPredicate::TraitBound { trait_ref, .. } = predicate {
            let instantiated = substitute_trait_ref(trait_ref, &bindings, &replacements);
            validate_trait_surface(
                workspace,
                resolved_module,
                module_path,
                scope,
                &instantiated,
                impl_decl.span,
                &format!("where predicate `{}`", instantiated.render()),
                diagnostics,
            );
            let has_impl = workspace_has_trait_impl(
                workspace,
                resolved_workspace,
                resolved_module,
                &instantiated,
                instantiated.args.first().unwrap_or(&impl_decl.target_type),
                scope,
            );
            if !has_impl {
                diagnostics.push(Diagnostic {
                    path: module_path.to_path_buf(),
                    line: impl_decl.span.line,
                    column: impl_decl.span.column,
                    message: format!(
                        "impl requires satisfying where-bound `{}` for target `{}`",
                        instantiated.render(),
                        impl_decl.target_type.render()
                    ),
                });
            }
        }
    }
}
