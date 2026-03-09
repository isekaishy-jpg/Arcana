use std::fs;
use std::path::{Path, PathBuf};

use arcana_hir::{
    HirModule, HirResolvedModule, HirResolvedTarget, HirResolvedWorkspace, HirWorkspaceSummary,
    lower_module_text, resolve_workspace,
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

    let root_dir = fs::canonicalize(path)
        .map_err(|err| format!("failed to open `{}`: {err}", path.display()))?;
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

    let root_dir = fs::canonicalize(path)
        .map_err(|err| format!("failed to open `{}`: {err}", path.display()))?;
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
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read `{}`: {err}", path.display()))?;
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

    for package in workspace.packages.values() {
        for module in &package.summary.modules {
            summary.module_count += 1;
            summary.non_empty_lines += module.non_empty_line_count;
            summary.directive_count += module.directives.len();
            summary.symbol_count += module.symbols.len();
        }
    }

    let (resolved_workspace, mut diagnostics) = match resolve_workspace(workspace) {
        Ok(resolved) => (Some(resolved), Vec::new()),
        Err(errors) => {
            let diagnostics = errors
                .into_iter()
                .map(|error| {
                    let package = workspace.package(&error.package_name);
                    Diagnostic {
                        path: package
                            .and_then(|package| package.module_path(&error.source_module_id))
                            .cloned()
                            .unwrap_or_else(|| {
                                package
                                    .map(|package| package.root_dir.join("src").join("unknown.arc"))
                                    .unwrap_or_else(|| PathBuf::from("unknown.arc"))
                            }),
                        line: error.span.line,
                        column: error.span.column,
                        message: error.message,
                    }
                })
                .collect::<Vec<_>>();
            (None, diagnostics)
        }
    };

    if let Some(resolved_workspace) = resolved_workspace.as_ref() {
        diagnostics.extend(validate_hir_semantics(workspace, resolved_workspace));
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

fn validate_hir_semantics(
    workspace: &HirWorkspaceSummary,
    resolved: &HirResolvedWorkspace,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (package_name, package) in &workspace.packages {
        let Some(resolved_package) = resolved.package(package_name) else {
            continue;
        };
        for module in &package.summary.modules {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                continue;
            };
            for lang_item in &module.lang_items {
                if !lang_item_resolves(workspace, resolved_module, lang_item.target.as_slice()) {
                    diagnostics.push(Diagnostic {
                        path: package
                            .module_path(&module.module_id)
                            .cloned()
                            .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc")),
                        line: lang_item.span.line,
                        column: lang_item.span.column,
                        message: format!(
                            "unresolved `lang` item target `{}` for `{}`",
                            lang_item.target.join("."),
                            lang_item.name
                        ),
                    });
                }
            }
        }
    }
    diagnostics
}

fn lang_item_resolves(
    workspace: &HirWorkspaceSummary,
    module: &HirResolvedModule,
    path: &[String],
) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.len() == 1 {
        return matches!(
            module.bindings.get(&path[0]).map(|binding| &binding.target),
            Some(HirResolvedTarget::Symbol { .. })
        );
    }

    let first = &path[0];
    if let Some(binding) = module.bindings.get(first) {
        return resolve_target_tail(workspace, &binding.target, &path[1..]);
    }

    let Some(package) = workspace.package(first) else {
        return false;
    };
    resolve_package_symbol_path(package, &path[1..])
}

fn resolve_target_tail(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
    tail: &[String],
) -> bool {
    match target {
        HirResolvedTarget::Symbol { .. } => tail.is_empty(),
        HirResolvedTarget::Module {
            package_name,
            module_id,
        } => {
            let Some(package) = workspace.package(package_name) else {
                return false;
            };
            let Some(module) = package.module(module_id) else {
                return false;
            };
            resolve_module_symbol_path(package, module, tail)
        }
    }
}

fn resolve_package_symbol_path(package: &arcana_hir::HirWorkspacePackage, path: &[String]) -> bool {
    if path.is_empty() {
        return false;
    }
    let Some((symbol_name, module_path)) = path.split_last() else {
        return false;
    };
    let module = if symbol_name.is_empty() {
        return false;
    } else if module_path.is_empty() {
        package.module(&package.summary.package_name)
    } else {
        package.resolve_relative_module(module_path)
    };
    module
        .map(|module| module.has_symbol(symbol_name))
        .unwrap_or(false)
}

