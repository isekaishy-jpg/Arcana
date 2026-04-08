use std::collections::BTreeMap;

use arcana_cabi::ArcanaCabiPassMode;
use arcana_ir::{
    ExecAssignOp, ExecAssignTarget, ExecBinaryOp, ExecBindLineKind, ExecCleanupFooter, ExecExpr,
    ExecHeadedModifier, ExecPhraseQualifierKind, ExecRecycleLineKind, ExecStmt,
};

use crate::artifact::AotRoutineArtifact;
use crate::emit::AotEmitTarget;
use crate::native_abi::{
    NativeAbiParam, NativeAbiType, NativeExport, NativeRoutineSignature, native_routine_signature,
};
use crate::native_plan::{NativeLaunchPlan, NativePackagePlan, NativeRoutineHints};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeDirectExpr {
    Int(i64),
    Bool(bool),
    Unit,
    Str(String),
    Bytes(Vec<u8>),
    Binding(String),
    IntBinary {
        op: NativeDirectIntBinaryOp,
        left: Box<NativeDirectExpr>,
        right: Box<NativeDirectExpr>,
    },
    IntCompare {
        op: NativeDirectIntCompareOp,
        left: Box<NativeDirectExpr>,
        right: Box<NativeDirectExpr>,
    },
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
        params: Vec<NativeAbiParam>,
        args: Vec<NativeDirectExpr>,
    },
    If {
        condition: Box<NativeDirectExpr>,
        then_block: Box<NativeDirectBlock>,
        else_block: Box<NativeDirectBlock>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeDirectStmt {
    Let {
        mutable: bool,
        name: String,
        value: NativeDirectExpr,
    },
    Expr {
        value: NativeDirectExpr,
    },
    Cleanup {
        action: NativeCleanupAction,
    },
    Assign {
        name: String,
        value: NativeDirectExpr,
    },
    Return {
        value: NativeDirectExpr,
    },
    If {
        condition: NativeDirectExpr,
        then_body: Vec<NativeDirectStmt>,
        else_body: Vec<NativeDirectStmt>,
    },
    While {
        condition: NativeDirectExpr,
        body: Vec<NativeDirectStmt>,
    },
    Break,
    Continue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeCleanupAction {
    Direct {
        value: NativeDirectExpr,
    },
    RuntimeDispatch {
        routine_key: String,
        arg: NativeDirectExpr,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeDirectBlock {
    pub statements: Vec<NativeDirectStmt>,
    pub return_expr: NativeDirectExpr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDirectIntBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeDirectIntCompareOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeDirectRoutine {
    pub routine_key: String,
    pub params: Vec<NativeAbiParam>,
    pub return_type: NativeAbiType,
    pub inline_hint: bool,
    pub cold_hint: bool,
    pub body: NativeDirectBlock,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoweredDirectExpr {
    expr: NativeDirectExpr,
    ty: NativeAbiType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeBinding {
    binding_id: u64,
    ty: NativeAbiType,
    mutable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeCleanupScope {
    defers: Vec<NativeDirectExpr>,
    cleanup_actions: Vec<NativeCleanupAction>,
    loop_boundary: bool,
}

#[derive(Clone, Copy)]
struct NativeBlockLoweringCtx<'a> {
    expected_return_type: &'a NativeAbiType,
    current_cleanup_footers: &'a [ExecCleanupFooter],
    current_cleanup_actions: &'a [NativeCleanupAction],
    outer_cleanup_scopes: &'a [NativeCleanupScope],
    current_scope_is_loop_boundary: bool,
}

struct NativeLoweringBuilder<'a> {
    routines_by_key: BTreeMap<&'a str, &'a AotRoutineArtifact>,
    routine_hints: &'a BTreeMap<String, NativeRoutineHints>,
    direct_routines: BTreeMap<String, NativeDirectRoutine>,
    states: BTreeMap<String, LoweringState>,
    temp_counter: usize,
}

impl<'a> NativeLoweringBuilder<'a> {
    fn new(
        routines: &'a [AotRoutineArtifact],
        routine_hints: &'a BTreeMap<String, NativeRoutineHints>,
    ) -> Self {
        Self {
            routines_by_key: routines
                .iter()
                .map(|routine| (routine.routine_key.as_str(), routine))
                .collect(),
            routine_hints,
            direct_routines: BTreeMap::new(),
            states: BTreeMap::new(),
            temp_counter: 0,
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
            || !routine.type_params.is_empty()
        {
            return None;
        }
        let bindings = signature
            .params
            .iter()
            .zip(routine.params.iter())
            .map(|param| {
                let (param, routine_param) = param;
                (
                    param.name.clone(),
                    NativeBinding {
                        binding_id: routine_param.binding_id,
                        ty: param.input_type.clone(),
                        mutable: matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let initial_binding_names = routine
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect::<Vec<_>>();
        let body = self.lower_block(
            &routine.statements,
            &bindings,
            &signature.return_type,
            &routine.cleanup_footers,
            &initial_binding_names,
        )?;
        let hints = self
            .routine_hints
            .get(&routine.routine_key)
            .copied()
            .unwrap_or_default();
        Some(NativeDirectRoutine {
            routine_key: routine.routine_key.clone(),
            params: signature.params.clone(),
            return_type: signature.return_type.clone(),
            inline_hint: hints.inline_hint,
            cold_hint: hints.cold_hint,
            body,
        })
    }

    fn lower_block(
        &mut self,
        statements: &[ExecStmt],
        bindings: &BTreeMap<String, NativeBinding>,
        expected_return_type: &NativeAbiType,
        current_cleanup_footers: &[ExecCleanupFooter],
        initial_binding_names: &[String],
    ) -> Option<NativeDirectBlock> {
        let current_cleanup_actions =
            self.initial_cleanup_actions(current_cleanup_footers, bindings, initial_binding_names)?;
        self.lower_value_block(
            statements,
            bindings,
            NativeBlockLoweringCtx {
                expected_return_type,
                current_cleanup_footers,
                current_cleanup_actions: &current_cleanup_actions,
                outer_cleanup_scopes: &[],
                current_scope_is_loop_boundary: false,
            },
        )
    }

    fn lower_value_block(
        &mut self,
        statements: &[ExecStmt],
        bindings: &BTreeMap<String, NativeBinding>,
        ctx: NativeBlockLoweringCtx<'_>,
    ) -> Option<NativeDirectBlock> {
        let expected_return_type = ctx.expected_return_type;
        let current_cleanup_footers = ctx.current_cleanup_footers;
        let outer_cleanup_scopes = ctx.outer_cleanup_scopes;
        let mut bindings = bindings.clone();
        let mut lowered_statements = Vec::new();
        let mut current_defers = Vec::new();
        let mut current_cleanup_actions = ctx.current_cleanup_actions.to_vec();
        let mut iter = statements.iter().peekable();
        while let Some(stmt) = iter.next() {
            let is_last = iter.peek().is_none();
            match stmt {
                ExecStmt::Defer(expr) => {
                    current_defers.push(self.lower_typed_expr(expr, &bindings)?.expr);
                }
                ExecStmt::Let { name, .. } => {
                    lowered_statements.push(self.lower_stmt(stmt, &mut bindings)?);
                    self.activate_cleanup_actions_for_binding(
                        &mut current_cleanup_actions,
                        current_cleanup_footers,
                        name,
                        &bindings,
                    )?;
                }
                ExecStmt::Assign { .. } => {
                    lowered_statements.push(self.lower_stmt(stmt, &mut bindings)?);
                }
                ExecStmt::Expr {
                    expr,
                    cleanup_footers,
                } if cleanup_footers.is_empty() => {
                    lowered_statements.push(NativeDirectStmt::Expr {
                        value: self.lower_typed_expr(expr, &bindings)?.expr,
                    });
                }
                ExecStmt::ReturnValue { value } if is_last => {
                    let return_expr = self.lower_expr(value, &bindings, expected_return_type)?;
                    return Some(self.wrap_tail_expr_with_cleanup(
                        lowered_statements,
                        return_expr,
                        &bindings,
                        &current_defers,
                        &current_cleanup_actions,
                    ));
                }
                ExecStmt::ReturnVoid if is_last && *expected_return_type == NativeAbiType::Unit => {
                    return Some(self.wrap_tail_expr_with_cleanup(
                        lowered_statements,
                        NativeDirectExpr::Unit,
                        &bindings,
                        &current_defers,
                        &current_cleanup_actions,
                    ));
                }
                ExecStmt::If {
                    condition,
                    then_branch,
                    else_branch,
                    cleanup_footers,
                    ..
                } if cleanup_footers.is_empty() && is_last => {
                    let nested_scopes = self.extend_cleanup_scopes(
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                        ctx.current_scope_is_loop_boundary,
                    );
                    let condition = self.lower_expr(condition, &bindings, &NativeAbiType::Bool)?;
                    let then_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    let else_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    let then_block = self.lower_value_block(
                        then_branch,
                        &bindings,
                        NativeBlockLoweringCtx {
                            expected_return_type,
                            current_cleanup_footers: cleanup_footers,
                            current_cleanup_actions: &then_cleanup_actions,
                            outer_cleanup_scopes: &nested_scopes,
                            current_scope_is_loop_boundary: false,
                        },
                    )?;
                    let else_block = self.lower_value_block(
                        else_branch,
                        &bindings,
                        NativeBlockLoweringCtx {
                            expected_return_type,
                            current_cleanup_footers: cleanup_footers,
                            current_cleanup_actions: &else_cleanup_actions,
                            outer_cleanup_scopes: &nested_scopes,
                            current_scope_is_loop_boundary: false,
                        },
                    )?;
                    return Some(self.wrap_tail_expr_with_cleanup(
                        lowered_statements,
                        NativeDirectExpr::If {
                            condition: Box::new(condition),
                            then_block: Box::new(then_block),
                            else_block: Box::new(else_block),
                        },
                        &bindings,
                        &current_defers,
                        &current_cleanup_actions,
                    ));
                }
                ExecStmt::ReturnValue { value } => {
                    let value = self.lower_typed_expr(value, &bindings)?.expr;
                    self.push_return_with_cleanup(
                        &mut lowered_statements,
                        value,
                        &bindings,
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                    );
                }
                ExecStmt::ReturnVoid if *expected_return_type == NativeAbiType::Unit => {
                    self.push_return_with_cleanup(
                        &mut lowered_statements,
                        NativeDirectExpr::Unit,
                        &bindings,
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                    );
                }
                ExecStmt::If {
                    condition,
                    then_branch,
                    else_branch,
                    cleanup_footers,
                    ..
                } => {
                    let nested_scopes = self.extend_cleanup_scopes(
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                        ctx.current_scope_is_loop_boundary,
                    );
                    let then_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    let else_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    lowered_statements.push(NativeDirectStmt::If {
                        condition: self.lower_expr(condition, &bindings, &NativeAbiType::Bool)?,
                        then_body: self.lower_void_block(
                            then_branch,
                            &bindings,
                            NativeBlockLoweringCtx {
                                expected_return_type,
                                current_cleanup_footers: cleanup_footers,
                                current_cleanup_actions: &then_cleanup_actions,
                                outer_cleanup_scopes: &nested_scopes,
                                current_scope_is_loop_boundary: false,
                            },
                        )?,
                        else_body: self.lower_void_block(
                            else_branch,
                            &bindings,
                            NativeBlockLoweringCtx {
                                expected_return_type,
                                current_cleanup_footers: cleanup_footers,
                                current_cleanup_actions: &else_cleanup_actions,
                                outer_cleanup_scopes: &nested_scopes,
                                current_scope_is_loop_boundary: false,
                            },
                        )?,
                    });
                }
                ExecStmt::While {
                    condition,
                    body,
                    cleanup_footers,
                    ..
                } => {
                    let nested_scopes = self.extend_cleanup_scopes(
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                        ctx.current_scope_is_loop_boundary,
                    );
                    let body_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    lowered_statements.push(NativeDirectStmt::While {
                        condition: self.lower_expr(condition, &bindings, &NativeAbiType::Bool)?,
                        body: self.lower_void_block(
                            body,
                            &bindings,
                            NativeBlockLoweringCtx {
                                expected_return_type,
                                current_cleanup_footers: cleanup_footers,
                                current_cleanup_actions: &body_cleanup_actions,
                                outer_cleanup_scopes: &nested_scopes,
                                current_scope_is_loop_boundary: true,
                            },
                        )?,
                    });
                }
                ExecStmt::Recycle {
                    default_modifier,
                    lines,
                } => {
                    lowered_statements.extend(self.lower_recycle_stmt(
                        default_modifier.as_ref(),
                        lines,
                        &bindings,
                        ctx,
                        &current_defers,
                        &current_cleanup_actions,
                    )?);
                }
                ExecStmt::Bind {
                    default_modifier,
                    lines,
                } => {
                    lowered_statements.extend(self.lower_bind_stmt(
                        default_modifier.as_ref(),
                        lines,
                        &bindings,
                        ctx,
                        &current_defers,
                        &current_cleanup_actions,
                    )?);
                }
                // General construct regions still rely on runtime dispatch because the
                // direct lowering model does not yet carry record/variant/match values.
                ExecStmt::Record(_) => return None,
                ExecStmt::Construct(_) => return None,
                ExecStmt::MemorySpec(_) => {}
                _ => return None,
            }
        }
        None
    }

    fn lower_void_block(
        &mut self,
        statements: &[ExecStmt],
        bindings: &BTreeMap<String, NativeBinding>,
        ctx: NativeBlockLoweringCtx<'_>,
    ) -> Option<Vec<NativeDirectStmt>> {
        let expected_return_type = ctx.expected_return_type;
        let current_cleanup_footers = ctx.current_cleanup_footers;
        let outer_cleanup_scopes = ctx.outer_cleanup_scopes;
        let mut bindings = bindings.clone();
        let mut lowered_statements = Vec::new();
        let mut current_defers = Vec::new();
        let mut current_cleanup_actions = ctx.current_cleanup_actions.to_vec();
        for stmt in statements {
            match stmt {
                ExecStmt::Defer(expr) => {
                    current_defers.push(self.lower_typed_expr(expr, &bindings)?.expr);
                }
                ExecStmt::Let { name, .. } => {
                    lowered_statements.push(self.lower_stmt(stmt, &mut bindings)?);
                    self.activate_cleanup_actions_for_binding(
                        &mut current_cleanup_actions,
                        current_cleanup_footers,
                        name,
                        &bindings,
                    )?;
                }
                ExecStmt::Assign { .. } => {
                    lowered_statements.push(self.lower_stmt(stmt, &mut bindings)?);
                }
                ExecStmt::Expr {
                    expr,
                    cleanup_footers,
                } if cleanup_footers.is_empty() => {
                    lowered_statements.push(NativeDirectStmt::Expr {
                        value: self.lower_typed_expr(expr, &bindings)?.expr,
                    });
                }
                ExecStmt::ReturnValue { value } => {
                    let value = self.lower_typed_expr(value, &bindings)?.expr;
                    self.push_return_with_cleanup(
                        &mut lowered_statements,
                        value,
                        &bindings,
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                    );
                    return Some(lowered_statements);
                }
                ExecStmt::ReturnVoid if *expected_return_type == NativeAbiType::Unit => {
                    self.push_return_with_cleanup(
                        &mut lowered_statements,
                        NativeDirectExpr::Unit,
                        &bindings,
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                    );
                    return Some(lowered_statements);
                }
                ExecStmt::If {
                    condition,
                    then_branch,
                    else_branch,
                    cleanup_footers,
                    ..
                } => {
                    let nested_scopes = self.extend_cleanup_scopes(
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                        ctx.current_scope_is_loop_boundary,
                    );
                    let then_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    let else_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    lowered_statements.push(NativeDirectStmt::If {
                        condition: self.lower_expr(condition, &bindings, &NativeAbiType::Bool)?,
                        then_body: self.lower_void_block(
                            then_branch,
                            &bindings,
                            NativeBlockLoweringCtx {
                                expected_return_type,
                                current_cleanup_footers: cleanup_footers,
                                current_cleanup_actions: &then_cleanup_actions,
                                outer_cleanup_scopes: &nested_scopes,
                                current_scope_is_loop_boundary: false,
                            },
                        )?,
                        else_body: self.lower_void_block(
                            else_branch,
                            &bindings,
                            NativeBlockLoweringCtx {
                                expected_return_type,
                                current_cleanup_footers: cleanup_footers,
                                current_cleanup_actions: &else_cleanup_actions,
                                outer_cleanup_scopes: &nested_scopes,
                                current_scope_is_loop_boundary: false,
                            },
                        )?,
                    });
                }
                ExecStmt::While {
                    condition,
                    body,
                    cleanup_footers,
                    ..
                } => {
                    let nested_scopes = self.extend_cleanup_scopes(
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                        ctx.current_scope_is_loop_boundary,
                    );
                    let body_cleanup_actions =
                        self.initial_cleanup_actions(cleanup_footers, &bindings, &[])?;
                    lowered_statements.push(NativeDirectStmt::While {
                        condition: self.lower_expr(condition, &bindings, &NativeAbiType::Bool)?,
                        body: self.lower_void_block(
                            body,
                            &bindings,
                            NativeBlockLoweringCtx {
                                expected_return_type,
                                current_cleanup_footers: cleanup_footers,
                                current_cleanup_actions: &body_cleanup_actions,
                                outer_cleanup_scopes: &nested_scopes,
                                current_scope_is_loop_boundary: true,
                            },
                        )?,
                    });
                }
                ExecStmt::Break => {
                    self.push_loop_exit_with_cleanup(
                        &mut lowered_statements,
                        NativeDirectStmt::Break,
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                        ctx.current_scope_is_loop_boundary,
                    )?;
                    return Some(lowered_statements);
                }
                ExecStmt::Continue => {
                    self.push_loop_exit_with_cleanup(
                        &mut lowered_statements,
                        NativeDirectStmt::Continue,
                        outer_cleanup_scopes,
                        &current_defers,
                        &current_cleanup_actions,
                        ctx.current_scope_is_loop_boundary,
                    )?;
                    return Some(lowered_statements);
                }
                ExecStmt::Recycle {
                    default_modifier,
                    lines,
                } => {
                    lowered_statements.extend(self.lower_recycle_stmt(
                        default_modifier.as_ref(),
                        lines,
                        &bindings,
                        ctx,
                        &current_defers,
                        &current_cleanup_actions,
                    )?);
                }
                ExecStmt::Bind {
                    default_modifier,
                    lines,
                } => {
                    lowered_statements.extend(self.lower_bind_stmt(
                        default_modifier.as_ref(),
                        lines,
                        &bindings,
                        ctx,
                        &current_defers,
                        &current_cleanup_actions,
                    )?);
                }
                // General construct regions still rely on runtime dispatch because the
                // direct lowering model does not yet carry record/variant/match values.
                ExecStmt::Record(_) => return None,
                ExecStmt::Construct(_) => return None,
                ExecStmt::MemorySpec(_) => {}
                _ => return None,
            }
        }
        self.append_cleanup_scope(
            &mut lowered_statements,
            &current_defers,
            &current_cleanup_actions,
        );
        Some(lowered_statements)
    }

    fn lower_stmt(
        &mut self,
        stmt: &ExecStmt,
        bindings: &mut BTreeMap<String, NativeBinding>,
    ) -> Option<NativeDirectStmt> {
        match stmt {
            ExecStmt::Let {
                binding_id,
                mutable,
                name,
                value,
            } => {
                let lowered = self.lower_typed_expr(value, bindings)?;
                bindings.insert(
                    name.clone(),
                    NativeBinding {
                        binding_id: *binding_id,
                        ty: lowered.ty.clone(),
                        mutable: *mutable,
                    },
                );
                Some(NativeDirectStmt::Let {
                    mutable: *mutable,
                    name: name.clone(),
                    value: lowered.expr,
                })
            }
            ExecStmt::Assign { target, op, value } => {
                // The current direct subset only supports writes back into simple local bindings.
                // Structured targets such as member/index assignment stay on runtime dispatch.
                let ExecAssignTarget::Name(name) = target else {
                    return None;
                };
                let binding = bindings.get(name)?;
                if !binding.mutable {
                    return None;
                }
                Some(NativeDirectStmt::Assign {
                    name: name.clone(),
                    value: self.lower_assignment_value(name, binding, *op, value, bindings)?,
                })
            }
            _ => None,
        }
    }

    fn lower_headed_exit_body(
        &mut self,
        modifier: &ExecHeadedModifier,
        bindings: &BTreeMap<String, NativeBinding>,
        ctx: NativeBlockLoweringCtx<'_>,
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
    ) -> Option<Vec<NativeDirectStmt>> {
        let mut body = Vec::new();
        match modifier.kind.as_str() {
            "return" => {
                let payload = modifier.payload.as_ref()?;
                let value = self.lower_expr(payload, bindings, ctx.expected_return_type)?;
                self.push_return_with_cleanup(
                    &mut body,
                    value,
                    bindings,
                    ctx.outer_cleanup_scopes,
                    current_defers,
                    current_cleanup_actions,
                );
            }
            "break" => {
                self.push_loop_exit_with_cleanup(
                    &mut body,
                    NativeDirectStmt::Break,
                    ctx.outer_cleanup_scopes,
                    current_defers,
                    current_cleanup_actions,
                    ctx.current_scope_is_loop_boundary,
                )?;
            }
            "continue" => {
                self.push_loop_exit_with_cleanup(
                    &mut body,
                    NativeDirectStmt::Continue,
                    ctx.outer_cleanup_scopes,
                    current_defers,
                    current_cleanup_actions,
                    ctx.current_scope_is_loop_boundary,
                )?;
            }
            _ => return None,
        }
        Some(body)
    }

    fn lower_recycle_stmt(
        &mut self,
        default_modifier: Option<&ExecHeadedModifier>,
        lines: &[arcana_ir::ExecRecycleLine],
        bindings: &BTreeMap<String, NativeBinding>,
        ctx: NativeBlockLoweringCtx<'_>,
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
    ) -> Option<Vec<NativeDirectStmt>> {
        let mut statements = Vec::new();
        for line in lines {
            let ExecRecycleLineKind::Expr { gate } = &line.kind else {
                return None;
            };
            let condition = self.lower_expr(gate, bindings, &NativeAbiType::Bool)?;
            let modifier = line.modifier.as_ref().or(default_modifier)?;
            let exit_body = self.lower_headed_exit_body(
                modifier,
                bindings,
                ctx,
                current_defers,
                current_cleanup_actions,
            )?;
            statements.push(NativeDirectStmt::If {
                condition,
                then_body: Vec::new(),
                else_body: exit_body,
            });
        }
        Some(statements)
    }

    fn lower_bind_stmt(
        &mut self,
        default_modifier: Option<&ExecHeadedModifier>,
        lines: &[arcana_ir::ExecBindLine],
        bindings: &BTreeMap<String, NativeBinding>,
        ctx: NativeBlockLoweringCtx<'_>,
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
    ) -> Option<Vec<NativeDirectStmt>> {
        let mut statements = Vec::new();
        for line in lines {
            let ExecBindLineKind::Require { expr } = &line.kind else {
                return None;
            };
            let condition = self.lower_expr(expr, bindings, &NativeAbiType::Bool)?;
            let modifier = line.modifier.as_ref().or(default_modifier)?;
            let exit_body = self.lower_headed_exit_body(
                modifier,
                bindings,
                ctx,
                current_defers,
                current_cleanup_actions,
            )?;
            statements.push(NativeDirectStmt::If {
                condition,
                then_body: Vec::new(),
                else_body: exit_body,
            });
        }
        Some(statements)
    }

    fn wrap_tail_expr_with_cleanup(
        &mut self,
        mut statements: Vec<NativeDirectStmt>,
        return_expr: NativeDirectExpr,
        bindings: &BTreeMap<String, NativeBinding>,
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
    ) -> NativeDirectBlock {
        if current_defers.is_empty() && current_cleanup_actions.is_empty() {
            return NativeDirectBlock {
                statements,
                return_expr,
            };
        }
        let temp_name = self.fresh_temp_name(bindings, "defer_return");
        statements.push(NativeDirectStmt::Let {
            mutable: false,
            name: temp_name.clone(),
            value: return_expr,
        });
        self.append_cleanup_scope(&mut statements, current_defers, current_cleanup_actions);
        NativeDirectBlock {
            statements,
            return_expr: NativeDirectExpr::Binding(temp_name),
        }
    }

    fn push_return_with_cleanup(
        &mut self,
        statements: &mut Vec<NativeDirectStmt>,
        value: NativeDirectExpr,
        bindings: &BTreeMap<String, NativeBinding>,
        outer_cleanup_scopes: &[NativeCleanupScope],
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
    ) {
        let needs_cleanup = !current_defers.is_empty()
            || !current_cleanup_actions.is_empty()
            || outer_cleanup_scopes
                .iter()
                .any(|scope| !scope.defers.is_empty() || !scope.cleanup_actions.is_empty());
        if !needs_cleanup {
            statements.push(NativeDirectStmt::Return { value });
            return;
        }
        let temp_name = self.fresh_temp_name(bindings, "defer_return");
        statements.push(NativeDirectStmt::Let {
            mutable: false,
            name: temp_name.clone(),
            value,
        });
        self.append_return_cleanup(
            statements,
            outer_cleanup_scopes,
            current_defers,
            current_cleanup_actions,
        );
        statements.push(NativeDirectStmt::Return {
            value: NativeDirectExpr::Binding(temp_name),
        });
    }

    fn push_loop_exit_with_cleanup(
        &mut self,
        statements: &mut Vec<NativeDirectStmt>,
        control: NativeDirectStmt,
        outer_cleanup_scopes: &[NativeCleanupScope],
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
        current_scope_is_loop_boundary: bool,
    ) -> Option<()> {
        self.append_loop_cleanup(
            statements,
            outer_cleanup_scopes,
            current_defers,
            current_cleanup_actions,
            current_scope_is_loop_boundary,
        )?;
        statements.push(control);
        Some(())
    }

    fn extend_cleanup_scopes(
        &self,
        outer_cleanup_scopes: &[NativeCleanupScope],
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
        current_scope_is_loop_boundary: bool,
    ) -> Vec<NativeCleanupScope> {
        let mut scopes = outer_cleanup_scopes.to_vec();
        scopes.push(NativeCleanupScope {
            defers: current_defers.to_vec(),
            cleanup_actions: current_cleanup_actions.to_vec(),
            loop_boundary: current_scope_is_loop_boundary,
        });
        scopes
    }

    fn append_return_cleanup(
        &self,
        statements: &mut Vec<NativeDirectStmt>,
        outer_cleanup_scopes: &[NativeCleanupScope],
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
    ) {
        self.append_cleanup_scope(statements, current_defers, current_cleanup_actions);
        for scope in outer_cleanup_scopes.iter().rev() {
            self.append_cleanup_scope(statements, &scope.defers, &scope.cleanup_actions);
        }
    }

    fn append_loop_cleanup(
        &self,
        statements: &mut Vec<NativeDirectStmt>,
        outer_cleanup_scopes: &[NativeCleanupScope],
        current_defers: &[NativeDirectExpr],
        current_cleanup_actions: &[NativeCleanupAction],
        current_scope_is_loop_boundary: bool,
    ) -> Option<()> {
        self.append_cleanup_scope(statements, current_defers, current_cleanup_actions);
        if current_scope_is_loop_boundary {
            return Some(());
        }
        for scope in outer_cleanup_scopes.iter().rev() {
            self.append_cleanup_scope(statements, &scope.defers, &scope.cleanup_actions);
            if scope.loop_boundary {
                return Some(());
            }
        }
        None
    }

    fn append_cleanup_scope(
        &self,
        statements: &mut Vec<NativeDirectStmt>,
        defers: &[NativeDirectExpr],
        cleanup_actions: &[NativeCleanupAction],
    ) {
        for defer in defers.iter().rev() {
            statements.push(NativeDirectStmt::Expr {
                value: defer.clone(),
            });
        }
        for cleanup in cleanup_actions.iter().rev() {
            statements.push(NativeDirectStmt::Cleanup {
                action: cleanup.clone(),
            });
        }
    }

    fn initial_cleanup_actions(
        &mut self,
        cleanup_footers: &[ExecCleanupFooter],
        bindings: &BTreeMap<String, NativeBinding>,
        initial_binding_names: &[String],
    ) -> Option<Vec<NativeCleanupAction>> {
        let mut cleanup_actions = Vec::new();
        for name in initial_binding_names {
            self.activate_cleanup_actions_for_binding(
                &mut cleanup_actions,
                cleanup_footers,
                name,
                bindings,
            )?;
        }
        Some(cleanup_actions)
    }

    fn activate_cleanup_actions_for_binding(
        &mut self,
        cleanup_actions: &mut Vec<NativeCleanupAction>,
        cleanup_footers: &[ExecCleanupFooter],
        name: &str,
        bindings: &BTreeMap<String, NativeBinding>,
    ) -> Option<()> {
        let binding_id = bindings
            .get(name)
            .map(|binding| binding.binding_id)
            .unwrap_or(0);
        for rollup in cleanup_footers {
            let matches_binding = if rollup.binding_id != 0 {
                rollup.binding_id == binding_id
            } else {
                rollup.subject == name
            };
            if !matches_binding {
                continue;
            }
            cleanup_actions.push(self.lower_cleanup_action(rollup, name, bindings)?);
        }
        Some(())
    }

    fn lower_cleanup_action(
        &mut self,
        rollup: &ExecCleanupFooter,
        binding_name: &str,
        bindings: &BTreeMap<String, NativeBinding>,
    ) -> Option<NativeCleanupAction> {
        let callee_key = rollup.resolved_routine.as_deref()?;
        let binding = bindings.get(binding_name)?;
        let arg = NativeDirectExpr::Binding(binding_name.to_string());
        if let Some(callee_signature) = self.signature_for(callee_key)
            && callee_signature.params.len() == 1
        {
            let direct_arg = match callee_signature.params[0].pass_mode {
                ArcanaCabiPassMode::In => arg.clone(),
                ArcanaCabiPassMode::InWithWriteBack if binding.mutable => arg.clone(),
                ArcanaCabiPassMode::InWithWriteBack => return None,
            };
            if let NativeRoutineLowering::Direct { routine_key } =
                self.lower_routine(callee_key, &callee_signature)
            {
                return Some(NativeCleanupAction::Direct {
                    value: NativeDirectExpr::Call {
                        routine_key,
                        params: callee_signature.params.clone(),
                        args: vec![direct_arg],
                    },
                });
            }
        }
        let routine = self.routines_by_key.get(callee_key)?;
        if routine.params.len() != 1 {
            return None;
        }
        match routine.params[0].mode.as_deref() {
            None | Some("take") | Some("read") => {}
            Some("edit") => return None,
            Some(_) => return None,
        }
        match binding.ty {
            NativeAbiType::Int
            | NativeAbiType::Bool
            | NativeAbiType::Str
            | NativeAbiType::Bytes
            | NativeAbiType::Opaque(_)
            | NativeAbiType::Unit
            | NativeAbiType::Pair(_, _) => {}
        };
        Some(NativeCleanupAction::RuntimeDispatch {
            routine_key: callee_key.to_string(),
            arg,
        })
    }

    fn fresh_temp_name(
        &mut self,
        bindings: &BTreeMap<String, NativeBinding>,
        suffix: &str,
    ) -> String {
        loop {
            let candidate = format!("__arcana_{suffix}_{}", self.temp_counter);
            self.temp_counter += 1;
            if !bindings.contains_key(&candidate) {
                return candidate;
            }
        }
    }

    fn lower_assignment_value(
        &mut self,
        name: &str,
        binding: &NativeBinding,
        op: ExecAssignOp,
        value: &ExecExpr,
        bindings: &BTreeMap<String, NativeBinding>,
    ) -> Option<NativeDirectExpr> {
        match op {
            ExecAssignOp::Assign => self.lower_expr(value, bindings, &binding.ty),
            ExecAssignOp::AddAssign if binding.ty == NativeAbiType::Int => {
                Some(NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Add,
                    left: Box::new(NativeDirectExpr::Binding(name.to_string())),
                    right: Box::new(self.lower_expr(value, bindings, &NativeAbiType::Int)?),
                })
            }
            ExecAssignOp::SubAssign if binding.ty == NativeAbiType::Int => {
                Some(NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Sub,
                    left: Box::new(NativeDirectExpr::Binding(name.to_string())),
                    right: Box::new(self.lower_expr(value, bindings, &NativeAbiType::Int)?),
                })
            }
            ExecAssignOp::MulAssign if binding.ty == NativeAbiType::Int => {
                Some(NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Mul,
                    left: Box::new(NativeDirectExpr::Binding(name.to_string())),
                    right: Box::new(self.lower_expr(value, bindings, &NativeAbiType::Int)?),
                })
            }
            ExecAssignOp::DivAssign if binding.ty == NativeAbiType::Int => {
                Some(NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Div,
                    left: Box::new(NativeDirectExpr::Binding(name.to_string())),
                    right: Box::new(self.lower_expr(value, bindings, &NativeAbiType::Int)?),
                })
            }
            ExecAssignOp::ModAssign if binding.ty == NativeAbiType::Int => {
                Some(NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Mod,
                    left: Box::new(NativeDirectExpr::Binding(name.to_string())),
                    right: Box::new(self.lower_expr(value, bindings, &NativeAbiType::Int)?),
                })
            }
            _ => None,
        }
    }

    fn lower_expr(
        &mut self,
        expr: &ExecExpr,
        bindings: &BTreeMap<String, NativeBinding>,
        expected: &NativeAbiType,
    ) -> Option<NativeDirectExpr> {
        let lowered = self.lower_typed_expr(expr, bindings)?;
        (lowered.ty == *expected).then_some(lowered.expr)
    }

    fn lower_typed_expr(
        &mut self,
        expr: &ExecExpr,
        bindings: &BTreeMap<String, NativeBinding>,
    ) -> Option<LoweredDirectExpr> {
        match expr {
            ExecExpr::Int(value) => Some(LoweredDirectExpr {
                expr: NativeDirectExpr::Int(*value),
                ty: NativeAbiType::Int,
            }),
            ExecExpr::Bool(value) => Some(LoweredDirectExpr {
                expr: NativeDirectExpr::Bool(*value),
                ty: NativeAbiType::Bool,
            }),
            ExecExpr::Str(value) => Some(LoweredDirectExpr {
                expr: NativeDirectExpr::Str(value.clone()),
                ty: NativeAbiType::Str,
            }),
            ExecExpr::Collection { items } => Some(LoweredDirectExpr {
                expr: NativeDirectExpr::Bytes(
                    items
                        .iter()
                        .map(|item| match item {
                            ExecExpr::Int(value) => u8::try_from(*value).ok(),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()?,
                ),
                ty: NativeAbiType::Bytes,
            }),
            ExecExpr::Pair { left, right } => {
                let left = self.lower_typed_expr(left, bindings)?;
                let right = self.lower_typed_expr(right, bindings)?;
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::Pair {
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Pair(Box::new(left.ty), Box::new(right.ty)),
                })
            }
            ExecExpr::Binary { left, op, right } => {
                self.lower_binary_expr(left, *op, right, bindings)
            }
            ExecExpr::Path(segments) if segments.len() == 1 => {
                let name = segments[0].clone();
                let ty = bindings.get(&name)?.ty.clone();
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::Binding(name),
                    ty,
                })
            }
            ExecExpr::Phrase {
                args,
                qualifier_kind,
                resolved_routine: Some(callee_key),
                dynamic_dispatch,
                attached,
                ..
            } if *qualifier_kind == ExecPhraseQualifierKind::Call
                && dynamic_dispatch.is_none()
                && attached.is_empty() =>
            {
                let callee_signature = self.signature_for(callee_key)?;
                if args.len() != callee_signature.params.len() {
                    return None;
                }
                let NativeRoutineLowering::Direct { routine_key } =
                    self.lower_routine(callee_key, &callee_signature)
                else {
                    return None;
                };
                let lowered_args =
                    self.lower_call_args(args, &callee_signature.params, bindings)?;
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::Call {
                        routine_key,
                        params: callee_signature.params.clone(),
                        args: lowered_args,
                    },
                    ty: callee_signature.return_type,
                })
            }
            // Construct expressions currently require runtime dispatch for non-ABI runtime values.
            ExecExpr::RecordRegion(_) => None,
            ExecExpr::ConstructRegion(_) => None,
            _ => None,
        }
    }

    fn lower_call_args(
        &mut self,
        args: &[arcana_ir::ExecPhraseArg],
        params: &[NativeAbiParam],
        bindings: &BTreeMap<String, NativeBinding>,
    ) -> Option<Vec<NativeDirectExpr>> {
        if args.iter().all(|arg| arg.name.is_none()) {
            return args
                .iter()
                .zip(params)
                .map(|(arg, param)| match param.pass_mode {
                    ArcanaCabiPassMode::In => {
                        self.lower_expr(&arg.value, bindings, &param.input_type)
                    }
                    ArcanaCabiPassMode::InWithWriteBack => {
                        let lowered = self.lower_expr(&arg.value, bindings, &param.input_type)?;
                        let NativeDirectExpr::Binding(name) = lowered else {
                            return None;
                        };
                        bindings
                            .get(&name)?
                            .mutable
                            .then_some(NativeDirectExpr::Binding(name))
                    }
                })
                .collect();
        }
        if !args.iter().all(|arg| arg.name.is_some()) {
            return None;
        }
        params
            .iter()
            .map(|param| {
                let arg = args
                    .iter()
                    .find(|arg| arg.name.as_deref() == Some(param.name.as_str()))?;
                match param.pass_mode {
                    ArcanaCabiPassMode::In => {
                        self.lower_expr(&arg.value, bindings, &param.input_type)
                    }
                    ArcanaCabiPassMode::InWithWriteBack => {
                        let lowered = self.lower_expr(&arg.value, bindings, &param.input_type)?;
                        let NativeDirectExpr::Binding(name) = lowered else {
                            return None;
                        };
                        bindings
                            .get(&name)?
                            .mutable
                            .then_some(NativeDirectExpr::Binding(name))
                    }
                }
            })
            .collect()
    }

    fn lower_binary_expr(
        &mut self,
        left: &ExecExpr,
        op: ExecBinaryOp,
        right: &ExecExpr,
        bindings: &BTreeMap<String, NativeBinding>,
    ) -> Option<LoweredDirectExpr> {
        let left = self.lower_typed_expr(left, bindings)?;
        let right = self.lower_typed_expr(right, bindings)?;
        match op {
            ExecBinaryOp::Add
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntBinary {
                        op: NativeDirectIntBinaryOp::Add,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Int,
                })
            }
            ExecBinaryOp::Sub
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntBinary {
                        op: NativeDirectIntBinaryOp::Sub,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Int,
                })
            }
            ExecBinaryOp::Mul
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntBinary {
                        op: NativeDirectIntBinaryOp::Mul,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Int,
                })
            }
            ExecBinaryOp::Div
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntBinary {
                        op: NativeDirectIntBinaryOp::Div,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Int,
                })
            }
            ExecBinaryOp::Mod
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntBinary {
                        op: NativeDirectIntBinaryOp::Mod,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Int,
                })
            }
            ExecBinaryOp::Add
                if left.ty == NativeAbiType::Str && right.ty == NativeAbiType::Str =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::StringConcat {
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Str,
                })
            }
            ExecBinaryOp::EqEq
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntCompare {
                        op: NativeDirectIntCompareOp::Eq,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Bool,
                })
            }
            ExecBinaryOp::NotEq
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntCompare {
                        op: NativeDirectIntCompareOp::NotEq,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Bool,
                })
            }
            ExecBinaryOp::Lt if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int => {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntCompare {
                        op: NativeDirectIntCompareOp::Lt,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Bool,
                })
            }
            ExecBinaryOp::LtEq
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntCompare {
                        op: NativeDirectIntCompareOp::LtEq,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Bool,
                })
            }
            ExecBinaryOp::Gt if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int => {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntCompare {
                        op: NativeDirectIntCompareOp::Gt,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Bool,
                })
            }
            ExecBinaryOp::GtEq
                if left.ty == NativeAbiType::Int && right.ty == NativeAbiType::Int =>
            {
                Some(LoweredDirectExpr {
                    expr: NativeDirectExpr::IntCompare {
                        op: NativeDirectIntCompareOp::GtEq,
                        left: Box::new(left.expr),
                        right: Box::new(right.expr),
                    },
                    ty: NativeAbiType::Bool,
                })
            }
            _ => None,
        }
    }

    fn signature_for(&self, routine_key: &str) -> Option<NativeRoutineSignature> {
        let routine = self.routines_by_key.get(routine_key)?;
        native_routine_signature(routine).ok()
    }
}

