use std::collections::BTreeMap;

// Tooling/debug projection of the cabi export contract, not the primary foreign ABI owner.
use arcana_cabi::ArcanaCabiPassMode;
use arcana_ir::{IrRoutineType, IrRoutineTypeKind};
use serde::Serialize;

use super::{
    RuntimeExecutionState, RuntimeHost, RuntimeOpaqueFamily, RuntimePackagePlan,
    RuntimeRoutinePlan, RuntimeValue, execute_routine_call_with_state,
    native_abi::project_export_write_backs, routine_plan::render_runtime_signature_text,
    runtime_eval_message, validate_runtime_requirements_supported,
};

pub const RUNTIME_JSON_ABI_FORMAT: &str = "arcana-runtime-json-abi-v3";

#[derive(Serialize)]
struct JsonAbiManifest<'a> {
    format: &'static str,
    package_name: &'a str,
    root_module_id: &'a str,
    routines: Vec<JsonAbiRoutine<'a>>,
}

#[derive(Serialize)]
struct JsonAbiRoutine<'a> {
    routine_key: &'a str,
    module_id: &'a str,
    symbol_name: &'a str,
    symbol_kind: &'a str,
    signature: String,
    params: Vec<JsonAbiParam<'a>>,
    return_type: String,
    impl_target_type: Option<String>,
    impl_trait_path: Option<&'a [String]>,
}

#[derive(Serialize)]
struct JsonAbiParam<'a> {
    name: &'a str,
    source_mode: &'static str,
    input_type: String,
    pass_mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    write_back_type: Option<String>,
}

pub fn render_exported_json_abi_manifest(plan: &RuntimePackagePlan) -> Result<String, String> {
    let manifest = JsonAbiManifest {
        format: RUNTIME_JSON_ABI_FORMAT,
        package_name: &plan.package_name,
        root_module_id: &plan.root_module_id,
        routines: exported_json_abi_routines(plan)
            .into_iter()
            .map(|routine| JsonAbiRoutine {
                routine_key: &routine.routine_key,
                module_id: &routine.module_id,
                symbol_name: &routine.symbol_name,
                symbol_kind: &routine.symbol_kind,
                signature: render_runtime_signature_text(routine),
                params: routine
                    .params
                    .iter()
                    .map(|param| JsonAbiParam {
                        name: &param.name,
                        source_mode: json_abi_source_mode(param.mode.as_deref()),
                        input_type: param.ty.render(),
                        pass_mode: json_abi_pass_mode(param.mode.as_deref()).as_str(),
                        write_back_type: (param.mode.as_deref() == Some("edit"))
                            .then(|| param.ty.render()),
                    })
                    .collect(),
                return_type: routine
                    .return_type
                    .as_ref()
                    .map(IrRoutineType::render)
                    .unwrap_or_else(|| "Unit".to_string()),
                impl_target_type: routine.impl_target_type.as_ref().map(|ty| ty.render()),
                impl_trait_path: routine.impl_trait_path.as_deref(),
            })
            .collect(),
    };
    serde_json::to_string(&manifest)
        .map_err(|e| format!("failed to render runtime json abi manifest: {e}"))
}

