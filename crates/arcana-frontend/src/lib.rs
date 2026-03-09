use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use arcana_hir::{HirDirectiveKind, HirModule, HirModuleSummary, lower_module_text};
use arcana_package::{GrimoireKind, WorkspaceGraph, load_workspace_graph, parse_manifest};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckSummary {
    pub package_count: usize,
    pub module_count: usize,
    pub non_empty_lines: usize,
    pub directive_count: usize,
    pub symbol_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Diagnostic {
    path: PathBuf,
    line: usize,
    column: usize,
    message: String,
}

impl Diagnostic {
    fn render(&self) -> String {
        format!(
            "{}:{}:{}: {}",
            self.path.display(),
            self.line,
            self.column,
            self.message
        )
    }
}

#[derive(Clone, Debug)]
struct ModuleRecord {
    path: PathBuf,
    relative_key: Option<String>,
    hir: HirModuleSummary,
}

#[derive(Clone, Debug)]
struct PackageRecord {
    name: String,
    root_dir: PathBuf,
    direct_deps: BTreeSet<String>,
    modules: Vec<ModuleRecord>,
    relative_modules: BTreeMap<String, usize>,
    absolute_modules: BTreeMap<String, usize>,
}

pub fn check_sources<'a, I>(sources: I) -> Result<CheckSummary, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut summary = CheckSummary::default();
    for (index, source) in sources.into_iter().enumerate() {
        let hir = lower_module_text(format!("memory.module.{index}"), source)?;
        summary.module_count += 1;
        summary.non_empty_lines += hir.non_empty_line_count;
        summary.directive_count += hir.directives.len();
        summary.symbol_count += hir.symbols.len();
    }
    Ok(summary)
}

pub fn check_path(path: &Path) -> Result<CheckSummary, String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    if metadata.is_file() {
        return check_file(path);
    }
    if !metadata.is_dir() {
        return Err(format!("`{}` is not a file or directory", path.display()));
    }

    let root_dir =
        fs::canonicalize(path).map_err(|err| format!("failed to open `{}`: {err}", path.display()))?;
    let manifest_path = root_dir.join("book.toml");
    if !manifest_path.is_file() {
        return Err(format!(
            "`{}` does not contain a `book.toml` manifest",
            root_dir.display()
        ));
    }

    let graph = load_workspace_graph(&root_dir)?;
    let packages = load_packages_for_check(&root_dir, &graph)?;
    validate_packages(&packages)
}

pub fn lower_to_hir(summary: &CheckSummary) -> HirModule {
    HirModule {
        symbol_count: summary.symbol_count.max(summary.module_count),
        item_count: summary.non_empty_lines + summary.directive_count,
    }
}

fn check_file(path: &Path) -> Result<CheckSummary, String> {
    let source =
        fs::read_to_string(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    let hir = lower_module_text(path.display().to_string(), &source)
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(CheckSummary {
        package_count: 0,
        module_count: 1,
        non_empty_lines: hir.non_empty_line_count,
        directive_count: hir.directives.len(),
        symbol_count: hir.symbols.len(),
    })
}

fn load_packages_for_check(
    root_dir: &Path,
    graph: &WorkspaceGraph,
) -> Result<BTreeMap<String, PackageRecord>, String> {
    let mut packages = BTreeMap::new();

    let root_manifest = parse_manifest(&root_dir.join("book.toml"))?;
    let root_already_in_graph = graph.members.iter().any(|member| member.abs_dir == root_dir);
    if !root_already_in_graph && has_root_module(root_dir, &root_manifest.kind) {
        let record = load_package(
            root_dir,
            &root_manifest.name,
            &root_manifest.kind,
            root_manifest.deps.keys().cloned().collect(),
        )?;
        packages.insert(record.name.clone(), record);
    }

    for member in &graph.members {
        let record = load_package(
            &member.abs_dir,
            &member.name,
            &member.kind,
            member.deps.iter().cloned().collect(),
        )?;
        packages.insert(record.name.clone(), record);
    }

    if let Some(std_dir) = find_implicit_std(root_dir)? {
        let manifest = parse_manifest(&std_dir.join("book.toml"))?;
        if !packages.contains_key(&manifest.name) {
            let record = load_package(&std_dir, &manifest.name, &manifest.kind, BTreeSet::new())?;
            packages.insert(record.name.clone(), record);
        }
    }

    Ok(packages)
}

fn validate_packages(packages: &BTreeMap<String, PackageRecord>) -> Result<CheckSummary, String> {
    let mut summary = CheckSummary {
        package_count: packages.len(),
        ..CheckSummary::default()
    };
    let mut diagnostics = Vec::new();

    for package in packages.values() {
        for module in &package.modules {
            summary.module_count += 1;
            summary.non_empty_lines += module.hir.non_empty_line_count;
            summary.directive_count += module.hir.directives.len();
            summary.symbol_count += module.hir.symbols.len();

            for directive in &module.hir.directives {
                let outcome = match directive.kind {
                    HirDirectiveKind::Import | HirDirectiveKind::Reexport => {
                        resolve_exact_module(package, packages, &directive.path).map(|_| ())
                    }
                    HirDirectiveKind::Use => resolve_use_target(package, packages, &directive.path),
                };

                if let Err(message) = outcome {
                    diagnostics.push(Diagnostic {
                        path: module.path.clone(),
                        line: directive.span.line,
                        column: directive.span.column,
                        message,
                    });
                }
            }
        }
    }

    if diagnostics.is_empty() {
        return Ok(summary);
    }

    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
            .then_with(|| left.message.cmp(&right.message))
    });
    Err(diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.render())
        .collect::<Vec<_>>()
        .join("\n"))
}

