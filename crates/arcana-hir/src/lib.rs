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
    pub surface_text: String,
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
                .map(|symbol| {
                    format!(
                        "export:{}:{}",
                        symbol.kind.as_str(),
                        encode_surface_text(&symbol.surface_text)
                    )
                }),
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
pub enum HirResolvedTarget {
    Module {
        package_name: String,
        module_id: String,
    },
    Symbol {
        package_name: String,
        module_id: String,
        symbol_name: String,
    },
}

impl HirResolvedTarget {
    pub fn binding_name(&self) -> String {
        match self {
            Self::Module { module_id, .. } => module_id
                .rsplit('.')
                .next()
                .unwrap_or(module_id)
                .to_string(),
            Self::Symbol { symbol_name, .. } => symbol_name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirBindingOrigin {
    LocalSymbol,
    Directive(HirDirectiveKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedBinding {
    pub local_name: String,
    pub origin: HirBindingOrigin,
    pub target: HirResolvedTarget,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedDirective {
    pub source_module_id: String,
    pub local_name: String,
    pub kind: HirDirectiveKind,
    pub target: HirResolvedTarget,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedModule {
    pub module_id: String,
    pub bindings: BTreeMap<String, HirResolvedBinding>,
    pub directives: Vec<HirResolvedDirective>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolvedPackage {
    pub package_name: String,
    pub modules: BTreeMap<String, HirResolvedModule>,
}

impl HirResolvedPackage {
    pub fn module(&self, module_id: &str) -> Option<&HirResolvedModule> {
        self.modules.get(module_id)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HirResolvedWorkspace {
    pub packages: BTreeMap<String, HirResolvedPackage>,
}

impl HirResolvedWorkspace {
    pub fn package(&self, package_name: &str) -> Option<&HirResolvedPackage> {
        self.packages.get(package_name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirResolutionError {
    pub package_name: String,
    pub source_module_id: String,
    pub span: Span,
    pub message: String,
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
                surface_text: symbol.surface_text.clone(),
                span: symbol.span,
            })
            .collect(),
    }
}

fn encode_surface_text(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}

fn resolve_module_target(
    package: &HirWorkspacePackage,
    workspace: &HirWorkspaceSummary,
    path: &[String],
) -> Result<(String, String), String> {
    if path.is_empty() {
        return Err("missing module path".to_string());
    }

    let key = path.join(".");
    let first = &path[0];
    if first == &package.summary.package_name {
        return package
            .module(&key)
            .map(|module| (package.summary.package_name.clone(), module.module_id.clone()))
            .ok_or_else(|| format!("unresolved module `{key}`"));
    }

    if first == "std" {
        return workspace
            .package("std")
            .ok_or_else(|| "implicit package `std` is not available".to_string())
            .and_then(|std_package| {
                std_package
                    .module(&key)
                    .map(|module| (std_package.summary.package_name.clone(), module.module_id.clone()))
                    .ok_or_else(|| format!("unresolved module `{key}`"))
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
                dependency
                    .module(&key)
                    .map(|module| (dependency.summary.package_name.clone(), module.module_id.clone()))
                    .ok_or_else(|| format!("unresolved module `{key}`"))
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
        .map(|module| (package.summary.package_name.clone(), module.module_id.clone()))
        .ok_or_else(|| format!("unresolved module `{key}`"))
}

enum ResolvedUseTarget {
    Module {
        package_name: String,
        module_id: String,
    },
    Symbol {
        package_name: String,
        module_id: String,
        symbol_name: String,
    },
}

fn resolve_use_target(
    package: &HirWorkspacePackage,
    workspace: &HirWorkspaceSummary,
    path: &[String],
) -> Result<ResolvedUseTarget, String> {
    if path.is_empty() {
        return Err("missing use target".to_string());
    }

    for prefix_len in (1..=path.len()).rev() {
        let prefix = &path[..prefix_len];
        let Ok((package_name, module_id)) = resolve_module_target(package, workspace, prefix) else {
            continue;
        };
        if prefix_len == path.len() {
            return Ok(ResolvedUseTarget::Module {
                package_name,
                module_id,
            });
        }

        let suffix = &path[prefix_len..];
        if suffix.len() != 1 {
            return Err(format!(
                "nested symbol path `{}` is not supported yet",
                path.join(".")
            ));
        }

        let symbol_name = &suffix[0];
        let resolved_package = workspace
            .package(&package_name)
            .ok_or_else(|| format!("resolved package `{package_name}` is not loaded"))?;
        let resolved_module = resolved_package
            .module(&module_id)
            .ok_or_else(|| format!("resolved module `{module_id}` is not loaded"))?;
        if resolved_module.has_symbol(symbol_name) {
            return Ok(ResolvedUseTarget::Symbol {
                package_name,
                module_id,
                symbol_name: symbol_name.clone(),
            });
        }
        return Err(format!(
            "unresolved symbol `{symbol_name}` in module `{module_id}`"
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

    Err(format!("unresolved module path `{}`", path.join(".")))
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

pub fn resolve_workspace(
    workspace: &HirWorkspaceSummary,
) -> Result<HirResolvedWorkspace, Vec<HirResolutionError>> {
    let mut packages = BTreeMap::new();
    let mut errors = Vec::new();

    for package in workspace.packages.values() {
        let mut modules = BTreeMap::new();
        for module in &package.summary.modules {
            let mut bindings = BTreeMap::new();
            for symbol in &module.symbols {
                bindings.entry(symbol.name.clone()).or_insert(HirResolvedBinding {
                    local_name: symbol.name.clone(),
                    origin: HirBindingOrigin::LocalSymbol,
                    target: HirResolvedTarget::Symbol {
                        package_name: package.summary.package_name.clone(),
                        module_id: module.module_id.clone(),
                        symbol_name: symbol.name.clone(),
                    },
                    span: symbol.span,
                });
            }

            let mut directives = Vec::new();
            for directive in &module.directives {
                let target = match directive.kind {
                    HirDirectiveKind::Import | HirDirectiveKind::Reexport => {
                        resolve_module_target(package, workspace, &directive.path).map(
                            |(package_name, module_id)| HirResolvedTarget::Module {
                                package_name,
                                module_id,
                            },
                        )
                    }
                    HirDirectiveKind::Use => {
                        resolve_use_target(package, workspace, &directive.path).map(
                            |resolved_target| match resolved_target {
                                ResolvedUseTarget::Module {
                                    package_name,
                                    module_id,
                                } => HirResolvedTarget::Module {
                                    package_name,
                                    module_id,
                                },
                                ResolvedUseTarget::Symbol {
                                    package_name,
                                    module_id,
                                    symbol_name,
                                } => HirResolvedTarget::Symbol {
                                    package_name,
                                    module_id,
                                    symbol_name,
                                },
                            },
                        )
                    }
                };

                match target {
                    Ok(target) => {
                        let local_name = directive
                            .alias
                            .clone()
                            .unwrap_or_else(|| target.binding_name());
                        bindings.entry(local_name.clone()).or_insert(HirResolvedBinding {
                            local_name: local_name.clone(),
                            origin: HirBindingOrigin::Directive(directive.kind),
                            target: target.clone(),
                            span: directive.span,
                        });
                        directives.push(HirResolvedDirective {
                            source_module_id: module.module_id.clone(),
                            local_name,
                            kind: directive.kind,
                            target,
                            alias: directive.alias.clone(),
                            span: directive.span,
                        });
                    }
                    Err(message) => errors.push(HirResolutionError {
                        package_name: package.summary.package_name.clone(),
                        source_module_id: module.module_id.clone(),
                        span: directive.span,
                        message,
                    }),
                }
            }

            modules.insert(
                module.module_id.clone(),
                HirResolvedModule {
                    module_id: module.module_id.clone(),
                    bindings,
                    directives,
                },
            );
        }

        packages.insert(
            package.summary.package_name.clone(),
            HirResolvedPackage {
                package_name: package.summary.package_name.clone(),
                modules,
            },
        );
    }

    if errors.is_empty() {
        Ok(HirResolvedWorkspace { packages })
    } else {
        Err(errors)
    }
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
        build_workspace_summary, derive_source_module_path, lower_module_text, resolve_workspace,
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
            vec![
                "export:fn:fn print() -> Int:".to_string(),
                "reexport:std.result".to_string(),
            ]
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
            "module=winspell:export:fn:fn open() -> Int:".to_string(),
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
    fn resolve_workspace_builds_module_and_symbol_bindings() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text(
                    "std.io",
                    "export fn print() -> Int:\n    return 0\n",
                )
                .expect("std.io should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.io".to_string(),
                Path::new("C:/repo/std/src/io.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_package = build_workspace_package(
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import std.io\nuse std.io as io\nuse std.io.print\nexport fn main() -> Int:\n    return 0\n",
                )
                .expect("app should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_package, std_package]).expect("workspace builds");
        let resolved = resolve_workspace(&workspace).expect("resolution should succeed");
        let app = resolved.package("app").expect("app should resolve");
        let root = app.module("app").expect("root module should resolve");
        assert_eq!(root.directives.len(), 3);
        assert_eq!(
            root.bindings.get("io").expect("alias should resolve").target,
            super::HirResolvedTarget::Module {
                package_name: "std".to_string(),
                module_id: "std.io".to_string(),
            }
        );
        assert_eq!(
            root.bindings
                .get("print")
                .expect("symbol should resolve")
                .target,
            super::HirResolvedTarget::Symbol {
                package_name: "std".to_string(),
                module_id: "std.io".to_string(),
                symbol_name: "print".to_string(),
            }
        );
    }

    #[test]
    fn resolve_workspace_reports_invalid_dependencies() {
        let core_summary = build_package_summary(
            "core",
            vec![lower_module_text(
                "core",
                "export fn value() -> Int:\n    return 0\n",
            )
            .expect("core should lower")],
        );
        let core_layout = build_package_layout(
            &core_summary,
            BTreeMap::from([(
                "core".to_string(),
                Path::new("C:/repo/core/src/book.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("core layout should build");
        let core_package = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let app_summary = build_package_summary(
            "app",
            vec![lower_module_text(
                "app",
                "import core\nfn main() -> Int:\n    return 0\n",
            )
            .expect("app should lower")],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([(
                "app".to_string(),
                Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_package = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_package, core_package]).expect("workspace builds");
        let errors = resolve_workspace(&workspace).expect_err("resolution should fail");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not a direct dependency"));
        assert_eq!(errors[0].source_module_id, "app");
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