fn resolve_module_symbol_path(
    package: &arcana_hir::HirWorkspacePackage,
    module: &arcana_hir::HirModuleSummary,
    path: &[String],
) -> bool {
    if path.len() == 1 {
        return module.has_symbol(&path[0]);
    }
    let Some((symbol_name, module_tail)) = path.split_last() else {
        return false;
    };
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut target_relative = base_relative;
    target_relative.extend_from_slice(module_tail);
    package
        .resolve_relative_module(&target_relative)
        .map(|target_module| target_module.has_symbol(symbol_name))
        .unwrap_or(false)
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
        let summary = check_sources(
            ["import std.io\nfn main() -> Int:\n    return 0\n"]
                .iter()
                .copied(),
        )
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
                (
                    "src/shelf.arc",
                    "import missing.module\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let err = check_path(&root).expect_err("expected unresolved import");
        assert!(err.contains("missing.module"), "{err}");

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_sources_rejects_tuple_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "tuple_field_out_of_range.arc",
                "tuple field access only supports `.0` and `.1` in v1",
            ),
            (
                "tuple_destructure_let.arc",
                "tuple destructuring is not allowed in `let` statements",
            ),
            (
                "tuple_triple_type.arc",
                "tuple types must have exactly 2 elements in v1",
            ),
            (
                "tuple_field_assignment.arc",
                "tuple field assignment is not allowed in v1",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_sources_rejects_page_rollup_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "page_rollup_stray.arc",
                "page rollup without a valid owning header",
            ),
            (
                "page_rollup_bad_subject.arc",
                "cleanup subject must be a binding name",
            ),
            (
                "page_rollup_unknown_subject.arc",
                "cleanup subject `missing` is not available in the owning header scope",
            ),
            (
                "page_rollup_reassign.arc",
                "cleanup subject `local` cannot be reassigned after activation",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_sources_rejects_foreword_and_intrinsic_contract_fixtures() {
        let repo_root = repo_root();
        for (fixture, expected) in [
            (
                "invalid_statement_foreword.arc",
                "`#inline` is not a valid statement-level contract",
            ),
            (
                "malformed_intrinsic.arc",
                "malformed intrinsic function declaration",
            ),
        ] {
            let source = fs::read_to_string(
                repo_root
                    .join("conformance")
                    .join("check_parity_fixtures")
                    .join(fixture),
            )
            .expect("fixture should be readable");
            let err = check_sources([source.as_str()]).expect_err("fixture should fail");
            assert!(err.contains(expected), "{fixture}: {err}");
        }
    }

    #[test]
    fn check_path_resolves_local_use_symbols() {
        let root = make_temp_package(
            "counter_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "import types\nuse types.Counter\nfn main() -> Int:\n    return 0\n",
                ),
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
    fn check_path_handles_builtin_foreword_example() {
        let summary = check_path(&repo_root().join("examples").join("forewords_builtin_app"))
            .expect("foreword example should check");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 3);
    }

    #[test]
    fn check_path_handles_std_intrinsics() {
        let summary = check_path(&repo_root().join("std")).expect("std should check");
        assert!(summary.package_count >= 1);
        assert!(summary.module_count >= 10);
    }

    #[test]
    fn check_path_handles_page_rollup_example() {
        let summary = check_path(&repo_root().join("examples").join("page_rollup_cleanup"))
            .expect("page rollup example should check");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 3);
    }

    #[test]
    fn check_path_rejects_unresolved_lang_item_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_lang_item"),
        )
        .expect_err("unresolved lang item package should fail");
        assert!(
            err.contains("unresolved `lang` item target `Missing` for `result`"),
            "{err}"
        );
    }

    #[test]
    fn load_workspace_hir_exposes_package_summaries() {
        let repo_root = repo_root();
        let workspace =
            load_workspace_hir(&repo_root.join("examples").join("workspace_vertical_slice"))
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

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root should resolve")
    }

    fn unique_test_id() -> u64 {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos() as u64;
        time ^ NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    }
}
