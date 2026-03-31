use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};

use arcana_aot::{
    AOT_INTERNAL_FORMAT, AOT_WINDOWS_DLL_FORMAT, AOT_WINDOWS_EXE_FORMAT,
    ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV,
};
use arcana_cabi::ArcanaCabiProductRole;
use arcana_hir::{
    HirForewordAdapterProduct, HirWorkspacePackage, HirWorkspaceSummary, build_package_layout,
    build_package_summary, build_workspace_summary, derive_source_module_path, lower_module_text,
};
use pathdiff::diff_paths;

mod build;
mod build_identity;
mod distribution;
mod fingerprint;
mod publish;
mod versioning;

pub type PackageResult<T> = Result<T, String>;

pub use build::{
    BuildDisposition, BuildExecutionContext, BuildProgress, BuildStatus, PreparedBuild,
    execute_build, execute_build_with_context, execute_build_with_context_and_progress, plan_build,
    plan_build_for_target, plan_build_for_target_with_context,
    plan_package_build_for_target_with_context, prepare_build, prepare_build_from_workspace,
    render_build_summary, render_lockfile, write_lockfile,
};
pub use distribution::{
    DISTRIBUTION_BUNDLE_FORMAT, DistributionBundle, default_distribution_dir,
    default_distribution_dir_for_build, stage_distribution_bundle,
    stage_distribution_bundle_for_build,
};
pub use fingerprint::{
    MemberFingerprints, WorkspaceFingerprints, compute_workspace_fingerprints,
    compute_workspace_snapshot_id,
};
pub use publish::publish_workspace_member;
pub use versioning::{GitSelector, PackageId, SemverVersion, SourceId, VersionReq};

pub(crate) const LOCKFILE_VERSION: i64 = 4;
pub(crate) const LEGACY_LOCKFILE_VERSION: i64 = 3;
pub(crate) const OLDER_LOCKFILE_VERSION: i64 = 2;
pub(crate) const OLDEST_LOCKFILE_VERSION: i64 = 1;
pub(crate) const CACHE_DIR: &str = ".arcana";
pub(crate) const ARTIFACT_DIR: &str = "artifacts";
pub(crate) const LOGS_DIR: &str = "logs";
pub(crate) const DEFAULT_REGISTRY_NAME: &str = "local";
pub(crate) const LOCAL_REGISTRY_METADATA_FILE: &str = "package.toml";
pub(crate) const LOCAL_REGISTRY_SNAPSHOT_DIR: &str = "snapshot";

pub(crate) fn collect_validated_support_file_paths<'a, I>(paths: I) -> PackageResult<Vec<String>>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut validated = Vec::new();
    let mut seen_paths = BTreeSet::new();
    for relative_path in paths {
        validate_support_file_relative_path(relative_path)?;
        if !seen_paths.insert(relative_path) {
            return Err(format!("duplicate support file path `{relative_path}`"));
        }
        validated.push(relative_path.to_string());
    }
    Ok(validated)
}

pub(crate) fn validate_support_file_relative_path(relative_path: &str) -> PackageResult<()> {
    if relative_path.is_empty() {
        return Err("support file path must not be empty".to_string());
    }
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_)
                    | Component::RootDir
                    | Component::CurDir
                    | Component::ParentDir
            )
        })
    {
        return Err(format!("invalid support file path `{relative_path}`"));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BuildTarget {
    InternalAot,
    WindowsExe,
    WindowsDll,
    Other(String),
}

impl BuildTarget {
    pub fn internal_aot() -> Self {
        Self::InternalAot
    }

    pub fn windows_exe() -> Self {
        Self::WindowsExe
    }

    pub fn windows_dll() -> Self {
        Self::WindowsDll
    }

    pub fn key(&self) -> &str {
        match self {
            Self::InternalAot => "internal-aot",
            Self::WindowsExe => "windows-exe",
            Self::WindowsDll => "windows-dll",
            Self::Other(other) => other,
        }
    }

    pub(crate) fn from_storage_key(text: &str) -> Self {
        match text {
            "internal-aot" => Self::InternalAot,
            "windows-exe" => Self::WindowsExe,
            "windows-dll" => Self::WindowsDll,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn format(&self) -> Option<&'static str> {
        match self {
            Self::InternalAot => Some(AOT_INTERNAL_FORMAT),
            Self::WindowsExe => Some(AOT_WINDOWS_EXE_FORMAT),
            Self::WindowsDll => Some(AOT_WINDOWS_DLL_FORMAT),
            Self::Other(_) => None,
        }
    }

    pub fn artifact_file_name(&self, kind: &GrimoireKind) -> PackageResult<&'static str> {
        match (self, kind) {
            (Self::InternalAot, GrimoireKind::App) => Ok("app.artifact.toml"),
            (Self::InternalAot, GrimoireKind::Lib) => Ok("lib.artifact.toml"),
            (Self::WindowsExe, GrimoireKind::App) => Ok("app.exe"),
            (Self::WindowsExe, GrimoireKind::Lib) => {
                Err("windows-exe target requires an app grimoire".to_string())
            }
            (Self::WindowsDll, GrimoireKind::Lib) => Ok("lib.dll"),
            (Self::WindowsDll, GrimoireKind::App) => {
                Err("windows-dll target requires a lib grimoire".to_string())
            }
            (Self::Other(_), _) => Err(format!("unsupported build target `{self}`")),
        }
    }
}

impl std::fmt::Display for BuildTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.key())
    }
}

