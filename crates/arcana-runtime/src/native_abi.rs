use super::{
    RuntimeExecutionState, RuntimeHost, RuntimePackagePlan, RuntimeValue,
    execute_routine_with_state, validate_runtime_requirements_supported,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeAbiValue {
    Int(i64),
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
    Pair(Box<RuntimeAbiValue>, Box<RuntimeAbiValue>),
    Unit,
}

pub fn execute_exported_abi_routine(
    plan: &RuntimePackagePlan,
    routine_key: &str,
    args: Vec<RuntimeAbiValue>,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeAbiValue, String> {
    let routine_index = plan
        .routines
        .iter()
        .enumerate()
        .find(|(_, routine)| {
            routine.exported
                && routine.symbol_kind == "fn"
                && !routine.is_async
                && routine.routine_key == routine_key
        })
        .map(|(index, _)| index)
        .ok_or_else(|| format!("abi routine `{routine_key}` is not exported or callable"))?;
    validate_runtime_requirements_supported(plan, host)?;
    let mut state = RuntimeExecutionState::default();
    let runtime_args = args
        .into_iter()
        .map(runtime_value_from_abi)
        .collect::<Vec<_>>();
    let value = execute_routine_with_state(
        plan,
        routine_index,
        Vec::new(),
        runtime_args,
        &mut state,
        host,
    )?;
    abi_value_from_runtime(value)
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
