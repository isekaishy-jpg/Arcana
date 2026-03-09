use arcana_hir::HirModule;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IrModule {
    pub symbol_count: usize,
    pub item_count: usize,
}

pub fn lower_hir(module: &HirModule) -> IrModule {
    IrModule {
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

#[cfg(test)]
mod tests {
    use super::{IrModule, lower_hir};
    use arcana_hir::HirModule;

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
}
