mod entrypoint;
mod executable;
mod routine_signature;
mod runtime_requirements;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use arcana_hir::{
    HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer, HirChainStep,
    HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg, HirHeaderAttachment,
    HirLocalTypeLookup, HirMatchPattern, HirModule, HirModuleDependency, HirModuleSummary,
    HirPackageSummary, HirPageRollup, HirPath, HirPhraseArg, HirResolvedModule,
    HirResolvedWorkspace, HirStatement, HirStatementKind, HirSymbol, HirSymbolBody, HirSymbolKind,
    HirType, HirTypeKind, HirUnaryOp, HirWorkspacePackage, HirWorkspaceSummary,
    impl_target_is_public_from_package, infer_receiver_expr_type,
    lookup_method_candidates_for_hir_type, lookup_symbol_path,
    match_name_resolves_to_zero_payload_variant, render_symbol_signature,
    routine_key_for_impl_method, routine_key_for_object_method, routine_key_for_symbol,
};
pub use entrypoint::{
    RUNTIME_MAIN_ENTRYPOINT_NAME, is_runtime_main_entry_symbol,
    validate_runtime_main_entry_contract, validate_runtime_main_entry_symbol,
};
pub use runtime_requirements::{
    RuntimeRequirementRoots, derive_runtime_requirements, derive_runtime_requirements_with_roots,
};

pub use executable::{
    ExecAssignOp, ExecAssignTarget, ExecAvailabilityAttachment, ExecAvailabilityKind, ExecBinaryOp,
    ExecChainConnector, ExecChainIntroducer, ExecChainStep, ExecDynamicDispatch, ExecExpr,
    ExecHeaderAttachment, ExecMatchArm, ExecMatchPattern, ExecPageRollup, ExecPhraseArg,
    ExecPhraseQualifierKind, ExecStmt, ExecUnaryOp,
};
pub use routine_signature::{IrRoutineParam, render_routine_signature_text};

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
    pub type_params: Vec<String>,
    pub behavior_attrs: BTreeMap<String, String>,
    pub params: Vec<IrRoutineParam>,
    pub return_type: Option<String>,
    pub intrinsic_impl: Option<String>,
    pub impl_target_type: Option<String>,
    pub impl_trait_path: Option<Vec<String>>,
    pub availability: Vec<ExecAvailabilityAttachment>,
    pub foreword_rows: Vec<String>,
    pub rollups: Vec<ExecPageRollup>,
    pub statements: Vec<ExecStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOwnerObject {
    pub type_path: Vec<String>,
    pub local_name: String,
    pub init_routine_key: Option<String>,
    pub init_with_context_routine_key: Option<String>,
    pub resume_routine_key: Option<String>,
    pub resume_with_context_routine_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOwnerExit {
    pub name: String,
    pub condition: ExecExpr,
    pub holds: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrOwnerDecl {
    pub module_id: String,
    pub owner_path: Vec<String>,
    pub owner_name: String,
    pub objects: Vec<IrOwnerObject>,
    pub exits: Vec<IrOwnerExit>,
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
    pub owners: Vec<IrOwnerDecl>,
}

impl IrPackage {
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }
}

#[derive(Clone, Debug, Default)]
struct LowerValueScope {
    locals: BTreeMap<String, HirType>,
    owner_member_types: BTreeMap<String, BTreeMap<String, HirType>>,
}

impl LowerValueScope {
    fn contains(&self, name: &str) -> bool {
        self.locals.contains_key(name)
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        self.locals.get(name)
    }

    fn insert(&mut self, name: impl Into<String>, ty: HirType) {
        self.locals.insert(name.into(), ty);
    }

    fn owner_member_type(&self, owner_name: &str, member: &str) -> Option<&HirType> {
        self.owner_member_types
            .get(owner_name)
            .and_then(|members| members.get(member))
    }

    fn activate_owner(
        &mut self,
        owner_local_name: &str,
        owner_path: &[String],
        objects: &[(String, HirType)],
        explicit_binding: Option<&str>,
    ) {
        let owner_type = synthetic_hir_type(format!("Owner<{}>", owner_path.join(".")));
        let mut owner_members = BTreeMap::new();
        self.insert(owner_local_name.to_string(), owner_type.clone());
        for (local_name, ty) in objects {
            self.insert(local_name.clone(), ty.clone());
            owner_members.insert(local_name.clone(), ty.clone());
        }
        self.owner_member_types
            .insert(owner_local_name.to_string(), owner_members.clone());
        if let Some(binding) = explicit_binding {
            self.insert(binding.to_string(), owner_type);
            self.owner_member_types
                .insert(binding.to_string(), owner_members);
        }
    }
}

impl HirLocalTypeLookup for LowerValueScope {
    fn contains_local(&self, name: &str) -> bool {
        LowerValueScope::contains(self, name)
    }

    fn type_of(&self, name: &str) -> Option<&HirType> {
        LowerValueScope::type_of(self, name)
    }
}

fn simple_hir_type(name: &str) -> HirType {
    synthetic_hir_type(name.to_string())
}

fn synthetic_hir_type(name: String) -> HirType {
    HirType {
        kind: HirTypeKind::Path(HirPath {
            segments: vec![name],
            span: Default::default(),
        }),
        span: Default::default(),
    }
}

fn pair_hir_type(left: HirType, right: HirType) -> HirType {
    HirType {
        kind: HirTypeKind::Apply {
            base: HirPath {
                segments: vec!["Pair".to_string()],
                span: Default::default(),
            },
            args: vec![left, right],
        },
        span: Default::default(),
    }
}

fn hir_path_matches(path: &HirPath, expected: &[&str]) -> bool {
    path.segments
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
}

fn hir_path_matches_any(path: &HirPath, expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|candidate| hir_path_matches(path, candidate))
}

