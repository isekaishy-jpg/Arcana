use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;

use arcana_aot::{AOT_INTERNAL_FORMAT, compile_package};
use arcana_hir::{HirResolvedWorkspace, HirWorkspaceSummary, resolve_workspace};
use arcana_ir::{
    IrPackage, RuntimeRequirementRoots, derive_runtime_requirements_with_roots,
    lower_workspace_package_with_resolution,
};

use crate::build_identity::{
    artifact_body_hash, artifact_body_hash_for_path, cached_artifact_matches_status,
    current_build_toolchain, render_cached_artifact,
};
use crate::fingerprint::{
    WorkspaceFingerprints, compute_workspace_fingerprints, package_uses_implicit_std,
};
use crate::{
    ARTIFACT_DIR, CACHE_DIR, GrimoireKind, LOCKFILE_VERSION, LOGS_DIR, Lockfile, PackageResult,
    WorkspaceGraph,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildDisposition {
    Built,
    CacheHit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildStatus {
    pub(crate) member: String,
    pub(crate) disposition: BuildDisposition,
    pub(crate) snapshot_id: String,
    pub(crate) fingerprint_set_id: String,
    pub(crate) fingerprint: String,
    pub(crate) api_fingerprint: String,
    pub(crate) artifact_rel_path: String,
    pub(crate) kind: GrimoireKind,
    pub(crate) format: String,
    pub(crate) toolchain: String,
}

impl BuildStatus {
    pub fn member(&self) -> &str {
        &self.member
    }

    pub fn disposition(&self) -> BuildDisposition {
        self.disposition
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn artifact_rel_path(&self) -> &str {
        &self.artifact_rel_path
    }
}

#[derive(Debug)]
pub struct PreparedBuild {
    pub(crate) snapshot_id: String,
    pub(crate) fingerprint_set_id: String,
    pub(crate) toolchain: String,
    pub(crate) fingerprints: WorkspaceFingerprints,
    pub(crate) workspace: HirWorkspaceSummary,
    pub(crate) lowered_packages: BTreeMap<String, IrPackage>,
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
                package.summary.package_name.clone(),
                lower_workspace_package_with_resolution(&workspace, &resolved_workspace, package)?,
            ))
        })
        .collect::<PackageResult<BTreeMap<_, _>>>()?;
    Ok(PreparedBuild {
        snapshot_id: fingerprints.snapshot_id().to_string(),
        fingerprint_set_id: fingerprints.identity(),
        toolchain: current_build_toolchain()?,
        fingerprints,
        workspace,
        lowered_packages,
    })
}

pub fn plan_build(
    graph: &WorkspaceGraph,
    order: &[String],
    prepared: &PreparedBuild,
    existing_lock: Option<&Lockfile>,
) -> PackageResult<Vec<BuildStatus>> {
    let mut statuses = Vec::new();
    let mut upstream_built = HashMap::<String, bool>::new();
    let toolchain_changed = existing_lock
        .map(|lock| lock.toolchain != prepared.toolchain.as_str())
        .unwrap_or(true);

    for name in order {
        let member = graph
            .member(name)
            .ok_or_else(|| format!("workspace planner missing member `{name}`"))?;
        let member_fingerprints = prepared
            .fingerprints
            .member(name)
            .ok_or_else(|| format!("missing fingerprint for member `{name}`"))?;
        let fingerprint = member_fingerprints.source().to_string();
        let api_fingerprint = member_fingerprints.api().to_string();
        let artifact_rel_path = artifact_rel_path(name, &fingerprint, &member.kind);
        let existing = existing_lock.and_then(|lock| lock.members.get(name));
        let format = AOT_INTERNAL_FORMAT.to_string();
        let artifact_abs_path = graph.root_dir.join(&artifact_rel_path);
        let fingerprint_changed = existing
            .map(|entry| entry.fingerprint != fingerprint)
            .unwrap_or(true);
        let api_fingerprint_changed = existing
            .map(|entry| entry.api_fingerprint != api_fingerprint)
            .unwrap_or(true);
        let format_changed = existing.map(|entry| entry.format != format).unwrap_or(true);
        let dependency_built = member
            .deps
            .iter()
            .any(|dep| upstream_built.get(dep).copied().unwrap_or(false));
        let built = fingerprint_changed
            || api_fingerprint_changed
            || dependency_built
            || format_changed
            || toolchain_changed
            || !cached_artifact_matches_status(
                &artifact_abs_path,
                name,
                &member.kind,
                &fingerprint,
                &api_fingerprint,
                &format,
                &prepared.toolchain,
                existing
                    .map(|entry| entry.artifact_hash.as_str())
                    .unwrap_or(""),
            );
        upstream_built.insert(name.clone(), built);
        statuses.push(BuildStatus {
            member: name.clone(),
            disposition: if built {
                BuildDisposition::Built
            } else {
                BuildDisposition::CacheHit
            },
            snapshot_id: prepared.snapshot_id.clone(),
            fingerprint_set_id: prepared.fingerprint_set_id.clone(),
            fingerprint,
            api_fingerprint,
            artifact_rel_path,
            kind: member.kind.clone(),
            format,
            toolchain: prepared.toolchain.clone(),
        });
    }

    Ok(statuses)
}