pub fn parse_build_target(text: &str) -> PackageResult<BuildTarget> {
    match text {
        "internal-aot" => Ok(BuildTarget::InternalAot),
        "windows-exe" => Ok(BuildTarget::WindowsExe),
        "windows-dll" => Ok(BuildTarget::WindowsDll),
        other => Err(format!("unsupported build target `{other}`")),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildOutputKey {
    pub target: BuildTarget,
    pub product: Option<String>,
}

impl BuildOutputKey {
    pub fn new(target: BuildTarget, product: Option<String>) -> Self {
        Self { target, product }
    }

    pub fn target(target: BuildTarget) -> Self {
        Self {
            target,
            product: None,
        }
    }

    pub fn target_ref(&self) -> &BuildTarget {
        &self.target
    }

    pub fn product(&self) -> Option<&str> {
        self.product.as_deref()
    }

    pub fn storage_key(&self) -> String {
        match &self.product {
            Some(product) => format!("{}@{}", self.target.key(), product),
            None => self.target.key().to_string(),
        }
    }

    pub fn from_storage_key(text: &str) -> Self {
        match text.split_once('@') {
            Some((target, product)) => Self {
                target: BuildTarget::from_storage_key(target),
                product: Some(product.to_string()),
            },
            None => Self::target(BuildTarget::from_storage_key(text)),
        }
    }
}

fn infer_build_target_from_format(format: &str) -> Option<BuildTarget> {
    match format {
        AOT_INTERNAL_FORMAT => Some(BuildTarget::InternalAot),
        other if other.starts_with("arcana-aot-") => Some(BuildTarget::InternalAot),
        AOT_WINDOWS_EXE_FORMAT => Some(BuildTarget::WindowsExe),
        AOT_WINDOWS_DLL_FORMAT => Some(BuildTarget::WindowsDll),
        _ => None,
    }
}

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
    Registry,
    Git,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencySourceSpec {
    Path {
        location: String,
    },
    Registry {
        registry_name: Option<String>,
        version: VersionReq,
        checksum: Option<String>,
    },
    Git {
        url: String,
        selector: Option<GitSelector>,
    },
}

impl DependencySourceSpec {
    pub fn kind(&self) -> DependencySource {
        match self {
            Self::Path { .. } => DependencySource::Path,
            Self::Registry { .. } => DependencySource::Registry,
            Self::Git { .. } => DependencySource::Git,
        }
    }

    pub fn location_label(&self) -> String {
        match self {
            Self::Path { location } => location.clone(),
            Self::Registry {
                registry_name,
                version,
                ..
            } => format!(
                "{}:{}",
                registry_name.as_deref().unwrap_or(DEFAULT_REGISTRY_NAME),
                version
            ),
            Self::Git { url, selector } => match selector {
                Some(selector) => format!("{url}#{}", selector.render()),
                None => url.clone(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDependencyDelivery {
    Baked,
    Dll,
}

impl NativeDependencyDelivery {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baked => "baked",
            Self::Dll => "dll",
        }
    }

    fn parse(text: &str) -> PackageResult<Self> {
        match text {
            "baked" => Ok(Self::Baked),
            "dll" => Ok(Self::Dll),
            other => Err(format!(
                "`native_delivery` must be \"baked\" or \"dll\" (found `{other}`)"
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeProductProducer {
    ArcanaSource,
    RustCdylib,
}

impl NativeProductProducer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ArcanaSource => "arcana-source",
            Self::RustCdylib => "rust-cdylib",
        }
    }

    fn parse(text: &str) -> PackageResult<Self> {
        match text {
            "arcana-source" => Ok(Self::ArcanaSource),
            "rust-cdylib" => Ok(Self::RustCdylib),
            other => Err(format!(
                "`producer` must be \"arcana-source\" or \"rust-cdylib\" (found `{other}`)"
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeProductSpec {
    pub name: String,
    pub kind: String,
    pub role: ArcanaCabiProductRole,
    pub producer: NativeProductProducer,
    pub file: String,
    pub contract: String,
    pub rust_cdylib_crate: Option<String>,
    pub sidecars: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForewordAdapterProductSpec {
    pub name: String,
    pub path: String,
    pub runner: Option<String>,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencySpec {
    pub package: String,
    pub source: DependencySourceSpec,
    pub native_delivery: NativeDependencyDelivery,
    pub native_child: Option<String>,
    pub native_plugins: Vec<String>,
    pub executable_forewords: bool,
}

impl DependencySpec {
    pub fn selected_native_child(&self) -> Option<&str> {
        self.native_child.as_deref().or(match self.native_delivery {
            NativeDependencyDelivery::Dll => Some("default"),
            NativeDependencyDelivery::Baked => None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub kind: GrimoireKind,
    pub version: Option<SemverVersion>,
    pub workspace_members: Vec<String>,
    pub deps: BTreeMap<String, DependencySpec>,
    pub native_products: BTreeMap<String, NativeProductSpec>,
    pub foreword_products: BTreeMap<String, ForewordAdapterProductSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceMember {
    pub package_id: String,
    pub source_id: String,
    pub name: String,
    pub kind: GrimoireKind,
    pub version: Option<SemverVersion>,
    pub source_kind: DependencySource,
    pub rel_dir: String,
    pub abs_dir: PathBuf,
    pub deps: Vec<String>,
    pub direct_dep_packages: BTreeMap<String, String>,
    pub direct_dep_ids: BTreeMap<String, String>,
    pub direct_dep_specs: BTreeMap<String, DependencySpec>,
    pub registry_name: Option<String>,
    pub checksum: Option<String>,
    pub git_url: Option<String>,
    pub git_selector: Option<String>,
    pub native_products: BTreeMap<String, NativeProductSpec>,
    pub foreword_products: BTreeMap<String, ForewordAdapterProductSpec>,
}

impl WorkspaceMember {
    pub fn display_label(&self) -> String {
        match (&self.source_kind, &self.version) {
            (DependencySource::Registry, Some(version)) => format!("{}@{}", self.name, version),
            _ => self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceGraph {
    pub root_name: String,
    pub root_id: String,
    pub root_dir: PathBuf,
    pub members: Vec<WorkspaceMember>,
}

impl WorkspaceGraph {
    pub fn member(&self, key: &str) -> Option<&WorkspaceMember> {
        if let Some(member) = self.members.iter().find(|member| member.package_id == key) {
            return Some(member);
        }
        let mut local_matches = self.members.iter().filter(|member| {
            member.name == key
                && member.source_kind == DependencySource::Path
                && !member.package_id.starts_with("path:..")
        });
        let first = local_matches.next()?;
        if local_matches.next().is_some() {
            return None;
        }
        Some(first)
    }

    pub fn member_by_id(&self, package_id: &str) -> Option<&WorkspaceMember> {
        self.members
            .iter()
            .find(|member| member.package_id == package_id)
    }

    pub fn local_member(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members
            .iter()
            .find(|member| member.name == name && member.source_kind == DependencySource::Path)
    }

    pub fn workspace_members(&self) -> impl Iterator<Item = &WorkspaceMember> {
        self.members
            .iter()
            .filter(|member| member.source_kind == DependencySource::Path)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockTargetEntry {
    pub fingerprint: String,
    pub api_fingerprint: String,
    pub artifact: String,
    pub artifact_hash: String,
    pub format: String,
    pub toolchain: String,
    pub product: Option<String>,
    pub native_product_closure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockNativeProductEntry {
    pub kind: String,
    pub role: String,
    pub producer: String,
    pub file: String,
    pub contract: String,
    pub rust_cdylib_crate: Option<String>,
    pub sidecars: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockMember {
    pub name: String,
    pub deps: Vec<String>,
    pub dep_bindings: BTreeMap<String, String>,
    pub kind: GrimoireKind,
    pub source_kind: DependencySource,
    pub path: Option<String>,
    pub version: Option<SemverVersion>,
    pub registry_name: Option<String>,
    pub checksum: Option<String>,
    pub git_url: Option<String>,
    pub git_selector: Option<String>,
    pub native_products: BTreeMap<String, LockNativeProductEntry>,
    pub targets: BTreeMap<BuildOutputKey, LockTargetEntry>,
}

impl LockMember {
    pub fn target(&self, target: &BuildTarget) -> Option<&LockTargetEntry> {
        if let Some(entry) = self.targets.get(&BuildOutputKey::target(target.clone())) {
            return Some(entry);
        }
        let mut matching = self
            .targets
            .iter()
            .filter(|(build_key, _)| build_key.target_ref() == target)
            .map(|(_, entry)| entry);
        let first = matching.next()?;
        if matching.next().is_some() {
            return None;
        }
        Some(first)
    }

    pub fn build(&self, build_key: &BuildOutputKey) -> Option<&LockTargetEntry> {
        self.targets.get(build_key)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lockfile {
    pub version: i64,
    pub workspace: String,
    pub workspace_root: String,
    pub order: Vec<String>,
    pub workspace_members: Vec<String>,
    pub members: BTreeMap<String, LockMember>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingMember {
    package_id: String,
    source_id: String,
    name: String,
    kind: GrimoireKind,
    version: Option<SemverVersion>,
    source_kind: DependencySource,
    abs_dir: PathBuf,
    rel_dir: String,
    dep_bindings: Vec<(String, String, String, DependencySpec)>,
    registry_name: Option<String>,
    checksum: Option<String>,
    git_url: Option<String>,
    git_selector: Option<String>,
    native_products: BTreeMap<String, NativeProductSpec>,
    foreword_products: BTreeMap<String, ForewordAdapterProductSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LockMemberBase {
    name: String,
    deps: Vec<String>,
    dep_bindings: BTreeMap<String, String>,
    kind: GrimoireKind,
    source_kind: DependencySource,
    path: Option<String>,
    version: Option<SemverVersion>,
    registry_name: Option<String>,
    checksum: Option<String>,
    git_url: Option<String>,
    git_selector: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingLoad {
    Path {
        abs_dir: PathBuf,
    },
    Registry {
        registry_name: String,
        package_name: String,
        version: SemverVersion,
    },
    Git {
        url: String,
        selector: Option<GitSelector>,
        package_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedDependency {
    package_id: String,
    display_name: String,
    pending_load: Option<PendingLoad>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceBackendKind {
    Path,
    Registry,
    Git,
}

struct ResolveDependencyRequest<'a> {
    root_dir: &'a Path,
    depender_package_id: &'a str,
    dep_name: &'a str,
    dep: &'a DependencySpec,
    base_dir: &'a Path,
    existing_lock: Option<&'a Lockfile>,
}

struct LoadPendingMemberRequest<'a> {
    root_dir: &'a Path,
    pending: PendingLoad,
    existing_lock: Option<&'a Lockfile>,
    pending_by_id: &'a BTreeMap<String, PendingMember>,
    path_name_to_id: &'a mut HashMap<String, String>,
    queue: &'a mut VecDeque<PendingLoad>,
}

impl SourceBackendKind {
    fn for_spec(spec: &DependencySourceSpec) -> Self {
        match spec {
            DependencySourceSpec::Path { .. } => Self::Path,
            DependencySourceSpec::Registry { .. } => Self::Registry,
            DependencySourceSpec::Git { .. } => Self::Git,
        }
    }

    fn for_pending(pending: &PendingLoad) -> Self {
        match pending {
            PendingLoad::Path { .. } => Self::Path,
            PendingLoad::Registry { .. } => Self::Registry,
            PendingLoad::Git { .. } => Self::Git,
        }
    }

    fn resolve_dependency(
        self,
        request: ResolveDependencyRequest<'_>,
    ) -> PackageResult<ResolvedDependency> {
        match self {
            Self::Path => {
                let DependencySourceSpec::Path { location } = &request.dep.source else {
                    return Err(
                        "internal error: path backend received non-path dependency source"
                            .to_string(),
                    );
                };
                resolve_path_dependency(request.root_dir, location, request.base_dir)
            }
            Self::Registry => {
                let DependencySourceSpec::Registry {
                    registry_name,
                    version,
                    checksum,
                } = &request.dep.source
                else {
                    return Err(
                        "internal error: registry backend received non-registry dependency source"
                            .to_string(),
                    );
                };
                resolve_registry_dependency(
                    request.depender_package_id,
                    request.dep_name,
                    &request.dep.package,
                    registry_name.as_deref(),
                    version,
                    checksum.as_deref(),
                    request.existing_lock,
                )
            }
            Self::Git => {
                let DependencySourceSpec::Git { url, selector } = &request.dep.source else {
                    return Err(
                        "internal error: git backend received non-git dependency source"
                            .to_string(),
                    );
                };
                resolve_git_dependency(
                    request.dep_name,
                    &request.dep.package,
                    url,
                    selector.as_ref(),
                )
            }
        }
    }

    fn pending_package_id(self, root_dir: &Path, pending: &PendingLoad) -> PackageResult<String> {
        match self {
            Self::Path => {
                let PendingLoad::Path { abs_dir } = pending else {
                    return Err(
                        "internal error: path backend received non-path pending load".to_string(),
                    );
                };
                Ok(PackageId::Path {
                    rel_path: relative_from_root(abs_dir, root_dir)?,
                }
                .render())
            }
            Self::Registry => {
                let PendingLoad::Registry {
                    registry_name,
                    package_name,
                    version,
                } = pending
                else {
                    return Err(
                        "internal error: registry backend received non-registry pending load"
                            .to_string(),
                    );
                };
                Ok(PackageId::Registry {
                    registry_name: registry_name.clone(),
                    package_name: package_name.clone(),
                    version: version.clone(),
                }
                .render())
            }
            Self::Git => {
                let PendingLoad::Git {
                    url,
                    selector,
                    package_name,
                } = pending
                else {
                    return Err(
                        "internal error: git backend received non-git pending load".to_string()
                    );
                };
                Ok(PackageId::Git {
                    url: url.clone(),
                    selector: selector
                        .as_ref()
                        .map(|selector| selector.render())
                        .unwrap_or_else(|| "head".to_string()),
                    package_name: package_name.clone(),
                }
                .render())
            }
        }
    }

    fn load_pending_member(
        self,
        request: LoadPendingMemberRequest<'_>,
    ) -> PackageResult<PendingMember> {
        match self {
            Self::Path => {
                let PendingLoad::Path { abs_dir } = request.pending else {
                    return Err(
                        "internal error: path backend received non-path pending load".to_string(),
                    );
                };
                load_path_pending_member(
                    request.root_dir,
                    abs_dir,
                    request.existing_lock,
                    request.pending_by_id,
                    request.path_name_to_id,
                    request.queue,
                )
            }
            Self::Registry => {
                let PendingLoad::Registry {
                    registry_name,
                    package_name,
                    version,
                } = request.pending
                else {
                    return Err(
                        "internal error: registry backend received non-registry pending load"
                            .to_string(),
                    );
                };
                load_registry_pending_member(
                    request.root_dir,
                    registry_name,
                    package_name,
                    version,
                    request.existing_lock,
                    request.queue,
                )
            }
            Self::Git => {
                let PendingLoad::Git {
                    url,
                    selector,
                    package_name,
                } = request.pending
                else {
                    return Err(
                        "internal error: git backend received non-git pending load".to_string()
                    );
                };
                load_git_pending_member(url, selector, package_name)
            }
        }
    }
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
    let version = table
        .get("version")
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("`version` in `{}` must be a string", path.display()))
        })
        .transpose()?
        .map(SemverVersion::parse)
        .transpose()?;

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
            deps.insert(name.clone(), spec);
        }
    }
    let native_products = parse_native_products(table, path)?;
    let foreword_products = parse_foreword_products(table, path)?;

    Ok(Manifest {
        name,
        kind,
        version,
        workspace_members,
        deps,
        native_products,
        foreword_products,
    })
}

pub fn load_workspace_graph(root_dir: &Path) -> PackageResult<WorkspaceGraph> {
    let root_dir = canonicalize_dir(root_dir)?;
    let existing_lock = read_lockfile(&root_dir.join("Arcana.lock"))?;
    let root_manifest_path = root_dir.join("book.toml");
    let root_manifest = parse_manifest(&root_manifest_path)?;
    let root_name = root_manifest.name.clone();
    let root_id = PackageId::Path {
        rel_path: ".".to_string(),
    }
    .render();

    let mut seed_paths = root_manifest
        .workspace_members
        .iter()
        .map(|rel| canonicalize_dir(&root_dir.join(rel)))
        .collect::<PackageResult<Vec<_>>>()?;
    if seed_paths.is_empty() || has_root_module(&root_dir, &root_manifest.kind) {
        seed_paths.push(root_dir.clone());
    }

    let mut queue = seed_paths
        .into_iter()
        .map(|abs_dir| PendingLoad::Path { abs_dir })
        .collect::<VecDeque<_>>();
    let mut pending_by_id = BTreeMap::<String, PendingMember>::new();
    let mut path_name_to_id = HashMap::<String, String>::new();
    let mut visited = BTreeSet::<String>::new();

    while let Some(item) = queue.pop_front() {
        let package_id = pending_load_package_id(&root_dir, &item)?;
        if !visited.insert(package_id) {
            continue;
        }
        let pending = load_pending_member(
            &root_dir,
            item,
            existing_lock.as_ref(),
            &pending_by_id,
            &mut path_name_to_id,
            &mut queue,
        )?;
        pending_by_id.insert(pending.package_id.clone(), pending);
    }

    let mut members = pending_by_id
        .values()
        .map(|member| {
            let mut deps = BTreeSet::new();
            let mut direct_dep_packages = BTreeMap::new();
            let mut direct_dep_ids = BTreeMap::new();
            let mut direct_dep_specs = BTreeMap::new();
            for (dep_name, dep_package_id, dep_display_name, spec) in &member.dep_bindings {
                let dep = pending_by_id.get(dep_package_id).ok_or_else(|| {
                    format!(
                        "dependency `{dep_name}` for `{}` resolved package `{dep_package_id}`, but that package is missing from the workspace graph",
                        member.name
                    )
                })?;
                deps.insert(dep.package_id.clone());
                direct_dep_packages.insert(dep_name.clone(), dep_display_name.clone());
                direct_dep_ids.insert(dep_name.clone(), dep.package_id.clone());
                direct_dep_specs.insert(dep_name.clone(), spec.clone());
            }
            Ok(WorkspaceMember {
                package_id: member.package_id.clone(),
                source_id: member.source_id.clone(),
                name: member.name.clone(),
                kind: member.kind.clone(),
                version: member.version.clone(),
                source_kind: member.source_kind.clone(),
                rel_dir: member.rel_dir.clone(),
                abs_dir: member.abs_dir.clone(),
                deps: deps.into_iter().collect(),
                direct_dep_packages,
                direct_dep_ids,
                direct_dep_specs,
                registry_name: member.registry_name.clone(),
                checksum: member.checksum.clone(),
                git_url: member.git_url.clone(),
                git_selector: member.git_selector.clone(),
                native_products: member.native_products.clone(),
                foreword_products: member.foreword_products.clone(),
            })
        })
        .collect::<PackageResult<Vec<_>>>()?;

    members.sort_by(|a, b| PackageId::compare_rendered(&a.package_id, &b.package_id));
    Ok(WorkspaceGraph {
        root_name,
        root_id,
        root_dir,
        members,
    })
}

fn resolve_workspace_dependency(
    root_dir: &Path,
    depender_package_id: &str,
    dep_name: &str,
    dep: &DependencySpec,
    base_dir: &Path,
    existing_lock: Option<&Lockfile>,
) -> PackageResult<(String, String, Option<PendingLoad>)> {
    let resolved =
        SourceBackendKind::for_spec(&dep.source).resolve_dependency(ResolveDependencyRequest {
            root_dir,
            depender_package_id,
            dep_name,
            dep,
            base_dir,
            existing_lock,
        })?;
    Ok((
        resolved.package_id,
        resolved.display_name,
        resolved.pending_load,
    ))
}

fn resolve_path_dependency(
    root_dir: &Path,
    location: &str,
    base_dir: &Path,
) -> PackageResult<ResolvedDependency> {
    let dep_dir = canonicalize_dir(&base_dir.join(location))?;
    let dep_manifest = parse_manifest(&dep_dir.join("book.toml"))?;
    let dep_package_id = PackageId::Path {
        rel_path: relative_from_root(&dep_dir, root_dir)?,
    }
    .render();
    Ok(ResolvedDependency {
        package_id: dep_package_id,
        display_name: dep_manifest.name,
        pending_load: Some(PendingLoad::Path { abs_dir: dep_dir }),
    })
}

fn resolve_registry_dependency(
    depender_package_id: &str,
    dep_name: &str,
    package_name: &str,
    registry_name: Option<&str>,
    version: &VersionReq,
    checksum: Option<&str>,
    existing_lock: Option<&Lockfile>,
) -> PackageResult<ResolvedDependency> {
    let registry_name = registry_name.unwrap_or(DEFAULT_REGISTRY_NAME).to_string();
    if registry_name != DEFAULT_REGISTRY_NAME {
        return Err(format!(
            "dependency `{dep_name}` uses registry `{registry_name}`, but only `registry = \"{DEFAULT_REGISTRY_NAME}\"` is enabled in this phase"
        ));
    }
    let locked_version = existing_lock
        .and_then(|lock| lock.members.get(depender_package_id))
        .and_then(|member| member.dep_bindings.get(dep_name))
        .and_then(|package_id| match PackageId::parse(package_id) {
            Ok(PackageId::Registry {
                registry_name,
                package_name: locked_package_name,
                version: locked_version,
            }) if registry_name == DEFAULT_REGISTRY_NAME
                && locked_package_name == package_name
                && version.matches(&locked_version)
                && local_registry_manifest_path(
                    DEFAULT_REGISTRY_NAME,
                    &locked_package_name,
                    &locked_version,
                )
                .map(|path| path.is_file())
                .unwrap_or(false) =>
            {
                Some(locked_version)
            }
            _ => None,
        });
    let resolved_version = match locked_version {
        Some(version) => version,
        None => resolve_local_registry_version(&registry_name, package_name, version)?,
    };
    validate_local_registry_dependency_checksum(
        dep_name,
        &registry_name,
        package_name,
        &resolved_version,
        checksum,
    )?;
    Ok(ResolvedDependency {
        package_id: PackageId::Registry {
            registry_name: registry_name.clone(),
            package_name: package_name.to_string(),
            version: resolved_version.clone(),
        }
        .render(),
        display_name: package_name.to_string(),
        pending_load: Some(PendingLoad::Registry {
            registry_name,
            package_name: package_name.to_string(),
            version: resolved_version,
        }),
    })
}

fn resolve_git_dependency(
    dep_name: &str,
    package_name: &str,
    url: &str,
    selector: Option<&GitSelector>,
) -> PackageResult<ResolvedDependency> {
    let _recognized_source = PendingLoad::Git {
        url: url.to_string(),
        selector: selector.cloned(),
        package_name: package_name.to_string(),
    };
    let selector_text = selector.as_ref().map(|selector| selector.render());
    let detail = selector_text
        .as_deref()
        .map(|value| format!("{url}#{value}"))
        .unwrap_or_else(|| url.to_string());
    let _ = package_name;
    Err(format!(
        "dependency `{dep_name}` uses `git` ({detail}), which is recognized but not enabled yet"
    ))
}

fn pending_load_package_id(root_dir: &Path, pending: &PendingLoad) -> PackageResult<String> {
    SourceBackendKind::for_pending(pending).pending_package_id(root_dir, pending)
}

fn load_pending_member(
    root_dir: &Path,
    pending: PendingLoad,
    existing_lock: Option<&Lockfile>,
    pending_by_id: &BTreeMap<String, PendingMember>,
    path_name_to_id: &mut HashMap<String, String>,
    queue: &mut VecDeque<PendingLoad>,
) -> PackageResult<PendingMember> {
    let backend = SourceBackendKind::for_pending(&pending);
    backend.load_pending_member(LoadPendingMemberRequest {
        root_dir,
        pending,
        existing_lock,
        pending_by_id,
        path_name_to_id,
        queue,
    })
}

fn load_path_pending_member(
    root_dir: &Path,
    abs_dir: PathBuf,
    existing_lock: Option<&Lockfile>,
    pending_by_id: &BTreeMap<String, PendingMember>,
    path_name_to_id: &mut HashMap<String, String>,
    queue: &mut VecDeque<PendingLoad>,
) -> PackageResult<PendingMember> {
    let rel_dir = relative_from_root(&abs_dir, root_dir)?;
    let package_id = PackageId::Path {
        rel_path: rel_dir.clone(),
    }
    .render();
    let manifest_path = abs_dir.join("book.toml");
    let manifest = parse_manifest(&manifest_path)?;
    validate_grimoire_layout(&abs_dir, &manifest.kind)?;
    if let Some(existing_id) = path_name_to_id.insert(manifest.name.clone(), package_id.clone())
        && existing_id != package_id
    {
        let existing = pending_by_id
            .get(&existing_id)
            .map(|member| member.abs_dir.display().to_string())
            .unwrap_or(existing_id);
        return Err(format!(
            "duplicate local grimoire name `{}` at `{existing}` and `{}`",
            manifest.name,
            abs_dir.display()
        ));
    }

    let dep_bindings = resolve_pending_member_dependencies(
        root_dir,
        &package_id,
        &abs_dir,
        &manifest,
        existing_lock,
        queue,
    )?;

    Ok(PendingMember {
        package_id,
        source_id: SourceId::Path(rel_dir.clone()).render(),
        name: manifest.name,
        kind: manifest.kind,
        version: manifest.version,
        source_kind: DependencySource::Path,
        abs_dir,
        rel_dir,
        dep_bindings,
        registry_name: None,
        checksum: None,
        git_url: None,
        git_selector: None,
        native_products: manifest.native_products,
        foreword_products: manifest.foreword_products,
    })
}

fn load_registry_pending_member(
    root_dir: &Path,
    registry_name: String,
    package_name: String,
    version: SemverVersion,
    existing_lock: Option<&Lockfile>,
    queue: &mut VecDeque<PendingLoad>,
) -> PackageResult<PendingMember> {
    let package_id = PackageId::Registry {
        registry_name: registry_name.clone(),
        package_name: package_name.clone(),
        version: version.clone(),
    }
    .render();
    let manifest = read_local_registry_manifest(&registry_name, &package_name, &version)?;
    let published_checksum =
        read_local_registry_published_checksum(&registry_name, &package_name, &version)?;
    let snapshot_dir = local_registry_snapshot_dir(&registry_name, &package_name, &version)?;
    validate_grimoire_layout(&snapshot_dir, &manifest.kind)?;
    let arcana_home = arcana_home_dir()?;
    let rel_dir = relative_from_root(&snapshot_dir, &arcana_home)?;
    let dep_bindings = resolve_pending_member_dependencies(
        root_dir,
        &package_id,
        &snapshot_dir,
        &manifest,
        existing_lock,
        queue,
    )?;

    Ok(PendingMember {
        package_id,
        source_id: SourceId::Registry {
            registry_name: registry_name.clone(),
        }
        .render(),
        name: manifest.name,
        kind: manifest.kind,
        version: manifest.version,
        source_kind: DependencySource::Registry,
        abs_dir: snapshot_dir,
        rel_dir,
        dep_bindings,
        registry_name: Some(registry_name),
        checksum: published_checksum,
        git_url: None,
        git_selector: None,
        native_products: manifest.native_products,
        foreword_products: manifest.foreword_products,
    })
}

fn load_git_pending_member(
    url: String,
    selector: Option<GitSelector>,
    package_name: String,
) -> PackageResult<PendingMember> {
    let detail = selector
        .as_ref()
        .map(GitSelector::render)
        .map(|value| format!("{url}#{value}"))
        .unwrap_or_else(|| url.clone());
    Err(format!(
        "git source loading is recognized but not enabled yet for package `{package_name}` ({detail})"
    ))
}

fn resolve_pending_member_dependencies(
    root_dir: &Path,
    package_id: &str,
    base_dir: &Path,
    manifest: &Manifest,
    existing_lock: Option<&Lockfile>,
    queue: &mut VecDeque<PendingLoad>,
) -> PackageResult<Vec<(String, String, String, DependencySpec)>> {
    let mut dep_bindings = Vec::new();
    for (dep_name, dep) in &manifest.deps {
        let (dep_package_id, dep_display_name, pending_load) = resolve_workspace_dependency(
            root_dir,
            package_id,
            dep_name,
            dep,
            base_dir,
            existing_lock,
        )?;
        dep_bindings.push((
            dep_name.clone(),
            dep_package_id.clone(),
            dep_display_name,
            dep.clone(),
        ));
        if let Some(pending_load) = pending_load {
            queue.push_back(pending_load);
        }
    }
    validate_direct_dependency_versions(&manifest.name, &dep_bindings)?;
    Ok(dep_bindings)
}

fn validate_direct_dependency_versions(
    package_name: &str,
    dep_bindings: &[(String, String, String, DependencySpec)],
) -> PackageResult<()> {
    let mut resolved_by_display = BTreeMap::<String, String>::new();
    for (_alias, dep_package_id, dep_display_name, _spec) in dep_bindings {
        if let Some(existing) =
            resolved_by_display.insert(dep_display_name.clone(), dep_package_id.clone())
            && existing != *dep_package_id
        {
            return Err(format!(
                "package `{package_name}` resolves multiple direct versions of `{dep_display_name}` (`{existing}` and `{dep_package_id}`); same-member side-by-side versions are not enabled yet"
            ));
        }
    }
    Ok(())
}

fn resolve_local_registry_version(
    registry_name: &str,
    package_name: &str,
    req: &VersionReq,
) -> PackageResult<SemverVersion> {
    let versions = list_local_registry_versions(registry_name, package_name)?;
    versions
        .into_iter()
        .filter(|candidate| req.matches(candidate))
        .max()
        .ok_or_else(|| {
            format!(
                "registry package `{package_name}` has no published version matching `{req}` in registry `{registry_name}`"
            )
        })
}

fn list_local_registry_versions(
    registry_name: &str,
    package_name: &str,
) -> PackageResult<Vec<SemverVersion>> {
    let package_root = local_registry_package_root(registry_name, package_name)?;
    if !package_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut versions = Vec::new();
    for entry in fs::read_dir(&package_root).map_err(|e| {
        format!(
            "failed to read local registry package directory `{}`: {e}",
            package_root.display()
        )
    })? {
        let entry = entry.map_err(|e| format!("failed to read registry entry: {e}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let Some(version_text) = entry.file_name().to_str().map(ToString::to_string) else {
            continue;
        };
        if let Ok(version) = SemverVersion::parse(&version_text) {
            versions.push(version);
        }
    }
    Ok(versions)
}

fn read_local_registry_manifest(
    registry_name: &str,
    package_name: &str,
    version: &SemverVersion,
) -> PackageResult<Manifest> {
    let manifest_path = local_registry_manifest_path(registry_name, package_name, version)?;
    let manifest = parse_manifest(&manifest_path)?;
    if manifest.name != package_name {
        return Err(format!(
            "registry metadata `{}` declares package `{}` instead of `{package_name}`",
            manifest_path.display(),
            manifest.name
        ));
    }
    if manifest.version.as_ref() != Some(version) {
        return Err(format!(
            "registry metadata `{}` declares version `{}` instead of `{version}`",
            manifest_path.display(),
            manifest
                .version
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "<missing>".to_string())
        ));
    }
    Ok(manifest)
}

fn read_local_registry_published_checksum(
    registry_name: &str,
    package_name: &str,
    version: &SemverVersion,
) -> PackageResult<Option<String>> {
    let manifest_path = local_registry_manifest_path(registry_name, package_name, version)?;
    let text = fs::read_to_string(&manifest_path).map_err(|e| {
        format!(
            "failed to read registry metadata `{}`: {e}",
            manifest_path.display()
        )
    })?;
    let parsed = text.parse::<toml::Value>().map_err(|e| {
        format!(
            "failed to parse registry metadata `{}` as TOML: {e}",
            manifest_path.display()
        )
    })?;
    Ok(parsed
        .get("published_checksum")
        .and_then(toml::Value::as_str)
        .map(ToString::to_string))
}

fn validate_local_registry_dependency_checksum(
    dep_name: &str,
    registry_name: &str,
    package_name: &str,
    version: &SemverVersion,
    expected_checksum: Option<&str>,
) -> PackageResult<()> {
    let Some(expected_checksum) = expected_checksum else {
        return Ok(());
    };
    let published_checksum =
        read_local_registry_published_checksum(registry_name, package_name, version)?;
    match published_checksum.as_deref() {
        Some(actual_checksum) if actual_checksum == expected_checksum => Ok(()),
        Some(actual_checksum) => Err(format!(
            "dependency `{dep_name}` requested checksum `{expected_checksum}` for `{package_name}@{version}`, but registry `{registry_name}` published `{actual_checksum}`"
        )),
        None => Err(format!(
            "dependency `{dep_name}` requested checksum `{expected_checksum}` for `{package_name}@{version}`, but registry `{registry_name}` has no published checksum"
        )),
    }
}

pub(crate) fn arcana_home_dir() -> PackageResult<PathBuf> {
    if let Some(path) = std::env::var_os("ARCANA_HOME") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(path).join(".arcana"));
    }
    if let Some(path) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(path).join(".arcana"));
    }
    Err(
        "unable to resolve `ARCANA_HOME`; set the environment variable or a home directory"
            .to_string(),
    )
}

pub(crate) fn local_registry_root(registry_name: &str) -> PackageResult<PathBuf> {
    Ok(arcana_home_dir()?
        .join("sources")
        .join("registry")
        .join(registry_name))
}

pub(crate) fn local_registry_package_root(
    registry_name: &str,
    package_name: &str,
) -> PackageResult<PathBuf> {
    Ok(local_registry_root(registry_name)?
        .join("packages")
        .join(package_name))
}

pub(crate) fn local_registry_package_dir(
    registry_name: &str,
    package_name: &str,
    version: &SemverVersion,
) -> PackageResult<PathBuf> {
    Ok(local_registry_package_root(registry_name, package_name)?.join(version.to_string()))
}

pub(crate) fn local_registry_manifest_path(
    registry_name: &str,
    package_name: &str,
    version: &SemverVersion,
) -> PackageResult<PathBuf> {
    Ok(
        local_registry_package_dir(registry_name, package_name, version)?
            .join(LOCAL_REGISTRY_METADATA_FILE),
    )
}

pub(crate) fn local_registry_snapshot_dir(
    registry_name: &str,
    package_name: &str,
    version: &SemverVersion,
) -> PackageResult<PathBuf> {
    Ok(
        local_registry_package_dir(registry_name, package_name, version)?
            .join(LOCAL_REGISTRY_SNAPSHOT_DIR),
    )
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
    let implicit_std = find_implicit_std(&root_dir)?
        .map(|std_dir| {
            let manifest = parse_manifest(&std_dir.join("book.toml"))?;
            Ok::<_, String>((std_dir, manifest))
        })
        .transpose()?;

    let root_manifest = parse_manifest(&root_dir.join("book.toml"))?;
    let root_already_in_graph = graph
        .members
        .iter()
        .any(|member| member.abs_dir == root_dir);
    if !root_already_in_graph && has_root_module(&root_dir, &root_manifest.kind) {
        let mut package = load_package_hir_with_dep_packages(
            &PackageId::Path {
                rel_path: ".".to_string(),
            }
            .render(),
            &root_dir,
            &root_manifest.name,
            &root_manifest.kind,
            resolve_manifest_dependency_packages(&root_dir, &root_manifest)?,
            resolve_manifest_dependency_ids(&root_dir, &root_manifest)?,
        )?;
        package.executable_foreword_deps = executable_foreword_aliases(&root_manifest.deps);
        package.foreword_products = lower_foreword_products(&root_manifest.foreword_products);
        attach_implicit_std_dependency(&mut package, implicit_std.as_ref());
        packages.push(package);
    }

    for member in &graph.members {
        let mut package = load_package_hir_with_dep_packages(
            &member.package_id,
            &member.abs_dir,
            &member.name,
            &member.kind,
            member.direct_dep_packages.clone(),
            member.direct_dep_ids.clone(),
        )?;
        package.executable_foreword_deps = executable_foreword_aliases(&member.direct_dep_specs);
        package.foreword_products = lower_foreword_products(&member.foreword_products);
        attach_implicit_std_dependency(&mut package, implicit_std.as_ref());
        packages.push(package);
    }

    if let Some((std_dir, manifest)) = implicit_std {
        let has_std = packages
            .iter()
            .any(|package| package.summary.package_name == manifest.name);
        if !has_std {
            let mut package = load_package_hir(
                "path:std",
                &std_dir,
                &manifest.name,
                &manifest.kind,
                BTreeSet::new(),
            )?;
            package.foreword_products = lower_foreword_products(&manifest.foreword_products);
            packages.push(package);
        }
    }

    build_workspace_summary(packages)
}

fn attach_implicit_std_dependency(
    package: &mut HirWorkspacePackage,
    implicit_std: Option<&(PathBuf, Manifest)>,
) {
    let Some((_std_dir, std_manifest)) = implicit_std else {
        return;
    };
    if package.summary.package_name == std_manifest.name {
        return;
    }
    let uses_std = package.summary.dependency_edges.iter().any(|edge| {
        edge.target_path
            .first()
            .is_some_and(|segment| segment == "std")
    });
    if !uses_std {
        return;
    }
    package.direct_deps.insert("path:std".to_string());
    package
        .direct_dep_packages
        .entry("std".to_string())
        .or_insert_with(|| std_manifest.name.clone());
    package
        .direct_dep_ids
        .entry("std".to_string())
        .or_insert_with(|| "path:std".to_string());
}

fn executable_foreword_aliases(specs: &BTreeMap<String, DependencySpec>) -> BTreeSet<String> {
    specs
        .iter()
        .filter_map(|(alias, spec)| spec.executable_forewords.then_some(alias.clone()))
        .collect()
}

fn lower_foreword_products(
    products: &BTreeMap<String, ForewordAdapterProductSpec>,
) -> BTreeMap<String, HirForewordAdapterProduct> {
    products
        .iter()
        .map(|(name, product)| {
            (
                name.clone(),
                HirForewordAdapterProduct {
                    name: product.name.clone(),
                    path: product.path.clone(),
                    runner: product.runner.clone(),
                    args: product.args.clone(),
                },
            )
        })
        .collect()
}

pub fn load_member_hir_package(member: &WorkspaceMember) -> PackageResult<HirWorkspacePackage> {
    let mut package = load_package_hir_with_dep_packages(
        &member.package_id,
        &member.abs_dir,
        &member.name,
        &member.kind,
        member.direct_dep_packages.clone(),
        member.direct_dep_ids.clone(),
    )?;
    package.executable_foreword_deps = executable_foreword_aliases(&member.direct_dep_specs);
    package.foreword_products = lower_foreword_products(&member.foreword_products);
    Ok(package)
}

pub fn plan_workspace(graph: &WorkspaceGraph) -> PackageResult<Vec<String>> {
    let indegree = graph
        .members
        .iter()
        .map(|member| (member.package_id.clone(), member.deps.len()))
        .collect::<HashMap<_, _>>();
    let mut indegree = indegree;
    let mut dependents = HashMap::<String, Vec<String>>::new();
    for member in &graph.members {
        for dep in &member.deps {
            dependents
                .entry(dep.clone())
                .or_default()
                .push(member.package_id.clone());
        }
    }

    let mut ready = graph
        .members
        .iter()
        .filter(|member| member.deps.is_empty())
        .map(|member| member.package_id.clone())
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
    let lockfile = match version {
        LOCKFILE_VERSION => read_lockfile_build_table(table, path, version)?,
        LEGACY_LOCKFILE_VERSION | OLDER_LOCKFILE_VERSION => {
            read_lockfile_legacy_build_table(table, path, version)?
        }
        OLDEST_LOCKFILE_VERSION => read_lockfile_v1(table, path, version)?,
        _ => {
            return Err(format!(
                "unsupported lockfile version `{version}` in `{}`; expected {LOCKFILE_VERSION}, {LEGACY_LOCKFILE_VERSION}, {OLDER_LOCKFILE_VERSION}, or {OLDEST_LOCKFILE_VERSION}",
                path.display()
            ));
        }
    };
    Ok(Some(lockfile))
}

fn read_lockfile_build_table(
    table: &toml::value::Table,
    path: &Path,
    version: i64,
) -> PackageResult<Lockfile> {
    let workspace = read_lockfile_workspace(table, path)?;
    let workspace_root = table
        .get("workspace_root")
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("missing `workspace_root` in `{}`", path.display()))?;
    let order = read_lockfile_order(table)?;
    let workspace_members = table
        .get("workspace_members")
        .map(parse_string_array)
        .transpose()?
        .unwrap_or_default();
    let base_members = read_lockfile_member_bases(table, path)?;
    let native_products = read_lockfile_native_products(table, path)?;
    let dependencies = read_lockfile_dependencies(table, path)?;
    let builds = table
        .get("builds")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[builds]` in `{}`", path.display()))?;

    let mut members = BTreeMap::new();
    for (package_id, base) in base_members {
        let member_native_products = native_products
            .get(&package_id)
            .or_else(|| native_products.get(&base.name))
            .cloned()
            .unwrap_or_default();
        let dep_bindings = dependencies.get(&package_id).cloned().unwrap_or_default();
        let deps = dep_bindings
            .values()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let targets = builds
            .get(&package_id)
            .map(|value| {
                let target_table = value.as_table().ok_or_else(|| {
                    format!("lockfile build entry for `{package_id}` must be a table")
                })?;
                read_lock_target_entries(package_id.as_str(), target_table)
            })
            .transpose()?
            .unwrap_or_default();
        members.insert(
            package_id,
            LockMember {
                name: base.name,
                deps,
                dep_bindings,
                kind: base.kind,
                source_kind: base.source_kind,
                path: base.path,
                version: base.version,
                registry_name: base.registry_name,
                checksum: base.checksum,
                git_url: base.git_url,
                git_selector: base.git_selector,
                native_products: member_native_products,
                targets,
            },
        );
    }

    Ok(Lockfile {
        version,
        workspace,
        workspace_root,
        order,
        workspace_members,
        members,
    })
}

fn read_lockfile_legacy_build_table(
    table: &toml::value::Table,
    path: &Path,
    version: i64,
) -> PackageResult<Lockfile> {
    let workspace = read_lockfile_workspace(table, path)?;
    let order = read_lockfile_order(table)?;
    let base_members = read_lockfile_member_bases_legacy(table, path)?;
    let native_products = read_lockfile_native_products(table, path)?;
    let builds = table
        .get("builds")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[builds]` in `{}`", path.display()))?;

    let workspace_root = base_members
        .iter()
        .find_map(|(package_id, base)| {
            (base.path.as_deref() == Some(".")).then_some(package_id.clone())
        })
        .unwrap_or_else(|| {
            base_members
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "path:.".to_string())
        });
    let workspace_members = base_members.keys().cloned().collect::<Vec<_>>();
    let order = order
        .into_iter()
        .map(|entry| {
            base_members
                .get(&entry)
                .map(|_| entry.clone())
                .or_else(|| {
                    base_members
                        .iter()
                        .find(|(_id, base)| base.name == entry)
                        .map(|(id, _)| id.clone())
                })
                .unwrap_or(entry)
        })
        .collect::<Vec<_>>();

    let mut members = BTreeMap::new();
    for (package_id, base) in base_members {
        let member_native_products = native_products
            .get(&package_id)
            .or_else(|| native_products.get(&base.name))
            .cloned()
            .unwrap_or_default();
        let targets = builds
            .get(&base.name)
            .or_else(|| builds.get(&package_id))
            .map(|value| {
                let target_table = value.as_table().ok_or_else(|| {
                    format!("lockfile build entry for `{package_id}` must be a table")
                })?;
                read_lock_target_entries(package_id.as_str(), target_table)
            })
            .transpose()?
            .unwrap_or_default();
        members.insert(
            package_id,
            LockMember {
                name: base.name,
                deps: base.deps,
                dep_bindings: base.dep_bindings,
                kind: base.kind,
                source_kind: base.source_kind,
                path: base.path,
                version: base.version,
                registry_name: base.registry_name,
                checksum: base.checksum,
                git_url: base.git_url,
                git_selector: base.git_selector,
                native_products: member_native_products,
                targets,
            },
        );
    }

    Ok(Lockfile {
        version,
        workspace,
        workspace_root,
        order,
        workspace_members,
        members,
    })
}

fn read_lockfile_v1(
    table: &toml::value::Table,
    path: &Path,
    version: i64,
) -> PackageResult<Lockfile> {
    let workspace = read_lockfile_workspace(table, path)?;
    let order = read_lockfile_order(table)?;
    let base_members = read_lockfile_member_bases_legacy(table, path)?;
    let toolchain = table
        .get("toolchain")
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .to_string();
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
    let targets = table.get("targets").and_then(toml::Value::as_table);
    let artifact_hashes = table.get("artifact_hashes").and_then(toml::Value::as_table);
    let formats = table
        .get("formats")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[formats]` in `{}`", path.display()))?;

    let mut members = BTreeMap::new();
    let workspace_root = base_members
        .iter()
        .find_map(|(package_id, base)| {
            (base.path.as_deref() == Some(".")).then_some(package_id.clone())
        })
        .unwrap_or_else(|| {
            base_members
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "path:.".to_string())
        });
    let workspace_members = base_members.keys().cloned().collect::<Vec<_>>();
    let order = order
        .into_iter()
        .map(|entry| {
            base_members
                .get(&entry)
                .map(|_| entry.clone())
                .or_else(|| {
                    base_members
                        .iter()
                        .find(|(_id, base)| base.name == entry)
                        .map(|(id, _)| id.clone())
                })
                .unwrap_or(entry)
        })
        .collect::<Vec<_>>();
    for (package_id, base) in base_members {
        let fingerprint = fingerprints
            .get(&base.name)
            .or_else(|| fingerprints.get(&package_id))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing fingerprint for `{}`", base.name))?
            .to_string();
        let api_fingerprint = api_fingerprints
            .get(&base.name)
            .or_else(|| api_fingerprints.get(&package_id))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing api fingerprint for `{}`", base.name))?
            .to_string();
        let artifact = artifacts
            .get(&base.name)
            .or_else(|| artifacts.get(&package_id))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing artifact path for `{}`", base.name))?
            .to_string();
        let artifact_hash = match artifact_hashes {
            Some(hashes) => hashes
                .get(&base.name)
                .or_else(|| hashes.get(&package_id))
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("missing artifact hash for `{}`", base.name))?
                .to_string(),
            None => String::new(),
        };
        let format = formats
            .get(&base.name)
            .or_else(|| formats.get(&package_id))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing format for `{}`", base.name))?
            .to_string();
        let target = match targets
            .and_then(|target_rows| {
                target_rows
                    .get(&base.name)
                    .or_else(|| target_rows.get(&package_id))
            })
            .and_then(toml::Value::as_str)
        {
            Some(target) => BuildTarget::from_storage_key(target),
            None => infer_build_target_from_format(&format).ok_or_else(|| {
                format!(
                    "missing target for `{}` and unable to infer one from format `{format}`",
                    base.name
                )
            })?,
        };
        let mut target_entries = BTreeMap::new();
        if lock_target_format_matches(&target, &format) {
            target_entries.insert(
                BuildOutputKey::target(target),
                LockTargetEntry {
                    fingerprint,
                    api_fingerprint,
                    artifact,
                    artifact_hash,
                    format,
                    toolchain: toolchain.clone(),
                    product: None,
                    native_product_closure: None,
                },
            );
        }
        members.insert(
            package_id,
            LockMember {
                name: base.name,
                deps: base.deps,
                dep_bindings: base.dep_bindings,
                kind: base.kind,
                source_kind: base.source_kind,
                path: base.path,
                version: base.version,
                registry_name: base.registry_name,
                checksum: base.checksum,
                git_url: base.git_url,
                git_selector: base.git_selector,
                native_products: BTreeMap::new(),
                targets: target_entries,
            },
        );
    }

    Ok(Lockfile {
        version,
        workspace,
        workspace_root,
        order,
        workspace_members,
        members,
    })
}

fn read_lockfile_native_products(
    table: &toml::value::Table,
    path: &Path,
) -> PackageResult<BTreeMap<String, BTreeMap<String, LockNativeProductEntry>>> {
    let Some(member_table) = table.get("native_products").and_then(toml::Value::as_table) else {
        return Ok(BTreeMap::new());
    };
    let mut members = BTreeMap::new();
    for (member_name, value) in member_table {
        let product_table = value.as_table().ok_or_else(|| {
            format!(
                "lockfile native product entry for `{member_name}` in `{}` must be a table",
                path.display()
            )
        })?;
        let mut products = BTreeMap::new();
        for (product_name, value) in product_table {
            let entry = value.as_table().ok_or_else(|| {
                format!(
                    "lockfile native product `{member_name}:{product_name}` in `{}` must be a table",
                    path.display()
                )
            })?;
            products.insert(
                product_name.clone(),
                LockNativeProductEntry {
                    kind: required_lockfile_string_field(
                        entry,
                        path,
                        &format!("native_products.{member_name}.{product_name}.kind"),
                    )?,
                    role: required_lockfile_string_field(
                        entry,
                        path,
                        &format!("native_products.{member_name}.{product_name}.role"),
                    )?,
                    producer: required_lockfile_string_field(
                        entry,
                        path,
                        &format!("native_products.{member_name}.{product_name}.producer"),
                    )?,
                    file: required_lockfile_string_field(
                        entry,
                        path,
                        &format!("native_products.{member_name}.{product_name}.file"),
                    )?,
                    contract: required_lockfile_string_field(
                        entry,
                        path,
                        &format!("native_products.{member_name}.{product_name}.contract"),
                    )?,
                    rust_cdylib_crate: entry
                        .get("rust_cdylib_crate")
                        .and_then(toml::Value::as_str)
                        .map(ToString::to_string),
                    sidecars: entry
                        .get("sidecars")
                        .map(parse_string_array)
                        .transpose()?
                        .unwrap_or_default(),
                },
            );
        }
        members.insert(member_name.clone(), products);
    }
    Ok(members)
}

fn required_lockfile_string_field(
    table: &toml::value::Table,
    path: &Path,
    field: &str,
) -> PackageResult<String> {
    table
        .get(field.rsplit('.').next().unwrap_or(field))
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("missing `{field}` in `{}`", path.display()))
}

fn read_lockfile_workspace(table: &toml::value::Table, path: &Path) -> PackageResult<String> {
    table
        .get("workspace")
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("missing `workspace` in `{}`", path.display()))
}

fn read_lockfile_order(table: &toml::value::Table) -> PackageResult<Vec<String>> {
    table
        .get("order")
        .map(parse_string_array)
        .transpose()
        .map(|order| order.unwrap_or_default())
}

fn read_lockfile_dependencies(
    table: &toml::value::Table,
    path: &Path,
) -> PackageResult<BTreeMap<String, BTreeMap<String, String>>> {
    let Some(dependency_table) = table.get("dependencies").and_then(toml::Value::as_table) else {
        return Ok(BTreeMap::new());
    };
    let mut dependencies = BTreeMap::new();
    for (package_id, value) in dependency_table {
        let binding_table = value.as_table().ok_or_else(|| {
            format!(
                "lockfile dependency entry for `{package_id}` in `{}` must be a table",
                path.display()
            )
        })?;
        let mut bindings = BTreeMap::new();
        for (alias, dep_id) in binding_table {
            let dep_id = dep_id.as_str().ok_or_else(|| {
                format!(
                    "lockfile dependency `{package_id}.{alias}` in `{}` must be a string",
                    path.display()
                )
            })?;
            bindings.insert(alias.clone(), dep_id.to_string());
        }
        dependencies.insert(package_id.clone(), bindings);
    }
    Ok(dependencies)
}

fn read_lockfile_member_bases(
    table: &toml::value::Table,
    path: &Path,
) -> PackageResult<BTreeMap<String, LockMemberBase>> {
    let packages = table
        .get("packages")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[packages]` in `{}`", path.display()))?;

    let mut members = BTreeMap::new();
    for (package_id, value) in packages {
        let package = value.as_table().ok_or_else(|| {
            format!(
                "lockfile package entry for `{package_id}` in `{}` must be a table",
                path.display()
            )
        })?;
        let name =
            required_lockfile_string_field(package, path, &format!("packages.{package_id}.name"))?;
        let kind = match required_lockfile_string_field(
            package,
            path,
            &format!("packages.{package_id}.kind"),
        )?
        .as_str()
        {
            "app" => GrimoireKind::App,
            "lib" => GrimoireKind::Lib,
            other => return Err(format!("unsupported kind `{other}` for `{package_id}`")),
        };
        let source_kind = match required_lockfile_string_field(
            package,
            path,
            &format!("packages.{package_id}.source_kind"),
        )?
        .as_str()
        {
            "path" => DependencySource::Path,
            "registry" => DependencySource::Registry,
            "git" => DependencySource::Git,
            other => {
                return Err(format!(
                    "unsupported source kind `{other}` for `{package_id}`"
                ));
            }
        };
        let path_value = package
            .get("path")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string);
        let version = package
            .get("version")
            .and_then(toml::Value::as_str)
            .map(SemverVersion::parse)
            .transpose()?;
        let registry_name = package
            .get("registry")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string);
        let checksum = package
            .get("checksum")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string);
        let git_url = package
            .get("git")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string);
        let git_selector = package
            .get("git_selector")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string);
        members.insert(
            package_id.clone(),
            LockMemberBase {
                name,
                deps: Vec::new(),
                dep_bindings: BTreeMap::new(),
                kind,
                source_kind,
                path: path_value,
                version,
                registry_name,
                checksum,
                git_url,
                git_selector,
            },
        );
    }
    Ok(members)
}

fn read_lockfile_member_bases_legacy(
    table: &toml::value::Table,
    path: &Path,
) -> PackageResult<BTreeMap<String, LockMemberBase>> {
    let paths = table
        .get("paths")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[paths]` in `{}`", path.display()))?;
    let deps = table
        .get("deps")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[deps]` in `{}`", path.display()))?;
    let kinds = table
        .get("kinds")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing `[kinds]` in `{}`", path.display()))?;

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
        let kind = match kinds
            .get(name)
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("missing kind for `{name}`"))?
        {
            "app" => GrimoireKind::App,
            "lib" => GrimoireKind::Lib,
            other => return Err(format!("unsupported kind `{other}` for `{name}`")),
        };
        let package_id = PackageId::Path {
            rel_path: path.clone(),
        }
        .render();
        members.insert(
            package_id,
            LockMemberBase {
                name: name.clone(),
                path: Some(path),
                deps: dep_list,
                dep_bindings: BTreeMap::new(),
                kind,
                source_kind: DependencySource::Path,
                version: None,
                registry_name: None,
                checksum: None,
                git_url: None,
                git_selector: None,
            },
        );
    }
    let name_to_id = members
        .iter()
        .map(|(package_id, base)| (base.name.clone(), package_id.clone()))
        .collect::<BTreeMap<_, _>>();
    for member in members.values_mut() {
        member.dep_bindings = member
            .deps
            .iter()
            .filter_map(|dep_name| {
                name_to_id
                    .get(dep_name)
                    .cloned()
                    .map(|dep_id| (dep_name.clone(), dep_id))
            })
            .collect();
        member.deps = member.dep_bindings.values().cloned().collect();
    }
    Ok(members)
}

fn read_lock_target_entries(
    member_name: &str,
    target_table: &toml::value::Table,
) -> PackageResult<BTreeMap<BuildOutputKey, LockTargetEntry>> {
    let mut targets = BTreeMap::new();
    for (target_key, value) in target_table {
        let build_table = value.as_table().ok_or_else(|| {
            format!(
                "lockfile build entry for `{member_name}` target `{target_key}` must be a table"
            )
        })?;
        let build_key = BuildOutputKey::from_storage_key(target_key);
        let target = build_key.target.clone();
        let fingerprint =
            read_lock_target_field(member_name, target_key, build_table, "fingerprint")?;
        let api_fingerprint =
            read_lock_target_field(member_name, target_key, build_table, "api_fingerprint")?;
        let artifact = read_lock_target_field(member_name, target_key, build_table, "artifact")?;
        let artifact_hash =
            read_lock_target_field(member_name, target_key, build_table, "artifact_hash")?;
        let format = read_lock_target_field(member_name, target_key, build_table, "format")?;
        let toolchain = read_lock_target_field(member_name, target_key, build_table, "toolchain")?;
        let product = build_table
            .get("product")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string)
            .or_else(|| build_key.product.clone());
        let native_product_closure = build_table
            .get("native_product_closure")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string);
        if !lock_target_format_matches(&target, &format) {
            continue;
        }
        targets.insert(
            BuildOutputKey::new(target, product.clone()),
            LockTargetEntry {
                fingerprint,
                api_fingerprint,
                artifact,
                artifact_hash,
                format,
                toolchain,
                product,
                native_product_closure,
            },
        );
    }
    Ok(targets)
}

fn read_lock_target_field(
    member_name: &str,
    target_key: &str,
    table: &toml::value::Table,
    field: &str,
) -> PackageResult<String> {
    table
        .get(field)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "lockfile build entry for `{member_name}` target `{target_key}` is missing `{field}`"
            )
        })
}

fn lock_target_format_matches(target: &BuildTarget, format: &str) -> bool {
    target
        .format()
        .map(|expected| format == expected)
        .unwrap_or(true)
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

fn native_product_probe(event: &str, message: impl AsRef<str>) {
    if std::env::var_os(ARCANA_NATIVE_PRODUCT_TEMP_PROBES_ENV).is_some() {
        eprintln!(
            "[arcana-native-product-probe] {event}: {}",
            message.as_ref()
        );
    }
}

fn parse_dependency_spec(
    name: &str,
    value: &toml::Value,
    manifest_path: &Path,
) -> PackageResult<DependencySpec> {
    if let Some(path) = value.as_str() {
        return Ok(DependencySpec {
            package: name.to_string(),
            source: DependencySourceSpec::Path {
                location: path.to_string(),
            },
            native_delivery: NativeDependencyDelivery::Baked,
            native_child: None,
            native_plugins: Vec::new(),
            executable_forewords: false,
        });
    }
    let Some(table) = value.as_table() else {
        return Err(format!(
            "dependency `{name}` in `{}` must be a string or table",
            manifest_path.display()
        ));
    };
    let reserved_keys = [
        "path",
        "package",
        "version",
        "registry",
        "git",
        "rev",
        "tag",
        "branch",
        "checksum",
        "native_delivery",
        "native_child",
        "native_plugins",
        "executable_forewords",
    ];
    for key in table.keys() {
        if !reserved_keys.contains(&key.as_str()) {
            return Err(format!(
                "dependency `{name}` in `{}` uses unsupported key `{key}`",
                manifest_path.display()
            ));
        }
    }
    let package = table
        .get("package")
        .map(|value| {
            value.as_str().map(ToString::to_string).ok_or_else(|| {
                format!(
                    "dependency `{name}` in `{}` must set `package` as a string",
                    manifest_path.display()
                )
            })
        })
        .transpose()?
        .unwrap_or_else(|| name.to_string());
    let native_delivery = match table.get("native_delivery") {
        Some(value) => NativeDependencyDelivery::parse(value.as_str().ok_or_else(|| {
            format!(
                "dependency `{name}` in `{}` must set `native_delivery` as a string",
                manifest_path.display()
            )
        })?)?,
        None => NativeDependencyDelivery::Baked,
    };
    let native_child = table
        .get("native_child")
        .map(|value| {
            value.as_str().map(ToString::to_string).ok_or_else(|| {
                format!(
                    "dependency `{name}` in `{}` must set `native_child` as a string",
                    manifest_path.display()
                )
            })
        })
        .transpose()?;
    let native_plugins = table
        .get("native_plugins")
        .map(parse_string_array)
        .transpose()?
        .unwrap_or_default();
    let executable_forewords = table
        .get("executable_forewords")
        .map(|value| {
            value.as_bool().ok_or_else(|| {
                format!(
                    "dependency `{name}` in `{}` must set `executable_forewords` as a bool",
                    manifest_path.display()
                )
            })
        })
        .transpose()?
        .unwrap_or(false);
    if native_delivery == NativeDependencyDelivery::Dll
        && native_child
            .as_deref()
            .is_some_and(|child| child != "default")
    {
        native_product_probe(
            "invalid_legacy_child_selection",
            format!(
                "dependency={} manifest={} native_child={}",
                name,
                manifest_path.display(),
                native_child.as_deref().unwrap_or_default()
            ),
        );
        return Err(format!(
            "dependency `{name}` in `{}` cannot mix `native_delivery = \"dll\"` with non-default `native_child = \"{}\"`",
            manifest_path.display(),
            native_child.as_deref().unwrap_or_default()
        ));
    }
    if native_delivery == NativeDependencyDelivery::Dll && native_child.is_none() {
        native_product_probe(
            "legacy_native_delivery_alias",
            format!(
                "dependency={} manifest={} selected_child=default",
                name,
                manifest_path.display()
            ),
        );
    }
    let path = table.get("path").and_then(toml::Value::as_str);
    let version = table
        .get("version")
        .map(|value| {
            value.as_str().ok_or_else(|| {
                format!(
                    "dependency `{name}` in `{}` must set `version` as a string",
                    manifest_path.display()
                )
            })
        })
        .transpose()?;
    let registry = table.get("registry").and_then(toml::Value::as_str);
    let checksum = table
        .get("checksum")
        .map(|value| {
            value.as_str().map(ToString::to_string).ok_or_else(|| {
                format!(
                    "dependency `{name}` in `{}` must set `checksum` as a string",
                    manifest_path.display()
                )
            })
        })
        .transpose()?;
    let git = table.get("git").and_then(toml::Value::as_str);
    let rev = table.get("rev").and_then(toml::Value::as_str);
    let tag = table.get("tag").and_then(toml::Value::as_str);
    let branch = table.get("branch").and_then(toml::Value::as_str);
    let selector_count =
        usize::from(rev.is_some()) + usize::from(tag.is_some()) + usize::from(branch.is_some());
    if selector_count > 1 {
        return Err(format!(
            "dependency `{name}` in `{}` may set only one of `rev`, `tag`, or `branch`",
            manifest_path.display()
        ));
    }
    let source_count =
        usize::from(path.is_some()) + usize::from(version.is_some()) + usize::from(git.is_some());
    if source_count == 0 {
        return Err(format!(
            "dependency `{name}` in `{}` must set `path`, `version`, or `git`",
            manifest_path.display()
        ));
    }
    if source_count > 1 {
        return Err(format!(
            "dependency `{name}` in `{}` must use exactly one source kind",
            manifest_path.display()
        ));
    }
    if let Some(path) = path {
        if version.is_some() || registry.is_some() || git.is_some() || selector_count > 0 {
            return Err(format!(
                "dependency `{name}` in `{}` cannot mix `path` with versioned or git settings",
                manifest_path.display()
            ));
        }
        return Ok(DependencySpec {
            package,
            source: DependencySourceSpec::Path {
                location: path.to_string(),
            },
            native_delivery,
            native_child,
            native_plugins,
            executable_forewords,
        });
    }
    if let Some(git) = git {
        return Ok(DependencySpec {
            package,
            source: DependencySourceSpec::Git {
                url: git.to_string(),
                selector: rev
                    .map(|value| GitSelector::Rev(value.to_string()))
                    .or_else(|| tag.map(|value| GitSelector::Tag(value.to_string())))
                    .or_else(|| branch.map(|value| GitSelector::Branch(value.to_string()))),
            },
            native_delivery,
            native_child,
            native_plugins,
            executable_forewords,
        });
    }
    if let Some(version) = version {
        return Ok(DependencySpec {
            package,
            source: DependencySourceSpec::Registry {
                registry_name: registry.map(ToString::to_string),
                version: VersionReq::parse(version)?,
                checksum,
            },
            native_delivery,
            native_child,
            native_plugins,
            executable_forewords,
        });
    }
    Err(format!(
        "dependency `{name}` in `{}` must set `path`, `version`, or `git`",
        manifest_path.display()
    ))
}

fn parse_native_products(
    table: &toml::value::Table,
    manifest_path: &Path,
) -> PackageResult<BTreeMap<String, NativeProductSpec>> {
    let Some(products) = table
        .get("native")
        .and_then(toml::Value::as_table)
        .and_then(|native| native.get("products"))
        .and_then(toml::Value::as_table)
    else {
        return Ok(BTreeMap::new());
    };
    let mut parsed = BTreeMap::new();
    for (name, value) in products {
        let product = value.as_table().ok_or_else(|| {
            format!(
                "native product `{name}` in `{}` must be a table",
                manifest_path.display()
            )
        })?;
        let kind = product
            .get("kind")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!(
                    "native product `{name}` in `{}` is missing `kind`",
                    manifest_path.display()
                )
            })?
            .to_string();
        if kind != "dll" {
            return Err(format!(
                "native product `{name}` in `{}` must set `kind = \"dll\"` for now",
                manifest_path.display()
            ));
        }
        let role = ArcanaCabiProductRole::parse(
            product
                .get("role")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    format!(
                        "native product `{name}` in `{}` is missing `role`",
                        manifest_path.display()
                    )
                })?,
        )?;
        let producer = NativeProductProducer::parse(
            product
                .get("producer")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| {
                    format!(
                        "native product `{name}` in `{}` is missing `producer`",
                        manifest_path.display()
                    )
                })?,
        )?;
        let file = product
            .get("file")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!(
                    "native product `{name}` in `{}` is missing `file`",
                    manifest_path.display()
                )
            })?
            .to_string();
        validate_support_file_relative_path(&file)?;
        let contract = product
            .get("contract")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!(
                    "native product `{name}` in `{}` is missing `contract`",
                    manifest_path.display()
                )
            })?
            .to_string();
        let rust_cdylib_crate = product
            .get("rust_cdylib_crate")
            .map(|value| {
                value.as_str().map(ToString::to_string).ok_or_else(|| {
                    format!(
                        "native product `{name}` in `{}` must set `rust_cdylib_crate` as a string",
                        manifest_path.display()
                    )
                })
            })
            .transpose()?;
        let sidecars = product
            .get("sidecars")
            .map(parse_string_array)
            .transpose()?
            .unwrap_or_default();
        for sidecar in &sidecars {
            validate_support_file_relative_path(sidecar)?;
        }
        if producer == NativeProductProducer::RustCdylib && rust_cdylib_crate.is_none() {
            return Err(format!(
                "native product `{name}` in `{}` must set `rust_cdylib_crate` for `producer = \"rust-cdylib\"`",
                manifest_path.display()
            ));
        }
        if producer == NativeProductProducer::ArcanaSource && rust_cdylib_crate.is_some() {
            return Err(format!(
                "native product `{name}` in `{}` cannot set `rust_cdylib_crate` for `producer = \"arcana-source\"`",
                manifest_path.display()
            ));
        }
        parsed.insert(
            name.clone(),
            NativeProductSpec {
                name: name.clone(),
                kind,
                role,
                producer,
                file,
                contract,
                rust_cdylib_crate,
                sidecars,
            },
        );
    }
    Ok(parsed)
}

