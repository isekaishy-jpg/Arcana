use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use arcana_aot::{
    AOT_INTERNAL_FORMAT, AotPackageArtifact, compile_package, render_package_artifact,
};
use arcana_hir::{
    HirWorkspacePackage, HirWorkspaceSummary, build_package_layout, build_package_summary,
    build_workspace_package, build_workspace_summary, derive_source_module_path, lower_module_text,
    resolve_workspace,
};
use arcana_ir::lower_workspace_package_with_resolution;
use pathdiff::diff_paths;

pub type PackageResult<T> = Result<T, String>;

const LOCKFILE_VERSION: i64 = 1;
const CACHE_DIR: &str = ".arcana";
const ARTIFACT_DIR: &str = "artifacts";
const LOGS_DIR: &str = "logs";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrimoireKind {
    App,
    Lib,
}

impl GrimoireKind {
    pub fn root_file_name(&self) -> &'static str {
        match self {
            Self::App => "shelf.arc",
            Self::Lib => "book.arc",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Lib => "lib",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencySource {
    Path,
    Git,
    Registry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencySpec {
    pub source: DependencySource,
    pub location: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub kind: GrimoireKind,
    pub workspace_members: Vec<String>,
    pub deps: BTreeMap<String, DependencySpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceMember {
    pub name: String,
    pub kind: GrimoireKind,
    pub rel_dir: String,
    pub abs_dir: PathBuf,
    pub deps: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceGraph {
    pub root_name: String,
    pub root_dir: PathBuf,
    pub members: Vec<WorkspaceMember>,
}

impl WorkspaceGraph {
    pub fn member(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members.iter().find(|member| member.name == name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockMember {
    pub path: String,
    pub deps: Vec<String>,
    pub fingerprint: String,
    pub api_fingerprint: String,
    pub artifact: String,
    pub kind: GrimoireKind,
    pub format: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lockfile {
    pub version: i64,
    pub workspace: String,
    pub order: Vec<String>,
    pub members: BTreeMap<String, LockMember>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildDisposition {
    Built,
    CacheHit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildStatus {
    pub member: String,
    pub disposition: BuildDisposition,
    pub fingerprint: String,
    pub api_fingerprint: String,
    pub artifact_rel_path: String,
    pub kind: GrimoireKind,
    pub format: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberFingerprints {
    pub source: String,
    pub api: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingMember {
    name: String,
    kind: GrimoireKind,
    abs_dir: PathBuf,
    rel_dir: String,
    dep_paths: Vec<PathBuf>,
}

pub fn parse_manifest(path: &Path) -> PackageResult<Manifest> {
    let src = fs::read_to_string(path)
        .map_err(|e| format!("failed to read `{}`: {e}", path.display()))?;
    let parsed: toml::Value = src
        .parse()
        .map_err(|e| format!("failed to parse `{}` as TOML: {e}", path.display()))?;
    let Some(table) = parsed.as_table() else {
        return Err(format!(
            "manifest root must be a table in `{}`",
            path.display()
        ));
    };

    let name = table
        .get("name")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("missing `name` in `{}`", path.display()))?
        .to_string();
    let kind = match table
        .get("kind")
        .and_then(toml::Value::as_str)
        .unwrap_or("app")
    {
        "app" => GrimoireKind::App,
        "lib" => GrimoireKind::Lib,
        other => {
            return Err(format!(
                "`kind` must be \"app\" or \"lib\" in `{}` (found `{other}`)",
                path.display()
            ));
        }
    };

    let workspace_members = table
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("members"))
        .map(parse_string_array)
        .transpose()?
        .unwrap_or_default();

    let mut deps = BTreeMap::new();
    if let Some(dep_table) = table.get("deps").and_then(toml::Value::as_table) {
        for (name, value) in dep_table {
            let spec = parse_dependency_spec(name, value, path)?;
            if spec.source != DependencySource::Path {
                return Err(format!(
                    "dependency `{name}` in `{}` uses unsupported source; only `path` is enabled before selfhost",
                    path.display()
                ));
            }
            deps.insert(name.clone(), spec);
        }
    }

    Ok(Manifest {
        name,
        kind,
        workspace_members,
        deps,
    })
}

pub fn load_workspace_graph(root_dir: &Path) -> PackageResult<WorkspaceGraph> {
    let root_dir = canonicalize_dir(root_dir)?;
    let root_manifest_path = root_dir.join("book.toml");
    let root_manifest = parse_manifest(&root_manifest_path)?;
    let root_name = root_manifest.name.clone();

    let mut seed_paths = root_manifest
        .workspace_members
        .iter()
        .map(|rel| canonicalize_dir(&root_dir.join(rel)))
        .collect::<PackageResult<Vec<_>>>()?;
    if seed_paths.is_empty() || has_root_module(&root_dir, &root_manifest.kind) {
        seed_paths.push(root_dir.clone());
    }

    let mut queue = VecDeque::from(seed_paths);
    let mut pending_by_dir = BTreeMap::<PathBuf, PendingMember>::new();
    let mut name_to_dir = HashMap::<String, PathBuf>::new();
    let mut visited = BTreeSet::<PathBuf>::new();

    while let Some(abs_dir) = queue.pop_front() {
        if !visited.insert(abs_dir.clone()) {
            continue;
        }

        let manifest_path = abs_dir.join("book.toml");
        let manifest = parse_manifest(&manifest_path)?;
        validate_grimoire_layout(&abs_dir, &manifest.kind)?;
        let rel_dir = relative_from_root(&abs_dir, &root_dir)?;
        if let Some(existing) = name_to_dir.insert(manifest.name.clone(), abs_dir.clone()) {
            if existing != abs_dir {
                return Err(format!(
                    "duplicate grimoire name `{}` at `{}` and `{}`",
                    manifest.name,
                    existing.display(),
                    abs_dir.display()
                ));
            }
        }

        let mut dep_paths = Vec::new();
        for dep in manifest.deps.values() {
            let dep_dir = canonicalize_dir(&abs_dir.join(&dep.location))?;
            dep_paths.push(dep_dir.clone());
            queue.push_back(dep_dir);
        }

        pending_by_dir.insert(
            abs_dir.clone(),
            PendingMember {
                name: manifest.name,
                kind: manifest.kind,
                abs_dir,
                rel_dir,
                dep_paths,
            },
        );
    }

    let mut members = pending_by_dir
        .values()
        .map(|member| {
            let deps = member
                .dep_paths
                .iter()
                .map(|path| {
                    pending_by_dir
                        .get(path)
                        .map(|dep| dep.name.clone())
                        .ok_or_else(|| {
                            format!(
                                "dependency at `{}` was not loaded into the workspace graph",
                                path.display()
                            )
                        })
                })
                .collect::<PackageResult<Vec<_>>>()?;
            Ok(WorkspaceMember {
                name: member.name.clone(),
                kind: member.kind.clone(),
                rel_dir: member.rel_dir.clone(),
                abs_dir: member.abs_dir.clone(),
                deps,
            })
        })
        .collect::<PackageResult<Vec<_>>>()?;

    members.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(WorkspaceGraph {
        root_name,
        root_dir,
        members,
    })
}

pub fn load_workspace_hir(root_dir: &Path) -> PackageResult<HirWorkspaceSummary> {
    let root_dir = canonicalize_dir(root_dir)?;
    let graph = load_workspace_graph(&root_dir)?;
    load_workspace_hir_from_graph(&root_dir, &graph)
}

pub fn load_workspace_hir_from_graph(
    root_dir: &Path,
    graph: &WorkspaceGraph,
) -> PackageResult<HirWorkspaceSummary> {
    let root_dir = canonicalize_dir(root_dir)?;
    let mut packages = Vec::new();

    let root_manifest = parse_manifest(&root_dir.join("book.toml"))?;
    let root_already_in_graph = graph
        .members
        .iter()
        .any(|member| member.abs_dir == root_dir);
    if !root_already_in_graph && has_root_module(&root_dir, &root_manifest.kind) {
        packages.push(load_package_hir(
            &root_dir,
            &root_manifest.name,
            &root_manifest.kind,
            root_manifest.deps.keys().cloned().collect(),
        )?);
    }

    for member in &graph.members {
        packages.push(load_member_hir_package(member)?);
    }

    if let Some(std_dir) = find_implicit_std(&root_dir)? {
        let manifest = parse_manifest(&std_dir.join("book.toml"))?;
        let has_std = packages
            .iter()
            .any(|package| package.summary.package_name == manifest.name);
        if !has_std {
            packages.push(load_package_hir(
                &std_dir,
                &manifest.name,
                &manifest.kind,
                BTreeSet::new(),
            )?);
        }
    }

    build_workspace_summary(packages)
}

pub fn load_member_hir_package(member: &WorkspaceMember) -> PackageResult<HirWorkspacePackage> {
    load_package_hir(
        &member.abs_dir,
        &member.name,
        &member.kind,
        member.deps.iter().cloned().collect(),
    )
}

pub fn plan_workspace(graph: &WorkspaceGraph) -> PackageResult<Vec<String>> {
    let indegree = graph
        .members
        .iter()
        .map(|member| (member.name.clone(), member.deps.len()))
        .collect::<HashMap<_, _>>();
    let mut indegree = indegree;
    let mut dependents = HashMap::<String, Vec<String>>::new();
    for member in &graph.members {
        for dep in &member.deps {
            dependents
                .entry(dep.clone())
                .or_default()
                .push(member.name.clone());
        }
    }

    let mut ready = graph
        .members
        .iter()
        .filter(|member| member.deps.is_empty())
        .map(|member| member.name.clone())
        .collect::<Vec<_>>();
    ready.sort();
    let mut ordered = Vec::with_capacity(graph.members.len());

    while let Some(name) = ready.first().cloned() {
        ready.remove(0);
        ordered.push(name.clone());
        let mut next = dependents.get(&name).cloned().unwrap_or_default();
        next.sort();
        for dependent in next {
            let count = indegree
                .get_mut(&dependent)
                .ok_or_else(|| format!("workspace planner missing member `{dependent}`"))?;
            *count -= 1;
            if *count == 0 {
                ready.push(dependent);
                ready.sort();
            }
        }
    }

    if ordered.len() != graph.members.len() {
        return Err("workspace dependency cycle detected".to_string());
    }
    Ok(ordered)
}

pub fn read_lockfile(path: &Path) -> PackageResult<Option<Lockfile>> {
    if !path.is_file() {
        return Ok(None);
    }
    let src = fs::read_to_string(path)
        .map_err(|e| format!("failed to read `{}`: {e}", path.display()))?;
    let parsed: toml::Value = src
        .parse()
        .map_err(|e| format!("failed to parse `{}` as TOML: {e}", path.display()))?;
    let Some(table) = parsed.as_table() else {
        return Err(format!(
            "lockfile root must be a TOML table in `{}`",
            path.display()
        ));
    };
    let version = table
        .get("version")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| format!("missing `version` in `{}`", path.display()))?;
    if version != LOCKFILE_VERSION {
        return Err(format!(
            "unsupported lockfile version `{version}` in `{}`; expected {LOCKFILE_VERSION}",
            path.display()
        ));
    }
    let workspace = table
        .get("workspace")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("missing `workspace` in `{}`", path.display()))?
        .to_string();
    let order = table
        .get("order")
        .map(parse_string_array)
        .transpose()?
        .unwrap_or_default();

    let paths = table
        .get("paths")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[paths]` in `{}`", path.display()))?;
    let deps = table
        .get("deps")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[deps]` in `{}`", path.display()))?;
    let fingerprints = table
        .get("fingerprints")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[fingerprints]` in `{}`", path.display()))?;
    let api_fingerprints = table
        .get("api_fingerprints")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[api_fingerprints]` in `{}`", path.display()))?;
    let artifacts = table
        .get("artifacts")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[artifacts]` in `{}`", path.display()))?;
    let kinds = table
        .get("kinds")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[kinds]` in `{}`", path.display()))?;
    let formats = table
        .get("formats")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[formats]` in `{}`", path.display()))?;

    let mut members = BTreeMap::new();
    for (name, path_value) in paths {
        let path = path_value
            .as_str()
            .ok_or_else(|| format!("lockfile path entry for `{name}` must be a string"))?
            .to_string();
        let dep_list = deps
            .get(name)
            .map(parse_string_array)
            .transpose()?
            .unwrap_or_default();
        let fingerprint = fingerprints
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing fingerprint for `{name}`"))?
            .to_string();
        let api_fingerprint = api_fingerprints
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing api fingerprint for `{name}`"))?
            .to_string();
        let artifact = artifacts
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing artifact path for `{name}`"))?
            .to_string();
        let kind = match kinds
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing kind for `{name}`"))?
        {
            "app" => GrimoireKind::App,
            "lib" => GrimoireKind::Lib,
            other => return Err(format!("unsupported kind `{other}` for `{name}`")),
        };
        let format = formats
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing format for `{name}`"))?
            .to_string();
        members.insert(
            name.clone(),
            LockMember {
                path,
                deps: dep_list,
                fingerprint,
                api_fingerprint,
                artifact,
                kind,
                format,
            },
        );
    }

    Ok(Some(Lockfile {
        version,
        workspace,
        order,
        members,
    }))
}

pub fn plan_build(
    graph: &WorkspaceGraph,
    order: &[String],
    fingerprints: &HashMap<String, MemberFingerprints>,
    existing_lock: Option<&Lockfile>,
) -> PackageResult<Vec<BuildStatus>> {
    let mut statuses = Vec::new();
    let mut api_changed = HashMap::<String, bool>::new();

    for name in order {
        let member = graph
            .member(name)
            .ok_or_else(|| format!("workspace planner missing member `{name}`"))?;
        let member_fingerprints = fingerprints
            .get(name)
            .ok_or_else(|| format!("missing fingerprint for member `{name}`"))?;
        let fingerprint = member_fingerprints.source.clone();
        let api_fingerprint = member_fingerprints.api.clone();
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
        let upstream_api_changed = member
            .deps
            .iter()
            .any(|dep| api_changed.get(dep).copied().unwrap_or(false));
        let built = fingerprint_changed
            || api_fingerprint_changed
            || upstream_api_changed
            || format_changed
            || !cached_artifact_matches_format(&artifact_abs_path, &format);
        api_changed.insert(name.clone(), api_fingerprint_changed);
        statuses.push(BuildStatus {
            member: name.clone(),
            disposition: if built {
                BuildDisposition::Built
            } else {
                BuildDisposition::CacheHit
            },
            fingerprint,
            api_fingerprint,
            artifact_rel_path,
            kind: member.kind.clone(),
            format,
        });
    }

    Ok(statuses)
}

pub fn execute_build(graph: &WorkspaceGraph, statuses: &[BuildStatus]) -> PackageResult<PathBuf> {
    let cache_root = graph.root_dir.join(CACHE_DIR);
    let workspace = load_workspace_hir_from_graph(&graph.root_dir, graph)?;
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
        let linked_package_names = collect_linked_package_names(graph, &workspace, &status.member)?;
        let root_package = lowered_packages
            .get(&member.name)
            .cloned()
            .ok_or_else(|| format!("missing lowered package `{}`", member.name))?;
        let linked_packages = linked_package_names
            .into_iter()
            .filter(|name| name != &member.name)
            .map(|name| {
                lowered_packages
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| format!("missing lowered linked package `{name}`"))
            })
            .collect::<PackageResult<Vec<_>>>()?;
        let artifact = compile_package(&link_ir_packages(root_package, linked_packages));
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
        let rendered = format!(
            "member = \"{}\"\nkind = \"{}\"\nfingerprint = \"{}\"\napi_fingerprint = \"{}\"\n{}",
            status.member,
            status.kind.as_str(),
            status.fingerprint,
            status.api_fingerprint,
            render_package_artifact(&artifact)
        );
        fs::write(&artifact_path, rendered).map_err(|e| {
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

fn package_uses_implicit_std(package: &HirWorkspacePackage) -> bool {
    package.summary.dependency_edges.iter().any(|edge| {
        edge.target_path
            .first()
            .is_some_and(|segment| segment == "std")
    })
}

fn link_ir_packages(
    root: arcana_ir::IrPackage,
    mut linked: Vec<arcana_ir::IrPackage>,
) -> arcana_ir::IrPackage {
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
    let mut exported_surface_rows = root
        .exported_surface_rows
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut runtime_requirements = root
        .runtime_requirements
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut routines = root.routines.clone();

    for package in linked {
        modules.extend(package.modules);
        dependency_rows.extend(package.dependency_rows);
        exported_surface_rows.extend(package.exported_surface_rows);
        runtime_requirements.extend(package.runtime_requirements);
        routines.extend(package.routines);
    }

    modules.sort_by(|left, right| left.module_id.cmp(&right.module_id));
    routines.sort_by(|left, right| {
        left.module_id
            .cmp(&right.module_id)
            .then_with(|| left.symbol_name.cmp(&right.symbol_name))
            .then_with(|| left.signature_row.cmp(&right.signature_row))
    });

    arcana_ir::IrPackage {
        package_name: root.package_name,
        root_module_id: root.root_module_id,
        direct_deps: direct_deps.into_iter().collect(),
        modules,
        dependency_edge_count: dependency_rows.len(),
        dependency_rows: dependency_rows.into_iter().collect(),
        exported_surface_rows: exported_surface_rows.into_iter().collect(),
        runtime_requirements: runtime_requirements.into_iter().collect(),
        entrypoints: root.entrypoints,
        routines,
    }
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
    let mut out = String::new();
    out.push_str(&format!("version = {LOCKFILE_VERSION}\n"));
    out.push_str(&format!(
        "workspace = \"{}\"\n",
        escape_toml(&graph.root_name)
    ));
    out.push_str(&format!(
        "toolchain = \"arcana-cli {}\"\n",
        env!("CARGO_PKG_VERSION")
    ));
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

    Ok(out)
}

pub fn validate_path(path: &Path) -> PackageResult<()> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) != Some("arc") {
            return Err(format!("source file must use `.arc`: `{}`", path.display()));
        }
        let src = fs::read_to_string(path)
            .map_err(|e| format!("failed to read `{}`: {e}", path.display()))?;
        if src.is_empty() {
            return Err(format!("source file is empty: `{}`", path.display()));
        }
        return Ok(());
    }

    let graph = load_workspace_graph(path)?;
    for member in &graph.members {
        validate_grimoire_layout(&member.abs_dir, &member.kind)?;
    }
    Ok(())
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

fn parse_string_array(value: &toml::Value) -> PackageResult<Vec<String>> {
    let Some(array) = value.as_array() else {
        return Err("expected array of strings".to_string());
    };
    let mut out = Vec::with_capacity(array.len());
    for item in array {
        let Some(text) = item.as_str() else {
            return Err("expected array of strings".to_string());
        };
        out.push(text.to_string());
    }
    Ok(out)
}

fn parse_dependency_spec(
    name: &str,
    value: &toml::Value,
    manifest_path: &Path,
) -> PackageResult<DependencySpec> {
    if let Some(path) = value.as_str() {
        return Ok(DependencySpec {
            source: DependencySource::Path,
            location: path.to_string(),
        });
    }
    let Some(table) = value.as_table() else {
        return Err(format!(
            "dependency `{name}` in `{}` must be a string or table",
            manifest_path.display()
        ));
    };
    if let Some(path) = table.get("path").and_then(toml::Value::as_str) {
        return Ok(DependencySpec {
            source: DependencySource::Path,
            location: path.to_string(),
        });
    }
    if let Some(git) = table.get("git").and_then(toml::Value::as_str) {
        return Ok(DependencySpec {
            source: DependencySource::Git,
            location: git.to_string(),
        });
    }
    if let Some(registry) = table.get("registry").and_then(toml::Value::as_str) {
        return Ok(DependencySpec {
            source: DependencySource::Registry,
            location: registry.to_string(),
        });
    }
    Err(format!(
        "dependency `{name}` in `{}` must set `path`, `git`, or `registry`",
        manifest_path.display()
    ))
}

fn validate_grimoire_layout(dir: &Path, kind: &GrimoireKind) -> PackageResult<()> {
    if !dir.is_dir() {
        return Err(format!(
            "grimoire path is not a directory: `{}`",
            dir.display()
        ));
    }
    let src = dir.join("src");
    if !src.is_dir() {
        return Err(format!("missing `src` directory in `{}`", dir.display()));
    }
    let root = src.join(kind.root_file_name());
    if !root.is_file() {
        return Err(format!(
            "missing root file `{}` in `{}`",
            kind.root_file_name(),
            dir.display()
        ));
    }
    let types = src.join("types.arc");
    if !types.is_file() {
        return Err(format!("missing `src/types.arc` in `{}`", dir.display()));
    }
    Ok(())
}

fn load_package_hir(
    root_dir: &Path,
    name: &str,
    kind: &GrimoireKind,
    direct_deps: BTreeSet<String>,
) -> PackageResult<HirWorkspacePackage> {
    let files = collect_arc_files(&root_dir.join("src"))?;
    build_package_hir(root_dir, name, kind, direct_deps, &files)
}

fn build_package_hir(
    root_dir: &Path,
    name: &str,
    kind: &GrimoireKind,
    direct_deps: BTreeSet<String>,
    files: &[PathBuf],
) -> PackageResult<HirWorkspacePackage> {
    let src_dir = root_dir.join("src");
    let root_file = src_dir.join(kind.root_file_name());
    if !root_file.is_file() {
        return Err(format!(
            "missing `{}` in `{}`",
            kind.root_file_name(),
            src_dir.display()
        ));
    }

    let mut module_paths = BTreeMap::new();
    let mut modules = Vec::new();
    let mut relative_to_absolute = BTreeMap::new();
    for file in files {
        let source_path = derive_source_module_path(name, kind.root_file_name(), &src_dir, file)?;
        let relative_key = source_path.relative_segments.join(".");
        let module_id = source_path.module_id;
        let source = fs::read_to_string(file)
            .map_err(|e| format!("failed to read `{}`: {e}", file.display()))?;
        let module = lower_module_text(module_id, &source)
            .map_err(|err| format!("{}: {err}", file.display()))?;
        if module_paths
            .insert(module.module_id.clone(), file.to_path_buf())
            .is_some()
        {
            return Err(format!(
                "duplicate module path `{}` in `{}`",
                module.module_id,
                root_dir.display()
            ));
        }
        if !relative_key.is_empty() {
            if relative_to_absolute
                .insert(relative_key.clone(), module.module_id.clone())
                .is_some()
            {
                return Err(format!(
                    "duplicate module path `{relative_key}` in `{}`",
                    root_dir.display()
                ));
            }
        }
        modules.push(module);
    }

    let summary = build_package_summary(name.to_string(), modules);
    let layout = build_package_layout(&summary, module_paths, relative_to_absolute)?;
    build_workspace_package(root_dir.to_path_buf(), direct_deps, summary, layout)
}

fn collect_arc_files(dir: &Path) -> PackageResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_files_recursive(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> PackageResult<()> {
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("failed to read directory `{}`: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("arc") {
            out.push(path);
        }
    }
    Ok(())
}

fn has_root_module(root_dir: &Path, kind: &GrimoireKind) -> bool {
    root_dir.join("src").join(kind.root_file_name()).is_file()
}

fn find_implicit_std(start: &Path) -> PackageResult<Option<PathBuf>> {
    let mut cursor = if start.is_file() {
        start.parent().map(Path::to_path_buf)
    } else {
        Some(start.to_path_buf())
    };

    while let Some(dir) = cursor {
        let candidate = dir.join("std").join("book.toml");
        if candidate.is_file() {
            let std_dir = candidate.parent().ok_or_else(|| {
                format!(
                    "failed to resolve implicit std from `{}`",
                    candidate.display()
                )
            })?;
            let canonical = fs::canonicalize(std_dir).map_err(|err| {
                format!(
                    "failed to open implicit std package `{}`: {err}",
                    std_dir.display()
                )
            })?;
            return Ok(Some(canonical));
        }
        cursor = dir.parent().map(Path::to_path_buf);
    }

    Ok(None)
}

fn canonicalize_dir(path: &Path) -> PackageResult<PathBuf> {
    path.canonicalize()
        .map_err(|e| format!("failed to resolve `{}`: {e}", path.display()))
}

fn relative_from_root(path: &Path, root: &Path) -> PackageResult<String> {
    let rel = diff_paths(path, root).ok_or_else(|| {
        format!(
            "failed to compute relative path from `{}` to `{}`",
            root.display(),
            path.display()
        )
    })?;
    let rendered = rel.to_string_lossy().replace('\\', "/");
    if rendered.is_empty() {
        Ok(".".to_string())
    } else {
        Ok(rendered)
    }
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

fn cached_artifact_matches_format(path: &Path, expected_format: &str) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(artifact) = toml::from_str::<AotPackageArtifact>(&text) else {
        return false;
    };
    artifact.format == expected_format
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
    use super::*;
    use arcana_aot::parse_package_artifact;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("arcana_package_{name}_{nanos}"))
    }

    fn write_file(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, text).expect("write file");
    }

    fn write_grimoire(dir: &Path, kind: GrimoireKind, name: &str, deps: &[(&str, &str)]) {
        let root_file = kind.root_file_name();
        let mut manifest = format!("name = \"{name}\"\nkind = \"{}\"\n", kind.as_str());
        if !deps.is_empty() {
            manifest.push_str("\n[deps]\n");
            for (dep_name, dep_path) in deps {
                manifest.push_str(&format!("{dep_name} = {{ path = \"{dep_path}\" }}\n"));
            }
        }
        write_file(&dir.join("book.toml"), &manifest);
        write_file(
            &dir.join("src").join(root_file),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src/types.arc"), "// types\n");
    }

    fn fake_member_fingerprints(graph: &WorkspaceGraph) -> HashMap<String, MemberFingerprints> {
        graph
            .members
            .iter()
            .map(|member| {
                (
                    member.name.clone(),
                    MemberFingerprints {
                        source: format!("test-source:{}", member.name),
                        api: format!("test-api:{}", member.name),
                    },
                )
            })
            .collect()
    }

    fn mutate_member_fingerprint(
        fingerprints: &HashMap<String, MemberFingerprints>,
        member: &str,
        source_suffix: Option<&str>,
        api_suffix: Option<&str>,
    ) -> HashMap<String, MemberFingerprints> {
        let mut next = fingerprints.clone();
        let fingerprint = next
            .get_mut(member)
            .expect("member fingerprint should exist");
        if let Some(source_suffix) = source_suffix {
            fingerprint.source.push(':');
            fingerprint.source.push_str(source_suffix);
        }
        if let Some(api_suffix) = api_suffix {
            fingerprint.api.push(':');
            fingerprint.api.push_str(api_suffix);
        }
        next
    }

    #[test]
    fn parse_manifest_rejects_non_path_deps() {
        let dir = temp_dir("manifest_git_dep");
        write_file(
            &dir.join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ncore = { git = \"https://example.com/repo\" }\n",
        );
        let err = parse_manifest(&dir.join("book.toml")).expect_err("expected git rejection");
        assert!(err.contains("only `path` is enabled"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_workspace_graph_resolves_recursive_local_deps() {
        let dir = temp_dir("graph_recursive");
        write_file(
            &dir.join("book.toml"),
            "name = \"workspace\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("core", "../core")],
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);

        let graph = load_workspace_graph(&dir).expect("load graph");
        assert_eq!(graph.members.len(), 2);
        assert!(graph.member("app").is_some());
        assert!(graph.member("core").is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_workspace_graph_includes_root_package_when_present() {
        let dir = temp_dir("graph_root_member");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"workspace\"\n",
                "kind = \"app\"\n",
                "[workspace]\n",
                "members = [\"app\"]\n",
                "[deps]\n",
                "core = { path = \"core\" }\n",
            ),
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            "import core\nfn main() -> Int:\n    return core.value :: :: call\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// root types\n");
        write_grimoire(&dir.join("app"), GrimoireKind::App, "app", &[]);
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("core/src/book.arc"),
            "export fn value() -> Int:\n    return 7\n",
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        assert_eq!(
            graph
                .members
                .iter()
                .map(|member| member.name.as_str())
                .collect::<Vec<_>>(),
            vec!["app", "core", "workspace"]
        );
        let root = graph
            .member("workspace")
            .expect("root package should be in workspace graph");
        assert_eq!(root.rel_dir, ".");
        assert_eq!(root.deps, vec!["core".to_string()]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_workspace_hir_includes_root_package_and_implicit_std() {
        let dir = temp_dir("workspace_hir");
        write_file(
            &dir.join("book.toml"),
            "name = \"workspace\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            "use std.io.print\nfn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");
        write_grimoire(&dir.join("app"), GrimoireKind::App, "app", &[]);
        write_grimoire(&dir.join("std"), GrimoireKind::Lib, "std", &[]);
        write_file(
            &dir.join("std/src/book.arc"),
            "export fn print() -> Int:\n    return 0\n",
        );

        let workspace = load_workspace_hir(&dir).expect("workspace hir should load");
        assert!(workspace.package("workspace").is_some());
        assert!(workspace.package("app").is_some());
        assert!(workspace.package("std").is_some());
        assert!(
            workspace
                .package("workspace")
                .expect("root package should exist")
                .summary
                .dependency_edges
                .iter()
                .any(|edge| edge.target_path
                    == vec!["std".to_string(), "io".to_string(), "print".to_string()])
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_workspace_is_deterministic() {
        let dir = temp_dir("plan");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"core\", \"gfx\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("core", "../core"), ("gfx", "../gfx")],
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_grimoire(
            &dir.join("gfx"),
            GrimoireKind::Lib,
            "gfx",
            &[("core", "../core")],
        );
        let graph = load_workspace_graph(&dir).expect("load graph");
        let first = plan_workspace(&graph).expect("plan");
        let second = plan_workspace(&graph).expect("plan");
        assert_eq!(first, second);
        assert_eq!(
            first,
            vec!["core".to_string(), "gfx".to_string(), "app".to_string()]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_lockfile_is_deterministic() {
        let dir = temp_dir("lock");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"core\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("core", "../core")],
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let fingerprints = fake_member_fingerprints(&graph);
        let statuses = plan_build(&graph, &order, &fingerprints, None).expect("build plan");
        let first = render_lockfile(&graph, &order, &statuses).expect("render");
        let second = render_lockfile(&graph, &order, &statuses).expect("render");
        assert_eq!(first, second);
        assert!(first.contains("version = 1"));
        assert!(first.contains("[api_fingerprints]"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_build_is_cache_hit_only() {
        let dir = temp_dir("cache_hit");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_grimoire(&dir.join("app"), GrimoireKind::App, "app", &[]);
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = fake_member_fingerprints(&graph);
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let second_fingerprints = first_fingerprints.clone();
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert!(
            second_statuses
                .iter()
                .all(|status| status.disposition == BuildDisposition::CacheHit)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_workspace_includes_root_package_in_lockfile() {
        let dir = temp_dir("root_build");
        write_file(
            &dir.join("book.toml"),
            "name = \"workspace\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// root types\n");
        write_grimoire(&dir.join("app"), GrimoireKind::App, "app", &[]);

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        assert_eq!(order, vec!["app".to_string(), "workspace".to_string()]);

        let fingerprints = fake_member_fingerprints(&graph);
        let statuses = plan_build(&graph, &order, &fingerprints, None).expect("build plan");
        execute_build(&graph, &statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &statuses).expect("write lockfile");
        let lock = read_lockfile(&lock_path)
            .expect("read lockfile")
            .expect("lockfile should exist");
        let root = lock
            .members
            .get("workspace")
            .expect("root package should be written to lockfile");
        assert_eq!(root.path, ".");
        assert!(dir.join(&root.artifact).is_file());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_cached_artifact_format_triggers_rebuild() {
        let dir = temp_dir("invalid_artifact_format");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = fake_member_fingerprints(&graph);
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let status = first_statuses
            .iter()
            .find(|status| status.member == "app")
            .expect("app status should exist");
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        let stale = fs::read_to_string(&artifact_path).expect("artifact should exist");
        fs::write(
            &artifact_path,
            stale.replace(
                &format!("format = \"{AOT_INTERNAL_FORMAT}\""),
                "format = \"arcana-aot-v3\"",
            ),
        )
        .expect("artifact should be rewritten");

        let second_statuses =
            plan_build(&graph, &order, &first_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member, "app");
        assert_eq!(second_statuses[0].disposition, BuildDisposition::Built);

        execute_build(&graph, &second_statuses).expect("rebuild should refresh artifact");
        let refreshed = fs::read_to_string(&artifact_path).expect("artifact should exist");
        let parsed = parse_package_artifact(&refreshed).expect("artifact should parse");
        assert_eq!(parsed.format, AOT_INTERNAL_FORMAT);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn built_artifact_includes_public_surface_rows() {
        let dir = temp_dir("artifact_surface");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"core\"]\n",
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("core/src/book.arc"),
            "reexport types\nexport fn shared_value() -> Int:\n    return 0\n",
        );
        write_file(
            &dir.join("core/src/types.arc"),
            concat!(
                "export record Counter:\n",
                "    value: Int\n",
                "impl Counter:\n",
                "    fn next(read self: Counter) -> Int:\n",
                "        return self.value + 1\n",
            ),
        );
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let fingerprints = fake_member_fingerprints(&graph);
        let statuses = plan_build(&graph, &order, &fingerprints, None).expect("plan");
        execute_build(&graph, &statuses).expect("execute");

        let core = statuses
            .iter()
            .find(|status| status.member == "core")
            .expect("core status");
        let artifact = fs::read_to_string(graph.root_dir.join(&core.artifact_rel_path))
            .expect("artifact should exist");
        let parsed = parse_package_artifact(&artifact).expect("artifact should parse");
        assert_eq!(parsed.format, AOT_INTERNAL_FORMAT);
        assert_eq!(parsed.package_name, "core");
        assert_eq!(parsed.root_module_id, "core");
        assert_eq!(parsed.module_count, 2);
        assert_eq!(parsed.dependency_edge_count, 1);
        assert!(parsed.direct_deps.is_empty());
        assert!(
            parsed
                .dependency_rows
                .iter()
                .any(|row| row.contains("types"))
        );
        assert!(parsed.runtime_requirements.is_empty());
        assert!(parsed.entrypoints.is_empty());
        assert_eq!(parsed.routines.len(), 2);
        assert!(
            parsed
                .routines
                .iter()
                .any(|routine| routine.symbol_name == "shared_value")
        );
        assert!(
            parsed
                .exported_surface_rows
                .iter()
                .any(|row| row == "module=core:export:fn:fn shared_value() -> Int:")
        );
        assert!(
            parsed
                .exported_surface_rows
                .iter()
                .any(|row| row == "module=core:reexport:types")
        );
        assert!(
            parsed
                .exported_surface_rows
                .iter()
                .any(|row| row == "module=core.types:export:record:record Counter:\\nvalue: Int")
        );
        assert!(
            parsed.exported_surface_rows.iter().any(|row| row
                == "module=core.types:impl:target=Counter:trait=:methods=[fn:fn next(read self: Counter) -> Int:]"),
            "expected public impl surface rows in artifact: {artifact}"
        );
        assert_eq!(parsed.modules[0].module_id, "core");
        assert_eq!(parsed.modules[0].symbol_count, 1);
        assert_eq!(parsed.modules[0].line_count, 3);
        assert_eq!(parsed.routines[0].routine_key, "core#sym-0");
        assert_eq!(
            parsed.routines[0].statements,
            vec![arcana_ir::ExecStmt::ReturnValue {
                value: arcana_ir::ExecExpr::Int(0),
            }]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn built_artifact_carries_resolved_bare_method_identity() {
        let dir = temp_dir("artifact_bare_method_identity");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"std\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("std", "../std")],
        );
        write_grimoire(&dir.join("std"), GrimoireKind::Lib, "std", &[]);
        write_file(&dir.join("std/src/book.arc"), "import collections.list\n");
        write_file(
            &dir.join("std/src/collections.arc"),
            "import collections.list\n",
        );
        write_file(
            &dir.join("std/src/collections/list.arc"),
            "impl List[T]:\n    fn len(read self: List[T]) -> Int:\n        return 0\n",
        );
        write_file(
            &dir.join("app/src/shelf.arc"),
            "import std.collections.list\nfn main() -> Int:\n    let xs = [1]\n    return xs :: :: len\n",
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let fingerprints = fake_member_fingerprints(&graph);
        let statuses = plan_build(&graph, &order, &fingerprints, None).expect("plan");
        execute_build(&graph, &statuses).expect("execute");

        let app = statuses
            .iter()
            .find(|status| status.member == "app")
            .expect("app status");
        let artifact = fs::read_to_string(graph.root_dir.join(&app.artifact_rel_path))
            .expect("artifact should exist");
        let parsed = parse_package_artifact(&artifact).expect("artifact should parse");
        let main = parsed
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let lowered = format!("{:?}", main.statements);
        assert!(
            lowered
                .contains("resolved_callable: Some([\"std\", \"collections\", \"list\", \"len\"])"),
            "expected lowered bare-method identity in artifact: {artifact}"
        );
        assert!(
            lowered.contains("resolved_routine: Some(\"std.collections.list#impl-0-method-0\")"),
            "expected lowered bare-method routine identity in artifact: {artifact}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn execute_build_rejects_ambiguous_concrete_bare_methods() {
        let dir = temp_dir("ambiguous_bare_method_build");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src/shelf.arc"),
            concat!(
                "import types\n",
                "use types.Counter\n",
                "fn main() -> Int:\n",
                "    let counter = Counter :: value = 1 :: call\n",
                "    return counter :: :: tap\n",
            ),
        );
        write_file(
            &dir.join("src/types.arc"),
            "export record Counter:\n    value: Int\n",
        );
        write_file(
            &dir.join("src/left.arc"),
            concat!(
                "import types\n",
                "use types.Counter\n",
                "impl Counter:\n",
                "    fn tap(read self: Counter) -> Int:\n",
                "        return self.value + 1\n",
            ),
        );
        write_file(
            &dir.join("src/right.arc"),
            concat!(
                "import types\n",
                "use types.Counter\n",
                "impl Counter:\n",
                "    fn tap(read self: Counter) -> Int:\n",
                "        return self.value + 2\n",
            ),
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let fingerprints = fake_member_fingerprints(&graph);
        let statuses = plan_build(&graph, &order, &fingerprints, None).expect("plan");
        let err = execute_build(&graph, &statuses)
            .expect_err("ambiguous concrete bare method should fail build");
        assert!(
            err.contains("bare-method qualifier `tap` on `app.types.Counter` is ambiguous"),
            "{err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_leaf_rebuilds_only_leaf() {
        let dir = temp_dir("leaf");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"core\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("core", "../core")],
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = fake_member_fingerprints(&graph);
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        let second_fingerprints =
            mutate_member_fingerprint(&first_fingerprints, "app", Some("leaf-edit"), None);
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member, "core");
        assert_eq!(second_statuses[0].disposition, BuildDisposition::CacheHit);
        assert_eq!(second_statuses[1].member, "app");
        assert_eq!(second_statuses[1].disposition, BuildDisposition::Built);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_private_dependency_code_does_not_rebuild_dependents() {
        let dir = temp_dir("shared");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"tool\", \"core\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("core", "../core")],
        );
        write_grimoire(
            &dir.join("tool"),
            GrimoireKind::App,
            "tool",
            &[("core", "../core")],
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("core/src/book.arc"),
            "export fn shared_value() -> Int:\n    return helper :: :: call\n",
        );
        write_file(
            &dir.join("core/src/helper.arc"),
            "fn helper() -> Int:\n    return 0\n",
        );
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = fake_member_fingerprints(&graph);
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        let second_fingerprints =
            mutate_member_fingerprint(&first_fingerprints, "core", Some("private-edit"), None);
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member, "core");
        assert_eq!(second_statuses[0].disposition, BuildDisposition::Built);
        assert_eq!(second_statuses[1].member, "app");
        assert_eq!(second_statuses[1].disposition, BuildDisposition::CacheHit);
        assert_eq!(second_statuses[2].member, "tool");
        assert_eq!(second_statuses[2].disposition, BuildDisposition::CacheHit);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_exported_surface_rebuilds_dependents() {
        let dir = temp_dir("shared_api");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"tool\", \"core\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("core", "../core")],
        );
        write_grimoire(
            &dir.join("tool"),
            GrimoireKind::App,
            "tool",
            &[("core", "../core")],
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("core/src/book.arc"),
            "export fn shared_value() -> Int:\n    return 0\n",
        );
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = fake_member_fingerprints(&graph);
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        let second_fingerprints = mutate_member_fingerprint(
            &first_fingerprints,
            "core",
            Some("api-shape"),
            Some("api-shape"),
        );
        let second_statuses =
            plan_build(&graph, &order, &second_fingerprints, Some(&existing)).expect("plan");
        assert_eq!(second_statuses[0].member, "core");
        assert_eq!(second_statuses[0].disposition, BuildDisposition::Built);
        assert!(
            second_statuses[1..]
                .iter()
                .all(|status| status.disposition == BuildDisposition::Built)
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