pub fn execute_build(
    graph: &WorkspaceGraph,
    prepared: &PreparedBuild,
    statuses: &[BuildStatus],
) -> PackageResult<PathBuf> {
    validate_prepared_snapshot(prepared, statuses)?;

    let cache_root = graph.root_dir.join(CACHE_DIR);
    fs::create_dir_all(cache_root.join(LOGS_DIR)).map_err(|e| {
        format!(
            "failed to create cache logs directory `{}`: {e}",
            cache_root.join(LOGS_DIR).display()
        )
    })?;

    for status in statuses {
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
            .get(&member.name)
            .cloned()
            .ok_or_else(|| format!("missing lowered package `{}`", member.name))?;
        let linked_packages = linked_package_names
            .into_iter()
            .filter(|name| name != &member.name)
            .map(|name| {
                prepared
                    .lowered_packages
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| format!("missing lowered linked package `{name}`"))
            })
            .collect::<PackageResult<Vec<_>>>()?;
        let artifact = compile_package(&link_ir_packages(
            &member.kind,
            root_package,
            linked_packages,
        ));
        if artifact.format != status.format {
            return Err(format!(
                "artifact format mismatch for `{}`: planner={}, compiler={}",
                status.member, status.format, artifact.format
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
        let artifact_hash = artifact_body_hash(&artifact);
        fs::write(
            &artifact_path,
            render_cached_artifact(
                &status.member,
                &status.kind,
                &status.fingerprint,
                &status.api_fingerprint,
                &status.toolchain,
                &artifact,
                &artifact_hash,
            ),
        )
        .map_err(|e| {
            format!(
                "failed to write artifact `{}`: {e}",
                artifact_path.display()
            )
        })?;
    }

    let summary_path = graph
        .root_dir
        .join(CACHE_DIR)
        .join(LOGS_DIR)
        .join("build-last.txt");
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
    let rendered = render_lockfile(graph, order, statuses)?;
    fs::write(&lock_path, rendered)
        .map_err(|e| format!("failed to write `{}`: {e}", lock_path.display()))?;
    Ok(lock_path)
}

pub fn render_lockfile(
    graph: &WorkspaceGraph,
    order: &[String],
    statuses: &[BuildStatus],
) -> PackageResult<String> {
    let toolchain = lockfile_toolchain(statuses)?;
    let mut out = String::new();
    out.push_str(&format!("version = {LOCKFILE_VERSION}\n"));
    out.push_str(&format!(
        "workspace = \"{}\"\n",
        escape_toml(&graph.root_name)
    ));
    out.push_str(&format!("toolchain = \"{}\"\n", escape_toml(&toolchain)));
    out.push_str(&format!("order = {}\n\n", format_string_array(order)));

    let status_map = statuses
        .iter()
        .map(|status| (status.member.clone(), status))
        .collect::<HashMap<_, _>>();

    let mut names = graph
        .members
        .iter()
        .map(|member| member.name.clone())
        .collect::<Vec<_>>();
    names.sort();

    out.push_str("[paths]\n");
    for name in &names {
        let member = graph
            .member(name)
            .ok_or_else(|| format!("missing workspace member `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            escape_toml(&member.rel_dir)
        ));
    }
    out.push('\n');

    out.push_str("[deps]\n");
    for name in &names {
        let member = graph
            .member(name)
            .ok_or_else(|| format!("missing workspace member `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = {}\n",
            escape_toml(name),
            format_string_array(&member.deps)
        ));
    }
    out.push('\n');

    out.push_str("[kinds]\n");
    for name in &names {
        let status = status_map
            .get(name)
            .ok_or_else(|| format!("missing build status for `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            status.kind.as_str()
        ));
    }
    out.push('\n');

    out.push_str("[formats]\n");
    for name in &names {
        let status = status_map
            .get(name)
            .ok_or_else(|| format!("missing build status for `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            escape_toml(&status.format)
        ));
    }
    out.push('\n');

    out.push_str("[fingerprints]\n");
    for name in &names {
        let status = status_map
            .get(name)
            .ok_or_else(|| format!("missing build status for `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            escape_toml(&status.fingerprint)
        ));
    }
    out.push('\n');

    out.push_str("[api_fingerprints]\n");
    for name in &names {
        let status = status_map
            .get(name)
            .ok_or_else(|| format!("missing build status for `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            escape_toml(&status.api_fingerprint)
        ));
    }
    out.push('\n');

    out.push_str("[artifacts]\n");
    for name in &names {
        let status = status_map
            .get(name)
            .ok_or_else(|| format!("missing build status for `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            escape_toml(&status.artifact_rel_path)
        ));
    }
    out.push('\n');

    out.push_str("[artifact_hashes]\n");
    for name in &names {
        let status = status_map
            .get(name)
            .ok_or_else(|| format!("missing build status for `{name}`"))?;
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        let artifact_hash = artifact_body_hash_for_path(&artifact_path)?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            escape_toml(&artifact_hash)
        ));
    }

    Ok(out)
}