fn parse_foreword_products(
    table: &toml::value::Table,
    manifest_path: &Path,
) -> PackageResult<BTreeMap<String, ForewordAdapterProductSpec>> {
    let Some(products) = table
        .get("toolchain")
        .and_then(toml::Value::as_table)
        .and_then(|toolchain| toolchain.get("foreword_products"))
        .and_then(toml::Value::as_table)
    else {
        return Ok(BTreeMap::new());
    };
    let mut parsed = BTreeMap::new();
    for (name, value) in products {
        let product = value.as_table().ok_or_else(|| {
            format!(
                "foreword product `{name}` in `{}` must be a table",
                manifest_path.display()
            )
        })?;
        let path = product
            .get("path")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!(
                    "foreword product `{name}` in `{}` is missing `path`",
                    manifest_path.display()
                )
            })?
            .to_string();
        validate_support_file_relative_path(&path)?;
        let runner = product
            .get("runner")
            .map(|value| {
                value.as_str().map(ToString::to_string).ok_or_else(|| {
                    format!(
                        "foreword product `{name}` in `{}` must set `runner` as a string",
                        manifest_path.display()
                    )
                })
            })
            .transpose()?;
        let args = product
            .get("args")
            .map(parse_string_array)
            .transpose()?
            .unwrap_or_default();
        for key in product.keys() {
            if !matches!(key.as_str(), "path" | "runner" | "args") {
                return Err(format!(
                    "foreword product `{name}` in `{}` uses unsupported key `{key}`",
                    manifest_path.display()
                ));
            }
        }
        parsed.insert(
            name.clone(),
            ForewordAdapterProductSpec {
                name: name.clone(),
                path,
                runner,
                args,
            },
        );
    }
    Ok(parsed)
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
    package_id: &str,
    root_dir: &Path,
    name: &str,
    kind: &GrimoireKind,
    direct_deps: BTreeSet<String>,
) -> PackageResult<HirWorkspacePackage> {
    let direct_dep_packages: BTreeMap<String, String> = direct_deps
        .into_iter()
        .map(|dep| (dep.clone(), dep))
        .collect();
    load_package_hir_with_dep_packages(
        package_id,
        root_dir,
        name,
        kind,
        direct_dep_packages.clone(),
        direct_dep_packages
            .iter()
            .map(|(alias, package_name)| (alias.clone(), package_name.clone()))
            .collect(),
    )
}