fn load_package(
    root_dir: &Path,
    name: &str,
    kind: &GrimoireKind,
    direct_deps: BTreeSet<String>,
) -> Result<PackageRecord, String> {
    let src_dir = root_dir.join("src");
    let root_file = src_dir.join(kind.root_file_name());
    if !root_file.is_file() {
        return Err(format!(
            "missing `{}` in `{}`",
            kind.root_file_name(),
            src_dir.display()
        ));
    }

    let mut package = PackageRecord {
        name: name.to_string(),
        root_dir: root_dir.to_path_buf(),
        direct_deps,
        modules: Vec::new(),
        relative_modules: BTreeMap::new(),
        absolute_modules: BTreeMap::new(),
    };

    for module_path in collect_arc_files(&src_dir)? {
        let relative_segments = module_segments(kind, &src_dir, &module_path)?;
        let relative_key = join_segments(&relative_segments);
        let absolute_segments = if relative_segments.is_empty() {
            vec![name.to_string()]
        } else {
            let mut segments = vec![name.to_string()];
            segments.extend(relative_segments.iter().cloned());
            segments
        };
        let absolute_key = join_segments(&absolute_segments);
        let source = fs::read_to_string(&module_path)
            .map_err(|err| format!("failed to read `{}`: {err}", module_path.display()))?;
        let hir = lower_module_text(absolute_key.clone(), &source)
            .map_err(|err| format!("{}: {err}", module_path.display()))?;
        let module = ModuleRecord {
            path: module_path,
            relative_key: if relative_key.is_empty() {
                None
            } else {
                Some(relative_key.clone())
            },
            hir,
        };

        let index = package.modules.len();
        if package
            .absolute_modules
            .insert(absolute_key.clone(), index)
            .is_some()
        {
            return Err(format!(
                "duplicate module path `{absolute_key}` in `{}`",
                package.root_dir.display()
            ));
        }
        if let Some(relative_key) = &module.relative_key {
            if package
                .relative_modules
                .insert(relative_key.clone(), index)
                .is_some()
            {
                return Err(format!(
                    "duplicate module path `{relative_key}` in `{}`",
                    package.root_dir.display()
                ));
            }
        }
        package.modules.push(module);
    }

    Ok(package)
}

