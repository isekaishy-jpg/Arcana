use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use arcana_hir::{
    HirImplDecl, HirModule, HirModuleSummary, HirResolvedModule, HirResolvedTarget,
    HirResolvedWorkspace, HirSymbol, HirSymbolBody, HirSymbolKind, HirWorkspacePackage,
    HirWorkspaceSummary, lower_module_text, resolve_workspace,
};
use arcana_package::load_workspace_hir as load_package_workspace_hir;
use arcana_syntax::Span;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceSymbolUse {
    TypeLike,
    Trait,
}

#[derive(Clone, Debug, Default)]
struct TypeScope {
    type_params: BTreeSet<String>,
    lifetimes: BTreeSet<String>,
    assoc_types: BTreeSet<String>,
    allow_self: bool,
}

impl TypeScope {
    fn with_params(&self, params: &[String]) -> Self {
        let mut next = self.clone();
        for param in params {
            if param.starts_with('\'') {
                next.lifetimes.insert(param.clone());
            } else {
                next.type_params.insert(param.clone());
            }
        }
        next
    }

    fn with_assoc_types<I>(&self, assoc_types: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut next = self.clone();
        next.assoc_types.extend(assoc_types);
        next
    }

    fn with_self(&self) -> Self {
        let mut next = self.clone();
        next.allow_self = true;
        next
    }

    fn allows_type_name(&self, name: &str) -> bool {
        self.type_params.contains(name)
            || self.assoc_types.contains(name)
            || (self.allow_self && name == "Self")
    }

    fn lifetime_declared(&self, lifetime: &str) -> bool {
        lifetime == "'static" || self.lifetimes.contains(lifetime)
    }
}

#[derive(Clone, Debug, Default)]
struct SurfaceRefs {
    paths: Vec<Vec<String>>,
    lifetimes: Vec<String>,
}

struct ResolvedSymbolRef<'a> {
    symbol: &'a HirSymbol,
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
            validate_module_semantics(
                workspace,
                package,
                module,
                resolved_module,
                &mut diagnostics,
            );
        }
    }
    diagnostics
}

