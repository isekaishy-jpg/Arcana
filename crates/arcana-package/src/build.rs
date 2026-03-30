use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use arcana_aot::{
    ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV, AotEmissionFile, AotEmitContext, AotEmitTarget,
    AotNativeProduct, AotPackageEmission, AotRuntimeBinding, compile_package,
    emit_package_with_context, render_package_artifact, validate_package_artifact,
};
use arcana_cabi::{
    ARCANA_CABI_CONTRACT_VERSION_V1, ARCANA_CABI_EXPORT_CONTRACT_ID, ArcanaCabiProductRole,
};
use arcana_hir::{HirResolvedWorkspace, HirWorkspaceSummary, resolve_workspace};
use arcana_ir::{
    IrPackage, RuntimeRequirementRoots, derive_runtime_requirements_with_roots,
    disambiguate_package_routine_keys, lower_workspace_package_with_resolution,
    render_routine_signature_text,
};

use crate::build_identity::{
    cache_metadata_path_for_output, cached_artifact_matches_status, cached_emission_hash,
    cached_emission_hash_for_path, current_build_toolchain_for_target_with_context,
    render_cached_artifact,
};
use crate::distribution::resolve_native_product_files;
use crate::fingerprint::{
    WorkspaceFingerprints, compute_workspace_fingerprints, package_uses_implicit_std,
};
use crate::{
    ARTIFACT_DIR, BuildOutputKey, BuildTarget, CACHE_DIR, GrimoireKind, LOCKFILE_VERSION, LOGS_DIR,
    LockNativeProductEntry, LockTargetEntry, Lockfile, NativeProductProducer, NativeProductSpec,
    PackageResult, WorkspaceGraph, collect_validated_support_file_paths,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildDisposition {
    Built,
    CacheHit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildProgress<'a> {
    pub index: usize,
    pub total: usize,
    pub status: &'a BuildStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildStatus {
    pub(crate) member: String,
    pub(crate) member_name: String,
    pub(crate) member_label: String,
    pub(crate) disposition: BuildDisposition,
    pub(crate) snapshot_id: String,
    pub(crate) fingerprint_set_id: String,
    pub(crate) fingerprint: String,
    pub(crate) api_fingerprint: String,
    pub(crate) build_key: BuildOutputKey,
    pub(crate) target: BuildTarget,
    pub(crate) product: Option<String>,
    pub(crate) artifact_rel_path: String,
    pub(crate) kind: GrimoireKind,
    pub(crate) format: String,
    pub(crate) toolchain: String,
    pub(crate) native_product_closure: Option<String>,
}

impl BuildStatus {
    pub fn member(&self) -> &str {
        &self.member
    }

    pub fn member_name(&self) -> &str {
        &self.member_name
    }

    pub fn member_label(&self) -> &str {
        &self.member_label
    }

    pub fn disposition(&self) -> BuildDisposition {
        self.disposition
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn target(&self) -> &BuildTarget {
        &self.target
    }

    pub fn product(&self) -> Option<&str> {
        self.product.as_deref()
    }

    pub fn build_key(&self) -> &BuildOutputKey {
        &self.build_key
    }

    pub fn artifact_rel_path(&self) -> &str {
        &self.artifact_rel_path
    }
}

#[derive(Debug)]
pub struct PreparedBuild {
    pub(crate) snapshot_id: String,
    pub(crate) fingerprint_set_id: String,
    pub(crate) fingerprints: WorkspaceFingerprints,
    pub(crate) workspace: HirWorkspaceSummary,
    pub(crate) lowered_packages: BTreeMap<String, IrPackage>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildExecutionContext {
    pub selected_product: Option<String>,
}

impl BuildExecutionContext {
    pub fn with_selected_product(selected_product: Option<String>) -> Self {
        Self { selected_product }
    }

    pub fn selected_product(&self) -> Option<&str> {
        self.selected_product.as_deref()
    }
}

pub fn prepare_build(graph: &WorkspaceGraph) -> PackageResult<PreparedBuild> {
    let workspace = crate::load_workspace_hir_from_graph(&graph.root_dir, graph)?;
    let resolved_workspace = resolve_workspace(&workspace).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| {
                format!(
                    "{}:{}:{}: {}",
                    error.source_module_id, error.span.line, error.span.column, error.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    prepare_build_from_workspace(graph, workspace, resolved_workspace)
}

pub fn prepare_build_from_workspace(
    graph: &WorkspaceGraph,
    workspace: HirWorkspaceSummary,
    resolved_workspace: HirResolvedWorkspace,
) -> PackageResult<PreparedBuild> {
    let fingerprints = compute_workspace_fingerprints(graph, &workspace, &resolved_workspace)?;
    let lowered_packages = workspace
        .packages
        .values()
        .map(|package| {
            Ok((
                package.package_id.clone(),
                lower_workspace_package_with_resolution(&workspace, &resolved_workspace, package)?,
            ))
        })
        .collect::<PackageResult<BTreeMap<_, _>>>()?;
    Ok(PreparedBuild {
        snapshot_id: fingerprints.snapshot_id().to_string(),
        fingerprint_set_id: fingerprints.identity(),
        fingerprints,
        workspace,
        lowered_packages,
    })
}

pub fn plan_build_for_target(
    graph: &WorkspaceGraph,
    order: &[String],
    prepared: &PreparedBuild,
    existing_lock: Option<&Lockfile>,
    target: BuildTarget,
) -> PackageResult<Vec<BuildStatus>> {
    plan_build_for_target_with_context(
        graph,
        order,
        prepared,
        existing_lock,
        target,
        &BuildExecutionContext::default(),
    )
}

pub fn plan_build_for_target_with_context(
    graph: &WorkspaceGraph,
    order: &[String],
    prepared: &PreparedBuild,
    existing_lock: Option<&Lockfile>,
    target: BuildTarget,
    context: &BuildExecutionContext,
) -> PackageResult<Vec<BuildStatus>> {
    plan_build_for_member_targets_with_context(
        graph,
        order,
        prepared,
        existing_lock,
        context,
        |member| {
            Ok(Some(resolve_member_build_key(
                member,
                planned_target_for_member(&target, &member.kind),
                context.selected_product(),
            )?))
        },
    )
}

pub fn plan_package_build_for_target_with_context(
    graph: &WorkspaceGraph,
    order: &[String],
    prepared: &PreparedBuild,
    existing_lock: Option<&Lockfile>,
    target: BuildTarget,
    packaged_member: &str,
    context: &BuildExecutionContext,
) -> PackageResult<Vec<BuildStatus>> {
    let packaged_member_id = graph
        .member(packaged_member)
        .map(|member| member.package_id.clone())
        .ok_or_else(|| format!("missing workspace member `{packaged_member}`"))?;
    let packaged_names = collect_transitive_member_names(graph, packaged_member)?;
    plan_build_for_member_targets_with_context(
        graph,
        order,
        prepared,
        existing_lock,
        context,
        |member| {
            if packaged_names.contains(&member.package_id) {
                Ok(Some(resolve_member_build_key(
                    member,
                    planned_package_target_for_member(
                        &target,
                        &packaged_member_id,
                        &member.package_id,
                    ),
                    context.selected_product(),
                )?))
            } else {
                Ok(None)
            }
        },
    )
}

fn plan_build_for_member_targets_with_context<F>(
    graph: &WorkspaceGraph,
    order: &[String],
    prepared: &PreparedBuild,
    existing_lock: Option<&Lockfile>,
    context: &BuildExecutionContext,
    mut target_for_member: F,
) -> PackageResult<Vec<BuildStatus>>
where
    F: FnMut(&crate::WorkspaceMember) -> PackageResult<Option<BuildOutputKey>>,
{
    let mut statuses = Vec::new();
    let mut upstream_built = HashMap::<String, bool>::new();

    for name in order {
        let member = graph
            .member(name)
            .ok_or_else(|| format!("workspace planner missing member `{name}`"))?;
        let Some(member_build_key) = target_for_member(member)? else {
            continue;
        };
        let member_target = member_build_key.target.clone();
        let member_fingerprints = prepared
            .fingerprints
            .member(name)
            .ok_or_else(|| format!("missing fingerprint for member `{name}`"))?;
        let planned_toolchain =
            current_build_toolchain_for_target_with_context(&member_target, context)?;
        let native_product_closure =
            crate::distribution::native_product_closure_digest(graph, name, &member_build_key)?;
        let fingerprint = member_fingerprints.source().to_string();
        let api_fingerprint = member_fingerprints.api().to_string();
        let artifact_rel_path = artifact_rel_path(member, &fingerprint, &member_build_key)?;
        let existing = existing_lock
            .and_then(|lock| lock.members.get(name))
            .and_then(|member| member.build(&member_build_key));
        let existing_member = existing_lock.and_then(|lock| lock.members.get(name));
        let format = member_target
            .format()
            .ok_or_else(|| format!("unsupported build target `{member_target}`"))?
            .to_string();
        let artifact_abs_path = graph.root_dir.join(&artifact_rel_path);
        let fingerprint_changed = existing
            .map(|entry| entry.fingerprint != fingerprint)
            .unwrap_or(true);
        let api_fingerprint_changed = existing
            .map(|entry| entry.api_fingerprint != api_fingerprint)
            .unwrap_or(true);
        let format_changed = existing.map(|entry| entry.format != format).unwrap_or(true);
        let toolchain_changed = existing
            .map(|entry| entry.toolchain != planned_toolchain)
            .unwrap_or(true);
        let native_product_closure_changed = existing
            .map(|entry| entry.native_product_closure != native_product_closure)
            .unwrap_or(native_product_closure.is_some());
        let native_products_changed = existing_member
            .map(|locked_member| {
                locked_member.native_products != lock_native_product_entries(member)
            })
            .unwrap_or(!member.native_products.is_empty());
        let dependency_built = member
            .deps
            .iter()
            .any(|dep| upstream_built.get(dep).copied().unwrap_or(false));
        let built = fingerprint_changed
            || api_fingerprint_changed
            || dependency_built
            || format_changed
            || toolchain_changed
            || native_products_changed
            || native_product_closure_changed
            || !cached_artifact_matches_status(
                &artifact_abs_path,
                name,
                &member.kind,
                &fingerprint,
                &api_fingerprint,
                &member_target,
                &format,
                &planned_toolchain,
                existing
                    .map(|entry| entry.artifact_hash.as_str())
                    .unwrap_or(""),
                native_product_closure.as_deref(),
            );
        upstream_built.insert(name.clone(), built);
        statuses.push(BuildStatus {
            member: name.clone(),
            member_name: member.name.clone(),
            member_label: member.display_label(),
            disposition: if built {
                BuildDisposition::Built
            } else {
                BuildDisposition::CacheHit
            },
            snapshot_id: prepared.snapshot_id.clone(),
            fingerprint_set_id: prepared.fingerprint_set_id.clone(),
            fingerprint,
            api_fingerprint,
            build_key: member_build_key.clone(),
            target: member_target,
            product: member_build_key.product.clone(),
            artifact_rel_path,
            kind: member.kind.clone(),
            format,
            toolchain: planned_toolchain.clone(),
            native_product_closure,
        });
    }

    Ok(statuses)
}

fn planned_target_for_member(requested: &BuildTarget, kind: &GrimoireKind) -> BuildTarget {
    match (requested, kind) {
        (BuildTarget::WindowsExe, GrimoireKind::Lib) => BuildTarget::InternalAot,
        (BuildTarget::WindowsDll, GrimoireKind::App) => BuildTarget::InternalAot,
        _ => requested.clone(),
    }
}

fn planned_package_target_for_member(
    requested: &BuildTarget,
    packaged_member: &str,
    member_name: &str,
) -> BuildTarget {
    if member_name == packaged_member {
        requested.clone()
    } else {
        BuildTarget::InternalAot
    }
}

pub fn plan_build(
    graph: &WorkspaceGraph,
    order: &[String],
    prepared: &PreparedBuild,
    existing_lock: Option<&Lockfile>,
) -> PackageResult<Vec<BuildStatus>> {
    plan_build_for_target(
        graph,
        order,
        prepared,
        existing_lock,
        BuildTarget::internal_aot(),
    )
}

pub fn execute_build(
    graph: &WorkspaceGraph,
    prepared: &PreparedBuild,
    statuses: &[BuildStatus],
) -> PackageResult<PathBuf> {
    execute_build_with_context(graph, prepared, statuses, &BuildExecutionContext::default())
}

pub fn execute_build_with_context(
    graph: &WorkspaceGraph,
    prepared: &PreparedBuild,
    statuses: &[BuildStatus],
    context: &BuildExecutionContext,
) -> PackageResult<PathBuf> {
    execute_build_with_context_and_progress(graph, prepared, statuses, context, |_| {})
}

pub fn execute_build_with_context_and_progress<F>(
    graph: &WorkspaceGraph,
    prepared: &PreparedBuild,
    statuses: &[BuildStatus],
    context: &BuildExecutionContext,
    mut on_progress: F,
) -> PackageResult<PathBuf>
where
    F: FnMut(BuildProgress<'_>),
{
    validate_prepared_snapshot_with_context(prepared, statuses, context)?;

    let cache_root = graph.root_dir.join(CACHE_DIR);
    fs::create_dir_all(cache_root.join(LOGS_DIR)).map_err(|e| {
        format!(
            "failed to create cache logs directory `{}`: {e}",
            cache_root.join(LOGS_DIR).display()
        )
    })?;

    let total = statuses.len();
    for (index, status) in statuses.iter().enumerate() {
        on_progress(BuildProgress {
            index: index + 1,
            total,
            status,
        });
        if status.disposition == BuildDisposition::CacheHit {
            continue;
        }
        let member = graph
            .member(&status.member)
            .ok_or_else(|| format!("missing workspace member `{}`", status.member))?;
        let linked_package_names =
            collect_linked_package_names(graph, &prepared.workspace, &status.member)?;
        let root_package = prepared
            .lowered_packages
            .get(&member.package_id)
            .cloned()
            .ok_or_else(|| format!("missing lowered package `{}`", member.package_id))?;
        let linked_packages = linked_package_names
            .into_iter()
            .filter(|package_id| package_id != &member.package_id)
            .map(|name| {
                prepared
                    .lowered_packages
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| format!("missing lowered linked package `{name}`"))
            })
            .collect::<PackageResult<Vec<_>>>()?;
        let emission = emit_artifact_for_target(
            graph,
            member,
            &status.build_key,
            &member.kind,
            root_package,
            linked_packages,
            context,
        )
        .map_err(|e| {
            format!(
                "failed to emit `{}` for target `{}`: {e}",
                status.member, status.target
            )
        })?;
        let artifact = &emission.artifact;
        if status.format != emission.target.format() {
            return Err(format!(
                "artifact format mismatch for `{}`: planner={}, emitter={}",
                status.member,
                status.format,
                emission.target.format()
            ));
        }
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        if let Some(parent) = artifact_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create artifact directory `{}`: {e}",
                    parent.display()
                )
            })?;
        }
        write_root_emission_artifact(&artifact_path, &emission)?;
        write_emission_support_files(&artifact_path, &emission)?;
        let artifact_hash = cached_emission_hash(
            &status.build_key.storage_key(),
            &status.format,
            artifact,
            emission.root_artifact_bytes.as_deref(),
            &emission.support_files,
        );
        let metadata_path = cache_metadata_path_for_output(&artifact_path, &status.target);
        fs::write(
            &metadata_path,
            render_cached_artifact(
                &status.member,
                &status.kind,
                &status.fingerprint,
                &status.api_fingerprint,
                &status.target,
                &status.format,
                &status.toolchain,
                &emission,
                &artifact_hash,
                status.native_product_closure.as_deref(),
            ),
        )
        .map_err(|e| {
            format!(
                "failed to write artifact `{}`: {e}",
                metadata_path.display()
            )
        })?;
    }

    let summary_path = graph
        .root_dir
        .join(CACHE_DIR)
        .join(LOGS_DIR)
        .join("build-last.txt");
    if let Some(parent) = summary_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create cache logs directory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    fs::write(&summary_path, render_build_summary(statuses, graph))
        .map_err(|e| format!("failed to write `{}`: {e}", summary_path.display()))?;
    Ok(summary_path)
}

pub fn write_lockfile(
    graph: &WorkspaceGraph,
    order: &[String],
    statuses: &[BuildStatus],
) -> PackageResult<PathBuf> {
    let lock_path = graph.root_dir.join("Arcana.lock");
    let existing_lock = crate::read_lockfile(&lock_path)?;
    let rendered = render_lockfile(graph, order, statuses, existing_lock.as_ref())?;
    fs::write(&lock_path, rendered)
        .map_err(|e| format!("failed to write `{}`: {e}", lock_path.display()))?;
    Ok(lock_path)
}

pub fn render_lockfile(
    graph: &WorkspaceGraph,
    order: &[String],
    statuses: &[BuildStatus],
    existing_lock: Option<&Lockfile>,
) -> PackageResult<String> {
    let mut out = String::new();
    out.push_str(&format!("version = {LOCKFILE_VERSION}\n"));
    out.push_str(&format!(
        "workspace = \"{}\"\n",
        escape_toml(&graph.root_name)
    ));
    out.push_str(&format!(
        "workspace_root = \"{}\"\n",
        escape_toml(&graph.root_id)
    ));
    out.push_str(&format!("order = {}\n\n", format_string_array(order)));

    let workspace_members = graph
        .workspace_members()
        .map(|member| member.package_id.clone())
        .collect::<Vec<_>>();
    out.push_str(&format!(
        "workspace_members = {}\n\n",
        format_string_array(&workspace_members)
    ));

    let mut package_ids = graph
        .members
        .iter()
        .map(|member| member.package_id.clone())
        .collect::<Vec<_>>();
    package_ids.sort();

    out.push_str("[packages]\n\n");
    for package_id in &package_ids {
        let member = graph
            .member_by_id(package_id)
            .ok_or_else(|| format!("missing workspace member `{package_id}`"))?;
        out.push_str(&format!("[packages.\"{}\"]\n", escape_toml(package_id)));
        out.push_str(&format!("name = \"{}\"\n", escape_toml(&member.name)));
        out.push_str(&format!("kind = \"{}\"\n", member.kind.as_str()));
        out.push_str(&format!(
            "source_kind = \"{}\"\n",
            match member.source_kind {
                crate::DependencySource::Path => "path",
                crate::DependencySource::Registry => "registry",
                crate::DependencySource::Git => "git",
            }
        ));
        if let Some(path) = &Some(member.rel_dir.clone())
            .filter(|_| member.source_kind == crate::DependencySource::Path)
        {
            out.push_str(&format!("path = \"{}\"\n", escape_toml(path)));
        }
        if let Some(version) = &member.version {
            out.push_str(&format!("version = \"{}\"\n", version));
        }
        if let Some(registry_name) = &member.registry_name {
            out.push_str(&format!("registry = \"{}\"\n", escape_toml(registry_name)));
        }
        if let Some(checksum) = &member.checksum {
            out.push_str(&format!("checksum = \"{}\"\n", escape_toml(checksum)));
        }
        if let Some(git_url) = &member.git_url {
            out.push_str(&format!("git = \"{}\"\n", escape_toml(git_url)));
        }
        if let Some(git_selector) = &member.git_selector {
            out.push_str(&format!(
                "git_selector = \"{}\"\n",
                escape_toml(git_selector)
            ));
        }
        out.push('\n');
    }

    out.push_str("[dependencies]\n\n");
    for package_id in &package_ids {
        let member = graph
            .member_by_id(package_id)
            .ok_or_else(|| format!("missing workspace member `{package_id}`"))?;
        if member.direct_dep_ids.is_empty() {
            continue;
        }
        out.push_str(&format!("[dependencies.\"{}\"]\n", escape_toml(package_id)));
        for (alias, dep_id) in &member.direct_dep_ids {
            out.push_str(&format!(
                "{} = \"{}\"\n",
                quote_toml_key(alias),
                escape_toml(dep_id)
            ));
        }
        out.push('\n');
    }

    out.push_str("[native_products]\n\n");
    for package_id in &package_ids {
        let member = graph
            .member_by_id(package_id)
            .ok_or_else(|| format!("missing workspace member `{package_id}`"))?;
        let native_products = lock_native_product_entries(member);
        if native_products.is_empty() {
            continue;
        }
        for (product_name, entry) in native_products {
            out.push_str(&format!(
                "[native_products.\"{}\".\"{}\"]\n",
                escape_toml(package_id),
                escape_toml(&product_name)
            ));
            out.push_str(&format!("kind = \"{}\"\n", escape_toml(&entry.kind)));
            out.push_str(&format!("role = \"{}\"\n", escape_toml(&entry.role)));
            out.push_str(&format!(
                "producer = \"{}\"\n",
                escape_toml(&entry.producer)
            ));
            out.push_str(&format!("file = \"{}\"\n", escape_toml(&entry.file)));
            out.push_str(&format!(
                "contract = \"{}\"\n",
                escape_toml(&entry.contract)
            ));
            if let Some(path) = &entry.rust_cdylib_crate {
                out.push_str(&format!("rust_cdylib_crate = \"{}\"\n", escape_toml(path)));
            }
            out.push_str(&format!(
                "sidecars = {}\n\n",
                format_string_array(&entry.sidecars)
            ));
        }
    }

    let merged_builds = merge_lock_build_entries(graph, &package_ids, statuses, existing_lock)?;
    out.push_str("[builds]\n\n");
    for package_id in &package_ids {
        let builds = merged_builds
            .get(package_id)
            .ok_or_else(|| format!("missing build entries for `{package_id}`"))?;
        if builds.is_empty() {
            continue;
        }
        for (build_key, entry) in builds {
            out.push_str(&format!(
                "[builds.\"{}\".\"{}\"]\n",
                escape_toml(package_id),
                escape_toml(&build_key.storage_key())
            ));
            out.push_str(&format!(
                "fingerprint = \"{}\"\n",
                escape_toml(&entry.fingerprint)
            ));
            out.push_str(&format!(
                "api_fingerprint = \"{}\"\n",
                escape_toml(&entry.api_fingerprint)
            ));
            out.push_str(&format!(
                "artifact = \"{}\"\n",
                escape_toml(&entry.artifact)
            ));
            out.push_str(&format!(
                "artifact_hash = \"{}\"\n",
                escape_toml(&entry.artifact_hash)
            ));
            out.push_str(&format!("format = \"{}\"\n", escape_toml(&entry.format)));
            if let Some(product) = &entry.product {
                out.push_str(&format!("product = \"{}\"\n", escape_toml(product)));
            }
            if let Some(closure) = &entry.native_product_closure {
                out.push_str(&format!(
                    "native_product_closure = \"{}\"\n",
                    escape_toml(closure)
                ));
            }
            out.push_str(&format!(
                "toolchain = \"{}\"\n\n",
                escape_toml(&entry.toolchain)
            ));
        }
    }

    Ok(out)
}

fn merge_lock_build_entries(
    graph: &WorkspaceGraph,
    names: &[String],
    statuses: &[BuildStatus],
    existing_lock: Option<&Lockfile>,
) -> PackageResult<BTreeMap<String, BTreeMap<BuildOutputKey, LockTargetEntry>>> {
    let mut updates = HashMap::<(String, BuildOutputKey), LockTargetEntry>::new();
    for status in statuses {
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        let artifact_hash = cached_emission_hash_for_path(&artifact_path, &status.target)?;
        updates.insert(
            (status.member.clone(), status.build_key.clone()),
            LockTargetEntry {
                fingerprint: status.fingerprint.clone(),
                api_fingerprint: status.api_fingerprint.clone(),
                artifact: status.artifact_rel_path.clone(),
                artifact_hash,
                format: status.format.clone(),
                toolchain: status.toolchain.clone(),
                product: status.product.clone(),
                native_product_closure: status.native_product_closure.clone(),
            },
        );
    }

    let mut merged = BTreeMap::new();
    for name in names {
        let mut entries = existing_lock
            .and_then(|lock| lock.members.get(name))
            .map(|member| member.targets.clone())
            .unwrap_or_default();
        for ((member_name, build_key), entry) in &updates {
            if member_name == name {
                entries.insert(build_key.clone(), entry.clone());
            }
        }
        merged.insert(name.clone(), entries);
    }
    Ok(merged)
}

fn lock_native_product_entries(
    member: &crate::WorkspaceMember,
) -> BTreeMap<String, LockNativeProductEntry> {
    member
        .native_products
        .iter()
        .map(|(name, product)| {
            (
                name.clone(),
                LockNativeProductEntry {
                    kind: product.kind.clone(),
                    role: product.role.as_str().to_string(),
                    producer: product.producer.as_str().to_string(),
                    file: product.file.clone(),
                    contract: product.contract.clone(),
                    rust_cdylib_crate: product.rust_cdylib_crate.clone(),
                    sidecars: product.sidecars.clone(),
                },
            )
        })
        .collect()
}

fn quote_toml_key(text: &str) -> String {
    format!("\"{}\"", escape_toml(text))
}

pub fn render_build_summary(statuses: &[BuildStatus], graph: &WorkspaceGraph) -> String {
    let mut out = String::from("summary-v1\n");
    for status in statuses {
        out.push_str(&format!(
            "BUILD_STATUS member={} build={} disposition={} fingerprint={}\n",
            status.member_label,
            status.build_key.storage_key(),
            match status.disposition {
                BuildDisposition::Built => "built",
                BuildDisposition::CacheHit => "cache_hit",
            },
            status.fingerprint
        ));
    }
    out.push_str(&format!(
        "LOCK path={}\n",
        graph.root_dir.join("Arcana.lock").display()
    ));
    out
}

fn collect_linked_package_names(
    graph: &WorkspaceGraph,
    workspace: &HirWorkspaceSummary,
    root_member: &str,
) -> PackageResult<Vec<String>> {
    let mut names = collect_transitive_member_names(graph, root_member)?;
    let uses_std = names.iter().any(|name| {
        workspace
            .package_by_id(name)
            .map(package_uses_implicit_std)
            .unwrap_or(false)
    });
    if uses_std && let Some(std_package) = workspace.package("std") {
        names.insert(std_package.package_id.clone());
    }
    Ok(names.into_iter().collect())
}

fn collect_transitive_member_names(
    graph: &WorkspaceGraph,
    root_member: &str,
) -> PackageResult<BTreeSet<String>> {
    let root_package_id = graph
        .member(root_member)
        .map(|member| member.package_id.clone())
        .ok_or_else(|| format!("missing workspace member `{root_member}`"))?;
    let mut pending = vec![root_package_id];
    let mut visited = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let member = graph
            .member(&name)
            .ok_or_else(|| format!("missing workspace member `{name}`"))?;
        for dep in member.deps.iter().rev() {
            pending.push(dep.clone());
        }
    }
    Ok(visited)
}