pub fn execute_exported_json_abi_routine(
    plan: &RuntimePackagePlan,
    routine_key: &str,
    args_json: &str,
    host: &mut dyn RuntimeHost,
) -> Result<String, String> {
    let args_value = serde_json::from_str::<serde_json::Value>(args_json)
        .map_err(|e| format!("failed to parse runtime json abi args: {e}"))?;
    let args = args_value
        .as_array()
        .ok_or_else(|| "runtime json abi args must be a JSON array".to_string())?;
    let (routine_index, routine) = plan
        .routines
        .iter()
        .enumerate()
        .find(|(_, routine)| json_abi_callable(plan, routine) && routine.routine_key == routine_key)
        .ok_or_else(|| format!("json abi routine `{routine_key}` is not exported or callable"))?;
    validate_runtime_requirements_supported(plan, host)?;
    let converted_args = args
        .iter()
        .map(json_value_to_runtime_value)
        .collect::<Result<Vec<_>, _>>()?;
    let mut state = RuntimeExecutionState::default();
    let outcome = execute_routine_call_with_state(
        plan,
        routine_index,
        Vec::new(),
        converted_args,
        &[],
        None,
        None,
        None,
        None,
        None,
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
                "unsupported json abi control flow `{other:?}`"
            )),
        }));
    }
    let write_backs = project_export_write_backs(routine, outcome.final_args)?;
    let rendered = serde_json::json!({
        "result": runtime_value_to_json_value(outcome.value)?,
        "write_backs": write_backs
            .into_iter()
            .map(|write_back| {
                Ok(serde_json::json!({
                    "index": write_back.index,
                    "name": write_back.name,
                    "value": runtime_value_to_json_value(runtime_value_from_abi(write_back.value))?,
                }))
            })
            .collect::<Result<Vec<_>, String>>()?,
    });
    serde_json::to_string(&rendered)
        .map_err(|e| format!("failed to render runtime json abi result: {e}"))
}

fn exported_json_abi_routines(plan: &RuntimePackagePlan) -> Vec<&RuntimeRoutinePlan> {
    plan.routines
        .iter()
        .filter(|routine| json_abi_callable(plan, routine))
        .collect()
}

fn json_abi_callable(plan: &RuntimePackagePlan, routine: &RuntimeRoutinePlan) -> bool {
    routine.exported
        && routine.symbol_kind == "fn"
        && !routine.is_async
        && routine.type_params.is_empty()
        && routine
            .params
            .iter()
            .all(|param| json_abi_supported_type(plan, &param.ty))
        && routine
            .return_type
            .as_ref()
            .is_none_or(|ty| json_abi_supported_type(plan, ty))
}

fn json_abi_supported_type(plan: &RuntimePackagePlan, ty: &IrRoutineType) -> bool {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => {
            !json_abi_blocks_path(plan, &path.render(), path.root_name())
        }
        IrRoutineTypeKind::Apply { base, args } => {
            !json_abi_blocks_path(plan, &base.render(), base.root_name())
                && args.iter().all(|arg| json_abi_supported_type(plan, arg))
        }
        IrRoutineTypeKind::Ref { .. } => false,
        IrRoutineTypeKind::Tuple(items) => {
            items.len() == 2 && items.iter().all(|item| json_abi_supported_type(plan, item))
        }
        IrRoutineTypeKind::Projection(_) => false,
    }
}

fn json_abi_blocks_path(
    plan: &RuntimePackagePlan,
    rendered: &str,
    root_name: Option<&str>,
) -> bool {
    root_name == Some("Owner") || json_abi_path_is_runtime_opaque(plan, rendered)
}

fn json_abi_path_is_runtime_opaque(plan: &RuntimePackagePlan, rendered: &str) -> bool {
    RuntimeOpaqueFamily::ALL.into_iter().any(|family| {
        family.canonical_type_name() == rendered
            || plan
                .opaque_family_types
                .get(family.lang_item_name())
                .is_some_and(|entries| entries.iter().any(|entry| entry == rendered))
    })
}

fn json_abi_pass_mode(mode: Option<&str>) -> ArcanaCabiPassMode {
    match mode {
        Some("edit") => ArcanaCabiPassMode::InWithWriteBack,
        _ => ArcanaCabiPassMode::In,
    }
}

fn json_abi_source_mode(mode: Option<&str>) -> &'static str {
    match mode {
        Some("take") => "take",
        Some("edit") => "edit",
        _ => "read",
    }
}

