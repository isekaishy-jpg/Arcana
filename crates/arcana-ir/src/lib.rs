use std::collections::{BTreeMap, BTreeSet};

use arcana_hir::{
    HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer, HirChainStep,
    HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg, HirHeaderAttachment, HirMatchArm,
    HirMatchPattern, HirModule, HirModuleDependency, HirModuleSummary, HirPackageSummary,
    HirPageRollup, HirPhraseArg, HirResolvedModule, HirResolvedTarget, HirResolvedWorkspace,
    HirStatement, HirStatementKind, HirSymbol, HirSymbolBody, HirSymbolKind, HirUnaryOp,
    HirWorkspacePackage, HirWorkspaceSummary,
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
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_param_rows: Vec<String>,
    pub behavior_attr_rows: Vec<String>,
    pub param_rows: Vec<String>,
    pub signature_row: String,
    pub intrinsic_impl: Option<String>,
    pub foreword_rows: Vec<String>,
    pub rollup_rows: Vec<String>,
    pub statement_rows: Vec<String>,
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

#[derive(Clone, Debug)]
struct ResolvedRenderScope<'a> {
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    current_where_clause: Option<&'a str>,
    value_scope: LowerValueScope,
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
        }
    }
}

#[derive(Clone, Debug)]
struct IrResolvedSymbolRef<'a> {
    module_id: String,
    symbol: &'a HirSymbol,
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

fn runtime_requirement_for_path(path: &[String]) -> Option<String> {
    let first = path.first()?;
    if first != "std" {
        return None;
    }
    if path.len() >= 3 && path[1] == "kernel" {
        return Some(format!("std.kernel.{}", path[2]));
    }
    if path.len() >= 2 {
        return Some(format!("std.{}", path[1]));
    }
    Some("std".to_string())
}

fn quote_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
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

fn render_rollup_row(rollup: &HirPageRollup) -> String {
    format!(
        "{}:{}:{}",
        rollup.kind.as_str(),
        rollup.subject,
        rollup.handler_path.join(".")
    )
}

fn render_phrase_arg(arg: &HirPhraseArg) -> String {
    match arg {
        HirPhraseArg::Positional(expr) => render_expr(expr),
        HirPhraseArg::Named { name, value } => format!("{name}={}", render_expr(value)),
    }
}

fn render_phrase_qualifier_kind(qualifier: &str) -> &'static str {
    match qualifier.trim() {
        "call" => "call",
        "?" => "try",
        ">" => "apply",
        ">>" => "await_apply",
        other if other.contains('.') => "named_path",
        _ => "bare_method",
    }
}

fn render_chain_connector(connector: HirChainConnector) -> &'static str {
    match connector {
        HirChainConnector::Forward => "=>",
        HirChainConnector::Reverse => "<=",
    }
}

fn render_chain_introducer(introducer: HirChainIntroducer) -> &'static str {
    match introducer {
        HirChainIntroducer::Forward => "forward",
        HirChainIntroducer::Reverse => "reverse",
    }
}

fn render_chain_step(step: &HirChainStep) -> String {
    let incoming = step.incoming.map(render_chain_connector).unwrap_or("start");
    let bind_args = step
        .bind_args
        .iter()
        .map(render_expr)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "step({incoming},stage={},bind=[{bind_args}],text=\"{}\")",
        render_expr(&step.stage),
        quote_text(&step.text)
    )
}

fn render_header_attachment(attachment: &HirHeaderAttachment) -> String {
    match attachment {
        HirHeaderAttachment::Named {
            name,
            value,
            forewords,
            ..
        } => format!(
            "named({name}={},forewords=[{}])",
            render_expr(value),
            forewords
                .iter()
                .map(render_foreword_row)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirHeaderAttachment::Chain {
            expr, forewords, ..
        } => format!(
            "chain({},forewords=[{}])",
            render_expr(expr),
            forewords
                .iter()
                .map(render_foreword_row)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
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

fn resolved_symbol_path(symbol_ref: IrResolvedSymbolRef<'_>) -> Vec<String> {
    let mut path = symbol_ref
        .module_id
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    path.push(symbol_ref.symbol.name.clone());
    path
}

fn lookup_symbol_in_module<'a>(
    workspace: &'a HirWorkspaceSummary,
    package_name: &str,
    module_id: &str,
    symbol_name: &str,
) -> Option<IrResolvedSymbolRef<'a>> {
    let module = workspace.package(package_name)?.module(module_id)?;
    let symbol = module
        .symbols
        .iter()
        .find(|symbol| symbol.name == symbol_name)?;
    Some(IrResolvedSymbolRef {
        module_id: module.module_id.clone(),
        symbol,
    })
}

fn lookup_symbol_from_package_path<'a>(
    workspace: &'a HirWorkspaceSummary,
    package_name: &str,
    path: &[String],
) -> Option<IrResolvedSymbolRef<'a>> {
    if path.len() < 2 {
        return None;
    }
    let package = workspace.package(package_name)?;
    for split in (1..path.len()).rev() {
        let module_id = path[..split].join(".");
        let symbol_name = &path[split];
        if package.module(&module_id).is_some() {
            return lookup_symbol_in_module(workspace, package_name, &module_id, symbol_name);
        }
    }
    None
}