fn runtime_requirement_roots_for_kind(kind: &GrimoireKind) -> RuntimeRequirementRoots {
    match kind {
        GrimoireKind::App => RuntimeRequirementRoots::Entrypoints,
        GrimoireKind::Lib => RuntimeRequirementRoots::ExportedRootPackageRoutines,
    }
}

fn link_ir_packages(
    root_kind: &GrimoireKind,
    root: IrPackage,
    mut linked: Vec<IrPackage>,
) -> IrPackage {
    linked.sort_by(|left, right| {
        left.package_name
            .cmp(&right.package_name)
            .then_with(|| left.package_id.cmp(&right.package_id))
    });

    let mut direct_deps = root.direct_deps.iter().cloned().collect::<BTreeSet<_>>();
    let mut direct_dep_ids = root.direct_dep_ids.iter().cloned().collect::<BTreeSet<_>>();
    for package in &linked {
        if package.package_name == "std" {
            direct_deps.insert("std".to_string());
            direct_dep_ids.insert(package.package_id.clone());
        }
    }

    let mut modules = root.modules.clone();
    let mut dependency_rows = root
        .dependency_rows
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let exported_surface_rows = root
        .exported_surface_rows
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut package_display_names = root.package_display_names.clone();
    let mut package_direct_dep_ids = root.package_direct_dep_ids.clone();
    let mut routines = root.routines.clone();
    let mut owners = root.owners.clone();

    for package in linked {
        package_display_names.extend(package.package_display_names);
        package_direct_dep_ids.extend(package.package_direct_dep_ids);
        modules.extend(package.modules);
        dependency_rows.extend(package.dependency_rows);
        routines.extend(package.routines);
        owners.extend(package.owners);
    }

    modules.sort_by(|left, right| {
        left.package_id
            .cmp(&right.package_id)
            .then_with(|| left.module_id.cmp(&right.module_id))
    });
    routines.sort_by(|left, right| {
        left.package_id
            .cmp(&right.package_id)
            .then_with(|| left.module_id.cmp(&right.module_id))
            .then_with(|| left.symbol_name.cmp(&right.symbol_name))
            .then_with(|| {
                render_routine_signature_text(
                    &left.symbol_kind,
                    &left.symbol_name,
                    left.is_async,
                    &left.type_params,
                    &left.params,
                    left.return_type.as_ref(),
                )
                .cmp(&render_routine_signature_text(
                    &right.symbol_kind,
                    &right.symbol_name,
                    right.is_async,
                    &right.type_params,
                    &right.params,
                    right.return_type.as_ref(),
                ))
            })
    });

    let mut linked_package = IrPackage {
        package_id: root.package_id,
        package_name: root.package_name,
        root_module_id: root.root_module_id,
        direct_deps: direct_deps.into_iter().collect(),
        direct_dep_ids: direct_dep_ids.into_iter().collect(),
        package_display_names,
        package_direct_dep_ids,
        modules,
        dependency_edge_count: dependency_rows.len(),
        dependency_rows: dependency_rows.into_iter().collect(),
        exported_surface_rows: exported_surface_rows.into_iter().collect(),
        runtime_requirements: Vec::new(),
        entrypoints: root.entrypoints,
        routines,
        owners,
    };
    disambiguate_package_routine_keys(&mut linked_package)
        .expect("linked package routine-key disambiguation should succeed");
    linked_package.runtime_requirements = derive_runtime_requirements_with_roots(
        &linked_package,
        runtime_requirement_roots_for_kind(root_kind),
    );
    linked_package
}

