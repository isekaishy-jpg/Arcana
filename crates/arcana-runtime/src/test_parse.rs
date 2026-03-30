use super::*;

fn split_top_level(text: &str) -> Vec<String> {
    split_top_level_with_delim(text, ',')
}

fn split_top_level_with_delim(text: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in text.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
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
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            ch if ch == delimiter && paren_depth == 0 && bracket_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn parse_named_fields(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut fields = BTreeMap::new();
    for part in split_top_level(text) {
        let Some((name, value)) = part.split_once('=') else {
            return Err(format!("expected named field in `{part}`"));
        };
        fields.insert(name.trim().to_string(), value.trim().to_string());
    }
    Ok(fields)
}

fn parse_list(text: &str) -> Result<Vec<String>, String> {
    let inner = strip_prefix_suffix(text, "[", "]")?;
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(split_top_level(inner))
}

pub(crate) fn parse_cleanup_footer_row(text: &str) -> Result<ParsedCleanupFooter, String> {
    let parts = text.splitn(3, ':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!("malformed cleanup footer row `{text}`"));
    }
    let kind = parts[0].trim();
    let subject = parts[1].trim();
    let handler = parts[2].trim();
    if kind != "cleanup" {
        return Err(format!("unsupported cleanup footer kind `{kind}`"));
    }
    if subject.is_empty() {
        return Err(format!("cleanup footer row missing subject in `{text}`"));
    }
    if handler.is_empty() {
        return Err(format!(
            "cleanup footer row missing handler path in `{text}`"
        ));
    }
    Ok(ParsedCleanupFooter {
        kind: kind.to_string(),
        binding_id: 0,
        subject: subject.to_string(),
        handler_path: handler.split('.').map(ToString::to_string).collect(),
        resolved_routine: None,
    })
}

pub(crate) fn parse_rollup_row(text: &str) -> Result<ParsedCleanupFooter, String> {
    parse_cleanup_footer_row(text)
}

fn parse_chain_connector(text: &str) -> Result<Option<ParsedChainConnector>, String> {
    match text.trim() {
        "start" => Ok(None),
        "=>" => Ok(Some(ParsedChainConnector::Forward)),
        "<=" => Ok(Some(ParsedChainConnector::Reverse)),
        other => Err(format!("unsupported runtime chain connector `{other}`")),
    }
}

fn parse_chain_introducer(text: &str) -> Result<ParsedChainIntroducer, String> {
    match text.trim() {
        "forward" => Ok(ParsedChainIntroducer::Forward),
        "reverse" => Ok(ParsedChainIntroducer::Reverse),
        other => Err(format!("unsupported runtime chain introducer `{other}`")),
    }
}

fn parse_chain_step(text: &str) -> Result<ParsedChainStep, String> {
    let inner = strip_prefix_suffix(text, "step(", ")")?;
    let parts = split_top_level(inner);
    if parts.len() != 4 {
        return Err(format!("malformed runtime chain step `{text}`"));
    }
    let incoming = parse_chain_connector(&parts[0])?;
    let fields = parse_named_fields(&parts[1..].join(","))?;
    let stage = parse_expr(
        fields
            .get("stage")
            .ok_or_else(|| format!("runtime chain step missing stage in `{text}`"))?,
    )?;
    let bind_args = parse_list(
        fields
            .get("bind")
            .ok_or_else(|| format!("runtime chain step missing bind args in `{text}`"))?,
    )?
    .into_iter()
    .map(|item| parse_expr(&item))
    .collect::<Result<Vec<_>, String>>()?;
    let text_value = decode_row_string(
        fields
            .get("text")
            .ok_or_else(|| format!("runtime chain step missing text in `{text}`"))?,
    )?;
    Ok(ParsedChainStep {
        incoming,
        stage,
        bind_args,
        text: text_value,
    })
}

fn parse_match_pattern(text: &str) -> Result<ParsedMatchPattern, String> {
    if text == "_" {
        return Ok(ParsedMatchPattern::Wildcard);
    }
    if text.starts_with("name(") && text.ends_with(')') {
        let name = strip_prefix_suffix(text, "name(", ")")?;
        if name.contains('.') {
            return Ok(ParsedMatchPattern::Variant {
                path: name.to_string(),
                args: Vec::new(),
            });
        }
        return Ok(ParsedMatchPattern::Name(name.to_string()));
    }
    if text.starts_with("lit(\"") && text.ends_with("\")") {
        return Ok(ParsedMatchPattern::Literal(decode_row_string(&format!(
            "\"{}\"",
            strip_prefix_suffix(text, "lit(\"", "\")")?
        ))?));
    }
    if text.starts_with("variant(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "variant(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed runtime match variant `{text}`"));
        }
        let args = parse_list(&parts[1])?
            .into_iter()
            .map(|item| parse_match_pattern(&item))
            .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedMatchPattern::Variant {
            path: parts[0].to_string(),
            args,
        });
    }
    Err(format!("unsupported runtime match pattern `{text}`"))
}

