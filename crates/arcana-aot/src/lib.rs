use arcana_ir::{IrModule, IrPackage};

pub const AOT_PLACEHOLDER_FORMAT: &str = "aot-placeholder-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotArtifact {
    pub format: &'static str,
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotPackageModuleArtifact {
    pub module_id: String,
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotPackageArtifact {
    pub format: &'static str,
    pub package_name: String,
    pub module_count: usize,
    pub dependency_edge_count: usize,
    pub exported_surface_rows: Vec<String>,
    pub modules: Vec<AotPackageModuleArtifact>,
}

pub fn compile_module(module: &IrModule) -> AotArtifact {
    AotArtifact {
        format: AOT_PLACEHOLDER_FORMAT,
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

pub fn compile_package(package: &IrPackage) -> AotPackageArtifact {
    let modules = package
        .modules
        .iter()
        .map(|module| {
            let compiled = compile_module(&IrModule {
                symbol_count: module.symbol_count,
                item_count: module.item_count,
            });
            AotPackageModuleArtifact {
                module_id: module.module_id.clone(),
                symbol_count: compiled.symbol_count,
                item_count: compiled.item_count,
            }
        })
        .collect();
    AotPackageArtifact {
        format: AOT_PLACEHOLDER_FORMAT,
        package_name: package.package_name.clone(),
        module_count: package.module_count(),
        dependency_edge_count: package.dependency_edge_count,
        exported_surface_rows: package.exported_surface_rows.clone(),
        modules,
    }
}

#[cfg(test)]
mod tests {
    use super::{AOT_PLACEHOLDER_FORMAT, compile_module, compile_package};
    use arcana_ir::{IrModule, IrPackage, IrPackageModule};

    #[test]
    fn compile_module_emits_placeholder_artifact() {
        let artifact = compile_module(&IrModule {
            symbol_count: 1,
            item_count: 3,
        });
        assert_eq!(artifact.format, AOT_PLACEHOLDER_FORMAT);
    }

    #[test]
    fn compile_package_emits_placeholder_artifact() {
        let artifact = compile_package(&IrPackage {
            package_name: "winspell".to_string(),
            modules: vec![
                IrPackageModule {
                    module_id: "winspell".to_string(),
                    symbol_count: 1,
                    item_count: 3,
                },
                IrPackageModule {
                    module_id: "winspell.window".to_string(),
                    symbol_count: 2,
                    item_count: 5,
                },
            ],
            dependency_edge_count: 1,
            exported_surface_rows: vec!["module=winspell:export:fn:fn open() -> Int:".to_string()],
        });
        assert_eq!(artifact.format, AOT_PLACEHOLDER_FORMAT);
        assert_eq!(artifact.package_name, "winspell");
        assert_eq!(artifact.module_count, 2);
        assert_eq!(artifact.modules.len(), 2);
    }
}
