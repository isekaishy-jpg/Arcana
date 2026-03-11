use std::collections::HashMap;

use arcana_hir::{
    HirDirectiveKind, HirImplDecl, HirResolvedModule, HirResolvedPackage, HirResolvedTarget,
    HirResolvedWorkspace, HirSymbol, HirSymbolBody, HirSymbolKind, HirWorkspacePackage,
    HirWorkspaceSummary,
};
use arcana_package::{MemberFingerprints, WorkspaceGraph, WorkspaceMember};
use sha2::{Digest, Sha256};

use crate::surface::{
    canonicalize_surface_path, canonicalize_surface_text, split_simple_path, surface_text_is_public,
};
use crate::{CheckedWorkspace, TypeScope};

pub(crate) fn compute_member_fingerprints_for_checked_workspace(
    graph: &WorkspaceGraph,
    checked: &CheckedWorkspace,
) -> Result<HashMap<String, MemberFingerprints>, String> {
    compute_member_fingerprints_for_workspace(
        graph,
        &checked.workspace,
        &checked.resolved_workspace,
    )
}

pub(crate) fn compute_member_fingerprints_for_workspace(
    graph: &WorkspaceGraph,
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
) -> Result<HashMap<String, MemberFingerprints>, String> {
    let mut fingerprints = HashMap::new();

    for member in &graph.members {
        let source = compute_member_source_fingerprint(member, workspace)?;
        let api = compute_resolved_api_fingerprint(member, workspace, resolved_workspace)?;
        fingerprints.insert(member.name.clone(), MemberFingerprints { source, api });
    }

    Ok(fingerprints)
}

