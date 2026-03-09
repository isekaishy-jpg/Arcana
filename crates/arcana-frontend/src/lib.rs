use std::fs;
use std::path::{Path, PathBuf};

use arcana_hir::{
    HirDirectiveKind, HirModule, HirModuleSummary, HirWorkspacePackage, HirWorkspaceSummary,
    lower_module_text,
};
use arcana_package::load_workspace_hir as load_package_workspace_hir;

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

    let workspace = load_package_workspace_hir(&root_dir)?;
    validate_packages(&workspace)
}

pub fn load_workspace_hir(path: &Path) -> Result<HirWorkspaceSummary, String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "workspace HIR requires a grimoire or workspace directory, got `{}`",
            path.display()
        ));
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

    load_package_workspace_hir(&root_dir)
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

fn validate_packages(workspace: &HirWorkspaceSummary) -> Result<CheckSummary, String> {
    let mut summary = CheckSummary {
        package_count: workspace.package_count(),
        ..CheckSummary::default()
    };
    let mut diagnostics = Vec::new();

    for package in workspace.packages.values() {
        for module in &package.summary.modules {
            summary.module_count += 1;
            summary.non_empty_lines += module.non_empty_line_count;
            summary.directive_count += module.directives.len();
            summary.symbol_count += module.symbols.len();
        }

        for edge in &package.summary.dependency_edges {
            let outcome = match edge.kind {
                HirDirectiveKind::Import | HirDirectiveKind::Reexport => {
                    resolve_exact_module(package, workspace, &edge.target_path).map(|_| ())
                }
                HirDirectiveKind::Use => resolve_use_target(package, workspace, &edge.target_path),
            };

            if let Err(message) = outcome {
                diagnostics.push(Diagnostic {
                    path: package
                        .module_path(&edge.source_module_id)
                        .cloned()
                        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc")),
                    line: edge.span.line,
                    column: edge.span.column,
                    message,
                });
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

fn resolve_exact_module<'a>(
    package: &'a HirWorkspacePackage,
    workspace: &'a HirWorkspaceSummary,
    path: &[String],
) -> Result<&'a HirModuleSummary, String> {
    if path.is_empty() {
        return Err("missing module path".to_string());
    }

    let key = join_segments(path);
    let first = &path[0];
    if first == &package.summary.package_name {
        return package.module(&key).ok_or_else(|| format!("unresolved module `{key}`"));
    }

    if first == "std" {
        return workspace
            .package("std")
            .ok_or_else(|| "implicit package `std` is not available".to_string())
            .and_then(|std_package| {
                std_package.module(&key).ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if package.direct_deps.contains(first) {
        return workspace
            .package(first)
            .ok_or_else(|| {
                format!(
                    "dependency `{first}` is not loaded for `{}`",
                    package.summary.package_name
                )
            })
            .and_then(|dependency| {
                dependency.module(&key).ok_or_else(|| format!("unresolved module `{key}`"))
            });
    }

    if workspace.package(first).is_some() {
        return Err(format!(
            "package `{first}` is not a direct dependency of `{}`",
            package.summary.package_name
        ));
    }

    package
        .resolve_relative_module(path)
        .ok_or_else(|| format!("unresolved module `{key}`"))
}

fn resolve_use_target(
    package: &HirWorkspacePackage,
    workspace: &HirWorkspaceSummary,
    path: &[String],
) -> Result<(), String> {
    if path.is_empty() {
        return Err("missing use target".to_string());
    }

    for prefix_len in (1..=path.len()).rev() {
        let prefix = &path[..prefix_len];
        let Ok(module) = resolve_exact_module(package, workspace, prefix) else {
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
        if module.has_symbol(symbol_name) {
            return Ok(());
        }
        return Err(format!(
            "unresolved symbol `{symbol_name}` in module `{}`",
            module.module_id
        ));
    }

    if let Some(first) = path.first() {
        if workspace.package(first).is_some()
            && first != &package.summary.package_name
            && first != "std"
            && !package.direct_deps.contains(first)
        {
            return Err(format!(
                "package `{first}` is not a direct dependency of `{}`",
                package.summary.package_name
            ));
        }
    }

    Err(format!("unresolved module path `{}`", join_segments(path)))
}

fn join_segments(segments: &[String]) -> String {
    segments.join(".")
}

#[cfg(test)]
mod tests {
    use super::{check_path, check_sources, load_workspace_hir, lower_to_hir};
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

    #[test]
    fn load_workspace_hir_exposes_package_summaries() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve");
        let workspace = load_workspace_hir(&repo_root.join("examples").join("workspace_vertical_slice"))
            .expect("workspace hir should load");
        assert!(workspace.package("desktop_app").is_some());
        assert!(workspace.package("winspell").is_some());
        assert!(
            workspace
                .package("winspell")
                .expect("winspell package should exist")
                .summary
                .dependency_edges
                .iter()
                .any(|edge| edge.target_path == vec!["std".to_string(), "canvas".to_string()])
        );
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
