use std::collections::BTreeSet;

use super::*;

pub fn current_workspace_package_for_module<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
) -> Option<&'a HirWorkspacePackage> {
    workspace.package_by_id(&resolved_module.package_id)
}

pub fn visible_package_root_for_module<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    root: &str,
) -> Option<&'a HirWorkspacePackage> {
    let current_package = current_workspace_package_for_module(workspace, resolved_module)?;
    if root == current_package.summary.package_name {
        return Some(current_package);
    }
    if root == "std" {
        return workspace.package("std");
    }
    current_package
        .dependency_package_id(root)
        .and_then(|dependency_id| workspace.package_by_id(dependency_id))
}

pub fn visible_method_package_names_for_module(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
) -> BTreeSet<String> {
    let mut visible = BTreeSet::new();
    let Some(current_package) = current_workspace_package_for_module(workspace, resolved_module)
    else {
        return visible;
    };
    visible.insert(current_package.summary.package_name.clone());
    if workspace.package("std").is_some() {
        visible.insert("std".to_string());
    }
    visible.extend(current_package.direct_dep_packages.values().cloned());
    visible
}

fn resolved_use_target_binding_name(target: &ResolvedUseTarget) -> &str {
    match target {
        ResolvedUseTarget::Module { module_id, .. } => {
            module_id.rsplit('.').next().unwrap_or(module_id)
        }
        ResolvedUseTarget::Symbol { symbol_name, .. } => symbol_name,
    }
}

fn symbol_visible_from_package_boundary(
    current_package_name: &str,
    target_package_name: &str,
    symbol: &HirSymbol,
) -> bool {
    current_package_name == target_package_name || symbol.exported
}

pub(crate) fn visible_symbol_refs_in_module_for_package<'a>(
    workspace: &'a HirWorkspaceSummary,
    current_package_name: &str,
    package_id: &str,
    module_id: &str,
    symbol_name: &str,
) -> Vec<HirResolvedSymbolRef<'a>> {
    let Some(package) = workspace.package_by_id(package_id) else {
        return Vec::new();
    };
    let Some(module) = package.module(module_id) else {
        return Vec::new();
    };
    module
        .symbols
        .iter()
        .enumerate()
        .filter(|(_, symbol)| {
            symbol.name == symbol_name
                && symbol_visible_from_package_boundary(
                    current_package_name,
                    &package.summary.package_name,
                    symbol,
                )
        })
        .map(|(symbol_index, symbol)| HirResolvedSymbolRef {
            package_id: &package.package_id,
            package_name: &package.summary.package_name,
            module_id: &module.module_id,
            symbol_index,
            symbol,
        })
        .collect()
}

fn first_visible_symbol_in_module_for_package<'a>(
    workspace: &'a HirWorkspaceSummary,
    current_package_name: &str,
    package_id: &str,
    module_id: &str,
    symbol_name: &str,
) -> Option<HirResolvedSymbolRef<'a>> {
    visible_symbol_refs_in_module_for_package(
        workspace,
        current_package_name,
        package_id,
        module_id,
        symbol_name,
    )
    .into_iter()
    .next()
}

fn lookup_package_symbol_path_filtered<'a>(
    workspace: &'a HirWorkspaceSummary,
    current_package_name: &str,
    package: &'a HirWorkspacePackage,
    path: &[String],
) -> Option<HirResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    let (symbol_name, module_path) = path.split_last()?;
    let module = if module_path.is_empty() {
        package.module(&package.summary.package_name)
    } else {
        package.resolve_relative_module(module_path)
    }?;
    first_visible_symbol_in_module_for_package(
        workspace,
        current_package_name,
        &package.package_id,
        &module.module_id,
        symbol_name,
    )
}