fn compute_resolved_api_fingerprint(
    member: &WorkspaceMember,
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
) -> Result<String, String> {
    let package = workspace
        .package(&member.name)
        .ok_or_else(|| format!("package `{}` is not loaded in workspace HIR", member.name))?;
    let resolved_package = resolved_workspace
        .package(&member.name)
        .ok_or_else(|| format!("resolved package `{}` is not loaded", member.name))?;

    let mut hasher = Sha256::new();
    hasher.update(b"arcana_resolved_api_v1\n");
    hasher.update(format!("name={}\n", member.name).as_bytes());
    hasher.update(format!("kind={}\n", member.kind.as_str()).as_bytes());
    for dep in &member.deps {
        hasher.update(format!("dep={dep}\n").as_bytes());
    }

    for row in resolved_package_api_rows(package, resolved_package, workspace)? {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }

    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn compute_member_source_fingerprint(
    member: &WorkspaceMember,
    workspace: &HirWorkspaceSummary,
) -> Result<String, String> {
    let package = workspace
        .package(&member.name)
        .ok_or_else(|| format!("package `{}` is not loaded in workspace HIR", member.name))?;
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_hir_member_v1\n");
    hasher.update(format!("name={}\n", member.name).as_bytes());
    hasher.update(format!("kind={}\n", member.kind.as_str()).as_bytes());
    for dep in &member.deps {
        hasher.update(format!("dep={dep}\n").as_bytes());
    }
    for row in package.summary.hir_fingerprint_rows() {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn resolved_package_api_rows(
    package: &HirWorkspacePackage,
    resolved_package: &HirResolvedPackage,
    workspace: &HirWorkspaceSummary,
) -> Result<Vec<String>, String> {
    let mut rows = Vec::new();
    for module in &package.summary.modules {
        let resolved_module = resolved_package
            .module(&module.module_id)
            .ok_or_else(|| format!("resolved module `{}` is not loaded", module.module_id))?;
        for row in resolved_module_api_rows(package, resolved_module, workspace, module) {
            rows.push(format!("module={}:{}", module.module_id, row));
        }
    }
    rows.sort();
    Ok(rows)
}

fn resolved_module_api_rows(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    module: &arcana_hir::HirModuleSummary,
) -> Vec<String> {
    let mut rows = resolved_module
        .directives
        .iter()
        .filter(|directive| directive.kind == HirDirectiveKind::Reexport)
        .map(|directive| {
            format!(
                "reexport:local={}|target={}",
                directive.local_name,
                render_resolved_target_fingerprint(&directive.target)
            )
        })
        .collect::<Vec<_>>();

    for symbol in &module.symbols {
        if symbol.exported {
            rows.push(format!(
                "export:{}:{}",
                symbol.kind.as_str(),
                render_symbol_api_fingerprint(workspace, resolved_module, symbol)
            ));
        }
    }

    let module_scope = TypeScope::default();
    for impl_decl in &module.impls {
        if impl_decl_is_public(
            package,
            resolved_module,
            workspace,
            &module_scope,
            impl_decl,
        ) {
            rows.push(format!(
                "impl:{}",
                render_impl_api_fingerprint(workspace, resolved_module, impl_decl)
            ));
        }
    }

    rows.sort();
    rows
}

fn render_resolved_target_fingerprint(target: &HirResolvedTarget) -> String {
    match target {
        HirResolvedTarget::Module { module_id, .. } => format!("module:{module_id}"),
        HirResolvedTarget::Symbol {
            module_id,
            symbol_name,
            ..
        } => format!("symbol:{module_id}.{symbol_name}"),
    }
}

fn render_symbol_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let base = match symbol.kind {
        HirSymbolKind::Fn | HirSymbolKind::System => render_callable_symbol_api_fingerprint(
            workspace,
            resolved_module,
            symbol,
            &TypeScope::default(),
        ),
        HirSymbolKind::Record => render_record_api_fingerprint(workspace, resolved_module, symbol),
        HirSymbolKind::Enum => render_enum_api_fingerprint(workspace, resolved_module, symbol),
        HirSymbolKind::Trait => render_trait_api_fingerprint(workspace, resolved_module, symbol),
        HirSymbolKind::Behavior => {
            render_behavior_api_fingerprint(workspace, resolved_module, symbol)
        }
        HirSymbolKind::Const => canonicalize_surface_text(
            workspace,
            resolved_module,
            &TypeScope::default(),
            &symbol.surface_text,
        ),
    };
    append_symbol_contract_metadata(base, workspace, resolved_module, symbol)
}

fn render_callable_symbol_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
    scope: &TypeScope,
) -> String {
    let scope = scope.with_params(&symbol.type_params);
    let mut rendered = String::new();
    if symbol.is_async {
        rendered.push_str("async");
    }
    rendered.push_str("fn:");
    rendered.push_str(&symbol.name);
    rendered.push('[');
    rendered.push_str(&symbol.type_params.join(","));
    rendered.push(']');
    if let Some(where_clause) = &symbol.where_clause {
        rendered.push_str("|where=");
        rendered.push_str(&canonicalize_surface_text(
            workspace,
            resolved_module,
            &scope,
            where_clause,
        ));
    }
    rendered.push('(');
    rendered.push_str(
        &symbol
            .params
            .iter()
            .map(|param| {
                let mut part = String::new();
                if let Some(mode) = param.mode {
                    part.push_str(mode.as_str());
                    part.push(':');
                }
                part.push_str(&param.name);
                part.push(':');
                part.push_str(&canonicalize_surface_text(
                    workspace,
                    resolved_module,
                    &scope,
                    &param.ty,
                ));
                part
            })
            .collect::<Vec<_>>()
            .join(","),
    );
    rendered.push(')');
    if let Some(return_type) = &symbol.return_type {
        rendered.push_str("->");
        rendered.push_str(&canonicalize_surface_text(
            workspace,
            resolved_module,
            &scope,
            return_type,
        ));
    }
    rendered
}

fn append_symbol_contract_metadata(
    mut base: String,
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    if !symbol.forewords.is_empty() {
        base.push_str("|forewords=[");
        base.push_str(
            &symbol
                .forewords
                .iter()
                .map(|foreword| {
                    render_foreword_api_fingerprint(workspace, resolved_module, foreword)
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        base.push(']');
    }
    if let Some(intrinsic_impl) = &symbol.intrinsic_impl {
        base.push_str("|intrinsic=");
        base.push_str(intrinsic_impl);
    }
    base
}

fn render_foreword_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    foreword: &arcana_hir::HirForewordApp,
) -> String {
    format!(
        "{}[{}]",
        foreword.name,
        foreword
            .args
            .iter()
            .map(|arg| {
                let value = canonicalize_foreword_arg_value(workspace, resolved_module, &arg.value);
                match &arg.name {
                    Some(name) => format!("{name}={value}"),
                    None => value,
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn canonicalize_foreword_arg_value(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    value: &str,
) -> String {
    let trimmed = value.trim();
    if let Some(unquoted) = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return format!("str:{unquoted}");
    }
    if let Some(path) = split_simple_path(trimmed) {
        return format!(
            "path:{}",
            canonicalize_surface_path(workspace, resolved_module, &TypeScope::default(), &path)
        );
    }
    trimmed.to_string()
}

fn render_record_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let scope = TypeScope::default().with_params(&symbol.type_params);
    let fields = match &symbol.body {
        HirSymbolBody::Record { fields } => fields
            .iter()
            .map(|field| {
                format!(
                    "{}:{}",
                    field.name,
                    canonicalize_surface_text(workspace, resolved_module, &scope, &field.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => String::new(),
    };
    let mut rendered = format!("record:{}[{}]", symbol.name, symbol.type_params.join(","));
    if let Some(where_clause) = &symbol.where_clause {
        rendered.push_str("|where=");
        rendered.push_str(&canonicalize_surface_text(
            workspace,
            resolved_module,
            &scope,
            where_clause,
        ));
    }
    rendered.push_str("|fields=[");
    rendered.push_str(&fields);
    rendered.push(']');
    rendered
}

fn render_enum_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let scope = TypeScope::default().with_params(&symbol.type_params);
    let variants = match &symbol.body {
        HirSymbolBody::Enum { variants } => variants
            .iter()
            .map(|variant| match &variant.payload {
                Some(payload) => format!(
                    "{}({})",
                    variant.name,
                    canonicalize_surface_text(workspace, resolved_module, &scope, payload)
                ),
                None => variant.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => String::new(),
    };
    let mut rendered = format!("enum:{}[{}]", symbol.name, symbol.type_params.join(","));
    if let Some(where_clause) = &symbol.where_clause {
        rendered.push_str("|where=");
        rendered.push_str(&canonicalize_surface_text(
            workspace,
            resolved_module,
            &scope,
            where_clause,
        ));
    }
    rendered.push_str("|variants=[");
    rendered.push_str(&variants);
    rendered.push(']');
    rendered
}

fn render_trait_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let scope = TypeScope::default().with_params(&symbol.type_params);
    let mut rendered = format!("trait:{}[{}]", symbol.name, symbol.type_params.join(","));
    if let Some(where_clause) = &symbol.where_clause {
        rendered.push_str("|where=");
        rendered.push_str(&canonicalize_surface_text(
            workspace,
            resolved_module,
            &scope,
            where_clause,
        ));
    }
    if let HirSymbolBody::Trait {
        assoc_types,
        methods,
    } = &symbol.body
    {
        rendered.push_str("|assoc=[");
        rendered.push_str(
            &assoc_types
                .iter()
                .map(|assoc_type| match &assoc_type.default_ty {
                    Some(default_ty) => format!(
                        "{}={}",
                        assoc_type.name,
                        canonicalize_surface_text(workspace, resolved_module, &scope, default_ty)
                    ),
                    None => assoc_type.name.clone(),
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        rendered.push(']');
        let method_scope =
            scope.with_assoc_types(assoc_types.iter().map(|assoc_type| assoc_type.name.clone()));
        rendered.push_str("|methods=[");
        rendered.push_str(
            &methods
                .iter()
                .map(|method| {
                    render_callable_symbol_api_fingerprint(
                        workspace,
                        resolved_module,
                        method,
                        &method_scope,
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        rendered.push(']');
    }
    rendered
}

fn render_behavior_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let mut rendered = String::from("behavior[");
    rendered.push_str(
        &symbol
            .behavior_attrs
            .iter()
            .map(|attr| format!("{}={}", attr.name, attr.value))
            .collect::<Vec<_>>()
            .join(","),
    );
    rendered.push(']');
    rendered.push_str(&render_callable_symbol_api_fingerprint(
        workspace,
        resolved_module,
        symbol,
        &TypeScope::default(),
    ));
    rendered
}

fn render_impl_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    impl_decl: &HirImplDecl,
) -> String {
    let scope = TypeScope::default()
        .with_params(&impl_decl.type_params)
        .with_assoc_types(
            impl_decl
                .assoc_types
                .iter()
                .map(|assoc_type| assoc_type.name.clone()),
        )
        .with_self();
    let mut rendered = format!(
        "target={}",
        canonicalize_surface_text(workspace, resolved_module, &scope, &impl_decl.target_type)
    );
    if let Some(trait_path) = &impl_decl.trait_path {
        rendered.push_str("|trait=");
        rendered.push_str(&canonicalize_surface_text(
            workspace,
            resolved_module,
            &scope,
            trait_path,
        ));
    }
    rendered.push_str("|assoc=[");
    rendered.push_str(
        &impl_decl
            .assoc_types
            .iter()
            .map(|assoc_type| match &assoc_type.value_ty {
                Some(value_ty) => format!(
                    "{}={}",
                    assoc_type.name,
                    canonicalize_surface_text(workspace, resolved_module, &scope, value_ty)
                ),
                None => assoc_type.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(","),
    );
    rendered.push(']');
    rendered.push_str("|methods=[");
    rendered.push_str(
        &impl_decl
            .methods
            .iter()
            .map(|method| {
                render_callable_symbol_api_fingerprint(
                    workspace,
                    resolved_module,
                    method,
                    &scope.with_params(&method.type_params),
                )
            })
            .collect::<Vec<_>>()
            .join(","),
    );
    rendered.push(']');
    rendered
}

fn impl_decl_is_public(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    scope: &TypeScope,
    impl_decl: &HirImplDecl,
) -> bool {
    if !surface_text_is_public(
        package,
        resolved_module,
        workspace,
        scope,
        &impl_decl.target_type,
    ) {
        return false;
    }
    impl_decl.trait_path.as_ref().is_none_or(|trait_path| {
        surface_text_is_public(package, resolved_module, workspace, scope, trait_path)
    })
}