fn emit_artifact_for_target(
    graph: &WorkspaceGraph,
    root_member: &crate::WorkspaceMember,
    build_key: &BuildOutputKey,
    root_kind: &GrimoireKind,
    root: IrPackage,
    linked_packages: Vec<IrPackage>,
    build_context: &BuildExecutionContext,
) -> PackageResult<AotPackageEmission> {
    let linked_package = link_ir_packages(root_kind, root, linked_packages);
    if let BuildTarget::WindowsDll = build_key.target_ref()
        && let Some(selected_product) = selected_native_product_for_build(
            root_member,
            build_key.target_ref(),
            build_key.product(),
        )?
        && !uses_arcana_source_export_bundle(&selected_product)
    {
        return emit_root_native_product_bundle(
            graph,
            root_member,
            &linked_package,
            &selected_product,
        );
    }
    let context = emit_context_for_target(graph, root_member, build_key, root_kind, build_context)?;
    match build_key.target_ref() {
        BuildTarget::InternalAot => {
            emit_package_with_context(AotEmitTarget::InternalArtifact, &linked_package, &context)
        }
        BuildTarget::WindowsExe => {
            emit_package_with_context(AotEmitTarget::WindowsExeBundle, &linked_package, &context)
        }
        BuildTarget::WindowsDll => {
            emit_package_with_context(AotEmitTarget::WindowsDllBundle, &linked_package, &context)
        }
        BuildTarget::Other(_) => Err(format!(
            "unsupported build target `{}`",
            build_key.target_ref()
        )),
    }
}