#[derive(Clone, Debug)]
struct ResolvedRenderScope<'a> {
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    current_where_clause: Option<String>,
    value_scope: LowerValueScope,
    errors: Rc<RefCell<Vec<String>>>,
}

impl<'a> ResolvedRenderScope<'a> {
    fn new(
        workspace: &'a HirWorkspaceSummary,
        resolved_module: &'a HirResolvedModule,
        current_where_clause: Option<String>,
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

fn resolved_module_lang_item_rows(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
) -> Vec<String> {
    module
        .lang_items
        .iter()
        .map(|item| {
            let target = lookup_symbol_path(workspace, resolved_module, &item.target)
                .map(resolved_symbol_path)
                .unwrap_or_else(|| item.target.clone());
            render_lang_item_row(&module.module_id, &item.name, &target)
        })
        .collect()
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
                encode_surface_text(&render_symbol_signature(method))
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "impl:target={}:trait={}:methods=[{}]",
        encode_surface_text(&impl_decl.target_type.render()),
        encode_surface_text(
            &impl_decl
                .trait_path
                .as_ref()
                .map(|path| path.render())
                .unwrap_or_default(),
        ),
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

fn canonical_impl_trait_path(path: &arcana_hir::HirTraitRef) -> Vec<String> {
    path.path.segments.clone()
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
    ty: &HirType,
    method_name: &str,
    candidates: &[ResolvedMethod<'_>],
) -> String {
    let rendered = candidates
        .iter()
        .map(|candidate| {
            format!(
                "{} [{}]",
                render_symbol_signature(candidate.method),
                candidate.routine_key
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "bare-method qualifier `{method_name}` on `{}` is ambiguous; candidates: {rendered}",
        ty.render()
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

struct ResolvedOwnerActivation<'a> {
    owner_path: Vec<String>,
    owner_local_name: String,
    objects: Vec<(String, HirType)>,
    context: Option<&'a HirExpr>,
}

fn lookup_method_resolution_for_type<'a>(
    scope: &'a ResolvedRenderScope<'a>,
    workspace: &'a HirWorkspaceSummary,
    ty: &HirType,
    method_name: &str,
) -> Result<Option<ResolvedMethod<'a>>, String> {
    let candidates =
        lookup_method_candidates_for_hir_type(workspace, scope.resolved_module, ty, method_name)
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
        _ => Err(format_method_ambiguity(ty, method_name, &candidates)),
    }
}

fn lookup_trait_method_resolution_from_where_clause<'a>(
    scope: &'a ResolvedRenderScope<'a>,
    ty: &HirType,
    method_name: &str,
) -> Result<Vec<ResolvedMethod<'a>>, String> {
    let rendered = ty.render();
    let wanted = erase_type_generics(strip_reference_prefix(&rendered));
    if !is_identifier_text(&wanted) {
        return Ok(Vec::new());
    }
    let Some(where_clause) = scope.current_where_clause.as_deref() else {
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

fn lower_routine_params(symbol: &HirSymbol) -> Vec<IrRoutineParam> {
    symbol
        .params
        .iter()
        .map(|param| IrRoutineParam {
            mode: param.mode.map(|mode| mode.as_str().to_string()),
            name: param.name.clone(),
            ty: param.ty.render(),
        })
        .collect()
}

fn lower_behavior_attrs(symbol: &HirSymbol) -> BTreeMap<String, String> {
    symbol
        .behavior_attrs
        .iter()
        .map(|attr| (attr.name.clone(), attr.value.clone()))
        .collect()
}

fn infer_iterable_binding_type(
    scope: &ResolvedRenderScope<'_>,
    iterable: &HirExpr,
) -> Option<HirType> {
    let iterable_ty = infer_expr_hir_type(scope, iterable)?;
    match &iterable_ty.kind {
        HirTypeKind::Path(path) if hir_path_matches(path, &["RangeInt"]) => {
            Some(simple_hir_type("Int"))
        }
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(
                base,
                &[
                    &["List"],
                    &["Array"],
                    &["std", "collections", "list", "List"],
                    &["std", "collections", "array", "Array"],
                ],
            ) =>
        {
            args.first().cloned()
        }
        HirTypeKind::Apply { base, args }
            if hir_path_matches_any(base, &[&["Map"], &["std", "collections", "map", "Map"]]) =>
        {
            match (args.first(), args.get(1)) {
                (Some(key), Some(value)) => Some(pair_hir_type(key.clone(), value.clone())),
                _ => None,
            }
        }
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
    let subject_ty = infer_receiver_expr_type(
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
                    "bare-method qualifier `{qualifier}` on `{}` is ambiguous across trait bounds",
                    subject_ty.render()
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

fn infer_expr_hir_type(scope: &ResolvedRenderScope<'_>, expr: &HirExpr) -> Option<HirType> {
    if let HirExpr::MemberAccess { expr, member } = expr {
        if let HirExpr::Path { segments } = expr.as_ref() {
            if segments.len() == 1 && scope.value_scope.contains(&segments[0]) {
                if let Some(ty) = scope.value_scope.owner_member_type(&segments[0], member) {
                    return Some(ty.clone());
                }
            }
        }
    }
    if let Some(inferred) = infer_receiver_expr_type(
        scope.workspace,
        scope.resolved_module,
        &scope.value_scope,
        expr,
    ) {
        return Some(inferred);
    }
    match expr {
        HirExpr::Pair { left, right } => Some(pair_hir_type(
            infer_expr_hir_type(scope, left)?,
            infer_expr_hir_type(scope, right)?,
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

fn resolve_owner_activation_expr<'a>(
    scope: &ResolvedRenderScope<'_>,
    expr: &'a HirExpr,
) -> Result<Option<ResolvedOwnerActivation<'a>>, String> {
    let HirExpr::QualifiedPhrase {
        subject,
        args,
        qualifier,
        ..
    } = expr
    else {
        return Ok(None);
    };
    if qualifier != "call" {
        return Ok(None);
    }
    let Some(path) = flatten_callable_expr_path(subject) else {
        return Ok(None);
    };
    let Some(resolved) = lookup_symbol_path(scope.workspace, scope.resolved_module, &path) else {
        return Ok(None);
    };
    if resolved.symbol.kind != HirSymbolKind::Owner {
        return Ok(None);
    }
    if args
        .iter()
        .any(|arg| matches!(arg, HirPhraseArg::Named { .. }))
    {
        return Err(format!(
            "owner activation `{}` does not support named arguments",
            path.join(".")
        ));
    }
    if args.len() > 1 {
        return Err(format!(
            "owner activation `{}` accepts at most one context argument",
            path.join(".")
        ));
    }
    let HirSymbolBody::Owner { objects, .. } = &resolved.symbol.body else {
        return Ok(None);
    };
    Ok(Some(ResolvedOwnerActivation {
        owner_path: resolved_symbol_path(resolved),
        owner_local_name: resolved.symbol.name.clone(),
        objects: objects
            .iter()
            .map(|object| {
                (
                    object.local_name.clone(),
                    HirType {
                        kind: HirTypeKind::Path(HirPath {
                            segments: object.type_path.clone(),
                            span: object.span,
                        }),
                        span: object.span,
                    },
                )
            })
            .collect(),
        context: args.first().and_then(|arg| match arg {
            HirPhraseArg::Positional(expr) => Some(expr),
            HirPhraseArg::Named { .. } => None,
        }),
    }))
}

fn lower_availability_attachment_exec(
    attachment: &arcana_hir::HirAvailabilityAttachment,
) -> ExecAvailabilityAttachment {
    ExecAvailabilityAttachment {
        kind: ExecAvailabilityKind::Object,
        path: attachment.path.clone(),
        local_name: attachment.path.last().cloned().unwrap_or_default(),
    }
}

fn canonical_symbol_path(module_id: &str, symbol_name: &str) -> Vec<String> {
    let mut path = module_id.split('.').map(str::to_string).collect::<Vec<_>>();
    path.push(symbol_name.to_string());
    path
}

fn lower_availability_attachment_exec_resolved(
    attachment: &arcana_hir::HirAvailabilityAttachment,
    scope: &ResolvedRenderScope<'_>,
) -> Result<ExecAvailabilityAttachment, String> {
    let resolved = lookup_symbol_path(scope.workspace, scope.resolved_module, &attachment.path)
        .ok_or_else(|| {
            format!(
                "unresolved availability attachment `{}`",
                attachment.path.join(".")
            )
        })?;
    let kind = match resolved.symbol.kind {
        HirSymbolKind::Owner => ExecAvailabilityKind::Owner,
        HirSymbolKind::Object => ExecAvailabilityKind::Object,
        other => {
            return Err(format!(
                "availability attachment `{}` must resolve to owner or object, found `{}`",
                attachment.path.join("."),
                other.as_str()
            ));
        }
    };
    Ok(ExecAvailabilityAttachment {
        kind,
        path: canonical_symbol_path(resolved.module_id, &resolved.symbol.name),
        local_name: resolved.symbol.name.clone(),
    })
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
            type_args: type_args.iter().map(arcana_hir::HirType::render).collect(),
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
            type_args: type_args.iter().map(arcana_hir::HirType::render).collect(),
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
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
            rollups: statement.rollups.iter().map(lower_rollup).collect(),
        },
        HirStatementKind::While { condition, body } => ExecStmt::While {
            condition: lower_exec_expr(condition),
            body: lower_exec_stmt_block(body),
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
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
            availability: statement
                .availability
                .iter()
                .map(lower_availability_attachment_exec)
                .collect(),
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
) -> Result<Vec<ExecStmt>, String> {
    statements
        .iter()
        .map(|statement| lower_exec_stmt_resolved(statement, scope))
        .collect()
}

fn lower_exec_stmt_resolved(
    statement: &HirStatement,
    scope: &mut ResolvedRenderScope<'_>,
) -> Result<ExecStmt, String> {
    Ok(match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => {
            if let Some(owner_activation) = resolve_owner_activation_expr(scope, value)? {
                let lowered_context = owner_activation
                    .context
                    .map(|expr| lower_exec_expr_resolved(expr, scope));
                scope.value_scope.activate_owner(
                    &owner_activation.owner_local_name,
                    &owner_activation.owner_path,
                    &owner_activation.objects,
                    Some(name),
                );
                return Ok(ExecStmt::ActivateOwner {
                    owner_path: owner_activation.owner_path,
                    owner_local_name: owner_activation.owner_local_name,
                    binding: Some(name.clone()),
                    context: lowered_context,
                });
            }
            let lowered = ExecStmt::Let {
                mutable: *mutable,
                name: name.clone(),
                value: lower_exec_expr_resolved(value, scope),
            };
            if let Some(ty) = infer_expr_hir_type(scope, value) {
                scope.value_scope.insert(name.clone(), ty);
            }
            lowered
        }
        HirStatementKind::Expr { expr } => {
            if let Some(owner_activation) = resolve_owner_activation_expr(scope, expr)? {
                let lowered_context = owner_activation
                    .context
                    .map(|value| lower_exec_expr_resolved(value, scope));
                scope.value_scope.activate_owner(
                    &owner_activation.owner_local_name,
                    &owner_activation.owner_path,
                    &owner_activation.objects,
                    None,
                );
                ExecStmt::ActivateOwner {
                    owner_path: owner_activation.owner_path,
                    owner_local_name: owner_activation.owner_local_name,
                    binding: None,
                    context: lowered_context,
                }
            } else {
                ExecStmt::Expr {
                    expr: lower_exec_expr_resolved(expr, scope),
                    rollups: statement
                        .rollups
                        .iter()
                        .map(|rollup| {
                            lower_rollup_resolved(scope.workspace, scope.resolved_module, rollup)
                        })
                        .collect(),
                }
            }
        }
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
            let then_branch = lower_exec_stmt_block_resolved(then_branch, &mut then_scope)?;
            let else_branch = else_branch
                .as_ref()
                .map(|branch| {
                    let mut else_scope = scope.clone();
                    lower_exec_stmt_block_resolved(branch, &mut else_scope)
                })
                .transpose()?
                .unwrap_or_default();
            ExecStmt::If {
                condition: lower_exec_expr_resolved(condition, scope),
                then_branch,
                else_branch,
                availability: statement
                    .availability
                    .iter()
                    .map(|attachment| {
                        lower_availability_attachment_exec_resolved(attachment, scope)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
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
            let body = lower_exec_stmt_block_resolved(body, &mut body_scope)?;
            ExecStmt::While {
                condition: lower_exec_expr_resolved(condition, scope),
                body,
                availability: statement
                    .availability
                    .iter()
                    .map(|attachment| {
                        lower_availability_attachment_exec_resolved(attachment, scope)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
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
            if let Some(ty) = infer_iterable_binding_type(scope, iterable) {
                body_scope.value_scope.insert(binding.clone(), ty);
            }
            let body = lower_exec_stmt_block_resolved(body, &mut body_scope)?;
            ExecStmt::For {
                binding: binding.clone(),
                iterable: lower_exec_expr_resolved(iterable, scope),
                body,
                availability: statement
                    .availability
                    .iter()
                    .map(|attachment| {
                        lower_availability_attachment_exec_resolved(attachment, scope)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
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
                    if let Some(ty) = infer_expr_hir_type(scope, value) {
                        scope.value_scope.insert(text.clone(), ty);
                    }
                }
            }
            lowered
        }
    })
}

fn is_routine_symbol(symbol: &HirSymbol) -> bool {
    matches!(
        symbol.kind,
        HirSymbolKind::Fn | HirSymbolKind::System | HirSymbolKind::Behavior | HirSymbolKind::Const
    )
}

fn lower_object_method_routines(module: &HirModuleSummary) -> Vec<IrRoutine> {
    module
        .symbols
        .iter()
        .enumerate()
        .flat_map(|(symbol_index, symbol)| {
            let HirSymbolBody::Object { methods, .. } = &symbol.body else {
                return Vec::new();
            };
            methods
                .iter()
                .enumerate()
                .filter(|(_, method)| is_routine_symbol(method))
                .map(|(method_index, method)| {
                    lower_routine(
                        &module.module_id,
                        routine_key_for_object_method(
                            &module.module_id,
                            symbol_index,
                            method_index,
                        ),
                        method,
                        None,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn lower_object_method_routines_resolved(
    workspace: &HirWorkspaceSummary,
    package: &HirWorkspacePackage,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
) -> Result<Vec<IrRoutine>, String> {
    module
        .symbols
        .iter()
        .enumerate()
        .flat_map(|(symbol_index, symbol)| {
            let HirSymbolBody::Object { methods, .. } = &symbol.body else {
                return Vec::new();
            };
            methods
                .iter()
                .enumerate()
                .filter(|(_, method)| is_routine_symbol(method))
                .map(move |(method_index, method)| {
                    lower_routine_resolved(
                        workspace,
                        package,
                        resolved_module,
                        module,
                        &module.module_id,
                        routine_key_for_object_method(
                            &module.module_id,
                            symbol_index,
                            method_index,
                        ),
                        method,
                        None,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn lower_owner_decl(module: &HirModuleSummary, symbol: &HirSymbol) -> Option<IrOwnerDecl> {
    let HirSymbolBody::Owner { objects, exits } = &symbol.body else {
        return None;
    };
    Some(IrOwnerDecl {
        module_id: module.module_id.clone(),
        owner_path: canonical_symbol_path(&module.module_id, &symbol.name),
        owner_name: symbol.name.clone(),
        objects: objects
            .iter()
            .map(|object| IrOwnerObject {
                type_path: object.type_path.clone(),
                local_name: object.local_name.clone(),
                init_routine_key: None,
                init_with_context_routine_key: None,
                resume_routine_key: None,
                resume_with_context_routine_key: None,
            })
            .collect(),
        exits: exits
            .iter()
            .map(|owner_exit| IrOwnerExit {
                name: owner_exit.name.clone(),
                condition: lower_exec_expr(&owner_exit.condition),
                holds: owner_exit.holds.clone(),
            })
            .collect(),
    })
}

fn resolve_object_lifecycle_routine_keys(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    type_path: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let Some(resolved) = lookup_symbol_path(workspace, resolved_module, type_path) else {
        return (None, None, None, None);
    };
    let HirSymbolBody::Object { methods, .. } = &resolved.symbol.body else {
        return (None, None, None, None);
    };

    let mut init_routine_key = None;
    let mut init_with_context_routine_key = None;
    let mut resume_routine_key = None;
    let mut resume_with_context_routine_key = None;

    for (method_index, method) in methods.iter().enumerate() {
        let slot = match method.name.as_str() {
            "init" => {
                if method.params.len() == 1 {
                    &mut init_routine_key
                } else if method.params.len() == 2 {
                    &mut init_with_context_routine_key
                } else {
                    continue;
                }
            }
            "resume" => {
                if method.params.len() == 1 {
                    &mut resume_routine_key
                } else if method.params.len() == 2 {
                    &mut resume_with_context_routine_key
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        if method.is_async || !method.type_params.is_empty() {
            continue;
        }
        let Some(receiver) = method.params.first() else {
            continue;
        };
        if receiver.mode != Some(arcana_hir::HirParamMode::Edit) {
            continue;
        }
        if let Some(context) = method.params.get(1)
            && context.mode != Some(arcana_hir::HirParamMode::Read)
        {
            continue;
        }
        if slot.is_none() {
            *slot = Some(routine_key_for_object_method(
                resolved.module_id,
                resolved.symbol_index,
                method_index,
            ));
        }
    }

    (
        init_routine_key,
        init_with_context_routine_key,
        resume_routine_key,
        resume_with_context_routine_key,
    )
}

fn lower_owner_decl_resolved(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module: &HirModuleSummary,
    symbol: &HirSymbol,
) -> Result<Option<IrOwnerDecl>, String> {
    let HirSymbolBody::Owner { objects, exits } = &symbol.body else {
        return Ok(None);
    };
    let scope = ResolvedRenderScope::new(
        workspace,
        resolved_module,
        symbol
            .where_clause
            .as_ref()
            .map(|where_clause| where_clause.render()),
        &symbol.type_params,
    );
    Ok(Some(IrOwnerDecl {
        module_id: module.module_id.clone(),
        owner_path: canonical_symbol_path(&module.module_id, &symbol.name),
        owner_name: symbol.name.clone(),
        objects: objects
            .iter()
            .map(|object| {
                let canonical = lookup_symbol_path(workspace, resolved_module, &object.type_path)
                    .map(|resolved| {
                        canonical_symbol_path(resolved.module_id, &resolved.symbol.name)
                    })
                    .unwrap_or_else(|| object.type_path.clone());
                let (
                    init_routine_key,
                    init_with_context_routine_key,
                    resume_routine_key,
                    resume_with_context_routine_key,
                ) = resolve_object_lifecycle_routine_keys(
                    workspace,
                    resolved_module,
                    &object.type_path,
                );
                IrOwnerObject {
                    type_path: canonical,
                    local_name: object.local_name.clone(),
                    init_routine_key,
                    init_with_context_routine_key,
                    resume_routine_key,
                    resume_with_context_routine_key,
                }
            })
            .collect(),
        exits: exits
            .iter()
            .map(|owner_exit| IrOwnerExit {
                name: owner_exit.name.clone(),
                condition: lower_exec_expr_resolved(&owner_exit.condition, &scope),
                holds: owner_exit.holds.clone(),
            })
            .collect(),
    }))
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
        type_params: symbol.type_params.clone(),
        behavior_attrs: lower_behavior_attrs(symbol),
        params: lower_routine_params(symbol),
        return_type: symbol.return_type.as_ref().map(|ty| ty.render()),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        impl_target_type: impl_decl.map(|decl| decl.target_type.render()),
        impl_trait_path: impl_decl.and_then(|decl| {
            decl.trait_path
                .as_ref()
                .map(|path| canonical_impl_trait_path(path))
        }),
        availability: symbol
            .availability
            .iter()
            .map(lower_availability_attachment_exec)
            .collect(),
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
        symbol
            .where_clause
            .as_ref()
            .map(|where_clause| where_clause.render()),
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
        type_params: symbol.type_params.clone(),
        behavior_attrs: lower_behavior_attrs(symbol),
        params: lower_routine_params(symbol),
        return_type: symbol.return_type.as_ref().map(|ty| ty.render()),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        impl_target_type: impl_decl.map(|decl| decl.target_type.render()),
        impl_trait_path: impl_decl.and_then(|decl| {
            decl.trait_path
                .as_ref()
                .map(|path| canonical_impl_trait_path(path))
        }),
        availability: symbol
            .availability
            .iter()
            .map(|attachment| lower_availability_attachment_exec_resolved(attachment, &scope))
            .collect::<Result<Vec<_>, _>>()?,
        foreword_rows: symbol.forewords.iter().map(render_foreword_row).collect(),
        rollups: symbol
            .rollups
            .iter()
            .map(|rollup| lower_rollup_resolved(workspace, resolved_module, rollup))
            .collect(),
        statements: lower_exec_stmt_block_resolved(&symbol.statements, &mut scope)?,
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
                routines.extend(lower_object_method_routines(module));
                routines
            })
            .collect::<Vec<_>>();
    let owners = package
        .modules
        .iter()
        .flat_map(|module| {
            module
                .symbols
                .iter()
                .filter_map(|symbol| lower_owner_decl(module, symbol))
                .collect::<Vec<_>>()
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
        owners,
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
            routines.extend(lower_object_method_routines_resolved(
                workspace,
                package,
                resolved_module,
                module,
            )?);
            Ok(routines)
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    lowered.owners = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                return Ok(Vec::new());
            };
            module
                .symbols
                .iter()
                .map(|symbol| lower_owner_decl_resolved(workspace, resolved_module, module, symbol))
                .collect::<Result<Vec<_>, String>>()
                .map(|owners| owners.into_iter().flatten().collect::<Vec<_>>())
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
    let module_lang_item_rows = package
        .summary
        .modules
        .iter()
        .map(|module| {
            let rows = resolved_package
                .module(&module.module_id)
                .map(|resolved_module| {
                    resolved_module_lang_item_rows(workspace, resolved_module, module)
                })
                .unwrap_or_else(|| {
                    module
                        .lang_items
                        .iter()
                        .map(|item| {
                            render_lang_item_row(&module.module_id, &item.name, &item.target)
                        })
                        .collect()
                });
            (module.module_id.clone(), rows)
        })
        .collect::<BTreeMap<_, _>>();
    for module in &mut lowered.modules {
        if let Some(rows) = module_surface_rows.get(&module.module_id) {
            module.exported_surface_rows = rows.clone();
        }
        if let Some(rows) = module_lang_item_rows.get(&module.module_id) {
            module.lang_item_rows = rows.clone();
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
        assert!(ir.routines[0].params.is_empty());
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
    fn lower_workspace_package_with_resolution_canonicalizes_lang_item_targets() {
        let app_summary = build_package_summary(
            "app",
            vec![
                lower_module_text(
                    "app",
                    concat!(
                        "import app.types\n",
                        "lang window_handle = types.Window\n",
                        "fn main() -> Int:\n",
                        "    return 0\n",
                    ),
                )
                .expect("root module should lower"),
                lower_module_text(
                    "app.types",
                    "export opaque type Window as move, boundary_unsafe\n",
                )
                .expect("types module should lower"),
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
                    "app.types".to_string(),
                    Path::new("C:/repo/app/src/types.arc").to_path_buf(),
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
        let root_module = ir
            .modules
            .iter()
            .find(|module| module.module_id == "app")
            .expect("root module should exist");
        assert_eq!(
            root_module.lang_item_rows,
            vec!["module=app:lang:window_handle:app.types.Window".to_string()]
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
