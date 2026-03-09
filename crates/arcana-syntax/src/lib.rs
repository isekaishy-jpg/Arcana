pub mod freeze;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectiveKind {
    Import,
    Use,
    Reexport,
}

impl DirectiveKind {
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Use => "use",
            Self::Reexport => "reexport",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleDirective {
    pub kind: DirectiveKind,
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,
    Record,
    Enum,
    Trait,
    Behavior,
    Const,
}

impl SymbolKind {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamMode {
    Read,
    Edit,
    Take,
}

impl ParamMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Take => "take",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParamDecl {
    pub mode: Option<ParamMode>,
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolDecl {
    pub name: String,
    pub kind: SymbolKind,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub where_clause: Option<String>,
    pub params: Vec<ParamDecl>,
    pub return_type: Option<String>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImplDecl {
    pub trait_path: Option<String>,
    pub target_type: String,
    pub body_entries: Vec<String>,
    pub surface_text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedModule {
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directives: Vec<ModuleDirective>,
    pub symbols: Vec<SymbolDecl>,
    pub impls: Vec<ImplDecl>,
}

pub fn parse_module(source: &str) -> Result<ParsedModule, String> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut line_count = 0usize;
    let mut non_empty = 0usize;
    let mut directives = Vec::new();
    let mut symbols = Vec::new();
    let mut impls = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        line_count = idx + 1;
        let analysis = analyze_line(line, idx)?;
        if !analysis.counts_as_non_empty() {
            continue;
        }

        non_empty += 1;
        if analysis.leading_spaces != 0 {
            continue;
        }

        let span = Span::new(idx + 1, 1);
        if let Some(directive) = parse_directive(analysis.trimmed, span)? {
            directives.push(directive);
            continue;
        }

        if let Some(impl_decl) = parse_impl_decl(&lines, idx, analysis.trimmed, span)? {
            impls.push(impl_decl);
            continue;
        }

        if let Some(mut symbol) = parse_symbol(analysis.trimmed, span) {
            symbol.surface_text = collect_symbol_surface(&lines, idx, &symbol.kind)?;
            symbols.push(symbol);
        }
    }
    Ok(ParsedModule {
        line_count: line_count.max(1),
        non_empty_line_count: non_empty,
        directives,
        symbols,
        impls,
    })
}

struct AnalyzedLine<'a> {
    trimmed: &'a str,
    leading_spaces: usize,
}

impl AnalyzedLine<'_> {
    fn counts_as_non_empty(&self) -> bool {
        !self.trimmed.is_empty() && !self.trimmed.starts_with('#')
    }
}

fn analyze_line<'a>(line: &'a str, line_index: usize) -> Result<AnalyzedLine<'a>, String> {
    let mut leading_spaces = 0usize;
    for (column, ch) in line.chars().enumerate() {
        match ch {
            ' ' => {
                leading_spaces += 1;
                continue;
            }
            '\t' => {
                return Err(format!(
                    "{}:{}: tabs are not allowed in indentation",
                    line_index + 1,
                    column + 1
                ));
            }
            _ => break,
        }
    }

    Ok(AnalyzedLine {
        trimmed: line.trim(),
        leading_spaces,
    })
}

fn parse_directive(trimmed: &str, span: Span) -> Result<Option<ModuleDirective>, String> {
    let (kind, rest) = if let Some(rest) = trimmed.strip_prefix("import ") {
        (DirectiveKind::Import, rest)
    } else if let Some(rest) = trimmed.strip_prefix("use ") {
        (DirectiveKind::Use, rest)
    } else if let Some(rest) = trimmed.strip_prefix("reexport ") {
        (DirectiveKind::Reexport, rest)
    } else {
        return Ok(None);
    };

    let (path_text, alias) = match rest.split_once(" as ") {
        Some((path, alias)) => (path, Some(alias)),
        None => (rest, None),
    };
    let path = parse_path(path_text).map_err(|detail| {
        format!(
            "{}:{}: malformed {} directive: {}",
            span.line,
            span.column,
            kind.keyword(),
            detail
        )
    })?;
    let alias = alias
        .map(str::trim)
        .filter(|alias| !alias.is_empty())
        .map(|alias| {
            if is_identifier(alias) {
                Ok(alias.to_string())
            } else {
                Err(format!(
                    "{}:{}: malformed {} directive: invalid alias `{}`",
                    span.line,
                    span.column,
                    kind.keyword(),
                    alias
                ))
            }
        })
        .transpose()?;

    Ok(Some(ModuleDirective {
        kind,
        path,
        alias,
        span,
    }))
}