fn runtime_value_from_abi(value: super::RuntimeAbiValue) -> RuntimeValue {
    match value {
        super::RuntimeAbiValue::Int(value) => RuntimeValue::Int(value),
        super::RuntimeAbiValue::Bool(value) => RuntimeValue::Bool(value),
        super::RuntimeAbiValue::Str(value) => RuntimeValue::Str(value),
        super::RuntimeAbiValue::Bytes(bytes) => RuntimeValue::Array(
            bytes
                .into_iter()
                .map(|byte| RuntimeValue::Int(i64::from(byte)))
                .collect(),
        ),
        super::RuntimeAbiValue::Pair(left, right) => RuntimeValue::Pair(
            Box::new(runtime_value_from_abi(*left)),
            Box::new(runtime_value_from_abi(*right)),
        ),
        super::RuntimeAbiValue::Unit => RuntimeValue::Unit,
    }
}

fn json_value_to_runtime_value(value: &serde_json::Value) -> Result<RuntimeValue, String> {
    match value {
        serde_json::Value::Null => Ok(RuntimeValue::Unit),
        serde_json::Value::Bool(value) => Ok(RuntimeValue::Bool(*value)),
        serde_json::Value::Number(value) => value
            .as_i64()
            .map(RuntimeValue::Int)
            .ok_or_else(|| "runtime json abi only supports signed 64-bit integers".to_string()),
        serde_json::Value::String(value) => Ok(RuntimeValue::Str(value.clone())),
        serde_json::Value::Array(values) => Ok(RuntimeValue::List(
            values
                .iter()
                .map(json_value_to_runtime_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        serde_json::Value::Object(entries) => {
            if let Some(values) = entries.get("$array") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$array` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::Array(
                    values
                        .iter()
                        .map(json_value_to_runtime_value)
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            if let Some(values) = entries.get("$pair") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$pair` must contain a JSON array".to_string())?;
                if values.len() != 2 {
                    return Err("`$pair` must contain exactly two elements".to_string());
                }
                return Ok(RuntimeValue::Pair(
                    Box::new(json_value_to_runtime_value(&values[0])?),
                    Box::new(json_value_to_runtime_value(&values[1])?),
                ));
            }
            if let Some(values) = entries.get("$map") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$map` must contain a JSON array".to_string())?;
                let entries = values
                    .iter()
                    .map(|entry| {
                        let pair = entry
                            .as_array()
                            .ok_or_else(|| "map entries must be two-element arrays".to_string())?;
                        if pair.len() != 2 {
                            return Err("map entries must contain exactly two elements".to_string());
                        }
                        Ok((
                            json_value_to_runtime_value(&pair[0])?,
                            json_value_to_runtime_value(&pair[1])?,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                return Ok(RuntimeValue::Map(entries));
            }
            if let Some(range) = entries.get("$range") {
                let range = range
                    .as_object()
                    .ok_or_else(|| "`$range` must contain an object".to_string())?;
                let start = match range.get("start") {
                    Some(serde_json::Value::Null) | None => None,
                    Some(value) => Some(expect_json_int(value, "range start")?),
                };
                let end = match range.get("end") {
                    Some(serde_json::Value::Null) | None => None,
                    Some(value) => Some(expect_json_int(value, "range end")?),
                };
                let inclusive_end = range
                    .get("inclusive_end")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                return Ok(RuntimeValue::Range {
                    start,
                    end,
                    inclusive_end,
                });
            }
            if let Some(name) = entries.get("$record") {
                let name = name
                    .as_str()
                    .ok_or_else(|| "`$record` must be a string".to_string())?;
                let fields = entries
                    .get("fields")
                    .and_then(serde_json::Value::as_object)
                    .ok_or_else(|| "record values must include a `fields` object".to_string())?;
                let mut mapped = BTreeMap::new();
                for (key, value) in fields {
                    mapped.insert(key.clone(), json_value_to_runtime_value(value)?);
                }
                return Ok(RuntimeValue::Record {
                    name: name.to_string(),
                    fields: mapped,
                });
            }
            if let Some(name) = entries.get("$variant") {
                let name = name
                    .as_str()
                    .ok_or_else(|| "`$variant` must be a string".to_string())?;
                let payload = entries
                    .get("payload")
                    .and_then(serde_json::Value::as_array)
                    .ok_or_else(|| "variant values must include a `payload` array".to_string())?;
                return Ok(RuntimeValue::Variant {
                    name: name.to_string(),
                    payload: payload
                        .iter()
                        .map(json_value_to_runtime_value)
                        .collect::<Result<Vec<_>, _>>()?,
                });
            }
            Ok(RuntimeValue::Map(
                entries
                    .iter()
                    .map(|(key, value)| {
                        Ok((
                            RuntimeValue::Str(key.clone()),
                            json_value_to_runtime_value(value)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            ))
        }
    }
}

fn runtime_value_to_json_value(value: RuntimeValue) -> Result<serde_json::Value, String> {
    match value {
        RuntimeValue::Unit => Ok(serde_json::Value::Null),
        RuntimeValue::Int(value) => Ok(serde_json::Value::Number(value.into())),
        RuntimeValue::Bool(value) => Ok(serde_json::Value::Bool(value)),
        RuntimeValue::Str(value) => Ok(serde_json::Value::String(value)),
        RuntimeValue::List(values) => Ok(serde_json::Value::Array(
            values
                .into_iter()
                .map(runtime_value_to_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        RuntimeValue::Array(values) => Ok(serde_json::json!({
            "$array": values
                .into_iter()
                .map(runtime_value_to_json_value)
                .collect::<Result<Vec<_>, _>>()?
        })),
        RuntimeValue::Pair(left, right) => Ok(serde_json::json!({
            "$pair": [
                runtime_value_to_json_value(*left)?,
                runtime_value_to_json_value(*right)?,
            ]
        })),
        RuntimeValue::Map(entries) => {
            let mut object = serde_json::Map::new();
            let mut string_keys = true;
            for (key, value) in &entries {
                let RuntimeValue::Str(key) = key else {
                    string_keys = false;
                    break;
                };
                object.insert(key.clone(), runtime_value_to_json_value(value.clone())?);
            }
            if string_keys {
                Ok(serde_json::Value::Object(object))
            } else {
                Ok(serde_json::json!({
                    "$map": entries
                        .into_iter()
                        .map(|(key, value)| {
                            Ok(serde_json::Value::Array(vec![
                                runtime_value_to_json_value(key)?,
                                runtime_value_to_json_value(value)?,
                            ]))
                        })
                        .collect::<Result<Vec<_>, String>>()?
                }))
            }
        }
        RuntimeValue::Range {
            start,
            end,
            inclusive_end,
        } => Ok(serde_json::json!({
            "$range": {
                "start": start,
                "end": end,
                "inclusive_end": inclusive_end,
            }
        })),
        RuntimeValue::Record { name, fields } => {
            let mut mapped = serde_json::Map::new();
            for (key, value) in fields {
                mapped.insert(key, runtime_value_to_json_value(value)?);
            }
            Ok(serde_json::json!({
                "$record": name,
                "fields": serde_json::Value::Object(mapped),
            }))
        }
        RuntimeValue::Variant { name, payload } => Ok(serde_json::json!({
            "$variant": name,
            "payload": payload
                .into_iter()
                .map(runtime_value_to_json_value)
                .collect::<Result<Vec<_>, _>>()?
        })),
        RuntimeValue::OwnerHandle(_) | RuntimeValue::Ref(_) | RuntimeValue::Opaque(_) => Err(
            "runtime json abi does not support owner, reference, or opaque runtime values"
                .to_string(),
        ),
    }
}

fn expect_json_int(value: &serde_json::Value, context: &str) -> Result<i64, String> {
    value
        .as_i64()
        .ok_or_else(|| format!("runtime json abi {context} must be a signed 64-bit integer"))
}
