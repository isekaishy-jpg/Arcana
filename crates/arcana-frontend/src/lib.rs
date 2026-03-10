use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use arcana_hir::{
    HirAssignTarget, HirChainStep, HirExpr, HirHeaderAttachment, HirImplDecl, HirMatchPattern,
    HirModule, HirModuleSummary, HirResolvedModule, HirResolvedTarget, HirResolvedWorkspace,
    HirStatement, HirStatementKind, HirSymbol, HirSymbolBody, HirSymbolKind, HirWorkspacePackage,
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
struct ValueScope {
    locals: BTreeSet<String>,
}

impl ValueScope {
    fn with_params<'a, I>(&self, params: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut next = self.clone();
        for param in params {
            next.locals.insert(param.to_string());
        }
        next
    }

    fn with_local(&self, name: &str) -> Self {
        let mut next = self.clone();
        next.locals.insert(name.to_string());
        next
    }

    fn insert(&mut self, name: &str) {
        self.locals.insert(name.to_string());
    }

    fn contains(&self, name: &str) -> bool {
        self.locals.contains(name)
    }
}

#[derive(Clone, Debug, Default)]
struct SurfaceRefs {
    paths: Vec<Vec<String>>,
    lifetimes: Vec<String>,
}

struct ResolvedSymbolRef<'a> {
    package_name: &'a str,
    module_id: &'a str,
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
                resolved,
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
    resolved_workspace: &HirResolvedWorkspace,
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
        let symbol_scope = TypeScope::default().with_params(&symbol.type_params);
        validate_boundary_symbol_contract(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            symbol,
            &symbol_scope,
            None,
            &BTreeMap::new(),
            diagnostics,
        );
        validate_symbol_value_semantics(
            workspace,
            resolved_module,
            &module_path,
            symbol,
            &TypeScope::default(),
            &ValueScope::default(),
            diagnostics,
        );
    }
    for impl_decl in &module.impls {
        validate_impl_surface_types(
            workspace,
            resolved_workspace,
            resolved_module,
            &module_path,
            impl_decl,
            diagnostics,
        );
        validate_impl_value_semantics(
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
    resolved_workspace: &HirResolvedWorkspace,
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
    let assoc_bindings = impl_decl
        .assoc_types
        .iter()
        .filter_map(|assoc_type| {
            assoc_type
                .value_ty
                .as_ref()
                .map(|value_ty| (assoc_type.name.clone(), value_ty.clone()))
        })
        .collect::<BTreeMap<_, _>>();

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
        let method_scope = scope.with_params(&method.type_params);
        validate_boundary_symbol_contract(
            workspace,
            resolved_workspace,
            resolved_module,
            module_path,
            method,
            &method_scope,
            Some(&impl_decl.target_type),
            &assoc_bindings,
            diagnostics,
        );
    }
}

fn validate_boundary_symbol_contract(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    scope: &TypeScope,
    self_type: Option<&str>,
    assoc_bindings: &BTreeMap<String, String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(target) = boundary_target_from_forewords(&symbol.forewords) else {
        return;
    };

    for param in &symbol.params {
        let mut visited = BTreeSet::new();
        if !boundary_type_is_safe(
            workspace,
            resolved_workspace,
            resolved_module,
            scope,
            &param.ty,
            self_type,
            assoc_bindings,
            &mut visited,
        ) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: symbol.span.line,
                column: symbol.span.column,
                message: format!(
                    "type `{}` is not boundary-safe for target `{target}`",
                    param.ty
                ),
            });
        }
    }

    if let Some(return_type) = &symbol.return_type {
        let mut visited = BTreeSet::new();
        if !boundary_type_is_safe(
            workspace,
            resolved_workspace,
            resolved_module,
            scope,
            return_type,
            self_type,
            assoc_bindings,
            &mut visited,
        ) {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: symbol.span.line,
                column: symbol.span.column,
                message: format!(
                    "type `{}` is not boundary-safe for target `{target}`",
                    return_type
                ),
            });
        }
    }
}

fn boundary_target_from_forewords(forewords: &[arcana_hir::HirForewordApp]) -> Option<String> {
    forewords
        .iter()
        .find(|foreword| foreword.name == "boundary")
        .and_then(|foreword| {
            foreword
                .args
                .iter()
                .find(|arg| arg.name.as_deref() == Some("target"))
        })
        .and_then(|arg| parse_symbol_or_string_literal(&arg.value))
}