fn parse_path(path: &str) -> Result<Vec<String>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("missing path".to_string());
    }

    let segments = trimmed
        .split('.')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(format!("invalid path `{trimmed}`"));
    }
    for segment in &segments {
        if !is_identifier(segment) {
            return Err(format!("invalid path segment `{segment}`"));
        }
    }
    Ok(segments)
}

fn parse_symbol(trimmed: &str, span: Span) -> Option<SymbolDecl> {
    let exported = trimmed.starts_with("export ");
    let rest = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (is_async, rest) = if let Some(rest) = rest.strip_prefix("async ") {
        (true, rest)
    } else {
        (false, rest)
    };
    for (keyword, kind) in [
        ("fn", SymbolKind::Fn),
        ("record", SymbolKind::Record),
        ("enum", SymbolKind::Enum),
        ("trait", SymbolKind::Trait),
        ("behavior", SymbolKind::Behavior),
        ("const", SymbolKind::Const),
    ] {
        let Some(rest) = rest.strip_prefix(keyword) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(' ') else {
            continue;
        };
        let signature = parse_symbol_signature(kind.clone(), rest)?;
        return Some(SymbolDecl {
            name: signature.name,
            kind,
            exported,
            is_async,
            type_params: signature.type_params,
            where_clause: signature.where_clause,
            params: signature.params,
            return_type: signature.return_type,
            surface_text: trimmed.to_string(),
            span,
        });
    }
    None
}

