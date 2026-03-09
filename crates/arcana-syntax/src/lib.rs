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
pub struct SymbolDecl {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedModule {
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directives: Vec<ModuleDirective>,
    pub symbols: Vec<SymbolDecl>,
}

pub fn parse_module(source: &str) -> Result<ParsedModule, String> {
    let mut line_count = 0usize;
    let mut non_empty = 0usize;
    let mut directives = Vec::new();
    let mut symbols = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        line_count = idx + 1;
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
                        idx + 1,
                        column + 1
                    ));
                }
                _ => break,
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        non_empty += 1;
        if leading_spaces != 0 {
            continue;
        }

        let span = Span::new(idx + 1, 1);
        if let Some(directive) = parse_directive(trimmed, span)? {
            directives.push(directive);
            continue;
        }

        if let Some(symbol) = parse_symbol(trimmed, span) {
            symbols.push(symbol);
        }
    }
    Ok(ParsedModule {
        line_count: line_count.max(1),
        non_empty_line_count: non_empty,
        directives,
        symbols,
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
    let rest = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    for keyword in ["fn", "record", "enum", "trait", "behavior", "const"] {
        let Some(rest) = rest.strip_prefix(keyword) else {
            continue;
        };
        let Some(rest) = rest.strip_prefix(' ') else {
            continue;
        };
        let name = parse_symbol_name(rest)?;
        return Some(SymbolDecl { name, span });
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
    use super::{DirectiveKind, parse_module};

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
            "import std.io\nuse std.result.Result\nreexport types\nexport record Counter:\nfn main() -> Int:\n",
        )
        .expect("parse should pass");

        assert_eq!(parsed.directives.len(), 3);
        assert_eq!(parsed.directives[0].kind, DirectiveKind::Import);
        assert_eq!(parsed.directives[0].path, ["std", "io"]);
        assert_eq!(parsed.directives[1].kind, DirectiveKind::Use);
        assert_eq!(parsed.directives[1].path, ["std", "result", "Result"]);
        assert_eq!(parsed.symbols.len(), 2);
        assert_eq!(parsed.symbols[0].name, "Counter");
        assert_eq!(parsed.symbols[1].name, "main");
    }
}