fn lookup_symbol_path<'a>(
    workspace: &'a HirWorkspaceSummary,
    resolved_module: &'a HirResolvedModule,
    path: &[String],
) -> Option<IrResolvedSymbolRef<'a>> {
    let first = path.first()?;
    if let Some(binding) = resolved_module.bindings.get(first) {
        return match &binding.target {
            HirResolvedTarget::Module {
                package_name,
                module_id,
            } => {
                if path.len() == 1 {
                    None
                } else {
                    let mut qualified = module_id
                        .split('.')
                        .map(ToString::to_string)
                        .collect::<Vec<_>>();
                    qualified.extend(path[1..].iter().cloned());
                    lookup_symbol_from_package_path(workspace, package_name, &qualified)
                }
            }
            HirResolvedTarget::Symbol {
                package_name,
                module_id,
                symbol_name,
            } => (path.len() == 1).then(|| {
                lookup_symbol_in_module(workspace, package_name, module_id, symbol_name)
            })?,
        };
    }

    if workspace.package(first).is_some() {
        return lookup_symbol_from_package_path(workspace, first, path);
    }

    let package_name = resolved_module.module_id.split('.').next()?;
    let package = workspace.package(package_name)?;
    for split in (1..path.len()).rev() {
        let relative = &path[..split];
        let symbol_name = &path[split];
        if let Some(module) = package.resolve_relative_module(relative) {
            return lookup_symbol_in_module(
                workspace,
                package_name,
                &module.module_id,
                symbol_name,
            );
        }
    }
    None
}

fn lookup_method_path_for_type(
    workspace: &HirWorkspaceSummary,
    type_text: &str,
    method_name: &str,
) -> Option<Vec<String>> {
    let wanted = erase_type_generics(type_text);
    let matches_target = |target: &str| {
        let target = erase_type_generics(target);
        target == wanted
            || target.ends_with(&format!(".{wanted}"))
            || wanted.ends_with(&format!(".{target}"))
    };
    for package in workspace.packages.values() {
        for module in &package.summary.modules {
            for impl_decl in &module.impls {
                if !matches_target(&impl_decl.target_type) {
                    continue;
                }
                if let Some(method) = impl_decl
                    .methods
                    .iter()
                    .find(|method| method.name == method_name)
                {
                    let mut path = module
                        .module_id
                        .split('.')
                        .map(ToString::to_string)
                        .collect::<Vec<_>>();
                    path.push(method.name.clone());
                    return Some(path);
                }
            }
            for symbol in &module.symbols {
                if symbol.name != method_name {
                    continue;
                }
                if !matches!(symbol.kind, HirSymbolKind::Fn) {
                    continue;
                }
                let Some(self_param) = symbol.params.first() else {
                    continue;
                };
                if self_param.name != "self" || !matches_target(&self_param.ty) {
                    continue;
                }
                let mut path = module
                    .module_id
                    .split('.')
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                path.push(symbol.name.clone());
                return Some(path);
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
struct ResolvedMethod<'a> {
    module_id: String,
    method: &'a HirSymbol,
    substitutions: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct ResolvedPhraseTarget {
    path: Vec<String>,
    signature_row: Option<String>,
}

fn substitute_type_params(text: &str, substitutions: &BTreeMap<String, String>) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\'' {
            out.push(ch);
            index += 1;
            while index < chars.len() {
                let current = chars[index];
                out.push(current);
                index += 1;
                if !(current == '_' || current.is_ascii_alphanumeric()) {
                    break;
                }
            }
            continue;
        }
        if ch == '_' || ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index] == '_' || chars[index].is_ascii_alphanumeric())
            {
                index += 1;
            }
            let ident = chars[start..index].iter().collect::<String>();
            if let Some(replacement) = substitutions.get(&ident) {
                out.push_str(replacement);
            } else {
                out.push_str(&ident);
            }
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn build_impl_type_substitutions(
    impl_target: &str,
    concrete_type: &str,
) -> BTreeMap<String, String> {
    let Some((_, target_args)) = parse_surface_type_application(impl_target) else {
        return BTreeMap::new();
    };
    let Some((_, concrete_args)) = parse_surface_type_application(concrete_type) else {
        return BTreeMap::new();
    };
    target_args
        .into_iter()
        .zip(concrete_args)
        .filter_map(|(name, value)| is_identifier_text(&name).then_some((name, value)))
        .collect()
}

fn lookup_method_resolution_for_type<'a>(
    workspace: &'a HirWorkspaceSummary,
    type_text: &str,
    method_name: &str,
) -> Option<ResolvedMethod<'a>> {
    let wanted = erase_type_generics(type_text);
    let matches_target = |target: &str| {
        let target = erase_type_generics(target);
        target == wanted
            || target.ends_with(&format!(".{wanted}"))
            || wanted.ends_with(&format!(".{target}"))
    };
    for package in workspace.packages.values() {
        for module in &package.summary.modules {
            for impl_decl in &module.impls {
                if !matches_target(&impl_decl.target_type) {
                    continue;
                }
                if let Some(method) = impl_decl
                    .methods
                    .iter()
                    .find(|method| method.name == method_name)
                {
                    return Some(ResolvedMethod {
                        module_id: module.module_id.clone(),
                        method,
                        substitutions: build_impl_type_substitutions(
                            &impl_decl.target_type,
                            type_text,
                        ),
                    });
                }
            }
            for symbol in &module.symbols {
                if symbol.name != method_name {
                    continue;
                }
                if !matches!(symbol.kind, HirSymbolKind::Fn) {
                    continue;
                }
                let Some(self_param) = symbol.params.first() else {
                    continue;
                };
                if self_param.name != "self" || !matches_target(&self_param.ty) {
                    continue;
                }
                return Some(ResolvedMethod {
                    module_id: module.module_id.clone(),
                    method: symbol,
                    substitutions: build_impl_type_substitutions(&self_param.ty, type_text),
                });
            }
        }
    }
    None
}

