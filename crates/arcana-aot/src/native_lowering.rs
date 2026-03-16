use std::collections::BTreeMap;

use arcana_ir::{ExecBinaryOp, ExecExpr, ExecPhraseQualifierKind, ExecStmt};

use crate::artifact::AotRoutineArtifact;
use crate::emit::AotEmitTarget;
use crate::native_abi::{
    NativeAbiParam, NativeAbiType, NativeExport, NativeRoutineSignature,
    parse_native_routine_signature,
};
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
    Call {
        routine_key: String,
        args: Vec<NativeDirectExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeDirectRoutine {
    pub routine_key: String,
    pub params: Vec<NativeAbiParam>,
    pub return_type: NativeAbiType,
    pub body: NativeDirectExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeRoutineLowering {
    Direct { routine_key: String },
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
    pub direct_routines: Vec<NativeDirectRoutine>,
    pub launch: NativeLaunchLowering,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoweringState {
    InProgress,
    Direct,
    RuntimeDispatch,
}

struct NativeLoweringBuilder<'a> {
    routines_by_key: BTreeMap<&'a str, &'a AotRoutineArtifact>,
    direct_routines: BTreeMap<String, NativeDirectRoutine>,
    states: BTreeMap<String, LoweringState>,
}

impl<'a> NativeLoweringBuilder<'a> {
    fn new(routines: &'a [AotRoutineArtifact]) -> Self {
        Self {
            routines_by_key: routines
                .iter()
                .map(|routine| (routine.routine_key.as_str(), routine))
                .collect(),
            direct_routines: BTreeMap::new(),
            states: BTreeMap::new(),
        }
    }

    fn finish(self, target: AotEmitTarget, launch: NativeLaunchLowering) -> NativeLoweringPlan {
        NativeLoweringPlan {
            target,
            direct_routines: self.direct_routines.into_values().collect(),
            launch,
        }
    }

    fn lower_root(
        &mut self,
        routine_key: &str,
        expected_params: &[NativeAbiParam],
        expected_return_type: &NativeAbiType,
    ) -> NativeRoutineLowering {
        let Some(signature) = self.signature_for(routine_key) else {
            return NativeRoutineLowering::RuntimeDispatch;
        };
        if signature.params != expected_params || signature.return_type != *expected_return_type {
            return NativeRoutineLowering::RuntimeDispatch;
        }
        self.lower_routine(routine_key, &signature)
    }

    fn lower_routine(
        &mut self,
        routine_key: &str,
        signature: &NativeRoutineSignature,
    ) -> NativeRoutineLowering {
        match self.states.get(routine_key).copied() {
            Some(LoweringState::InProgress) => return NativeRoutineLowering::RuntimeDispatch,
            Some(LoweringState::Direct) => {
                return NativeRoutineLowering::Direct {
                    routine_key: routine_key.to_string(),
                };
            }
            Some(LoweringState::RuntimeDispatch) => return NativeRoutineLowering::RuntimeDispatch,
            None => {}
        }

        self.states
            .insert(routine_key.to_string(), LoweringState::InProgress);

        let Some(routine) = self
            .routines_by_key
            .get(routine_key)
            .map(|routine| (*routine).clone())
        else {
            self.states
                .insert(routine_key.to_string(), LoweringState::RuntimeDispatch);
            return NativeRoutineLowering::RuntimeDispatch;
        };

        let lowering = self.compute_direct_routine(&routine, signature);
        match lowering {
            Some(direct) => {
                self.direct_routines
                    .insert(routine_key.to_string(), direct.clone());
                self.states
                    .insert(routine_key.to_string(), LoweringState::Direct);
                NativeRoutineLowering::Direct {
                    routine_key: routine_key.to_string(),
                }
            }
            None => {
                self.states
                    .insert(routine_key.to_string(), LoweringState::RuntimeDispatch);
                NativeRoutineLowering::RuntimeDispatch
            }
        }
    }

    fn compute_direct_routine(
        &mut self,
        routine: &AotRoutineArtifact,
        signature: &NativeRoutineSignature,
    ) -> Option<NativeDirectRoutine> {
        if routine.symbol_kind != "fn"
            || routine.is_async
            || routine.intrinsic_impl.is_some()
            || routine.impl_target_type.is_some()
            || routine.impl_trait_path.is_some()
            || !routine.type_param_rows.is_empty()
            || !routine.foreword_rows.is_empty()
            || !routine.rollups.is_empty()
        {
            return None;
        }

        let [ExecStmt::ReturnValue { value }] = routine.statements.as_slice() else {
            return None;
        };

        let param_types = signature
            .params
            .iter()
            .map(|param| (param.name.clone(), param.ty.clone()))
            .collect::<BTreeMap<_, _>>();
        let body = self.lower_expr(value, &param_types, &signature.return_type)?;
        Some(NativeDirectRoutine {
            routine_key: routine.routine_key.clone(),
            params: signature.params.clone(),
            return_type: signature.return_type.clone(),
            body,
        })
    }

    fn lower_expr(
        &mut self,
        expr: &ExecExpr,
        param_types: &BTreeMap<String, NativeAbiType>,
        expected: &NativeAbiType,
    ) -> Option<NativeDirectExpr> {
        match (expected, expr) {
            (NativeAbiType::Int, ExecExpr::Int(value)) => Some(NativeDirectExpr::Int(*value)),
            (NativeAbiType::Bool, ExecExpr::Bool(value)) => Some(NativeDirectExpr::Bool(*value)),
            (NativeAbiType::Str, ExecExpr::Str(value)) => {
                Some(NativeDirectExpr::Str(value.clone()))
            }
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
                    left: Box::new(self.lower_expr(left, param_types, left_ty)?),
                    right: Box::new(self.lower_expr(right, param_types, right_ty)?),
                })
            }
            (NativeAbiType::Str, ExecExpr::Binary { left, op, right })
                if *op == ExecBinaryOp::Add =>
            {
                Some(NativeDirectExpr::StringConcat {
                    left: Box::new(self.lower_expr(left, param_types, &NativeAbiType::Str)?),
                    right: Box::new(self.lower_expr(right, param_types, &NativeAbiType::Str)?),
                })
            }
            (_, ExecExpr::Path(segments)) if segments.len() == 1 => {
                let name = segments[0].clone();
                param_types
                    .get(&name)
                    .filter(|ty| *ty == expected)
                    .map(|_| NativeDirectExpr::Param(name))
            }
            (
                _,
                ExecExpr::Phrase {
                    args,
                    qualifier_kind,
                    resolved_routine: Some(callee_key),
                    dynamic_dispatch,
                    attached,
                    ..
                },
            ) if *qualifier_kind == ExecPhraseQualifierKind::Call
                && dynamic_dispatch.is_none()
                && attached.is_empty()
                && args.iter().all(|arg| arg.name.is_none()) =>
            {
                let callee_signature = self.signature_for(callee_key)?;
                if callee_signature.return_type != *expected {
                    return None;
                }
                if args.len() != callee_signature.params.len() {
                    return None;
                }
                let NativeRoutineLowering::Direct { routine_key } =
                    self.lower_routine(callee_key, &callee_signature)
                else {
                    return None;
                };
                let lowered_args = args
                    .iter()
                    .zip(&callee_signature.params)
                    .map(|(arg, param)| self.lower_expr(&arg.value, param_types, &param.ty))
                    .collect::<Option<Vec<_>>>()?;
                Some(NativeDirectExpr::Call {
                    routine_key,
                    args: lowered_args,
                })
            }
            _ => None,
        }
    }

    fn signature_for(&self, routine_key: &str) -> Option<NativeRoutineSignature> {
        let routine = self.routines_by_key.get(routine_key)?;
        parse_native_routine_signature(&routine.param_rows, &routine.signature_row).ok()
    }
}

