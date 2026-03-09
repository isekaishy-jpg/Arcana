use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

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
) -> PackageResult<HashMap<String, String>> {
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
    fingerprints: &HashMap<String, String>,
    existing_lock: Option<&Lockfile>,
) -> PackageResult<Vec<BuildStatus>> {
    let mut statuses = Vec::new();
    let mut api_changed = HashMap::<String, bool>::new();

    for name in order {
        let member = graph
            .member(name)
            .ok_or_else(|| format!("workspace planner missing member `{name}`"))?;
        let fingerprint = fingerprints
            .get(name)
            .cloned()
            .ok_or_else(|| format!("missing fingerprint for member `{name}`"))?;
        let api_fingerprint = fingerprint.clone();
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
        let built =
            fingerprint_changed || api_fingerprint_changed || upstream_api_changed || artifact_missing;
        api_changed.insert(name.clone(), built);
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

pub fn execute_build(
    graph: &WorkspaceGraph,
    statuses: &[BuildStatus],
) -> PackageResult<PathBuf> {
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

    let summary_path = graph.root_dir.join(CACHE_DIR).join(LOGS_DIR).join("build-last.txt");
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
