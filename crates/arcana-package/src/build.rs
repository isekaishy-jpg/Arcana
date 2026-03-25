use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use arcana_aot::{
    AotEmitContext, AotEmitTarget, AotPackageEmission, AotRuntimeBinding, emit_package_with_context,
};
use arcana_hir::{HirResolvedWorkspace, HirWorkspaceSummary, resolve_workspace};
use arcana_ir::{
    IrPackage, RuntimeRequirementRoots, derive_runtime_requirements_with_roots,
    lower_workspace_package_with_resolution,
    render_routine_signature_text,
};

use crate::build_identity::{
    cache_metadata_path_for_output, cached_artifact_matches_status, cached_emission_hash,
    cached_emission_hash_for_path, current_build_toolchain_for_target_with_context,
    render_cached_artifact,
};
use crate::fingerprint::{
    WorkspaceFingerprints, compute_workspace_fingerprints, package_uses_implicit_std,
};
use crate::{
    ARTIFACT_DIR, BuildTarget, CACHE_DIR, GrimoireKind, LOCKFILE_VERSION, LOGS_DIR,
    LockTargetEntry, Lockfile, NativeDependencyDelivery, PackageResult, WorkspaceGraph,
    collect_validated_support_file_paths,
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
    pub(crate) target: BuildTarget,
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

    pub fn target(&self) -> &BuildTarget {
        &self.target
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
pub struct BuildExecutionContext;

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
        |member| Some(planned_target_for_member(&target, &member.kind)),
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
    let packaged_names = collect_transitive_member_names(graph, packaged_member)?;
    plan_build_for_member_targets_with_context(
        graph,
        order,
        prepared,
        existing_lock,
        context,
        |member| {
            if packaged_names.contains(&member.name) {
                Some(planned_package_target_for_member(
                    &target,
                    packaged_member,
                    &member.name,
                ))
            } else {
                None
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
    F: FnMut(&crate::WorkspaceMember) -> Option<BuildTarget>,
{
    let mut statuses = Vec::new();
    let mut upstream_built = HashMap::<String, bool>::new();

    for name in order {
        let member = graph
            .member(name)
            .ok_or_else(|| format!("workspace planner missing member `{name}`"))?;
        let Some(member_target) = target_for_member(member) else {
            continue;
        };
        let member_fingerprints = prepared
            .fingerprints
            .member(name)
            .ok_or_else(|| format!("missing fingerprint for member `{name}`"))?;
        let planned_toolchain =
            current_build_toolchain_for_target_with_context(&member_target, context)?;
        let fingerprint = member_fingerprints.source().to_string();
        let api_fingerprint = member_fingerprints.api().to_string();
        let artifact_rel_path =
            artifact_rel_path(name, &fingerprint, &member.kind, &member_target)?;
        let existing = existing_lock
            .and_then(|lock| lock.members.get(name))
            .and_then(|member| member.target(&member_target));
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
                &member_target,
                &format,
                &planned_toolchain,
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
            target: member_target,
            artifact_rel_path,
            kind: member.kind.clone(),
            format,
            toolchain: planned_toolchain.clone(),
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
    validate_prepared_snapshot_with_context(prepared, statuses, context)?;

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
        let emission = emit_artifact_for_target(
            graph,
            member,
            &status.target,
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
            status.target.key(),
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
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create cache logs directory `{}`: {e}", parent.display()))?;
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
    out.push_str(&format!("order = {}\n\n", format_string_array(order)));

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
        let member = graph
            .member(name)
            .ok_or_else(|| format!("missing workspace member `{name}`"))?;
        out.push_str(&format!(
            "\"{}\" = \"{}\"\n",
            escape_toml(name),
            member.kind.as_str()
        ));
    }
    out.push('\n');

    let merged_builds = merge_lock_build_entries(graph, &names, statuses, existing_lock)?;
    out.push_str("[builds]\n\n");
    for name in &names {
        let builds = merged_builds
            .get(name)
            .ok_or_else(|| format!("missing build entries for `{name}`"))?;
        if builds.is_empty() {
            continue;
        }
        for (target, entry) in builds {
            out.push_str(&format!(
                "[builds.\"{}\".\"{}\"]\n",
                escape_toml(name),
                escape_toml(target.key())
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
) -> PackageResult<BTreeMap<String, BTreeMap<BuildTarget, LockTargetEntry>>> {
    let mut updates = HashMap::<(String, BuildTarget), LockTargetEntry>::new();
    for status in statuses {
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        let artifact_hash = cached_emission_hash_for_path(&artifact_path, &status.target)?;
        updates.insert(
            (status.member.clone(), status.target.clone()),
            LockTargetEntry {
                fingerprint: status.fingerprint.clone(),
                api_fingerprint: status.api_fingerprint.clone(),
                artifact: status.artifact_rel_path.clone(),
                artifact_hash,
                format: status.format.clone(),
                toolchain: status.toolchain.clone(),
            },
        );
    }

    let mut merged = BTreeMap::new();
    for name in names {
        let mut entries = existing_lock
            .and_then(|lock| lock.members.get(name))
            .map(|member| member.targets.clone())
            .unwrap_or_default();
        for ((member_name, target), entry) in &updates {
            if member_name == name {
                entries.insert(target.clone(), entry.clone());
            }
        }
        merged.insert(name.clone(), entries);
    }
    Ok(merged)
}

pub fn render_build_summary(statuses: &[BuildStatus], graph: &WorkspaceGraph) -> String {
    let mut out = String::from("summary-v1\n");
    for status in statuses {
        out.push_str(&format!(
            "BUILD_STATUS member={} target={} disposition={} fingerprint={}\n",
            status.member,
            status.target(),
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

fn select_native_runtime_binding(
    graph: &WorkspaceGraph,
    root_member: &str,
) -> PackageResult<AotRuntimeBinding> {
    let closure = collect_transitive_member_names(graph, root_member)?;
    let mut binding = AotRuntimeBinding::Baked;
    for member_name in closure {
        let member = graph
            .member(&member_name)
            .ok_or_else(|| format!("missing workspace member `{member_name}`"))?;
        for (alias, spec) in &member.direct_dep_specs {
            if spec.native_delivery != NativeDependencyDelivery::Dll {
                continue;
            }
            let package_name = member.direct_dep_packages.get(alias).ok_or_else(|| {
                format!(
                    "workspace member `{}` is missing direct dependency package metadata for alias `{alias}`",
                    member.name
                )
            })?;
            let requested = native_runtime_binding_for_package(package_name).ok_or_else(|| {
                format!(
                    "dependency `{}` (package `{package_name}`) in `{}` requests `native_delivery = \"dll\"`, but that package does not define a supported native runtime DLL lane",
                    alias,
                    member.name
                )
            })?;
            if binding != AotRuntimeBinding::Baked && binding != requested {
                return Err(format!(
                    "native bundle for `{root_member}` requires incompatible DLL runtime bindings"
                ));
            }
            binding = requested;
        }
    }
    Ok(binding)
}

fn native_runtime_binding_for_package(package_name: &str) -> Option<AotRuntimeBinding> {
    match package_name {
        "arcana_desktop" => Some(AotRuntimeBinding::DesktopRuntimeDll),
        _ => None,
    }
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
    let mut owners = root.owners.clone();

    for package in linked {
        modules.extend(package.modules);
        dependency_rows.extend(package.dependency_rows);
        routines.extend(package.routines);
        owners.extend(package.owners);
    }

    modules.sort_by(|left, right| left.module_id.cmp(&right.module_id));
    routines.sort_by(|left, right| {
        left.module_id
            .cmp(&right.module_id)
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
        owners,
    };
    linked_package.runtime_requirements = derive_runtime_requirements_with_roots(
        &linked_package,
        runtime_requirement_roots_for_kind(root_kind),
    );
    linked_package
}

fn emit_artifact_for_target(
    graph: &WorkspaceGraph,
    root_member: &crate::WorkspaceMember,
    target: &BuildTarget,
    root_kind: &GrimoireKind,
    root: IrPackage,
    linked_packages: Vec<IrPackage>,
    build_context: &BuildExecutionContext,
) -> PackageResult<AotPackageEmission> {
    let context = emit_context_for_target(graph, root_member, target, root_kind, build_context)?;
    match target {
        BuildTarget::InternalAot => emit_package_with_context(
            AotEmitTarget::InternalArtifact,
            &link_ir_packages(root_kind, root, linked_packages),
            &context,
        ),
        BuildTarget::WindowsExe => emit_package_with_context(
            AotEmitTarget::WindowsExeBundle,
            &link_ir_packages(root_kind, root, linked_packages),
            &context,
        ),
        BuildTarget::WindowsDll => emit_package_with_context(
            AotEmitTarget::WindowsDllBundle,
            &link_ir_packages(root_kind, root, linked_packages),
            &context,
        ),
        BuildTarget::Other(_) => Err(format!("unsupported build target `{target}`")),
    }
}

fn emit_context_for_target(
    graph: &WorkspaceGraph,
    root_member: &crate::WorkspaceMember,
    target: &BuildTarget,
    kind: &GrimoireKind,
    build_context: &BuildExecutionContext,
) -> PackageResult<AotEmitContext> {
    let root_artifact_file_name = target.artifact_file_name(kind)?.to_string();
    let _ = build_context;
    let runtime_binding = match target {
        BuildTarget::WindowsExe | BuildTarget::WindowsDll => {
            select_native_runtime_binding(graph, &root_member.name)?
        }
        BuildTarget::InternalAot | BuildTarget::Other(_) => AotRuntimeBinding::Baked,
    };
    match target {
        BuildTarget::InternalAot | BuildTarget::WindowsExe | BuildTarget::WindowsDll => {
            Ok(AotEmitContext {
                root_artifact_file_name: Some(root_artifact_file_name),
                runtime_binding,
            })
        }
        BuildTarget::Other(_) => Err(format!("unsupported build target `{target}`")),
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
    name: &str,
    fingerprint: &str,
    kind: &GrimoireKind,
    target: &BuildTarget,
) -> PackageResult<String> {
    Ok(format!(
        "{CACHE_DIR}/{ARTIFACT_DIR}/{name}/{}/{}/{}",
        target.key(),
        sanitize_fingerprint(fingerprint),
        target.artifact_file_name(kind)?
    ))
}

fn sanitize_fingerprint(text: &str) -> String {
    text.replace(':', "_")
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

    fn dummy_emission(files: Vec<AotEmissionFile>) -> AotPackageEmission {
        AotPackageEmission {
            target: AotEmitTarget::InternalArtifact,
            artifact: AotPackageArtifact {
                format: AOT_INTERNAL_FORMAT.to_string(),
                package_name: "tool".to_string(),
                root_module_id: "tool".to_string(),
                direct_deps: Vec::new(),
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
