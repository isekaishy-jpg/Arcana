use arcana_ir::IrModule;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AotArtifact {
    pub format: &'static str,
    pub symbol_count: usize,
    pub item_count: usize,
}

pub fn compile_module(module: &IrModule) -> AotArtifact {
    AotArtifact {
        format: "aot-placeholder-v1",
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

#[cfg(test)]
mod tests {
    use super::compile_module;
    use arcana_ir::IrModule;

    #[test]
    fn compile_module_emits_placeholder_artifact() {
        let artifact = compile_module(&IrModule {
            symbol_count: 1,
            item_count: 3,
        });
        assert_eq!(artifact.format, "aot-placeholder-v1");
    }
}