fn validate_module_semantics(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    resolved_module: &HirResolvedModule,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let module_path = package
        .module_path(&module.module_id)
        .cloned()
        .unwrap_or_else(|| package.root_dir.join("src").join("unknown.arc"));

    for lang_item in &module.lang_items {
        if lookup_symbol_path(workspace, resolved_module, lang_item.target.as_slice()).is_none() {
            diagnostics.push(Diagnostic {
                path: module_path.clone(),
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

    for symbol in &module.symbols {
        validate_symbol_surface_types(
            workspace,
            resolved_module,
            &module_path,
            symbol,
            &TypeScope::default(),
            diagnostics,
        );
    }
    for impl_decl in &module.impls {
        validate_impl_surface_types(
            workspace,
            resolved_module,
            &module_path,
            impl_decl,
            diagnostics,
        );
    }
}

fn validate_symbol_surface_types(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    inherited_scope: &TypeScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = inherited_scope.with_params(&symbol.type_params);
    for param in &symbol.params {
        validate_type_surface_text(
            workspace,
            resolved_module,
            module_path,
            &scope,
            &param.ty,
            symbol.span,
            &format!("parameter type `{}`", param.name),
            SurfaceSymbolUse::TypeLike,
            diagnostics,
        );
    }
    if let Some(return_type) = &symbol.return_type {
        validate_type_surface_text(
            workspace,
            resolved_module,
            module_path,
            &scope,
            return_type,
            symbol.span,
            "return type",
            SurfaceSymbolUse::TypeLike,
            diagnostics,
        );
    }
    if let Some(where_clause) = &symbol.where_clause {
        validate_type_surface_text(
            workspace,
            resolved_module,
            module_path,
            &scope,
            where_clause,
            symbol.span,
            "where clause",
            SurfaceSymbolUse::TypeLike,
            diagnostics,
        );
    }

    match &symbol.body {
        HirSymbolBody::None => {}
        HirSymbolBody::Record { fields } => {
            for field in fields {
                validate_type_surface_text(
                    workspace,
                    resolved_module,
                    module_path,
                    &scope,
                    &field.ty,
                    field.span,
                    &format!("field type `{}`", field.name),
                    SurfaceSymbolUse::TypeLike,
                    diagnostics,
                );
            }
        }
        HirSymbolBody::Enum { variants } => {
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    validate_type_surface_text(
                        workspace,
                        resolved_module,
                        module_path,
                        &scope,
                        payload,
                        variant.span,
                        &format!("enum variant payload `{}`", variant.name),
                        SurfaceSymbolUse::TypeLike,
                        diagnostics,
                    );
                }
            }
        }
        HirSymbolBody::Trait {
            assoc_types,
            methods,
        } => {
            let trait_scope = scope
                .with_assoc_types(assoc_types.iter().map(|assoc_type| assoc_type.name.clone()))
                .with_self();
            for assoc_type in assoc_types {
                if let Some(default_ty) = &assoc_type.default_ty {
                    validate_type_surface_text(
                        workspace,
                        resolved_module,
                        module_path,
                        &trait_scope,
                        default_ty,
                        assoc_type.span,
                        &format!("associated type default `{}`", assoc_type.name),
                        SurfaceSymbolUse::TypeLike,
                        diagnostics,
                    );
                }
            }
            for method in methods {
                validate_symbol_surface_types(
                    workspace,
                    resolved_module,
                    module_path,
                    method,
                    &trait_scope,
                    diagnostics,
                );
            }
        }
    }
}

fn validate_impl_surface_types(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    impl_decl: &HirImplDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let scope = TypeScope::default()
        .with_params(&impl_decl.type_params)
        .with_assoc_types(
            impl_decl
                .assoc_types
                .iter()
                .map(|assoc_type| assoc_type.name.clone()),
        )
        .with_self();

    if let Some(trait_path) = &impl_decl.trait_path {
        validate_type_surface_text(
            workspace,
            resolved_module,
            module_path,
            &scope,
            trait_path,
            impl_decl.span,
            "impl trait path",
            SurfaceSymbolUse::Trait,
            diagnostics,
        );
    }
    validate_type_surface_text(
        workspace,
        resolved_module,
        module_path,
        &scope,
        &impl_decl.target_type,
        impl_decl.span,
        "impl target type",
        SurfaceSymbolUse::TypeLike,
        diagnostics,
    );
    for assoc_type in &impl_decl.assoc_types {
        if let Some(value_ty) = &assoc_type.value_ty {
            validate_type_surface_text(
                workspace,
                resolved_module,
                module_path,
                &scope,
                value_ty,
                assoc_type.span,
                &format!("associated type binding `{}`", assoc_type.name),
                SurfaceSymbolUse::TypeLike,
                diagnostics,
            );
        }
    }
    for method in &impl_decl.methods {
        validate_symbol_surface_types(
            workspace,
            resolved_module,
            module_path,
            method,
            &scope,
            diagnostics,
        );
    }
}

fn validate_type_surface_text(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &TypeScope,
    text: &str,
    span: Span,
    context: &str,
    expected_use: SurfaceSymbolUse,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let refs = collect_surface_refs(text);
    let mut seen_lifetimes = BTreeSet::new();
    for lifetime in refs.lifetimes {
        if seen_lifetimes.insert(lifetime.clone()) && !scope.lifetime_declared(&lifetime) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: span.line,
                column: span.column,
                message: format!("undeclared lifetime `{lifetime}` in {context}"),
            });
        }
    }

    let mut seen_paths = BTreeSet::new();
    for (path_index, path) in refs.paths.into_iter().enumerate() {
        let path_key = path.join(".");
        if !seen_paths.insert(path_key.clone()) {
            continue;
        }
        let path_use = if expected_use == SurfaceSymbolUse::Trait && path_index == 0 {
            SurfaceSymbolUse::Trait
        } else {
            SurfaceSymbolUse::TypeLike
        };
        if path.len() == 1 && scope.allows_type_name(&path[0]) {
            continue;
        }
        if path.len() == 1 && is_builtin_type_name(&path[0]) {
            continue;
        }
        let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path) else {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: span.line,
                column: span.column,
                message: format!("unresolved type reference `{path_key}` in {context}"),
            });
            continue;
        };
        if !symbol_matches_surface_use(symbol_ref.symbol.kind, path_use) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: span.line,
                column: span.column,
                message: format!(
                    "`{path_key}` does not resolve to a valid {} in {context}",
                    surface_use_name(path_use)
                ),
            });
        }
    }
}

fn lookup_symbol_path<'a>(
    workspace: &'a HirWorkspaceSummary,
    module: &HirResolvedModule,
    path: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return module
            .bindings
            .get(&path[0])
            .and_then(|binding| lookup_target_symbol_tail(workspace, &binding.target, &[]));
    }

    let first = &path[0];
    if let Some(binding) = module.bindings.get(first) {
        return lookup_target_symbol_tail(workspace, &binding.target, &path[1..]);
    }

    let package = workspace.package(first)?;
    lookup_package_symbol_path(package, &path[1..])
}

fn lookup_target_symbol_tail<'a>(
    workspace: &'a HirWorkspaceSummary,
    target: &HirResolvedTarget,
    tail: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    match target {
        HirResolvedTarget::Symbol {
            package_name,
            module_id,
            symbol_name,
        } => {
            if !tail.is_empty() {
                return None;
            }
            let package = workspace.package(package_name)?;
            let module = package.module(module_id)?;
            let symbol = module
                .symbols
                .iter()
                .find(|symbol| symbol.name == *symbol_name)?;
            Some(ResolvedSymbolRef { symbol })
        }
        HirResolvedTarget::Module {
            package_name,
            module_id,
        } => {
            let package = workspace.package(package_name)?;
            let module = package.module(module_id)?;
            lookup_module_symbol_path(package, module, tail)
        }
    }
}