fn collect_symbol_surface(
    lines: &[&str],
    start_index: usize,
    kind: &SymbolKind,
) -> Result<String, String> {
    let line = analyze_line(lines[start_index], start_index)?;
    let mut surface_lines = vec![
        line.trimmed
            .strip_prefix("export ")
            .unwrap_or(line.trimmed)
            .to_string(),
    ];

    if matches!(kind, SymbolKind::Fn | SymbolKind::Const) {
        return Ok(surface_lines.join("\n"));
    }

    surface_lines.extend(collect_indented_entries(lines, start_index)?);

    Ok(surface_lines.join("\n"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedSymbolSignature {
    name: String,
    type_params: Vec<String>,
    where_clause: Option<String>,
    params: Vec<ParamDecl>,
    return_type: Option<String>,
}

fn parse_symbol_signature(kind: SymbolKind, rest: &str) -> Option<ParsedSymbolSignature> {
    let rest = rest.trim();
    let header = rest.strip_suffix(':').unwrap_or(rest).trim();
    let name = parse_symbol_name(header)?;
    let after_name = &header[name.len()..];
    let (type_params, where_clause, params, return_type) = match kind {
        SymbolKind::Fn => parse_function_signature_tail(after_name)?,
        SymbolKind::Record | SymbolKind::Enum | SymbolKind::Trait | SymbolKind::Behavior => {
            parse_named_type_tail(after_name)?
        }
        SymbolKind::Const => parse_const_signature_tail(after_name),
    };

    Some(ParsedSymbolSignature {
        name,
        type_params,
        where_clause,
        params,
        return_type,
    })
}

fn parse_function_signature_tail(
    tail: &str,
) -> Option<(Vec<String>, Option<String>, Vec<ParamDecl>, Option<String>)> {
    let tail = tail.trim();
    let (type_params, where_clause, remainder) = parse_type_params_and_where(tail)?;
    let remainder = remainder.trim();
    let open_idx = remainder.find('(')?;
    let close_idx = find_matching_delim(remainder, open_idx, '(', ')')?;
    let params = parse_param_list(&remainder[open_idx + 1..close_idx]).ok()?;
    let after_params = remainder[close_idx + 1..].trim();
    let return_type = after_params
        .strip_prefix("->")
        .map(|ty| ty.trim().to_string())
        .filter(|ty| !ty.is_empty());
    Some((type_params, where_clause, params, return_type))
}

fn parse_named_type_tail(tail: &str) -> Option<(Vec<String>, Option<String>, Vec<ParamDecl>, Option<String>)> {
    let (type_params, where_clause, remainder) = parse_type_params_and_where(tail.trim())?;
    if !remainder.trim().is_empty() {
        return None;
    }
    Some((type_params, where_clause, Vec::new(), None))
}

fn parse_const_signature_tail(
    tail: &str,
) -> (Vec<String>, Option<String>, Vec<ParamDecl>, Option<String>) {
    let return_type = tail
        .trim()
        .strip_prefix(':')
        .map(|ty| ty.trim().to_string())
        .filter(|ty| !ty.is_empty());
    (Vec::new(), None, Vec::new(), return_type)
}

fn parse_type_params_and_where(tail: &str) -> Option<(Vec<String>, Option<String>, &str)> {
    let tail = tail.trim();
    let Some('[') = tail.chars().next() else {
        return Some((Vec::new(), None, tail));
    };
    let close_idx = find_matching_delim(tail, 0, '[', ']')?;
    let inside = &tail[1..close_idx];
    let mut type_params = Vec::new();
    let mut where_clause = None;
    for part in split_top_level(inside, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(clause) = part.strip_prefix("where ") {
            where_clause = Some(clause.trim().to_string());
        } else {
            type_params.push(part.to_string());
        }
    }
    Some((type_params, where_clause, &tail[close_idx + 1..]))
}

fn parse_param_list(source: &str) -> Result<Vec<ParamDecl>, String> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(Vec::new());
    }

    let mut params = Vec::new();
    for part in split_top_level(source, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let (mode, rest) = if let Some(rest) = part.strip_prefix("read ") {
            (Some(ParamMode::Read), rest)
        } else if let Some(rest) = part.strip_prefix("edit ") {
            (Some(ParamMode::Edit), rest)
        } else if let Some(rest) = part.strip_prefix("take ") {
            (Some(ParamMode::Take), rest)
        } else {
            (None, part)
        };

        let (name, ty) = rest
            .split_once(':')
            .ok_or_else(|| format!("malformed parameter `{part}`"))?;
        let name = name.trim();
        let ty = ty.trim();
        if !is_identifier(name) || ty.is_empty() {
            return Err(format!("malformed parameter `{part}`"));
        }
        params.push(ParamDecl {
            mode,
            name: name.to_string(),
            ty: ty.to_string(),
        });
    }

    Ok(params)
}

fn parse_impl_decl(
    lines: &[&str],
    start_index: usize,
    trimmed: &str,
    span: Span,
) -> Result<Option<ImplDecl>, String> {
    let Some(rest) = trimmed.strip_prefix("impl ") else {
        return Ok(None);
    };
    let header = rest.strip_suffix(':').unwrap_or(rest).trim();
    let (trait_path, target_type) = match header.rsplit_once(" for ") {
        Some((trait_path, target_type)) => (
            Some(trait_path.trim().to_string()),
            target_type.trim().to_string(),
        ),
        None => (None, header.to_string()),
    };
    if target_type.is_empty() {
        return Err(format!("{}:{}: malformed impl declaration", span.line, span.column));
    }
    let body_entries = collect_indented_entries(lines, start_index)?;
    let mut surface_lines = vec![trimmed.to_string()];
    surface_lines.extend(body_entries.iter().cloned());
    Ok(Some(ImplDecl {
        trait_path,
        target_type,
        body_entries,
        surface_text: surface_lines.join("\n"),
        span,
    }))
}

fn collect_indented_entries(lines: &[&str], start_index: usize) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    let mut body_indent = None;
    let mut index = start_index + 1;
    while index < lines.len() {
        let analysis = analyze_line(lines[index], index)?;
        if !analysis.counts_as_non_empty() {
            index += 1;
            continue;
        }
        if analysis.leading_spaces == 0 {
            break;
        }
        let indent = *body_indent.get_or_insert(analysis.leading_spaces);
        if analysis.leading_spaces < indent {
            break;
        }
        if analysis.leading_spaces == indent {
            entries.push(analysis.trimmed.to_string());
        }
        index += 1;
    }
    Ok(entries)
}

fn split_top_level(source: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    let mut depth_brace = 0usize;
    let mut start = 0usize;

    for (idx, ch) in source.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren = depth_paren.saturating_sub(1),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = depth_bracket.saturating_sub(1),
            '{' => depth_brace += 1,
            '}' => depth_brace = depth_brace.saturating_sub(1),
            _ => {}
        }

        if ch == separator && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
            parts.push(&source[start..idx]);
            start = idx + ch.len_utf8();
        }
    }

    parts.push(&source[start..]);
    parts
}