fn collect_arc_files(src_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_arc_files_recursive(src_dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_arc_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|err| format!("failed to read `{}`: {err}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read `{}`: {err}", dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to inspect `{}`: {err}", path.display()))?;
        if file_type.is_dir() {
            collect_arc_files_recursive(&path, files)?;
            continue;
        }
        if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("arc") {
            files.push(path);
        }
    }

    Ok(())
}

fn module_segments(kind: &GrimoireKind, src_dir: &Path, module_path: &Path) -> Result<Vec<String>, String> {
    let relative = module_path
        .strip_prefix(src_dir)
        .map_err(|err| format!("failed to relativize `{}`: {err}", module_path.display()))?;
    let mut components = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if components.is_empty() {
        return Err(format!("empty module path for `{}`", module_path.display()));
    }

    let file_name = components
        .pop()
        .ok_or_else(|| format!("empty module path for `{}`", module_path.display()))?;
    let stem = file_name
        .strip_suffix(".arc")
        .ok_or_else(|| format!("non-Arcana file `{}`", module_path.display()))?;
    if stem != "book" && stem != "shelf" {
        components.push(stem.to_string());
    } else if file_name != kind.root_file_name() && !components.is_empty() {
        components.push(stem.to_string());
    }
    Ok(components)
}

fn resolve_exact_module<'a>(
    package: &'a PackageRecord,
    packages: &'a BTreeMap<String, PackageRecord>,
    path: &[String],
) -> Result<&'a ModuleRecord, String> {
    if path.is_empty() {
        return Err("missing module path".to_string());
    }

    let key = join_segments(path);
    let first = &path[0];
    if first == &package.name {
        return package
            .absolute_modules
            .get(&key)
            .map(|index| &package.modules[*index])
            .ok_or_else(|| format!("unresolved module `{key}`"));
    }

    if first == "std" {
        return packages
            .get("std")
            .ok_or_else(|| "implicit package `std` is not available".to_string())
            .and_then(|std_package| {
                std_package
                    .absolute_modules
                    .get(&key)
                    .map(|index| &std_package.modules[*index])
                    .ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if package.direct_deps.contains(first) {
        return packages
            .get(first)
            .ok_or_else(|| format!("dependency `{first}` is not loaded for `{}`", package.name))
            .and_then(|dependency| {
                dependency
                    .absolute_modules
                    .get(&key)
                    .map(|index| &dependency.modules[*index])
                    .ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if packages.contains_key(first) {
        return Err(format!(
            "package `{first}` is not a direct dependency of `{}`",
            package.name
        ));
    }

    package
        .relative_modules
        .get(&key)
        .map(|index| &package.modules[*index])
        .ok_or_else(|| format!("unresolved module `{key}`"))
}

fn resolve_use_target(
    package: &PackageRecord,
    packages: &BTreeMap<String, PackageRecord>,
    path: &[String],
) -> Result<(), String> {
    if path.is_empty() {
        return Err("missing use target".to_string());
    }

    for prefix_len in (1..=path.len()).rev() {
        let prefix = &path[..prefix_len];
        let Ok(module) = resolve_exact_module(package, packages, prefix) else {
            continue;
        };
        if prefix_len == path.len() {
            return Ok(());
        }

        let suffix = &path[prefix_len..];
        if suffix.len() != 1 {
            return Err(format!(
                "nested symbol path `{}` is not supported yet",
                join_segments(path)
            ));
        }

        let symbol_name = &suffix[0];
        if module.hir.has_symbol(symbol_name) {
            return Ok(());
        }
        return Err(format!(
            "unresolved symbol `{symbol_name}` in module `{}`",
            module.hir.module_id
        ));
    }

    if let Some(first) = path.first() {
        if packages.contains_key(first) && first != &package.name && first != "std" {
            if !package.direct_deps.contains(first) {
                return Err(format!(
                    "package `{first}` is not a direct dependency of `{}`",
                    package.name
                ));
            }
        }
    }

    Err(format!("unresolved module path `{}`", join_segments(path)))
}

fn join_segments(segments: &[String]) -> String {
    segments.join(".")
}

fn has_root_module(root_dir: &Path, kind: &GrimoireKind) -> bool {
    root_dir.join("src").join(kind.root_file_name()).is_file()
}

fn find_implicit_std(start: &Path) -> Result<Option<PathBuf>, String> {
    let mut cursor = if start.is_file() {
        start.parent().map(Path::to_path_buf)
    } else {
        Some(start.to_path_buf())
    };

    while let Some(dir) = cursor {
        let candidate = dir.join("std").join("book.toml");
        if candidate.is_file() {
            let std_dir = candidate
                .parent()
                .ok_or_else(|| format!("failed to resolve implicit std from `{}`", candidate.display()))?;
            let canonical = fs::canonicalize(std_dir).map_err(|err| {
                format!("failed to open implicit std package `{}`: {err}", std_dir.display())
            })?;
            return Ok(Some(canonical));
        }
        cursor = dir.parent().map(Path::to_path_buf);
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{check_path, check_sources, lower_to_hir};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn check_sources_counts_modules() {
        let summary = check_sources(["import std.io\nfn main() -> Int:\n    return 0\n"].iter().copied())
            .expect("check should pass");
        assert_eq!(summary.module_count, 1);
        assert_eq!(summary.directive_count, 1);
        assert!(summary.symbol_count >= 1);

        let hir = lower_to_hir(&summary);
        assert!(hir.symbol_count >= 1);
    }

    #[test]
    fn check_path_reports_unresolved_import() {
        let root = make_temp_package(
            "broken_app",
            "app",
            &[],
            &[
                ("src/shelf.arc", "import missing.module\nfn main() -> Int:\n    return 0\n"),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("expected unresolved import");
        assert!(err.contains("missing.module"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_resolves_local_use_symbols() {
        let root = make_temp_package(
            "counter_app",
            "app",
            &[],
            &[
                ("src/shelf.arc", "import types\nuse types.Counter\nfn main() -> Int:\n    return 0\n"),
                ("src/types.arc", "export record Counter:\n    value: Int\n"),
            ],
        );

        let summary = check_path(&root).expect("local symbols should resolve");
        assert_eq!(summary.module_count, 2);
        assert_eq!(summary.package_count, 1);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_real_first_party_grimoire() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve");
        let summary = check_path(&repo_root.join("grimoires").join("winspell"))
            .expect("first-party grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 5);
    }

    fn make_temp_package(
        name: &str,
        kind: &str,
        deps: &[(&str, &str)],
        files: &[(&str, &str)],
    ) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "arcana-frontend-test-{}-{}",
            unique_test_id(),
            name
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale temp dir should be removable");
        }

        fs::create_dir_all(root.join("src")).expect("src dir should be creatable");
        let mut manifest = format!("name = \"{name}\"\nkind = \"{kind}\"\n");
        if !deps.is_empty() {
            manifest.push_str("\n[deps]\n");
            for (dep_name, dep_path) in deps {
                manifest.push_str(&format!("{dep_name} = {{ path = \"{dep_path}\" }}\n"));
            }
        }
        fs::write(root.join("book.toml"), manifest).expect("manifest should be writable");

        for (rel_path, contents) in files {
            let path = root.join(rel_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent dirs should be creatable");
            }
            fs::write(path, contents).expect("source file should be writable");
        }

        root
    }

    fn unique_test_id() -> u64 {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos() as u64;
        time ^ NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    }
}
