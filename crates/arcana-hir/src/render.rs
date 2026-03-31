use super::{
    HirAssignTarget, HirAvailabilityAttachment, HirBehaviorAttr, HirBinaryOp, HirBindLineKind,
    HirChainConnector, HirChainIntroducer, HirChainStep, HirCleanupFooter, HirConstructDestination,
    HirConstructRegion, HirDirective, HirEnumVariant, HirExpr, HirForewordApp, HirForewordArg,
    HirHeadedModifier, HirHeadedModifierKind, HirHeaderAttachment, HirImplAssocTypeBinding,
    HirImplDecl, HirLangItem, HirMatchArm, HirMatchPattern, HirMemorySpecDecl, HirModuleDependency,
    HirOwnerExit, HirOwnerObject, HirPhraseArg, HirRecycleLineKind, HirStatement, HirStatementKind,
    HirSymbol, HirSymbolBody, HirUnaryOp, signature::render_symbol_signature,
};

pub(crate) fn encode_surface_text(text: &str) -> String {
    text.to_string()
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

pub(crate) fn quote_fingerprint_text(text: impl ToString) -> String {
    let escaped = text.to_string().replace('\\', "\\\\").replace('|', "\\|");
    escaped.replace('[', "\\[").replace(']', "\\]")
}

pub(crate) fn render_directive_fingerprint(directive: &HirDirective) -> String {
    format!(
        "directive(kind={}|path=[{}]|alias={}|forewords=[{}])",
        directive.kind.as_str(),
        directive
            .path
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(","),
        directive
            .alias
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string()),
        directive
            .forewords
            .iter()
            .map(render_foreword_fingerprint)
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(crate) fn render_lang_item_fingerprint(lang_item: &HirLangItem) -> String {
    format!(
        "lang(name={}|target=[{}])",
        quote_fingerprint_text(&lang_item.name),
        lang_item
            .target
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub fn render_symbol_fingerprint(symbol: &HirSymbol) -> String {
    format!(
        concat!(
            "symbol(",
            "kind={}|name={}|exported={}|async={}|signature={}|type_params=[{}]|",
            "where_clause={}|behavior_attrs=[{}]|availability=[{}]|forewords=[{}]|intrinsic={}|body={}|",
            "statements=[{}]|cleanup_footers=[{}])"
        ),
        symbol.kind.as_str(),
        quote_fingerprint_text(&symbol.name),
        symbol.exported,
        symbol.is_async,
        quote_fingerprint_text(render_symbol_signature(symbol)),
        symbol
            .type_params
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .where_clause
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string()),
        symbol
            .behavior_attrs
            .iter()
            .map(render_behavior_attr_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .availability
            .iter()
            .map(render_availability_attachment_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .forewords
            .iter()
            .map(render_foreword_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .intrinsic_impl
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string()),
        render_symbol_body_fingerprint(&symbol.body),
        symbol
            .statements
            .iter()
            .map(render_statement_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        symbol
            .cleanup_footers
            .iter()
            .map(render_rollup_fingerprint)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_behavior_attr_fingerprint(attr: &HirBehaviorAttr) -> String {
    format!(
        "attr(name={}|value={})",
        quote_fingerprint_text(&attr.name),
        quote_fingerprint_text(&attr.value)
    )
}

fn render_foreword_fingerprint(foreword: &HirForewordApp) -> String {
    format!(
        "foreword(name={}|args=[{}])",
        quote_fingerprint_text(&foreword.name),
        foreword
            .args
            .iter()
            .map(render_foreword_arg_fingerprint)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_foreword_arg_fingerprint(arg: &HirForewordArg) -> String {
    format!(
        "arg(name={}|value={})",
        arg.name
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string()),
        quote_fingerprint_text(&arg.value)
    )
}

fn render_symbol_body_fingerprint(body: &HirSymbolBody) -> String {
    match body {
        HirSymbolBody::None => "none".to_string(),
        HirSymbolBody::Record { fields } => format!(
            "record([{}])",
            fields
                .iter()
                .map(render_field_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirSymbolBody::Object { fields, methods } => format!(
            "object(fields=[{}]|methods=[{}])",
            fields
                .iter()
                .map(render_field_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            methods
                .iter()
                .map(render_symbol_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirSymbolBody::Enum { variants } => format!(
            "enum([{}])",
            variants
                .iter()
                .map(render_enum_variant_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirSymbolBody::Owner { objects, exits } => format!(
            "owner(objects=[{}]|exits=[{}])",
            objects
                .iter()
                .map(render_owner_object_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            exits
                .iter()
                .map(render_owner_exit_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirSymbolBody::Trait {
            assoc_types,
            methods,
        } => format!(
            "trait(assoc_types=[{}]|methods=[{}])",
            assoc_types
                .iter()
                .map(render_trait_assoc_type_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            methods
                .iter()
                .map(render_symbol_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_field_fingerprint(field: &super::HirField) -> String {
    format!(
        "field(name={}|ty={})",
        quote_fingerprint_text(&field.name),
        quote_fingerprint_text(&field.ty)
    )
}

fn render_enum_variant_fingerprint(variant: &HirEnumVariant) -> String {
    format!(
        "variant(name={}|payload={})",
        quote_fingerprint_text(&variant.name),
        variant
            .payload
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string())
    )
}

fn render_owner_object_fingerprint(object: &HirOwnerObject) -> String {
    format!(
        "object(type=[{}]|name={})",
        object
            .type_path
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(","),
        quote_fingerprint_text(&object.local_name)
    )
}

fn render_owner_exit_fingerprint(owner_exit: &HirOwnerExit) -> String {
    format!(
        "exit(name={}|condition={}|holds=[{}])",
        quote_fingerprint_text(&owner_exit.name),
        render_expr_fingerprint(&owner_exit.condition),
        owner_exit
            .holds
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_trait_assoc_type_fingerprint(assoc_type: &super::HirTraitAssocType) -> String {
    format!(
        "assoc_type(name={}|default={})",
        quote_fingerprint_text(&assoc_type.name),
        assoc_type
            .default_ty
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string())
    )
}

pub(crate) fn render_impl_fingerprint(impl_decl: &HirImplDecl) -> String {
    format!(
        concat!(
            "impl(type_params=[{}]|trait={}|target={}|assoc_types=[{}]|methods=[{}]|",
            "body_entries=[{}])"
        ),
        impl_decl
            .type_params
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(","),
        impl_decl
            .trait_path
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string()),
        quote_fingerprint_text(&impl_decl.target_type),
        impl_decl
            .assoc_types
            .iter()
            .map(render_impl_assoc_type_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        impl_decl
            .methods
            .iter()
            .map(render_symbol_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        impl_decl
            .body_entries
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_impl_assoc_type_fingerprint(assoc_type: &HirImplAssocTypeBinding) -> String {
    format!(
        "assoc(name={}|value={})",
        quote_fingerprint_text(&assoc_type.name),
        assoc_type
            .value_ty
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string())
    )
}

fn render_rollup_fingerprint(rollup: &HirCleanupFooter) -> String {
    format!(
        "rollup(kind={}|subject={}|handler=[{}])",
        rollup.kind.as_str(),
        quote_fingerprint_text(&rollup.subject),
        rollup
            .handler_path
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_statement_fingerprint(statement: &HirStatement) -> String {
    format!(
        "stmt(availability=[{}]|forewords=[{}]|cleanup_footers=[{}]|kind={})",
        statement
            .availability
            .iter()
            .map(render_availability_attachment_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        statement
            .forewords
            .iter()
            .map(render_foreword_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        statement
            .cleanup_footers
            .iter()
            .map(render_rollup_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        render_statement_kind_fingerprint(&statement.kind)
    )
}

fn render_availability_attachment_fingerprint(attachment: &HirAvailabilityAttachment) -> String {
    format!(
        "availability(path=[{}])",
        attachment
            .path
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_statement_kind_fingerprint(kind: &HirStatementKind) -> String {
    match kind {
        HirStatementKind::Let {
            mutable,
            name,
            value,
        } => format!(
            "let(mutable={}|name={}|value={})",
            mutable,
            quote_fingerprint_text(name),
            render_expr_fingerprint(value)
        ),
        HirStatementKind::Return { value } => format!(
            "return({})",
            value
                .as_ref()
                .map(render_expr_fingerprint)
                .unwrap_or_else(|| "none".to_string())
        ),
        HirStatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if(cond={}|then=[{}]|else={})",
            render_expr_fingerprint(condition),
            then_branch
                .iter()
                .map(render_statement_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            else_branch
                .as_ref()
                .map(|branch| format!(
                    "[{}]",
                    branch
                        .iter()
                        .map(render_statement_fingerprint)
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .unwrap_or_else(|| "none".to_string())
        ),
        HirStatementKind::While { condition, body } => format!(
            "while(cond={}|body=[{}])",
            render_expr_fingerprint(condition),
            body.iter()
                .map(render_statement_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::For {
            binding,
            iterable,
            body,
        } => format!(
            "for(binding={}|iterable={}|body=[{}])",
            quote_fingerprint_text(binding),
            render_expr_fingerprint(iterable),
            body.iter()
                .map(render_statement_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::Defer { expr } => format!("defer({})", render_expr_fingerprint(expr)),
        HirStatementKind::Break => "break".to_string(),
        HirStatementKind::Continue => "continue".to_string(),
        HirStatementKind::Assign { target, op, value } => format!(
            "assign(target={}|op={}|value={})",
            render_assign_target_fingerprint(target),
            op.as_str(),
            render_expr_fingerprint(value)
        ),
        HirStatementKind::Recycle {
            default_modifier,
            lines,
        } => format!(
            "recycle(default_modifier={}|lines=[{}])",
            default_modifier
                .as_ref()
                .map(render_headed_modifier_fingerprint)
                .unwrap_or_else(|| "none".to_string()),
            lines
                .iter()
                .map(|line| {
                    let kind = match &line.kind {
                        HirRecycleLineKind::Expr { gate } => {
                            format!("expr({})", render_expr_fingerprint(gate))
                        }
                        HirRecycleLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => format!(
                            "let(mutable={}|name={}|gate={})",
                            mutable,
                            quote_fingerprint_text(name),
                            render_expr_fingerprint(gate)
                        ),
                        HirRecycleLineKind::Assign { name, gate } => format!(
                            "assign(name={}|gate={})",
                            quote_fingerprint_text(name),
                            render_expr_fingerprint(gate)
                        ),
                    };
                    format!(
                        "line(kind={}|modifier={})",
                        kind,
                        line.modifier
                            .as_ref()
                            .map(render_headed_modifier_fingerprint)
                            .unwrap_or_else(|| "none".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::Bind {
            default_modifier,
            lines,
        } => format!(
            "bind(default_modifier={}|lines=[{}])",
            default_modifier
                .as_ref()
                .map(render_headed_modifier_fingerprint)
                .unwrap_or_else(|| "none".to_string()),
            lines
                .iter()
                .map(|line| {
                    let kind = match &line.kind {
                        HirBindLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => format!(
                            "let(mutable={}|name={}|gate={})",
                            mutable,
                            quote_fingerprint_text(name),
                            render_expr_fingerprint(gate)
                        ),
                        HirBindLineKind::Assign { name, gate } => format!(
                            "assign(name={}|gate={})",
                            quote_fingerprint_text(name),
                            render_expr_fingerprint(gate)
                        ),
                        HirBindLineKind::Require { expr } => {
                            format!("require({})", render_expr_fingerprint(expr))
                        }
                    };
                    format!(
                        "line(kind={}|modifier={})",
                        kind,
                        line.modifier
                            .as_ref()
                            .map(render_headed_modifier_fingerprint)
                            .unwrap_or_else(|| "none".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirStatementKind::Construct(region) => {
            format!("construct({})", render_construct_region_fingerprint(region))
        }
        HirStatementKind::MemorySpec(spec) => {
            format!("memory_spec({})", render_memory_spec_fingerprint(spec))
        }
        HirStatementKind::Expr { expr } => format!("expr({})", render_expr_fingerprint(expr)),
    }
}

fn render_assign_target_fingerprint(target: &HirAssignTarget) -> String {
    match target {
        HirAssignTarget::Name { text } => format!("name({})", quote_fingerprint_text(text)),
        HirAssignTarget::MemberAccess { target, member } => format!(
            "member(base={}|member={})",
            render_assign_target_fingerprint(target),
            quote_fingerprint_text(member)
        ),
        HirAssignTarget::Index { target, index } => format!(
            "index(base={}|index={})",
            render_assign_target_fingerprint(target),
            render_expr_fingerprint(index)
        ),
    }
}

pub fn render_expr_fingerprint(expr: &HirExpr) -> String {
    match expr {
        HirExpr::Path { segments } => format!(
            "path([{}])",
            segments
                .iter()
                .map(quote_fingerprint_text)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::BoolLiteral { value } => format!("bool({value})"),
        HirExpr::IntLiteral { text } => format!("int({})", quote_fingerprint_text(text)),
        HirExpr::StrLiteral { text } => format!("str({})", quote_fingerprint_text(text)),
        HirExpr::Pair { left, right } => format!(
            "pair({},{})",
            render_expr_fingerprint(left),
            render_expr_fingerprint(right)
        ),
        HirExpr::Range {
            start,
            end,
            inclusive_end,
        } => format!(
            "range(start={}|end={}|inclusive={})",
            start
                .as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            inclusive_end
        ),
        HirExpr::CollectionLiteral { items } => format!(
            "collection([{}])",
            items
                .iter()
                .map(render_expr_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::ConstructRegion(region) => {
            format!("construct({})", render_construct_region_fingerprint(region))
        }
        HirExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => format!(
            "memory(family={}|arena={}|init_args=[{}]|constructor={}|attached=[{}])",
            quote_fingerprint_text(family),
            render_expr_fingerprint(arena),
            init_args
                .iter()
                .map(render_phrase_arg_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            render_expr_fingerprint(constructor),
            attached
                .iter()
                .map(render_header_attachment_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::QualifiedPhrase {
            subject,
            qualifier,
            args,
            attached,
        } => format!(
            "qualified_phrase(subject={}|qualifier={}|args=[{}]|headers=[{}])",
            render_expr_fingerprint(subject),
            quote_fingerprint_text(qualifier),
            args.iter()
                .map(render_phrase_arg_fingerprint)
                .collect::<Vec<_>>()
                .join(","),
            attached
                .iter()
                .map(render_header_attachment_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::MemberAccess { expr, member } => format!(
            "member_access(expr={}|member={})",
            render_expr_fingerprint(expr),
            quote_fingerprint_text(member)
        ),
        HirExpr::Await { expr } => format!("await({})", render_expr_fingerprint(expr)),
        HirExpr::Unary { op, expr } => format!(
            "unary(op={}|expr={})",
            render_unary_op_fingerprint(*op),
            render_expr_fingerprint(expr)
        ),
        HirExpr::Binary { left, op, right } => format!(
            "binary(left={}|op={}|right={})",
            render_expr_fingerprint(left),
            render_binary_op_fingerprint(*op),
            render_expr_fingerprint(right)
        ),
        HirExpr::Index { expr, index } => format!(
            "index(expr={}|index={})",
            render_expr_fingerprint(expr),
            render_expr_fingerprint(index)
        ),
        HirExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => format!(
            "slice(expr={}|start={}|end={}|inclusive={})",
            render_expr_fingerprint(expr),
            start
                .as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            end.as_ref()
                .map(|expr| render_expr_fingerprint(expr))
                .unwrap_or_else(|| "none".to_string()),
            inclusive_end
        ),
        HirExpr::Match { subject, arms } => format!(
            "match(subject={}|arms=[{}])",
            render_expr_fingerprint(subject),
            arms.iter()
                .map(render_match_arm_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::Chain {
            style,
            introducer,
            steps,
        } => format!(
            "chain(style={}|introducer={}|steps=[{}])",
            quote_fingerprint_text(style),
            render_chain_introducer_fingerprint(*introducer),
            steps
                .iter()
                .map(render_chain_step_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirExpr::GenericApply { expr, type_args } => format!(
            "generic_apply(expr={}|type_args=[{}])",
            render_expr_fingerprint(expr),
            type_args
                .iter()
                .map(quote_fingerprint_text)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_headed_modifier_fingerprint(modifier: &HirHeadedModifier) -> String {
    let kind = match &modifier.kind {
        HirHeadedModifierKind::Keyword(keyword) => keyword.as_str().to_string(),
        HirHeadedModifierKind::Name(name) => quote_fingerprint_text(name),
    };
    format!(
        "modifier(kind={}|payload={})",
        kind,
        modifier
            .payload
            .as_ref()
            .map(render_expr_fingerprint)
            .unwrap_or_else(|| "none".to_string())
    )
}

fn render_construct_region_fingerprint(region: &HirConstructRegion) -> String {
    format!(
        "completion={}|target={}|destination={}|default_modifier={}|lines=[{}]",
        region.completion.as_str(),
        render_expr_fingerprint(&region.target),
        region
            .destination
            .as_ref()
            .map(|destination| match destination {
                HirConstructDestination::Deliver { name } => {
                    format!("deliver({})", quote_fingerprint_text(name))
                }
                HirConstructDestination::Place { target } => {
                    format!("place({})", render_assign_target_fingerprint(target))
                }
            })
            .unwrap_or_else(|| "none".to_string()),
        region
            .default_modifier
            .as_ref()
            .map(render_headed_modifier_fingerprint)
            .unwrap_or_else(|| "none".to_string()),
        region
            .lines
            .iter()
            .map(|line| {
                format!(
                    "contrib(name={}|value={}|modifier={})",
                    quote_fingerprint_text(&line.name),
                    render_expr_fingerprint(&line.value),
                    line.modifier
                        .as_ref()
                        .map(render_headed_modifier_fingerprint)
                        .unwrap_or_else(|| "none".to_string())
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(crate) fn render_memory_spec_fingerprint(spec: &HirMemorySpecDecl) -> String {
    format!(
        "family={}|name={}|default_modifier={}|details=[{}]",
        spec.family.as_str(),
        quote_fingerprint_text(&spec.name),
        spec.default_modifier
            .as_ref()
            .map(render_headed_modifier_fingerprint)
            .unwrap_or_else(|| "none".to_string()),
        spec.details
            .iter()
            .map(|detail| {
                format!(
                    "detail(key={}|value={}|modifier={})",
                    detail.key.as_str(),
                    render_expr_fingerprint(&detail.value),
                    detail
                        .modifier
                        .as_ref()
                        .map(render_headed_modifier_fingerprint)
                        .unwrap_or_else(|| "none".to_string())
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn render_phrase_arg_fingerprint(arg: &HirPhraseArg) -> String {
    match arg {
        HirPhraseArg::Positional(expr) => format!("pos({})", render_expr_fingerprint(expr)),
        HirPhraseArg::Named { name, value } => format!(
            "named(name={}|value={})",
            quote_fingerprint_text(name),
            render_expr_fingerprint(value)
        ),
    }
}

fn render_match_arm_fingerprint(arm: &HirMatchArm) -> String {
    format!(
        "arm(patterns=[{}]|value={})",
        arm.patterns
            .iter()
            .map(render_match_pattern_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        render_expr_fingerprint(&arm.value)
    )
}

fn render_match_pattern_fingerprint(pattern: &HirMatchPattern) -> String {
    match pattern {
        HirMatchPattern::Wildcard => "wildcard".to_string(),
        HirMatchPattern::Literal { text } => format!("literal({})", quote_fingerprint_text(text)),
        HirMatchPattern::Name { text } => format!("name({})", quote_fingerprint_text(text)),
        HirMatchPattern::Variant { path, args } => format!(
            "variant(path={}|args=[{}])",
            quote_fingerprint_text(path),
            args.iter()
                .map(render_match_pattern_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn render_chain_step_fingerprint(step: &HirChainStep) -> String {
    format!(
        "step(incoming={}|stage={}|bind_args=[{}]|text={})",
        step.incoming
            .map(render_chain_connector_fingerprint)
            .unwrap_or("none"),
        render_expr_fingerprint(&step.stage),
        step.bind_args
            .iter()
            .map(render_expr_fingerprint)
            .collect::<Vec<_>>()
            .join(","),
        quote_fingerprint_text(&step.text)
    )
}

fn render_header_attachment_fingerprint(attachment: &HirHeaderAttachment) -> String {
    match attachment {
        HirHeaderAttachment::Named {
            name,
            value,
            forewords,
            ..
        } => format!(
            "named(name={}|value={}|forewords=[{}])",
            quote_fingerprint_text(name),
            render_expr_fingerprint(value),
            forewords
                .iter()
                .map(render_foreword_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        HirHeaderAttachment::Chain {
            expr, forewords, ..
        } => format!(
            "chain(expr={}|forewords=[{}])",
            render_expr_fingerprint(expr),
            forewords
                .iter()
                .map(render_foreword_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

pub(crate) fn render_dependency_edge_fingerprint(edge: &HirModuleDependency) -> String {
    format!(
        "dep(source={}|kind={}|target=[{}]|alias={})",
        quote_fingerprint_text(&edge.source_module_id),
        edge.kind.as_str(),
        edge.target_path
            .iter()
            .map(quote_fingerprint_text)
            .collect::<Vec<_>>()
            .join(","),
        edge.alias
            .as_ref()
            .map(quote_fingerprint_text)
            .unwrap_or_else(|| "none".to_string())
    )
}

fn render_chain_connector_fingerprint(connector: HirChainConnector) -> &'static str {
    match connector {
        HirChainConnector::Forward => "forward",
        HirChainConnector::Reverse => "reverse",
    }
}

fn render_chain_introducer_fingerprint(introducer: HirChainIntroducer) -> &'static str {
    match introducer {
        HirChainIntroducer::Forward => "forward",
        HirChainIntroducer::Reverse => "reverse",
    }
}

fn render_unary_op_fingerprint(op: HirUnaryOp) -> &'static str {
    match op {
        HirUnaryOp::Neg => "neg",
        HirUnaryOp::Not => "not",
        HirUnaryOp::BitNot => "bit_not",
        HirUnaryOp::BorrowRead => "borrow_read",
        HirUnaryOp::BorrowMut => "borrow_mut",
        HirUnaryOp::Deref => "deref",
        HirUnaryOp::Weave => "weave",
        HirUnaryOp::Split => "split",
    }
}

fn render_binary_op_fingerprint(op: HirBinaryOp) -> &'static str {
    match op {
        HirBinaryOp::Or => "or",
        HirBinaryOp::And => "and",
        HirBinaryOp::EqEq => "eq_eq",
        HirBinaryOp::NotEq => "not_eq",
        HirBinaryOp::Lt => "lt",
        HirBinaryOp::LtEq => "lt_eq",
        HirBinaryOp::Gt => "gt",
        HirBinaryOp::GtEq => "gt_eq",
        HirBinaryOp::BitOr => "bit_or",
        HirBinaryOp::BitXor => "bit_xor",
        HirBinaryOp::BitAnd => "bit_and",
        HirBinaryOp::Shl => "shl",
        HirBinaryOp::Shr => "shr",
        HirBinaryOp::Add => "add",
        HirBinaryOp::Sub => "sub",
        HirBinaryOp::Mul => "mul",
        HirBinaryOp::Div => "div",
        HirBinaryOp::Mod => "mod",
    }
}