fn emit_root_native_product_bundle(
    graph: &WorkspaceGraph,
    root_member: &crate::WorkspaceMember,
    linked_package: &IrPackage,
    product: &NativeProductSpec,
) -> PackageResult<AotPackageEmission> {
    native_product_probe(
        "emit_root_native_product",
        format!(
            "member={} product={} role={} producer={}",
            root_member.name,
            product.name,
            product.role.as_str(),
            product.producer.as_str()
        ),
    );
    let files = resolve_native_product_files(graph, root_member, product)?;
    let root_file = files
        .iter()
        .find(|file| file.relative_path == product.file)
        .ok_or_else(|| {
            format!(
                "root native product `{}` on `{}` did not resolve its primary file `{}`",
                product.name, root_member.name, product.file
            )
        })?;
    let root_artifact_bytes = fs::read(&root_file.source_path).map_err(|e| {
        format!(
            "failed to read root native product artifact `{}`: {e}",
            root_file.source_path.display()
        )
    })?;
    let support_files = files
        .into_iter()
        .filter(|file| file.relative_path != product.file)
        .map(|file| {
            fs::read(&file.source_path)
                .map(|bytes| AotEmissionFile {
                    relative_path: file.relative_path,
                    bytes,
                })
                .map_err(|e| {
                    format!(
                        "failed to read native product support file `{}`: {e}",
                        file.source_path.display()
                    )
                })
        })
        .collect::<PackageResult<Vec<_>>>()?;
    let artifact = compile_package(linked_package);
    validate_package_artifact(&artifact)
        .map_err(|e| format!("root native product artifact validation failed: {e}"))?;
    let primary_artifact_body = render_package_artifact(&artifact);
    Ok(AotPackageEmission {
        target: AotEmitTarget::WindowsDllBundle,
        artifact,
        primary_artifact_body,
        root_artifact_bytes: Some(root_artifact_bytes),
        support_files,
    })
}