fn boundary_type_is_safe(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    text: &str,
    self_type: Option<&str>,
    assoc_bindings: &BTreeMap<String, String>,
    visited_symbols: &mut BTreeSet<String>,
) -> bool {
    for path in collect_surface_refs(text).paths {
        if path.len() == 1 {
            let name = &path[0];
            if name == "Self" {
                if let Some(self_type) = self_type {
                    if !boundary_type_is_safe(
                        workspace,
                        resolved_workspace,
                        resolved_module,
                        scope,
                        self_type,
                        None,
                        assoc_bindings,
                        visited_symbols,
                    ) {
                        return false;
                    }
                }
                continue;
            }
            if let Some(value_ty) = assoc_bindings.get(name) {
                if !boundary_type_is_safe(
                    workspace,
                    resolved_workspace,
                    resolved_module,
                    scope,
                    value_ty,
                    self_type,
                    assoc_bindings,
                    visited_symbols,
                ) {
                    return false;
                }
                continue;
            }
            if scope.allows_type_name(name) || is_boundary_safe_builtin_name(name) {
                continue;
            }
            if is_boundary_unsafe_builtin_name(name) {
                return false;
            }
        }

        let Some(symbol_ref) = lookup_symbol_path(workspace, resolved_module, &path) else {
            continue;
        };
        if !boundary_symbol_is_safe(
            workspace,
            resolved_workspace,
            resolved_module,
            scope,
            &symbol_ref,
            self_type,
            assoc_bindings,
            visited_symbols,
        ) {
            return false;
        }
    }
    true
}

fn boundary_symbol_is_safe(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    resolved_module: &HirResolvedModule,
    scope: &TypeScope,
    symbol_ref: &ResolvedSymbolRef<'_>,
    self_type: Option<&str>,
    assoc_bindings: &BTreeMap<String, String>,
    visited_symbols: &mut BTreeSet<String>,
) -> bool {
    let visit_key = format!(
        "{}::{}::{}",
        symbol_ref.package_name, symbol_ref.module_id, symbol_ref.symbol.name
    );
    if !visited_symbols.insert(visit_key) {
        return true;
    }

    let nested_scope = TypeScope::default().with_params(&symbol_ref.symbol.type_params);
    let owner_module = resolved_workspace
        .package(symbol_ref.package_name)
        .and_then(|package| package.module(symbol_ref.module_id))
        .unwrap_or(resolved_module);
    match &symbol_ref.symbol.body {
        HirSymbolBody::Record { fields } => fields.iter().all(|field| {
            boundary_type_is_safe(
                workspace,
                resolved_workspace,
                owner_module,
                &nested_scope,
                &field.ty,
                self_type,
                assoc_bindings,
                visited_symbols,
            )
        }),
        HirSymbolBody::Enum { variants } => variants.iter().all(|variant| {
            variant.payload.as_ref().is_none_or(|payload| {
                boundary_type_is_safe(
                    workspace,
                    resolved_workspace,
                    owner_module,
                    &nested_scope,
                    payload,
                    self_type,
                    assoc_bindings,
                    visited_symbols,
                )
            })
        }),
        _ => scope.allows_type_name(&symbol_ref.symbol.name),
    }
}

fn is_boundary_safe_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "Int" | "Bool" | "Str" | "Unit" | "List" | "Array" | "Map"
    )
}

fn is_boundary_unsafe_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "Task"
            | "Thread"
            | "Channel"
            | "Mutex"
            | "Arena"
            | "ArenaId"
            | "FrameArena"
            | "FrameId"
            | "PoolArena"
            | "PoolId"
            | "RangeInt"
            | "Window"
            | "Image"
            | "AudioDevice"
            | "AudioBuffer"
            | "AudioPlayback"
            | "AtomicInt"
            | "AtomicBool"
    )
}

fn parse_symbol_or_string_literal(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if let Some(unquoted) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        return Some(unquoted.to_string());
    }
    split_simple_path(trimmed)
        .filter(|path| path.len() == 1)
        .map(|path| path[0].clone())
}