fn lookup_trait_method_path_from_where_clause(
    scope: &ResolvedRenderScope<'_>,
    type_text: &str,
    method_name: &str,
) -> Option<Vec<String>> {
    let wanted = erase_type_generics(strip_reference_prefix(type_text));
    if !is_identifier_text(&wanted) {
        return None;
    }
    let where_clause = scope.current_where_clause?;
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
        let trait_path = split_simple_path(&trait_base)?;
        let symbol_ref = lookup_symbol_path(scope.workspace, scope.resolved_module, &trait_path)?;
        let HirSymbolBody::Trait { methods, .. } = &symbol_ref.symbol.body else {
            continue;
        };
        if methods.iter().any(|method| method.name == method_name) {
            let mut path = symbol_ref
                .module_id
                .split('.')
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            path.push(method_name.to_string());
            return Some(path);
        }
    }
    None
}

fn lookup_trait_method_resolution_from_where_clause<'a>(
    scope: &'a ResolvedRenderScope<'a>,
    type_text: &str,
    method_name: &str,
) -> Option<ResolvedMethod<'a>> {
    let wanted = erase_type_generics(strip_reference_prefix(type_text));
    if !is_identifier_text(&wanted) {
        return None;
    }
    let where_clause = scope.current_where_clause?;
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
        let trait_path = split_simple_path(&trait_base)?;
        let symbol_ref = lookup_symbol_path(scope.workspace, scope.resolved_module, &trait_path)?;
        let HirSymbolBody::Trait { methods, .. } = &symbol_ref.symbol.body else {
            continue;
        };
        if let Some(method) = methods.iter().find(|method| method.name == method_name) {
            return Some(ResolvedMethod {
                module_id: symbol_ref.module_id.clone(),
                method,
                substitutions: BTreeMap::new(),
            });
        }
    }
    None
}

fn render_unary_op(op: HirUnaryOp) -> &'static str {
    match op {
        HirUnaryOp::Neg => "-",
        HirUnaryOp::Not => "not",
        HirUnaryOp::BitNot => "~",
        HirUnaryOp::BorrowRead => "&",
        HirUnaryOp::BorrowMut => "&mut",
        HirUnaryOp::Deref => "*",
        HirUnaryOp::Weave => "weave",
        HirUnaryOp::Split => "split",
    }
}

fn render_binary_op(op: HirBinaryOp) -> &'static str {
    match op {
        HirBinaryOp::Or => "or",
        HirBinaryOp::And => "and",
        HirBinaryOp::EqEq => "==",
        HirBinaryOp::NotEq => "!=",
        HirBinaryOp::Lt => "<",
        HirBinaryOp::LtEq => "<=",
        HirBinaryOp::Gt => ">",
        HirBinaryOp::GtEq => ">=",
        HirBinaryOp::BitOr => "|",
        HirBinaryOp::BitXor => "^",
        HirBinaryOp::BitAnd => "&",
        HirBinaryOp::Shl => "<<",
        HirBinaryOp::Shr => "shr",
        HirBinaryOp::Add => "+",
        HirBinaryOp::Sub => "-",
        HirBinaryOp::Mul => "*",
        HirBinaryOp::Div => "/",
        HirBinaryOp::Mod => "%",
    }
}

