use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use arcana_syntax::{DirectiveKind, parse_module};
use pathdiff::diff_paths;
use sha2::{Digest, Sha256};

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

    let seed_paths = if root_manifest.workspace_members.is_empty() {
        vec![root_dir.clone()]
    } else {
        root_manifest
            .workspace_members
            .iter()
            .map(|rel| canonicalize_dir(&root_dir.join(rel)))
            .collect::<PackageResult<Vec<_>>>()?
    };

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

pub fn compute_member_fingerprints(
    graph: &WorkspaceGraph,
) -> PackageResult<HashMap<String, MemberFingerprints>> {
    let mut out = HashMap::new();
    for member in &graph.members {
        out.insert(member.name.clone(), compute_member_fingerprint(member)?);
    }
    Ok(out)
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
        let artifact_abs_path = graph.root_dir.join(&artifact_rel_path);
        let fingerprint_changed = existing
            .map(|entry| entry.fingerprint != fingerprint)
            .unwrap_or(true);
        let api_fingerprint_changed = existing
            .map(|entry| entry.api_fingerprint != api_fingerprint)
            .unwrap_or(true);
        let upstream_api_changed = member
            .deps
            .iter()
            .any(|dep| api_changed.get(dep).copied().unwrap_or(false));
        let artifact_missing = !artifact_abs_path.is_file();
        let built = fingerprint_changed
            || api_fingerprint_changed
            || upstream_api_changed
            || artifact_missing;
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
            format: "foundation-placeholder-v1".to_string(),
        });
    }

    Ok(statuses)
}

pub fn execute_build(graph: &WorkspaceGraph, statuses: &[BuildStatus]) -> PackageResult<PathBuf> {
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
            "format = \"{}\"\nmember = \"{}\"\nkind = \"{}\"\nfingerprint = \"{}\"\napi_fingerprint = \"{}\"\n",
            status.format,
            status.member,
            status.kind.as_str(),
            status.fingerprint,
            status.api_fingerprint
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

fn compute_member_fingerprint(member: &WorkspaceMember) -> PackageResult<MemberFingerprints> {
    let files = collect_arc_files(&member.abs_dir.join("src"))?;
    let manifest_path = member.abs_dir.join("book.toml");
    let source = compute_source_fingerprint(member, &manifest_path, &files)?;
    let api = compute_api_fingerprint(member, &files)?;
    Ok(MemberFingerprints { source, api })
}

fn compute_source_fingerprint(
    member: &WorkspaceMember,
    manifest_path: &Path,
    files: &[PathBuf],
) -> PackageResult<String> {
    let mut hasher = Sha256::new();
    for file in files
        .iter()
        .cloned()
        .chain(std::iter::once(manifest_path.to_path_buf()))
    {
        let rel = file
            .strip_prefix(&member.abs_dir)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        hasher.update(rel.as_bytes());
        let bytes =
            fs::read(&file).map_err(|e| format!("failed to read `{}`: {e}", file.display()))?;
        hasher.update(bytes);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn compute_api_fingerprint(
    member: &WorkspaceMember,
    files: &[PathBuf],
) -> PackageResult<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_api_v1\n");
    hasher.update(format!("name={}\n", member.name).as_bytes());
    hasher.update(format!("kind={}\n", member.kind.as_str()).as_bytes());
    for dep in &member.deps {
        hasher.update(format!("dep={dep}\n").as_bytes());
    }

    for file in files {
        let module_id = module_id_from_path(member, file)?;
        let source =
            fs::read_to_string(file).map_err(|e| format!("failed to read `{}`: {e}", file.display()))?;
        let parsed = parse_module(&source)
            .map_err(|err| format!("{}: {err}", file.display()))?;

        hasher.update(format!("module={module_id}\n").as_bytes());

        let mut reexports = parsed
            .directives
            .iter()
            .filter(|directive| directive.kind == DirectiveKind::Reexport)
            .map(|directive| directive.path.join("."))
            .collect::<Vec<_>>();
        reexports.sort();
        for reexport in reexports {
            hasher.update(format!("reexport={reexport}\n").as_bytes());
        }

        let mut exported_symbols = parsed
            .symbols
            .iter()
            .filter(|symbol| symbol.exported)
            .map(|symbol| (symbol.kind.as_str().to_string(), symbol.name.clone()))
            .collect::<Vec<_>>();
        exported_symbols.sort();
        for (kind, name) in exported_symbols {
            hasher.update(format!("export={kind}:{name}\n").as_bytes());
        }
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
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

fn module_id_from_path(member: &WorkspaceMember, path: &Path) -> PackageResult<String> {
    let src_dir = member.abs_dir.join("src");
    let relative = path
        .strip_prefix(&src_dir)
        .map_err(|e| format!("failed to resolve module path `{}`: {e}", path.display()))?;
    let mut segments = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return Err(format!("empty module path for `{}`", path.display()));
    }
    let file_name = segments
        .pop()
        .ok_or_else(|| format!("empty module path for `{}`", path.display()))?;
    let stem = file_name
        .strip_suffix(".arc")
        .ok_or_else(|| format!("non-Arcana file `{}`", path.display()))?;
    if stem == "book" || stem == "shelf" {
        if file_name != member.kind.root_file_name() && !segments.is_empty() {
            segments.push(stem.to_string());
        }
    } else {
        segments.push(stem.to_string());
    }

    let mut full = vec![member.name.clone()];
    full.extend(segments);
    Ok(full.join("."))
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
    Ok(rel.to_string_lossy().replace('\\', "/"))
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
        let fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
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
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
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
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        write_file(
            &dir.join("app/src/shelf.arc"),
            "fn main() -> Int:\n    return 1\n",
        );
        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
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
            "export fn shared_value() -> Int:\n    return helper()\n",
        );
        write_file(
            &dir.join("core/src/helper.arc"),
            "fn helper() -> Int:\n    return 0\n",
        );
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        write_file(
            &dir.join("core/src/helper.arc"),
            "fn helper() -> Int:\n    return 1\n",
        );
        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
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
        let first_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
        let first_statuses = plan_build(&graph, &order, &first_fingerprints, None).expect("plan");
        execute_build(&graph, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        write_file(
            &dir.join("core/src/book.arc"),
            "export fn shared_value() -> Int:\n    return 0\n\nexport fn shared_value_v2() -> Int:\n    return 1\n",
        );
        let second_fingerprints = compute_member_fingerprints(&graph).expect("fingerprints");
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