fn lookup_module_symbol_path_filtered<'a>(
    workspace: &'a HirWorkspaceSummary,
    current_package_name: &str,
    package: &'a HirWorkspacePackage,
    module: &'a HirModuleSummary,
    path: &[String],
) -> Option<HirResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return first_visible_symbol_in_module_for_package(
            workspace,
            current_package_name,
            &package.package_id,
            &module.module_id,
            &path[0],
        );
    }
    let (symbol_name, module_tail) = path.split_last()?;
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut target_relative = base_relative;
    target_relative.extend_from_slice(module_tail);
    let target_module = package.resolve_relative_module(&target_relative)?;
    first_visible_symbol_in_module_for_package(
        workspace,
        current_package_name,
        &package.package_id,
        &target_module.module_id,
        symbol_name,
    )
}

fn lookup_target_symbol_tail_filtered<'a>(
    workspace: &'a HirWorkspaceSummary,
    current_package_name: &str,
    target: &'a HirResolvedTarget,
    tail: &[String],
) -> Option<HirResolvedSymbolRef<'a>> {
    match target {
        HirResolvedTarget::Symbol {
            package_id,
            module_id,
            symbol_name,
            ..
        } => {
            if !tail.is_empty() {
                return None;
            }
            first_visible_symbol_in_module_for_package(
                workspace,
                current_package_name,
                package_id,
                module_id,
                symbol_name,
            )
        }
        HirResolvedTarget::Module {
            package_id,
            module_id,
            ..
        } => {
            let package = workspace.package_by_id(package_id)?;
            let module = package.module(module_id)?;
            lookup_module_symbol_path_filtered(
                workspace,
                current_package_name,
                package,
                module,
                tail,
            )
        }
    }
}

fn lookup_symbol_tail_in_resolved_use_target<'a>(
    workspace: &'a HirWorkspaceSummary,
    current_package_name: &str,
    target: &ResolvedUseTarget,
    tail: &[String],
) -> Option<HirResolvedSymbolRef<'a>> {
    match target {
        ResolvedUseTarget::Symbol {
            package_id,
            module_id,
            symbol_name,
            ..
        } => {
            if !tail.is_empty() {
                return None;
            }
            first_visible_symbol_in_module_for_package(
                workspace,
                current_package_name,
                package_id,
                module_id,
                symbol_name,
            )
        }
        ResolvedUseTarget::Module {
            package_id,
            module_id,
            ..
        } => {
            let package = workspace.package_by_id(package_id)?;
            let module = package.module(module_id)?;
            lookup_module_symbol_path_filtered(
                workspace,
                current_package_name,
                package,
                module,
                tail,
            )
        }
    }
}

fn lookup_symbol_path_via_module_directives<'a>(
    workspace: &'a HirWorkspaceSummary,
    package: &'a HirWorkspacePackage,
    module: &'a HirModuleSummary,
    path: &[String],
) -> Option<HirResolvedSymbolRef<'a>> {
    let first = path.first()?;
    let current_package_name = package.summary.package_name.as_str();
    for directive in &module.directives {
        match directive.kind {
            HirDirectiveKind::Import => {
                let binding_name = directive
                    .alias
                    .clone()
                    .or_else(|| directive.path.last().cloned())?;
                if &binding_name != first {
                    continue;
                }
                let (package_id, _package_name, module_id) =
                    resolve_module_target(package, workspace, &directive.path).ok()?;
                let dependency = workspace.package_by_id(&package_id)?;
                let target_module = dependency.module(&module_id)?;
                return lookup_module_symbol_path_filtered(
                    workspace,
                    current_package_name,
                    dependency,
                    target_module,
                    &path[1..],
                );
            }
            HirDirectiveKind::Use | HirDirectiveKind::Reexport => {
                let target = resolve_use_target(package, workspace, &directive.path).ok()?;
                let binding_name = directive
                    .alias
                    .as_deref()
                    .unwrap_or_else(|| resolved_use_target_binding_name(&target));
                if binding_name != first {
                    continue;
                }
                return lookup_symbol_tail_in_resolved_use_target(
                    workspace,
                    current_package_name,
                    &target,
                    &path[1..],
                );
            }
        }
    }
    None
}