fn render_match_pattern(pattern: &HirMatchPattern) -> String {
    match pattern {
        HirMatchPattern::Wildcard => "_".to_string(),
        HirMatchPattern::Literal { text } => format!("lit(\"{}\")", quote_text(text)),
        HirMatchPattern::Name { text } => format!("name({text})"),
        HirMatchPattern::Variant { path, args } => format!(
            "variant({path},[{}])",
            args.iter()
                .map(render_match_pattern)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_match_arm(arm: &HirMatchArm) -> String {
    format!(
        "arm(patterns=[{}],value={})",
        arm.patterns
            .iter()
            .map(render_match_pattern)
            .collect::<Vec<_>>()
            .join("|"),
        render_expr(&arm.value)
    )
}

fn render_expr(expr: &HirExpr) -> String {
    match expr {
        HirExpr::Path { segments } => format!("path({})", segments.join(".")),
        HirExpr::BoolLiteral { value } => format!("bool({value})"),
        HirExpr::IntLiteral { text } => format!("int({text})"),
        HirExpr::StrLiteral { text } => format!("str(\"{}\")", quote_text(text)),
        HirExpr::Pair { left, right } => {
            format!("pair({}, {})", render_expr(left), render_expr(right))
        }
        HirExpr::CollectionLiteral { items } => format!(
            "collection([{}])",
            items.iter().map(render_expr).collect::<Vec<_>>().join(",")
        ),
        HirExpr::Match { subject, arms } => format!(
            "match(subject={},arms=[{}])",
            render_expr(subject),
            arms.iter()
                .map(render_match_arm)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => format!(
            "chain(style={style},introducer={},steps=[{}])",
            render_chain_introducer(*introducer),
            steps
                .iter()
                .map(render_chain_step)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => format!(
            "memory(family={family},arena={},init=[{}],ctor={},attached=[{}])",
            render_expr(arena),
            init_args
                .iter()
                .map(render_phrase_arg)
                .collect::<Vec<_>>()
                .join(","),
            render_expr(constructor),
            attached
                .iter()
                .map(render_header_attachment)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::GenericApply { expr, type_args } => format!(
            "generic(expr={},types=[{}])",
            render_expr(expr),
            type_args.join(",")
        ),
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
        } => format!(
            "phrase(subject={},args=[{}],kind={},qualifier={qualifier},attached=[{}])",
            render_expr(subject),
            args.iter()
                .map(render_phrase_arg)
                .collect::<Vec<_>>()
                .join(","),
            render_phrase_qualifier_kind(qualifier),
            attached
                .iter()
                .map(render_header_attachment)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Await { expr } => format!("await({})", render_expr(expr)),
        HirExpr::Unary { op, expr } => {
            format!("unary({}, {})", render_unary_op(*op), render_expr(expr))
        }
        HirExpr::Binary { left, op, right } => format!(
            "binary({}, {}, {})",
            render_expr(left),
            render_binary_op(*op),
            render_expr(right)
        ),
        HirExpr::MemberAccess { expr, member } => {
            format!("member({}, {member})", render_expr(expr))
        }
        HirExpr::Index { expr, index } => {
            format!("index({}, {})", render_expr(expr), render_expr(index))
        }
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => format!(
            "slice(expr={},start={},end={},inclusive={inclusive_end})",
            render_expr(expr),
            start
                .as_ref()
                .map(|expr| render_expr(expr))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr(expr))
                .unwrap_or_else(|| "none".to_string())
        ),
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => format!(
            "range(start={},end={},inclusive={inclusive_end})",
            start
                .as_ref()
                .map(|expr| render_expr(expr))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr(expr))
                .unwrap_or_else(|| "none".to_string())
        ),
    }
}

fn render_assign_target(target: &HirAssignTarget) -> String {
    match target {
        HirAssignTarget::Name { text } => format!("name({text})"),
        HirAssignTarget::MemberAccess { target, member } => {
            format!("member({}, {member})", render_assign_target(target))
        }
        HirAssignTarget::Index { target, index } => {
            format!(
                "index({}, {})",
                render_assign_target(target),
                render_expr(index)
            )
        }
    }
}

fn render_assign_op(op: HirAssignOp) -> &'static str {
    op.as_str()
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
        .map(|attr| format!("name={}:value={}", attr.name, attr.value))
        .collect()
}

fn render_statement(statement: &HirStatement) -> String {
    let forewords = statement
        .forewords
        .iter()
        .map(render_foreword_row)
        .collect::<Vec<_>>()
        .join(",");
    let rollups = statement
        .rollups
        .iter()
        .map(render_rollup_row)
        .collect::<Vec<_>>()
        .join(",");
    let core = match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => format!(
            "let(mutable={mutable},name={name},value={})",
            render_expr(value)
        ),
        HirStatementKind::Return { value } => format!(
            "return({})",
            value
                .as_ref()
                .map(render_expr)
                .unwrap_or_else(|| "none".to_string())
        ),
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if(cond={},then=[{}],else=[{}])",
            render_expr(condition),
            then_branch
                .iter()
                .map(render_statement)
                .collect::<Vec<_>>()
                .join(","),
            else_branch
                .as_ref()
                .map(|branch| branch
                    .iter()
                    .map(render_statement)
                    .collect::<Vec<_>>()
                    .join(","))
                .unwrap_or_default()
        ),
        HirStatementKind::While { condition, body } => format!(
            "while(cond={},body=[{}])",
            render_expr(condition),
            body.iter()
                .map(render_statement)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => format!(
            "for(binding={binding},iterable={},body=[{}])",
            render_expr(iterable),
            body.iter()
                .map(render_statement)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::Defer { expr } => format!("defer({})", render_expr(expr)),
        HirStatementKind::Break => "break".to_string(),
        HirStatementKind::Continue => "continue".to_string(),
        HirStatementKind::Assign { target, op, value } => format!(
            "assign(target={},op={},value={})",
            render_assign_target(target),
            render_assign_op(*op),
            render_expr(value)
        ),
        HirStatementKind::Expr { expr } => format!("expr({})", render_expr(expr)),
    };
    format!("stmt(core={core},forewords=[{forewords}],rollups=[{rollups}])")
}

fn symbol_return_type_text(symbol_ref: IrResolvedSymbolRef<'_>) -> Option<String> {
    symbol_ref.symbol.return_type.clone().or_else(|| {
        matches!(
            symbol_ref.symbol.kind,
            HirSymbolKind::Record | HirSymbolKind::Enum | HirSymbolKind::OpaqueType
        )
        .then(|| resolved_symbol_path(symbol_ref).join("."))
    })
}

fn infer_member_access_type(
    scope: &ResolvedRenderScope<'_>,
    expr: &HirExpr,
    member: &str,
) -> Option<String> {
    let base_ty = infer_expr_type_text(scope, expr)?;
    let (base, _) = parse_surface_type_application(strip_reference_prefix(&base_ty))?;
    let path = split_simple_path(&base)?;
    let symbol_ref = lookup_symbol_path(scope.workspace, scope.resolved_module, &path)?;
    match &symbol_ref.symbol.body {
        HirSymbolBody::Record { fields } => fields
            .iter()
            .find(|field| field.name == member)
            .map(|field| field.ty.clone()),
        _ => None,
    }
}

fn infer_index_type_text(scope: &ResolvedRenderScope<'_>, expr: &HirExpr) -> Option<String> {
    let base_ty = infer_expr_type_text(scope, expr)?;
    let (base, args) = parse_surface_type_application(strip_reference_prefix(&base_ty))?;
    match base.as_str() {
        "List" | "Array" => args.first().cloned(),
        "Map" => args.get(1).cloned(),
        _ => None,
    }
}

fn infer_slice_type_text(scope: &ResolvedRenderScope<'_>, expr: &HirExpr) -> Option<String> {
    let base_ty = infer_expr_type_text(scope, expr)?;
    let (base, args) = parse_surface_type_application(strip_reference_prefix(&base_ty))?;
    match base.as_str() {
        "List" => Some(format!(
            "List[{}]",
            args.first().cloned().unwrap_or_else(|| "_".to_string())
        )),
        "Array" => Some(format!(
            "Array[{}]",
            args.first().cloned().unwrap_or_else(|| "_".to_string())
        )),
        _ => Some(base_ty),
    }
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
) -> Option<Vec<String>> {
    if qualifier == "call" {
        let path = flatten_callable_expr_path(subject)?;
        return lookup_symbol_path(scope.workspace, scope.resolved_module, &path)
            .map(resolved_symbol_path);
    }

    if let Some(path) = split_simple_path(qualifier).filter(|path| path.len() > 1) {
        if let Some(resolved) = lookup_symbol_path(scope.workspace, scope.resolved_module, &path) {
            return Some(resolved_symbol_path(resolved));
        }
    }

    if is_identifier_text(qualifier) {
        let subject_ty = infer_expr_type_text(scope, subject)?;
        return lookup_method_path_for_type(scope.workspace, &subject_ty, qualifier)
            .or_else(|| lookup_trait_method_path_from_where_clause(scope, &subject_ty, qualifier));
    }

    None
}

fn resolve_bare_method_target(
    scope: &ResolvedRenderScope<'_>,
    subject: &HirExpr,
    qualifier: &str,
) -> Option<ResolvedPhraseTarget> {
    let subject_ty = infer_expr_type_text(scope, subject)?;
    if let Some(resolved) =
        lookup_method_resolution_for_type(scope.workspace, &subject_ty, qualifier)
    {
        let mut path = resolved
            .module_id
            .split('.')
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        path.push(resolved.method.name.clone());
        return Some(ResolvedPhraseTarget {
            path,
            signature_row: Some(resolved.method.surface_text.clone()),
        });
    }
    let resolved = lookup_trait_method_resolution_from_where_clause(scope, &subject_ty, qualifier)?;
    let mut path = resolved
        .module_id
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    path.push(resolved.method.name.clone());
    Some(ResolvedPhraseTarget {
        path,
        signature_row: None,
    })
}

fn infer_call_target_return_type(
    scope: &ResolvedRenderScope<'_>,
    subject: &HirExpr,
) -> Option<String> {
    let path = flatten_callable_expr_path(subject)?;
    if let Some(symbol_ref) = lookup_symbol_path(scope.workspace, scope.resolved_module, &path) {
        return symbol_return_type_text(symbol_ref);
    }
    if path.len() >= 2 {
        let enum_path = path[..path.len() - 1].to_vec();
        if let Some(enum_ref) =
            lookup_symbol_path(scope.workspace, scope.resolved_module, &enum_path)
        {
            if matches!(enum_ref.symbol.kind, HirSymbolKind::Enum) {
                return symbol_return_type_text(enum_ref);
            }
        }
    }
    None
}

fn infer_expr_type_text(scope: &ResolvedRenderScope<'_>, expr: &HirExpr) -> Option<String> {
    match expr {
        HirExpr::BoolLiteral { .. } => Some("Bool".to_string()),
        HirExpr::IntLiteral { .. } => Some("Int".to_string()),
        HirExpr::StrLiteral { .. } => Some("Str".to_string()),
        HirExpr::CollectionLiteral { .. } => Some("List[_]".to_string()),
        HirExpr::Range { .. } => Some("RangeInt".to_string()),
        HirExpr::Path { segments }
            if segments.len() == 1 && scope.value_scope.contains(&segments[0]) =>
        {
            scope
                .value_scope
                .type_text_of(&segments[0])
                .map(ToOwned::to_owned)
        }
        HirExpr::Path { segments } => {
            let symbol_ref = lookup_symbol_path(scope.workspace, scope.resolved_module, segments)?;
            symbol_return_type_text(symbol_ref)
        }
        HirExpr::Unary { op, expr }
            if matches!(op, HirUnaryOp::BorrowRead | HirUnaryOp::BorrowMut) =>
        {
            infer_expr_type_text(scope, expr).map(|text| format!("& {text}"))
        }
        HirExpr::Unary {
            op: HirUnaryOp::Weave,
            expr,
        } => infer_expr_type_text(scope, expr).map(|text| format!("std.concurrent.Task[{text}]")),
        HirExpr::Unary {
            op: HirUnaryOp::Split,
            expr,
        } => infer_expr_type_text(scope, expr).map(|text| format!("std.concurrent.Thread[{text}]")),
        HirExpr::Unary {
            op: HirUnaryOp::Deref,
            expr,
        } => infer_expr_type_text(scope, expr).map(|text| {
            let stripped = strip_reference_prefix(&text);
            if stripped == text.trim() {
                text
            } else {
                stripped.to_string()
            }
        }),
        HirExpr::GenericApply { expr, .. } => infer_expr_type_text(scope, expr),
        HirExpr::QualifiedPhrase {
            subject, qualifier, ..
        } if qualifier == "call" => infer_call_target_return_type(scope, subject),
        HirExpr::QualifiedPhrase { qualifier, .. } if qualifier.contains('.') => {
            let path = split_simple_path(qualifier)?;
            lookup_symbol_path(scope.workspace, scope.resolved_module, &path)
                .and_then(symbol_return_type_text)
        }
        HirExpr::QualifiedPhrase {
            subject, qualifier, ..
        } if is_identifier_text(qualifier) => {
            let subject_ty = infer_expr_type_text(scope, subject)?;
            lookup_method_resolution_for_type(scope.workspace, &subject_ty, qualifier)
                .or_else(|| {
                    lookup_trait_method_resolution_from_where_clause(scope, &subject_ty, qualifier)
                })
                .and_then(|resolved| {
                    resolved
                        .method
                        .return_type
                        .as_ref()
                        .map(|text| substitute_type_params(text, &resolved.substitutions))
                })
        }
        HirExpr::MemberAccess { expr, member } => infer_member_access_type(scope, expr, member),
        HirExpr::Index { expr, .. } => infer_index_type_text(scope, expr),
        HirExpr::Slice { expr, .. } => infer_slice_type_text(scope, expr),
        HirExpr::Match { arms, .. } => {
            let inferred = arms
                .iter()
                .filter_map(|arm| infer_expr_type_text(scope, &arm.value))
                .collect::<Vec<_>>();
            let first = inferred.first()?.clone();
            inferred
                .iter()
                .all(|candidate| candidate == &first)
                .then_some(first)
        }
        HirExpr::Await { expr } => {
            let awaited = infer_expr_type_text(scope, expr)?;
            let (base, args) = parse_surface_type_application(strip_reference_prefix(&awaited))?;
            match base.as_str() {
                "std.concurrent.Task" | "std.concurrent.Thread" | "Task" | "Thread" => {
                    args.first().cloned()
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn render_phrase_arg_resolved(arg: &HirPhraseArg, scope: &ResolvedRenderScope<'_>) -> String {
    match arg {
        HirPhraseArg::Positional(expr) => render_expr_resolved(expr, scope),
        HirPhraseArg::Named { name, value } => {
            format!("{name}={}", render_expr_resolved(value, scope))
        }
    }
}

fn render_chain_step_resolved(step: &HirChainStep, scope: &ResolvedRenderScope<'_>) -> String {
    let incoming = step.incoming.map(render_chain_connector).unwrap_or("start");
    let bind_args = step
        .bind_args
        .iter()
        .map(|expr| render_expr_resolved(expr, scope))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "step({incoming},stage={},bind=[{bind_args}],text=\"{}\")",
        render_expr_resolved(&step.stage, scope),
        quote_text(&step.text)
    )
}

fn render_header_attachment_resolved(
    attachment: &HirHeaderAttachment,
    scope: &ResolvedRenderScope<'_>,
) -> String {
    match attachment {
        HirHeaderAttachment::Named {
            name,
            value,
            forewords,
            ..
        } => format!(
            "named({name}={},forewords=[{}])",
            render_expr_resolved(value, scope),
            forewords
                .iter()
                .map(render_foreword_row)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirHeaderAttachment::Chain {
            expr, forewords, ..
        } => format!(
            "chain({},forewords=[{}])",
            render_expr_resolved(expr, scope),
            forewords
                .iter()
                .map(render_foreword_row)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_match_arm_resolved(arm: &HirMatchArm, scope: &ResolvedRenderScope<'_>) -> String {
    format!(
        "arm(patterns=[{}],value={})",
        arm.patterns
            .iter()
            .map(render_match_pattern)
            .collect::<Vec<_>>()
            .join("|"),
        render_expr_resolved(&arm.value, scope)
    )
}

fn render_assign_target_resolved(
    target: &HirAssignTarget,
    scope: &ResolvedRenderScope<'_>,
) -> String {
    match target {
        HirAssignTarget::Name { text } => format!("name({text})"),
        HirAssignTarget::MemberAccess { target, member } => {
            format!(
                "member({}, {member})",
                render_assign_target_resolved(target, scope)
            )
        }
        HirAssignTarget::Index { target, index } => format!(
            "index({}, {})",
            render_assign_target_resolved(target, scope),
            render_expr_resolved(index, scope)
        ),
    }
}

fn render_expr_resolved(expr: &HirExpr, scope: &ResolvedRenderScope<'_>) -> String {
    match expr {
        HirExpr::Path { segments } => format!("path({})", segments.join(".")),
        HirExpr::BoolLiteral { value } => format!("bool({value})"),
        HirExpr::IntLiteral { text } => format!("int({text})"),
        HirExpr::StrLiteral { text } => format!("str(\"{}\")", quote_text(text)),
        HirExpr::Pair { left, right } => format!(
            "pair({}, {})",
            render_expr_resolved(left, scope),
            render_expr_resolved(right, scope)
        ),
        HirExpr::CollectionLiteral { items } => format!(
            "collection([{}])",
            items
                .iter()
                .map(|item| render_expr_resolved(item, scope))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Match { subject, arms } => format!(
            "match(subject={},arms=[{}])",
            render_expr_resolved(subject, scope),
            arms.iter()
                .map(|arm| render_match_arm_resolved(arm, scope))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => format!(
            "chain(style={style},introducer={},steps=[{}])",
            render_chain_introducer(*introducer),
            steps
                .iter()
                .map(|step| render_chain_step_resolved(step, scope))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => format!(
            "memory(family={family},arena={},init=[{}],ctor={},attached=[{}])",
            render_expr_resolved(arena, scope),
            init_args
                .iter()
                .map(|arg| render_phrase_arg_resolved(arg, scope))
                .collect::<Vec<_>>()
                .join(","),
            render_expr_resolved(constructor, scope),
            attached
                .iter()
                .map(|attachment| render_header_attachment_resolved(attachment, scope))
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::GenericApply { expr, type_args } => format!(
            "generic(expr={},types=[{}])",
            render_expr_resolved(expr, scope),
            type_args.join(",")
        ),
        HirExpr::QualifiedPhrase {
            subject,
            args,
            qualifier,
            attached,
        } => {
            let resolved = match render_phrase_qualifier_kind(qualifier) {
                "bare_method" => resolve_bare_method_target(scope, subject, qualifier)
                    .map(|target| {
                        let mut rendered = format!(",resolved={}", target.path.join("."));
                        if let Some(signature_row) = target.signature_row {
                            rendered.push_str(&format!(
                                ",resolved_signature=str(\"{}\")",
                                quote_text(&signature_row)
                            ));
                        }
                        rendered
                    })
                    .unwrap_or_default(),
                "named_path" => resolve_qualified_phrase_target_path(scope, subject, qualifier)
                    .map(|path| format!(",resolved={}", path.join(".")))
                    .unwrap_or_default(),
                _ => String::new(),
            };
            format!(
                "phrase(subject={},args=[{}],kind={},qualifier={qualifier}{resolved},attached=[{}])",
                render_expr_resolved(subject, scope),
                args.iter()
                    .map(|arg| render_phrase_arg_resolved(arg, scope))
                    .collect::<Vec<_>>()
                    .join(","),
                render_phrase_qualifier_kind(qualifier),
                attached
                    .iter()
                    .map(|attachment| render_header_attachment_resolved(attachment, scope))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        HirExpr::Await { expr } => format!("await({})", render_expr_resolved(expr, scope)),
        HirExpr::Unary { op, expr } => format!(
            "unary({}, {})",
            render_unary_op(*op),
            render_expr_resolved(expr, scope)
        ),
        HirExpr::Binary { left, op, right } => format!(
            "binary({}, {}, {})",
            render_expr_resolved(left, scope),
            render_binary_op(*op),
            render_expr_resolved(right, scope)
        ),
        HirExpr::MemberAccess { expr, member } => {
            format!("member({}, {member})", render_expr_resolved(expr, scope))
        }
        HirExpr::Index { expr, index } => format!(
            "index({}, {})",
            render_expr_resolved(expr, scope),
            render_expr_resolved(index, scope)
        ),
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => format!(
            "slice(expr={},start={},end={},inclusive={inclusive_end})",
            render_expr_resolved(expr, scope),
            start
                .as_ref()
                .map(|expr| render_expr_resolved(expr, scope))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr_resolved(expr, scope))
                .unwrap_or_else(|| "none".to_string())
        ),
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => format!(
            "range(start={},end={},inclusive={inclusive_end})",
            start
                .as_ref()
                .map(|expr| render_expr_resolved(expr, scope))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr_resolved(expr, scope))
                .unwrap_or_else(|| "none".to_string())
        ),
    }
}

fn render_statement_block_resolved(
    statements: &[HirStatement],
    scope: &mut ResolvedRenderScope<'_>,
) -> Vec<String> {
    statements
        .iter()
        .map(|statement| render_statement_resolved(statement, scope))
        .collect()
}

fn render_statement_resolved(
    statement: &HirStatement,
    scope: &mut ResolvedRenderScope<'_>,
) -> String {
    let forewords = statement
        .forewords
        .iter()
        .map(render_foreword_row)
        .collect::<Vec<_>>()
        .join(",");
    let rollups = statement
        .rollups
        .iter()
        .map(render_rollup_row)
        .collect::<Vec<_>>()
        .join(",");
    let core = match &statement.kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => {
            let rendered = format!(
                "let(mutable={mutable},name={name},value={})",
                render_expr_resolved(value, scope)
            );
            if let Some(type_text) = infer_expr_type_text(scope, value) {
                scope.value_scope.insert(name.clone(), type_text);
            }
            rendered
        }
        HirStatementKind::Return { value } => format!(
            "return({})",
            value
                .as_ref()
                .map(|value| render_expr_resolved(value, scope))
                .unwrap_or_else(|| "none".to_string())
        ),
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let mut then_scope = scope.clone();
            let then_rows = render_statement_block_resolved(then_branch, &mut then_scope);
            let else_rows = else_branch.as_ref().map(|branch| {
                let mut else_scope = scope.clone();
                render_statement_block_resolved(branch, &mut else_scope).join(",")
            });
            format!(
                "if(cond={},then=[{}],else=[{}])",
                render_expr_resolved(condition, scope),
                then_rows.join(","),
                else_rows.unwrap_or_default()
            )
        }
        HirStatementKind::While { condition, body } => {
            let mut body_scope = scope.clone();
            let body_rows = render_statement_block_resolved(body, &mut body_scope);
            format!(
                "while(cond={},body=[{}])",
                render_expr_resolved(condition, scope),
                body_rows.join(",")
            )
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
            let body_rows = render_statement_block_resolved(body, &mut body_scope);
            format!(
                "for(binding={binding},iterable={},body=[{}])",
                render_expr_resolved(iterable, scope),
                body_rows.join(",")
            )
        }
        HirStatementKind::Defer { expr } => format!("defer({})", render_expr_resolved(expr, scope)),
        HirStatementKind::Break => "break".to_string(),
        HirStatementKind::Continue => "continue".to_string(),
        HirStatementKind::Assign { target, op, value } => {
            let rendered = format!(
                "assign(target={},op={},value={})",
                render_assign_target_resolved(target, scope),
                render_assign_op(*op),
                render_expr_resolved(value, scope)
            );
            if matches!(op, HirAssignOp::Assign) {
                if let HirAssignTarget::Name { text } = target {
                    if let Some(type_text) = infer_expr_type_text(scope, value) {
                        scope.value_scope.insert(text.clone(), type_text);
                    }
                }
            }
            rendered
        }
        HirStatementKind::Expr { expr } => format!("expr({})", render_expr_resolved(expr, scope)),
    };
    format!("stmt(core={core},forewords=[{forewords}],rollups=[{rollups}])")
}

fn is_routine_symbol(symbol: &HirSymbol) -> bool {
    matches!(
        symbol.kind,
        HirSymbolKind::Fn | HirSymbolKind::System | HirSymbolKind::Behavior | HirSymbolKind::Const
    )
}

fn lower_routine(module_id: &str, symbol: &HirSymbol) -> IrRoutine {
    IrRoutine {
        module_id: module_id.to_string(),
        symbol_name: symbol.name.clone(),
        symbol_kind: symbol.kind.as_str().to_string(),
        exported: symbol.exported,
        is_async: symbol.is_async,
        type_param_rows: render_type_param_rows(symbol),
        behavior_attr_rows: render_behavior_attr_rows(symbol),
        param_rows: render_param_row(symbol),
        signature_row: symbol.surface_text.clone(),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        foreword_rows: symbol.forewords.iter().map(render_foreword_row).collect(),
        rollup_rows: symbol.rollups.iter().map(render_rollup_row).collect(),
        statement_rows: symbol.statements.iter().map(render_statement).collect(),
    }
}

fn lower_routine_resolved(
    workspace: &HirWorkspaceSummary,
    resolved_module: &HirResolvedModule,
    module_id: &str,
    symbol: &HirSymbol,
) -> IrRoutine {
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
    IrRoutine {
        module_id: module_id.to_string(),
        symbol_name: symbol.name.clone(),
        symbol_kind: symbol.kind.as_str().to_string(),
        exported: symbol.exported,
        is_async: symbol.is_async,
        type_param_rows: render_type_param_rows(symbol),
        behavior_attr_rows: render_behavior_attr_rows(symbol),
        param_rows: render_param_row(symbol),
        signature_row: symbol.surface_text.clone(),
        intrinsic_impl: symbol.intrinsic_impl.clone(),
        foreword_rows: symbol.forewords.iter().map(render_foreword_row).collect(),
        rollup_rows: symbol.rollups.iter().map(render_rollup_row).collect(),
        statement_rows: render_statement_block_resolved(&symbol.statements, &mut scope),
    }
}

pub fn lower_package(package: &HirPackageSummary) -> IrPackage {
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
                exported_surface_rows: module.exported_surface_rows(),
            }
        })
        .collect::<Vec<_>>();
    let dependency_rows = package
        .dependency_edges
        .iter()
        .map(render_dependency_row)
        .collect::<Vec<_>>();
    let runtime_requirements = package
        .dependency_edges
        .iter()
        .filter_map(|edge| runtime_requirement_for_path(&edge.target_path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let entrypoints = package
        .modules
        .iter()
        .flat_map(|module| {
            module.symbols.iter().filter_map(|symbol| {
                let is_entry = symbol.kind == HirSymbolKind::System
                    || symbol.kind == HirSymbolKind::Behavior
                    || (symbol.kind == HirSymbolKind::Fn
                        && module.module_id == package.package_name
                        && symbol.name == "main");
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
    let routines = package
        .modules
        .iter()
        .flat_map(|module| {
            let mut routines = module
                .symbols
                .iter()
                .filter(|symbol| is_routine_symbol(symbol))
                .map(|symbol| lower_routine(&module.module_id, symbol))
                .collect::<Vec<_>>();
            routines.extend(
                module
                    .impls
                    .iter()
                    .flat_map(|impl_decl| impl_decl.methods.iter())
                    .filter(|symbol| is_routine_symbol(symbol))
                    .map(|symbol| lower_routine(&module.module_id, symbol)),
            );
            routines
        })
        .collect::<Vec<_>>();

    IrPackage {
        package_name: package.package_name.clone(),
        root_module_id: package.package_name.clone(),
        direct_deps: Vec::new(),
        modules,
        dependency_edge_count: package.dependency_edges.len(),
        dependency_rows,
        exported_surface_rows: package.exported_surface_rows(),
        runtime_requirements,
        entrypoints,
        routines,
    }
}

pub fn lower_workspace_package(package: &HirWorkspacePackage) -> IrPackage {
    let mut lowered = lower_package(&package.summary);
    lowered.direct_deps = package.direct_deps.iter().cloned().collect();
    lowered
}

pub fn lower_workspace_package_with_resolution(
    workspace: &HirWorkspaceSummary,
    resolved_workspace: &HirResolvedWorkspace,
    package: &HirWorkspacePackage,
) -> IrPackage {
    let mut lowered = lower_package(&package.summary);
    lowered.direct_deps = package.direct_deps.iter().cloned().collect();
    let Some(resolved_package) = resolved_workspace.package(&package.summary.package_name) else {
        return lowered;
    };
    lowered.routines = package
        .summary
        .modules
        .iter()
        .flat_map(|module| {
            let Some(resolved_module) = resolved_package.module(&module.module_id) else {
                return Vec::new();
            };
            let mut routines = module
                .symbols
                .iter()
                .filter(|symbol| is_routine_symbol(symbol))
                .map(|symbol| {
                    lower_routine_resolved(workspace, resolved_module, &module.module_id, symbol)
                })
                .collect::<Vec<_>>();
            routines.extend(
                module
                    .impls
                    .iter()
                    .flat_map(|impl_decl| impl_decl.methods.iter())
                    .filter(|symbol| is_routine_symbol(symbol))
                    .map(|symbol| {
                        lower_routine_resolved(
                            workspace,
                            resolved_module,
                            &module.module_id,
                            symbol,
                        )
                    }),
            );
            routines
        })
        .collect();
    lowered
}

#[cfg(test)]
mod tests {
    use super::{
        IrModule, lower_hir, lower_package, lower_workspace_package,
        lower_workspace_package_with_resolution,
    };
    use arcana_hir::{
        HirModule, build_package_layout, build_package_summary, build_workspace_package,
        build_workspace_summary, lower_module_text, resolve_workspace,
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
        assert_eq!(ir.runtime_requirements, vec!["std.canvas".to_string()]);
        assert!(ir.entrypoints.is_empty());
        assert_eq!(ir.routines.len(), 1);
        assert_eq!(ir.routines[0].symbol_name, "open");
        assert!(ir.routines[0].param_rows.is_empty());
        assert_eq!(
            ir.routines[0].statement_rows,
            vec!["stmt(core=return(int(0)),forewords=[],rollups=[])".to_string()]
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

        let ir = lower_workspace_package_with_resolution(&workspace, &resolved, package);
        let main = ir
            .routines
            .iter()
            .find(|routine| routine.symbol_name == "main")
            .expect("main routine should exist");
        assert!(
            main.statement_rows.iter().any(|row| row
                .contains("kind=bare_method,qualifier=len,resolved=std.collections.list.len")),
            "expected resolved bare-method callable path in lowered statements: {:?}",
            main.statement_rows
        );
    }
}