fn emit_context_for_target(
    graph: &WorkspaceGraph,
    root_member: &crate::WorkspaceMember,
    build_key: &BuildOutputKey,
    kind: &GrimoireKind,
    build_context: &BuildExecutionContext,
) -> PackageResult<AotEmitContext> {
    let selected_native_product = selected_native_product_for_build(
        root_member,
        build_key.target_ref(),
        build_key.product(),
    )?;
    let root_artifact_file_name = match &selected_native_product {
        Some(product) => product.file.clone(),
        None => build_key.target_ref().artifact_file_name(kind)?.to_string(),
    };
    let _ = graph;
    let _ = build_context;
    let runtime_binding = AotRuntimeBinding::Baked;
    let native_product = selected_native_product
        .as_ref()
        .filter(|product| uses_arcana_source_export_bundle(product))
        .map(export_native_product_metadata_from_spec)
        .transpose()?;
    match build_key.target_ref() {
        BuildTarget::InternalAot | BuildTarget::WindowsExe | BuildTarget::WindowsDll => {
            Ok(AotEmitContext {
                root_artifact_file_name: Some(root_artifact_file_name),
                runtime_binding,
                native_product,
            })
        }
        BuildTarget::Other(_) => Err(format!(
            "unsupported build target `{}`",
            build_key.target_ref()
        )),
    }
}

