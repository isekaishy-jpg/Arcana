use std::collections::{BTreeSet, HashMap};

use arcana_hir::{
    HirDirectiveKind, HirImplDecl, HirResolvedModule, HirResolvedPackage, HirResolvedTarget,
    HirResolvedWorkspace, HirSymbol, HirSymbolBody, HirSymbolKind, HirWorkspacePackage,
    HirWorkspaceSummary,
};
use arcana_syntax::is_builtin_type_name;
use sha2::{Digest, Sha256};

use crate::{PackageResult, WorkspaceGraph, WorkspaceMember};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberFingerprints {
    pub source: String,
    pub api: String,
}

impl MemberFingerprints {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn api(&self) -> &str {
        &self.api
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceFingerprints {
    snapshot_id: String,
    members: HashMap<String, MemberFingerprints>,
}

impl WorkspaceFingerprints {
    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }

    pub fn member(&self, name: &str) -> Option<&MemberFingerprints> {
        self.members.get(name)
    }

    pub fn get(&self, name: &str) -> Option<&MemberFingerprints> {
        self.member(name)
    }

    pub(crate) fn from_parts(
        snapshot_id: String,
        members: HashMap<String, MemberFingerprints>,
    ) -> Self {
        Self {
            snapshot_id,
            members,
        }
    }

    pub(crate) fn identity(&self) -> String {
        let mut names = self.members.keys().cloned().collect::<Vec<_>>();
        names.sort();

        let mut hasher = Sha256::new();
        hasher.update(b"arcana_workspace_fingerprints_v1\n");
        hasher.update(self.snapshot_id.as_bytes());
        hasher.update(b"\n");
        for name in names {
            let fingerprint = self
                .members
                .get(&name)
                .expect("fingerprint key should exist while hashing identity");
            hasher.update(format!("member={name}\n").as_bytes());
            hasher.update(format!("source={}\n", fingerprint.source).as_bytes());
            hasher.update(format!("api={}\n", fingerprint.api).as_bytes());
        }
        format!("sha256:{:x}", hasher.finalize())
    }
}

pub fn compute_workspace_fingerprints(
    graph: &WorkspaceGraph,
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
) -> PackageResult<WorkspaceFingerprints> {
    let mut fingerprints = HashMap::new();

    for member in &graph.members {
        let source = compute_member_source_fingerprint(member, workspace)?;
        let api = compute_resolved_api_fingerprint(member, workspace, resolved_workspace)?;
        fingerprints.insert(member.name.clone(), MemberFingerprints { source, api });
    }

    Ok(WorkspaceFingerprints::from_parts(
        compute_workspace_snapshot_id(graph, workspace)?,
        fingerprints,
    ))
}

pub fn compute_workspace_snapshot_id(
    graph: &WorkspaceGraph,
    workspace: &HirWorkspaceSummary,
) -> PackageResult<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_workspace_snapshot_v1\n");
    hasher.update(format!("root={}\n", graph.root_name).as_bytes());

    for member in &graph.members {
        hasher.update(format!("member={}\n", member.name).as_bytes());
        hasher.update(format!("kind={}\n", member.kind.as_str()).as_bytes());
        hasher.update(format!("rel_dir={}\n", member.rel_dir).as_bytes());
        for dep in &member.deps {
            hasher.update(format!("dep={dep}\n").as_bytes());
        }
    }