fn validate_symbol_value_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    symbol: &HirSymbol,
    inherited_type_scope: &TypeScope,
    inherited_scope: &ValueScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let type_scope = inherited_type_scope.with_params(&symbol.type_params);
    let mut scope =
        inherited_scope.with_params(symbol.params.iter().map(|param| param.name.as_str()));
    validate_rollup_handlers(
        workspace,
        resolved_module,
        module_path,
        &symbol.rollups,
        diagnostics,
    );
    validate_statement_block_semantics(
        workspace,
        resolved_module,
        module_path,
        &symbol.statements,
        &type_scope,
        &mut scope,
        diagnostics,
    );

    if let HirSymbolBody::Trait {
        assoc_types,
        methods,
    } = &symbol.body
    {
        let trait_scope = type_scope
            .with_assoc_types(assoc_types.iter().map(|assoc_type| assoc_type.name.clone()))
            .with_self();
        for method in methods {
            validate_symbol_value_semantics(
                workspace,
                resolved_module,
                module_path,
                method,
                &trait_scope,
                &ValueScope::default(),
                diagnostics,
            );
        }
    }
}

fn validate_impl_value_semantics(
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
    for method in &impl_decl.methods {
        validate_symbol_value_semantics(
            workspace,
            resolved_module,
            module_path,
            method,
            &scope,
            &ValueScope::default(),
            diagnostics,
        );
    }
}

fn validate_rollup_handlers(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    rollups: &[arcana_hir::HirPageRollup],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for rollup in rollups {
        if lookup_symbol_path(workspace, resolved_module, &rollup.handler_path).is_none() {
            diagnostics.push(Diagnostic {
                path: module_path.to_path_buf(),
                line: rollup.span.line,
                column: rollup.span.column,
                message: format!(
                    "unresolved page rollup handler `{}`",
                    rollup.handler_path.join(".")
                ),
            });
        }
    }
}

fn validate_statement_block_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    statements: &[HirStatement],
    type_scope: &TypeScope,
    scope: &mut ValueScope,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in statements {
        validate_rollup_handlers(
            workspace,
            resolved_module,
            module_path,
            &statement.rollups,
            diagnostics,
        );
        match &statement.kind {
            HirStatementKind::Let { name, value, .. } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value,
                    statement.span,
                    diagnostics,
                );
                scope.insert(name);
            }
            HirStatementKind::Return { value } => {
                if let Some(value) = value {
                    validate_expr_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        type_scope,
                        scope,
                        value,
                        statement.span,
                        diagnostics,
                    );
                }
            }
            HirStatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    diagnostics,
                );
                let mut then_scope = scope.clone();
                validate_statement_block_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    then_branch,
                    type_scope,
                    &mut then_scope,
                    diagnostics,
                );
                if let Some(else_branch) = else_branch {
                    let mut else_scope = scope.clone();
                    validate_statement_block_semantics(
                        workspace,
                        resolved_module,
                        module_path,
                        else_branch,
                        type_scope,
                        &mut else_scope,
                        diagnostics,
                    );
                }
            }
            HirStatementKind::While { condition, body } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    condition,
                    statement.span,
                    diagnostics,
                );
                let mut body_scope = scope.clone();
                validate_statement_block_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    body,
                    type_scope,
                    &mut body_scope,
                    diagnostics,
                );
            }
            HirStatementKind::For {
                binding,
                iterable,
                body,
            } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    iterable,
                    statement.span,
                    diagnostics,
                );
                let mut body_scope = scope.with_local(binding);
                validate_statement_block_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    body,
                    type_scope,
                    &mut body_scope,
                    diagnostics,
                );
            }
            HirStatementKind::Defer { expr } | HirStatementKind::Expr { expr } => {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    expr,
                    statement.span,
                    diagnostics,
                );
            }
            HirStatementKind::Assign { target, value, .. } => {
                validate_assign_target_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    target,
                    statement.span,
                    diagnostics,
                );
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    value,
                    statement.span,
                    diagnostics,
                );
            }
            HirStatementKind::Break | HirStatementKind::Continue => {}
        }
    }
}

