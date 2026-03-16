use std::collections::BTreeMap;

use arcana_ir::{ExecBinaryOp, ExecExpr, ExecStmt};

use crate::artifact::AotRoutineArtifact;
use crate::emit::AotEmitTarget;
use crate::native_abi::{NativeAbiType, NativeExport};
use crate::native_plan::{NativeLaunchPlan, NativePackagePlan};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeDirectExpr {
    Int(i64),
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
    Param(String),
    Pair {
        left: Box<NativeDirectExpr>,
        right: Box<NativeDirectExpr>,
    },
    StringConcat {
        left: Box<NativeDirectExpr>,
        right: Box<NativeDirectExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeRoutineLowering {
    Direct(NativeDirectExpr),
    RuntimeDispatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeExportLowering {
    pub export: NativeExport,
    pub lowering: NativeRoutineLowering,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeLaunchLowering {
    Executable {
        main_routine_key: String,
        lowering: NativeRoutineLowering,
    },
    DynamicLibrary {
        exports: Vec<NativeExportLowering>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeLoweringPlan {
    pub target: AotEmitTarget,
    pub launch: NativeLaunchLowering,
}

pub fn build_native_lowering_plan(plan: &NativePackagePlan) -> Result<NativeLoweringPlan, String> {
    let launch = match &plan.launch {
        NativeLaunchPlan::Executable { main_routine_key } => NativeLaunchLowering::Executable {
            main_routine_key: main_routine_key.clone(),
            lowering: lower_routine(
                find_routine(&plan.artifact.routines, main_routine_key)?,
                &BTreeMap::new(),
                &NativeAbiType::Int,
            ),
        },
        NativeLaunchPlan::DynamicLibrary { exports } => NativeLaunchLowering::DynamicLibrary {
            exports: exports
                .iter()
                .cloned()
                .map(|export| {
                    let params = export
                        .params
                        .iter()
                        .map(|param| (param.name.clone(), param.ty.clone()))
                        .collect::<BTreeMap<_, _>>();
                    let lowering = lower_routine(
                        find_routine(&plan.artifact.routines, &export.routine_key)?,
                        &params,
                        &export.return_type,
                    );
                    Ok(NativeExportLowering { export, lowering })
                })
                .collect::<Result<Vec<_>, String>>()?,
        },
    };
    Ok(NativeLoweringPlan {
        target: plan.target,
        launch,
    })
}

fn find_routine<'a>(
    routines: &'a [AotRoutineArtifact],
    routine_key: &str,
) -> Result<&'a AotRoutineArtifact, String> {
    routines
        .iter()
        .find(|routine| routine.routine_key == routine_key)
        .ok_or_else(|| format!("native lowering could not resolve routine `{routine_key}`"))
}

fn lower_routine(
    routine: &AotRoutineArtifact,
    param_types: &BTreeMap<String, NativeAbiType>,
    return_type: &NativeAbiType,
) -> NativeRoutineLowering {
    if routine.intrinsic_impl.is_some()
        || !routine.foreword_rows.is_empty()
        || !routine.rollups.is_empty()
    {
        return NativeRoutineLowering::RuntimeDispatch;
    }
    let [ExecStmt::ReturnValue { value }] = routine.statements.as_slice() else {
        return NativeRoutineLowering::RuntimeDispatch;
    };
    match lower_expr(value, param_types, return_type) {
        Some(expr) => NativeRoutineLowering::Direct(expr),
        None => NativeRoutineLowering::RuntimeDispatch,
    }
}

fn lower_expr(
    expr: &ExecExpr,
    param_types: &BTreeMap<String, NativeAbiType>,
    expected: &NativeAbiType,
) -> Option<NativeDirectExpr> {
    match (expected, expr) {
        (NativeAbiType::Int, ExecExpr::Int(value)) => Some(NativeDirectExpr::Int(*value)),
        (NativeAbiType::Bool, ExecExpr::Bool(value)) => Some(NativeDirectExpr::Bool(*value)),
        (NativeAbiType::Str, ExecExpr::Str(value)) => Some(NativeDirectExpr::Str(value.clone())),
        (NativeAbiType::Bytes, ExecExpr::Collection { items }) => items
            .iter()
            .map(|item| match item {
                ExecExpr::Int(value) => u8::try_from(*value).ok(),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(NativeDirectExpr::Bytes),
        (NativeAbiType::Pair(left_ty, right_ty), ExecExpr::Pair { left, right }) => {
            Some(NativeDirectExpr::Pair {
                left: Box::new(lower_expr(left, param_types, left_ty)?),
                right: Box::new(lower_expr(right, param_types, right_ty)?),
            })
        }
        (NativeAbiType::Str, ExecExpr::Binary { left, op, right }) if *op == ExecBinaryOp::Add => {
            Some(NativeDirectExpr::StringConcat {
                left: Box::new(lower_expr(left, param_types, &NativeAbiType::Str)?),
                right: Box::new(lower_expr(right, param_types, &NativeAbiType::Str)?),
            })
        }
        (_, ExecExpr::Path(segments)) if segments.len() == 1 => {
            let name = segments[0].clone();
            param_types
                .get(&name)
                .filter(|ty| *ty == expected)
                .map(|_| NativeDirectExpr::Param(name))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeDirectExpr, NativeLaunchLowering, NativeRoutineLowering, build_native_lowering_plan,
    };
    use crate::emit::{AotEmitContext, AotEmitTarget};
    use crate::native_plan::build_native_package_plan;
    use arcana_ir::{ExecExpr, ExecStmt, IrEntrypoint, IrPackage, IrPackageModule, IrRoutine};

    fn base_package() -> IrPackage {
        IrPackage {
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            modules: vec![IrPackageModule {
                module_id: "core".to_string(),
                symbol_count: 1,
                item_count: 1,
                line_count: 1,
                non_empty_line_count: 1,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
            dependency_edge_count: 0,
            dependency_rows: Vec::new(),
            exported_surface_rows: Vec::new(),
            runtime_requirements: Vec::new(),
            entrypoints: Vec::new(),
            routines: Vec::new(),
        }
    }

    #[test]
    fn lowering_marks_simple_main_as_direct() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            module_id: "core".to_string(),
            routine_key: "core#fn-0".to_string(),
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
                value: ExecExpr::Int(9),
            }],
        });

        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &AotEmitContext {
                root_artifact_file_name: Some("app.exe".to_string()),
            },
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-0".to_string(),
                lowering: NativeRoutineLowering::Direct(NativeDirectExpr::Int(9)),
            }
        );
    }

    #[test]
    fn lowering_splits_direct_exports_from_runtime_fallbacks() {
        let mut package = base_package();
        package.routines.extend([
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "answer".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn answer() -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(11),
                }],
            },
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "greet".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=read:name=name:ty=Str".to_string()],
                signature_row: "fn greet(read name: Str) -> Str:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Binary {
                        left: Box::new(ExecExpr::Str("hi ".to_string())),
                        op: arcana_ir::ExecBinaryOp::Add,
                        right: Box::new(ExecExpr::Path(vec!["name".to_string()])),
                    },
                }],
            },
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-2".to_string(),
                symbol_name: "prefix".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=read:name=bytes:ty=Array[Int]".to_string()],
                signature_row: "fn prefix(read bytes: Array[Int]) -> Array[Int]:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["std".to_string()])),
                        args: Vec::new(),
                        qualifier_kind: arcana_ir::ExecPhraseQualifierKind::Call,
                        qualifier: "bytes".to_string(),
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-3".to_string(),
                symbol_name: "echo_pair".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=read:name=pair:ty=Pair[Str, Int]".to_string()],
                signature_row: "fn echo_pair(read pair: Pair[Str, Int]) -> Pair[Str, Int]:"
                    .to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["pair".to_string()]),
                }],
            },
        ]);

        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &AotEmitContext {
                root_artifact_file_name: Some("lib.dll".to_string()),
            },
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let NativeLaunchLowering::DynamicLibrary { exports } = lowering_plan.launch else {
            panic!("expected dynamic-library lowering");
        };
        assert_eq!(exports.len(), 4);
        assert!(matches!(
            exports[0].lowering,
            NativeRoutineLowering::Direct(NativeDirectExpr::Int(11))
        ));
        assert!(matches!(
            exports[1].lowering,
            NativeRoutineLowering::Direct(NativeDirectExpr::StringConcat { .. })
        ));
        assert_eq!(exports[2].lowering, NativeRoutineLowering::RuntimeDispatch);
        assert!(matches!(
            exports[3].lowering,
            NativeRoutineLowering::Direct(NativeDirectExpr::Param(ref name)) if name == "pair"
        ));
    }
}
