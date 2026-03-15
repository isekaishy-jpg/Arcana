mod entrypoint;
mod executable;
mod runtime_requirements;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use arcana_hir::{
    HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer, HirChainStep,
    HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg, HirHeaderAttachment,
    HirLocalTypeLookup, HirMatchPattern, HirModule, HirModuleDependency, HirModuleSummary,
    HirPackageSummary, HirPageRollup, HirPhraseArg, HirResolvedModule, HirResolvedWorkspace,
    HirStatement, HirStatementKind, HirSymbol, HirSymbolBody, HirSymbolKind, HirUnaryOp,
    HirWorkspacePackage, HirWorkspaceSummary, impl_target_is_public_from_package,
    infer_receiver_expr_type_text, lookup_method_candidates_for_type, lookup_symbol_path,
    match_name_resolves_to_zero_payload_variant, routine_key_for_impl_method,
    routine_key_for_symbol,
};
pub use entrypoint::{
    RUNTIME_MAIN_ENTRYPOINT_NAME, is_runtime_main_entry_symbol,
    runtime_main_return_type_from_signature, validate_runtime_main_entry_contract,
    validate_runtime_main_entry_symbol,
};
pub use runtime_requirements::{
    RuntimeRequirementRoots, derive_runtime_requirements, derive_runtime_requirements_with_roots,
};