fn write_root_emission_artifact(
    artifact_path: &Path,
    emission: &AotPackageEmission,
) -> PackageResult<()> {
    if let Some(bytes) = &emission.root_artifact_bytes {
        fs::write(artifact_path, bytes).map_err(|e| {
            format!(
                "failed to write emitted artifact `{}`: {e}",
                artifact_path.display()
            )
        })?;
    }
    Ok(())
}

fn write_emission_support_files(
    artifact_path: &Path,
    emission: &AotPackageEmission,
) -> PackageResult<()> {
    let Some(artifact_dir) = artifact_path.parent() else {
        return Err(format!(
            "artifact path `{}` is missing a parent directory",
            artifact_path.display()
        ));
    };
    collect_validated_support_file_paths(
        emission
            .support_files
            .iter()
            .map(|file| file.relative_path.as_str()),
    )
    .map_err(|e| format!("backend emission produced {e}"))?;
    for file in &emission.support_files {
        let relative = Path::new(&file.relative_path);
        let output_path = artifact_dir.join(relative);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create emitted support file directory `{}`: {e}",
                    parent.display()
                )
            })?;
        }
        fs::write(&output_path, &file.bytes).map_err(|e| {
            format!(
                "failed to write emitted support file `{}`: {e}",
                output_path.display()
            )
        })?;
    }
    Ok(())
}