fn validate_assign_target_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    target: &HirAssignTarget,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match target {
        HirAssignTarget::Opaque { text } => validate_opaque_value_text(
            workspace,
            resolved_module,
            module_path,
            scope,
            text,
            span,
            "assignment target",
            diagnostics,
        ),
        HirAssignTarget::Name { text } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            &[text.clone()],
            span,
            "assignment target",
            diagnostics,
        ),
        target @ HirAssignTarget::MemberAccess {
            target: inner_target,
            ..
        } => {
            if let Some(path) = flatten_assign_target_path(target) {
                if should_resolve_member_path_as_namespace(workspace, resolved_module, scope, &path)
                {
                    validate_value_path_segments(
                        workspace,
                        resolved_module,
                        module_path,
                        scope,
                        &path,
                        span,
                        "assignment target",
                        diagnostics,
                    );
                    return;
                }
            }
            validate_assign_target_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                inner_target,
                span,
                diagnostics,
            );
        }
        HirAssignTarget::Index { target, index } => {
            validate_assign_target_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                target,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                index,
                span,
                diagnostics,
            );
        }
    }
}

fn validate_expr_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    expr: &HirExpr,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Opaque { text, .. } => validate_opaque_value_text(
            workspace,
            resolved_module,
            module_path,
            scope,
            text,
            span,
            "value expression",
            diagnostics,
        ),
        HirExpr::Path { segments } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            segments,
            span,
            "value expression",
            diagnostics,
        ),
        HirExpr::BoolLiteral { .. } | HirExpr::IntLiteral { .. } | HirExpr::StrLiteral { .. } => {}
        HirExpr::Pair { left, right } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                left,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                right,
                span,
                diagnostics,
            );
        }
        HirExpr::CollectionLiteral { items } => {
            for item in items {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    item,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::Match { subject, arms } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                diagnostics,
            );
            for arm in arms {
                let mut arm_scope = scope.clone();
                for pattern in &arm.patterns {
                    validate_match_pattern_semantics(pattern, &mut arm_scope);
                }
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    &arm_scope,
                    &arm.value,
                    arm.span,
                    diagnostics,
                );
            }
        }
        HirExpr::Chain { steps, .. } => {
            for step in steps {
                validate_chain_step_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    step,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::MemoryPhrase {
            arena,
            init_args,
            constructor,
            attached,
            ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                arena,
                span,
                diagnostics,
            );
            for arg in init_args {
                validate_phrase_arg_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    arg,
                    span,
                    diagnostics,
                );
            }
            if let Some(path) = split_simple_path(constructor) {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    &format!("memory constructor `{constructor}`"),
                    diagnostics,
                );
            }
            for attachment in attached {
                validate_header_attachment_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    attachment,
                    diagnostics,
                );
            }
        }
        HirExpr::GenericApply { expr, type_args } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            for type_arg in type_args {
                validate_type_surface_text(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    type_arg,
                    span,
                    &format!("expression generic argument `{type_arg}`"),
                    SurfaceSymbolUse::TypeLike,
                    diagnostics,
                );
            }
        }
        HirExpr::QualifiedPhrase {
            subject,
            args,
            attached,
            ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                subject,
                span,
                diagnostics,
            );
            for arg in args {
                validate_phrase_arg_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    arg,
                    span,
                    diagnostics,
                );
            }
            for attachment in attached {
                validate_header_attachment_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    attachment,
                    diagnostics,
                );
            }
        }
        HirExpr::Await { expr } | HirExpr::Unary { expr, .. } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expr,
            span,
            diagnostics,
        ),
        HirExpr::Binary { left, right, .. } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                left,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                right,
                span,
                diagnostics,
            );
        }
        member_expr @ HirExpr::MemberAccess { expr, .. } => {
            if let Some(path) = flatten_member_expr_path(member_expr) {
                if should_resolve_member_path_as_namespace(workspace, resolved_module, scope, &path)
                {
                    validate_value_path_segments(
                        workspace,
                        resolved_module,
                        module_path,
                        scope,
                        &path,
                        span,
                        "value expression",
                        diagnostics,
                    );
                    return;
                }
            }
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
        }
        HirExpr::Index { expr, index } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                index,
                span,
                diagnostics,
            );
        }
        HirExpr::Slice {
            expr, start, end, ..
        } => {
            validate_expr_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                diagnostics,
            );
            if let Some(start) = start {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::Range { start, end, .. } => {
            if let Some(start) = start {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    start,
                    span,
                    diagnostics,
                );
            }
            if let Some(end) = end {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    end,
                    span,
                    diagnostics,
                );
            }
        }
    }
}