    for (name, package) in &workspace.packages {
        hasher.update(format!("package={name}\n").as_bytes());
        for dep in &package.direct_deps {
            hasher.update(format!("direct_dep={dep}\n").as_bytes());
        }
        for row in package.summary.hir_fingerprint_rows() {
            hasher.update(row.as_bytes());
            hasher.update(b"\n");
        }
    }

    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub(crate) fn package_uses_implicit_std(package: &HirWorkspacePackage) -> bool {
    package.summary.dependency_edges.iter().any(|edge| {
        edge.target_path
            .first()
            .is_some_and(|segment| segment == "std")
    })
}

fn compute_resolved_api_fingerprint(
    member: &WorkspaceMember,
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
) -> PackageResult<String> {
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
) -> PackageResult<String> {
    let package = workspace
        .package(&member.name)
        .ok_or_else(|| format!("package `{}` is not loaded in workspace HIR", member.name))?;
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_hir_member_v2\n");
    hasher.update(format!("name={}\n", member.name).as_bytes());
    hasher.update(format!("kind={}\n", member.kind.as_str()).as_bytes());
    for dep in &member.deps {
        hasher.update(format!("dep={dep}\n").as_bytes());
    }
    for row in package.summary.hir_fingerprint_rows() {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    if member.name != "std" && package_uses_implicit_std(package) {
        if let Some(std_package) = workspace.package("std") {
            hasher.update(b"implicit_std\n");
            for row in std_package.summary.hir_fingerprint_rows() {
                hasher.update(row.as_bytes());
                hasher.update(b"\n");
            }
        }
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn resolved_package_api_rows(
    package: &HirWorkspacePackage,
    resolved_package: &HirResolvedPackage,
    workspace: &HirWorkspaceSummary,
) -> PackageResult<Vec<String>> {
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
        HirSymbolKind::OpaqueType => {
            render_opaque_type_api_fingerprint(workspace, resolved_module, symbol)
        }
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

fn render_opaque_type_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let scope = TypeScope::default().with_params(&symbol.type_params);
    let mut rendered = String::new();
    rendered.push_str("opaque:");
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
    if let Some(policy) = symbol.opaque_policy {
        rendered.push_str("|ownership=");
        rendered.push_str(policy.ownership.as_str());
        rendered.push_str("|boundary=");
        rendered.push_str(policy.boundary.as_str());
    }
    rendered
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

#[derive(Clone, Debug, Default)]
struct TypeScope {
    type_params: BTreeSet<String>,
    lifetimes: BTreeSet<String>,
    assoc_types: BTreeSet<String>,
    allow_self: bool,
}

impl TypeScope {
    fn with_params(&self, params: &[String]) -> Self {
        let mut next = self.clone();
        for param in params {
            if param.starts_with('\'') {
                next.lifetimes.insert(param.clone());
            } else {
                next.type_params.insert(param.clone());
            }
        }
        next
    }

    fn with_assoc_types<I>(&self, assoc_types: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut next = self.clone();
        next.assoc_types.extend(assoc_types);
        next
    }

    fn with_self(&self) -> Self {
        let mut next = self.clone();
        next.allow_self = true;
        next
    }

    fn allows_type_name(&self, name: &str) -> bool {
        self.type_params.contains(name)
            || self.assoc_types.contains(name)
            || (self.allow_self && name == "Self")
    }
}

#[derive(Clone, Debug, Default)]
struct SurfaceRefs {
    paths: Vec<Vec<String>>,
}

enum ParsedSurfaceToken {
    Text(String),
    Lifetime(String),
    Path(Vec<String>),
}

struct ParsedSurface {
    tokens: Vec<ParsedSurfaceToken>,
    refs: SurfaceRefs,
}

fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('.') {
        let segment = segment.trim();
        if segment.is_empty() {
            return None;
        }
        let mut chars = segment.chars();
        let first = chars.next()?;
        if !is_ident_start(first) || !chars.all(is_ident_continue) {
            return None;
        }
        segments.push(segment.to_string());
    }

    (!segments.is_empty()).then_some(segments)
}

fn canonicalize_surface_text(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    text: &str,
) -> String {
    let parsed = parse_surface_text(text);
    let mut out = String::new();
    for token in parsed.tokens {
        match token {
            ParsedSurfaceToken::Text(text) | ParsedSurfaceToken::Lifetime(text) => {
                out.push_str(&text);
            }
            ParsedSurfaceToken::Path(path) => out.push_str(&canonicalize_surface_path(
                workspace,
                resolved_module,
                scope,
                &path,
            )),
        }
    }
    out
}

fn canonicalize_surface_path(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    path: &[String],
) -> String {
    if path.len() == 1 && (scope.allows_type_name(&path[0]) || is_builtin_type_name(&path[0])) {
        return path[0].clone();
    }
    if let Some(symbol_ref) = arcana_hir::lookup_symbol_path(workspace, resolved_module, path) {
        return format!("{}.{}", symbol_ref.module_id, symbol_ref.symbol.name);
    }
    path.join(".")
}

fn surface_text_is_public(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    scope: &TypeScope,
    text: &str,
) -> bool {
    let refs = parse_surface_text(text).refs;
    if refs.paths.is_empty() {
        return true;
    }
    for path in refs.paths {
        if path.len() == 1 && (scope.allows_type_name(&path[0]) || is_builtin_type_name(&path[0])) {
            continue;
        }
        let Some(symbol_ref) = arcana_hir::lookup_symbol_path(workspace, resolved_module, &path)
        else {
            return false;
        };
        if symbol_ref.package_name == package.summary.package_name && !symbol_ref.symbol.exported {
            return false;
        }
    }
    true
}

fn parse_surface_text(text: &str) -> ParsedSurface {
    let chars = text.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut refs = SurfaceRefs::default();
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '\'' {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            let lifetime = chars[start..index].iter().collect::<String>();
            tokens.push(ParsedSurfaceToken::Lifetime(lifetime));
            continue;
        }
        if is_ident_start(ch) && is_projection_tail(&chars, index) {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            tokens.push(ParsedSurfaceToken::Text(
                chars[start..index].iter().collect::<String>(),
            ));
            continue;
        }
        if is_ident_start(ch) {
            let (end, token) = parse_surface_ident(&chars, index);
            match token {
                ParsedSurfaceToken::Path(path) => {
                    refs.paths.push(path.clone());
                    tokens.push(ParsedSurfaceToken::Path(path));
                }
                token => tokens.push(token),
            }
            index = end;
            continue;
        }
        tokens.push(ParsedSurfaceToken::Text(ch.to_string()));
        index += 1;
    }

    ParsedSurface { tokens, refs }
}

fn parse_surface_ident(chars: &[char], start: usize) -> (usize, ParsedSurfaceToken) {
    let mut end = start;
    let mut segments = Vec::new();
    let mut keyword = None::<String>;
    loop {
        let segment_start = end;
        end += 1;
        while end < chars.len() && is_ident_continue(chars[end]) {
            end += 1;
        }
        let segment = chars[segment_start..end].iter().collect::<String>();
        if is_surface_keyword(&segment) {
            keyword = Some(segment);
            segments.clear();
            break;
        }
        segments.push(segment);

        let Some(dot_idx) = next_non_ws_index(chars, end) else {
            break;
        };
        if chars[dot_idx] != '.' {
            break;
        }
        let Some(next_idx) = next_non_ws_index(chars, dot_idx + 1) else {
            break;
        };
        if !is_ident_start(chars[next_idx]) {
            break;
        }
        end = next_idx;
    }

    if let Some(keyword) = keyword {
        return (end, ParsedSurfaceToken::Text(keyword));
    }
    if !segments.is_empty() {
        return (end, ParsedSurfaceToken::Path(segments));
    }
    (
        end,
        ParsedSurfaceToken::Text(chars[start..end].iter().collect::<String>()),
    )
}

fn is_projection_tail(chars: &[char], index: usize) -> bool {
    matches!(
        chars.get(index + 1..index + 3),
        Some([first, second]) if *first == ':' && *second == ':'
    )
}

fn next_non_ws_index(chars: &[char], mut index: usize) -> Option<usize> {
    while index < chars.len() {
        if !chars[index].is_whitespace() {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_surface_keyword(segment: &str) -> bool {
    matches!(segment, "mut" | "read" | "take" | "edit")
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