fn artifact_rel_path(
    member: &crate::WorkspaceMember,
    fingerprint: &str,
    build_key: &BuildOutputKey,
) -> PackageResult<String> {
    let artifact_file_name = match selected_native_product_for_build(
        member,
        build_key.target_ref(),
        build_key.product(),
    )? {
        Some(product) => product.file,
        None => build_key
            .target_ref()
            .artifact_file_name(&member.kind)?
            .to_string(),
    };
    let package_dir = format!(
        "{}_{}",
        sanitize_package_name(&member.name),
        short_package_id_hash(&member.package_id)
    );
    Ok(format!(
        "{CACHE_DIR}/{ARTIFACT_DIR}/{package_dir}/{}/{}/{}",
        build_key.storage_key(),
        sanitize_fingerprint(fingerprint),
        artifact_file_name,
    ))
}

fn sanitize_fingerprint(text: &str) -> String {
    text.replace(':', "_")
}

fn sanitize_package_name(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn short_package_id_hash(package_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_package_id_v1\n");
    hasher.update(package_id.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    digest.chars().take(12).collect()
}

fn resolve_member_build_key(
    member: &crate::WorkspaceMember,
    target: BuildTarget,
    requested_product: Option<&str>,
) -> PackageResult<BuildOutputKey> {
    match target {
        BuildTarget::WindowsDll => {
            let selected = selected_native_product_for_build(member, &target, requested_product)?;
            let product_name = selected
                .map(|product| product.name)
                .unwrap_or_else(|| "default".to_string());
            Ok(BuildOutputKey::new(target, Some(product_name)))
        }
        _ => Ok(BuildOutputKey::target(target)),
    }
}

pub(crate) fn selected_native_product_for_build(
    member: &crate::WorkspaceMember,
    target: &BuildTarget,
    requested_product: Option<&str>,
) -> PackageResult<Option<NativeProductSpec>> {
    match target {
        BuildTarget::WindowsDll => {
            select_root_windows_dll_product_spec(member, requested_product).map(Some)
        }
        BuildTarget::InternalAot | BuildTarget::WindowsExe | BuildTarget::Other(_) => Ok(None),
    }
}

fn select_root_windows_dll_product_spec(
    member: &crate::WorkspaceMember,
    requested_product: Option<&str>,
) -> PackageResult<NativeProductSpec> {
    let products = member.native_products.values().cloned().collect::<Vec<_>>();

    if products.is_empty() {
        let requested_name = requested_product.unwrap_or("default");
        if requested_product.is_some() && requested_name != "default" {
            native_product_probe(
                "missing_implicit_export_product",
                format!("member={} requested_product={requested_name}", member.name),
            );
            return Err(format!(
                "workspace member `{}` has no named native product `{requested_name}`",
                member.name
            ));
        }
        native_product_probe(
            "implicit_export_product",
            format!(
                "member={} requested_product={} synthesized_default=true",
                member.name, requested_name
            ),
        );
        return Ok(NativeProductSpec {
            name: requested_name.to_string(),
            kind: "dll".to_string(),
            role: ArcanaCabiProductRole::Export,
            producer: NativeProductProducer::ArcanaSource,
            file: BuildTarget::WindowsDll
                .artifact_file_name(&member.kind)?
                .to_string(),
            contract: ARCANA_CABI_EXPORT_CONTRACT_ID.to_string(),
            rust_cdylib_crate: None,
            sidecars: Vec::new(),
        });
    }

    let selected = match requested_product {
        Some(name) => products
            .into_iter()
            .find(|product| product.name == name)
            .ok_or_else(|| {
                native_product_probe(
                    "missing_root_native_product",
                    format!("member={} requested_product={name}", member.name),
                );
                format!(
                    "workspace member `{}` has no native product `{name}`",
                    member.name
                )
            })?,
        None => {
            let export_products = products
                .into_iter()
                .filter(|product| product.role == ArcanaCabiProductRole::Export)
                .collect::<Vec<_>>();
            if export_products.is_empty() {
                native_product_probe(
                    "missing_default_export_product",
                    format!(
                        "member={} declared_products={}",
                        member.name,
                        member.native_products.len()
                    ),
                );
                return Err(format!(
                    "workspace member `{}` has no default export native product; pass `--product <name>`",
                    member.name
                ));
            }
            let [product] = export_products.as_slice() else {
                native_product_probe(
                    "ambiguous_export_product",
                    format!(
                        "member={} export_products={}",
                        member.name,
                        export_products
                            .iter()
                            .map(|product| product.name.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                );
                return Err(format!(
                    "workspace member `{}` has multiple export native products; pass `--product <name>`",
                    member.name
                ));
            };
            product.clone()
        }
    };

    native_product_probe(
        "selected_root_native_product",
        format!(
            "member={} product={} role={} producer={} contract={}",
            member.name,
            selected.name,
            selected.role.as_str(),
            selected.producer.as_str(),
            selected.contract
        ),
    );

    Ok(selected)
}

fn export_native_product_metadata_from_spec(
    spec: &NativeProductSpec,
) -> PackageResult<AotNativeProduct> {
    if spec.role != ArcanaCabiProductRole::Export {
        return Err(format!(
            "native product `{}` uses role `{}` but `windows-dll` requires `role = \"export\"`",
            spec.name,
            spec.role.as_str()
        ));
    }

    Ok(AotNativeProduct {
        name: spec.name.clone(),
        role: spec.role,
        contract_id: spec.contract.clone(),
        contract_version: ARCANA_CABI_CONTRACT_VERSION_V1,
    })
}

fn uses_arcana_source_export_bundle(spec: &NativeProductSpec) -> bool {
    spec.role == ArcanaCabiProductRole::Export
        && spec.producer == NativeProductProducer::ArcanaSource
}

fn validate_prepared_snapshot_with_context(
    prepared: &PreparedBuild,
    statuses: &[BuildStatus],
    context: &BuildExecutionContext,
) -> PackageResult<()> {
    for status in statuses {
        if status.snapshot_id != prepared.snapshot_id {
            return Err(format!(
                "build status for `{}` was planned from snapshot `{}` but prepared build uses `{}`",
                status.member, status.snapshot_id, prepared.snapshot_id
            ));
        }
        if status.fingerprint_set_id != prepared.fingerprint_set_id {
            return Err(format!(
                "build status for `{}` was planned from fingerprint set `{}` but prepared build uses `{}`",
                status.member, status.fingerprint_set_id, prepared.fingerprint_set_id
            ));
        }
        let expected_toolchain =
            current_build_toolchain_for_target_with_context(&status.target, context)?;
        if status.toolchain != expected_toolchain {
            return Err(format!(
                "build status for `{}` targets toolchain `{}` but current `{}` toolchain is `{expected_toolchain}`",
                status.member,
                status.toolchain,
                status.target()
            ));
        }
    }
    Ok(())
}

fn native_product_probe(event: &str, message: impl AsRef<str>) {
    if std::env::var_os(ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV).is_some() {
        eprintln!(
            "[arcana-native-product-probe] {event}: {}",
            message.as_ref()
        );
    }
}

fn format_string_array(items: &[String]) -> String {
    let rendered = items
        .iter()
        .map(|item| format!("\"{}\"", escape_toml(item)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{rendered}]")
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use arcana_aot::{AOT_INTERNAL_FORMAT, AotEmissionFile, AotPackageArtifact};

    use super::{AotEmitTarget, AotPackageEmission, write_emission_support_files};

    fn repo_root() -> PathBuf {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir
            .parent()
            .and_then(Path::parent)
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join("arcana-build-tests")
            .join(format!("{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn test_package_display_names_with_deps(
        package_id: impl Into<String>,
        package_name: impl Into<String>,
        direct_deps: Vec<String>,
        direct_dep_ids: Vec<String>,
    ) -> BTreeMap<String, String> {
        let mut names = BTreeMap::from([(package_id.into(), package_name.into())]);
        for (dep_name, dep_id) in direct_deps.into_iter().zip(direct_dep_ids) {
            names.entry(dep_id).or_insert(dep_name);
        }
        names
    }

    fn test_package_direct_dep_ids(
        package_id: impl Into<String>,
        direct_deps: Vec<String>,
        direct_dep_ids: Vec<String>,
    ) -> BTreeMap<String, BTreeMap<String, String>> {
        BTreeMap::from([(
            package_id.into(),
            direct_deps.into_iter().zip(direct_dep_ids).collect(),
        )])
    }

    fn dummy_emission(files: Vec<AotEmissionFile>) -> AotPackageEmission {
        AotPackageEmission {
            target: AotEmitTarget::InternalArtifact,
            artifact: AotPackageArtifact {
                format: AOT_INTERNAL_FORMAT.to_string(),
                package_id: "tool".to_string(),
                package_name: "tool".to_string(),
                root_module_id: "tool".to_string(),
                direct_deps: Vec::new(),
                direct_dep_ids: Vec::new(),
                package_display_names: test_package_display_names_with_deps(
                    "tool".to_string(),
                    "tool".to_string(),
                    Vec::new(),
                    Vec::new(),
                ),
                package_direct_dep_ids: test_package_direct_dep_ids(
                    "tool".to_string(),
                    Vec::new(),
                    Vec::new(),
                ),
                module_count: 0,
                dependency_edge_count: 0,
                dependency_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
                runtime_requirements: Vec::new(),
                entrypoints: Vec::new(),
                routines: Vec::new(),
                owners: Vec::new(),
                modules: Vec::new(),
            },
            primary_artifact_body: String::new(),
            root_artifact_bytes: None,
            support_files: files,
        }
    }

    #[test]
    fn write_emission_support_files_writes_relative_outputs() {
        let dir = temp_dir("support_files_ok");
        let artifact_path = dir.join("app.artifact.toml");
        let emission = dummy_emission(vec![AotEmissionFile {
            relative_path: "bin/app.exe".to_string(),
            bytes: b"binary".to_vec(),
        }]);

        write_emission_support_files(&artifact_path, &emission)
            .expect("support files should write");

        let written = fs::read(dir.join("bin").join("app.exe")).expect("support file should exist");
        assert_eq!(written, b"binary");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_emission_support_files_rejects_parent_traversal() {
        let dir = temp_dir("support_files_bad_path");
        let artifact_path = dir.join("app.artifact.toml");
        let emission = dummy_emission(vec![AotEmissionFile {
            relative_path: "..\\escape.exe".to_string(),
            bytes: b"binary".to_vec(),
        }]);

        let err = write_emission_support_files(&artifact_path, &emission)
            .expect_err("support files should reject parent traversal");
        assert!(err.contains("invalid support file path"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_emission_support_files_rejects_duplicate_paths() {
        let dir = temp_dir("support_files_duplicate");
        let artifact_path = dir.join("app.artifact.toml");
        let emission = dummy_emission(vec![
            AotEmissionFile {
                relative_path: "bin/app.exe".to_string(),
                bytes: b"first".to_vec(),
            },
            AotEmissionFile {
                relative_path: "bin/app.exe".to_string(),
                bytes: b"second".to_vec(),
            },
        ]);

        let err = write_emission_support_files(&artifact_path, &emission)
            .expect_err("support files should reject duplicate paths");
        assert!(err.contains("duplicate support file path"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }
}