pub fn render_build_summary(statuses: &[BuildStatus], graph: &WorkspaceGraph) -> String {
    let mut out = String::from("summary-v1\n");
    for status in statuses {
        out.push_str(&format!(
            "BUILD_STATUS member={} disposition={} fingerprint={}\n",
            status.member,
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
            .package(name)
            .map(package_uses_implicit_std)
            .unwrap_or(false)
    });
    if uses_std && workspace.package("std").is_some() {
        names.insert("std".to_string());
    }
    Ok(names.into_iter().collect())
}

fn collect_transitive_member_names(
    graph: &WorkspaceGraph,
    root_member: &str,
) -> PackageResult<BTreeSet<String>> {
    let mut pending = vec![root_member.to_string()];
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
    linked.sort_by(|left, right| left.package_name.cmp(&right.package_name));

    let mut direct_deps = root.direct_deps.iter().cloned().collect::<BTreeSet<_>>();
    if linked.iter().any(|package| package.package_name == "std") {
        direct_deps.insert("std".to_string());
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
    let mut routines = root.routines.clone();

    for package in linked {
        modules.extend(package.modules);
        dependency_rows.extend(package.dependency_rows);
        routines.extend(package.routines);
    }

    modules.sort_by(|left, right| left.module_id.cmp(&right.module_id));
    routines.sort_by(|left, right| {
        left.module_id
            .cmp(&right.module_id)
            .then_with(|| left.symbol_name.cmp(&right.symbol_name))
            .then_with(|| left.signature_row.cmp(&right.signature_row))
    });

    let mut linked_package = IrPackage {
        package_name: root.package_name,
        root_module_id: root.root_module_id,
        direct_deps: direct_deps.into_iter().collect(),
        modules,
        dependency_edge_count: dependency_rows.len(),
        dependency_rows: dependency_rows.into_iter().collect(),
        exported_surface_rows: exported_surface_rows.into_iter().collect(),
        runtime_requirements: Vec::new(),
        entrypoints: root.entrypoints,
        routines,
    };
    linked_package.runtime_requirements = derive_runtime_requirements_with_roots(
        &linked_package,
        runtime_requirement_roots_for_kind(root_kind),
    );
    linked_package
}

fn artifact_rel_path(name: &str, fingerprint: &str, kind: &GrimoireKind) -> String {
    let suffix = match kind {
        GrimoireKind::App => "app.artifact.toml",
        GrimoireKind::Lib => "lib.artifact.toml",
    };
    format!(
        "{CACHE_DIR}/{ARTIFACT_DIR}/{name}/{}/{}",
        sanitize_fingerprint(fingerprint),
        suffix
    )
}

fn sanitize_fingerprint(text: &str) -> String {
    text.replace(':', "_")
}

fn validate_prepared_snapshot(
    prepared: &PreparedBuild,
    statuses: &[BuildStatus],
) -> PackageResult<()> {
    let expected_toolchain = current_build_toolchain()?;
    if prepared.toolchain != expected_toolchain {
        return Err(format!(
            "prepared build targets toolchain `{}` but current toolchain is `{expected_toolchain}`",
            prepared.toolchain
        ));
    }
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
        if status.toolchain != prepared.toolchain {
            return Err(format!(
                "build status for `{}` targets toolchain `{}` but prepared build uses `{}`",
                status.member, status.toolchain, prepared.toolchain
            ));
        }
    }
    Ok(())
}

fn lockfile_toolchain(statuses: &[BuildStatus]) -> PackageResult<String> {
    match statuses.first() {
        Some(status) => Ok(status.toolchain.clone()),
        None => current_build_toolchain(),
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