pub(crate) fn lookup_symbol_path_in_module_context<'a>(
    workspace: &'a HirWorkspaceSummary,
    package: &'a HirWorkspacePackage,
    module: &'a HirModuleSummary,
    path: &[String],
) -> Option<HirResolvedSymbolRef<'a>> {
    let first = path.first()?;
    let current_package_name = package.summary.package_name.as_str();
    if first == &package.summary.package_name {
        return lookup_package_symbol_path_filtered(
            workspace,
            current_package_name,
            package,
            &path[1..],
        );
    }
    if first == "std" {
        return workspace.package("std").and_then(|std_package| {
            lookup_package_symbol_path_filtered(
                workspace,
                current_package_name,
                std_package,
                &path[1..],
            )
        });
    }
    if let Some(dependency_name) = package.dependency_package_name(first) {
        let dependency_id = package.dependency_package_id(first)?;
        let _ = dependency_name;
        return workspace
            .package_by_id(dependency_id)
            .and_then(|dependency| {
                lookup_package_symbol_path_filtered(
                    workspace,
                    current_package_name,
                    dependency,
                    &path[1..],
                )
            });
    }
    lookup_module_symbol_path_filtered(workspace, current_package_name, package, module, path)
        .or_else(|| lookup_symbol_path_via_module_directives(workspace, package, module, path))
        .or_else(|| {
            lookup_package_symbol_path_filtered(workspace, current_package_name, package, path)
        })
}

pub fn lookup_symbol_path<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    path: &[String],
) -> Option<HirResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    let current_package = current_workspace_package_for_module(workspace, resolved_module)?;
    let current_package_name = current_package.summary.package_name.as_str();
    if path.len() == 1 {
        return resolved_module.bindings.get(&path[0]).and_then(|binding| {
            lookup_target_symbol_tail_filtered(
                workspace,
                current_package_name,
                &binding.target,
                &[],
            )
        });
    }

    let first = &path[0];
    if let Some(binding) = resolved_module.bindings.get(first) {
        return lookup_target_symbol_tail_filtered(
            workspace,
            current_package_name,
            &binding.target,
            &path[1..],
        );
    }

    let current_module = current_package.module(&resolved_module.module_id)?;

    if let Some(package) = visible_package_root_for_module(workspace, resolved_module, first) {
        return lookup_package_symbol_path_filtered(
            workspace,
            current_package_name,
            package,
            &path[1..],
        );
    }

    lookup_module_symbol_path_filtered(
        workspace,
        current_package_name,
        current_package,
        current_module,
        path,
    )
    .or_else(|| {
        lookup_package_symbol_path_filtered(workspace, current_package_name, current_package, path)
    })
}

pub fn impl_target_is_public_from_package(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    target_type: &HirType,
) -> bool {
    let Some(base_path) = hir_type_base_path(hir_strip_reference_type(target_type)) else {
        return true;
    };
    if matches!(base_path.as_slice(), [name] if builtin_type_info(name).is_some())
        || canonical_ambient_type_root(&base_path).is_some()
    {
        return true;
    }
    let Some(symbol_ref) =
        lookup_symbol_path_in_module_context(workspace, package, module, &base_path)
    else {
        return false;
    };
    symbol_ref.package_name != package.summary.package_name || symbol_ref.symbol.exported
}

fn object_declared_receiver_type(module_id: &str, symbol: &HirSymbol) -> HirType {
    let base = HirPath {
        segments: module_id
            .split('.')
            .map(str::to_string)
            .chain(std::iter::once(symbol.name.clone()))
            .collect(),
        span: symbol.span,
    };
    if symbol.type_params.is_empty() {
        HirType {
            kind: HirTypeKind::Path(base),
            span: symbol.span,
        }
    } else {
        HirType {
            kind: HirTypeKind::Apply {
                base,
                args: symbol
                    .type_params
                    .iter()
                    .map(|param| HirType {
                        kind: HirTypeKind::Path(HirPath {
                            segments: vec![param.clone()],
                            span: symbol.span,
                        }),
                        span: symbol.span,
                    })
                    .collect(),
            },
            span: symbol.span,
        }
    }
}