fn lookup_package_symbol_path<'a>(
    package: &'a HirWorkspacePackage,
    path: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    let (symbol_name, module_path) = path.split_last()?;
    if symbol_name.is_empty() {
        return None;
    }
    let module = if module_path.is_empty() {
        package.module(&package.summary.package_name)
    } else {
        package.resolve_relative_module(module_path)
    }?;
    let symbol = module
        .symbols
        .iter()
        .find(|symbol| symbol.name == *symbol_name)?;
    Some(ResolvedSymbolRef { symbol })
}

fn lookup_module_symbol_path<'a>(
    package: &'a HirWorkspacePackage,
    module: &'a HirModuleSummary,
    path: &[String],
) -> Option<ResolvedSymbolRef<'a>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        let symbol = module
            .symbols
            .iter()
            .find(|symbol| symbol.name == path[0])?;
        return Some(ResolvedSymbolRef { symbol });
    }
    let (symbol_name, module_tail) = path.split_last()?;
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut target_relative = base_relative;
    target_relative.extend_from_slice(module_tail);
    let target_module = package.resolve_relative_module(&target_relative)?;
    let symbol = target_module
        .symbols
        .iter()
        .find(|symbol| symbol.name == *symbol_name)?;
    Some(ResolvedSymbolRef { symbol })
}

fn symbol_matches_surface_use(kind: HirSymbolKind, expected_use: SurfaceSymbolUse) -> bool {
    match expected_use {
        SurfaceSymbolUse::TypeLike => {
            matches!(
                kind,
                HirSymbolKind::Record | HirSymbolKind::Enum | HirSymbolKind::Trait
            )
        }
        SurfaceSymbolUse::Trait => kind == HirSymbolKind::Trait,
    }
}

fn surface_use_name(expected_use: SurfaceSymbolUse) -> &'static str {
    match expected_use {
        SurfaceSymbolUse::TypeLike => "type",
        SurfaceSymbolUse::Trait => "trait",
    }
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "Int"
            | "Str"
            | "Bool"
            | "RangeInt"
            | "List"
            | "Array"
            | "Map"
            | "Arena"
            | "ArenaId"
            | "FrameArena"
            | "FrameId"
            | "PoolArena"
            | "PoolId"
            | "Task"
            | "Thread"
            | "Channel"
            | "Mutex"
            | "AtomicInt"
            | "AtomicBool"
            | "Window"
            | "Image"
    )
}

fn collect_surface_refs(text: &str) -> SurfaceRefs {
    let chars = text.chars().collect::<Vec<_>>();
    let mut refs = SurfaceRefs::default();
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if ch == '\'' {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            if index > start + 1 {
                refs.lifetimes
                    .push(chars[start..index].iter().collect::<String>());
            }
            continue;
        }
        if is_ident_start(ch) {
            if is_projection_tail(&chars, index) {
                index += 1;
                while index < chars.len() && is_ident_continue(chars[index]) {
                    index += 1;
                }
                continue;
            }

            let mut segments = Vec::new();
            loop {
                let start = index;
                index += 1;
                while index < chars.len() && is_ident_continue(chars[index]) {
                    index += 1;
                }
                let segment = chars[start..index].iter().collect::<String>();
                if is_surface_keyword(&segment) {
                    segments.clear();
                    break;
                }
                segments.push(segment);

                let Some(dot_idx) = next_non_ws_index(&chars, index) else {
                    break;
                };
                if chars[dot_idx] != '.' {
                    break;
                }
                let Some(next_idx) = next_non_ws_index(&chars, dot_idx + 1) else {
                    break;
                };
                if !is_ident_start(chars[next_idx]) {
                    break;
                }
                index = next_idx;
            }

            if !segments.is_empty() {
                refs.paths.push(segments);
            }
            continue;
        }
        index += 1;
    }

    refs
}

fn is_projection_tail(chars: &[char], index: usize) -> bool {
    let Some(dot_idx) = previous_non_ws_index(chars, index) else {
        return false;
    };
    if chars[dot_idx] != '.' {
        return false;
    }
    let Some(owner_idx) = previous_non_ws_index(chars, dot_idx) else {
        return false;
    };
    matches!(chars[owner_idx], ']' | ')')
}

fn previous_non_ws_index(chars: &[char], before: usize) -> Option<usize> {
    let mut index = before;
    while index > 0 {
        index -= 1;
        if !chars[index].is_whitespace() {
            return Some(index);
        }
    }
    None
}

fn next_non_ws_index(chars: &[char], start: usize) -> Option<usize> {
    let mut index = start;
    while index < chars.len() {
        if !chars[index].is_whitespace() {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_surface_keyword(token: &str) -> bool {
    matches!(token, "mut" | "where")
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
    fn check_path_rejects_unresolved_type_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_type_ref"),
        )
        .expect_err("unresolved type package should fail");
        assert!(
            err.contains("unresolved type reference `MissingType` in field type `value`"),
            "{err}"
        );
    }

    #[test]
    fn check_path_rejects_undeclared_lifetime_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("undeclared_lifetime"),
        )
        .expect_err("undeclared lifetime package should fail");
        assert!(
            err.contains("undeclared lifetime `'a` in parameter type `value`"),
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