fn validate_phrase_arg_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    arg: &arcana_hir::HirPhraseArg,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match arg {
        arcana_hir::HirPhraseArg::Positional(expr)
        | arcana_hir::HirPhraseArg::Named { value: expr, .. } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            expr,
            span,
            diagnostics,
        ),
    }
}

fn validate_chain_step_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    step: &HirChainStep,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_chain_stage_semantics(
        workspace,
        resolved_module,
        module_path,
        type_scope,
        scope,
        &step.stage,
        span,
        &step.text,
        diagnostics,
    );
    for arg in &step.bind_args {
        validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            arg,
            span,
            diagnostics,
        );
    }
}

fn validate_chain_stage_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    stage: &HirExpr,
    span: Span,
    text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match stage {
        HirExpr::Path { segments } => validate_value_path_segments(
            workspace,
            resolved_module,
            module_path,
            scope,
            segments,
            span,
            &format!("chain step `{text}`"),
            diagnostics,
        ),
        stage @ HirExpr::MemberAccess { .. } => {
            if let Some(path) = flatten_member_expr_path(stage) {
                validate_value_path_segments(
                    workspace,
                    resolved_module,
                    module_path,
                    scope,
                    &path,
                    span,
                    &format!("chain step `{text}`"),
                    diagnostics,
                );
            } else {
                validate_expr_semantics(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    scope,
                    stage,
                    span,
                    diagnostics,
                );
            }
        }
        HirExpr::GenericApply { expr, type_args } => {
            validate_chain_stage_semantics(
                workspace,
                resolved_module,
                module_path,
                type_scope,
                scope,
                expr,
                span,
                text,
                diagnostics,
            );
            for type_arg in type_args {
                validate_type_surface_text(
                    workspace,
                    resolved_module,
                    module_path,
                    type_scope,
                    type_arg,
                    span,
                    &format!("chain step generic argument `{type_arg}` in `{text}`"),
                    SurfaceSymbolUse::TypeLike,
                    diagnostics,
                );
            }
        }
        _ => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            stage,
            span,
            diagnostics,
        ),
    }
}

fn validate_header_attachment_semantics(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    type_scope: &TypeScope,
    scope: &ValueScope,
    attachment: &HirHeaderAttachment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match attachment {
        HirHeaderAttachment::Named { value, span, .. }
        | HirHeaderAttachment::Chain {
            expr: value, span, ..
        } => validate_expr_semantics(
            workspace,
            resolved_module,
            module_path,
            type_scope,
            scope,
            value,
            *span,
            diagnostics,
        ),
    }
}

fn validate_match_pattern_semantics(pattern: &HirMatchPattern, scope: &mut ValueScope) {
    match pattern {
        HirMatchPattern::Wildcard
        | HirMatchPattern::Literal { .. }
        | HirMatchPattern::Opaque { .. } => {}
        HirMatchPattern::Name { text } => {
            let is_binding = match split_simple_path(text) {
                Some(path) => path.len() == 1,
                None => true,
            };
            if is_binding {
                scope.insert(text.trim());
            }
        }
        HirMatchPattern::Variant { args, .. } => {
            for arg in args {
                validate_match_pattern_semantics(arg, scope);
            }
        }
    }
}

fn validate_opaque_value_text(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &ValueScope,
    text: &str,
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if matches!(text.trim(), "true" | "false") {
        return;
    }
    let Some(path) = split_simple_path(text) else {
        return;
    };
    validate_value_path_segments(
        workspace,
        resolved_module,
        module_path,
        scope,
        &path,
        span,
        context,
        diagnostics,
    );
}

fn validate_value_path_segments(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_path: &Path,
    scope: &ValueScope,
    path: &[String],
    span: Span,
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if path.len() == 1 && scope.contains(&path[0]) {
        return;
    }
    if value_path_exists(workspace, resolved_module, path) {
        return;
    }
    diagnostics.push(Diagnostic {
        path: module_path.to_path_buf(),
        line: span.line,
        column: span.column,
        message: format!(
            "unresolved value reference `{}` in {context}",
            path.join(".")
        ),
    });
}