fn load_package_hir_with_dep_packages(
    package_id: &str,
    root_dir: &Path,
    name: &str,
    kind: &GrimoireKind,
    direct_dep_packages: BTreeMap<String, String>,
    direct_dep_ids: BTreeMap<String, String>,
) -> PackageResult<HirWorkspacePackage> {
    let files = collect_arc_files(&root_dir.join("src"))?;
    build_package_hir(
        package_id,
        root_dir,
        name,
        kind,
        direct_dep_packages,
        direct_dep_ids,
        &files,
    )
}

fn build_package_hir(
    package_id: &str,
    root_dir: &Path,
    name: &str,
    kind: &GrimoireKind,
    direct_dep_packages: BTreeMap<String, String>,
    direct_dep_ids: BTreeMap<String, String>,
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
        if !relative_key.is_empty()
            && relative_to_absolute
                .insert(relative_key.clone(), module.module_id.clone())
                .is_some()
        {
            return Err(format!(
                "duplicate module path `{relative_key}` in `{}`",
                root_dir.display()
            ));
        }
        modules.push(module);
    }

    let summary = build_package_summary(name.to_string(), modules);
    let layout = build_package_layout(&summary, module_paths, relative_to_absolute)?;
    arcana_hir::build_workspace_package_with_dep_packages(
        package_id.to_string(),
        root_dir.to_path_buf(),
        direct_dep_packages,
        direct_dep_ids,
        summary,
        layout,
    )
}