pub fn lookup_method_candidates_for_hir_type<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    wanted: &HirType,
    method_name: &str,
) -> Vec<HirMethodCandidate<'a>> {
    let Some(current_package) = current_workspace_package_for_module(workspace, resolved_module)
    else {
        return Vec::new();
    };
    let Some(current_module) = current_package.module(&resolved_module.module_id) else {
        return Vec::new();
    };
    let wanted = canonicalize_hir_type_in_module(
        workspace,
        current_package,
        current_module,
        hir_strip_reference_type(wanted),
    );
    let visible_packages = visible_method_package_names_for_module(workspace, resolved_module);
    let current_package_name = current_workspace_package_for_module(workspace, resolved_module)
        .map(|package| package.summary.package_name.as_str());
    let mut candidates = Vec::new();
    let mut seen_routines = BTreeSet::new();
    for package in workspace.packages.values() {
        if !visible_packages.contains(&package.summary.package_name) {
            continue;
        }
        let foreign_package = current_package_name
            .map(|name| name != package.summary.package_name)
            .unwrap_or(false);
        for module in &package.summary.modules {
            for (symbol_index, symbol) in module.symbols.iter().enumerate() {
                let HirSymbolBody::Object { methods, .. } = &symbol.body else {
                    continue;
                };
                if foreign_package && !symbol.exported {
                    continue;
                }
                let declared_hir = object_declared_receiver_type(&module.module_id, symbol);
                let canonical_declared =
                    canonicalize_hir_type_in_module(workspace, package, module, &declared_hir);
                let bindings = placeholder_binding_scope_for_type(&declared_hir);
                if !hir_type_matches(
                    &canonical_declared,
                    &wanted,
                    &bindings,
                    &mut HirTypeSubstitutions::new(),
                ) {
                    continue;
                }
                for (method_index, method) in methods.iter().enumerate() {
                    if method.name != method_name {
                        continue;
                    }
                    let routine_key = routine_key_for_object_method(
                        &module.module_id,
                        symbol_index,
                        method_index,
                    );
                    if !seen_routines.insert(routine_key.clone()) {
                        continue;
                    }
                    candidates.push(HirMethodCandidate {
                        package_id: &package.package_id,
                        package_name: &package.summary.package_name,
                        module_id: &module.module_id,
                        symbol: method,
                        declared_receiver_hir: declared_hir.clone(),
                        routine_key,
                        trait_path: None,
                    });
                }
            }
            for (impl_index, impl_decl) in module.impls.iter().enumerate() {
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
                let canonical_declared = canonicalize_hir_type_in_module(
                    workspace,
                    package,
                    module,
                    &impl_decl.target_type,
                );
                let bindings = placeholder_binding_scope_for_type(&impl_decl.target_type);
                if !hir_type_matches(
                    &canonical_declared,
                    &wanted,
                    &bindings,
                    &mut HirTypeSubstitutions::new(),
                ) {
                    continue;
                }
                for (method_index, method) in impl_decl.methods.iter().enumerate() {
                    if method.name != method_name {
                        continue;
                    }
                    let routine_key =
                        routine_key_for_impl_method(&module.module_id, impl_index, method_index);
                    if !seen_routines.insert(routine_key.clone()) {
                        continue;
                    }
                    candidates.push(HirMethodCandidate {
                        package_id: &package.package_id,
                        package_name: &package.summary.package_name,
                        module_id: &module.module_id,
                        symbol: method,
                        declared_receiver_hir: impl_decl.target_type.clone(),
                        routine_key,
                        trait_path: impl_decl.trait_path.as_ref().map(|trait_ref| {
                            canonicalize_hir_trait_ref_in_module(
                                workspace, package, module, trait_ref,
                            )
                            .path
                            .segments
                        }),
                    });
                }
            }
        }
    }
    candidates
}