fn value_path_exists(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    path: &[String],
) -> bool {
    if path.is_empty() {
        return false;
    }
    if let Some(binding) = resolved_module.bindings.get(&path[0]) {
        if target_path_exists(workspace, &binding.target, &path[1..]) {
            return true;
        }
    }
    let Some(package) = workspace.package(&path[0]) else {
        let Some(package_name) = resolved_module.module_id.split('.').next() else {
            return false;
        };
        let Some(package) = workspace.package(package_name) else {
            return false;
        };
        return package_path_exists(package, path);
    };
    package_path_exists(package, &path[1..])
}

fn target_path_exists(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
    tail: &[String],
) -> bool {
    match target {
        HirResolvedTarget::Symbol {
            package_name,
            module_id,
            symbol_name,
        } => {
            let Some(package) = workspace.package(package_name) else {
                return false;
            };
            let Some(module) = package.module(module_id) else {
                return false;
            };
            let Some(symbol) = module
                .symbols
                .iter()
                .find(|symbol| symbol.name == *symbol_name)
            else {
                return false;
            };
            symbol_tail_exists(symbol, tail)
        }
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
            module_path_exists(package, module, tail)
        }
    }
}

fn package_path_exists(package: &HirWorkspacePackage, path: &[String]) -> bool {
    if path.is_empty() {
        return true;
    }
    let Some(module) = package.module(&package.summary.package_name) else {
        return false;
    };
    module_path_exists(package, module, path)
}

fn module_path_exists(
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
    path: &[String],
) -> bool {
    if path.is_empty() {
        return true;
    }
    if module_value_member_exists(module, &path[0], &path[1..]) {
        return true;
    }
    let base_relative = module
        .module_id
        .split('.')
        .skip(1)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut module_candidate = base_relative.clone();
    module_candidate.extend_from_slice(path);
    if package.resolve_relative_module(&module_candidate).is_some() {
        return true;
    }
    for split_index in 1..path.len() {
        let mut symbol_module_path = base_relative.clone();
        symbol_module_path.extend_from_slice(&path[..split_index]);
        let Some(target_module) = package.resolve_relative_module(&symbol_module_path) else {
            continue;
        };
        if module_value_member_exists(target_module, &path[split_index], &path[split_index + 1..]) {
            return true;
        }
    }
    false
}

fn module_value_member_exists(module: &HirModuleSummary, member: &str, tail: &[String]) -> bool {
    if let Some(symbol) = module.symbols.iter().find(|symbol| symbol.name == member) {
        if symbol_tail_exists(symbol, tail) {
            return true;
        }
    }
    tail.is_empty()
        && module
            .impls
            .iter()
            .flat_map(|impl_decl| impl_decl.methods.iter())
            .any(|method| method.name == member)
}

fn symbol_tail_exists(symbol: &HirSymbol, tail: &[String]) -> bool {
    if tail.is_empty() {
        return true;
    }
    match &symbol.body {
        HirSymbolBody::Enum { variants } => {
            tail.len() == 1 && variants.iter().any(|variant| variant.name == tail[0])
        }
        _ => false,
    }
}

fn should_resolve_member_path_as_namespace(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    scope: &ValueScope,
    path: &[String],
) -> bool {
    if path.len() < 2 || scope.contains(&path[0]) {
        return false;
    }
    if let Some(binding) = resolved_module.bindings.get(&path[0]) {
        return match &binding.target {
            HirResolvedTarget::Module { .. } => true,
            HirResolvedTarget::Symbol { .. } => {
                target_supports_member_namespace(workspace, &binding.target)
            }
        };
    }
    if workspace.package(&path[0]).is_some() {
        return true;
    }
    let Some(package_name) = resolved_module.module_id.split('.').next() else {
        return false;
    };
    let Some(package) = workspace.package(package_name) else {
        return false;
    };
    package
        .resolve_relative_module(&[path[0].clone()])
        .is_some()
}

fn target_supports_member_namespace(
    workspace: &HirWorkspaceSummary,
    target: &HirResolvedTarget,
) -> bool {
    match target {
        HirResolvedTarget::Module { .. } => true,
        HirResolvedTarget::Symbol {
            package_name,
            module_id,
            symbol_name,
        } => workspace
            .package(package_name)
            .and_then(|package| package.module(module_id))
            .and_then(|module| {
                module
                    .symbols
                    .iter()
                    .find(|symbol| symbol.name == *symbol_name)
            })
            .map(|symbol| matches!(symbol.body, HirSymbolBody::Enum { .. }))
            .unwrap_or(false),
    }
}

