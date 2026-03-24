use std::collections::BTreeMap;

use serde::Serialize;

use super::{
    RuntimeExecutionState, RuntimeHost, RuntimePackagePlan, RuntimeRoutinePlan, RuntimeValue,
    execute_routine_with_state, routine_plan::render_runtime_signature_text,
    validate_runtime_requirements_supported,
};

pub const RUNTIME_JSON_ABI_FORMAT: &str = "arcana-runtime-json-abi-v1";

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
    impl_target_type: Option<&'a str>,
    impl_trait_path: Option<&'a [String]>,
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
                impl_target_type: routine.impl_target_type.as_deref(),
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
    let routine_index = plan
        .routines
        .iter()
        .enumerate()
        .find(|(_, routine)| json_abi_callable(routine) && routine.routine_key == routine_key)
        .map(|(index, _)| index)
        .ok_or_else(|| format!("json abi routine `{routine_key}` is not exported or callable"))?;
    validate_runtime_requirements_supported(plan, host)?;
    let converted_args = args
        .iter()
        .map(json_value_to_runtime_value)
        .collect::<Result<Vec<_>, _>>()?;
    let mut state = RuntimeExecutionState::default();
    let value = execute_routine_with_state(
        plan,
        routine_index,
        Vec::new(),
        converted_args,
        &mut state,
        host,
    )?;
    let rendered = runtime_value_to_json_value(value)?;
    serde_json::to_string(&rendered)
        .map_err(|e| format!("failed to render runtime json abi result: {e}"))
}

fn exported_json_abi_routines(plan: &RuntimePackagePlan) -> Vec<&RuntimeRoutinePlan> {
    plan.routines
        .iter()
        .filter(|routine| json_abi_callable(routine))
        .collect()
}

fn json_abi_callable(routine: &RuntimeRoutinePlan) -> bool {
    routine.exported
        && routine.symbol_kind == "fn"
        && !routine.is_async
        && routine.type_params.is_empty()
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
