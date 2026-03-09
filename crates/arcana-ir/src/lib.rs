use arcana_hir::{HirModule, HirModuleSummary, HirPackageSummary};

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrPackage {
    pub package_name: String,
    pub modules: Vec<IrPackageModule>,
    pub dependency_edge_count: usize,
    pub exported_surface_rows: Vec<String>,
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
            }
        })
        .collect();
    IrPackage {
        package_name: package.package_name.clone(),
        modules,
        dependency_edge_count: package.dependency_edges.len(),
        exported_surface_rows: package.exported_surface_rows(),
    }
}

#[cfg(test)]
mod tests {
    use super::{IrModule, lower_hir, lower_package};
    use arcana_hir::{HirModule, build_package_summary, lower_module_text};

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
                    "export record Window:\n    title: Text\n",
                )
                .expect("nested module should lower"),
            ],
        );

        let ir = lower_package(&summary);
        assert_eq!(ir.package_name, "winspell");
        assert_eq!(ir.module_count(), 2);
        assert_eq!(ir.dependency_edge_count, 1);
        assert_eq!(
            ir.exported_surface_rows,
            vec![
                "module=winspell.window:export:record:Window".to_string(),
                "module=winspell:export:fn:open".to_string(),
                "module=winspell:reexport:winspell.window".to_string(),
            ]
        );
    }
}