fn flatten_member_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::Path { segments } => Some(segments.clone()),
        HirExpr::MemberAccess { expr, member } if is_identifier_text(member) => {
            let mut path = flatten_member_expr_path(expr)?;
            path.push(member.clone());
            Some(path)
        }
        _ => None,
    }
}

fn flatten_assign_target_path(target: &HirAssignTarget) -> Option<Vec<String>> {
    match target {
        HirAssignTarget::Name { text } => Some(vec![text.clone()]),
        HirAssignTarget::MemberAccess { target, member } if is_identifier_text(member) => {
            let mut path = flatten_assign_target_path(target)?;
            path.push(member.clone());
            Some(path)
        }
        _ => None,
    }
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_ident_start(first) && chars.all(is_ident_continue)
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
    module: &'a HirResolvedModule,
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

    if let Some(package) = workspace.package(first) {
        return lookup_package_symbol_path(package, &path[1..]);
    }

    let package_name = module.module_id.split('.').next()?;
    let package = workspace.package(package_name)?;
    lookup_package_symbol_path(package, path)
}

fn lookup_target_symbol_tail<'a>(
    workspace: &'a HirWorkspaceSummary,
    target: &'a HirResolvedTarget,
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
            Some(ResolvedSymbolRef {
                package_name,
                module_id,
                symbol,
            })
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
    Some(ResolvedSymbolRef {
        package_name: &package.summary.package_name,
        module_id: &module.module_id,
        symbol,
    })
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
        return Some(ResolvedSymbolRef {
            package_name: &package.summary.package_name,
            module_id: &module.module_id,
            symbol,
        });
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
    Some(ResolvedSymbolRef {
        package_name: &package.summary.package_name,
        module_id: &target_module.module_id,
        symbol,
    })
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
            | "AudioDevice"
            | "AudioBuffer"
            | "AudioPlayback"
    )
}

fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('.') {
        let segment = segment.trim();
        if segment.is_empty() {
            return None;
        }
        let mut chars = segment.chars();
        let first = chars.next()?;
        if !is_ident_start(first) || !chars.all(is_ident_continue) {
            return None;
        }
        segments.push(segment.to_string());
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
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
                "phrase_arg_arity.arc",
                "qualified phrase allows at most 3 top-level arguments",
            ),
            (
                "phrase_arg_shape.arc",
                "trailing comma is not allowed before phrase qualifier",
            ),
            (
                "memory_phrase_arg_arity.arc",
                "memory phrase allows at most 3 top-level arguments",
            ),
            (
                "unknown_memory_type.arc",
                "unknown memory type `weird`; supported now: arena, frame, pool (reserved for future expansion)",
            ),
            (
                "unknown_chain_style.arc",
                "unknown chain style `mystery`; supported: forward, lazy, parallel, async, plan, broadcast, collect",
            ),
            (
                "reverse_parallel_chain.arc",
                "chain style `parallel` does not support reverse-introduced chains",
            ),
            (
                "unknown_memory_type.arc",
                "unknown memory type `weird`; supported now: arena, frame, pool (reserved for future expansion)",
            ),
            (
                "invalid_boundary_payload.arc",
                "invalid payload for foreword `#boundary`: `target` must be a string or symbol",
            ),
            (
                "test_payload.arc",
                "invalid payload for foreword `#test`: expected no payload",
            ),
            (
                "invalid_stage_contract_key.arc",
                "invalid #stage contract key 'bad_key'",
            ),
            (
                "invalid_chain_contract_key.arc",
                "invalid #chain contract key 'bad_key'",
            ),
            (
                "invalid_chain_contract_phase.arc",
                "invalid payload for `phase`",
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
    fn check_path_rejects_unresolved_tuple_value_package() {
        let err = check_path(
            &repo_root()
                .join("conformance")
                .join("check_parity_packages")
                .join("unresolved_tuple_value_ref"),
        )
        .expect_err("fixture should fail");
        assert!(
            err.contains("unresolved value reference `missing` in value expression"),
            "{err}"
        );
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
    fn check_path_handles_rewrite_owned_audio_grimoire() {
        let summary = check_path(&repo_root().join("grimoires").join("spell-audio"))
            .expect("audio grimoire should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 4);
    }

    #[test]
    fn check_path_handles_builtin_foreword_example() {
        let summary = check_path(&repo_root().join("examples").join("forewords_builtin_app"))
            .expect("foreword example should check");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 3);
    }

    #[test]
    fn check_path_handles_boundary_interop_example() {
        let summary = check_path(
            &repo_root()
                .join("examples")
                .join("interop_boundary_contracts"),
        )
        .expect("boundary interop example should check");
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
    fn check_path_handles_audio_smoke_example() {
        let summary = check_path(&repo_root().join("examples").join("audio_smoke_demo"))
            .expect("audio smoke example should check");
        assert!(summary.package_count >= 2);
        assert!(summary.module_count >= 4);
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
    fn check_path_rejects_invalid_boundary_and_test_packages() {
        let repo_root = repo_root()
            .join("conformance")
            .join("check_parity_packages");
        for (package, expected) in [
            (
                "invalid_boundary_signature",
                "`#boundary` target `lua` does not allow mutable borrows",
            ),
            (
                "nested_boundary_unsafe",
                "type `types.Payload` is not boundary-safe for target `lua`",
            ),
            (
                "invalid_test_signature",
                "`#test` functions must have zero parameters",
            ),
        ] {
            let err = check_path(&repo_root.join(package)).expect_err("package should fail");
            assert!(err.contains(expected), "{package}: {err}");
        }
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
    fn check_path_rejects_unresolved_body_value_packages() {
        let repo_root = repo_root()
            .join("conformance")
            .join("check_parity_packages");
        for (package, expected) in [
            (
                "unresolved_value_ref",
                "unresolved value reference `missing` in value expression",
            ),
            (
                "unresolved_namespace_member_ref",
                "unresolved value reference `std.kernel.text.missing` in value expression",
            ),
            (
                "unresolved_expr_type_arg",
                "unresolved type reference `Missing` in expression generic argument `Missing`",
            ),
            (
                "unresolved_chain_step",
                "unresolved value reference `missing` in chain step `missing`",
            ),
            (
                "unresolved_rollup_handler",
                "unresolved page rollup handler `missing.cleanup`",
            ),
        ] {
            let err = check_path(&repo_root.join(package))
                .expect_err("unresolved body semantic package should fail");
            assert!(err.contains(expected), "{package}: {err}");
        }
    }

    #[test]
    fn check_path_handles_local_member_field_access() {
        let root = make_temp_package(
            "local_member_access",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "record Box:\n    value: Int\nfn main() -> Int:\n    let item = Box :: value = 1 :: call\n    return item.value\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("local member access should check");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
    }

    #[test]
    fn check_path_handles_enum_variant_constructor_example() {
        let summary = check_path(&repo_root().join("examples").join("result_qmark"))
            .expect("enum variant constructors should resolve");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 3);
    }

    #[test]
    fn check_path_handles_mixed_chain_example() {
        let summary = check_path(&repo_root().join("examples").join("chain_styles_matrix"))
            .expect("mixed chain example should resolve");
        assert_eq!(summary.package_count, 2);
        assert!(summary.module_count >= 3);
    }

    #[test]
    fn check_path_handles_bound_chain_showcase() {
        let summary = check_path(&repo_root().join("examples").join("topdown_arena_showcase"))
            .expect("bound chain showcase should resolve");
        assert!(summary.package_count >= 3);
        assert!(summary.module_count >= 10);
    }

    #[test]
    fn check_path_filters_only_forewords_for_current_target() {
        let root = make_temp_package(
            "only_filter_app",
            "app",
            &[],
            &[
                (
                    "src/shelf.arc",
                    "#only[os = \"definitely_not_host\"]\nfn skipped() -> MissingType:\n    return 0\nfn main() -> Int:\n    return 0\n",
                ),
                ("src/types.arc", ""),
            ],
        );

        let summary = check_path(&root).expect("#only should exclude non-matching declarations");
        assert_eq!(summary.package_count, 1);
        assert_eq!(summary.module_count, 2);

        fs::remove_dir_all(root).expect("cleanup should succeed");
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
