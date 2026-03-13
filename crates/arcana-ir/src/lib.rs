use std::collections::BTreeSet;

use arcana_hir::{
    HirAssignOp, HirAssignTarget, HirBinaryOp, HirChainConnector, HirChainIntroducer, HirChainStep,
    HirDirectiveKind, HirExpr, HirForewordApp, HirForewordArg, HirHeaderAttachment, HirMatchArm,
    HirMatchPattern, HirModule, HirModuleDependency, HirModuleSummary, HirPackageSummary,
    HirPageRollup, HirPhraseArg, HirStatement, HirStatementKind, HirSymbol, HirSymbolKind,
    HirUnaryOp, HirWorkspacePackage,
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

#[cfg(test)]
mod tests {
    use super::{IrModule, lower_hir, lower_package, lower_workspace_package};
    use arcana_hir::{
        HirModule, build_package_layout, build_package_summary, build_workspace_package,
        lower_module_text,
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
}
