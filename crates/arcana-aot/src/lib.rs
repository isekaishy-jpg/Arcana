use arcana_ir::{
    ExecPageRollup, ExecStmt, IrEntrypoint, IrModule, IrPackage, IrPackageModule, IrRoutine,
};
use serde::{Deserialize, Serialize};

pub const AOT_INTERNAL_FORMAT: &str = "arcana-aot-v4";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotArtifact {
    pub format: String,
    pub symbol_count: usize,
    pub item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotPackageModuleArtifact {
    pub module_id: String,
    pub symbol_count: usize,
    pub item_count: usize,
    pub line_count: usize,
    pub non_empty_line_count: usize,
    pub directive_rows: Vec<String>,
    pub lang_item_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotEntrypointArtifact {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotRoutineArtifact {
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_param_rows: Vec<String>,
    pub behavior_attr_rows: Vec<String>,
    pub param_rows: Vec<String>,
    pub signature_row: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intrinsic_impl: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_target_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impl_trait_path: Option<Vec<String>>,
    pub foreword_rows: Vec<String>,
    pub rollups: Vec<ExecPageRollup>,
    pub statements: Vec<ExecStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AotPackageArtifact {
    pub format: String,
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub module_count: usize,
    pub dependency_edge_count: usize,
    pub dependency_rows: Vec<String>,
    pub exported_surface_rows: Vec<String>,
    pub runtime_requirements: Vec<String>,
    pub entrypoints: Vec<AotEntrypointArtifact>,
    pub routines: Vec<AotRoutineArtifact>,
    pub modules: Vec<AotPackageModuleArtifact>,
}

pub fn compile_module(module: &IrModule) -> AotArtifact {
    AotArtifact {
        format: AOT_INTERNAL_FORMAT.to_string(),
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    }
}

fn compile_module_artifact(module: &IrPackageModule) -> AotPackageModuleArtifact {
    let compiled = compile_module(&IrModule {
        symbol_count: module.symbol_count,
        item_count: module.item_count,
    });
    AotPackageModuleArtifact {
        module_id: module.module_id.clone(),
        symbol_count: compiled.symbol_count,
        item_count: compiled.item_count,
        line_count: module.line_count,
        non_empty_line_count: module.non_empty_line_count,
        directive_rows: module.directive_rows.clone(),
        lang_item_rows: module.lang_item_rows.clone(),
        exported_surface_rows: module.exported_surface_rows.clone(),
    }
}

fn compile_entrypoint(entrypoint: &IrEntrypoint) -> AotEntrypointArtifact {
    AotEntrypointArtifact {
        module_id: entrypoint.module_id.clone(),
        symbol_name: entrypoint.symbol_name.clone(),
        symbol_kind: entrypoint.symbol_kind.clone(),
        is_async: entrypoint.is_async,
        exported: entrypoint.exported,
    }
}

fn compile_routine(routine: &IrRoutine) -> AotRoutineArtifact {
    AotRoutineArtifact {
        module_id: routine.module_id.clone(),
        routine_key: routine.routine_key.clone(),
        symbol_name: routine.symbol_name.clone(),
        symbol_kind: routine.symbol_kind.clone(),
        exported: routine.exported,
        is_async: routine.is_async,
        type_param_rows: routine.type_param_rows.clone(),
        behavior_attr_rows: routine.behavior_attr_rows.clone(),
        param_rows: routine.param_rows.clone(),
        signature_row: routine.signature_row.clone(),
        intrinsic_impl: routine.intrinsic_impl.clone(),
        impl_target_type: routine.impl_target_type.clone(),
        impl_trait_path: routine.impl_trait_path.clone(),
        foreword_rows: routine.foreword_rows.clone(),
        rollups: routine.rollups.clone(),
        statements: routine.statements.clone(),
    }
}

pub fn compile_package(package: &IrPackage) -> AotPackageArtifact {
    AotPackageArtifact {
        format: AOT_INTERNAL_FORMAT.to_string(),
        package_name: package.package_name.clone(),
        root_module_id: package.root_module_id.clone(),
        direct_deps: package.direct_deps.clone(),
        module_count: package.module_count(),
        dependency_edge_count: package.dependency_edge_count,
        dependency_rows: package.dependency_rows.clone(),
        exported_surface_rows: package.exported_surface_rows.clone(),
        runtime_requirements: package.runtime_requirements.clone(),
        entrypoints: package.entrypoints.iter().map(compile_entrypoint).collect(),
        routines: package.routines.iter().map(compile_routine).collect(),
        modules: package
            .modules
            .iter()
            .map(compile_module_artifact)
            .collect(),
    }
}

pub fn render_package_artifact(artifact: &AotPackageArtifact) -> String {
    toml::to_string(artifact).expect("backend artifact should serialize")
}

pub fn parse_package_artifact(text: &str) -> Result<AotPackageArtifact, String> {
    let artifact = toml::from_str::<AotPackageArtifact>(text)
        .map_err(|err| format!("failed to parse backend artifact: {err}"))?;
    if artifact.format != AOT_INTERNAL_FORMAT {
        return Err(format!(
            "unsupported backend artifact format `{}`; expected `{AOT_INTERNAL_FORMAT}`",
            artifact.format
        ));
    }
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::{
        AOT_INTERNAL_FORMAT, AotEntrypointArtifact, AotPackageArtifact, AotPackageModuleArtifact,
        AotRoutineArtifact, compile_module, compile_package, parse_package_artifact,
        render_package_artifact,
    };
    use arcana_ir::{
        ExecExpr, ExecPageRollup, ExecPhraseQualifierKind, ExecStmt, IrEntrypoint, IrModule,
        IrPackage, IrPackageModule, IrRoutine,
    };

    #[test]
    fn compile_module_emits_internal_artifact() {
        let artifact = compile_module(&IrModule {
            symbol_count: 1,
            item_count: 3,
        });
        assert_eq!(artifact.format, AOT_INTERNAL_FORMAT);
    }

    #[test]
    fn compile_package_emits_backend_contract_artifact() {
        let artifact = compile_package(&IrPackage {
            package_name: "winspell".to_string(),
            root_module_id: "winspell".to_string(),
            direct_deps: vec!["std".to_string()],
            modules: vec![
                IrPackageModule {
                    module_id: "winspell".to_string(),
                    symbol_count: 1,
                    item_count: 3,
                    line_count: 4,
                    non_empty_line_count: 3,
                    directive_rows: vec!["module=winspell:reexport:winspell.window:".to_string()],
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: vec!["export:fn:fn open() -> Int:".to_string()],
                },
                IrPackageModule {
                    module_id: "winspell.window".to_string(),
                    symbol_count: 2,
                    item_count: 5,
                    line_count: 6,
                    non_empty_line_count: 5,
                    directive_rows: vec!["module=winspell.window:import:std.canvas:".to_string()],
                    lang_item_rows: Vec::new(),
                    exported_surface_rows: Vec::new(),
                },
            ],
            dependency_edge_count: 2,
            dependency_rows: vec![
                "source=winspell:reexport:winspell.window:".to_string(),
                "source=winspell.window:import:std.canvas:".to_string(),
            ],
            exported_surface_rows: vec!["module=winspell:export:fn:fn open() -> Int:".to_string()],
            runtime_requirements: vec!["std.canvas".to_string()],
            entrypoints: vec![IrEntrypoint {
                module_id: "winspell".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![IrRoutine {
                module_id: "winspell".to_string(),
                routine_key: "winspell#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            }],
        });
        assert_eq!(artifact.format, AOT_INTERNAL_FORMAT);
        assert_eq!(artifact.module_count, 2);
        assert_eq!(artifact.modules[0].module_id, "winspell");
    }

    #[test]
    fn package_artifact_roundtrips() {
        let artifact = AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT.to_string(),
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: vec!["std".to_string()],
            module_count: 1,
            dependency_edge_count: 1,
            dependency_rows: vec!["source=tool:import:std.io:".to_string()],
            exported_surface_rows: vec!["module=tool:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: vec!["std.io".to_string()],
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "tool".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "tool".to_string(),
                routine_key: "tool#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=:name=x:ty=Int".to_string()],
                signature_row: "fn main(x: Int) -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: vec!["test()".to_string()],
                rollups: vec![ExecPageRollup {
                    kind: "cleanup".to_string(),
                    subject: "page".to_string(),
                    handler_path: vec!["handler".to_string()],
                }],
                statements: vec![ExecStmt::Expr {
                    expr: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["x".to_string()])),
                        args: Vec::new(),
                        qualifier_kind: ExecPhraseQualifierKind::BareMethod,
                        qualifier: "is_ok".to_string(),
                        resolved_callable: Some(vec![
                            "std".to_string(),
                            "result".to_string(),
                            "is_ok".to_string(),
                        ]),
                        resolved_routine: Some("std.result#impl-0-method-0".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                    rollups: Vec::new(),
                }],
            }],
            modules: vec![AotPackageModuleArtifact {
                module_id: "tool".to_string(),
                symbol_count: 1,
                item_count: 2,
                line_count: 3,
                non_empty_line_count: 2,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["export:fn:fn main() -> Int:".to_string()],
            }],
        };
        let rendered = render_package_artifact(&artifact);
        let parsed = parse_package_artifact(&rendered).expect("artifact should roundtrip");
        assert_eq!(parsed, artifact);
    }
}
