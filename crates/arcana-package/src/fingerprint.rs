use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use arcana_hir::{
    HirDirectiveKind, HirImplDecl, HirPredicate, HirResolvedModule, HirResolvedPackage,
    HirResolvedTarget, HirResolvedWorkspace, HirSymbol, HirSymbolBody, HirSymbolKind, HirTraitRef,
    HirType, HirTypeKind, HirWhereClause, HirWorkspacePackage, HirWorkspaceSummary,
    collect_hir_type_refs, render_expr_fingerprint, render_symbol_fingerprint,
};
use arcana_syntax::is_builtin_type_name;
use sha2::{Digest, Sha256};

use crate::{CACHE_DIR, PackageResult, WorkspaceGraph, WorkspaceMember};

fn quote_fingerprint_text(text: impl ToString) -> String {
    let escaped = text.to_string().replace('\\', "\\\\").replace('|', "\\|");
    escaped.replace('[', "\\[").replace(']', "\\]")
}

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
    let asset_rows = collect_workspace_asset_rows(graph)?;
    let mut fingerprints = HashMap::new();

    for member in &graph.members {
        let source = compute_member_source_fingerprint(member, workspace, &asset_rows)?;
        let api = compute_resolved_api_fingerprint(member, workspace, resolved_workspace)?;
        fingerprints.insert(
            member.package_id.clone(),
            MemberFingerprints { source, api },
        );
    }

    Ok(WorkspaceFingerprints::from_parts(
        compute_workspace_snapshot_id_with_asset_rows(graph, workspace, &asset_rows)?,
        fingerprints,
    ))
}

pub fn compute_workspace_snapshot_id(
    graph: &WorkspaceGraph,
    workspace: &HirWorkspaceSummary,
) -> PackageResult<String> {
    let asset_rows = collect_workspace_asset_rows(graph)?;
    compute_workspace_snapshot_id_with_asset_rows(graph, workspace, &asset_rows)
}

