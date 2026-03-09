pub mod freeze;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use arcana_syntax::{
    DirectiveKind as ParsedDirectiveKind, ParsedModule, Span, SymbolKind as ParsedSymbolKind,
    parse_module,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirModule {
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HirDirectiveKind {
    Import,
    Use,
    Reexport,
}

impl HirDirectiveKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Use => "use",
            Self::Reexport => "reexport",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirDirective {
    pub kind: HirDirectiveKind,
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HirSymbolKind {
    Fn,
    Record,
    Enum,
    Trait,
    Behavior,
    Const,
}

impl HirSymbolKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fn => "fn",
            Self::Record => "record",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Behavior => "behavior",
            Self::Const => "const",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirSymbol {
    pub kind: HirSymbolKind,
    pub name: String,
    pub exported: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirModuleSummary {
    pub module_id: String,
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directives: Vec<HirDirective>,
    pub symbols: Vec<HirSymbol>,
}

impl HirModuleSummary {
    pub fn has_symbol(&self, name: &str) -> bool {
        self.symbols.iter().any(|symbol| symbol.name == name)
    }

    pub fn exported_surface_rows(&self) -> Vec<String> {
        let mut rows = self
            .directives
            .iter()
            .filter(|directive| directive.kind == HirDirectiveKind::Reexport)
            .map(|directive| format!("reexport:{}", directive.path.join(".")))
            .collect::<Vec<_>>();
        rows.extend(
            self.symbols
                .iter()
                .filter(|symbol| symbol.exported)
                .map(|symbol| format!("export:{}:{}", symbol.kind.as_str(), symbol.name)),
        );
        rows.sort();
        rows
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirModuleDependency {
    pub source_module_id: String,
    pub kind: HirDirectiveKind,
    pub target_path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirPackageSummary {
    pub package_name: String,
    pub modules: Vec<HirModuleSummary>,
    pub dependency_edges: Vec<HirModuleDependency>,
}

impl HirPackageSummary {
    pub fn module(&self, module_id: &str) -> Option<&HirModuleSummary> {
        self.modules.iter().find(|module| module.module_id == module_id)
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    pub fn exported_surface_rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        for module in &self.modules {
            for row in module.exported_surface_rows() {
                rows.push(format!("module={}:{}", module.module_id, row));
            }
        }
        rows.sort();
        rows
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirPackageLayout {
    pub module_paths: BTreeMap<String, PathBuf>,
    pub relative_modules: BTreeMap<String, String>,
    pub absolute_modules: BTreeMap<String, usize>,
}

impl HirPackageLayout {
    pub fn module<'a>(
        &self,
        summary: &'a HirPackageSummary,
        module_id: &str,
    ) -> Option<&'a HirModuleSummary> {
        self.absolute_modules
            .get(module_id)
            .and_then(|index| summary.modules.get(*index))
    }

    pub fn module_path(&self, module_id: &str) -> Option<&PathBuf> {
        self.module_paths.get(module_id)
    }

    pub fn resolve_relative_module<'a>(
        &self,
        summary: &'a HirPackageSummary,
        path: &[String],
    ) -> Option<&'a HirModuleSummary> {
        let key = path.join(".");
        self.relative_modules
            .get(&key)
            .and_then(|module_id| self.module(summary, module_id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirWorkspacePackage {
    pub root_dir: PathBuf,
    pub direct_deps: BTreeSet<String>,
    pub summary: HirPackageSummary,
    pub layout: HirPackageLayout,
}

impl HirWorkspacePackage {
    pub fn module(&self, module_id: &str) -> Option<&HirModuleSummary> {
        self.layout.module(&self.summary, module_id)
    }

    pub fn module_path(&self, module_id: &str) -> Option<&PathBuf> {
        self.layout.module_path(module_id)
    }

    pub fn resolve_relative_module(&self, path: &[String]) -> Option<&HirModuleSummary> {
        self.layout.resolve_relative_module(&self.summary, path)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirWorkspaceSummary {
    pub packages: BTreeMap<String, HirWorkspacePackage>,
}

impl HirWorkspaceSummary {
    pub fn package(&self, name: &str) -> Option<&HirWorkspacePackage> {
        self.packages.get(name)
    }

    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    pub fn module_count(&self) -> usize {
        self.packages
            .values()
            .map(|package| package.summary.modules.len())
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceModulePath {
    pub relative_segments: Vec<String>,
    pub module_id: String,
}

pub fn lower_module_text(module_id: impl Into<String>, source: &str) -> Result<HirModuleSummary, String> {
    let parsed = parse_module(source)?;
    Ok(lower_parsed_module(module_id, &parsed))
}

pub fn lower_parsed_module(module_id: impl Into<String>, parsed: &ParsedModule) -> HirModuleSummary {
    HirModuleSummary {
        module_id: module_id.into(),
        line_count: parsed.line_count,
        non_empty_line_count: parsed.non_empty_line_count,
        directives: parsed
            .directives
            .iter()
            .map(|directive| HirDirective {
                kind: lower_directive_kind(&directive.kind),
                path: directive.path.clone(),
                alias: directive.alias.clone(),
                span: directive.span,
            })
            .collect(),
        symbols: parsed
            .symbols
            .iter()
            .map(|symbol| HirSymbol {
                kind: lower_symbol_kind(&symbol.kind),
                name: symbol.name.clone(),
                exported: symbol.exported,
                span: symbol.span,
            })
            .collect(),
    }
}

pub fn build_package_summary(
    package_name: impl Into<String>,
    mut modules: Vec<HirModuleSummary>,
) -> HirPackageSummary {
    modules.sort_by(|left, right| left.module_id.cmp(&right.module_id));

    let mut dependency_edges = modules
        .iter()
        .flat_map(|module| {
            module.directives.iter().map(move |directive| HirModuleDependency {
                source_module_id: module.module_id.clone(),
                kind: directive.kind,
                target_path: directive.path.clone(),
                alias: directive.alias.clone(),
                span: directive.span,
            })
        })
        .collect::<Vec<_>>();
    dependency_edges.sort_by(|left, right| {
        left.source_module_id
            .cmp(&right.source_module_id)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.target_path.cmp(&right.target_path))
            .then_with(|| left.alias.cmp(&right.alias))
            .then_with(|| left.span.line.cmp(&right.span.line))
            .then_with(|| left.span.column.cmp(&right.span.column))
    });

    HirPackageSummary {
        package_name: package_name.into(),
        modules,
        dependency_edges,
    }
}

pub fn build_package_layout(
    summary: &HirPackageSummary,
    module_paths: BTreeMap<String, PathBuf>,
    relative_modules: BTreeMap<String, String>,
) -> Result<HirPackageLayout, String> {
    let mut absolute_modules = BTreeMap::new();
    for (index, module) in summary.modules.iter().enumerate() {
        if absolute_modules
            .insert(module.module_id.clone(), index)
            .is_some()
        {
            return Err(format!(
                "duplicate module id `{}` in package `{}`",
                module.module_id, summary.package_name
            ));
        }
    }

    for module_id in absolute_modules.keys() {
        if !module_paths.contains_key(module_id) {
            return Err(format!(
                "missing source path for module `{module_id}` in package `{}`",
                summary.package_name
            ));
        }
    }
    for module_id in module_paths.keys() {
        if !absolute_modules.contains_key(module_id) {
            return Err(format!(
                "source path provided for unknown module `{module_id}` in package `{}`",
                summary.package_name
            ));
        }
    }
    for module_id in relative_modules.values() {
        if !absolute_modules.contains_key(module_id) {
            return Err(format!(
                "relative module mapping targets unknown module `{module_id}` in package `{}`",
                summary.package_name
            ));
        }
    }

    Ok(HirPackageLayout {
        module_paths,
        relative_modules,
        absolute_modules,
    })
}

pub fn build_workspace_package(
    root_dir: PathBuf,
    direct_deps: BTreeSet<String>,
    summary: HirPackageSummary,
    layout: HirPackageLayout,
) -> Result<HirWorkspacePackage, String> {
    if summary.modules.len() != layout.absolute_modules.len() {
        return Err(format!(
            "package `{}` has {} modules but layout indexes {}",
            summary.package_name,
            summary.modules.len(),
            layout.absolute_modules.len()
        ));
    }

    Ok(HirWorkspacePackage {
        root_dir,
        direct_deps,
        summary,
        layout,
    })
}

pub fn build_workspace_summary(
    packages: Vec<HirWorkspacePackage>,
) -> Result<HirWorkspaceSummary, String> {
    let mut package_map = BTreeMap::new();
    for package in packages {
        let name = package.summary.package_name.clone();
        if package_map.insert(name.clone(), package).is_some() {
            return Err(format!("duplicate package `{name}` in workspace summary"));
        }
    }
    Ok(HirWorkspaceSummary {
        packages: package_map,
    })
}

pub fn derive_source_module_path(
    package_name: &str,
    root_file_name: &str,
    src_dir: &Path,
    module_path: &Path,
) -> Result<SourceModulePath, String> {
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
    if stem == "book" || stem == "shelf" {
        if file_name != root_file_name && !components.is_empty() {
            components.push(stem.to_string());
        }
    } else {
        components.push(stem.to_string());
    }

    let mut module_segments = vec![package_name.to_string()];
    module_segments.extend(components.iter().cloned());
    Ok(SourceModulePath {
        relative_segments: components,
        module_id: module_segments.join("."),
    })
}

fn lower_directive_kind(kind: &ParsedDirectiveKind) -> HirDirectiveKind {
    match kind {
        ParsedDirectiveKind::Import => HirDirectiveKind::Import,
        ParsedDirectiveKind::Use => HirDirectiveKind::Use,
        ParsedDirectiveKind::Reexport => HirDirectiveKind::Reexport,
    }
}

fn lower_symbol_kind(kind: &ParsedSymbolKind) -> HirSymbolKind {
    match kind {
        ParsedSymbolKind::Fn => HirSymbolKind::Fn,
        ParsedSymbolKind::Record => HirSymbolKind::Record,
        ParsedSymbolKind::Enum => HirSymbolKind::Enum,
        ParsedSymbolKind::Trait => HirSymbolKind::Trait,
        ParsedSymbolKind::Behavior => HirSymbolKind::Behavior,
        ParsedSymbolKind::Const => HirSymbolKind::Const,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use super::{
        HirDirectiveKind, build_package_layout, build_package_summary, build_workspace_package,
        build_workspace_summary, derive_source_module_path, lower_module_text,
    };
    use super::freeze::FROZEN_HIR_NODE_KINDS;

    #[test]
    fn frozen_hir_list_is_unique() {
        let mut kinds = FROZEN_HIR_NODE_KINDS.to_vec();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), FROZEN_HIR_NODE_KINDS.len());
    }

    #[test]
    fn lower_module_text_preserves_public_surface() {
        let module = lower_module_text(
            "std.io",
            "import std.result\nreexport std.result\nexport fn print() -> Int:\n    return 0\nfn helper() -> Int:\n    return 1\n",
        )
        .expect("lowering should pass");

        assert_eq!(module.module_id, "std.io");
        assert_eq!(module.directives[0].kind, HirDirectiveKind::Import);
        assert_eq!(module.directives[1].kind, HirDirectiveKind::Reexport);
        assert!(module.has_symbol("print"));
        assert!(module.has_symbol("helper"));
        assert_eq!(
            module.exported_surface_rows(),
            vec!["export:fn:print".to_string(), "reexport:std.result".to_string()]
        );
    }

    #[test]
    fn build_package_summary_collects_dependency_edges() {
        let book = lower_module_text(
            "winspell",
            "reexport winspell.window\nexport fn open() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");
        let window = lower_module_text(
            "winspell.window",
            "import std.canvas\nuse std.result.Result\nfn helper() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");

        let package = build_package_summary("winspell", vec![window, book]);
        assert_eq!(package.package_name, "winspell");
        assert_eq!(package.module_count(), 2);
        assert!(package.module("winspell.window").is_some());
        assert_eq!(package.dependency_edges.len(), 3);
        assert_eq!(package.dependency_edges[0].source_module_id, "winspell");
        assert_eq!(package.dependency_edges[0].kind, HirDirectiveKind::Reexport);
        assert_eq!(package.dependency_edges[1].source_module_id, "winspell.window");
        assert_eq!(package.exported_surface_rows(), vec![
            "module=winspell:export:fn:open".to_string(),
            "module=winspell:reexport:winspell.window".to_string(),
        ]);
    }

    #[test]
    fn build_workspace_summary_indexes_module_paths() {
        let book = lower_module_text(
            "winspell",
            "reexport winspell.window\nexport fn open() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");
        let window = lower_module_text(
            "winspell.window",
            "import std.canvas\nfn helper() -> Int:\n    return 0\n",
        )
        .expect("lowering should pass");
        let summary = build_package_summary("winspell", vec![book, window]);
        let layout = build_package_layout(
            &summary,
            BTreeMap::from([
                (
                    "winspell".to_string(),
                    Path::new("C:/repo/winspell/src/book.arc").to_path_buf(),
                ),
                (
                    "winspell.window".to_string(),
                    Path::new("C:/repo/winspell/src/window.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::from([("window".to_string(), "winspell.window".to_string())]),
        )
        .expect("layout should build");
        let package = build_workspace_package(
            Path::new("C:/repo/winspell").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            summary,
            layout,
        )
        .expect("workspace package should build");
        let workspace =
            build_workspace_summary(vec![package]).expect("workspace summary should build");

        let winspell = workspace.package("winspell").expect("package should exist");
        assert_eq!(workspace.package_count(), 1);
        assert_eq!(workspace.module_count(), 2);
        assert!(winspell.module("winspell.window").is_some());
        assert!(
            winspell
                .module_path("winspell.window")
                .expect("module path should exist")
                .ends_with("window.arc")
        );
        assert!(
            winspell
                .resolve_relative_module(&["window".to_string()])
                .is_some()
        );
    }

    #[test]
    fn derive_source_module_path_handles_root_and_nested_modules() {
        let src_dir = Path::new("C:/repo/winspell/src");
        let root = derive_source_module_path(
            "winspell",
            "book.arc",
            src_dir,
            Path::new("C:/repo/winspell/src/book.arc"),
        )
        .expect("root module should resolve");
        assert_eq!(root.relative_segments, Vec::<String>::new());
        assert_eq!(root.module_id, "winspell");

        let nested = derive_source_module_path(
            "winspell",
            "book.arc",
            src_dir,
            Path::new("C:/repo/winspell/src/render/window.arc"),
        )
        .expect("nested module should resolve");
        assert_eq!(
            nested.relative_segments,
            vec!["render".to_string(), "window".to_string()]
        );
        assert_eq!(nested.module_id, "winspell.render.window");
    }
}