fn parse_match_arm(text: &str) -> Result<ParsedMatchArm, String> {
    let fields = parse_named_fields(strip_prefix_suffix(text, "arm(", ")")?)?;
    let patterns_src = fields
        .get("patterns")
        .ok_or_else(|| format!("runtime arm missing patterns in `{text}`"))?;
    let patterns_inner = strip_prefix_suffix(patterns_src, "[", "]")?;
    let patterns = if patterns_inner.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level_with_delim(patterns_inner, '|')
            .into_iter()
            .map(|item| parse_match_pattern(&item))
            .collect::<Result<Vec<_>, String>>()?
    };
    let value = parse_expr(
        fields
            .get("value")
            .ok_or_else(|| format!("runtime arm missing value in `{text}`"))?,
    )?;
    Ok(ParsedMatchArm { patterns, value })
}

fn decode_row_string(text: &str) -> Result<String, String> {
    let inner = strip_prefix_suffix(text, "\"", "\"")?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                return Err("unterminated escape in runtime string".to_string());
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

fn parse_unary_op(text: &str) -> Result<ParsedUnaryOp, String> {
    match text {
        "-" => Ok(ParsedUnaryOp::Neg),
        "not" => Ok(ParsedUnaryOp::Not),
        "~" => Ok(ParsedUnaryOp::BitNot),
        "&" => Ok(ParsedUnaryOp::BorrowRead),
        "&mut" => Ok(ParsedUnaryOp::BorrowMut),
        "*" => Ok(ParsedUnaryOp::Deref),
        "weave" => Ok(ParsedUnaryOp::Weave),
        "split" => Ok(ParsedUnaryOp::Split),
        _ => Err(format!("unsupported runtime unary op `{text}`")),
    }
}

fn parse_binary_op(text: &str) -> Result<ParsedBinaryOp, String> {
    match text {
        "or" => Ok(ParsedBinaryOp::Or),
        "and" => Ok(ParsedBinaryOp::And),
        "==" => Ok(ParsedBinaryOp::EqEq),
        "!=" => Ok(ParsedBinaryOp::NotEq),
        "<" => Ok(ParsedBinaryOp::Lt),
        "<=" => Ok(ParsedBinaryOp::LtEq),
        ">" => Ok(ParsedBinaryOp::Gt),
        ">=" => Ok(ParsedBinaryOp::GtEq),
        "|" => Ok(ParsedBinaryOp::BitOr),
        "^" => Ok(ParsedBinaryOp::BitXor),
        "&" => Ok(ParsedBinaryOp::BitAnd),
        "<<" => Ok(ParsedBinaryOp::Shl),
        "shr" => Ok(ParsedBinaryOp::Shr),
        "+" => Ok(ParsedBinaryOp::Add),
        "-" => Ok(ParsedBinaryOp::Sub),
        "*" => Ok(ParsedBinaryOp::Mul),
        "/" => Ok(ParsedBinaryOp::Div),
        "%" => Ok(ParsedBinaryOp::Mod),
        _ => Err(format!("unsupported runtime binary op `{text}`")),
    }
}

fn parse_assign_op(text: &str) -> Result<ParsedAssignOp, String> {
    match text {
        "=" => Ok(ParsedAssignOp::Assign),
        "+=" => Ok(ParsedAssignOp::AddAssign),
        "-=" => Ok(ParsedAssignOp::SubAssign),
        "*=" => Ok(ParsedAssignOp::MulAssign),
        "/=" => Ok(ParsedAssignOp::DivAssign),
        "%=" => Ok(ParsedAssignOp::ModAssign),
        "&=" => Ok(ParsedAssignOp::BitAndAssign),
        "|=" => Ok(ParsedAssignOp::BitOrAssign),
        "^=" => Ok(ParsedAssignOp::BitXorAssign),
        "<<=" => Ok(ParsedAssignOp::ShlAssign),
        "shr=" => Ok(ParsedAssignOp::ShrAssign),
        _ => Err(format!("unsupported runtime assign op `{text}`")),
    }
}

fn parse_expr(text: &str) -> Result<ParsedExpr, String> {
    if let Some(inner) = text
        .strip_prefix("int(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return inner
            .parse::<i64>()
            .map(ParsedExpr::Int)
            .map_err(|err| format!("invalid runtime int `{inner}`: {err}"));
    }
    if let Some(inner) = text
        .strip_prefix("bool(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return match inner {
            "true" => Ok(ParsedExpr::Bool(true)),
            "false" => Ok(ParsedExpr::Bool(false)),
            _ => Err(format!("invalid runtime bool `{inner}`")),
        };
    }
    if text.starts_with("str(") && text.ends_with(')') {
        return Ok(ParsedExpr::Str(decode_source_string_literal(
            strip_prefix_suffix(text, "str(", ")")?,
        )?));
    }
    if text.starts_with("pair(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "pair(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed pair expression `{text}`"));
        }
        return Ok(ParsedExpr::Pair {
            left: Box::new(parse_expr(&parts[0])?),
            right: Box::new(parse_expr(&parts[1])?),
        });
    }
    if text.starts_with("collection(") && text.ends_with(')') {
        let items = parse_list(strip_prefix_suffix(text, "collection(", ")")?)?
            .into_iter()
            .map(|item| parse_expr(&item))
            .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedExpr::Collection { items });
    }
    if text.starts_with("match(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "match(", ")")?)?;
        let arms = parse_list(
            fields
                .get("arms")
                .ok_or_else(|| format!("match expression missing arms in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_match_arm(&item))
        .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedExpr::Match {
            subject: Box::new(parse_expr(fields.get("subject").ok_or_else(|| {
                format!("match expression missing subject in `{text}`")
            })?)?),
            arms,
        });
    }
    if text.starts_with("chain(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "chain(", ")")?)?;
        let style = fields
            .get("style")
            .ok_or_else(|| format!("chain expression missing style in `{text}`"))?
            .to_string();
        let introducer = parse_chain_introducer(
            fields
                .get("introducer")
                .ok_or_else(|| format!("chain expression missing introducer in `{text}`"))?,
        )?;
        let steps = parse_list(
            fields
                .get("steps")
                .ok_or_else(|| format!("chain expression missing steps in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_chain_step(&item))
        .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedExpr::Chain {
            style,
            introducer,
            steps,
        });
    }
    if text.starts_with("await(") && text.ends_with(')') {
        return Ok(ParsedExpr::Await {
            expr: Box::new(parse_expr(strip_prefix_suffix(text, "await(", ")")?)?),
        });
    }
    if text.starts_with("memory(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "memory(", ")")?)?;
        let init_args = parse_list(
            fields
                .get("init")
                .ok_or_else(|| format!("memory phrase missing init args in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_phrase_arg(&item))
        .collect::<Result<Vec<_>, String>>()?;
        let attached = parse_list(
            fields
                .get("attached")
                .ok_or_else(|| format!("memory phrase missing attached args in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_header_attachment(&item))
        .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedExpr::MemoryPhrase {
            family: fields
                .get("family")
                .ok_or_else(|| format!("memory phrase missing family in `{text}`"))?
                .to_string(),
            arena: Box::new(parse_expr(
                fields
                    .get("arena")
                    .ok_or_else(|| format!("memory phrase missing arena in `{text}`"))?,
            )?),
            init_args,
            constructor: Box::new(parse_expr(
                fields
                    .get("ctor")
                    .ok_or_else(|| format!("memory phrase missing constructor in `{text}`"))?,
            )?),
            attached,
        });
    }
    if text.starts_with("path(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "path(", ")")?;
        if inner.is_empty() {
            return Err("empty path in runtime row".to_string());
        }
        return Ok(ParsedExpr::Path(
            inner.split('.').map(ToString::to_string).collect(),
        ));
    }
    if text.starts_with("member(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "member(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed member expression `{text}`"));
        }
        return Ok(ParsedExpr::Member {
            expr: Box::new(parse_expr(&parts[0])?),
            member: parts[1].to_string(),
        });
    }
    if text.starts_with("index(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "index(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed index expression `{text}`"));
        }
        return Ok(ParsedExpr::Index {
            expr: Box::new(parse_expr(&parts[0])?),
            index: Box::new(parse_expr(&parts[1])?),
        });
    }
    if text.starts_with("slice(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "slice(", ")")?)?;
        return Ok(ParsedExpr::Slice {
            expr: Box::new(parse_expr(fields.get("expr").ok_or_else(|| {
                format!("slice expression missing expr in `{text}`")
            })?)?),
            start: parse_optional_runtime_expr(
                fields
                    .get("start")
                    .ok_or_else(|| format!("slice expression missing start in `{text}`"))?,
            )?,
            end: parse_optional_runtime_expr(
                fields
                    .get("end")
                    .ok_or_else(|| format!("slice expression missing end in `{text}`"))?,
            )?,
            inclusive_end: parse_runtime_bool_keyword(
                fields
                    .get("inclusive")
                    .ok_or_else(|| format!("slice expression missing inclusive in `{text}`"))?,
                "slice inclusive",
            )?,
        });
    }
    if text.starts_with("range(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "range(", ")")?)?;
        return Ok(ParsedExpr::Range {
            start: parse_optional_runtime_expr(
                fields
                    .get("start")
                    .ok_or_else(|| format!("range expression missing start in `{text}`"))?,
            )?,
            end: parse_optional_runtime_expr(
                fields
                    .get("end")
                    .ok_or_else(|| format!("range expression missing end in `{text}`"))?,
            )?,
            inclusive_end: parse_runtime_bool_keyword(
                fields
                    .get("inclusive")
                    .ok_or_else(|| format!("range expression missing inclusive in `{text}`"))?,
                "range inclusive",
            )?,
        });
    }
    if text.starts_with("unary(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "unary(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed unary expression `{text}`"));
        }
        return Ok(ParsedExpr::Unary {
            op: parse_unary_op(&parts[0])?,
            expr: Box::new(parse_expr(&parts[1])?),
        });
    }
    if text.starts_with("binary(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "binary(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 3 {
            return Err(format!("malformed binary expression `{text}`"));
        }
        return Ok(ParsedExpr::Binary {
            left: Box::new(parse_expr(&parts[0])?),
            op: parse_binary_op(&parts[1])?,
            right: Box::new(parse_expr(&parts[2])?),
        });
    }
    if text.starts_with("generic(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "generic(", ")")?)?;
        let type_args = parse_list(
            fields
                .get("types")
                .ok_or_else(|| format!("generic expression missing types in `{text}`"))?,
        )?;
        return Ok(ParsedExpr::Generic {
            expr: Box::new(parse_expr(fields.get("expr").ok_or_else(|| {
                format!("generic expression missing expr in `{text}`")
            })?)?),
            type_args,
        });
    }
    if text.starts_with("phrase(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "phrase(", ")")?)?;
        let args = parse_list(
            fields
                .get("args")
                .ok_or_else(|| format!("phrase missing args in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_phrase_arg(&item))
        .collect::<Result<Vec<_>, String>>()?;
        let attached = parse_list(
            fields
                .get("attached")
                .ok_or_else(|| format!("phrase missing attached in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_header_attachment(&item))
        .collect::<Result<Vec<_>, String>>()?;
        let qualifier = fields
            .get("qualifier")
            .ok_or_else(|| format!("phrase missing qualifier in `{text}`"))?
            .to_string();
        let qualifier_kind = match fields.get("kind") {
            Some(kind) => parse_phrase_qualifier_kind(kind)?,
            None => classify_phrase_qualifier_kind(&qualifier)?,
        };
        let resolved_callable = fields
            .get("resolved")
            .map(|text| parse_runtime_symbol_path(text))
            .transpose()?;
        let resolved_routine = fields
            .get("resolved_routine")
            .map(|text| parse_runtime_string_field(text, "phrase resolved_routine"))
            .transpose()?;
        return Ok(ParsedExpr::Phrase {
            subject: Box::new(parse_expr(
                fields
                    .get("subject")
                    .ok_or_else(|| format!("phrase missing subject in `{text}`"))?,
            )?),
            args,
            qualifier_kind,
            qualifier,
            resolved_callable,
            resolved_routine,
            dynamic_dispatch: None,
            attached,
        });
    }
    Err(format!("unsupported runtime expression `{text}`"))
}

fn parse_runtime_symbol_path(text: &str) -> Result<Vec<String>, String> {
    let path = text
        .split('.')
        .map(str::trim)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if path.is_empty()
        || path
            .iter()
            .any(|segment| !is_simple_runtime_identifier(segment))
    {
        return Err(format!("unsupported runtime symbol path `{text}`"));
    }
    Ok(path)
}

fn parse_runtime_string_field(text: &str, context: &str) -> Result<String, String> {
    match parse_expr(text)? {
        ParsedExpr::Str(value) => Ok(value),
        other => Err(format!(
            "{context} expected string literal field, got `{other:?}`"
        )),
    }
}

fn is_simple_runtime_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn parse_phrase_qualifier_kind(text: &str) -> Result<ParsedPhraseQualifierKind, String> {
    match text.trim() {
        "call" => Ok(ParsedPhraseQualifierKind::Call),
        "try" => Ok(ParsedPhraseQualifierKind::Try),
        "apply" => Ok(ParsedPhraseQualifierKind::Apply),
        "await_apply" => Ok(ParsedPhraseQualifierKind::AwaitApply),
        "bare_method" => Ok(ParsedPhraseQualifierKind::BareMethod),
        "named_path" => Ok(ParsedPhraseQualifierKind::NamedPath),
        other => Err(format!(
            "unsupported runtime phrase qualifier kind `{other}`"
        )),
    }
}

fn is_simple_runtime_path(text: &str) -> bool {
    text.split('.').all(is_simple_runtime_identifier)
}

fn classify_phrase_qualifier_kind(text: &str) -> Result<ParsedPhraseQualifierKind, String> {
    match text.trim() {
        "call" => Ok(ParsedPhraseQualifierKind::Call),
        "?" => Ok(ParsedPhraseQualifierKind::Try),
        ">" => Ok(ParsedPhraseQualifierKind::Apply),
        ">>" => Ok(ParsedPhraseQualifierKind::AwaitApply),
        other if is_simple_runtime_path(other) && other.contains('.') => {
            Ok(ParsedPhraseQualifierKind::NamedPath)
        }
        other if is_simple_runtime_path(other) => Ok(ParsedPhraseQualifierKind::BareMethod),
        other => Err(format!("unsupported runtime phrase qualifier `{other}`")),
    }
}

fn parse_runtime_bool_keyword(text: &str, context: &str) -> Result<bool, String> {
    match text.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid runtime bool `{other}` in {context}")),
    }
}

fn parse_optional_runtime_expr(text: &str) -> Result<Option<Box<ParsedExpr>>, String> {
    if text.trim() == "none" {
        Ok(None)
    } else {
        Ok(Some(Box::new(parse_expr(text)?)))
    }
}

fn parse_phrase_arg(text: &str) -> Result<ParsedPhraseArg, String> {
    if let Some((name, value)) = split_top_level_assignment(text) {
        return Ok(ParsedPhraseArg {
            name: Some(name.to_string()),
            value: parse_expr(value)?,
        });
    }
    Ok(ParsedPhraseArg {
        name: None,
        value: parse_expr(text)?,
    })
}

fn parse_header_attachment(text: &str) -> Result<ParsedHeaderAttachment, String> {
    if text.starts_with("named(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "named(", ")")?)?;
        let named_fields = fields
            .into_iter()
            .filter(|(name, _)| name != "forewords")
            .collect::<Vec<_>>();
        if named_fields.len() != 1 {
            return Err(format!(
                "named attachment must contain exactly one value in `{text}`"
            ));
        }
        let (name, value) = named_fields
            .into_iter()
            .next()
            .ok_or_else(|| format!("named attachment is empty in `{text}`"))?;
        return Ok(ParsedHeaderAttachment::Named {
            name,
            value: parse_expr(&value)?,
        });
    }
    if text.starts_with("chain(") && text.ends_with(')') {
        let parts = split_top_level(strip_prefix_suffix(text, "chain(", ")")?);
        if parts.len() != 2 {
            return Err(format!("malformed chain attachment `{text}`"));
        }
        let _ = split_top_level_assignment(&parts[1])
            .ok_or_else(|| format!("chain attachment missing forewords in `{text}`"))?;
        return Ok(ParsedHeaderAttachment::Chain {
            expr: parse_expr(&parts[0])?,
        });
    }
    Err(format!("unsupported runtime attachment `{text}`"))
}

fn split_top_level_assignment(text: &str) -> Option<(&str, &str)> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    for (index, ch) in text.char_indices() {
        if in_string {
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
            '"' => in_string = true,
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '=' if depth == 0 => return Some((&text[..index], &text[index + 1..])),
            _ => {}
        }
    }
    None
}

fn parse_assign_target(text: &str) -> Result<ParsedAssignTarget, String> {
    if text.starts_with("name(") && text.ends_with(')') {
        return Ok(ParsedAssignTarget::Name(
            strip_prefix_suffix(text, "name(", ")")?.to_string(),
        ));
    }
    if text.starts_with("member(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "member(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed runtime assign target `{text}`"));
        }
        return Ok(ParsedAssignTarget::Member {
            target: Box::new(parse_assign_target(&parts[0])?),
            member: parts[1].to_string(),
        });
    }
    if text.starts_with("index(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "index(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed runtime assign target `{text}`"));
        }
        return Ok(ParsedAssignTarget::Index {
            target: Box::new(parse_assign_target(&parts[0])?),
            index: parse_expr(&parts[1])?,
        });
    }
    Err(format!("unsupported runtime assign target `{text}`"))
}

pub(crate) fn parse_stmt(text: &str) -> Result<ParsedStmt, String> {
    let fields = parse_named_fields(strip_prefix_suffix(text, "stmt(", ")")?)?;
    let _forewords = parse_list(
        fields
            .get("forewords")
            .ok_or_else(|| format!("runtime stmt missing forewords in `{text}`"))?,
    )?;
    let cleanup_footers = parse_list(
        fields
            .get("cleanup_footers")
            .ok_or_else(|| format!("runtime stmt missing cleanup_footers in `{text}`"))?,
    )?
    .into_iter()
    .map(|row| parse_rollup_row(&row))
    .collect::<Result<Vec<_>, String>>()?;
    let core = fields
        .get("core")
        .ok_or_else(|| format!("runtime stmt missing core in `{text}`"))?;
    if core.starts_with("let(") && core.ends_with(')') {
        let let_fields = parse_named_fields(strip_prefix_suffix(core, "let(", ")")?)?;
        let mutable = match let_fields
            .get("mutable")
            .map(String::as_str)
            .ok_or_else(|| format!("runtime let missing mutable in `{text}`"))?
        {
            "true" => true,
            "false" => false,
            other => return Err(format!("invalid runtime let mutable `{other}`")),
        };
        return Ok(ParsedStmt::Let {
            binding_id: 0,
            mutable,
            name: let_fields
                .get("name")
                .ok_or_else(|| format!("runtime let missing name in `{text}`"))?
                .to_string(),
            value: parse_expr(
                let_fields
                    .get("value")
                    .ok_or_else(|| format!("runtime let missing value in `{text}`"))?,
            )?,
        });
    }
    if core.starts_with("expr(") && core.ends_with(')') {
        return Ok(ParsedStmt::Expr {
            expr: parse_expr(strip_prefix_suffix(core, "expr(", ")")?)?,
            cleanup_footers,
        });
    }
    if core.starts_with("return(") && core.ends_with(')') {
        let inner = strip_prefix_suffix(core, "return(", ")")?;
        return Ok(if inner == "none" {
            ParsedStmt::ReturnVoid
        } else {
            ParsedStmt::ReturnValue {
                value: parse_expr(inner)?,
            }
        });
    }
    if core.starts_with("if(") && core.ends_with(')') {
        let if_fields = parse_named_fields(strip_prefix_suffix(core, "if(", ")")?)?;
        return Ok(ParsedStmt::If {
            condition: parse_expr(
                if_fields
                    .get("cond")
                    .ok_or_else(|| format!("runtime if missing cond in `{text}`"))?,
            )?,
            then_branch: parse_list(
                if_fields
                    .get("then")
                    .ok_or_else(|| format!("runtime if missing then in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
            else_branch: parse_list(
                if_fields
                    .get("else")
                    .ok_or_else(|| format!("runtime if missing else in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
            availability: Vec::new(),
            cleanup_footers,
        });
    }
    if core.starts_with("while(") && core.ends_with(')') {
        let while_fields = parse_named_fields(strip_prefix_suffix(core, "while(", ")")?)?;
        return Ok(ParsedStmt::While {
            condition: parse_expr(
                while_fields
                    .get("cond")
                    .ok_or_else(|| format!("runtime while missing cond in `{text}`"))?,
            )?,
            body: parse_list(
                while_fields
                    .get("body")
                    .ok_or_else(|| format!("runtime while missing body in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
            availability: Vec::new(),
            cleanup_footers,
        });
    }
    if core.starts_with("for(") && core.ends_with(')') {
        let for_fields = parse_named_fields(strip_prefix_suffix(core, "for(", ")")?)?;
        return Ok(ParsedStmt::For {
            binding_id: 0,
            binding: for_fields
                .get("binding")
                .ok_or_else(|| format!("runtime for missing binding in `{text}`"))?
                .to_string(),
            iterable: parse_expr(
                for_fields
                    .get("iterable")
                    .ok_or_else(|| format!("runtime for missing iterable in `{text}`"))?,
            )?,
            body: parse_list(
                for_fields
                    .get("body")
                    .ok_or_else(|| format!("runtime for missing body in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
            availability: Vec::new(),
            cleanup_footers,
        });
    }
    if core.starts_with("defer(") && core.ends_with(')') {
        return Ok(ParsedStmt::Defer(parse_expr(strip_prefix_suffix(
            core, "defer(", ")",
        )?)?));
    }
    if core == "break" {
        return Ok(ParsedStmt::Break);
    }
    if core == "continue" {
        return Ok(ParsedStmt::Continue);
    }
    if core.starts_with("assign(") && core.ends_with(')') {
        let assign_fields = parse_named_fields(strip_prefix_suffix(core, "assign(", ")")?)?;
        return Ok(ParsedStmt::Assign {
            target: parse_assign_target(
                assign_fields
                    .get("target")
                    .ok_or_else(|| format!("runtime assign missing target in `{text}`"))?,
            )?,
            op: parse_assign_op(
                assign_fields
                    .get("op")
                    .ok_or_else(|| format!("runtime assign missing op in `{text}`"))?,
            )?,
            value: parse_expr(
                assign_fields
                    .get("value")
                    .ok_or_else(|| format!("runtime assign missing value in `{text}`"))?,
            )?,
        });
    }
    Err(format!("unsupported runtime statement `{core}`"))
}
