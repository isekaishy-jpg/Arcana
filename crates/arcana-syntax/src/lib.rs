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
pub struct ParsedModule {
    pub line_count: usize,
    pub non_empty_line_count: usize,
}

pub fn parse_module(source: &str) -> Result<ParsedModule, String> {
    let mut line_count = 0usize;
    let mut non_empty = 0usize;
    for (idx, line) in source.lines().enumerate() {
        line_count = idx + 1;
        for (column, ch) in line.chars().enumerate() {
            match ch {
                ' ' => continue,
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
        if !line.trim().is_empty() {
            non_empty += 1;
        }
    }
    Ok(ParsedModule {
        line_count: line_count.max(1),
        non_empty_line_count: non_empty,
    })
}

#[cfg(test)]
mod tests {
    use super::freeze::{FROZEN_AST_NODE_KINDS, FROZEN_TOKEN_KINDS};
    use super::parse_module;

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
}