pub fn build_native_lowering_plan(plan: &NativePackagePlan) -> Result<NativeLoweringPlan, String> {
    let mut builder = NativeLoweringBuilder::new(&plan.artifact.routines);
    let launch = match &plan.launch {
        NativeLaunchPlan::Executable { main_routine_key } => NativeLaunchLowering::Executable {
            main_routine_key: main_routine_key.clone(),
            lowering: builder.lower_root(main_routine_key, &[], &NativeAbiType::Int),
        },
        NativeLaunchPlan::DynamicLibrary { exports } => NativeLaunchLowering::DynamicLibrary {
            exports: exports
                .iter()
                .cloned()
                .map(|export| NativeExportLowering {
                    lowering: builder.lower_root(
                        &export.routine_key,
                        &export.params,
                        &export.return_type,
                    ),
                    export,
                })
                .collect(),
        },
    };
    Ok(builder.finish(plan.target, launch))
}

#[cfg(test)]
mod tests {
    use super::{
        NativeDirectExpr, NativeLaunchLowering, NativeRoutineLowering, build_native_lowering_plan,
    };
    use crate::emit::{AotEmitContext, AotEmitTarget};
    use crate::native_plan::build_native_package_plan;
    use arcana_ir::{
        ExecExpr, ExecPhraseArg, ExecPhraseQualifierKind, ExecStmt, IrEntrypoint, IrPackage,
        IrPackageModule, IrRoutine,
    };

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
                lowering: NativeRoutineLowering::Direct {
                    routine_key: "core#fn-0".to_string(),
                },
            }
        );
        assert_eq!(lowering_plan.direct_routines.len(), 1);
        assert_eq!(
            lowering_plan.direct_routines[0].body,
            NativeDirectExpr::Int(9)
        );
    }

    #[test]
    fn lowering_marks_resolved_helper_calls_as_direct() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
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
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: vec![ExecPhraseArg {
                            name: None,
                            value: ExecExpr::Int(9),
                        }],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=:name=value:ty=Int".to_string()],
                signature_row: "fn helper(value: Int) -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["value".to_string()]),
                }],
            },
        ]);

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

        assert_eq!(lowering_plan.direct_routines.len(), 2);
        assert!(matches!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                lowering: NativeRoutineLowering::Direct { .. },
                ..
            }
        ));
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-1"
                && routine.body == NativeDirectExpr::Param("value".to_string())
        }));
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-0"
                && routine.body
                    == NativeDirectExpr::Call {
                        routine_key: "core#fn-1".to_string(),
                        args: vec![NativeDirectExpr::Int(9)],
                    }
        }));
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
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-4".to_string(),
                symbol_name: "answer_via_helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn answer_via_helper() -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: Vec::new(),
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-5".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-5".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn helper() -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(21),
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
        assert_eq!(exports.len(), 5);
        assert!(matches!(
            exports[0].lowering,
            NativeRoutineLowering::Direct { ref routine_key } if routine_key == "core#fn-0"
        ));
        assert!(matches!(
            exports[1].lowering,
            NativeRoutineLowering::Direct { ref routine_key } if routine_key == "core#fn-1"
        ));
        assert_eq!(exports[2].lowering, NativeRoutineLowering::RuntimeDispatch);
        assert!(matches!(
            exports[3].lowering,
            NativeRoutineLowering::Direct { ref routine_key } if routine_key == "core#fn-3"
        ));
        assert!(matches!(
            exports[4].lowering,
            NativeRoutineLowering::Direct { ref routine_key } if routine_key == "core#fn-4"
        ));
        assert_eq!(lowering_plan.direct_routines.len(), 5);
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-4"
                && routine.body
                    == NativeDirectExpr::Call {
                        routine_key: "core#fn-5".to_string(),
                        args: Vec::new(),
                    }
        }));
    }

    #[test]
    fn lowering_keeps_named_or_attached_calls_on_runtime_dispatch() {
        let mut package = base_package();
        package.routines.extend([
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "named_call".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=:name=value:ty=Int".to_string()],
                signature_row: "fn named_call(value: Int) -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: vec![ExecPhraseArg {
                            name: Some("value".to_string()),
                            value: ExecExpr::Path(vec!["value".to_string()]),
                        }],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: vec!["mode=:name=value:ty=Int".to_string()],
                signature_row: "fn helper(value: Int) -> Int:".to_string(),
                intrinsic_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                foreword_rows: Vec::new(),
                rollups: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["value".to_string()]),
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
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].lowering, NativeRoutineLowering::RuntimeDispatch);
    }
}