pub fn build_native_lowering_plan(plan: &NativePackagePlan) -> Result<NativeLoweringPlan, String> {
    let mut builder = NativeLoweringBuilder::new(&plan.artifact.routines, &plan.routine_hints);
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
    use std::collections::BTreeMap;

    use super::{
        NativeCleanupAction, NativeDirectExpr, NativeDirectIntBinaryOp, NativeDirectIntCompareOp,
        NativeDirectStmt, NativeLaunchLowering, NativeRoutineLowering, build_native_lowering_plan,
    };
    use crate::emit::{AotEmitContext, AotEmitTarget, AotRuntimeBinding};
    use crate::native_plan::build_native_package_plan;
    use arcana_ir::{
        ExecAssignOp, ExecAssignTarget, ExecBinaryOp, ExecBindLine, ExecBindLineKind,
        ExecCleanupFooter, ExecConstructContributionMode, ExecConstructDestination,
        ExecConstructLine, ExecConstructRegion, ExecExpr, ExecHeadedModifier, ExecMemorySpecDecl,
        ExecPhraseArg, ExecPhraseQualifierKind, ExecRecycleLine, ExecRecycleLineKind, ExecStmt,
        IrEntrypoint, IrPackage, IrPackageModule, IrRoutine, IrRoutineParam, IrRoutineType,
        parse_routine_type_text, render_routine_signature_text,
    };

    fn test_return_type(signature: &str) -> Option<IrRoutineType> {
        let (_, tail) = signature.rsplit_once("->")?;
        let trimmed = tail.trim().trim_end_matches(':').trim();
        (!trimmed.is_empty())
            .then(|| parse_routine_type_text(trimmed).expect("return type should parse"))
    }

    fn test_params<S: AsRef<str>>(rows: &[S]) -> Vec<IrRoutineParam> {
        rows.iter()
            .map(|row| {
                let row = row.as_ref();
                let parts = row.splitn(3, ':').collect::<Vec<_>>();
                let mode = parts[0].strip_prefix("mode=").unwrap_or_default();
                let name = parts[1].strip_prefix("name=").unwrap_or_default();
                let ty = parts[2].strip_prefix("ty=").unwrap_or_default();
                IrRoutineParam {
                    binding_id: 0,
                    mode: (!mode.is_empty()).then(|| mode.to_string()),
                    name: name.to_string(),
                    ty: parse_routine_type_text(ty).expect("type should parse"),
                }
            })
            .collect()
    }

    fn sync_exported_function_surface_rows(package: &mut IrPackage) {
        let exported_routines = package
            .routines
            .iter()
            .filter(|routine| routine.exported && routine.impl_target_type.is_none())
            .collect::<Vec<_>>();
        package.exported_surface_rows = exported_routines
            .iter()
            .map(|routine| {
                format!(
                    "module={}:export:{}:{}",
                    routine.module_id,
                    routine.symbol_kind,
                    render_routine_signature_text(
                        &routine.symbol_kind,
                        &routine.symbol_name,
                        routine.is_async,
                        &routine.type_params,
                        &routine.params,
                        routine.return_type.as_ref(),
                    )
                )
            })
            .collect();
        for module in &mut package.modules {
            module.exported_surface_rows = exported_routines
                .iter()
                .filter(|routine| routine.module_id == module.module_id)
                .map(|routine| {
                    format!(
                        "export:{}:{}",
                        routine.symbol_kind,
                        render_routine_signature_text(
                            &routine.symbol_kind,
                            &routine.symbol_name,
                            routine.is_async,
                            &routine.type_params,
                            &routine.params,
                            routine.return_type.as_ref(),
                        )
                    )
                })
                .collect();
        }
    }

    fn base_package() -> IrPackage {
        IrPackage {
            package_id: "core".to_string(),
            package_name: "core".to_string(),
            root_module_id: "core".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: test_package_display_names_with_deps(
                "core".to_string(),
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            package_direct_dep_ids: test_package_direct_dep_ids(
                "core".to_string(),
                Vec::new(),
                Vec::new(),
            ),
            modules: vec![IrPackageModule {
                package_id: test_package_id_for_module("core"),
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
            foreword_index: Vec::new(),
            foreword_registrations: Vec::new(),
            entrypoints: Vec::new(),
            routines: Vec::new(),
            native_callbacks: Vec::new(),
            shackle_decls: Vec::new(),
            owners: Vec::new(),
        }
    }

    fn test_package_id_for_module(module_id: &str) -> String {
        module_id.split('.').next().unwrap_or(module_id).to_string()
    }

    fn test_package_display_names_with_deps(
        package_id: impl Into<String>,
        package_name: impl Into<String>,
        direct_deps: Vec<String>,
        direct_dep_ids: Vec<String>,
    ) -> BTreeMap<String, String> {
        let mut names = BTreeMap::from([(package_id.into(), package_name.into())]);
        for (dep_name, dep_id) in direct_deps.into_iter().zip(direct_dep_ids) {
            names.entry(dep_id).or_insert(dep_name);
        }
        names
    }

    fn test_package_direct_dep_ids(
        package_id: impl Into<String>,
        direct_deps: Vec<String>,
        direct_dep_ids: Vec<String>,
    ) -> BTreeMap<String, BTreeMap<String, String>> {
        BTreeMap::from([(
            package_id.into(),
            direct_deps.into_iter().zip(direct_dep_ids).collect(),
        )])
    }

    fn test_emit_context(file_name: &str) -> AotEmitContext {
        AotEmitContext {
            root_artifact_file_name: Some(file_name.to_string()),
            runtime_binding: AotRuntimeBinding::Baked,
            native_product: None,
        }
    }

    #[test]
    fn lowering_marks_simple_main_as_direct() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![ExecStmt::ReturnValue {
                value: ExecExpr::Int(9),
            }],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
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
            lowering_plan.direct_routines[0].body.return_expr,
            NativeDirectExpr::Int(9)
        );
    }

    #[test]
    fn lowering_keeps_foreworded_routines_in_direct_lane_and_records_hints() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: true,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![ExecStmt::ReturnValue {
                value: ExecExpr::Int(1),
            }],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert!(matches!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                lowering: NativeRoutineLowering::Direct { .. },
                ..
            }
        ));
        assert_eq!(lowering_plan.direct_routines.len(), 1);
        assert!(lowering_plan.direct_routines[0].inline_hint);
        assert!(!lowering_plan.direct_routines[0].cold_hint);
    }

    #[test]
    fn lowering_keeps_bool_recycle_regions_direct() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-recycle".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Recycle {
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "return".to_string(),
                        payload: Some(ExecExpr::Int(0)),
                    }),
                    lines: vec![ExecRecycleLine {
                        kind: ExecRecycleLineKind::Expr {
                            gate: ExecExpr::Bool(false),
                        },
                        modifier: None,
                    }],
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-recycle".to_string(),
                lowering: NativeRoutineLowering::Direct {
                    routine_key: "core#fn-recycle".to_string(),
                },
            }
        );
        let main = &lowering_plan.direct_routines[0];
        let NativeDirectStmt::If {
            condition,
            then_body,
            else_body,
        } = &main.body.statements[0]
        else {
            panic!("expected lowered recycle guard");
        };
        assert_eq!(condition, &NativeDirectExpr::Bool(false));
        assert!(then_body.is_empty());
        assert_eq!(
            else_body,
            &vec![NativeDirectStmt::Return {
                value: NativeDirectExpr::Int(0),
            }]
        );
        assert_eq!(main.body.return_expr, NativeDirectExpr::Int(9));
    }

    #[test]
    fn lowering_keeps_bind_require_regions_direct() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-bind".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Bind {
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "return".to_string(),
                        payload: Some(ExecExpr::Int(0)),
                    }),
                    lines: vec![ExecBindLine {
                        kind: ExecBindLineKind::Require {
                            expr: ExecExpr::Bool(false),
                        },
                        modifier: None,
                    }],
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-bind".to_string(),
                lowering: NativeRoutineLowering::Direct {
                    routine_key: "core#fn-bind".to_string(),
                },
            }
        );
        let main = &lowering_plan.direct_routines[0];
        let NativeDirectStmt::If { else_body, .. } = &main.body.statements[0] else {
            panic!("expected lowered bind guard");
        };
        assert_eq!(
            else_body,
            &vec![NativeDirectStmt::Return {
                value: NativeDirectExpr::Int(0),
            }]
        );
        assert_eq!(main.body.return_expr, NativeDirectExpr::Int(9));
    }

    #[test]
    fn lowering_keeps_bind_require_loop_exit_regions_direct() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-bind-loop".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Let {
                    binding_id: 0,
                    mutable: true,
                    name: "i".to_string(),
                    value: ExecExpr::Int(0),
                },
                ExecStmt::Let {
                    binding_id: 1,
                    mutable: true,
                    name: "sum".to_string(),
                    value: ExecExpr::Int(0),
                },
                ExecStmt::While {
                    condition: ExecExpr::Binary {
                        left: Box::new(ExecExpr::Path(vec!["i".to_string()])),
                        op: arcana_ir::ExecBinaryOp::Lt,
                        right: Box::new(ExecExpr::Int(5)),
                    },
                    body: vec![
                        ExecStmt::Assign {
                            target: ExecAssignTarget::Name("i".to_string()),
                            op: ExecAssignOp::AddAssign,
                            value: ExecExpr::Int(1),
                        },
                        ExecStmt::Bind {
                            default_modifier: Some(ExecHeadedModifier {
                                kind: "continue".to_string(),
                                payload: None,
                            }),
                            lines: vec![ExecBindLine {
                                kind: ExecBindLineKind::Require {
                                    expr: ExecExpr::Binary {
                                        left: Box::new(ExecExpr::Path(vec!["i".to_string()])),
                                        op: arcana_ir::ExecBinaryOp::NotEq,
                                        right: Box::new(ExecExpr::Int(3)),
                                    },
                                },
                                modifier: None,
                            }],
                        },
                        ExecStmt::Assign {
                            target: ExecAssignTarget::Name("sum".to_string()),
                            op: ExecAssignOp::AddAssign,
                            value: ExecExpr::Path(vec!["i".to_string()]),
                        },
                        ExecStmt::Bind {
                            default_modifier: Some(ExecHeadedModifier {
                                kind: "break".to_string(),
                                payload: None,
                            }),
                            lines: vec![ExecBindLine {
                                kind: ExecBindLineKind::Require {
                                    expr: ExecExpr::Binary {
                                        left: Box::new(ExecExpr::Path(vec!["sum".to_string()])),
                                        op: arcana_ir::ExecBinaryOp::LtEq,
                                        right: Box::new(ExecExpr::Int(6)),
                                    },
                                },
                                modifier: None,
                            }],
                        },
                    ],
                    cleanup_footers: Vec::new(),
                    availability: Vec::new(),
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["sum".to_string()]),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-bind-loop".to_string(),
                lowering: NativeRoutineLowering::Direct {
                    routine_key: "core#fn-bind-loop".to_string(),
                },
            }
        );
        let main = &lowering_plan.direct_routines[0];
        let NativeDirectStmt::While { body, .. } = &main.body.statements[2] else {
            panic!("expected lowered while body");
        };
        let NativeDirectStmt::If { else_body, .. } = &body[1] else {
            panic!("expected lowered bind-continue guard");
        };
        assert_eq!(else_body, &vec![NativeDirectStmt::Continue]);
        let NativeDirectStmt::If { else_body, .. } = &body[3] else {
            panic!("expected lowered bind-break guard");
        };
        assert_eq!(else_body, &vec![NativeDirectStmt::Break]);
        assert_eq!(
            main.body.return_expr,
            NativeDirectExpr::Binding("sum".to_string())
        );
    }

    #[test]
    fn lowering_ignores_memory_specs_in_direct_subset() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-memory".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::MemorySpec(ExecMemorySpecDecl {
                    family: "arena".to_string(),
                    name: "cache".to_string(),
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "alloc".to_string(),
                        payload: None,
                    }),
                    details: Vec::new(),
                }),
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-memory".to_string(),
                lowering: NativeRoutineLowering::Direct {
                    routine_key: "core#fn-memory".to_string(),
                },
            }
        );
        assert!(lowering_plan.direct_routines[0].body.statements.is_empty());
        assert_eq!(
            lowering_plan.direct_routines[0].body.return_expr,
            NativeDirectExpr::Int(9)
        );
    }

    #[test]
    fn lowering_marks_construct_regions_as_runtime_dispatch() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-construct".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Let {
                    binding_id: 0,
                    mutable: false,
                    name: "built".to_string(),
                    value: ExecExpr::ConstructRegion(Box::new(ExecConstructRegion {
                        completion: "yield".to_string(),
                        target: Box::new(ExecExpr::Path(vec![
                            "core".to_string(),
                            "Widget".to_string(),
                        ])),
                        destination: None,
                        default_modifier: Some(ExecHeadedModifier {
                            kind: "return".to_string(),
                            payload: Some(ExecExpr::Int(0)),
                        }),
                        lines: vec![ExecConstructLine {
                            name: "value".to_string(),
                            value: ExecExpr::Int(1),
                            mode: ExecConstructContributionMode::Direct,
                            modifier: None,
                        }],
                    })),
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-construct".to_string(),
                lowering: NativeRoutineLowering::RuntimeDispatch,
            }
        );
        assert!(lowering_plan.direct_routines.is_empty());
    }

    #[test]
    fn lowering_marks_construct_deliver_regions_as_runtime_dispatch() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-construct-deliver".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Construct(ExecConstructRegion {
                    completion: "deliver".to_string(),
                    target: Box::new(ExecExpr::Path(vec![
                        "core".to_string(),
                        "Widget".to_string(),
                    ])),
                    destination: Some(ExecConstructDestination::Deliver {
                        name: "built".to_string(),
                    }),
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "return".to_string(),
                        payload: Some(ExecExpr::Int(0)),
                    }),
                    lines: vec![ExecConstructLine {
                        name: "value".to_string(),
                        value: ExecExpr::Int(1),
                        mode: ExecConstructContributionMode::Direct,
                        modifier: None,
                    }],
                }),
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-construct-deliver".to_string(),
                lowering: NativeRoutineLowering::RuntimeDispatch,
            }
        );
        assert!(lowering_plan.direct_routines.is_empty());
    }

    #[test]
    fn lowering_marks_construct_place_regions_as_runtime_dispatch() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-construct-place".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Let {
                    binding_id: 0,
                    mutable: true,
                    name: "built".to_string(),
                    value: ExecExpr::Int(1),
                },
                ExecStmt::Construct(ExecConstructRegion {
                    completion: "place".to_string(),
                    target: Box::new(ExecExpr::Path(vec![
                        "core".to_string(),
                        "Widget".to_string(),
                    ])),
                    destination: Some(ExecConstructDestination::Place {
                        target: ExecAssignTarget::Name("built".to_string()),
                    }),
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "return".to_string(),
                        payload: Some(ExecExpr::Int(0)),
                    }),
                    lines: vec![ExecConstructLine {
                        name: "value".to_string(),
                        value: ExecExpr::Int(1),
                        mode: ExecConstructContributionMode::Direct,
                        modifier: None,
                    }],
                }),
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-construct-place".to_string(),
                lowering: NativeRoutineLowering::RuntimeDispatch,
            }
        );
        assert!(lowering_plan.direct_routines.is_empty());
    }

    #[test]
    fn lowering_marks_named_owner_exit_recycle_regions_as_runtime_dispatch() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-recycle-owner-exit".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Recycle {
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "done".to_string(),
                        payload: None,
                    }),
                    lines: vec![ExecRecycleLine {
                        kind: ExecRecycleLineKind::Expr {
                            gate: ExecExpr::Bool(false),
                        },
                        modifier: None,
                    }],
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-recycle-owner-exit".to_string(),
                lowering: NativeRoutineLowering::RuntimeDispatch,
            }
        );
        assert!(lowering_plan.direct_routines.is_empty());
    }

    #[test]
    fn lowering_marks_payload_bind_regions_as_runtime_dispatch() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-bind-payload".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Bind {
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "return".to_string(),
                        payload: Some(ExecExpr::Int(0)),
                    }),
                    lines: vec![ExecBindLine {
                        kind: ExecBindLineKind::Let {
                            mutable: false,
                            name: "value".to_string(),
                            gate: ExecExpr::Bool(true),
                        },
                        modifier: None,
                    }],
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-bind-payload".to_string(),
                lowering: NativeRoutineLowering::RuntimeDispatch,
            }
        );
        assert!(lowering_plan.direct_routines.is_empty());
    }

    #[test]
    fn lowering_marks_payload_recycle_regions_as_runtime_dispatch() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-recycle-payload".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Recycle {
                    default_modifier: Some(ExecHeadedModifier {
                        kind: "return".to_string(),
                        payload: Some(ExecExpr::Int(0)),
                    }),
                    lines: vec![ExecRecycleLine {
                        kind: ExecRecycleLineKind::Let {
                            mutable: false,
                            name: "value".to_string(),
                            gate: ExecExpr::Bool(true),
                        },
                        modifier: None,
                    }],
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Int(9),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                main_routine_key: "core#fn-recycle-payload".to_string(),
                lowering: NativeRoutineLowering::RuntimeDispatch,
            }
        );
        assert!(lowering_plan.direct_routines.is_empty());
    }

    #[test]
    fn lowering_marks_resolved_helper_calls_as_direct() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: vec![ExecPhraseArg {
                            name: None,
                            value: ExecExpr::Int(9),
                        }],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn helper(value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["value".to_string()]),
                }],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
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
                && routine.body.return_expr == NativeDirectExpr::Binding("value".to_string())
        }));
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-0"
                && matches!(
                    &routine.body.return_expr,
                    NativeDirectExpr::Call { routine_key, args, .. }
                        if routine_key == "core#fn-1"
                            && args == &vec![NativeDirectExpr::Int(9)]
                )
        }));
    }

    #[test]
    fn lowering_splits_direct_exports_from_runtime_fallbacks() {
        let mut package = base_package();
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "answer".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn answer() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(11),
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "greet".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=read:name=name:ty=Str".to_string()]),
                return_type: test_return_type("fn greet(read name: Str) -> Str:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Binary {
                        left: Box::new(ExecExpr::Str("hi ".to_string())),
                        op: arcana_ir::ExecBinaryOp::Add,
                        right: Box::new(ExecExpr::Path(vec!["name".to_string()])),
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-2".to_string(),
                symbol_name: "prefix".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=read:name=bytes:ty=Array[Int]".to_string()]),
                return_type: test_return_type("fn prefix(read bytes: Array[Int]) -> Array[Int]:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["std".to_string()])),
                        args: Vec::new(),
                        qualifier_kind: arcana_ir::ExecPhraseQualifierKind::Call,
                        qualifier: "bytes".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: None,
                        resolved_routine: None,
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-3".to_string(),
                symbol_name: "echo_pair".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=read:name=pair:ty=Pair[Str, Int]".to_string()]),
                return_type: test_return_type(
                    "fn echo_pair(read pair: Pair[Str, Int]) -> Pair[Str, Int]:",
                ),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["pair".to_string()]),
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-4".to_string(),
                symbol_name: "answer_via_helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn answer_via_helper() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: Vec::new(),
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-5".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-5".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn helper() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(21),
                }],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
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
                && matches!(
                    &routine.body.return_expr,
                    NativeDirectExpr::Call { routine_key, args, .. }
                        if routine_key == "core#fn-5" && args.is_empty()
                )
        }));
    }

    #[test]
    fn lowering_directly_exports_edit_root_routines() {
        let mut package = base_package();
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-0".to_string(),
            symbol_name: "bump".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: test_params(&["mode=edit:name=value:ty=Int".to_string()]),
            return_type: test_return_type("fn bump(edit value: Int) -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Assign {
                    target: ExecAssignTarget::Name("value".to_string()),
                    op: ExecAssignOp::AddAssign,
                    value: ExecExpr::Int(1),
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["value".to_string()]),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let NativeLaunchLowering::DynamicLibrary { exports } = lowering_plan.launch else {
            panic!("expected dynamic-library lowering");
        };
        assert_eq!(exports.len(), 1);
        assert!(matches!(
            exports[0].lowering,
            NativeRoutineLowering::Direct { ref routine_key } if routine_key == "core#fn-0"
        ));
        assert_eq!(lowering_plan.direct_routines.len(), 1);
        assert_eq!(
            lowering_plan.direct_routines[0].body.statements,
            vec![NativeDirectStmt::Assign {
                name: "value".to_string(),
                value: NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Add,
                    left: Box::new(NativeDirectExpr::Binding("value".to_string())),
                    right: Box::new(NativeDirectExpr::Int(1)),
                },
            }]
        );
        assert_eq!(
            lowering_plan.direct_routines[0].body.return_expr,
            NativeDirectExpr::Binding("value".to_string())
        );
    }

    #[test]
    fn lowering_supports_direct_edit_write_back_call_chains() {
        let mut package = base_package();
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "outer".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=edit:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn outer(edit value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: vec![ExecPhraseArg {
                            name: None,
                            value: ExecExpr::Path(vec!["value".to_string()]),
                        }],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=edit:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn helper(edit value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![
                    ExecStmt::Assign {
                        target: ExecAssignTarget::Name("value".to_string()),
                        op: ExecAssignOp::AddAssign,
                        value: ExecExpr::Int(2),
                    },
                    ExecStmt::ReturnValue {
                        value: ExecExpr::Path(vec!["value".to_string()]),
                    },
                ],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let NativeLaunchLowering::DynamicLibrary { exports } = lowering_plan.launch else {
            panic!("expected dynamic-library lowering");
        };
        assert_eq!(exports.len(), 1);
        assert!(matches!(
            exports[0].lowering,
            NativeRoutineLowering::Direct { ref routine_key } if routine_key == "core#fn-0"
        ));
        assert_eq!(lowering_plan.direct_routines.len(), 2);
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-0"
                && matches!(
                    &routine.body.return_expr,
                    NativeDirectExpr::Call { routine_key, args, .. }
                        if routine_key == "core#fn-1"
                            && args == &vec![NativeDirectExpr::Binding("value".to_string())]
                )
        }));
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-1"
                && routine.body.statements
                    == vec![NativeDirectStmt::Assign {
                        name: "value".to_string(),
                        value: NativeDirectExpr::IntBinary {
                            op: NativeDirectIntBinaryOp::Add,
                            left: Box::new(NativeDirectExpr::Binding("value".to_string())),
                            right: Box::new(NativeDirectExpr::Int(2)),
                        },
                    }]
                && routine.body.return_expr == NativeDirectExpr::Binding("value".to_string())
        }));
    }

    #[test]
    fn lowering_keeps_non_name_edit_write_back_targets_on_runtime_dispatch() {
        let mut package = base_package();
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-0".to_string(),
            symbol_name: "touch_first".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: test_params(&["mode=edit:name=bytes:ty=Array[Int]".to_string()]),
            return_type: test_return_type("fn touch_first(edit bytes: Array[Int]) -> Array[Int]:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Assign {
                    target: ExecAssignTarget::Index {
                        target: Box::new(ExecAssignTarget::Name("bytes".to_string())),
                        index: ExecExpr::Int(0),
                    },
                    op: ExecAssignOp::Assign,
                    value: ExecExpr::Int(1),
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["bytes".to_string()]),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let NativeLaunchLowering::DynamicLibrary { exports } = lowering_plan.launch else {
            panic!("expected dynamic-library lowering");
        };
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].lowering, NativeRoutineLowering::RuntimeDispatch);
        assert!(
            lowering_plan.direct_routines.is_empty(),
            "non-name edit write-back targets should stay outside the current direct subset"
        );
    }

    #[test]
    fn lowering_supports_simple_let_blocks() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Let {
                    binding_id: 0,
                    mutable: false,
                    name: "value".to_string(),
                    value: ExecExpr::Int(9),
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["value".to_string()]),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(lowering_plan.direct_routines.len(), 1);
        assert_eq!(
            lowering_plan.direct_routines[0].body.statements,
            vec![NativeDirectStmt::Let {
                mutable: false,
                name: "value".to_string(),
                value: NativeDirectExpr::Int(9),
            }]
        );
        assert_eq!(
            lowering_plan.direct_routines[0].body.return_expr,
            NativeDirectExpr::Binding("value".to_string())
        );
    }

    #[test]
    fn lowering_supports_terminal_if_and_int_ops() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![
                    ExecStmt::Let {
                        binding_id: 0,
                        mutable: false,
                        name: "base".to_string(),
                        value: ExecExpr::Int(8),
                    },
                    ExecStmt::If {
                        condition: ExecExpr::Binary {
                            left: Box::new(ExecExpr::Path(vec!["base".to_string()])),
                            op: arcana_ir::ExecBinaryOp::GtEq,
                            right: Box::new(ExecExpr::Int(8)),
                        },
                        then_branch: vec![ExecStmt::ReturnValue {
                            value: ExecExpr::Phrase {
                                subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                                args: vec![ExecPhraseArg {
                                    name: None,
                                    value: ExecExpr::Path(vec!["base".to_string()]),
                                }],
                                qualifier_kind: ExecPhraseQualifierKind::Call,
                                qualifier: "call".to_string(),
                                qualifier_type_args: Vec::new(),
                                resolved_callable: Some(vec![
                                    "core".to_string(),
                                    "helper".to_string(),
                                ]),
                                resolved_routine: Some("core#fn-1".to_string()),
                                dynamic_dispatch: None,
                                attached: Vec::new(),
                            },
                        }],
                        else_branch: vec![ExecStmt::ReturnValue {
                            value: ExecExpr::Int(0),
                        }],
                        cleanup_footers: Vec::new(),
                        availability: Vec::new(),
                    },
                ],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn helper(value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![
                    ExecStmt::Let {
                        binding_id: 0,
                        mutable: false,
                        name: "bumped".to_string(),
                        value: ExecExpr::Binary {
                            left: Box::new(ExecExpr::Path(vec!["value".to_string()])),
                            op: arcana_ir::ExecBinaryOp::Add,
                            right: Box::new(ExecExpr::Int(1)),
                        },
                    },
                    ExecStmt::ReturnValue {
                        value: ExecExpr::Path(vec!["bumped".to_string()]),
                    },
                ],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(lowering_plan.direct_routines.len(), 2);
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-1"
                && routine.body.statements
                    == vec![NativeDirectStmt::Let {
                        mutable: false,
                        name: "bumped".to_string(),
                        value: NativeDirectExpr::IntBinary {
                            op: NativeDirectIntBinaryOp::Add,
                            left: Box::new(NativeDirectExpr::Binding("value".to_string())),
                            right: Box::new(NativeDirectExpr::Int(1)),
                        },
                    }]
                && routine.body.return_expr == NativeDirectExpr::Binding("bumped".to_string())
        }));
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            if routine.routine_key != "core#fn-0" {
                return false;
            }
            let NativeDirectExpr::If {
                condition,
                then_block,
                else_block,
            } = &routine.body.return_expr
            else {
                return false;
            };
            if condition.as_ref()
                != &(NativeDirectExpr::IntCompare {
                    op: NativeDirectIntCompareOp::GtEq,
                    left: Box::new(NativeDirectExpr::Binding("base".to_string())),
                    right: Box::new(NativeDirectExpr::Int(8)),
                })
            {
                return false;
            }
            if !then_block.statements.is_empty() {
                return false;
            }
            if else_block.as_ref()
                != &(super::NativeDirectBlock {
                    statements: Vec::new(),
                    return_expr: NativeDirectExpr::Int(0),
                })
            {
                return false;
            }
            matches!(
                &then_block.return_expr,
                NativeDirectExpr::Call { routine_key, args, .. }
                    if routine_key == "core#fn-1"
                        && args == &vec![NativeDirectExpr::Binding("base".to_string())]
            )
        }));
    }

    #[test]
    fn lowering_supports_named_calls_but_keeps_attached_calls_on_runtime_dispatch() {
        let mut package = base_package();
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "named_call".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn named_call(value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: vec![ExecPhraseArg {
                            name: Some("value".to_string()),
                            value: ExecExpr::Path(vec!["value".to_string()]),
                        }],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn helper(value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["value".to_string()]),
                }],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let NativeLaunchLowering::DynamicLibrary { exports } = lowering_plan.launch else {
            panic!("expected dynamic-library lowering");
        };
        assert_eq!(exports.len(), 1);
        assert!(matches!(
            exports[0].lowering,
            NativeRoutineLowering::Direct { ref routine_key } if routine_key == "core#fn-0"
        ));
    }

    #[test]
    fn lowering_keeps_attached_calls_on_runtime_dispatch() {
        let mut package = base_package();
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "attached_call".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn attached_call(value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["helper".to_string()])),
                        args: vec![ExecPhraseArg {
                            name: None,
                            value: ExecExpr::Path(vec!["value".to_string()]),
                        }],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: Some(vec!["core".to_string(), "helper".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: vec![arcana_ir::ExecHeaderAttachment::Named {
                            name: "trace".to_string(),
                            value: ExecExpr::Bool(true),
                        }],
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "helper".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn helper(value: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["value".to_string()]),
                }],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsDllBundle,
            &package,
            &test_emit_context("lib.dll"),
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

    #[test]
    fn lowering_supports_while_mutation_and_loop_control() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.push(IrRoutine {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            routine_key: "core#fn-0".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: Vec::new(),
            return_type: test_return_type("fn main() -> Int:"),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            inline_hint: false,
            cold_hint: false,
            cleanup_footers: Vec::new(),
            statements: vec![
                ExecStmt::Let {
                    binding_id: 0,
                    mutable: true,
                    name: "i".to_string(),
                    value: ExecExpr::Int(0),
                },
                ExecStmt::Let {
                    binding_id: 0,
                    mutable: true,
                    name: "sum".to_string(),
                    value: ExecExpr::Int(0),
                },
                ExecStmt::While {
                    condition: ExecExpr::Binary {
                        left: Box::new(ExecExpr::Path(vec!["i".to_string()])),
                        op: arcana_ir::ExecBinaryOp::Lt,
                        right: Box::new(ExecExpr::Int(5)),
                    },
                    body: vec![
                        ExecStmt::Assign {
                            target: ExecAssignTarget::Name("i".to_string()),
                            op: ExecAssignOp::AddAssign,
                            value: ExecExpr::Int(1),
                        },
                        ExecStmt::If {
                            condition: ExecExpr::Binary {
                                left: Box::new(ExecExpr::Path(vec!["i".to_string()])),
                                op: arcana_ir::ExecBinaryOp::EqEq,
                                right: Box::new(ExecExpr::Int(3)),
                            },
                            then_branch: vec![ExecStmt::Continue],
                            else_branch: Vec::new(),
                            cleanup_footers: Vec::new(),
                            availability: Vec::new(),
                        },
                        ExecStmt::Assign {
                            target: ExecAssignTarget::Name("sum".to_string()),
                            op: ExecAssignOp::AddAssign,
                            value: ExecExpr::Path(vec!["i".to_string()]),
                        },
                        ExecStmt::If {
                            condition: ExecExpr::Binary {
                                left: Box::new(ExecExpr::Path(vec!["sum".to_string()])),
                                op: arcana_ir::ExecBinaryOp::Gt,
                                right: Box::new(ExecExpr::Int(6)),
                            },
                            then_branch: vec![ExecStmt::Break],
                            else_branch: Vec::new(),
                            cleanup_footers: Vec::new(),
                            availability: Vec::new(),
                        },
                    ],
                    cleanup_footers: Vec::new(),
                    availability: Vec::new(),
                },
                ExecStmt::ReturnValue {
                    value: ExecExpr::Path(vec!["sum".to_string()]),
                },
            ],
        });

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(lowering_plan.direct_routines.len(), 1);
        assert_eq!(
            lowering_plan.direct_routines[0].body.statements,
            vec![
                NativeDirectStmt::Let {
                    mutable: true,
                    name: "i".to_string(),
                    value: NativeDirectExpr::Int(0),
                },
                NativeDirectStmt::Let {
                    mutable: true,
                    name: "sum".to_string(),
                    value: NativeDirectExpr::Int(0),
                },
                NativeDirectStmt::While {
                    condition: NativeDirectExpr::IntCompare {
                        op: NativeDirectIntCompareOp::Lt,
                        left: Box::new(NativeDirectExpr::Binding("i".to_string())),
                        right: Box::new(NativeDirectExpr::Int(5)),
                    },
                    body: vec![
                        NativeDirectStmt::Assign {
                            name: "i".to_string(),
                            value: NativeDirectExpr::IntBinary {
                                op: NativeDirectIntBinaryOp::Add,
                                left: Box::new(NativeDirectExpr::Binding("i".to_string())),
                                right: Box::new(NativeDirectExpr::Int(1)),
                            },
                        },
                        NativeDirectStmt::If {
                            condition: NativeDirectExpr::IntCompare {
                                op: NativeDirectIntCompareOp::Eq,
                                left: Box::new(NativeDirectExpr::Binding("i".to_string())),
                                right: Box::new(NativeDirectExpr::Int(3)),
                            },
                            then_body: vec![NativeDirectStmt::Continue],
                            else_body: Vec::new(),
                        },
                        NativeDirectStmt::Assign {
                            name: "sum".to_string(),
                            value: NativeDirectExpr::IntBinary {
                                op: NativeDirectIntBinaryOp::Add,
                                left: Box::new(NativeDirectExpr::Binding("sum".to_string())),
                                right: Box::new(NativeDirectExpr::Binding("i".to_string())),
                            },
                        },
                        NativeDirectStmt::If {
                            condition: NativeDirectExpr::IntCompare {
                                op: NativeDirectIntCompareOp::Gt,
                                left: Box::new(NativeDirectExpr::Binding("sum".to_string())),
                                right: Box::new(NativeDirectExpr::Int(6)),
                            },
                            then_body: vec![NativeDirectStmt::Break],
                            else_body: Vec::new(),
                        },
                    ],
                },
            ]
        );
        assert_eq!(
            lowering_plan.direct_routines[0].body.return_expr,
            NativeDirectExpr::Binding("sum".to_string())
        );
    }

    #[test]
    fn lowering_supports_statement_calls_and_early_return_in_if() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![
                    ExecStmt::Expr {
                        expr: ExecExpr::Phrase {
                            subject: Box::new(ExecExpr::Path(vec!["touch".to_string()])),
                            args: Vec::new(),
                            qualifier_kind: ExecPhraseQualifierKind::Call,
                            qualifier: "call".to_string(),
                            qualifier_type_args: Vec::new(),
                            resolved_callable: Some(vec!["core".to_string(), "touch".to_string()]),
                            resolved_routine: Some("core#fn-1".to_string()),
                            dynamic_dispatch: None,
                            attached: Vec::new(),
                        },
                        cleanup_footers: Vec::new(),
                    },
                    ExecStmt::If {
                        condition: ExecExpr::Bool(true),
                        then_branch: vec![ExecStmt::ReturnValue {
                            value: ExecExpr::Int(9),
                        }],
                        else_branch: Vec::new(),
                        cleanup_footers: Vec::new(),
                        availability: Vec::new(),
                    },
                    ExecStmt::ReturnValue {
                        value: ExecExpr::Int(0),
                    },
                ],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "touch".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn touch():"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnVoid],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(lowering_plan.direct_routines.len(), 2);
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-1" && routine.body.return_expr == NativeDirectExpr::Unit
        }));
        assert!(lowering_plan.direct_routines.iter().any(|routine| {
            routine.routine_key == "core#fn-0"
                && routine.body.statements
                    == vec![
                        NativeDirectStmt::Expr {
                            value: NativeDirectExpr::Call {
                                routine_key: "core#fn-1".to_string(),
                                params: Vec::new(),
                                args: Vec::new(),
                            },
                        },
                        NativeDirectStmt::If {
                            condition: NativeDirectExpr::Bool(true),
                            then_body: vec![NativeDirectStmt::Return {
                                value: NativeDirectExpr::Int(9),
                            }],
                            else_body: Vec::new(),
                        },
                    ]
                && routine.body.return_expr == NativeDirectExpr::Int(0)
        }));
    }

    #[test]
    fn lowering_supports_defer_cleanup_on_routine_exit() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![
                    ExecStmt::Let {
                        binding_id: 0,
                        mutable: true,
                        name: "value".to_string(),
                        value: ExecExpr::Int(1),
                    },
                    ExecStmt::Defer(ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["touch".to_string()])),
                        args: vec![ExecPhraseArg {
                            name: None,
                            value: ExecExpr::Path(vec!["value".to_string()]),
                        }],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: Some(vec!["core".to_string(), "touch".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    }),
                    ExecStmt::Assign {
                        target: ExecAssignTarget::Name("value".to_string()),
                        op: ExecAssignOp::AddAssign,
                        value: ExecExpr::Int(2),
                    },
                    ExecStmt::ReturnValue {
                        value: ExecExpr::Path(vec!["value".to_string()]),
                    },
                ],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "touch".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn touch(value: Int):"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnVoid],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert_eq!(lowering_plan.direct_routines.len(), 2);
        let main = lowering_plan
            .direct_routines
            .iter()
            .find(|routine| routine.routine_key == "core#fn-0")
            .expect("main routine should lower directly");
        assert_eq!(main.body.statements.len(), 4);
        assert_eq!(
            main.body.statements[0],
            NativeDirectStmt::Let {
                mutable: true,
                name: "value".to_string(),
                value: NativeDirectExpr::Int(1),
            }
        );
        assert_eq!(
            main.body.statements[1],
            NativeDirectStmt::Assign {
                name: "value".to_string(),
                value: NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Add,
                    left: Box::new(NativeDirectExpr::Binding("value".to_string())),
                    right: Box::new(NativeDirectExpr::Int(2)),
                },
            }
        );
        let temp_name = match &main.body.statements[2] {
            NativeDirectStmt::Let {
                mutable,
                name,
                value,
            } => {
                assert!(!mutable);
                assert!(name.starts_with("__arcana_defer_return_"));
                assert_eq!(value, &NativeDirectExpr::Binding("value".to_string()));
                name.clone()
            }
            other => panic!("expected defer temp let, got {other:?}"),
        };
        match &main.body.statements[3] {
            NativeDirectStmt::Expr {
                value:
                    NativeDirectExpr::Call {
                        routine_key, args, ..
                    },
            } => {
                assert_eq!(routine_key, "core#fn-1");
                assert_eq!(args, &vec![NativeDirectExpr::Binding("value".to_string())]);
            }
            other => panic!("expected deferred cleanup call, got {other:?}"),
        }
        assert_eq!(main.body.return_expr, NativeDirectExpr::Binding(temp_name));
    }

    #[test]
    fn lowering_keeps_cleanup_footer_in_direct_lane_with_runtime_dispatched_handler() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: vec![ExecCleanupFooter {
                    binding_id: 0,
                    kind: "cleanup".to_string(),
                    subject: "value".to_string(),
                    handler_path: vec!["cleanup".to_string()],
                    resolved_routine: Some("core#fn-1".to_string()),
                }],
                statements: vec![
                    ExecStmt::Let {
                        binding_id: 0,
                        mutable: false,
                        name: "value".to_string(),
                        value: ExecExpr::Int(9),
                    },
                    ExecStmt::ReturnValue {
                        value: ExecExpr::Int(0),
                    },
                ],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "cleanup".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=take:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn cleanup(take value: Int) -> Result[Unit, Str]:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnVoid],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        assert!(matches!(
            lowering_plan.launch,
            NativeLaunchLowering::Executable {
                lowering: NativeRoutineLowering::Direct { .. },
                ..
            }
        ));
        let main = lowering_plan
            .direct_routines
            .iter()
            .find(|routine| routine.routine_key == "core#fn-0")
            .expect("main routine should lower directly");
        assert!(main.body.statements.iter().any(|stmt| {
            matches!(
                stmt,
                NativeDirectStmt::Cleanup {
                    action: NativeCleanupAction::RuntimeDispatch { routine_key, arg }
                } if routine_key == "core#fn-1"
                    && arg == &NativeDirectExpr::Binding("value".to_string())
            )
        }));
    }

    #[test]
    fn lowering_runs_param_cleanup_in_reverse_lexical_activation_order() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Phrase {
                        subject: Box::new(ExecExpr::Path(vec!["run".to_string()])),
                        args: vec![
                            ExecPhraseArg {
                                name: None,
                                value: ExecExpr::Int(1),
                            },
                            ExecPhraseArg {
                                name: None,
                                value: ExecExpr::Int(2),
                            },
                        ],
                        qualifier_kind: ExecPhraseQualifierKind::Call,
                        qualifier: "call".to_string(),
                        qualifier_type_args: Vec::new(),
                        resolved_callable: Some(vec!["core".to_string(), "run".to_string()]),
                        resolved_routine: Some("core#fn-1".to_string()),
                        dynamic_dispatch: None,
                        attached: Vec::new(),
                    },
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "run".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&[
                    "mode=take:name=zebra:ty=Int".to_string(),
                    "mode=take:name=alpha:ty=Int".to_string(),
                ]),
                return_type: test_return_type("fn run(take zebra: Int, take alpha: Int) -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: vec![
                    ExecCleanupFooter {
                        binding_id: 0,
                        kind: "cleanup".to_string(),
                        subject: "zebra".to_string(),
                        handler_path: vec!["cleanup".to_string()],
                        resolved_routine: Some("core#fn-2".to_string()),
                    },
                    ExecCleanupFooter {
                        binding_id: 0,
                        kind: "cleanup".to_string(),
                        subject: "alpha".to_string(),
                        handler_path: vec!["cleanup".to_string()],
                        resolved_routine: Some("core#fn-2".to_string()),
                    },
                ],
                statements: vec![ExecStmt::ReturnValue {
                    value: ExecExpr::Int(0),
                }],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-2".to_string(),
                symbol_name: "cleanup".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=take:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn cleanup(take value: Int) -> Result[Unit, Str]:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnVoid],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let main = lowering_plan
            .direct_routines
            .iter()
            .find(|routine| routine.routine_key == "core#fn-1")
            .expect("run routine should lower directly");
        assert_eq!(main.body.statements.len(), 3);
        assert!(matches!(
            &main.body.statements[1],
            NativeDirectStmt::Cleanup {
                action: NativeCleanupAction::RuntimeDispatch { arg, .. }
            } if arg == &NativeDirectExpr::Binding("alpha".to_string())
        ));
        assert!(matches!(
            &main.body.statements[2],
            NativeDirectStmt::Cleanup {
                action: NativeCleanupAction::RuntimeDispatch { arg, .. }
            } if arg == &NativeDirectExpr::Binding("zebra".to_string())
        ));
    }

    #[test]
    fn lowering_runs_defer_before_early_return_inside_if() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![
                    ExecStmt::If {
                        condition: ExecExpr::Bool(true),
                        then_branch: vec![
                            ExecStmt::Let {
                                binding_id: 0,
                                mutable: false,
                                name: "value".to_string(),
                                value: ExecExpr::Int(9),
                            },
                            ExecStmt::Defer(ExecExpr::Phrase {
                                subject: Box::new(ExecExpr::Path(vec!["touch".to_string()])),
                                args: vec![ExecPhraseArg {
                                    name: None,
                                    value: ExecExpr::Path(vec!["value".to_string()]),
                                }],
                                qualifier_kind: ExecPhraseQualifierKind::Call,
                                qualifier: "call".to_string(),
                                qualifier_type_args: Vec::new(),
                                resolved_callable: Some(vec![
                                    "core".to_string(),
                                    "touch".to_string(),
                                ]),
                                resolved_routine: Some("core#fn-1".to_string()),
                                dynamic_dispatch: None,
                                attached: Vec::new(),
                            }),
                            ExecStmt::ReturnValue {
                                value: ExecExpr::Path(vec!["value".to_string()]),
                            },
                        ],
                        else_branch: Vec::new(),
                        cleanup_footers: Vec::new(),
                        availability: Vec::new(),
                    },
                    ExecStmt::ReturnValue {
                        value: ExecExpr::Int(0),
                    },
                ],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "touch".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn touch(value: Int):"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnVoid],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let main = lowering_plan
            .direct_routines
            .iter()
            .find(|routine| routine.routine_key == "core#fn-0")
            .expect("main routine should lower directly");
        assert_eq!(main.body.statements.len(), 1);
        let NativeDirectStmt::If {
            condition,
            then_body,
            else_body,
        } = &main.body.statements[0]
        else {
            panic!("expected leading if statement");
        };
        assert_eq!(condition, &NativeDirectExpr::Bool(true));
        assert!(else_body.is_empty());
        assert_eq!(then_body.len(), 4);
        assert_eq!(
            then_body[0],
            NativeDirectStmt::Let {
                mutable: false,
                name: "value".to_string(),
                value: NativeDirectExpr::Int(9),
            }
        );
        let temp_name = match &then_body[1] {
            NativeDirectStmt::Let {
                mutable,
                name,
                value,
            } => {
                assert!(!mutable);
                assert!(name.starts_with("__arcana_defer_return_"));
                assert_eq!(value, &NativeDirectExpr::Binding("value".to_string()));
                name.clone()
            }
            other => panic!("expected branch defer temp let, got {other:?}"),
        };
        match &then_body[2] {
            NativeDirectStmt::Expr {
                value:
                    NativeDirectExpr::Call {
                        routine_key, args, ..
                    },
            } => {
                assert_eq!(routine_key, "core#fn-1");
                assert_eq!(args, &vec![NativeDirectExpr::Binding("value".to_string())]);
            }
            other => panic!("expected branch deferred cleanup call, got {other:?}"),
        }
        assert_eq!(
            then_body[3],
            NativeDirectStmt::Return {
                value: NativeDirectExpr::Binding(temp_name),
            }
        );
        assert_eq!(main.body.return_expr, NativeDirectExpr::Int(0));
    }

    #[test]
    fn lowering_runs_loop_body_defers_before_continue() {
        let mut package = base_package();
        package.entrypoints.push(IrEntrypoint {
            package_id: test_package_id_for_module("core"),
            module_id: "core".to_string(),
            symbol_name: "main".to_string(),
            symbol_kind: "fn".to_string(),
            is_async: false,
            exported: true,
        });
        package.routines.extend([
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-0".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: Vec::new(),
                return_type: test_return_type("fn main() -> Int:"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![
                    ExecStmt::Let {
                        binding_id: 0,
                        mutable: true,
                        name: "i".to_string(),
                        value: ExecExpr::Int(0),
                    },
                    ExecStmt::Let {
                        binding_id: 0,
                        mutable: true,
                        name: "sum".to_string(),
                        value: ExecExpr::Int(0),
                    },
                    ExecStmt::While {
                        condition: ExecExpr::Binary {
                            left: Box::new(ExecExpr::Path(vec!["i".to_string()])),
                            op: ExecBinaryOp::Lt,
                            right: Box::new(ExecExpr::Int(3)),
                        },
                        body: vec![
                            ExecStmt::Defer(ExecExpr::Phrase {
                                subject: Box::new(ExecExpr::Path(vec!["touch".to_string()])),
                                args: vec![ExecPhraseArg {
                                    name: None,
                                    value: ExecExpr::Path(vec!["i".to_string()]),
                                }],
                                qualifier_kind: ExecPhraseQualifierKind::Call,
                                qualifier: "call".to_string(),
                                qualifier_type_args: Vec::new(),
                                resolved_callable: Some(vec![
                                    "core".to_string(),
                                    "touch".to_string(),
                                ]),
                                resolved_routine: Some("core#fn-1".to_string()),
                                dynamic_dispatch: None,
                                attached: Vec::new(),
                            }),
                            ExecStmt::If {
                                condition: ExecExpr::Binary {
                                    left: Box::new(ExecExpr::Path(vec!["i".to_string()])),
                                    op: ExecBinaryOp::EqEq,
                                    right: Box::new(ExecExpr::Int(1)),
                                },
                                then_branch: vec![
                                    ExecStmt::Assign {
                                        target: ExecAssignTarget::Name("i".to_string()),
                                        op: ExecAssignOp::AddAssign,
                                        value: ExecExpr::Int(1),
                                    },
                                    ExecStmt::Continue,
                                ],
                                else_branch: Vec::new(),
                                cleanup_footers: Vec::new(),
                                availability: Vec::new(),
                            },
                            ExecStmt::Assign {
                                target: ExecAssignTarget::Name("sum".to_string()),
                                op: ExecAssignOp::AddAssign,
                                value: ExecExpr::Path(vec!["i".to_string()]),
                            },
                            ExecStmt::Assign {
                                target: ExecAssignTarget::Name("i".to_string()),
                                op: ExecAssignOp::AddAssign,
                                value: ExecExpr::Int(1),
                            },
                        ],
                        cleanup_footers: Vec::new(),
                        availability: Vec::new(),
                    },
                    ExecStmt::ReturnValue {
                        value: ExecExpr::Path(vec!["sum".to_string()]),
                    },
                ],
            },
            IrRoutine {
                package_id: test_package_id_for_module("core"),
                module_id: "core".to_string(),
                routine_key: "core#fn-1".to_string(),
                symbol_name: "touch".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_params: Vec::new(),
                behavior_attrs: BTreeMap::new(),
                params: test_params(&["mode=:name=value:ty=Int".to_string()]),
                return_type: test_return_type("fn touch(value: Int):"),
                intrinsic_impl: None,
                native_impl: None,
                impl_target_type: None,
                impl_trait_path: None,
                availability: Vec::new(),
                inline_hint: false,
                cold_hint: false,
                cleanup_footers: Vec::new(),
                statements: vec![ExecStmt::ReturnVoid],
            },
        ]);

        sync_exported_function_surface_rows(&mut package);
        let package_plan = build_native_package_plan(
            AotEmitTarget::WindowsExeBundle,
            &package,
            &test_emit_context("app.exe"),
        )
        .expect("native package plan should build");
        let lowering_plan =
            build_native_lowering_plan(&package_plan).expect("native lowering should build");

        let main = lowering_plan
            .direct_routines
            .iter()
            .find(|routine| routine.routine_key == "core#fn-0")
            .expect("main routine should lower directly");
        assert_eq!(main.body.statements.len(), 3);
        let NativeDirectStmt::While { condition, body } = &main.body.statements[2] else {
            panic!("expected while statement");
        };
        assert_eq!(
            condition,
            &NativeDirectExpr::IntCompare {
                op: NativeDirectIntCompareOp::Lt,
                left: Box::new(NativeDirectExpr::Binding("i".to_string())),
                right: Box::new(NativeDirectExpr::Int(3)),
            }
        );
        assert_eq!(body.len(), 4);
        let NativeDirectStmt::If {
            condition,
            then_body,
            else_body,
        } = &body[0]
        else {
            panic!("expected loop guard if");
        };
        assert_eq!(
            condition,
            &NativeDirectExpr::IntCompare {
                op: NativeDirectIntCompareOp::Eq,
                left: Box::new(NativeDirectExpr::Binding("i".to_string())),
                right: Box::new(NativeDirectExpr::Int(1)),
            }
        );
        assert!(else_body.is_empty());
        assert_eq!(
            then_body[0],
            NativeDirectStmt::Assign {
                name: "i".to_string(),
                value: NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Add,
                    left: Box::new(NativeDirectExpr::Binding("i".to_string())),
                    right: Box::new(NativeDirectExpr::Int(1)),
                },
            }
        );
        match &then_body[1] {
            NativeDirectStmt::Expr {
                value:
                    NativeDirectExpr::Call {
                        routine_key, args, ..
                    },
            } => {
                assert_eq!(routine_key, "core#fn-1");
                assert_eq!(args, &vec![NativeDirectExpr::Binding("i".to_string())]);
            }
            other => panic!("expected loop continue cleanup call, got {other:?}"),
        }
        assert_eq!(then_body[2], NativeDirectStmt::Continue);
        assert_eq!(
            body[1],
            NativeDirectStmt::Assign {
                name: "sum".to_string(),
                value: NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Add,
                    left: Box::new(NativeDirectExpr::Binding("sum".to_string())),
                    right: Box::new(NativeDirectExpr::Binding("i".to_string())),
                },
            }
        );
        assert_eq!(
            body[2],
            NativeDirectStmt::Assign {
                name: "i".to_string(),
                value: NativeDirectExpr::IntBinary {
                    op: NativeDirectIntBinaryOp::Add,
                    left: Box::new(NativeDirectExpr::Binding("i".to_string())),
                    right: Box::new(NativeDirectExpr::Int(1)),
                },
            }
        );
        match &body[3] {
            NativeDirectStmt::Expr {
                value:
                    NativeDirectExpr::Call {
                        routine_key, args, ..
                    },
            } => {
                assert_eq!(routine_key, "core#fn-1");
                assert_eq!(args, &vec![NativeDirectExpr::Binding("i".to_string())]);
            }
            other => panic!("expected loop fallthrough cleanup call, got {other:?}"),
        }
        assert_eq!(
            main.body.return_expr,
            NativeDirectExpr::Binding("sum".to_string())
        );
    }
}
