use arcana_hir::HirModule;
use arcana_syntax::parse_module;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckSummary {
    pub module_count: usize,
    pub non_empty_lines: usize,
}

pub fn check_sources<'a, I>(sources: I) -> Result<CheckSummary, String>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut summary = CheckSummary::default();
    for source in sources {
        let parsed = parse_module(source)?;
        summary.module_count += 1;
        summary.non_empty_lines += parsed.non_empty_line_count;
    }
    Ok(summary)
}

pub fn lower_to_hir(summary: &CheckSummary) -> HirModule {
    HirModule {
        symbol_count: summary.module_count,
        item_count: summary.non_empty_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::{check_sources, lower_to_hir};

    #[test]
    fn check_sources_counts_modules() {
        let summary = check_sources(["fn main() -> Int:\n    return 0\n"].iter().copied())
            .expect("check should pass");
        assert_eq!(summary.module_count, 1);
        assert!(summary.non_empty_lines >= 2);

        let hir = lower_to_hir(&summary);
        assert_eq!(hir.symbol_count, 1);
    }
}

