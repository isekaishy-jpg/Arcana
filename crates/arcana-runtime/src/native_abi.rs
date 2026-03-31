// Runtime-facing projection of the cabi export contract for generated export shims.
use super::{
    RuntimeExecutionState, RuntimeHost, RuntimePackagePlan, RuntimeRoutinePlan, RuntimeValue,
    execute_routine_call_with_state, runtime_eval_message, validate_runtime_requirements_supported,
    variant_name_matches,
};
use arcana_ir::{IrRoutineType, IrRoutineTypeKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeAbiValue {
    Int(i64),
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
    Pair(Box<RuntimeAbiValue>, Box<RuntimeAbiValue>),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAbiWriteBack {
    pub index: usize,
    pub name: String,
    pub value: RuntimeAbiValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAbiExportOutcome {
    pub result: RuntimeAbiValue,
    pub write_backs: Vec<RuntimeAbiWriteBack>,
}

pub fn execute_exported_abi_routine(
    plan: &RuntimePackagePlan,
    routine_key: &str,
    args: Vec<RuntimeAbiValue>,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeAbiExportOutcome, String> {
    let (routine_index, routine) = plan
        .routines
        .iter()
        .enumerate()
        .find(|(_, routine)| native_abi_callable(routine) && routine.routine_key == routine_key)
        .ok_or_else(|| format!("abi routine `{routine_key}` is not exported or callable"))?;
    validate_runtime_requirements_supported(plan, host)?;
    let mut state = RuntimeExecutionState::default();
    let runtime_args = args
        .into_iter()
        .map(runtime_value_from_abi)
        .collect::<Vec<_>>();
    let outcome = execute_routine_call_with_state(
        plan,
        routine_index,
        Vec::new(),
        runtime_args,
        &[],
        &mut state,
        host,
        false,
    )
    .map_err(runtime_eval_message)?;
    if let Some(control) = outcome.control {
        return Err(runtime_eval_message(match control {
            super::FlowSignal::OwnerExit {
                owner_key,
                exit_name,
            } => super::RuntimeEvalSignal::OwnerExit {
                owner_key,
                exit_name,
            },
            other => super::RuntimeEvalSignal::Message(format!(
                "unsupported native abi control flow `{other:?}`"
            )),
        }));
    }
    Ok(RuntimeAbiExportOutcome {
        result: abi_value_from_runtime(outcome.value)?,
        write_backs: project_export_write_backs(routine, outcome.final_args)?,
    })
}

pub fn execute_cleanup_runtime_abi_routine(
    plan: &RuntimePackagePlan,
    routine_key: &str,
    arg: RuntimeAbiValue,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    validate_runtime_requirements_supported(plan, host)?;
    let (routine_index, _) = plan
        .routines
        .iter()
        .enumerate()
        .find(|(_, routine)| routine.routine_key == routine_key)
        .ok_or_else(|| format!("cleanup routine `{routine_key}` is not present"))?;
    let outcome = execute_routine_call_with_state(
        plan,
        routine_index,
        Vec::new(),
        vec![runtime_value_from_abi(arg)],
        &[],
        state,
        host,
        false,
    )
    .map_err(runtime_eval_message)?;
    if let Some(control) = outcome.control {
        return Err(runtime_eval_message(match control {
            super::FlowSignal::OwnerExit {
                owner_key,
                exit_name,
            } => super::RuntimeEvalSignal::OwnerExit {
                owner_key,
                exit_name,
            },
            other => super::RuntimeEvalSignal::Message(format!(
                "unsupported cleanup abi control flow `{other:?}`"
            )),
        }));
    }
    expect_cleanup_runtime_value(outcome.value)
}

pub(crate) fn project_export_write_backs(
    routine: &RuntimeRoutinePlan,
    final_args: Vec<RuntimeValue>,
) -> Result<Vec<RuntimeAbiWriteBack>, String> {
    routine
        .params
        .iter()
        .enumerate()
        .filter(|(_, param)| param.mode.as_deref() == Some("edit"))
        .map(|(index, param)| {
            let value = final_args.get(index).cloned().ok_or_else(|| {
                format!(
                    "missing final arg `{}` at exported edit index `{index}`",
                    param.name
                )
            })?;
            Ok(RuntimeAbiWriteBack {
                index,
                name: param.name.clone(),
                value: abi_value_from_runtime(value)?,
            })
        })
        .collect()
}

fn native_abi_callable(routine: &RuntimeRoutinePlan) -> bool {
    routine.exported
        && routine.symbol_kind == "fn"
        && !routine.is_async
        && routine.type_params.is_empty()
        && routine
            .params
            .iter()
            .all(|param| native_abi_supported_type(&param.ty))
        && routine
            .return_type
            .as_ref()
            .is_none_or(native_abi_supported_type)
}

fn native_abi_supported_type(ty: &IrRoutineType) -> bool {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => matches!(
            path.root_name(),
            Some("Int") | Some("Bool") | Some("Str") | Some("Unit")
        ),
        IrRoutineTypeKind::Apply { base, args } => match base.root_name() {
            Some("Pair") if args.len() == 2 => args.iter().all(native_abi_supported_type),
            Some("Array") if args.len() == 1 => args[0].root_name() == Some("Int"),
            _ => false,
        },
        IrRoutineTypeKind::Tuple(items) if items.len() == 2 => {
            items.iter().all(native_abi_supported_type)
        }
        _ => false,
    }
}

fn runtime_value_from_abi(value: RuntimeAbiValue) -> RuntimeValue {
    match value {
        RuntimeAbiValue::Int(value) => RuntimeValue::Int(value),
        RuntimeAbiValue::Bool(value) => RuntimeValue::Bool(value),
        RuntimeAbiValue::Str(value) => RuntimeValue::Str(value),
        RuntimeAbiValue::Bytes(bytes) => RuntimeValue::Array(
            bytes
                .into_iter()
                .map(|byte| RuntimeValue::Int(i64::from(byte)))
                .collect(),
        ),
        RuntimeAbiValue::Pair(left, right) => RuntimeValue::Pair(
            Box::new(runtime_value_from_abi(*left)),
            Box::new(runtime_value_from_abi(*right)),
        ),
        RuntimeAbiValue::Unit => RuntimeValue::Unit,
    }
}

fn expect_cleanup_runtime_value(value: RuntimeValue) -> Result<(), String> {
    match value {
        RuntimeValue::Unit => Ok(()),
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Ok") => {
            if payload.is_empty() || matches!(payload.first(), Some(RuntimeValue::Unit)) {
                Ok(())
            } else {
                Err(format!(
                    "cleanup footer expected Result.Ok(Unit), got `{name}`"
                ))
            }
        }
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Err") => {
            match payload.as_slice() {
                [RuntimeValue::Str(message)] => Err(message.clone()),
                [other] => Err(format!(
                    "cleanup footer expected Result.Err(Str), got `{other:?}`"
                )),
                _ => Err(format!(
                    "cleanup footer expected Result.Err(Str), got `{name}`"
                )),
            }
        }
        other => Err(format!(
            "cleanup footer expected Unit or Result[Unit, Str], got `{other:?}`"
        )),
    }
}

fn abi_value_from_runtime(value: RuntimeValue) -> Result<RuntimeAbiValue, String> {
    match value {
        RuntimeValue::Int(value) => Ok(RuntimeAbiValue::Int(value)),
        RuntimeValue::Bool(value) => Ok(RuntimeAbiValue::Bool(value)),
        RuntimeValue::Str(value) => Ok(RuntimeAbiValue::Str(value)),
        RuntimeValue::Array(values) => Ok(RuntimeAbiValue::Bytes(
            values
                .into_iter()
                .enumerate()
                .map(|(index, value)| match value {
                    RuntimeValue::Int(value) => u8::try_from(value).map_err(|_| {
                        format!(
                            "runtime abi byte array index `{index}` is out of range 0..255: `{value}`"
                        )
                    }),
                    other => Err(format!(
                        "runtime abi only supports Array[Int] byte results, got `{other:?}` at index `{index}`"
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        RuntimeValue::Pair(left, right) => Ok(RuntimeAbiValue::Pair(
            Box::new(abi_value_from_runtime(*left)?),
            Box::new(abi_value_from_runtime(*right)?),
        )),
        RuntimeValue::Unit => Ok(RuntimeAbiValue::Unit),
        other => Err(format!(
            "runtime abi only supports Int, Bool, Str, Array[Int], Pair, or Unit results, got `{other:?}`"
        )),
    }
}