fn compute_workspace_snapshot_id_with_asset_rows(
    graph: &WorkspaceGraph,
    workspace: &HirWorkspaceSummary,
    asset_rows: &HashMap<String, Vec<String>>,
) -> PackageResult<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_workspace_snapshot_v1\n");
    hasher.update(format!("root={}\n", graph.root_id).as_bytes());

    for member in &graph.members {
        hasher.update(format!("member={}\n", member.package_id).as_bytes());
        hasher.update(format!("name={}\n", member.name).as_bytes());
        hasher.update(format!("kind={}\n", member.kind.as_str()).as_bytes());
        hasher.update(format!("source={:?}\n", member.source_kind).as_bytes());
        hasher.update(format!("rel_dir={}\n", member.rel_dir).as_bytes());
        if let Some(version) = &member.version {
            hasher.update(format!("version={version}\n").as_bytes());
        }
        for dep in &member.deps {
            hasher.update(format!("dep={dep}\n").as_bytes());
        }
        for row in render_member_dependency_setting_rows(member) {
            hasher.update(row.as_bytes());
            hasher.update(b"\n");
        }
        for row in render_member_native_product_rows(member)? {
            hasher.update(row.as_bytes());
            hasher.update(b"\n");
        }
        for row in render_member_foreword_product_rows(member)? {
            hasher.update(row.as_bytes());
            hasher.update(b"\n");
        }
        for row in member_asset_rows(asset_rows, member) {
            hasher.update(row.as_bytes());
            hasher.update(b"\n");
        }
    }

    for (package_id, package) in &workspace.packages {
        hasher.update(format!("package={package_id}\n").as_bytes());
        hasher.update(format!("package_name={}\n", package.summary.package_name).as_bytes());
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
    let package = workspace.package_by_id(&member.package_id).ok_or_else(|| {
        format!(
            "package `{}` is not loaded in workspace HIR",
            member.package_id
        )
    })?;
    let resolved_package = resolved_workspace
        .package_by_id(&member.package_id)
        .ok_or_else(|| format!("resolved package `{}` is not loaded", member.package_id))?;

    let mut hasher = Sha256::new();
    hasher.update(b"arcana_resolved_api_v1\n");
    hasher.update(format!("package_id={}\n", member.package_id).as_bytes());
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
    asset_rows: &HashMap<String, Vec<String>>,
) -> PackageResult<String> {
    let package = workspace.package_by_id(&member.package_id).ok_or_else(|| {
        format!(
            "package `{}` is not loaded in workspace HIR",
            member.package_id
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_hir_member_v2\n");
    hasher.update(format!("package_id={}\n", member.package_id).as_bytes());
    hasher.update(format!("name={}\n", member.name).as_bytes());
    hasher.update(format!("kind={}\n", member.kind.as_str()).as_bytes());
    for dep in &member.deps {
        hasher.update(format!("dep={dep}\n").as_bytes());
    }
    for row in render_member_dependency_setting_rows(member) {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    for row in render_member_native_product_rows(member)? {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    for row in render_member_foreword_product_rows(member)? {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    for row in member_asset_rows(asset_rows, member) {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    for row in package.summary.hir_fingerprint_rows() {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    if member.name != "std"
        && package_uses_implicit_std(package)
        && let Some(std_package) = workspace.package("std")
    {
        hasher.update(b"implicit_std\n");
        for row in std_package.summary.hir_fingerprint_rows() {
            hasher.update(row.as_bytes());
            hasher.update(b"\n");
        }
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn render_member_dependency_setting_rows(member: &WorkspaceMember) -> Vec<String> {
    let mut rows = member
        .direct_dep_specs
        .iter()
        .map(|(alias, spec)| {
            let package_name = member
                .direct_dep_packages
                .get(alias)
                .map(String::as_str)
                .unwrap_or("");
            let package_id = member
                .direct_dep_ids
                .get(alias)
                .map(String::as_str)
                .unwrap_or("");
            let mut native_plugins = spec.native_plugins.clone();
            native_plugins.sort();
            format!(
                "dep_setting:alias={alias}|package={package_name}|package_id={package_id}|source={:?}|source_label={}|native_delivery={}|native_child={}|native_provider={}|native_plugins={}|executable_forewords={}",
                spec.source.kind(),
                spec.source.location_label(),
                spec.native_delivery.as_str(),
                spec.native_child.as_deref().unwrap_or(""),
                spec.native_provider.as_deref().unwrap_or(""),
                native_plugins.join(","),
                spec.executable_forewords
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

fn collect_workspace_asset_rows(
    graph: &WorkspaceGraph,
) -> PackageResult<HashMap<String, Vec<String>>> {
    let mut cache = PackageAssetFingerprintCache::new(&graph.root_dir);
    let mut asset_rows = HashMap::new();
    for member in &graph.members {
        asset_rows.insert(
            member.package_id.clone(),
            render_member_package_asset_rows(member, &mut cache)?,
        );
    }
    Ok(asset_rows)
}

fn member_asset_rows<'a>(
    asset_rows: &'a HashMap<String, Vec<String>>,
    member: &WorkspaceMember,
) -> &'a [String] {
    asset_rows
        .get(&member.package_id)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn render_member_package_asset_rows(
    member: &WorkspaceMember,
    cache: &mut PackageAssetFingerprintCache,
) -> PackageResult<Vec<String>> {
    let assets_dir = member.abs_dir.join("assets");
    if !assets_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    collect_member_package_asset_rows(&assets_dir, &assets_dir, member, cache, &mut rows)?;
    rows.sort();
    Ok(rows)
}

fn collect_member_package_asset_rows(
    dir: &Path,
    root: &Path,
    member: &WorkspaceMember,
    cache: &mut PackageAssetFingerprintCache,
    out: &mut Vec<String>,
) -> PackageResult<()> {
    let mut entries = fs::read_dir(dir)
        .map_err(|e| format!("failed to read asset directory `{}`: {e}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            format!(
                "failed to read asset directory entry `{}`: {e}",
                dir.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_member_package_asset_rows(&path, root, member, cache, out)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|e| format!("failed to relativize asset `{}`: {e}", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        out.push(cache.asset_row(member, &path, &relative)?);
    }
    Ok(())
}

struct PackageAssetFingerprintCache {
    cache_root: PathBuf,
}

impl PackageAssetFingerprintCache {
    fn new(workspace_root: &Path) -> Self {
        Self {
            cache_root: workspace_root.join(CACHE_DIR).join("asset-fingerprints-v1"),
        }
    }

    fn asset_row(
        &mut self,
        member: &WorkspaceMember,
        path: &Path,
        relative: &str,
    ) -> PackageResult<String> {
        let metadata = fs::metadata(path)
            .map_err(|e| format!("failed to read asset metadata `{}`: {e}", path.display()))?;
        let length = metadata.len();
        let modified_unix_nanos = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_nanos()).ok());
        if let Some(modified_unix_nanos) = modified_unix_nanos
            && let Some(row) = self.read_cached_row(member, relative, length, modified_unix_nanos)
        {
            return Ok(row);
        }
        let row = compute_asset_row(path, relative)?;
        if let Some(modified_unix_nanos) = modified_unix_nanos {
            self.write_cached_row(member, relative, length, modified_unix_nanos, &row);
        }
        Ok(row)
    }

    fn read_cached_row(
        &self,
        member: &WorkspaceMember,
        relative: &str,
        length: u64,
        modified_unix_nanos: i64,
    ) -> Option<String> {
        let cache_path = self.cache_path(member, relative);
        let text = fs::read_to_string(cache_path).ok()?;
        let value = text.parse::<toml::Value>().ok()?;
        let table = value.as_table()?;
        if table.get("version").and_then(toml::Value::as_integer) != Some(1) {
            return None;
        }
        if table.get("relative").and_then(toml::Value::as_str) != Some(relative) {
            return None;
        }
        if table.get("length").and_then(toml::Value::as_integer) != Some(length as i64) {
            return None;
        }
        if table
            .get("modified_unix_nanos")
            .and_then(toml::Value::as_integer)
            != Some(modified_unix_nanos)
        {
            return None;
        }
        table
            .get("row")
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    }

    fn write_cached_row(
        &self,
        member: &WorkspaceMember,
        relative: &str,
        length: u64,
        modified_unix_nanos: i64,
        row: &str,
    ) {
        let cache_path = self.cache_path(member, relative);
        let Some(parent) = cache_path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let rendered = format!(
            concat!(
                "version = 1\n",
                "relative = \"{}\"\n",
                "length = {}\n",
                "modified_unix_nanos = {}\n",
                "row = \"{}\"\n",
            ),
            escape_cache_text(relative),
            length,
            modified_unix_nanos,
            escape_cache_text(row),
        );
        let _ = fs::write(cache_path, rendered);
    }

    fn cache_path(&self, member: &WorkspaceMember, relative: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(b"arcana_package_asset_cache_key_v1\n");
        hasher.update(member.package_id.as_bytes());
        hasher.update(b"\n");
        hasher.update(relative.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        self.cache_root
            .join(&digest[0..2])
            .join(format!("{digest}.toml"))
    }
}

fn compute_asset_row(path: &Path, relative: &str) -> PackageResult<String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("failed to read asset file `{}`: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_package_asset_v1\n");
    hasher.update(relative.as_bytes());
    hasher.update(b"\n");
    hasher.update(&bytes);
    Ok(format!(
        "asset:{}:{:x}",
        quote_fingerprint_text(relative),
        hasher.finalize()
    ))
}

fn escape_cache_text(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn render_member_native_product_rows(member: &WorkspaceMember) -> PackageResult<Vec<String>> {
    let mut rows = Vec::new();
    for (name, product) in &member.native_products {
        rows.push(format!(
            "native_product:name={name}|kind={}|role={}|producer={}|file={}|contract={}|rust_cdylib_crate={}|provider_dir={}",
            product.kind,
            product.role.as_str(),
            product.producer.as_str(),
            product.file,
            product.contract,
            product.rust_cdylib_crate.as_deref().unwrap_or(""),
            product.provider_dir.as_deref().unwrap_or("")
        ));
        for sidecar in &product.sidecars {
            rows.push(format!(
                "native_product_sidecar:name={name}|path={sidecar}|hash={}",
                render_native_product_sidecar_hash(member, sidecar)?
            ));
        }
    }
    rows.sort();
    Ok(rows)
}

fn render_member_foreword_product_rows(member: &WorkspaceMember) -> PackageResult<Vec<String>> {
    let mut rows = Vec::new();
    for (name, product) in &member.foreword_products {
        rows.push(format!(
            "foreword_product:name={name}|path={}|runner={}|args={}|hash={}",
            product.path,
            product.runner.as_deref().unwrap_or(""),
            product.args.join(","),
            render_foreword_product_hash(member, product)?
        ));
    }
    rows.sort();
    Ok(rows)
}

fn render_foreword_product_hash(
    member: &WorkspaceMember,
    product: &crate::ForewordAdapterProductSpec,
) -> PackageResult<String> {
    let product_path = member.abs_dir.join(&product.path);
    match fs::read(&product_path) {
        Ok(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(b"arcana_foreword_product_v1\n");
            hasher.update(product.path.as_bytes());
            hasher.update(b"\n");
            if let Some(runner) = &product.runner {
                hasher.update(runner.as_bytes());
            }
            hasher.update(b"\n");
            for arg in &product.args {
                hasher.update(arg.as_bytes());
                hasher.update(b"\n");
            }
            hasher.update(&bytes);
            Ok(format!("sha256:{:x}", hasher.finalize()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok("missing".to_string()),
        Err(err) => Err(format!(
            "failed to read foreword product `{}` for `{}`: {err}",
            product_path.display(),
            member.name
        )),
    }
}

fn render_native_product_sidecar_hash(
    member: &WorkspaceMember,
    relative_path: &str,
) -> PackageResult<String> {
    let sidecar_path = member.abs_dir.join(relative_path);
    match fs::read(&sidecar_path) {
        Ok(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(b"arcana_native_product_sidecar_v1\n");
            hasher.update(relative_path.as_bytes());
            hasher.update(b"\n");
            hasher.update(&bytes);
            Ok(format!("sha256:{:x}", hasher.finalize()))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok("missing".to_string()),
        Err(err) => Err(format!(
            "failed to read native product sidecar `{}` for `{}`: {err}",
            sidecar_path.display(),
            member.name
        )),
    }
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

    for entry in &module.emitted_foreword_metadata {
        if entry.public {
            rows.push(format!(
                "emitted_foreword:{}",
                render_emitted_foreword_api_fingerprint(workspace, resolved_module, entry)
            ));
        }
    }
    for row in &module.foreword_registrations {
        if row.public {
            rows.push(format!(
                "foreword_registration:{}",
                render_foreword_registration_api_fingerprint(workspace, resolved_module, row)
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
        HirSymbolKind::Object => render_object_api_fingerprint(workspace, resolved_module, symbol),
        HirSymbolKind::Owner => render_owner_api_fingerprint(workspace, resolved_module, symbol),
        HirSymbolKind::Enum => render_enum_api_fingerprint(workspace, resolved_module, symbol),
        HirSymbolKind::OpaqueType => {
            render_opaque_type_api_fingerprint(workspace, resolved_module, symbol)
        }
        HirSymbolKind::Trait => render_trait_api_fingerprint(workspace, resolved_module, symbol),
        HirSymbolKind::Behavior => {
            render_behavior_api_fingerprint(workspace, resolved_module, symbol)
        }
        HirSymbolKind::Const => format!("const:{}", render_symbol_fingerprint(symbol)),
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
        rendered.push_str(&canonicalize_where_clause(
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
        rendered.push_str(&canonicalize_where_clause(
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
                part.push_str(&canonicalize_surface_type(
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
        rendered.push_str(&canonicalize_surface_type(
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
    if !symbol.availability.is_empty() {
        base.push_str("|availability=[");
        base.push_str(
            &symbol
                .availability
                .iter()
                .map(|attachment| {
                    canonicalize_surface_path(
                        workspace,
                        resolved_module,
                        &TypeScope::default(),
                        &attachment.path,
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        base.push(']');
    }
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
    if let Some(generated_by) = &symbol.generated_by {
        base.push_str("|generated_by=");
        base.push_str(&render_generated_by_api_fingerprint(
            workspace,
            resolved_module,
            generated_by,
        ));
    }
    if let Some(generated_name_key) = &symbol.generated_name_key {
        base.push_str("|generated_name_key=");
        base.push_str(&quote_fingerprint_text(generated_name_key));
    }
    base
}

fn render_generated_by_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    generated_by: &arcana_hir::HirGeneratedByForeword,
) -> String {
    format!(
        "generated(applied={}|resolved={}|provider={}|owner_kind={}|owner_path={}|retention={}|args=[{}])",
        quote_fingerprint_text(&generated_by.applied_name),
        quote_fingerprint_text(&generated_by.resolved_name),
        quote_fingerprint_text(&generated_by.provider_package_id),
        quote_fingerprint_text(&generated_by.owner_kind),
        quote_fingerprint_text(&generated_by.owner_path),
        generated_by.retention.as_str(),
        generated_by
            .args
            .iter()
            .map(|arg| match &arg.name {
                Some(name) => format!(
                    "{name}={}",
                    quote_fingerprint_text(canonicalize_foreword_arg_value(
                        workspace,
                        resolved_module,
                        &arg.value,
                    ))
                ),
                None => quote_fingerprint_text(canonicalize_foreword_arg_value(
                    workspace,
                    resolved_module,
                    &arg.value,
                )),
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_emitted_foreword_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    entry: &arcana_hir::HirEmittedForewordMetadata,
) -> String {
    format!(
        "name={}|target_kind={}|target_path={}|retention={}|args=[{}]|generated_by={}",
        quote_fingerprint_text(&entry.qualified_name),
        quote_fingerprint_text(&entry.target_kind),
        quote_fingerprint_text(&entry.target_path),
        entry.retention.as_str(),
        entry
            .args
            .iter()
            .map(|arg| match &arg.name {
                Some(name) => format!(
                    "{name}={}",
                    quote_fingerprint_text(canonicalize_foreword_arg_value(
                        workspace,
                        resolved_module,
                        &arg.value,
                    ))
                ),
                None => quote_fingerprint_text(canonicalize_foreword_arg_value(
                    workspace,
                    resolved_module,
                    &arg.value,
                )),
            })
            .collect::<Vec<_>>()
            .join(","),
        render_generated_by_api_fingerprint(workspace, resolved_module, &entry.generated_by)
    )
}

fn render_foreword_registration_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    row: &arcana_hir::HirForewordRegistrationRow,
) -> String {
    format!(
        "namespace={}|key={}|value={}|target_kind={}|target_path={}|generated_by={}",
        quote_fingerprint_text(&row.namespace),
        quote_fingerprint_text(&row.key),
        quote_fingerprint_text(&row.value),
        quote_fingerprint_text(&row.target_kind),
        quote_fingerprint_text(&row.target_path),
        render_generated_by_api_fingerprint(workspace, resolved_module, &row.generated_by)
    )
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
                    canonicalize_surface_type(workspace, resolved_module, &scope, &field.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => String::new(),
    };
    let mut rendered = format!("record:{}[{}]", symbol.name, symbol.type_params.join(","));
    if let Some(where_clause) = &symbol.where_clause {
        rendered.push_str("|where=");
        rendered.push_str(&canonicalize_where_clause(
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

fn render_object_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let scope = TypeScope::default().with_params(&symbol.type_params);
    let mut rendered = format!("object:{}[{}]", symbol.name, symbol.type_params.join(","));
    if let Some(where_clause) = &symbol.where_clause {
        rendered.push_str("|where=");
        rendered.push_str(&canonicalize_where_clause(
            workspace,
            resolved_module,
            &scope,
            where_clause,
        ));
    }
    if let HirSymbolBody::Object { fields, methods } = &symbol.body {
        rendered.push_str("|fields=[");
        rendered.push_str(
            &fields
                .iter()
                .map(|field| {
                    format!(
                        "{}:{}",
                        field.name,
                        canonicalize_surface_type(workspace, resolved_module, &scope, &field.ty)
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        rendered.push(']');
        rendered.push_str("|methods=[");
        rendered.push_str(
            &methods
                .iter()
                .map(|method| {
                    render_callable_symbol_api_fingerprint(
                        workspace,
                        resolved_module,
                        method,
                        &scope,
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        rendered.push(']');
    }
    rendered
}

fn render_owner_api_fingerprint(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    symbol: &HirSymbol,
) -> String {
    let mut rendered = format!("owner:{}", symbol.name);
    if let HirSymbolBody::Owner { objects, exits, .. } = &symbol.body {
        rendered.push_str("|objects=[");
        rendered.push_str(
            &objects
                .iter()
                .map(|object| {
                    let lifecycle = arcana_hir::lookup_symbol_path(
                        workspace,
                        resolved_module,
                        &object.type_path,
                    )
                    .and_then(|resolved_object| match &resolved_object.symbol.body {
                        HirSymbolBody::Object { methods, .. } => {
                            let hooks = methods
                                .iter()
                                .filter(|method| matches!(method.name.as_str(), "init" | "resume"))
                                .map(render_symbol_fingerprint)
                                .collect::<Vec<_>>();
                            (!hooks.is_empty()).then(|| format!(":[{}]", hooks.join(",")))
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                    format!(
                        "{}:{}{}",
                        object.local_name,
                        canonicalize_surface_path(
                            workspace,
                            resolved_module,
                            &TypeScope::default(),
                            &object.type_path,
                        ),
                        lifecycle
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        rendered.push(']');
        rendered.push_str("|exits=[");
        rendered.push_str(
            &exits
                .iter()
                .map(|owner_exit| {
                    format!(
                        "{}:{}:[{}]",
                        owner_exit.name,
                        render_expr_fingerprint(&owner_exit.condition),
                        owner_exit.holds.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(","),
        );
        rendered.push(']');
    }
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
                    canonicalize_surface_type(workspace, resolved_module, &scope, payload)
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
        rendered.push_str(&canonicalize_where_clause(
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
        rendered.push_str(&canonicalize_where_clause(
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
                        canonicalize_surface_type(workspace, resolved_module, &scope, default_ty)
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
        canonicalize_surface_type(workspace, resolved_module, &scope, &impl_decl.target_type)
    );
    if let Some(trait_path) = &impl_decl.trait_path {
        rendered.push_str("|trait=");
        rendered.push_str(&canonicalize_surface_trait_ref(
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
                    canonicalize_surface_type(workspace, resolved_module, &scope, value_ty)
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
    if let Some(generated_by) = &impl_decl.generated_by {
        rendered.push_str("|generated_by=");
        rendered.push_str(&render_generated_by_api_fingerprint(
            workspace,
            resolved_module,
            generated_by,
        ));
    }
    if let Some(generated_name_key) = &impl_decl.generated_name_key {
        rendered.push_str("|generated_name_key=");
        rendered.push_str(&quote_fingerprint_text(generated_name_key));
    }
    rendered
}

fn impl_decl_is_public(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    scope: &TypeScope,
    impl_decl: &HirImplDecl,
) -> bool {
    if !surface_type_is_public(
        package,
        resolved_module,
        workspace,
        scope,
        &impl_decl.target_type,
    ) {
        return false;
    }
    impl_decl.trait_path.as_ref().is_none_or(|trait_path| {
        surface_trait_ref_is_public(package, resolved_module, workspace, scope, trait_path)
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

fn canonicalize_surface_type(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    ty: &HirType,
) -> String {
    match &ty.kind {
        HirTypeKind::Path(path) => {
            canonicalize_surface_path(workspace, resolved_module, scope, &path.segments)
        }
        HirTypeKind::Apply { base, args } => format!(
            "{}[{}]",
            canonicalize_surface_path(workspace, resolved_module, scope, &base.segments),
            args.iter()
                .map(|arg| canonicalize_surface_type(workspace, resolved_module, scope, arg))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        HirTypeKind::Ref {
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
            rendered.push_str(&canonicalize_surface_type(
                workspace,
                resolved_module,
                scope,
                inner,
            ));
            rendered
        }
        HirTypeKind::Tuple(items) => format!(
            "({})",
            items
                .iter()
                .map(|item| canonicalize_surface_type(workspace, resolved_module, scope, item))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        HirTypeKind::Projection(projection) => format!(
            "{}.{}",
            canonicalize_surface_trait_ref(
                workspace,
                resolved_module,
                scope,
                &projection.trait_ref
            ),
            projection.assoc
        ),
    }
}

fn canonicalize_surface_trait_ref(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    trait_ref: &HirTraitRef,
) -> String {
    let base =
        canonicalize_surface_path(workspace, resolved_module, scope, &trait_ref.path.segments);
    if trait_ref.args.is_empty() {
        base
    } else {
        format!(
            "{}[{}]",
            base,
            trait_ref
                .args
                .iter()
                .map(|arg| canonicalize_surface_type(workspace, resolved_module, scope, arg))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn canonicalize_where_clause(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    where_clause: &HirWhereClause,
) -> String {
    where_clause
        .predicates
        .iter()
        .map(|predicate| canonicalize_predicate(workspace, resolved_module, scope, predicate))
        .collect::<Vec<_>>()
        .join(", ")
}

fn canonicalize_predicate(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    predicate: &HirPredicate,
) -> String {
    match predicate {
        HirPredicate::TraitBound { trait_ref, .. } => {
            canonicalize_surface_trait_ref(workspace, resolved_module, scope, trait_ref)
        }
        HirPredicate::ProjectionEq {
            projection, value, ..
        } => format!(
            "{}.{} = {}",
            canonicalize_surface_trait_ref(
                workspace,
                resolved_module,
                scope,
                &projection.trait_ref
            ),
            projection.assoc,
            canonicalize_surface_type(workspace, resolved_module, scope, value)
        ),
        HirPredicate::LifetimeOutlives {
            longer, shorter, ..
        } => format!("{}: {}", longer.render(), shorter.render()),
        HirPredicate::TypeOutlives { ty, lifetime, .. } => format!(
            "{}: {}",
            canonicalize_surface_type(workspace, resolved_module, scope, ty),
            lifetime.render()
        ),
    }
}

fn surface_type_is_public(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    scope: &TypeScope,
    ty: &HirType,
) -> bool {
    surface_refs_are_public(
        package,
        resolved_module,
        workspace,
        scope,
        &collect_hir_type_refs(ty).paths,
    )
}

fn surface_trait_ref_is_public(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    scope: &TypeScope,
    trait_ref: &HirTraitRef,
) -> bool {
    let mut refs = arcana_hir::HirSurfaceRefs::default();
    trait_ref.collect_refs(&mut refs);
    surface_refs_are_public(package, resolved_module, workspace, scope, &refs.paths)
}

fn surface_refs_are_public(
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    workspace: &HirWorkspaceSummary,
    scope: &TypeScope,
    paths: &[Vec<String>],
) -> bool {
    for path in paths {
        if path.len() == 1 && (scope.allows_type_name(&path[0]) || is_builtin_type_name(&path[0])) {
            continue;
        }
        let Some(symbol_ref) = arcana_hir::lookup_symbol_path(workspace, resolved_module, path)
        else {
            return false;
        };
        if symbol_ref.package_name == package.summary.package_name && !symbol_ref.symbol.exported {
            return false;
        }
    }
    true
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