fn resolve_manifest_dependency_packages(
    root_dir: &Path,
    manifest: &Manifest,
) -> PackageResult<BTreeMap<String, String>> {
    let mut direct_dep_packages = BTreeMap::new();
    for (dep_name, dep) in &manifest.deps {
        match &dep.source {
            DependencySourceSpec::Path { location } => {
                let dep_dir = canonicalize_dir(&root_dir.join(location))?;
                let dep_manifest = parse_manifest(&dep_dir.join("book.toml"))?;
                direct_dep_packages.insert(dep_name.clone(), dep_manifest.name);
            }
            DependencySourceSpec::Registry { .. } | DependencySourceSpec::Git { .. } => {
                direct_dep_packages.insert(dep_name.clone(), dep.package.clone());
            }
        }
    }
    Ok(direct_dep_packages)
}

fn resolve_manifest_dependency_ids(
    root_dir: &Path,
    manifest: &Manifest,
) -> PackageResult<BTreeMap<String, String>> {
    let lock = read_lockfile(&root_dir.join("Arcana.lock"))?;
    let package_id = PackageId::Path {
        rel_path: ".".to_string(),
    }
    .render();
    let mut direct_dep_ids = BTreeMap::new();
    for (dep_name, dep) in &manifest.deps {
        let (dep_package_id, _dep_display_name, _pending_load) = resolve_workspace_dependency(
            root_dir,
            &package_id,
            dep_name,
            dep,
            root_dir,
            lock.as_ref(),
        )?;
        direct_dep_ids.insert(dep_name.clone(), dep_package_id);
    }
    Ok(direct_dep_ids)
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

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_aot::{AOT_WINDOWS_DLL_FORMAT, AOT_WINDOWS_EXE_FORMAT, parse_package_artifact};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_root() -> PathBuf {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir
            .parent()
            .and_then(Path::parent)
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        repo_root()
            .join("target")
            .join("arcana-package-tests")
            .join(format!("{name}_{nanos}"))
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

    fn write_std_io_grimoire(dir: &Path) {
        write_grimoire(&dir.join("std"), GrimoireKind::Lib, "std", &[]);
        write_file(&dir.join("std/src/book.arc"), "// std root\n");
        write_file(&dir.join("std/src/types.arc"), "// std types\n");
        write_file(
            &dir.join("std/src/io.arc"),
            concat!(
                "import std.kernel.io\n",
                "export fn print[T](read value: T):\n",
                "    std.kernel.io.print[T] :: value :: call\n",
            ),
        );
        write_file(
            &dir.join("std/src/kernel/io.arc"),
            "intrinsic fn print[T](read value: T) = IoPrint\n",
        );
    }

    fn prepare_test_build(graph: &WorkspaceGraph) -> PreparedBuild {
        prepare_build(graph).expect("prepare build")
    }

    fn plan_test_build(
        graph: &WorkspaceGraph,
        order: &[String],
        existing_lock: Option<&Lockfile>,
    ) -> (PreparedBuild, Vec<BuildStatus>) {
        let prepared = prepare_test_build(graph);
        let statuses = plan_build(graph, order, &prepared, existing_lock).expect("build plan");
        (prepared, statuses)
    }

    fn member_id(graph: &WorkspaceGraph, member: &str) -> String {
        graph
            .member(member)
            .unwrap_or_else(|| panic!("member `{member}` should exist"))
            .package_id
            .clone()
    }

    fn order_display_names(graph: &WorkspaceGraph, order: &[String]) -> Vec<String> {
        order
            .iter()
            .map(|member| {
                graph
                    .member(member)
                    .map(|member| member.name.clone())
                    .unwrap_or_else(|| member.clone())
            })
            .collect()
    }

    fn lock_member<'a>(
        lock: &'a Lockfile,
        graph: Option<&WorkspaceGraph>,
        member: &str,
    ) -> &'a LockMember {
        if let Some(found) = lock.members.get(member) {
            return found;
        }
        if let Some(graph) = graph {
            let package_id = member_id(graph, member);
            if let Some(found) = lock.members.get(&package_id) {
                return found;
            }
        }
        let legacy_path_id = PackageId::Path {
            rel_path: member.to_string(),
        }
        .render();
        lock.members
            .get(&legacy_path_id)
            .unwrap_or_else(|| panic!("lock member `{member}` should exist"))
    }

    fn fingerprint_member<'a>(
        fingerprints: &'a WorkspaceFingerprints,
        graph: &WorkspaceGraph,
        member: &str,
    ) -> &'a MemberFingerprints {
        let package_id = member_id(graph, member);
        fingerprints
            .member(&package_id)
            .expect("member fingerprint should exist")
    }

    fn status<'a>(statuses: &'a [BuildStatus], member: &str) -> &'a BuildStatus {
        if let Some(status) = statuses.iter().find(|status| status.member == member) {
            return status;
        }
        let mut matching = statuses
            .iter()
            .filter(|status| status.member_name == member);
        let first = matching.next().expect("status should exist");
        assert!(
            matching.next().is_none(),
            "status lookup by display name `{member}` is ambiguous"
        );
        first
    }

    #[test]
    fn parse_build_target_accepts_native_targets() {
        assert_eq!(
            parse_build_target("internal-aot").expect("target"),
            BuildTarget::InternalAot
        );
        assert_eq!(
            parse_build_target("windows-exe").expect("target"),
            BuildTarget::WindowsExe
        );
        assert_eq!(
            parse_build_target("windows-dll").expect("target"),
            BuildTarget::WindowsDll
        );
        assert_eq!(
            BuildTarget::WindowsExe
                .artifact_file_name(&GrimoireKind::App)
                .expect("app exe target"),
            "app.exe"
        );
        assert_eq!(
            BuildTarget::WindowsDll
                .artifact_file_name(&GrimoireKind::Lib)
                .expect("lib dll target"),
            "lib.dll"
        );
    }

    #[cfg(windows)]
    #[test]
    fn execute_build_emits_windows_exe_bundle() {
        let dir = temp_dir("pending_windows_exe");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect("windows exe plan should build");
        assert_eq!(statuses[0].target(), &BuildTarget::windows_exe());
        execute_build_with_context(
            &graph,
            &prepared,
            &statuses,
            &BuildExecutionContext::default(),
        )
        .expect("windows exe build should succeed");
        let artifact_path = graph.root_dir.join(statuses[0].artifact_rel_path());
        let metadata_path = crate::build_identity::cache_metadata_path_for_output(
            &artifact_path,
            &BuildTarget::windows_exe(),
        );
        assert!(
            artifact_path.is_file(),
            "expected emitted exe at {}",
            artifact_path.display()
        );
        assert!(
            metadata_path.is_file(),
            "expected cache metadata at {}",
            metadata_path.display()
        );
        let metadata = crate::build_identity::read_cached_output_metadata(
            &artifact_path,
            &BuildTarget::windows_exe(),
        )
        .expect("native exe cache metadata should read");
        let launch_path = artifact_path.with_file_name("app.exe.arcana-launch.toml");
        let embedded_artifact_path = artifact_path.with_file_name("app.exe.artifact.toml");
        let exe_bytes = fs::read(&artifact_path).expect("emitted exe should read");
        assert!(
            !exe_bytes.is_empty(),
            "expected non-empty emitted exe at {}",
            artifact_path.display()
        );
        assert_eq!(metadata.target_format, AOT_WINDOWS_EXE_FORMAT);
        assert_eq!(
            metadata.support_files,
            vec!["app.exe.arcana-bundle.toml".to_string()]
        );
        assert!(
            !launch_path.exists(),
            "did not expect legacy launch metadata at {}",
            launch_path.display()
        );
        assert!(
            !embedded_artifact_path.exists(),
            "did not expect legacy embedded artifact at {}",
            embedded_artifact_path.display()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn stage_distribution_bundle_exports_windows_exe_output() {
        let dir = temp_dir("dist_windows_exe");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect("windows exe plan should build");
        execute_build_with_context(
            &graph,
            &prepared,
            &statuses,
            &BuildExecutionContext::default(),
        )
        .expect("windows exe build should succeed");

        let bundle_dir = default_distribution_dir(&graph, "app", &BuildTarget::windows_exe());
        let bundle = stage_distribution_bundle(
            &graph,
            &statuses,
            "app",
            &BuildTarget::windows_exe(),
            &bundle_dir,
        )
        .expect("distribution staging should succeed");
        assert_eq!(bundle.root_artifact, "app.exe");
        assert!(bundle.support_files.is_empty());
        let manifest_text = &bundle.manifest_text;
        assert!(manifest_text.contains("format = \"arcana-distribution-bundle-v2\""));
        assert!(bundle.bundle_dir.join("app.exe").is_file());
        assert!(
            !bundle.bundle_dir.join("arcana.bundle.toml").exists(),
            "did not expect staged distribution manifest beside exe"
        );
        assert!(
            !bundle
                .bundle_dir
                .join("app.exe.arcana-bundle.toml")
                .exists(),
            "did not expect staged native manifest beside exe"
        );
        assert!(
            !bundle.bundle_dir.join("app.exe.artifact.toml").exists(),
            "did not expect legacy embedded artifact in staged bundle"
        );
        assert!(
            !bundle
                .bundle_dir
                .join("app.exe.arcana-launch.toml")
                .exists(),
            "did not expect legacy launch manifest in staged bundle"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn stage_distribution_bundle_records_native_products_and_child_bindings() {
        let dir = temp_dir("dist_windows_exe_native_products");
        write_file(
            &dir.join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ndesktop = { path = \"desktop\", native_child = \"default\", native_plugins = [\"tools\"] }\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        write_file(
            &dir.join("desktop").join("book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n",
                "\n[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"child\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"desktop_child.dll\"\n",
                "contract = \"arcana.cabi.child.v1\"\n",
                "sidecars = []\n",
                "\n[native.products.tools]\n",
                "kind = \"dll\"\n",
                "role = \"plugin\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"desktop_tools.dll\"\n",
                "contract = \"arcana.cabi.plugin.v1\"\n",
                "sidecars = []\n",
            ),
        );
        write_file(
            &dir.join("desktop").join("src").join("book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(
            &dir.join("desktop").join("src").join("types.arc"),
            "// types\n",
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect("windows exe plan should build");
        execute_build_with_context(
            &graph,
            &prepared,
            &statuses,
            &BuildExecutionContext::default(),
        )
        .expect("windows exe build should succeed");

        let bundle_dir = default_distribution_dir(&graph, "app", &BuildTarget::windows_exe());
        let bundle = stage_distribution_bundle(
            &graph,
            &statuses,
            "app",
            &BuildTarget::windows_exe(),
            &bundle_dir,
        )
        .expect("distribution staging should succeed");
        let manifest_text = &bundle.manifest_text;

        assert!(bundle.bundle_dir.join("desktop_child.dll").is_file());
        assert!(bundle.bundle_dir.join("desktop_tools.dll").is_file());
        assert!(manifest_text.contains("[[native_products]]"));
        assert!(manifest_text.contains("package_name = \"desktop\""));
        assert!(manifest_text.contains("product_name = \"default\""));
        assert!(manifest_text.contains("role = \"child\""));
        assert!(manifest_text.contains("contract_id = \"arcana.cabi.child.v1\""));
        assert!(manifest_text.contains("contract_version = 1"));
        assert!(manifest_text.contains("producer = \"arcana-source\""));
        assert!(manifest_text.contains("sidecars = []"));
        assert!(manifest_text.contains("file_hash = \"sha256:"));
        assert!(manifest_text.contains("file = \"desktop_child.dll\""));
        assert!(manifest_text.contains("product_name = \"tools\""));
        assert!(manifest_text.contains("role = \"plugin\""));
        assert!(manifest_text.contains("contract_id = \"arcana.cabi.plugin.v1\""));
        assert!(manifest_text.contains("file = \"desktop_tools.dll\""));
        assert!(manifest_text.contains("native_product_closure = \"sha256:"));
        assert!(manifest_text.contains("[runtime_child_binding]"));
        assert!(manifest_text.contains("consumer_member = \"app\""));
        assert!(manifest_text.contains("dependency_alias = \"desktop\""));
        assert!(manifest_text.contains("package_name = \"desktop\""));
        assert!(manifest_text.contains("product_name = \"default\""));
        assert!(manifest_text.contains("[[child_bindings]]"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn windows_exe_build_rejects_ambiguous_root_native_child_runtime_providers() {
        let dir = temp_dir("ambiguous_root_native_child_runtime_providers");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"app\"\n",
                "kind = \"app\"\n",
                "[deps]\n",
                "desktop = { path = \"desktop\", native_child = \"default\" }\n",
                "input = { path = \"input\", native_child = \"default\" }\n",
            ),
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        for (member, file_name) in [
            ("desktop", "desktop_child.dll"),
            ("input", "input_child.dll"),
        ] {
            write_file(
                &dir.join(member).join("book.toml"),
                &format!(
                    concat!(
                        "name = \"{member}\"\n",
                        "kind = \"lib\"\n",
                        "\n[native.products.default]\n",
                        "kind = \"dll\"\n",
                        "role = \"child\"\n",
                        "producer = \"arcana-source\"\n",
                        "file = \"{file_name}\"\n",
                        "contract = \"arcana.cabi.child.v1\"\n",
                        "sidecars = []\n",
                    ),
                    member = member,
                    file_name = file_name
                ),
            );
            write_file(
                &dir.join(member).join("src").join("book.arc"),
                "export fn ready() -> Int:\n    return 1\n",
            );
            write_file(
                &dir.join(member).join("src").join("types.arc"),
                "// types\n",
            );
        }

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let err = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect_err("ambiguous root runtime child bindings should fail planning");
        assert!(
            err.contains("multiple native child runtime providers"),
            "{err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn stage_distribution_bundle_removes_stale_files_before_copying() {
        let dir = temp_dir("dist_windows_exe_clean");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect("windows exe plan should build");
        execute_build_with_context(
            &graph,
            &prepared,
            &statuses,
            &BuildExecutionContext::default(),
        )
        .expect("windows exe build should succeed");

        let bundle_dir = default_distribution_dir(&graph, "app", &BuildTarget::windows_exe());
        stage_distribution_bundle(
            &graph,
            &statuses,
            "app",
            &BuildTarget::windows_exe(),
            &bundle_dir,
        )
        .expect("first distribution staging should succeed");
        fs::write(bundle_dir.join("stale.txt"), "stale").expect("stale file should write");
        fs::create_dir_all(bundle_dir.join("stale-dir")).expect("stale dir should write");
        fs::write(bundle_dir.join("stale-dir").join("nested.txt"), "stale")
            .expect("nested stale file should write");

        let bundle = stage_distribution_bundle(
            &graph,
            &statuses,
            "app",
            &BuildTarget::windows_exe(),
            &bundle_dir,
        )
        .expect("second distribution staging should succeed");
        assert!(
            !bundle.bundle_dir.join("stale.txt").exists(),
            "expected stale file to be removed before staging"
        );
        assert!(
            !bundle.bundle_dir.join("stale-dir").exists(),
            "expected stale directory to be removed before staging"
        );
        assert!(bundle.bundle_dir.join("app.exe").is_file());
        assert!(
            !bundle.bundle_dir.join("arcana.bundle.toml").exists(),
            "did not expect staged distribution manifest beside exe after cleanup"
        );
        assert!(
            !bundle
                .bundle_dir
                .join("app.exe.arcana-bundle.toml")
                .exists(),
            "did not expect staged native manifest beside exe after cleanup"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn stage_distribution_bundle_rejects_unmanaged_non_empty_output_dir() {
        let dir = temp_dir("dist_windows_exe_unmanaged");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect("windows exe plan should build");
        execute_build_with_context(
            &graph,
            &prepared,
            &statuses,
            &BuildExecutionContext::default(),
        )
        .expect("windows exe build should succeed");

        let bundle_dir = default_distribution_dir(&graph, "app", &BuildTarget::windows_exe());
        fs::create_dir_all(&bundle_dir).expect("bundle dir should exist");
        fs::write(bundle_dir.join("user.txt"), "keep").expect("user file should write");

        let err = stage_distribution_bundle(
            &graph,
            &statuses,
            "app",
            &BuildTarget::windows_exe(),
            &bundle_dir,
        )
        .expect_err("unmanaged output directory should be rejected");
        assert!(
            err.contains("refusing to overwrite non-empty unmanaged distribution directory"),
            "{err}"
        );
        assert!(
            bundle_dir.join("user.txt").is_file(),
            "expected unmanaged directory contents to be preserved"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn second_windows_exe_build_is_cache_hit() {
        let dir = temp_dir("windows_exe_cache_hit");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let first_statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect("first windows exe plan should build");
        execute_build_with_context(
            &graph,
            &prepared,
            &first_statuses,
            &BuildExecutionContext::default(),
        )
        .expect("first windows exe build should succeed");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lockfile")
            .expect("lockfile should exist");

        let second_statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            Some(&existing),
            BuildTarget::windows_exe(),
            &BuildExecutionContext::default(),
        )
        .expect("second windows exe plan should succeed");
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::CacheHit)]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn execute_build_emits_windows_dll_bundle_with_typed_header() {
        let dir = temp_dir("pending_windows_dll");
        write_file(&dir.join("book.toml"), "name = \"core\"\nkind = \"lib\"\n");
        write_file(
            &dir.join("src").join("book.arc"),
            "export fn answer() -> Int:\n    return 11\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let statuses = plan_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_dll(),
            &BuildExecutionContext::default(),
        )
        .expect("windows dll plan should build");
        execute_build_with_context(
            &graph,
            &prepared,
            &statuses,
            &BuildExecutionContext::default(),
        )
        .expect("windows dll build should succeed");

        let artifact_path = graph.root_dir.join(statuses[0].artifact_rel_path());
        let metadata = crate::build_identity::read_cached_output_metadata(
            &artifact_path,
            &BuildTarget::windows_dll(),
        )
        .expect("native dll cache metadata should read");
        assert!(artifact_path.is_file());
        assert_eq!(metadata.target_format, AOT_WINDOWS_DLL_FORMAT);
        assert_eq!(
            metadata.support_files,
            vec![
                "lib.dll.h".to_string(),
                "lib.dll.def".to_string(),
                "lib.dll.arcana-bundle.toml".to_string()
            ]
        );
        let header_text = fs::read_to_string(artifact_path.with_file_name("lib.dll.h"))
            .expect("typed dll header should read");
        assert!(header_text.contains("uint8_t answer(int64_t* out_result);"));
        assert!(header_text.contains("typedef struct ArcanaCabiProductApiV1"));
        assert!(header_text.contains("typedef struct ArcanaCabiInstanceOpsV1"));
        assert!(header_text.contains("arcana_cabi_owned_str_free_v1"));
        let def_text = fs::read_to_string(artifact_path.with_file_name("lib.dll.def"))
            .expect("dll definition file should read");
        assert!(def_text.contains("EXPORTS"));
        assert!(def_text.contains("answer"));
        let native_manifest =
            fs::read_to_string(artifact_path.with_file_name("lib.dll.arcana-bundle.toml"))
                .expect("native dll manifest should read");
        assert!(native_manifest.contains("format = \"arcana-native-manifest-v3\""));
        assert!(native_manifest.contains("kind = \"dynamic-library\""));
        assert!(
            native_manifest.contains("owned_str_free_symbol = \"arcana_cabi_owned_str_free_v1\"")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn package_target_planning_builds_only_selected_lib_closure_for_windows_dll() {
        let dir = temp_dir("package_windows_dll_closure");
        write_file(
            &dir.join("book.toml"),
            "name = \"workspace\"\nkind = \"lib\"\n\n[workspace]\nmembers = [\"util\", \"core\", \"app\"]\n",
        );
        write_file(&dir.join("src/book.arc"), "// workspace root\n");
        write_file(&dir.join("src/types.arc"), "// workspace types\n");

        write_grimoire(&dir.join("util"), GrimoireKind::Lib, "util", &[]);
        write_file(
            &dir.join("util/src/book.arc"),
            "export fn answer() -> Int:\n    return 7\n",
        );

        write_grimoire(
            &dir.join("core"),
            GrimoireKind::Lib,
            "core",
            &[("util", "../util")],
        );
        write_file(
            &dir.join("core/src/book.arc"),
            concat!(
                "import util\n",
                "export fn answer() -> Int:\n",
                "    return util.answer :: :: call\n",
            ),
        );

        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("core", "../core")],
        );
        write_file(
            &dir.join("app/src/shelf.arc"),
            concat!(
                "import core\n",
                "fn main() -> Int:\n",
                "    return core.answer :: :: call\n",
            ),
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let prepared = prepare_test_build(&graph);
        let statuses = plan_package_build_for_target_with_context(
            &graph,
            &order,
            &prepared,
            None,
            BuildTarget::windows_dll(),
            "core",
            &build::BuildExecutionContext::default(),
        )
        .expect("package target plan should succeed");
        assert_eq!(
            statuses
                .iter()
                .map(|status| (status.member_name().to_string(), status.target.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("util".to_string(), BuildTarget::internal_aot()),
                ("core".to_string(), BuildTarget::windows_dll()),
            ]
        );

        execute_build_with_context(
            &graph,
            &prepared,
            &statuses,
            &build::BuildExecutionContext::default(),
        )
        .expect("selected lib closure should build");

        let lock_path = write_lockfile(&graph, &order, &statuses).expect("lockfile");
        let lock = read_lockfile(&lock_path)
            .expect("read lockfile")
            .expect("lockfile should exist");
        assert!(
            lock_member(&lock, Some(&graph), "app").targets.is_empty(),
            "unbuilt app member should keep an empty target set"
        );
        assert!(
            lock_member(&lock, Some(&graph), "workspace")
                .targets
                .is_empty(),
            "unbuilt root workspace member should keep an empty target set"
        );
        assert!(
            lock_member(&lock, Some(&graph), "util")
                .target(&BuildTarget::internal_aot())
                .is_some()
        );
        assert!(
            lock_member(&lock, Some(&graph), "core")
                .target(&BuildTarget::windows_dll())
                .is_some()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    fn disposition_map(statuses: &[BuildStatus]) -> BTreeMap<String, BuildDisposition> {
        statuses
            .iter()
            .map(|status| (status.member_name.clone(), status.disposition))
            .collect()
    }

    fn assert_dispositions(statuses: &[BuildStatus], expected: &[(&str, BuildDisposition)]) {
        assert_eq!(
            statuses.len(),
            expected.len(),
            "expected {} statuses but saw {}: {:?}",
            expected.len(),
            statuses.len(),
            disposition_map(statuses)
        );
        for (member, disposition) in expected {
            assert_eq!(status(statuses, member).disposition(), *disposition);
        }
    }

    fn execute_planned_build(
        graph: &WorkspaceGraph,
        prepared: &PreparedBuild,
        statuses: &[BuildStatus],
    ) -> PackageResult<PathBuf> {
        execute_build(graph, prepared, statuses)
    }

    #[test]
    fn parse_manifest_recognizes_but_gates_git_deps() {
        let dir = temp_dir("manifest_git_dep");
        write_file(
            &dir.join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ncore = { git = \"https://example.com/repo\" }\n",
        );
        let manifest = parse_manifest(&dir.join("book.toml")).expect("manifest should parse");
        assert!(matches!(
            manifest
                .deps
                .get("core")
                .expect("git dep should exist")
                .source,
            DependencySourceSpec::Git { .. }
        ));
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");
        let err = load_workspace_graph(&dir).expect_err("expected git gating");
        assert!(err.contains("recognized but not enabled yet"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_manifest_recognizes_but_gates_nonlocal_registry_deps() {
        let dir = temp_dir("manifest_remote_registry_dep");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"app\"\n",
                "kind = \"app\"\n",
                "[deps]\n",
                "core = { version = \"^1.2.3\", registry = \"central\" }\n",
            ),
        );
        let manifest = parse_manifest(&dir.join("book.toml")).expect("manifest should parse");
        assert!(matches!(
            manifest
                .deps
                .get("core")
                .expect("registry dep should exist")
                .source,
            DependencySourceSpec::Registry { .. }
        ));
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");
        let err = load_workspace_graph(&dir).expect_err("expected registry gating");
        assert!(err.contains("only `registry = \"local\"` is enabled in this phase"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_manifest_reads_foreword_adapter_products() {
        let dir = temp_dir("manifest_foreword_products");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"tool\"\n",
                "kind = \"lib\"\n",
                "[toolchain.foreword_products.adapter]\n",
                "path = \"forewords/adapter.cmd\"\n",
                "runner = \"cmd\"\n",
                "args = [\"/c\"]\n",
            ),
        );

        let manifest = parse_manifest(&dir.join("book.toml")).expect("manifest should parse");
        let product = manifest
            .foreword_products
            .get("adapter")
            .expect("foreword product should exist");
        assert_eq!(product.path, "forewords/adapter.cmd");
        assert_eq!(product.runner.as_deref(), Some("cmd"));
        assert_eq!(product.args, vec!["/c"]);

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
            vec!["workspace", "app", "core"]
        );
        let root = graph
            .member("workspace")
            .expect("root package should be in workspace graph");
        assert_eq!(root.rel_dir, ".");
        assert_eq!(root.deps, vec![member_id(&graph, "core")]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_workspace_graph_preserves_dependency_aliases() {
        let dir = temp_dir("graph_dep_alias");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_file(
            &dir.join("app/book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\nutil = { path = \"../core\" }\n",
        );
        write_file(
            &dir.join("app/src/shelf.arc"),
            "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
        );
        write_file(&dir.join("app/src/types.arc"), "// app types\n");
        write_file(
            &dir.join("core/book.toml"),
            "name = \"core\"\nkind = \"lib\"\n",
        );
        write_file(
            &dir.join("core/src/book.arc"),
            "export fn value() -> Int:\n    return 7\n",
        );
        write_file(&dir.join("core/src/types.arc"), "// core types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let app = graph.member("app").expect("app should exist");
        assert_eq!(app.deps, vec![member_id(&graph, "core")]);
        assert_eq!(
            app.direct_dep_packages.get("util"),
            Some(&"core".to_string())
        );
        assert_eq!(
            app.direct_dep_ids.get("util"),
            Some(&member_id(&graph, "core"))
        );
        assert_eq!(
            app.direct_dep_specs
                .get("util")
                .expect("dependency spec should exist")
                .native_delivery,
            NativeDependencyDelivery::Baked
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_workspace_graph_preserves_native_delivery_metadata() {
        let dir = temp_dir("graph_native_delivery");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_file(
            &dir.join("app/book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ndesktop = { path = \"../desktop\", native_delivery = \"dll\" }\n",
        );
        write_file(
            &dir.join("app/src/shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("app/src/types.arc"), "// app types\n");
        write_file(
            &dir.join("desktop/book.toml"),
            "name = \"arcana_desktop\"\nkind = \"lib\"\n",
        );
        write_file(
            &dir.join("desktop/src/book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(&dir.join("desktop/src/types.arc"), "// desktop types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let app = graph.member("app").expect("app should exist");
        let spec = app
            .direct_dep_specs
            .get("desktop")
            .expect("desktop dependency spec should exist");
        assert_eq!(spec.native_delivery, NativeDependencyDelivery::Dll);
        assert_eq!(
            app.direct_dep_ids.get("desktop"),
            Some(&member_id(&graph, "arcana_desktop"))
        );
        assert_eq!(
            app.direct_dep_packages.get("desktop"),
            Some(&"arcana_desktop".to_string())
        );

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
        assert_eq!(
            workspace
                .package("workspace")
                .expect("root package should exist")
                .direct_dep_ids
                .get("std")
                .map(String::as_str),
            Some("path:std")
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
            order_display_names(&graph, &first),
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
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute build");
        let first = render_lockfile(&graph, &order, &statuses, None).expect("render");
        let second = render_lockfile(&graph, &order, &statuses, None).expect("render");
        assert_eq!(first, second);
        assert!(first.contains("version = 4"));
        assert!(first.contains("workspace_root = \"path:.\""));
        assert!(first.contains("[builds]"));
        assert!(first.contains("internal-aot"));
        assert!(first.contains("[builds.\"path:app\".\"internal-aot\"]"));
        assert!(first.contains("artifact_hash"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_lockfile_uses_v4_package_sections() {
        let dir = temp_dir("lock_v4");
        write_file(
            &dir.join("book.toml"),
            "name = \"app\"\nkind = \"app\"\n[workspace]\nmembers = [\"core\"]\n[deps]\ncore = { path = \"core\" }\n",
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");
        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute build");
        let rendered = render_lockfile(&graph, &order, &statuses, None).expect("render");
        assert!(rendered.contains("version = 4"));
        assert!(rendered.contains("workspace_root = \"path:.\""));
        assert!(rendered.contains("[packages.\"path:.\"]"));
        assert!(rendered.contains("[dependencies.\"path:.\"]"));
        assert!(rendered.contains("[builds.\"path:.\".\"internal-aot\"]"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_lockfile_infers_internal_aot_target() {
        let dir = temp_dir("legacy_lock_target");
        let lock_path = dir.join("Arcana.lock");
        write_file(
            &lock_path,
            &format!(
                concat!(
                    "version = 1\n",
                    "workspace = \"ws\"\n",
                    "toolchain = \"binary-sha256:abc\"\n",
                    "order = [\"app\"]\n\n",
                    "[paths]\n",
                    "\"app\" = \"app\"\n\n",
                    "[deps]\n",
                    "\"app\" = []\n\n",
                    "[kinds]\n",
                    "\"app\" = \"app\"\n\n",
                    "[formats]\n",
                    "\"app\" = \"{}\"\n\n",
                    "[fingerprints]\n",
                    "\"app\" = \"fp\"\n\n",
                    "[api_fingerprints]\n",
                    "\"app\" = \"api\"\n\n",
                    "[artifacts]\n",
                    "\"app\" = \".arcana/artifacts/app/internal-aot/fp/app.artifact.toml\"\n\n",
                    "[artifact_hashes]\n",
                    "\"app\" = \"sha256:deadbeef\"\n",
                ),
                AOT_INTERNAL_FORMAT
            ),
        );

        let lock = read_lockfile(&lock_path)
            .expect("lockfile should parse")
            .expect("lockfile should exist");
        let app = lock_member(&lock, None, "app");
        assert!(app.target(&BuildTarget::internal_aot()).is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_lockfile_skips_stale_internal_aot_target_format() {
        let dir = temp_dir("stale_lock_target");
        let lock_path = dir.join("Arcana.lock");
        write_file(
            &lock_path,
            concat!(
                "version = 1\n",
                "workspace = \"ws\"\n",
                "toolchain = \"binary-sha256:abc\"\n",
                "order = [\"app\"]\n\n",
                "[paths]\n",
                "\"app\" = \"app\"\n\n",
                "[deps]\n",
                "\"app\" = []\n\n",
                "[kinds]\n",
                "\"app\" = \"app\"\n\n",
                "[formats]\n",
                "\"app\" = \"arcana-aot-v6\"\n\n",
                "[fingerprints]\n",
                "\"app\" = \"fp\"\n\n",
                "[api_fingerprints]\n",
                "\"app\" = \"api\"\n\n",
                "[artifacts]\n",
                "\"app\" = \".arcana/artifacts/app/internal-aot/fp/app.artifact.toml\"\n\n",
                "[artifact_hashes]\n",
                "\"app\" = \"sha256:deadbeef\"\n",
            ),
        );

        let lock = read_lockfile(&lock_path)
            .expect("stale lockfile should parse")
            .expect("lockfile should exist");
        let app = lock_member(&lock, None, "app");
        assert!(app.target(&BuildTarget::internal_aot()).is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_lockfile_preserves_git_package_metadata() {
        let dir = temp_dir("git_lockfile_metadata");
        let lock_path = dir.join("Arcana.lock");
        let git_id = "git:https://example.com/arcana/tooling.git#tag:v1.2.3:tool";
        write_file(
            &lock_path,
            &format!(
                concat!(
                    "version = 4\n",
                    "workspace = \"ws\"\n",
                    "workspace_root = \"path:.\"\n",
                    "order = [\"path:.\", \"{git_id}\"]\n",
                    "workspace_members = [\"path:.\"]\n\n",
                    "[packages.\"path:.\"]\n",
                    "name = \"app\"\n",
                    "kind = \"app\"\n",
                    "source_kind = \"path\"\n",
                    "path = \".\"\n\n",
                    "[packages.\"{git_id}\"]\n",
                    "name = \"tool\"\n",
                    "kind = \"lib\"\n",
                    "source_kind = \"git\"\n",
                    "git = \"https://example.com/arcana/tooling.git\"\n",
                    "git_selector = \"tag:v1.2.3\"\n\n",
                    "[dependencies.\"path:.\"]\n",
                    "tool = \"{git_id}\"\n\n",
                    "[builds.\"path:.\".\"internal-aot\"]\n",
                    "fingerprint = \"fp-app\"\n",
                    "api_fingerprint = \"api-app\"\n",
                    "artifact = \".arcana/artifacts/app/internal-aot/app.artifact.toml\"\n",
                    "artifact_hash = \"sha256:app\"\n",
                    "format = \"{format}\"\n",
                    "toolchain = \"toolchain-1\"\n\n",
                    "[builds.\"{git_id}\".\"internal-aot\"]\n",
                    "fingerprint = \"fp-tool\"\n",
                    "api_fingerprint = \"api-tool\"\n",
                    "artifact = \".arcana/artifacts/tool/internal-aot/lib.artifact.toml\"\n",
                    "artifact_hash = \"sha256:tool\"\n",
                    "format = \"{format}\"\n",
                    "toolchain = \"toolchain-1\"\n",
                ),
                git_id = git_id,
                format = AOT_INTERNAL_FORMAT,
            ),
        );

        let lock = read_lockfile(&lock_path)
            .expect("lockfile should parse")
            .expect("lockfile should exist");
        let app = lock_member(&lock, None, "path:.");
        assert_eq!(app.dep_bindings.get("tool"), Some(&git_id.to_string()));
        let tool = lock.members.get(git_id).expect("git package should exist");
        assert_eq!(tool.name, "tool");
        assert_eq!(tool.source_kind, DependencySource::Git);
        assert_eq!(
            tool.git_url.as_deref(),
            Some("https://example.com/arcana/tooling.git")
        );
        assert_eq!(tool.git_selector.as_deref(), Some("tag:v1.2.3"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_lockfile_infers_legacy_native_exe_target() {
        let dir = temp_dir("legacy_lock_native_exe");
        let lock_path = dir.join("Arcana.lock");
        write_file(
            &lock_path,
            concat!(
                "version = 1\n",
                "workspace = \"ws\"\n",
                "toolchain = \"binary-sha256:abc\"\n",
                "order = [\"app\"]\n\n",
                "[paths]\n",
                "\"app\" = \"app\"\n\n",
                "[deps]\n",
                "\"app\" = []\n\n",
                "[kinds]\n",
                "\"app\" = \"app\"\n\n",
                "[formats]\n",
                "\"app\" = \"arcana-native-exe-v1\"\n\n",
                "[fingerprints]\n",
                "\"app\" = \"fp\"\n\n",
                "[api_fingerprints]\n",
                "\"app\" = \"api\"\n\n",
                "[artifacts]\n",
                "\"app\" = \".arcana/artifacts/app/windows-exe/fp/app.exe\"\n\n",
                "[artifact_hashes]\n",
                "\"app\" = \"sha256:deadbeef\"\n",
            ),
        );

        let lock = read_lockfile(&lock_path)
            .expect("legacy lockfile should parse")
            .expect("lockfile should exist");
        let app = lock_member(&lock, None, "app");
        assert!(app.target(&BuildTarget::windows_exe()).is_some());

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
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::CacheHit)]);
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
        assert_eq!(
            order_display_names(&graph, &order),
            vec!["workspace".to_string(), "app".to_string()]
        );

        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &statuses).expect("write lockfile");
        let lock = read_lockfile(&lock_path)
            .expect("read lockfile")
            .expect("lockfile should exist");
        let root = lock_member(&lock, Some(&graph), "workspace");
        assert_eq!(root.path.as_deref(), Some("."));
        let target = root
            .target(&BuildTarget::internal_aot())
            .expect("root package should include the internal-aot artifact");
        assert!(!target.artifact_hash.is_empty());
        assert!(dir.join(&target.artifact).is_file());

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
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let status = status(&first_statuses, "app");
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

        let (second_prepared, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::Built)]);

        execute_planned_build(&graph, &second_prepared, &second_statuses)
            .expect("rebuild should refresh artifact");
        let refreshed = fs::read_to_string(&artifact_path).expect("artifact should exist");
        let parsed = parse_package_artifact(&refreshed).expect("artifact should parse");
        assert_eq!(parsed.format, AOT_INTERNAL_FORMAT);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_cached_artifact_identity_triggers_rebuild() {
        let dir = temp_dir("invalid_artifact_identity");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let status = status(&first_statuses, "app");
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        let stale = fs::read_to_string(&artifact_path).expect("artifact should exist");
        fs::write(
            &artifact_path,
            stale.replace("package_name = \"app\"", "package_name = \"wrong\""),
        )
        .expect("artifact should be rewritten");

        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::Built)]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_cached_artifact_payload_triggers_rebuild() {
        let dir = temp_dir("invalid_artifact_payload");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let status = status(&first_statuses, "app");
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        let stale = fs::read_to_string(&artifact_path).expect("artifact should exist");
        fs::write(&artifact_path, stale.replace("Int = 0", "Int = 99"))
            .expect("artifact should be rewritten");

        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::Built)]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_cached_artifact_rows_trigger_rebuild_even_with_matching_hashes() {
        let dir = temp_dir("invalid_artifact_rows");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "import std.io\nfn main() -> Int:\n    std.io.print :: 1 :: call\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");
        write_std_io_grimoire(&dir);

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");

        let status = status(&first_statuses, "app");
        let artifact_path = graph.root_dir.join(&status.artifact_rel_path);
        let stale = fs::read_to_string(&artifact_path).expect("artifact should exist");
        let malformed = stale.replace("module=app:import:std.io:", "module=app:import::");
        fs::write(&artifact_path, malformed).expect("artifact should be rewritten");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::Built)]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lockfile_toolchain_mismatch_triggers_rebuild() {
        let dir = temp_dir("toolchain_mismatch");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");
        let stale_lock = fs::read_to_string(&lock_path).expect("lockfile should exist");
        fs::write(
            &lock_path,
            stale_lock.replace(
                &format!(
                    "toolchain = \"{}\"",
                    crate::build_identity::current_build_toolchain()
                        .expect("toolchain id should compute")
                ),
                "toolchain = \"arcana-cli definitely-different\"",
            ),
        )
        .expect("lockfile should be rewritten");
        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");

        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::Built)]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_lockfile_preserves_existing_foreign_target_entries() {
        let dir = temp_dir("lockfile_target_preservation");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src").join("shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src").join("types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute build");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lockfile");

        let stale_lock = fs::read_to_string(&lock_path).expect("lockfile should exist");
        fs::write(
            &lock_path,
            format!(
                "{stale_lock}\n[builds.\"path:.\".\"future-exe\"]\n\
fingerprint = \"future-fp\"\n\
api_fingerprint = \"future-api\"\n\
artifact = \".arcana/artifacts/app/future-exe/future-fp/app.exe\"\n\
artifact_hash = \"sha256:future\"\n\
format = \"arcana-native-exe-v1\"\n\
toolchain = \"future-toolchain\"\n"
            ),
        )
        .expect("lockfile should be rewritten");

        let existing = read_lockfile(&lock_path)
            .expect("read lock")
            .expect("lock exists");
        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::CacheHit)]);
        write_lockfile(&graph, &order, &second_statuses).expect("lockfile");

        let rendered = fs::read_to_string(&lock_path).expect("lockfile should exist");
        assert!(rendered.contains("[builds.\"path:.\".\"future-exe\"]"));
        assert!(rendered.contains(".arcana/artifacts/app/future-exe/future-fp/app.exe"));

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
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute");

        let core = status(&statuses, "core");
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
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute");

        let app = status(&statuses, "app");
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
    fn built_artifact_runtime_requirements_follow_reachable_intrinsics() {
        let dir = temp_dir("artifact_runtime_requirements");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src/shelf.arc"),
            concat!(
                "import std.text\n",
                "fn main() -> Int:\n",
                "    return std.text.len_bytes :: \"hi\" :: call\n",
            ),
        );
        write_file(&dir.join("src/types.arc"), "// app types\n");
        write_file(
            &dir.join("std/book.toml"),
            "name = \"std\"\nkind = \"lib\"\n",
        );
        write_file(&dir.join("std/src/book.arc"), "// std root\n");
        write_file(&dir.join("std/src/types.arc"), "// std types\n");
        write_file(
            &dir.join("std/src/text.arc"),
            concat!(
                "import std.kernel.text\n",
                "export fn len_bytes(read text: Str) -> Int:\n",
                "    return std.kernel.text.text_len_bytes :: text :: call\n",
            ),
        );
        write_file(
            &dir.join("std/src/io.arc"),
            concat!(
                "import std.kernel.io\n",
                "export fn print[T](read value: T):\n",
                "    std.kernel.io.print[T] :: value :: call\n",
            ),
        );
        write_file(
            &dir.join("std/src/kernel/text.arc"),
            "intrinsic fn text_len_bytes(text: Str) -> Int = HostTextLenBytes\n",
        );
        write_file(
            &dir.join("std/src/kernel/io.arc"),
            "intrinsic fn print[T](read value: T) = IoPrint\n",
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute");

        let app = status(&statuses, "app");
        let artifact = fs::read_to_string(graph.root_dir.join(&app.artifact_rel_path))
            .expect("artifact should exist");
        let parsed = parse_package_artifact(&artifact).expect("artifact should parse");
        assert_eq!(
            parsed.runtime_requirements,
            vec!["std.kernel.text".to_string()]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn built_artifact_canonicalizes_dependency_alias_metadata() {
        let dir = temp_dir("artifact_dependency_alias_metadata");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"core\"]\n",
        );
        write_grimoire(
            &dir.join("app"),
            GrimoireKind::App,
            "app",
            &[("util", "../core")],
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("app/src/shelf.arc"),
            "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
        );
        write_file(
            &dir.join("core/src/book.arc"),
            "export fn value() -> Int:\n    return 7\n",
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute");

        let app = status(&statuses, "app");
        let artifact = fs::read_to_string(graph.root_dir.join(app.artifact_rel_path()))
            .expect("artifact should exist");
        let parsed = parse_package_artifact(&artifact).expect("artifact should parse");
        assert_eq!(parsed.direct_deps, vec!["core".to_string()]);
        assert!(
            parsed
                .dependency_rows
                .iter()
                .any(|row| row == "source=app:import:core:"),
            "expected canonical dependency rows in artifact: {artifact}"
        );
        assert!(
            !parsed
                .dependency_rows
                .iter()
                .any(|row| row.contains(":util") || row.contains(".util")),
            "expected alias names to stay out of artifact dependency metadata: {artifact}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn built_lib_artifact_runtime_requirements_follow_exported_surface() {
        let dir = temp_dir("lib_artifact_runtime_requirements");
        write_file(&dir.join("book.toml"), "name = \"core\"\nkind = \"lib\"\n");
        write_file(
            &dir.join("src/book.arc"),
            concat!(
                "import std.io\n",
                "export fn announce() -> Int:\n",
                "    std.io.print[Int] :: 7 :: call\n",
                "    return 7\n",
            ),
        );
        write_file(&dir.join("src/types.arc"), "// core types\n");
        write_grimoire(&dir.join("std"), GrimoireKind::Lib, "std", &[]);
        write_file(&dir.join("std/src/book.arc"), "// std root\n");
        write_file(&dir.join("std/src/types.arc"), "// std types\n");
        write_file(
            &dir.join("std/src/io.arc"),
            concat!(
                "import std.kernel.io\n",
                "export fn print[T](read value: T):\n",
                "    std.kernel.io.print[T] :: value :: call\n",
            ),
        );
        write_file(
            &dir.join("std/src/audio.arc"),
            concat!(
                "import std.kernel.audio\n",
                "export fn default_output() -> Int:\n",
                "    return std.kernel.audio.default_output :: :: call\n",
            ),
        );
        write_file(
            &dir.join("std/src/kernel/io.arc"),
            "intrinsic fn print[T](read value: T) = IoPrint\n",
        );
        write_file(
            &dir.join("std/src/kernel/audio.arc"),
            "intrinsic fn default_output() -> Int = AudioDefaultOutputTry\n",
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute");

        let core = status(&statuses, "core");
        let artifact = fs::read_to_string(graph.root_dir.join(core.artifact_rel_path()))
            .expect("artifact should exist");
        let parsed = parse_package_artifact(&artifact).expect("artifact should parse");
        assert_eq!(
            parsed.runtime_requirements,
            vec!["std.kernel.io".to_string()]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn built_lib_artifact_runtime_requirements_follow_exported_impl_surface() {
        let dir = temp_dir("lib_artifact_impl_runtime_requirements");
        write_file(&dir.join("book.toml"), "name = \"core\"\nkind = \"lib\"\n");
        write_file(&dir.join("src/book.arc"), "reexport types\n");
        write_file(
            &dir.join("src/types.arc"),
            concat!(
                "import std.io\n",
                "export record Counter:\n",
                "    value: Int\n",
                "impl Counter:\n",
                "    fn announce(read self: Counter) -> Int:\n",
                "        std.io.print[Int] :: self.value :: call\n",
                "        return self.value\n",
            ),
        );
        write_std_io_grimoire(&dir);

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute");

        let core = status(&statuses, "core");
        let artifact = fs::read_to_string(graph.root_dir.join(core.artifact_rel_path()))
            .expect("artifact should exist");
        let parsed = parse_package_artifact(&artifact).expect("artifact should parse");
        let announce = parsed
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "announce")
            .expect("impl method should be present");

        assert!(announce.exported);
        assert_eq!(
            parsed.runtime_requirements,
            vec!["std.kernel.io".to_string()]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn built_lib_artifact_surface_rows_exclude_dependency_exports() {
        let dir = temp_dir("lib_artifact_surface_rows");
        write_file(&dir.join("book.toml"), "name = \"core\"\nkind = \"lib\"\n");
        write_file(
            &dir.join("src/book.arc"),
            concat!(
                "import std.io\n",
                "export fn announce() -> Int:\n",
                "    std.io.print[Int] :: 7 :: call\n",
                "    return 7\n",
            ),
        );
        write_file(&dir.join("src/types.arc"), "// core types\n");
        write_std_io_grimoire(&dir);

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("execute");

        let core = status(&statuses, "core");
        let artifact = fs::read_to_string(graph.root_dir.join(core.artifact_rel_path()))
            .expect("artifact should exist");
        let parsed = parse_package_artifact(&artifact).expect("artifact should parse");

        assert!(
            parsed
                .modules
                .iter()
                .any(|module| module.module_id == "std.io"),
            "expected linked dependency modules to remain in artifact: {artifact}"
        );
        assert!(
            parsed
                .exported_surface_rows
                .iter()
                .any(|row| row == "module=core:export:fn:fn announce() -> Int:"),
            "expected root package surface rows in artifact: {artifact}"
        );
        assert!(
            !parsed
                .exported_surface_rows
                .iter()
                .any(|row| row.starts_with("module=std")),
            "dependency package exports should not leak into root artifact surface rows: {artifact}"
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
        let err = prepare_build(&graph).expect_err("ambiguous concrete bare method should fail");
        assert!(
            err.contains("bare-method qualifier `tap` on `app.types.Counter` is ambiguous"),
            "{err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prepared_build_uses_workspace_snapshot() {
        let dir = temp_dir("prepared_snapshot");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src/shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src/types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let workspace = load_workspace_hir_from_graph(&dir, &graph).expect("workspace");
        let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
        let prepared =
            prepare_build_from_workspace(&graph, workspace, resolved_workspace).expect("prepare");

        write_file(
            &dir.join("src/shelf.arc"),
            "fn main() -> Int:\n    return 7\n",
        );

        let statuses = plan_build(&graph, &order, &prepared, None).expect("plan");
        execute_build(&graph, &prepared, &statuses).expect("execute");

        let artifact_path = graph.root_dir.join(&statuses[0].artifact_rel_path);
        let artifact = fs::read_to_string(&artifact_path).expect("artifact should exist");
        assert!(
            artifact.contains("Int = 0"),
            "expected prepared build to write the checked snapshot, got: {artifact}"
        );
        assert!(!artifact.contains("Int = 7"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn owner_body_changes_update_api_fingerprint() {
        fn member_api_fingerprint(dir: &Path, member: &str) -> String {
            let graph = load_workspace_graph(dir).expect("load graph");
            let workspace = load_workspace_hir_from_graph(dir, &graph).expect("workspace");
            let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
            let fingerprints =
                compute_workspace_fingerprints(&graph, &workspace, &resolved_workspace)
                    .expect("fingerprints");
            fingerprint_member(&fingerprints, &graph, member)
                .api()
                .to_string()
        }

        let dir = temp_dir("owner_api_fingerprint");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"core\"]\n",
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("core/src/book.arc"),
            concat!(
                "export obj Counter:\n",
                "    value: Int\n",
                "\n",
                "export create Session [Counter] scope-exit:\n",
                "    done: when Counter.value >= 1 hold [Counter]\n",
            ),
        );

        let first = member_api_fingerprint(&dir, "core");

        write_file(
            &dir.join("core/src/book.arc"),
            concat!(
                "export obj Counter:\n",
                "    value: Int\n",
                "\n",
                "export create Session [Counter] scope-exit:\n",
                "    done: when Counter.value >= 2\n",
            ),
        );

        let second = member_api_fingerprint(&dir, "core");

        assert_ne!(
            first, second,
            "owner exit/body changes should affect the public api fingerprint"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn owner_lifecycle_hook_changes_update_api_fingerprint() {
        fn member_api_fingerprint(dir: &Path, member: &str) -> String {
            let graph = load_workspace_graph(dir).expect("load graph");
            let workspace = load_workspace_hir_from_graph(dir, &graph).expect("workspace");
            let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
            let fingerprints =
                compute_workspace_fingerprints(&graph, &workspace, &resolved_workspace)
                    .expect("fingerprints");
            fingerprint_member(&fingerprints, &graph, member)
                .api()
                .to_string()
        }

        let dir = temp_dir("owner_lifecycle_api_fingerprint");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"core\"]\n",
        );
        write_grimoire(&dir.join("core"), GrimoireKind::Lib, "core", &[]);
        write_file(
            &dir.join("core/src/book.arc"),
            concat!(
                "export obj SessionCtx:\n",
                "    base: Int\n",
                "\n",
                "export obj Counter:\n",
                "    value: Int\n",
                "    fn init(edit self: Self, read ctx: SessionCtx):\n",
                "        self.value = ctx.base\n",
                "\n",
                "export create Session [Counter] scope-exit:\n",
                "    done: when Counter.value >= 1 hold [Counter]\n",
            ),
        );

        let first = member_api_fingerprint(&dir, "core");

        write_file(
            &dir.join("core/src/book.arc"),
            concat!(
                "export obj SessionCtx:\n",
                "    base: Int\n",
                "\n",
                "export obj Counter:\n",
                "    value: Int\n",
                "    fn resume(edit self: Self, read ctx: SessionCtx):\n",
                "        self.value = ctx.base\n",
                "\n",
                "export create Session [Counter] scope-exit:\n",
                "    done: when Counter.value >= 1 hold [Counter]\n",
            ),
        );

        let second = member_api_fingerprint(&dir, "core");

        assert_ne!(
            first, second,
            "owner lifecycle hook changes should affect the public api fingerprint"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dependency_native_delivery_changes_update_source_fingerprint() {
        fn member_source_fingerprint(dir: &Path, member: &str) -> String {
            let graph = load_workspace_graph(dir).expect("graph should load");
            let workspace = load_workspace_hir_from_graph(dir, &graph).expect("workspace");
            let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
            let fingerprints =
                compute_workspace_fingerprints(&graph, &workspace, &resolved_workspace)
                    .expect("fingerprints");
            fingerprint_member(&fingerprints, &graph, member)
                .source()
                .to_string()
        }

        let dir = temp_dir("dependency_native_delivery_fingerprint");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_file(
            &dir.join("app/book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ndesktop = { path = \"../desktop\" }\n",
        );
        write_file(
            &dir.join("app/src/shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("app/src/types.arc"), "// app types\n");
        write_file(
            &dir.join("desktop/book.toml"),
            "name = \"arcana_desktop\"\nkind = \"lib\"\n",
        );
        write_file(
            &dir.join("desktop/src/book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(&dir.join("desktop/src/types.arc"), "// desktop types\n");

        let first = member_source_fingerprint(&dir, "app");
        write_file(
            &dir.join("app/book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ndesktop = { path = \"../desktop\", native_delivery = \"dll\" }\n",
        );
        let second = member_source_fingerprint(&dir, "app");

        assert_ne!(
            first, second,
            "dependency native delivery changes should affect the source fingerprint"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dependency_native_selection_changes_update_source_fingerprint() {
        fn member_source_fingerprint(dir: &Path, member: &str) -> String {
            let graph = load_workspace_graph(dir).expect("graph should load");
            let workspace = load_workspace_hir_from_graph(dir, &graph).expect("workspace");
            let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
            let fingerprints =
                compute_workspace_fingerprints(&graph, &workspace, &resolved_workspace)
                    .expect("fingerprints");
            fingerprint_member(&fingerprints, &graph, member)
                .source()
                .to_string()
        }

        let dir = temp_dir("dependency_native_selection_fingerprint");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\"]\n",
        );
        write_file(
            &dir.join("app/book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ndesktop = { path = \"../desktop\" }\n",
        );
        write_file(
            &dir.join("app/src/shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("app/src/types.arc"), "// app types\n");
        write_file(
            &dir.join("desktop/book.toml"),
            concat!(
                "name = \"arcana_desktop\"\n",
                "kind = \"lib\"\n",
                "\n[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"child\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"arcwin.dll\"\n",
                "contract = \"arcana.cabi.child.v1\"\n",
                "\n[native.products.tools]\n",
                "kind = \"dll\"\n",
                "role = \"plugin\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"arcana_tools.dll\"\n",
                "contract = \"arcana.cabi.plugin.v1\"\n",
            ),
        );
        write_file(
            &dir.join("desktop/src/book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(&dir.join("desktop/src/types.arc"), "// desktop types\n");

        let first = member_source_fingerprint(&dir, "app");
        write_file(
            &dir.join("app/book.toml"),
            concat!(
                "name = \"app\"\n",
                "kind = \"app\"\n",
                "[deps]\n",
                "desktop = { path = \"../desktop\", native_child = \"default\", native_plugins = [\"tools\"] }\n",
            ),
        );
        let second = member_source_fingerprint(&dir, "app");

        assert_ne!(
            first, second,
            "dependency native child/plugin selection changes should affect the source fingerprint"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn native_product_manifest_changes_update_source_fingerprint() {
        fn member_source_fingerprint(dir: &Path, member: &str) -> String {
            let graph = load_workspace_graph(dir).expect("graph should load");
            let workspace = load_workspace_hir_from_graph(dir, &graph).expect("workspace");
            let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
            let fingerprints =
                compute_workspace_fingerprints(&graph, &workspace, &resolved_workspace)
                    .expect("fingerprints");
            fingerprint_member(&fingerprints, &graph, member)
                .source()
                .to_string()
        }

        let dir = temp_dir("native_product_manifest_fingerprint");
        write_file(
            &dir.join("book.toml"),
            "name = \"desktop\"\nkind = \"lib\"\n",
        );
        write_file(
            &dir.join("src/book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(&dir.join("src/types.arc"), "// desktop types\n");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n",
                "\n[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"child\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"arcwin.dll\"\n",
                "contract = \"arcana.cabi.child.v1\"\n",
            ),
        );

        let first = member_source_fingerprint(&dir, "desktop");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n",
                "\n[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"child\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"arcana_runtime_provider.dll\"\n",
                "contract = \"arcana.cabi.child.v1\"\n",
            ),
        );
        let second = member_source_fingerprint(&dir, "desktop");

        assert_ne!(
            first, second,
            "native product manifest changes should affect the source fingerprint"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn native_product_sidecar_content_changes_update_source_fingerprint() {
        fn member_source_fingerprint(dir: &Path, member: &str) -> String {
            let graph = load_workspace_graph(dir).expect("graph should load");
            let workspace = load_workspace_hir_from_graph(dir, &graph).expect("workspace");
            let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
            let fingerprints =
                compute_workspace_fingerprints(&graph, &workspace, &resolved_workspace)
                    .expect("fingerprints");
            fingerprint_member(&fingerprints, &graph, member)
                .source()
                .to_string()
        }

        let dir = temp_dir("native_product_sidecar_fingerprint");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n",
                "\n[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"plugin\"\n",
                "producer = \"rust-cdylib\"\n",
                "file = \"desktop_tools.dll\"\n",
                "contract = \"arcana.cabi.plugin.v1\"\n",
                "rust_cdylib_crate = \"../../crates/plugin\"\n",
                "sidecars = [\"assets/runtime.txt\"]\n",
            ),
        );
        write_file(
            &dir.join("src/book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(&dir.join("src/types.arc"), "// desktop types\n");
        write_file(&dir.join("assets/runtime.txt"), "alpha\n");

        let first = member_source_fingerprint(&dir, "desktop");
        write_file(&dir.join("assets/runtime.txt"), "beta\n");
        let second = member_source_fingerprint(&dir, "desktop");

        assert_ne!(
            first, second,
            "native product sidecar content changes should affect the source fingerprint"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn foreword_product_manifest_changes_update_source_fingerprint() {
        fn member_source_fingerprint(dir: &Path, member: &str) -> String {
            let graph = load_workspace_graph(dir).expect("graph should load");
            let workspace = load_workspace_hir_from_graph(dir, &graph).expect("workspace");
            let resolved_workspace = arcana_hir::resolve_workspace(&workspace).expect("resolve");
            let fingerprints =
                compute_workspace_fingerprints(&graph, &workspace, &resolved_workspace)
                    .expect("fingerprints");
            fingerprint_member(&fingerprints, &graph, member)
                .source()
                .to_string()
        }

        let dir = temp_dir("foreword_product_manifest_fingerprint");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"tool\"\n",
                "kind = \"lib\"\n",
                "[toolchain.foreword_products.adapter]\n",
                "path = \"forewords/adapter.cmd\"\n",
            ),
        );
        write_file(
            &dir.join("src/book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(&dir.join("src/types.arc"), "// tool types\n");
        write_file(
            &dir.join("forewords/adapter.cmd"),
            "@echo off\r\necho {}\r\n",
        );

        let first = member_source_fingerprint(&dir, "tool");
        write_file(
            &dir.join("book.toml"),
            concat!(
                "name = \"tool\"\n",
                "kind = \"lib\"\n",
                "[toolchain.foreword_products.adapter]\n",
                "path = \"forewords/adapter2.cmd\"\n",
            ),
        );
        write_file(
            &dir.join("forewords/adapter2.cmd"),
            "@echo off\r\necho {}\r\n",
        );
        let second = member_source_fingerprint(&dir, "tool");

        assert_ne!(
            first, second,
            "foreword product manifest changes should affect the source fingerprint"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_and_read_lockfile_preserve_native_product_metadata() {
        let dir = temp_dir("lockfile_native_products");
        write_file(
            &dir.join("book.toml"),
            "name = \"ws\"\nkind = \"app\"\n[workspace]\nmembers = [\"app\", \"desktop\"]\n",
        );
        write_file(
            &dir.join("app/book.toml"),
            "name = \"app\"\nkind = \"app\"\n[deps]\ndesktop = { path = \"../desktop\", native_child = \"default\" }\n",
        );
        write_file(
            &dir.join("app/src/shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("app/src/types.arc"), "// app types\n");
        write_file(
            &dir.join("desktop/book.toml"),
            concat!(
                "name = \"desktop\"\n",
                "kind = \"lib\"\n",
                "\n[native.products.default]\n",
                "kind = \"dll\"\n",
                "role = \"child\"\n",
                "producer = \"arcana-source\"\n",
                "file = \"arcwin.dll\"\n",
                "contract = \"arcana.cabi.child.v1\"\n",
                "sidecars = [\"assets/runtime.txt\"]\n",
            ),
        );
        write_file(
            &dir.join("desktop/src/book.arc"),
            "export fn ready() -> Int:\n    return 1\n",
        );
        write_file(&dir.join("desktop/src/types.arc"), "// desktop types\n");
        write_file(&dir.join("desktop/assets/runtime.txt"), "desktop-runtime\n");

        let graph = load_workspace_graph(&dir).expect("graph should load");
        let order = plan_workspace(&graph).expect("plan");
        let (prepared, statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &prepared, &statuses).expect("build should execute");
        let lock_text = render_lockfile(&graph, &order, &statuses, None).expect("render lockfile");
        let lock_path = dir.join("Arcana.lock");
        write_file(&lock_path, &lock_text);

        let lock = read_lockfile(&lock_path)
            .expect("lockfile should read")
            .expect("lockfile should exist");
        let desktop = lock_member(&lock, Some(&graph), "desktop");
        let product = desktop
            .native_products
            .get("default")
            .expect("desktop native product should persist");
        assert_eq!(product.role, "child");
        assert_eq!(product.producer, "arcana-source");
        assert_eq!(product.file, "arcwin.dll");
        assert_eq!(product.contract, "arcana.cabi.child.v1");
        assert_eq!(product.sidecars, vec!["assets/runtime.txt".to_string()]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn execute_build_rejects_statuses_from_different_snapshot() {
        let dir = temp_dir("prepared_snapshot_mismatch");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src/shelf.arc"),
            "fn main() -> Int:\n    return 0\n",
        );
        write_file(&dir.join("src/types.arc"), "// types\n");

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let first_prepared = prepare_test_build(&graph);
        write_file(
            &dir.join("src/shelf.arc"),
            "fn main() -> Int:\n    return 7\n",
        );
        let (_, statuses) = plan_test_build(&graph, &order, None);
        let err = execute_build(&graph, &first_prepared, &statuses)
            .expect_err("mismatched snapshot should fail");
        assert!(
            err.contains("planned from snapshot"),
            "expected snapshot mismatch error, got: {err}"
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
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        write_file(
            &dir.join("app/src/shelf.arc"),
            "fn main() -> Int:\n    return 1\n",
        );
        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(
            &second_statuses,
            &[
                ("core", BuildDisposition::CacheHit),
                ("app", BuildDisposition::Built),
            ],
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_linked_dependency_code_rebuilds_dependents() {
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
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        write_file(
            &dir.join("core/src/helper.arc"),
            "fn helper() -> Int:\n    return 7\n",
        );
        let (second_prepared, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(
            &second_statuses,
            &[
                ("core", BuildDisposition::Built),
                ("app", BuildDisposition::Built),
                ("tool", BuildDisposition::Built),
            ],
        );

        execute_planned_build(&graph, &second_prepared, &second_statuses)
            .expect("rebuild dependents");
        let app_artifact_path = graph.root_dir.join(&second_statuses[1].artifact_rel_path);
        let app_artifact = fs::read_to_string(&app_artifact_path).expect("app artifact");
        assert!(
            app_artifact.contains("Int = 7"),
            "expected rebuilt app artifact to embed updated linked dependency: {app_artifact}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_implicit_std_rebuilds_dependents() {
        let dir = temp_dir("implicit_std");
        write_file(&dir.join("book.toml"), "name = \"app\"\nkind = \"app\"\n");
        write_file(
            &dir.join("src/shelf.arc"),
            "import std.answer\nfn main() -> Int:\n    return std.answer.value :: :: call\n",
        );
        write_file(&dir.join("src/types.arc"), "// types\n");
        write_grimoire(&dir.join("std"), GrimoireKind::Lib, "std", &[]);
        write_file(&dir.join("std/src/book.arc"), "import answer\n");
        write_file(
            &dir.join("std/src/answer.arc"),
            "export fn value() -> Int:\n    return 0\n",
        );

        let graph = load_workspace_graph(&dir).expect("load graph");
        let order = plan_workspace(&graph).expect("plan");
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        write_file(
            &dir.join("std/src/answer.arc"),
            "export fn value() -> Int:\n    return 7\n",
        );
        let (second_prepared, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(&second_statuses, &[("app", BuildDisposition::Built)]);

        execute_planned_build(&graph, &second_prepared, &second_statuses)
            .expect("rebuild dependents");
        let artifact_path = graph
            .root_dir
            .join(status(&second_statuses, "app").artifact_rel_path());
        let artifact = fs::read_to_string(&artifact_path).expect("app artifact");
        assert!(
            artifact.contains("Int = 7"),
            "expected rebuilt app artifact to embed updated std routine: {artifact}"
        );

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
        let (first_prepared, first_statuses) = plan_test_build(&graph, &order, None);
        execute_planned_build(&graph, &first_prepared, &first_statuses).expect("execute");
        let lock_path = write_lockfile(&graph, &order, &first_statuses).expect("lock");
        let existing = read_lockfile(&lock_path).expect("read").expect("lock");

        write_file(
            &dir.join("core/src/book.arc"),
            "export fn shared_value(read value: Int) -> Int:\n    return value\n",
        );
        let (_, second_statuses) = plan_test_build(&graph, &order, Some(&existing));
        assert_dispositions(
            &second_statuses,
            &[
                ("core", BuildDisposition::Built),
                ("app", BuildDisposition::Built),
                ("tool", BuildDisposition::Built),
            ],
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