pub use executable::{
    ExecAssignOp, ExecAssignTarget, ExecBinaryOp, ExecChainConnector, ExecChainIntroducer,
    ExecChainStep, ExecDynamicDispatch, ExecExpr, ExecHeaderAttachment, ExecMatchArm,
    ExecMatchPattern, ExecPageRollup, ExecPhraseArg, ExecPhraseQualifierKind, ExecStmt,
    ExecUnaryOp,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IrModule {
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrPackageModule {
    pub module_id: String,
    pub symbol_count: usize,
    pub item_count: usize,
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directive_rows: Vec<String>,
    pub lang_item_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrEntrypoint {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrRoutine {
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_param_rows: Vec<String>,
    pub behavior_attr_rows: Vec<String>,
    pub param_rows: Vec<String>,
    pub signature_row: String,
    pub intrinsic_impl: Option<String>,
    pub impl_target_type: Option<String>,
    pub impl_trait_path: Option<Vec<String>>,
    pub foreword_rows: Vec<String>,
    pub rollups: Vec<ExecPageRollup>,
    pub statements: Vec<ExecStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrPackage {
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub modules: Vec<IrPackageModule>,
    pub dependency_edge_count: usize,
    pub dependency_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
    pub runtime_requirements: Vec<String>,
    pub entrypoints: Vec<IrEntrypoint>,
    pub routines: Vec<IrRoutine>,
}

impl IrPackage {
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

#[derive(Clone, Debug, Default)]
struct LowerValueScope {
    locals: BTreeMap<String, String>,
}

impl LowerValueScope {
    fn contains(&self, name: &str) -> bool {
        self.locals.contains_key(name)
    }

    fn type_text_of(&self, name: &str) -> Option<&str> {
        self.locals.get(name).map(String::as_str)
    }

    fn insert(&mut self, name: impl Into<String>, type_text: impl Into<String>) {
        self.locals.insert(name.into(), type_text.into());
    }
}

impl HirLocalTypeLookup for LowerValueScope {
    fn contains_local(&self, name: &str) -> bool {
        LowerValueScope::contains(self, name)
    }

    fn type_text_of(&self, name: &str) -> Option<&str> {
        LowerValueScope::type_text_of(self, name)
    }
}

#[derive(Clone, Debug)]
struct ResolvedRenderScope<'a> {
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    current_where_clause: Option<&'a str>,
    value_scope: LowerValueScope,
    errors: Rc<RefCell<Vec<String>>>,
}

impl<'a> ResolvedRenderScope<'a> {
    fn new(
        workspace: &'a HirWorkspaceSummary,
        resolved_module: &'a HirResolvedModule,
        current_where_clause: Option<&'a str>,
        _type_params: &[String],
    ) -> Self {
        Self {
            workspace,
            resolved_module,
            current_where_clause,
            value_scope: LowerValueScope::default(),
            errors: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn note_error(&self, message: impl Into<String>) {
        self.errors.borrow_mut().push(message.into());
    }

    fn finish(self) -> Result<(), String> {
        let errors = self.errors.borrow();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

pub fn lower_hir(module: &HirModule) -> IrModule {
    IrModule {
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

pub fn lower_module_summary(module: &HirModuleSummary) -> IrModule {
    IrModule {
        symbol_count: module.symbols.len(),
        item_count: module.non_empty_line_count + module.directives.len(),
    }
}

fn render_directive_row(
    module_id: &str,
    kind: HirDirectiveKind,
    path: &[String],
    alias: &Option<String>,
) -> String {
    format!(
        "module={module_id}:{}:{}:{}",
        kind.as_str(),
        path.join("."),
        alias.as_deref().unwrap_or("")
    )
}

fn render_lang_item_row(module_id: &str, name: &str, target: &[String]) -> String {
    format!("module={module_id}:lang:{name}:{}", target.join("."))
}

fn render_dependency_row(edge: &HirModuleDependency) -> String {
    format!(
        "source={}:{}:{}:{}",
        edge.source_module_id,
        edge.kind.as_str(),
        edge.target_path.join("."),
        edge.alias.as_deref().unwrap_or("")
    )
}

fn canonical_dependency_path(package: &HirWorkspacePackage, path: &[String]) -> Vec<String> {
    let Some((first, suffix)) = path.split_first() else {
        return Vec::new();
    };
    let canonical_root = package
        .dependency_package_name(first)
        .unwrap_or(first)
        .to_string();
    let mut canonical = Vec::with_capacity(path.len());
    canonical.push(canonical_root);
    canonical.extend(suffix.iter().cloned());
    canonical
}

fn render_resolved_dependency_row(
    package: &HirWorkspacePackage,
    edge: &HirModuleDependency,
) -> String {
    format!(
        "source={}:{}:{}:{}",
        edge.source_module_id,
        edge.kind.as_str(),
        canonical_dependency_path(package, &edge.target_path).join("."),
        edge.alias.as_deref().unwrap_or("")
    )
}

fn resolved_direct_deps(package: &HirWorkspacePackage) -> Vec<String> {
    package
        .direct_dep_packages
        .values()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn encode_surface_text(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}

fn render_impl_surface_row(impl_decl: &arcana_hir::HirImplDecl) -> String {
    let methods = impl_decl
        .methods
        .iter()
        .map(|method| {
            format!(
                "{}:{}",
                method.kind.as_str(),
                encode_surface_text(&method.surface_text)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "impl:target={}:trait={}:methods=[{}]",
        encode_surface_text(&impl_decl.target_type),
        encode_surface_text(impl_decl.trait_path.as_deref().unwrap_or("")),
        methods
    )
}

fn resolved_module_exported_surface_rows(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    module: &HirModuleSummary,
) -> Vec<String> {
    let mut rows = module.summary_surface_rows();
    rows.extend(module.impls.iter().filter_map(|impl_decl| {
        impl_target_is_public_from_package(workspace, package, module, &impl_decl.target_type)
            .then(|| render_impl_surface_row(impl_decl))
    }));
    rows.sort();
    rows.dedup();
    rows
}

fn quote_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn decode_row_string(text: &str) -> Result<String, String> {
    let inner = text
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or_else(|| format!("malformed source string literal `{text}`"))?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                return Err("unterminated escape in source string".to_string());
            };
            match next {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn decode_source_string_literal(text: &str) -> Result<String, String> {
    let source = decode_row_string(text)?;
    if source.starts_with('"') && source.ends_with('"') && source.len() >= 2 {
        decode_row_string(&source)
    } else {
        Ok(source)
    }
}

fn render_foreword_arg(arg: &HirForewordArg) -> String {
    match &arg.name {
        Some(name) => format!("{name}=\"{}\"", quote_text(&arg.value)),
        None => format!("\"{}\"", quote_text(&arg.value)),
    }
}

fn render_foreword_row(app: &HirForewordApp) -> String {
    format!(
        "{}({})",
        app.name,
        app.args
            .iter()
            .map(render_foreword_arg)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn is_identifier_text(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn split_simple_path(text: &str) -> Option<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let segments = trimmed
        .split('.')
        .map(str::trim)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    (!segments.is_empty() && segments.iter().all(|segment| is_identifier_text(segment)))
        .then_some(segments)
}

fn split_top_level_surface_items(text: &str, delimiter: char) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;
    for ch in text.chars() {
        if in_string {
            current.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '[' | '(' => {
                depth += 1;
                current.push(ch);
            }
            ']' | ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            _ if ch == delimiter && depth == 0 => {
                let item = current.trim();
                if !item.is_empty() {
                    items.push(item.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let tail = current.trim();
    if !tail.is_empty() {
        items.push(tail.to_string());
    }
    items
}

fn erase_type_generics(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 && !ch.is_whitespace() => out.push(ch),
            _ => {}
        }
    }
    out
}

fn strip_reference_prefix(text: &str) -> &str {
    let trimmed = text.trim_start();
    if let Some(rest) = trimmed.strip_prefix("&mut") {
        return rest.trim_start();
    }
    if let Some(rest) = trimmed.strip_prefix('&') {
        return rest.trim_start();
    }
    trimmed
}

fn parse_surface_type_application(text: &str) -> Option<(String, Vec<String>)> {
    let trimmed = text.trim();
    if let Some(path) = split_simple_path(trimmed) {
        return Some((path.join("."), Vec::new()));
    }
    let mut depth = 0usize;
    let mut open = None;
    for (index, ch) in trimmed.char_indices() {
        match ch {
            '[' if depth == 0 => {
                open = Some(index);
                break;
            }
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let open = open?;
    if !trimmed.ends_with(']') || open == 0 {
        return None;
    }
    let base = trimmed[..open].trim();
    let path = split_simple_path(base)?;
    let args = split_top_level_surface_items(&trimmed[open + 1..trimmed.len() - 1], ',');
    Some((path.join("."), args))
}

fn canonical_impl_trait_path(path: &str) -> Vec<String> {
    let base = parse_surface_type_application(path)
        .map(|(base, _)| base)
        .unwrap_or_else(|| path.to_string());
    base.split('.').map(ToString::to_string).collect()
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

fn flatten_callable_expr_path(expr: &HirExpr) -> Option<Vec<String>> {
    match expr {
        HirExpr::GenericApply { expr, .. } => flatten_callable_expr_path(expr),
        _ => flatten_member_expr_path(expr),
    }
}

fn resolved_symbol_path(symbol_ref: arcana_hir::HirResolvedSymbolRef<'_>) -> Vec<String> {
    let mut path = symbol_ref
        .module_id
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    path.push(symbol_ref.symbol.name.clone());
    path
}

fn resolved_symbol_routine_key(symbol_ref: &arcana_hir::HirResolvedSymbolRef<'_>) -> String {
    routine_key_for_symbol(symbol_ref.module_id, symbol_ref.symbol_index)
}

fn format_method_ambiguity(
    type_text: &str,
    method_name: &str,
    candidates: &[ResolvedMethod<'_>],
) -> String {
    let rendered = candidates
        .iter()
        .map(|candidate| {
            format!(
                "{} [{}]",
                candidate.method.surface_text, candidate.routine_key
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "bare-method qualifier `{method_name}` on `{type_text}` is ambiguous; candidates: {rendered}"
    )
}

#[derive(Clone, Debug)]
struct ResolvedMethod<'a> {
    module_id: String,
    method: &'a HirSymbol,
    routine_key: String,
    trait_path: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
struct ResolvedPhraseTarget {
    path: Vec<String>,
    routine_key: Option<String>,
    dynamic_dispatch: Option<ExecDynamicDispatch>,
}

fn lookup_method_resolution_for_type<'a>(
    scope: &'a ResolvedRenderScope<'a>,
    workspace: &'a HirWorkspaceSummary,
    type_text: &str,
    method_name: &str,
) -> Result<Option<ResolvedMethod<'a>>, String> {
    let candidates =
        lookup_method_candidates_for_type(workspace, scope.resolved_module, type_text, method_name)
            .into_iter()
            .map(|candidate| ResolvedMethod {
                module_id: candidate.module_id.to_string(),
                method: candidate.symbol,
                routine_key: candidate.routine_key,
                trait_path: None,
            })
            .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => Ok(None),
        [resolved] => Ok(Some(resolved.clone())),
        _ => Err(format_method_ambiguity(type_text, method_name, &candidates)),
    }
}

fn lookup_trait_method_resolution_from_where_clause<'a>(
    scope: &'a ResolvedRenderScope<'a>,
    type_text: &str,
    method_name: &str,
) -> Result<Vec<ResolvedMethod<'a>>, String> {
    let wanted = erase_type_generics(strip_reference_prefix(type_text));
    if !is_identifier_text(&wanted) {
        return Ok(Vec::new());
    }
    let Some(where_clause) = scope.current_where_clause else {
        return Ok(Vec::new());
    };
    let mut candidates = Vec::new();
    for predicate in split_top_level_surface_items(where_clause, ',') {
        let Some((trait_base, args)) = parse_surface_type_application(&predicate) else {
            continue;
        };
        if !args
            .iter()
            .any(|arg| erase_type_generics(strip_reference_prefix(arg)) == wanted)
        {
            continue;
        }
        let Some(trait_path) = split_simple_path(&trait_base) else {
            continue;
        };
        let Some(symbol_ref) =
            lookup_symbol_path(scope.workspace, scope.resolved_module, &trait_path)
        else {
            continue;
        };
        let HirSymbolBody::Trait { methods, .. } = &symbol_ref.symbol.body else {
            continue;
        };
        if let Some(method) = methods.iter().find(|method| method.name == method_name) {
            candidates.push(ResolvedMethod {
                module_id: symbol_ref.module_id.to_string(),
                method,
                routine_key: String::new(),
                trait_path: Some(trait_path),
            });
        }
    }
    Ok(candidates)
}

fn render_param_row(symbol: &HirSymbol) -> Vec<String> {
    symbol
        .params
        .iter()
        .map(|param| {
            format!(
                "mode={}:name={}:ty={}",
                param.mode.map(|mode| mode.as_str()).unwrap_or(""),
                param.name,
                param.ty
            )
        })
        .collect()
}

fn render_type_param_rows(symbol: &HirSymbol) -> Vec<String> {
    symbol
        .type_params
        .iter()
        .map(|name| format!("name={name}"))
        .collect()
}

fn render_behavior_attr_rows(symbol: &HirSymbol) -> Vec<String> {
    symbol
        .behavior_attrs
        .iter()
        .map(|attr| {
            format!(
                "name=\"{}\":value=\"{}\"",
                quote_text(&attr.name),
                quote_text(&attr.value)
            )
        })
        .collect()
}

fn infer_iterable_binding_type_text(
    scope: &ResolvedRenderScope<'_>,
    iterable: &HirExpr,
) -> Option<String> {
    let iterable_ty = infer_expr_type_text(scope, iterable)?;
    let (base, args) = parse_surface_type_application(strip_reference_prefix(&iterable_ty))?;
    match base.as_str() {
        "RangeInt" => Some("Int".to_string()),
        "List" | "Array" => args.first().cloned(),
        "Map" => match (args.first(), args.get(1)) {
            (Some(key), Some(value)) => Some(format!("Pair[{key}, {value}]")),
            _ => None,
        },
        _ => None,
    }
}

fn resolve_qualified_phrase_target_path(
    scope: &ResolvedRenderScope<'_>,
    subject: &HirExpr,
    qualifier: &str,
) -> Option<ResolvedPhraseTarget> {
    if qualifier == "call" {
        let path = flatten_callable_expr_path(subject)?;
        return lookup_symbol_path(scope.workspace, scope.resolved_module, &path).map(|resolved| {
            let routine_key = resolved_symbol_routine_key(&resolved);
            ResolvedPhraseTarget {
                path: resolved_symbol_path(resolved),
                routine_key: Some(routine_key),
                dynamic_dispatch: None,
            }
        });
    }

    if let Some(path) = split_simple_path(qualifier).filter(|path| path.len() > 1) {
        if let Some(resolved) = lookup_symbol_path(scope.workspace, scope.resolved_module, &path) {
            let routine_key = resolved_symbol_routine_key(&resolved);
            return Some(ResolvedPhraseTarget {
                path: resolved_symbol_path(resolved),
                routine_key: Some(routine_key),
                dynamic_dispatch: None,
            });
        }
    }
    None
}

fn resolve_bare_method_target(
    scope: &ResolvedRenderScope<'_>,
    subject: &HirExpr,
    qualifier: &str,
) -> Option<ResolvedPhraseTarget> {
    let subject_ty = infer_receiver_expr_type_text(
        scope.workspace,
        scope.resolved_module,
        &scope.value_scope,
        subject,
    )?;
    match lookup_method_resolution_for_type(scope, scope.workspace, &subject_ty, qualifier) {
        Ok(Some(resolved)) => {
            let mut path = resolved
                .module_id
                .split('.')
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            path.push(resolved.method.name.clone());
            return Some(ResolvedPhraseTarget {
                path,
                routine_key: Some(resolved.routine_key),
                dynamic_dispatch: None,
            });
        }
        Ok(None) => {}
        Err(message) => {
            scope.note_error(message);
            return None;
        }
    }
    match lookup_trait_method_resolution_from_where_clause(scope, &subject_ty, qualifier) {
        Ok(candidates) => {
            if candidates.len() > 1 {
                scope.note_error(format!(
                    "bare-method qualifier `{qualifier}` on `{subject_ty}` is ambiguous across trait bounds"
                ));
                return None;
            }
            let resolved = candidates.into_iter().next()?;
            let mut path = resolved
                .module_id
                .split('.')
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            path.push(resolved.method.name.clone());
            Some(ResolvedPhraseTarget {
                path,
                routine_key: None,
                dynamic_dispatch: resolved
                    .trait_path
                    .map(|trait_path| ExecDynamicDispatch::TraitMethod { trait_path }),
            })
        }
        Err(message) => {
            scope.note_error(message);
            None
        }
    }
}

fn infer_expr_type_text(scope: &ResolvedRenderScope<'_>, expr: &HirExpr) -> Option<String> {
    if let Some(inferred) = infer_receiver_expr_type_text(
        scope.workspace,
        scope.resolved_module,
        &scope.value_scope,
        expr,
    ) {
        return Some(inferred);
    }
    match expr {
        HirExpr::Pair { left, right } => Some(format!(
            "Pair[{}, {}]",
            infer_expr_type_text(scope, left)?,
            infer_expr_type_text(scope, right)?
        )),
        _ => None,
    }
}

fn lower_rollup(rollup: &HirPageRollup) -> ExecPageRollup {
    ExecPageRollup {
        kind: rollup.kind.as_str().to_string(),
        subject: rollup.subject.clone(),
        handler_path: rollup.handler_path.clone(),
    }
}

fn lower_rollup_resolved(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    rollup: &HirPageRollup,
) -> ExecPageRollup {
    let handler_path = lookup_symbol_path(workspace, resolved_module, &rollup.handler_path)
        .map(|symbol_ref| {
            let mut path = symbol_ref
                .module_id
                .split('.')
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            path.push(symbol_ref.symbol.name.clone());
            path
        })
        .unwrap_or_else(|| rollup.handler_path.clone());
    ExecPageRollup {
        kind: rollup.kind.as_str().to_string(),
        subject: rollup.subject.clone(),
        handler_path,
    }
}

fn lower_phrase_qualifier_kind(qualifier: &str) -> ExecPhraseQualifierKind {
    match qualifier.trim() {
        "call" => ExecPhraseQualifierKind::Call,
        "?" => ExecPhraseQualifierKind::Try,
        ">" => ExecPhraseQualifierKind::Apply,
        ">>" => ExecPhraseQualifierKind::AwaitApply,
        other if other.contains('.') => ExecPhraseQualifierKind::NamedPath,
        _ => ExecPhraseQualifierKind::BareMethod,
    }
}

fn lower_chain_connector(connector: HirChainConnector) -> ExecChainConnector {
    match connector {
        HirChainConnector::Forward => ExecChainConnector::Forward,
        HirChainConnector::Reverse => ExecChainConnector::Reverse,
    }
}

fn lower_chain_introducer(introducer: HirChainIntroducer) -> ExecChainIntroducer {
    match introducer {
        HirChainIntroducer::Forward => ExecChainIntroducer::Forward,
        HirChainIntroducer::Reverse => ExecChainIntroducer::Reverse,
    }
}

fn lower_unary_op(op: HirUnaryOp) -> ExecUnaryOp {
    match op {
        HirUnaryOp::Neg => ExecUnaryOp::Neg,
        HirUnaryOp::Not => ExecUnaryOp::Not,
        HirUnaryOp::BitNot => ExecUnaryOp::BitNot,
        HirUnaryOp::BorrowRead => ExecUnaryOp::BorrowRead,
        HirUnaryOp::BorrowMut => ExecUnaryOp::BorrowMut,
        HirUnaryOp::Deref => ExecUnaryOp::Deref,
        HirUnaryOp::Weave => ExecUnaryOp::Weave,
        HirUnaryOp::Split => ExecUnaryOp::Split,
    }
}

fn lower_binary_op(op: HirBinaryOp) -> ExecBinaryOp {
    match op {
        HirBinaryOp::Or => ExecBinaryOp::Or,
        HirBinaryOp::And => ExecBinaryOp::And,
        HirBinaryOp::EqEq => ExecBinaryOp::EqEq,
        HirBinaryOp::NotEq => ExecBinaryOp::NotEq,
        HirBinaryOp::Lt => ExecBinaryOp::Lt,
        HirBinaryOp::LtEq => ExecBinaryOp::LtEq,
        HirBinaryOp::Gt => ExecBinaryOp::Gt,
        HirBinaryOp::GtEq => ExecBinaryOp::GtEq,
        HirBinaryOp::BitOr => ExecBinaryOp::BitOr,
        HirBinaryOp::BitXor => ExecBinaryOp::BitXor,
        HirBinaryOp::BitAnd => ExecBinaryOp::BitAnd,
        HirBinaryOp::Shl => ExecBinaryOp::Shl,
        HirBinaryOp::Shr => ExecBinaryOp::Shr,
        HirBinaryOp::Add => ExecBinaryOp::Add,
        HirBinaryOp::Sub => ExecBinaryOp::Sub,
        HirBinaryOp::Mul => ExecBinaryOp::Mul,
        HirBinaryOp::Div => ExecBinaryOp::Div,
        HirBinaryOp::Mod => ExecBinaryOp::Mod,
    }
}

fn lower_assign_op(op: HirAssignOp) -> ExecAssignOp {
    match op {
        HirAssignOp::Assign => ExecAssignOp::Assign,
        HirAssignOp::AddAssign => ExecAssignOp::AddAssign,
        HirAssignOp::SubAssign => ExecAssignOp::SubAssign,
        HirAssignOp::MulAssign => ExecAssignOp::MulAssign,
        HirAssignOp::DivAssign => ExecAssignOp::DivAssign,
        HirAssignOp::ModAssign => ExecAssignOp::ModAssign,
        HirAssignOp::BitAndAssign => ExecAssignOp::BitAndAssign,
        HirAssignOp::BitOrAssign => ExecAssignOp::BitOrAssign,
        HirAssignOp::BitXorAssign => ExecAssignOp::BitXorAssign,
        HirAssignOp::ShlAssign => ExecAssignOp::ShlAssign,
        HirAssignOp::ShrAssign => ExecAssignOp::ShrAssign,
    }
}

fn lower_match_pattern_exec(pattern: &HirMatchPattern) -> ExecMatchPattern {
    match pattern {
        HirMatchPattern::Wildcard => ExecMatchPattern::Wildcard,
        HirMatchPattern::Literal { text } => ExecMatchPattern::Literal(text.clone()),
        HirMatchPattern::Name { text } => {
            if text.contains('.') {
                return ExecMatchPattern::Variant {
                    path: text.clone(),
                    args: Vec::new(),
                };
            }
            ExecMatchPattern::Name(text.clone())
        }
        HirMatchPattern::Variant { path, args } => ExecMatchPattern::Variant {
            path: path.clone(),
            args: args.iter().map(lower_match_pattern_exec).collect(),
        },
    }
}

fn lower_subject_match_pattern_exec_resolved(
    pattern: &HirMatchPattern,
    subject: &HirExpr,
    scope: &ResolvedRenderScope<'_>,
) -> ExecMatchPattern {
    match pattern {
        HirMatchPattern::Wildcard => ExecMatchPattern::Wildcard,
        HirMatchPattern::Literal { text } => ExecMatchPattern::Literal(text.clone()),
        HirMatchPattern::Name { text } => {
            if text.contains('.')
                || match_name_resolves_to_zero_payload_variant(
                    scope.workspace,
                    scope.resolved_module,
                    &scope.value_scope,
                    subject,
                    text,
                )
            {
                return ExecMatchPattern::Variant {
                    path: text.clone(),
                    args: Vec::new(),
                };
            }
            ExecMatchPattern::Name(text.clone())
        }
        HirMatchPattern::Variant { path, args } => ExecMatchPattern::Variant {
            path: path.clone(),
            args: args.iter().map(lower_match_pattern_exec).collect(),
        },
    }
}

fn lower_phrase_arg_exec(arg: &HirPhraseArg) -> ExecPhraseArg {
    match arg {
        HirPhraseArg::Positional(expr) => ExecPhraseArg {
            name: None,
            value: lower_exec_expr(expr),
        },
        HirPhraseArg::Named { name, value } => ExecPhraseArg {
            name: Some(name.clone()),
            value: lower_exec_expr(value),
        },
    }
}

fn lower_phrase_arg_exec_resolved(
    arg: &HirPhraseArg,
    scope: &ResolvedRenderScope<'_>,
) -> ExecPhraseArg {
    match arg {
        HirPhraseArg::Positional(expr) => ExecPhraseArg {
            name: None,
            value: lower_exec_expr_resolved(expr, scope),
        },
        HirPhraseArg::Named { name, value } => ExecPhraseArg {
            name: Some(name.clone()),
            value: lower_exec_expr_resolved(value, scope),
        },
    }
}

fn lower_header_attachment_exec(attachment: &HirHeaderAttachment) -> ExecHeaderAttachment {
    match attachment {
        HirHeaderAttachment::Named { name, value, .. } => ExecHeaderAttachment::Named {
            name: name.clone(),
            value: lower_exec_expr(value),
        },
        HirHeaderAttachment::Chain { expr, .. } => ExecHeaderAttachment::Chain {
            expr: lower_exec_expr(expr),
        },
    }
}

fn lower_header_attachment_exec_resolved(
    attachment: &HirHeaderAttachment,
    scope: &ResolvedRenderScope<'_>,
) -> ExecHeaderAttachment {
    match attachment {
        HirHeaderAttachment::Named { name, value, .. } => ExecHeaderAttachment::Named {
            name: name.clone(),
            value: lower_exec_expr_resolved(value, scope),
        },
        HirHeaderAttachment::Chain { expr, .. } => ExecHeaderAttachment::Chain {
            expr: lower_exec_expr_resolved(expr, scope),
        },
    }
}

fn lower_chain_step_exec(step: &HirChainStep) -> ExecChainStep {
    ExecChainStep {
        incoming: step.incoming.map(lower_chain_connector),
        stage: lower_exec_expr(&step.stage),
        bind_args: step.bind_args.iter().map(lower_exec_expr).collect(),
        text: step.text.clone(),
    }
}

fn lower_chain_step_exec_resolved(
    step: &HirChainStep,
    scope: &ResolvedRenderScope<'_>,
) -> ExecChainStep {
    ExecChainStep {
        incoming: step.incoming.map(lower_chain_connector),
        stage: lower_exec_expr_resolved(&step.stage, scope),
        bind_args: step
            .bind_args
            .iter()
            .map(|expr| lower_exec_expr_resolved(expr, scope))
            .collect(),
        text: step.text.clone(),
    }
}

fn lower_assign_target_exec(target: &HirAssignTarget) -> ExecAssignTarget {
    match target {
        HirAssignTarget::Name { text } => ExecAssignTarget::Name(text.clone()),
        HirAssignTarget::MemberAccess { target, member } => ExecAssignTarget::Member {
            target: Box::new(lower_assign_target_exec(target)),
            member: member.clone(),
        },
        HirAssignTarget::Index { target, index } => ExecAssignTarget::Index {
            target: Box::new(lower_assign_target_exec(target)),
            index: lower_exec_expr(index),
        },
    }
}

fn lower_assign_target_exec_resolved(
    target: &HirAssignTarget,
    scope: &ResolvedRenderScope<'_>,
) -> ExecAssignTarget {
    match target {
        HirAssignTarget::Name { text } => ExecAssignTarget::Name(text.clone()),
        HirAssignTarget::MemberAccess { target, member } => ExecAssignTarget::Member {
            target: Box::new(lower_assign_target_exec_resolved(target, scope)),
            member: member.clone(),
        },
        HirAssignTarget::Index { target, index } => ExecAssignTarget::Index {
            target: Box::new(lower_assign_target_exec_resolved(target, scope)),
            index: lower_exec_expr_resolved(index, scope),
        },
    }
}

fn lower_exec_expr(expr: &HirExpr) -> ExecExpr {
    match expr {
        HirExpr::Path { segments } => ExecExpr::Path(segments.clone()),
        HirExpr::BoolLiteral { value } => ExecExpr::Bool(*value),
        HirExpr::IntLiteral { text } => ExecExpr::Int(text.parse().unwrap_or_default()),
        HirExpr::StrLiteral { text } => {
            ExecExpr::Str(decode_source_string_literal(text).unwrap_or_else(|_| text.clone()))
        }
        HirExpr::Pair { left, right } => ExecExpr::Pair {
            left: Box::new(lower_exec_expr(left)),
            right: Box::new(lower_exec_expr(right)),
        },
        HirExpr::CollectionLiteral { items } => ExecExpr::Collection {
            items: items.iter().map(lower_exec_expr).collect(),
        },
        HirExpr::Match { subject, arms } => ExecExpr::Match {
            subject: Box::new(lower_exec_expr(subject)),
            arms: arms
                .iter()
                .map(|arm| ExecMatchArm {
                    patterns: arm.patterns.iter().map(lower_match_pattern_exec).collect(),
                    value: lower_exec_expr(&arm.value),
                })
                .collect(),
        },
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => ExecExpr::Chain {
            style: style.clone(),
            introducer: lower_chain_introducer(*introducer),
            steps: steps.iter().map(lower_chain_step_exec).collect(),
        },
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => ExecExpr::MemoryPhrase {
            family: family.clone(),
            arena: Box::new(lower_exec_expr(arena)),
            init_args: init_args.iter().map(lower_phrase_arg_exec).collect(),
            constructor: Box::new(lower_exec_expr(constructor)),
            attached: attached.iter().map(lower_header_attachment_exec).collect(),
        },
        HirExpr::GenericApply { expr, type_args } => ExecExpr::Generic {
            expr: Box::new(lower_exec_expr(expr)),
            type_args: type_args.clone(),
        },
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
        } => ExecExpr::Phrase {
            subject: Box::new(lower_exec_expr(subject)),
            args: args.iter().map(lower_phrase_arg_exec).collect(),
            qualifier_kind: lower_phrase_qualifier_kind(qualifier),
            qualifier: qualifier.clone(),
            resolved_callable: None,
            resolved_routine: None,
            dynamic_dispatch: None,
            attached: attached.iter().map(lower_header_attachment_exec).collect(),
        },
        HirExpr::Await { expr } => ExecExpr::Await {
            expr: Box::new(lower_exec_expr(expr)),
        },
        HirExpr::Unary { op, expr } => ExecExpr::Unary {
            op: lower_unary_op(*op),
            expr: Box::new(lower_exec_expr(expr)),
        },
        HirExpr::Binary { left, op, right } => ExecExpr::Binary {
            left: Box::new(lower_exec_expr(left)),
            op: lower_binary_op(*op),
            right: Box::new(lower_exec_expr(right)),
        },
        HirExpr::MemberAccess { expr, member } => ExecExpr::Member {
            expr: Box::new(lower_exec_expr(expr)),
            member: member.clone(),
        },
        HirExpr::Index { expr, index } => ExecExpr::Index {
            expr: Box::new(lower_exec_expr(expr)),
            index: Box::new(lower_exec_expr(index)),
        },
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => ExecExpr::Slice {
            expr: Box::new(lower_exec_expr(expr)),
            start: start.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            end: end.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            inclusive_end: *inclusive_end,
        },
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => ExecExpr::Range {
            start: start.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            end: end.as_ref().map(|expr| Box::new(lower_exec_expr(expr))),
            inclusive_end: *inclusive_end,
        },
    }
}

fn lower_exec_expr_resolved(expr: &HirExpr, scope: &ResolvedRenderScope<'_>) -> ExecExpr {
    match expr {
        HirExpr::Path { segments } => ExecExpr::Path(segments.clone()),
        HirExpr::BoolLiteral { value } => ExecExpr::Bool(*value),
        HirExpr::IntLiteral { text } => ExecExpr::Int(text.parse().unwrap_or_default()),
        HirExpr::StrLiteral { text } => {
            ExecExpr::Str(decode_source_string_literal(text).unwrap_or_else(|_| text.clone()))
        }
        HirExpr::Pair { left, right } => ExecExpr::Pair {
            left: Box::new(lower_exec_expr_resolved(left, scope)),
            right: Box::new(lower_exec_expr_resolved(right, scope)),
        },
        HirExpr::CollectionLiteral { items } => ExecExpr::Collection {
            items: items
                .iter()
                .map(|item| lower_exec_expr_resolved(item, scope))
                .collect(),
        },
        HirExpr::Match { subject, arms } => ExecExpr::Match {
            subject: Box::new(lower_exec_expr_resolved(subject, scope)),
            arms: arms
                .iter()
                .map(|arm| ExecMatchArm {
                    patterns: arm
                        .patterns
                        .iter()
                        .map(|pattern| {
                            lower_subject_match_pattern_exec_resolved(pattern, subject, scope)
                        })
                        .collect(),
                    value: lower_exec_expr_resolved(&arm.value, scope),
                })
                .collect(),
        },
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => ExecExpr::Chain {
            style: style.clone(),
            introducer: lower_chain_introducer(*introducer),
            steps: steps
                .iter()
                .map(|step| lower_chain_step_exec_resolved(step, scope))
                .collect(),
        },
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => ExecExpr::MemoryPhrase {
            family: family.clone(),
            arena: Box::new(lower_exec_expr_resolved(arena, scope)),
            init_args: init_args
                .iter()
                .map(|arg| lower_phrase_arg_exec_resolved(arg, scope))
                .collect(),
            constructor: Box::new(lower_exec_expr_resolved(constructor, scope)),
            attached: attached
                .iter()
                .map(|attachment| lower_header_attachment_exec_resolved(attachment, scope))
                .collect(),
        },
        HirExpr::GenericApply { expr, type_args } => ExecExpr::Generic {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            type_args: type_args.clone(),
        },
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
        } => {
            let qualifier_kind = lower_phrase_qualifier_kind(qualifier);
            let resolved = match qualifier_kind {
                ExecPhraseQualifierKind::Call | ExecPhraseQualifierKind::NamedPath => {
                    resolve_qualified_phrase_target_path(scope, subject, qualifier)
                }
                ExecPhraseQualifierKind::BareMethod => {
                    resolve_bare_method_target(scope, subject, qualifier)
                }
                _ => None,
            };
            ExecExpr::Phrase {
                subject: Box::new(lower_exec_expr_resolved(subject, scope)),
                args: args
                    .iter()
                    .map(|arg| lower_phrase_arg_exec_resolved(arg, scope))
                    .collect(),
                qualifier_kind,
                qualifier: qualifier.clone(),
                resolved_callable: resolved.as_ref().map(|target| target.path.clone()),
                resolved_routine: resolved
                    .as_ref()
                    .and_then(|target| target.routine_key.clone()),
                dynamic_dispatch: resolved.and_then(|target| target.dynamic_dispatch),
                attached: attached
                    .iter()
                    .map(|attachment| lower_header_attachment_exec_resolved(attachment, scope))
                    .collect(),
            }
        }
        HirExpr::Await { expr } => ExecExpr::Await {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
        },
        HirExpr::Unary { op, expr } => ExecExpr::Unary {
            op: lower_unary_op(*op),
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
        },
        HirExpr::Binary { left, op, right } => ExecExpr::Binary {
            left: Box::new(lower_exec_expr_resolved(left, scope)),
            op: lower_binary_op(*op),
            right: Box::new(lower_exec_expr_resolved(right, scope)),
        },
        HirExpr::MemberAccess { expr, member } => ExecExpr::Member {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            member: member.clone(),
        },
        HirExpr::Index { expr, index } => ExecExpr::Index {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            index: Box::new(lower_exec_expr_resolved(index, scope)),
        },
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => ExecExpr::Slice {
            expr: Box::new(lower_exec_expr_resolved(expr, scope)),
            start: start
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            end: end
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            inclusive_end: *inclusive_end,
        },
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => ExecExpr::Range {
            start: start
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            end: end
                .as_ref()
                .map(|expr| Box::new(lower_exec_expr_resolved(expr, scope))),
            inclusive_end: *inclusive_end,
        },
    }
}

fn lower_exec_stmt_block(statements: &[HirStatement]) -> Vec<ExecStmt> {
    statements.iter().map(lower_exec_stmt).collect()
}

fn lower_exec_stmt(statement: &HirStatement) -> ExecStmt {
    match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => ExecStmt::Let {
            mutable: *mutable,
            name: name.clone(),
            value: lower_exec_expr(value),
        },
        HirStatementKind::Expr { expr } => ExecStmt::Expr {
            expr: lower_exec_expr(expr),
            rollups: statement.rollups.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::Return { value } => match value.as_ref() {
            Some(value) => ExecStmt::ReturnValue {
                value: lower_exec_expr(value),
            },
            None => ExecStmt::ReturnVoid,
        },
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => ExecStmt::If {
            condition: lower_exec_expr(condition),
            then_branch: lower_exec_stmt_block(then_branch),
            else_branch: else_branch
                .as_ref()
                .map(|branch| branch.iter().map(lower_exec_stmt).collect())
                .unwrap_or_default(),
            rollups: statement.rollups.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::While { condition, body } => ExecStmt::While {
            condition: lower_exec_expr(condition),
            body: lower_exec_stmt_block(body),
            rollups: statement.rollups.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => ExecStmt::For {
            binding: binding.clone(),
            iterable: lower_exec_expr(iterable),
            body: lower_exec_stmt_block(body),
            rollups: statement.rollups.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::Defer { expr } => ExecStmt::Defer(lower_exec_expr(expr)),
        HirStatementKind::Break => ExecStmt::Break,
        HirStatementKind::Continue => ExecStmt::Continue,
        HirStatementKind::Assign { target, op, value } => ExecStmt::Assign {
            target: lower_assign_target_exec(target),
            op: lower_assign_op(*op),
            value: lower_exec_expr(value),
        },
    }
}

fn lower_exec_stmt_block_resolved(
    statements: &[HirStatement],
    scope: &mut ResolvedRenderScope<'_>,
) -> Vec<ExecStmt> {
    statements
        .iter()
        .map(|statement| lower_exec_stmt_resolved(statement, scope))
        .collect()
}

fn lower_exec_stmt_resolved(
    statement: &HirStatement,
    scope: &mut ResolvedRenderScope<'_>,
) -> ExecStmt {
    match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => {
            let lowered = ExecStmt::Let {
                mutable: *mutable,
                name: name.clone(),
                value: lower_exec_expr_resolved(value, scope),
            };
            if let Some(type_text) = infer_expr_type_text(scope, value) {
                scope.value_scope.insert(name.clone(), type_text);
            }
            lowered
        }
        HirStatementKind::Expr { expr } => ExecStmt::Expr {
            expr: lower_exec_expr_resolved(expr, scope),
            rollups: statement
                .rollups
                .iter()
                .map(|rollup| lower_rollup_resolved(scope.workspace, scope.resolved_module, rollup))
                .collect(),
        },
        HirStatementKind::Return { value } => match value.as_ref() {
            Some(value) => ExecStmt::ReturnValue {
                value: lower_exec_expr_resolved(value, scope),
            },
            None => ExecStmt::ReturnVoid,
        },
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let mut then_scope = scope.clone();
            let then_branch = lower_exec_stmt_block_resolved(then_branch, &mut then_scope);
            let else_branch = else_branch
                .as_ref()
                .map(|branch| {
                    let mut else_scope = scope.clone();
                    lower_exec_stmt_block_resolved(branch, &mut else_scope)
                })
                .unwrap_or_default();
            ExecStmt::If {
                condition: lower_exec_expr_resolved(condition, scope),
                then_branch,
                else_branch,
                rollups: statement
                    .rollups
                    .iter()
                    .map(|rollup| {
                        lower_rollup_resolved(scope.workspace, scope.resolved_module, rollup)
                    })
                    .collect(),
            }
        }
        HirStatementKind::While { condition, body } => {
            let mut body_scope = scope.clone();
            let body = lower_exec_stmt_block_resolved(body, &mut body_scope);
            ExecStmt::While {
                condition: lower_exec_expr_resolved(condition, scope),
                body,
                rollups: statement
                    .rollups
                    .iter()
                    .map(|rollup| {
                        lower_rollup_resolved(scope.workspace, scope.resolved_module, rollup)
                    })
                    .collect(),
            }
        }
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => {
            let mut body_scope = scope.clone();
            if let Some(type_text) = infer_iterable_binding_type_text(scope, iterable) {
                body_scope.value_scope.insert(binding.clone(), type_text);
            }
            let body = lower_exec_stmt_block_resolved(body, &mut body_scope);
            ExecStmt::For {
                binding: binding.clone(),
                iterable: lower_exec_expr_resolved(iterable, scope),
                body,
                rollups: statement
                    .rollups
                    .iter()
                    .map(|rollup| {
                        lower_rollup_resolved(scope.workspace, scope.resolved_module, rollup)
                    })
                    .collect(),
            }
        }
        HirStatementKind::Defer { expr } => ExecStmt::Defer(lower_exec_expr_resolved(expr, scope)),
        HirStatementKind::Break => ExecStmt::Break,
        HirStatementKind::Continue => ExecStmt::Continue,
        HirStatementKind::Assign { target, op, value } => {
            let lowered = ExecStmt::Assign {
                target: lower_assign_target_exec_resolved(target, scope),
                op: lower_assign_op(*op),
                value: lower_exec_expr_resolved(value, scope),
            };
            if matches!(op, HirAssignOp::Assign) {
                if let HirAssignTarget::Name { text } = target {
                    if let Some(type_text) = infer_expr_type_text(scope, value) {
                        scope.value_scope.insert(text.clone(), type_text);
                    }
                }
            }
            lowered
        }
    }
}

fn is_routine_symbol(symbol: &HirSymbol) -> bool {
    matches!(
        symbol.kind,
        HirSymbolKind::Fn | HirSymbolKind::System | HirSymbolKind::Behavior | HirSymbolKind::Const
    )
}

fn lower_routine(
    module_id: &str,
    routine_key: String,
    symbol: &HirSymbol,
    impl_decl: Option<&arcana_hir::HirImplDecl>,
) -> IrRoutine {
    IrRoutine {
        module_id: module_id.to_string(),
        routine_key,
        symbol_name: symbol.name.clone(),
        symbol_kind: symbol.kind.as_str().to_string(),
        exported: symbol.exported,
        is_async: symbol.is_async,
        type_param_rows: render_type_param_rows(symbol),
        behavior_attr_rows: render_behavior_attr_rows(symbol),
        param_rows: render_param_row(symbol),
        signature_row: symbol.surface_text.clone(),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        impl_target_type: impl_decl.map(|decl| decl.target_type.clone()),
        impl_trait_path: impl_decl.and_then(|decl| {
            decl.trait_path
                .as_ref()
                .map(|path| canonical_impl_trait_path(path))
        }),
        foreword_rows: symbol.forewords.iter().map(render_foreword_row).collect(),
        rollups: symbol.rollups.iter().map(lower_rollup).collect(),
        statements: lower_exec_stmt_block(&symbol.statements),
    }
}

fn lower_routine_resolved(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
    module_id: &str,
    routine_key: String,
    symbol: &HirSymbol,
    impl_decl: Option<&arcana_hir::HirImplDecl>,
) -> Result<IrRoutine, String> {
    let mut scope = ResolvedRenderScope::new(
        workspace,
        resolved_module,
        symbol.where_clause.as_deref(),
        &symbol.type_params,
    );
    for param in &symbol.params {
        scope
            .value_scope
            .insert(param.name.clone(), param.ty.clone());
    }
    let routine = IrRoutine {
        module_id: module_id.to_string(),
        routine_key,
        symbol_name: symbol.name.clone(),
        symbol_kind: symbol.kind.as_str().to_string(),
        exported: symbol.exported
            || impl_decl.is_some_and(|decl| {
                impl_target_is_public_from_package(workspace, package, module, &decl.target_type)
            }),
        is_async: symbol.is_async,
        type_param_rows: render_type_param_rows(symbol),
        behavior_attr_rows: render_behavior_attr_rows(symbol),
        param_rows: render_param_row(symbol),
        signature_row: symbol.surface_text.clone(),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        impl_target_type: impl_decl.map(|decl| decl.target_type.clone()),
        impl_trait_path: impl_decl.and_then(|decl| {
            decl.trait_path
                .as_ref()
                .map(|path| canonical_impl_trait_path(path))
        }),
        foreword_rows: symbol.forewords.iter().map(render_foreword_row).collect(),
        rollups: symbol
            .rollups
            .iter()
            .map(|rollup| lower_rollup_resolved(workspace, resolved_module, rollup))
            .collect(),
        statements: lower_exec_stmt_block_resolved(&symbol.statements, &mut scope),
    };
    scope.finish()?;
    Ok(routine)
}

fn lower_package(package: &HirPackageSummary) -> IrPackage {
    let modules = package
        .modules
        .iter()
        .map(|module| {
            let lowered = lower_module_summary(module);
            IrPackageModule {
                module_id: module.module_id.clone(),
                symbol_count: lowered.symbol_count,
                item_count: lowered.item_count,
                line_count: module.line_count,
                non_empty_line_count: module.non_empty_line_count,
                directive_rows: module
                    .directives
                    .iter()
                    .map(|directive| {
                        render_directive_row(
                            &module.module_id,
                            directive.kind,
                            &directive.path,
                            &directive.alias,
                        )
                    })
                    .collect(),
                lang_item_rows: module
                    .lang_items
                    .iter()
                    .map(|item| render_lang_item_row(&module.module_id, &item.name, &item.target))
                    .collect(),
                exported_surface_rows: module.summary_surface_rows(),
            }
        })
        .collect::<Vec<_>>();
    let dependency_rows = package
        .dependency_edges
        .iter()
        .map(render_dependency_row)
        .collect::<Vec<_>>();
    let entrypoints = package
        .modules
        .iter()
        .flat_map(|module| {
            module.symbols.iter().filter_map(|symbol| {
                let is_entry = symbol.kind == HirSymbolKind::System
                    || symbol.kind == HirSymbolKind::Behavior
                    || is_runtime_main_entry_symbol(
                        &package.package_name,
                        &module.module_id,
                        symbol,
                    );
                if !is_entry {
                    return None;
                }
                Some(IrEntrypoint {
                    module_id: module.module_id.clone(),
                    symbol_name: symbol.name.clone(),
                    symbol_kind: symbol.kind.as_str().to_string(),
                    is_async: symbol.is_async,
                    exported: symbol.exported,
                })
            })
        })
        .collect::<Vec<_>>();
    let routines =
        package
            .modules
            .iter()
            .flat_map(|module| {
                let mut routines = module
                    .symbols
                    .iter()
                    .enumerate()
                    .filter(|(_, symbol)| is_routine_symbol(symbol))
                    .map(|(symbol_index, symbol)| {
                        lower_routine(
                            &module.module_id,
                            routine_key_for_symbol(&module.module_id, symbol_index),
                            symbol,
                            None,
                        )
                    })
                    .collect::<Vec<_>>();
                routines.extend(module.impls.iter().enumerate().flat_map(
                    |(impl_index, impl_decl)| {
                        impl_decl
                            .methods
                            .iter()
                            .enumerate()
                            .filter(|(_, symbol)| is_routine_symbol(symbol))
                            .map(move |(method_index, symbol)| {
                                lower_routine(
                                    &module.module_id,
                                    routine_key_for_impl_method(
                                        &module.module_id,
                                        impl_index,
                                        method_index,
                                    ),
                                    symbol,
                                    Some(impl_decl),
                                )
                            })
                    },
                ));
                routines
            })
            .collect::<Vec<_>>();

    let mut lowered = IrPackage {
        package_name: package.package_name.clone(),
        root_module_id: package.package_name.clone(),
        direct_deps: Vec::new(),
        modules,
        dependency_edge_count: package.dependency_edges.len(),
        dependency_rows,
        exported_surface_rows: package.summary_surface_rows(),
        runtime_requirements: Vec::new(),
        entrypoints,
        routines,
    };
    lowered.runtime_requirements = derive_runtime_requirements(&lowered);
    lowered
}

#[cfg(test)]
fn lower_workspace_package(package: &HirWorkspacePackage) -> IrPackage {
    let mut lowered = lower_package(&package.summary);
    lowered.direct_deps = package.direct_deps.iter().cloned().collect();
    lowered
}

pub fn lower_workspace_package_with_resolution(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    package: &HirWorkspacePackage,
) -> Result<IrPackage, String> {
    let mut lowered = lower_package(&package.summary);
    lowered.direct_deps = resolved_direct_deps(package);
    lowered.dependency_rows = package
        .summary
        .dependency_edges
        .iter()
        .map(|edge| render_resolved_dependency_row(package, edge))
        .collect();
    lowered.dependency_edge_count = lowered.dependency_rows.len();
    let Some(resolved_package) = resolved_workspace.package(&package.summary.package_name) else {
        return Ok(lowered);
    };
    lowered.routines = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                return Ok(Vec::new());
            };
            let mut routines = module
                .symbols
                .iter()
                .enumerate()
                .filter(|(_, symbol)| is_routine_symbol(symbol))
                .map(|(symbol_index, symbol)| {
                    lower_routine_resolved(
                        workspace,
                        package,
                        resolved_module,
                        module,
                        &module.module_id,
                        routine_key_for_symbol(&module.module_id, symbol_index),
                        symbol,
                        None,
                    )
                })
                .collect::<Result<Vec<_>, String>>()?;
            routines.extend(
                module
                    .impls
                    .iter()
                    .enumerate()
                    .flat_map(|(impl_index, impl_decl)| {
                        impl_decl
                            .methods
                            .iter()
                            .enumerate()
                            .filter(|(_, symbol)| is_routine_symbol(symbol))
                            .map(move |(method_index, symbol)| {
                                lower_routine_resolved(
                                    workspace,
                                    package,
                                    resolved_module,
                                    module,
                                    &module.module_id,
                                    routine_key_for_impl_method(
                                        &module.module_id,
                                        impl_index,
                                        method_index,
                                    ),
                                    symbol,
                                    Some(impl_decl),
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            );
            Ok(routines)
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    let module_surface_rows = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let rows = resolved_package
                .module(&module.module_id)
                .map(|_| resolved_module_exported_surface_rows(workspace, package, module))
                .unwrap_or_else(|| module.summary_surface_rows());
            (module.module_id.clone(), rows)
        })
        .collect::<BTreeMap<_, _>>();
    for module in &mut lowered.modules {
        if let Some(rows) = module_surface_rows.get(&module.module_id) {
            module.exported_surface_rows = rows.clone();
        }
    }
    lowered.exported_surface_rows = package
        .summary
        .modules
        .iter()
        .flat_map(|module| {
            module_surface_rows
                .get(&module.module_id)
                .into_iter()
                .flatten()
                .map(|row| format!("module={}:{}", module.module_id, row))
                .collect::<Vec<_>>()
        })
        .collect();
    lowered.runtime_requirements = derive_runtime_requirements(&lowered);
    Ok(lowered)
}

#[cfg(test)]
mod tests {
    use super::{
        ExecExpr, ExecStmt, IrModule, lower_hir, lower_package, lower_workspace_package,
        lower_workspace_package_with_resolution,
    };
    use arcana_hir::{
        HirModule, build_package_layout, build_package_summary, build_workspace_package,
        build_workspace_package_with_dep_packages, build_workspace_summary, lower_module_text,
        resolve_workspace,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    #[test]
    fn lower_hir_preserves_counts() {
        let hir = HirModule {
            symbol_count: 2,
            item_count: 7,
        };
        let ir: IrModule = lower_hir(&hir);
        assert_eq!(ir.symbol_count, 2);
        assert_eq!(ir.item_count, 7);
    }

    #[test]
    fn lower_package_preserves_public_surface_rows() {
        let summary = build_package_summary(
            "winspell",
            vec![
                lower_module_text(
                    "winspell",
                    "reexport winspell.window\nexport fn open() -> Int:\n    return 0\n",
                )
                .expect("root module should lower"),
                lower_module_text(
                    "winspell.window",
                    "import std.canvas\nexport record Window:\n    title: Text\n",
                )
                .expect("nested module should lower"),
            ],
        );

        let ir = lower_package(&summary);
        assert_eq!(ir.package_name, "winspell");
        assert_eq!(ir.root_module_id, "winspell");
        assert_eq!(ir.module_count(), 2);
        assert_eq!(ir.dependency_edge_count, 2);
        assert_eq!(
            ir.exported_surface_rows,
            vec![
                "module=winspell.window:export:record:record Window:\\ntitle: Text".to_string(),
                "module=winspell:export:fn:fn open() -> Int:".to_string(),
                "module=winspell:reexport:winspell.window".to_string(),
            ]
        );
        assert!(ir.runtime_requirements.is_empty());
        assert!(ir.entrypoints.is_empty());
        assert_eq!(ir.routines.len(), 1);
        assert_eq!(ir.routines[0].symbol_name, "open");
        assert!(ir.routines[0].param_rows.is_empty());
        assert_eq!(
            ir.routines[0].statements,
            vec![ExecStmt::ReturnValue {
                value: ExecExpr::Int(0),
            }]
        );
        assert!(
            ir.dependency_rows
                .iter()
                .any(|row| row.contains("std.canvas"))
        );
    }

    #[test]
    fn lower_workspace_package_preserves_direct_deps() {
        let summary = build_package_summary(
            "desktop",
            vec![
                lower_module_text("desktop", "export fn main() -> Int:\n    return 0\n")
                    .expect("root module should lower"),
            ],
        );
        let layout = build_package_layout(
            &summary,
            BTreeMap::from([(
                "desktop".to_string(),
                Path::new("C:/repo/desktop/src/shelf.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("layout should build");
        let workspace = build_workspace_package(
            Path::new("C:/repo/desktop").to_path_buf(),
            BTreeSet::from(["core".to_string(), "std".to_string()]),
            summary,
            layout,
        )
        .expect("workspace should build");

        let ir = lower_workspace_package(&workspace);
        assert_eq!(ir.direct_deps, vec!["core".to_string(), "std".to_string()]);
    }

    #[test]
    fn lower_package_includes_impl_methods_as_routines() {
        let summary = build_package_summary(
            "records",
            vec![
                lower_module_text(
                    "records",
                    "record Counter:\n    value: Int\nimpl Counter:\n    fn double(read self: Counter) -> Int:\n        return self.value * 2\nfn main() -> Int:\n    return 0\n",
                )
                .expect("module should lower"),
            ],
        );

        let ir = lower_package(&summary);
        assert!(
            ir.routines
                .iter()
                .any(|routine| routine.module_id == "records" && routine.symbol_name == "double"),
            "expected impl method to be lowered into routine rows"
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_emits_resolved_bare_method_paths() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text(
                    "std.collections.list",
                    "impl List[T]:\n    fn len(read self: List[T]) -> Int:\n        return 0\n",
                )
                .expect("std module should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.collections.list".to_string(),
                Path::new("C:/repo/std/src/collections/list.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_workspace = build_workspace_package(
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std workspace should build");

        let app_summary = build_package_summary(
            "app",
            vec![lower_module_text(
                "app",
                "import std.collections.list\nfn main() -> Int:\n    let xs = [1]\n    return xs :: :: len\n",
            )
            .expect("app module should lower")],
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
        let app_workspace = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace = build_workspace_summary(vec![std_workspace, app_workspace])
            .expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let lowered = format!("{:?}", main.statements);
        assert!(
            lowered
                .contains("resolved_callable: Some([\"std\", \"collections\", \"list\", \"len\"])"),
            "expected resolved bare-method callable path in lowered statements: {lowered}",
        );
        assert!(
            lowered.contains("resolved_routine: Some(\"std.collections.list#impl-0-method-0\")"),
            "expected resolved bare-method routine identity in lowered statements: {lowered}",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_resolves_bare_methods_on_generic_enum_values() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text(
                    "std.result",
                    concat!(
                        "export enum Result[T, E]:\n",
                        "    Ok(T)\n",
                        "    Err(E)\n",
                        "impl[T, E] Result[T, E]:\n",
                        "    fn is_ok(read self: Result[T, E]) -> Bool:\n",
                        "        return true\n",
                        "    fn unwrap_or(read self: Result[T, E], take fallback: T) -> T:\n",
                        "        return fallback\n",
                    ),
                )
                .expect("std result module should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.result".to_string(),
                Path::new("C:/repo/std/src/result.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_workspace = build_workspace_package(
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std workspace should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "import std.result\n",
                        "use std.result.Result\n",
                        "fn main() -> Int:\n",
                        "    let ok = Result.Ok[Int, Str] :: 7 :: call\n",
                        "    let check = ok :: :: is_ok\n",
                        "    return ok :: 13 :: unwrap_or\n",
                    ),
                )
                .expect("app module should lower"),
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
        let app_workspace = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace = build_workspace_summary(vec![std_workspace, app_workspace])
            .expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let lowered = format!("{:?}", main.statements);
        assert!(
            lowered.contains("qualifier: \"is_ok\"")
                && lowered.contains("resolved_callable: Some([\"std\", \"result\", \"is_ok\"])")
                && lowered.contains("resolved_routine: Some(\"std.result#impl-0-method-0\")"),
            "expected resolved Result.is_ok bare method in lowered statements: {lowered}",
        );
        assert!(
            lowered.contains("qualifier: \"unwrap_or\"")
                && lowered
                    .contains("resolved_callable: Some([\"std\", \"result\", \"unwrap_or\"])")
                && lowered.contains("resolved_routine: Some(\"std.result#impl-0-method-1\")"),
            "expected resolved Result.unwrap_or bare method in lowered statements: {lowered}",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_resolves_bare_methods_on_spawn_handles() {
        let std_summary = build_package_summary(
            "std",
            vec![
                lower_module_text(
                    "std.concurrent",
                    concat!(
                        "impl[T] Task[T]:\n",
                        "    fn done(read self: Task[T]) -> Bool:\n",
                        "        return false\n",
                        "    fn join(read self: Task[T]) -> T:\n",
                        "        return 0\n",
                        "impl[T] Thread[T]:\n",
                        "    fn done(read self: Thread[T]) -> Bool:\n",
                        "        return false\n",
                        "    fn join(read self: Thread[T]) -> T:\n",
                        "        return 0\n",
                    ),
                )
                .expect("std concurrent module should lower"),
            ],
        );
        let std_layout = build_package_layout(
            &std_summary,
            BTreeMap::from([(
                "std.concurrent".to_string(),
                Path::new("C:/repo/std/src/concurrent.arc").to_path_buf(),
            )]),
            BTreeMap::new(),
        )
        .expect("std layout should build");
        let std_workspace = build_workspace_package(
            Path::new("C:/repo/std").to_path_buf(),
            BTreeSet::new(),
            std_summary,
            std_layout,
        )
        .expect("std workspace should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "fn worker() -> Int:\n",
                        "    return 1\n",
                        "fn helper() -> Int:\n",
                        "    return 2\n",
                        "fn main() -> Int:\n",
                        "    let task = weave worker :: :: call\n",
                        "    let thread = split helper :: :: call\n",
                        "    if task :: :: done:\n",
                        "        return thread :: :: join\n",
                        "    return task :: :: join\n",
                    ),
                )
                .expect("app module should lower"),
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
        let app_workspace = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::from(["std".to_string()]),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace = build_workspace_summary(vec![std_workspace, app_workspace])
            .expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        let lowered = format!("{:?}", main.statements);
        assert!(
            lowered.contains("qualifier: \"done\"")
                && lowered.contains("resolved_callable: Some([\"std\", \"concurrent\", \"done\"])"),
            "expected resolved Task.done bare method on spawned handle: {lowered}",
        );
        assert!(
            lowered.contains("qualifier: \"join\"")
                && lowered.contains("resolved_callable: Some([\"std\", \"concurrent\", \"join\"])")
                && (lowered.contains("resolved_routine: Some(\"std.concurrent#impl-1-method-1\")")
                    || lowered
                        .contains("resolved_routine: Some(\"std.concurrent#impl-0-method-1\")")),
            "expected resolved join bare method on spawned handle: {lowered}",
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_rejects_ambiguous_concrete_bare_methods() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "record Counter:\n",
                        "    value: Int\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 1\n",
                        "impl Counter:\n",
                        "    fn tap(read self: Counter) -> Int:\n",
                        "        return self.value + 2\n",
                        "fn main() -> Int:\n",
                        "    let counter = Counter :: value = 1 :: call\n",
                        "    return counter :: :: tap\n",
                    ),
                )
                .expect("module should lower"),
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
        let app_workspace = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let err = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect_err("ambiguous concrete bare method should fail lowering");
        assert!(
            err.contains("bare-method qualifier `tap` on `app.Counter` is ambiguous"),
            "{err}"
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_canonicalizes_dependency_alias_metadata() {
        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text("core", "export fn value() -> Int:\n    return 7\n")
                    .expect("core module should lower"),
            ],
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
        let core_workspace = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core package should build");

        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    "import util\nfn main() -> Int:\n    return util.value :: :: call\n",
                )
                .expect("app module should lower"),
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
        let app_workspace = build_workspace_package_with_dep_packages(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeMap::from([("util".to_string(), "core".to_string())]),
            app_summary,
            app_layout,
        )
        .expect("app package should build");

        let workspace =
            build_workspace_summary(vec![app_workspace, core_workspace]).expect("workspace");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        assert_eq!(ir.direct_deps, vec!["core".to_string()]);
        assert_eq!(
            ir.dependency_rows,
            vec!["source=app:import:core:".to_string()]
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_canonicalizes_rollup_handler_paths() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "import app.handlers\n",
                        "fn main() -> Int:\n",
                        "    let value = 1\n",
                        "    return 0\n",
                        "[value, handlers.cleanup]#cleanup\n",
                    ),
                )
                .expect("root module should lower"),
                lower_module_text(
                    "app.handlers",
                    "export fn cleanup(value: Int) -> Int:\n    return value\n",
                )
                .expect("handler module should lower"),
            ],
        );
        let app_layout = build_package_layout(
            &app_summary,
            BTreeMap::from([
                (
                    "app".to_string(),
                    Path::new("C:/repo/app/src/shelf.arc").to_path_buf(),
                ),
                (
                    "app.handlers".to_string(),
                    Path::new("C:/repo/app/src/handlers.arc").to_path_buf(),
                ),
            ]),
            BTreeMap::new(),
        )
        .expect("app layout should build");
        let app_workspace = build_workspace_package(
            Path::new("C:/repo/app").to_path_buf(),
            BTreeSet::new(),
            app_summary,
            app_layout,
        )
        .expect("app workspace should build");

        let workspace =
            build_workspace_summary(vec![app_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace.package("app").expect("app package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");

        assert_eq!(main.rollups.len(), 1);
        assert_eq!(
            main.rollups[0].handler_path,
            vec![
                "app".to_string(),
                "handlers".to_string(),
                "cleanup".to_string()
            ]
        );
    }

    #[test]
    fn lower_workspace_package_with_resolution_marks_public_impl_methods_exported() {
        let core_summary = build_package_summary(
            "core",
            vec![
                lower_module_text(
                    "core",
                    concat!(
                        "export record Counter:\n",
                        "    value: Int\n",
                        "impl Counter:\n",
                        "    fn announce(read self: Counter) -> Int:\n",
                        "        return self.value\n",
                    ),
                )
                .expect("core module should lower"),
            ],
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
        let core_workspace = build_workspace_package(
            Path::new("C:/repo/core").to_path_buf(),
            BTreeSet::new(),
            core_summary,
            core_layout,
        )
        .expect("core workspace should build");

        let workspace =
            build_workspace_summary(vec![core_workspace]).expect("workspace should build");
        let resolved = resolve_workspace(&workspace).expect("workspace should resolve");
        let package = workspace
            .package("core")
            .expect("core package should exist");

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package)
            .expect("workspace lowering should succeed");
        let announce = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "announce")
            .expect("impl method should lower");

        assert!(announce.exported);
    }
}