fn find_matching_delim(source: &str, open_idx: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in source.char_indices().skip_while(|(idx, _)| *idx < open_idx) {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn parse_symbol_name(rest: &str) -> Option<String> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !is_identifier_start(first) {
        return None;
    }

    let mut name = String::new();
    name.push(first);
    for ch in chars {
        if !is_identifier_continue(ch) {
            break;
        }
        name.push(ch);
    }
    Some(name)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_identifier_start(first) {
        return false;
    }
    chars.all(is_identifier_continue)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::freeze::{FROZEN_AST_NODE_KINDS, FROZEN_TOKEN_KINDS};
    use super::{DirectiveKind, ParamMode, parse_module};

    #[test]
    fn frozen_lists_are_unique() {
        let mut tokens = FROZEN_TOKEN_KINDS.to_vec();
        tokens.sort_unstable();
        tokens.dedup();
        assert_eq!(tokens.len(), FROZEN_TOKEN_KINDS.len());

        let mut nodes = FROZEN_AST_NODE_KINDS.to_vec();
        nodes.sort_unstable();
        nodes.dedup();
        assert_eq!(nodes.len(), FROZEN_AST_NODE_KINDS.len());
    }

    #[test]
    fn parse_module_rejects_tabs() {
        let err = parse_module("fn main()\n\treturn 0\n").expect_err("expected tab rejection");
        assert!(err.contains("tabs are not allowed"));
    }

    #[test]
    fn parse_module_collects_directives_and_symbols() {
        let parsed = parse_module(
            "import std.io\nuse std.result.Result\nreexport types\nexport record Counter:\n    value: Int\nfn main() -> Int:\n",
        )
        .expect("parse should pass");

        assert_eq!(parsed.directives.len(), 3);
        assert_eq!(parsed.directives[0].kind, DirectiveKind::Import);
        assert_eq!(parsed.directives[0].path, ["std", "io"]);
        assert_eq!(parsed.directives[1].kind, DirectiveKind::Use);
        assert_eq!(parsed.directives[1].path, ["std", "result", "Result"]);
        assert_eq!(parsed.symbols.len(), 2);
        assert_eq!(parsed.symbols[0].name, "Counter");
        assert_eq!(parsed.symbols[0].kind.as_str(), "record");
        assert!(parsed.symbols[0].exported);
        assert_eq!(parsed.symbols[0].surface_text, "record Counter:\nvalue: Int");
        assert_eq!(parsed.symbols[0].type_params, Vec::<String>::new());
        assert_eq!(parsed.symbols[1].name, "main");
        assert_eq!(parsed.symbols[1].kind.as_str(), "fn");
        assert!(!parsed.symbols[1].exported);
        assert_eq!(parsed.symbols[1].surface_text, "fn main() -> Int:");
        assert_eq!(parsed.symbols[1].return_type, Some("Int".to_string()));
    }

    #[test]
    fn parse_module_collects_async_functions_and_impls() {
        let parsed = parse_module(
            "async fn worker[T, where std.iter.Iterator[T]](read it: T, count: Int) -> Int:\n    return count\nimpl std.iter.Iterator[T] for RangeIter:\n    type Item = Int\n    fn next(edit self: RangeIter) -> (Bool, Int):\n        return (false, 0)\n",
        )
        .expect("parse should pass");

        assert_eq!(parsed.symbols.len(), 1);
        let worker = &parsed.symbols[0];
        assert!(worker.is_async);
        assert_eq!(worker.type_params, vec!["T".to_string()]);
        assert_eq!(worker.where_clause, Some("std.iter.Iterator[T]".to_string()));
        assert_eq!(worker.params.len(), 2);
        assert_eq!(worker.params[0].mode, Some(ParamMode::Read));
        assert_eq!(worker.params[0].name, "it");
        assert_eq!(worker.params[0].ty, "T");
        assert_eq!(worker.params[1].mode, None);
        assert_eq!(worker.return_type, Some("Int".to_string()));

        assert_eq!(parsed.impls.len(), 1);
        let impl_decl = &parsed.impls[0];
        assert_eq!(impl_decl.trait_path, Some("std.iter.Iterator[T]".to_string()));
        assert_eq!(impl_decl.target_type, "RangeIter");
        assert_eq!(impl_decl.body_entries.len(), 2);
        assert!(impl_decl.body_entries[0].starts_with("type Item"));
    }
}
