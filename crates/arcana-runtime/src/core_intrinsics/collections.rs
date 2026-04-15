use super::*;
use crate::runtime_intrinsics::CollectionsIntrinsic as RuntimeIntrinsic;

#[allow(unused_variables)]
pub(super) fn execute(
    intrinsic: RuntimeIntrinsic,
    type_args: &[String],
    final_args: &mut Vec<RuntimeValue>,
    plan: &RuntimePackagePlan,
    mut scopes: Option<&mut Vec<RuntimeScope>>,
    current_package_id: Option<&str>,
    current_module_id: Option<&str>,
    aliases: Option<&BTreeMap<String, Vec<String>>>,
    type_bindings: Option<&RuntimeTypeBindings>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let args = final_args.clone();
    match intrinsic {
        RuntimeIntrinsic::OptionIsSome => {
            let value = expect_single_arg(args, "option_is_some")?;
            match value {
                RuntimeValue::Variant { name, payload } => {
                    if variant_name_matches(&name, "Option.Some") && payload.len() == 1 {
                        Ok(RuntimeValue::Bool(true))
                    } else if variant_name_matches(&name, "Option.None") && payload.is_empty() {
                        Ok(RuntimeValue::Bool(false))
                    } else {
                        Err("option_is_some expects Option".to_string())
                    }
                }
                _ => Err("option_is_some expects Option".to_string()),
            }
        }
        RuntimeIntrinsic::OptionIsNone => {
            let value = expect_single_arg(args, "option_is_none")?;
            match value {
                RuntimeValue::Variant { name, payload } => {
                    if variant_name_matches(&name, "Option.Some") && payload.len() == 1 {
                        Ok(RuntimeValue::Bool(false))
                    } else if variant_name_matches(&name, "Option.None") && payload.is_empty() {
                        Ok(RuntimeValue::Bool(true))
                    } else {
                        Err("option_is_none expects Option".to_string())
                    }
                }
                _ => Err("option_is_none expects Option".to_string()),
            }
        }
        RuntimeIntrinsic::OptionUnwrapOr => {
            if args.len() != 2 {
                return Err("option_unwrap_or expects two arguments".to_string());
            }
            let fallback = args[1].clone();
            match args[0].clone() {
                RuntimeValue::Variant { name, mut payload } => {
                    if variant_name_matches(&name, "Option.Some") && payload.len() == 1 {
                        Ok(payload.remove(0))
                    } else if variant_name_matches(&name, "Option.None") && payload.is_empty() {
                        Ok(fallback)
                    } else {
                        Err("option_unwrap_or expects Option".to_string())
                    }
                }
                _ => Err("option_unwrap_or expects Option".to_string()),
            }
        }
        RuntimeIntrinsic::ResultOk => match args.len() {
            0 => Ok(ok_variant(RuntimeValue::Unit)),
            1 => Ok(ok_variant(
                args.into_iter().next().unwrap_or(RuntimeValue::Unit),
            )),
            _ => Err("Result.Ok expects zero or one argument".to_string()),
        },
        RuntimeIntrinsic::ResultErr => {
            let value = expect_str(expect_single_arg(args, "Result.Err")?, "Result.Err")?;
            Ok(err_variant(value))
        }
        RuntimeIntrinsic::ResultIsOk => {
            let value = expect_single_arg(args, "result_is_ok")?;
            match value {
                RuntimeValue::Variant { name, payload } if payload.len() == 1 => {
                    if variant_name_matches(&name, "Result.Ok") {
                        Ok(RuntimeValue::Bool(true))
                    } else if variant_name_matches(&name, "Result.Err") {
                        Ok(RuntimeValue::Bool(false))
                    } else {
                        Err("result_is_ok expects Result".to_string())
                    }
                }
                _ => Err("result_is_ok expects Result".to_string()),
            }
        }
        RuntimeIntrinsic::ResultIsErr => {
            let value = expect_single_arg(args, "result_is_err")?;
            match value {
                RuntimeValue::Variant { name, payload } if payload.len() == 1 => {
                    if variant_name_matches(&name, "Result.Ok") {
                        Ok(RuntimeValue::Bool(false))
                    } else if variant_name_matches(&name, "Result.Err") {
                        Ok(RuntimeValue::Bool(true))
                    } else {
                        Err("result_is_err expects Result".to_string())
                    }
                }
                _ => Err("result_is_err expects Result".to_string()),
            }
        }
        RuntimeIntrinsic::ResultUnwrapOr => {
            if args.len() != 2 {
                return Err("result_unwrap_or expects two arguments".to_string());
            }
            let fallback = args[1].clone();
            match args[0].clone() {
                RuntimeValue::Variant { name, mut payload } if payload.len() == 1 => {
                    if variant_name_matches(&name, "Result.Ok") {
                        Ok(payload.remove(0))
                    } else if variant_name_matches(&name, "Result.Err") {
                        Ok(fallback)
                    } else {
                        Err("result_unwrap_or expects Result".to_string())
                    }
                }
                _ => Err("result_unwrap_or expects Result".to_string()),
            }
        }
        RuntimeIntrinsic::ListNew => {
            if !args.is_empty() {
                return Err("list_new expects zero arguments".to_string());
            }
            Ok(RuntimeValue::List(Vec::new()))
        }
        RuntimeIntrinsic::ListLen => {
            let value = expect_single_arg(args, "list_len")?;
            let RuntimeValue::List(values) = value else {
                return Err("list_len expects List".to_string());
            };
            Ok(RuntimeValue::Int(i64::try_from(values.len()).map_err(
                |_| "list length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::ListIsEmpty => {
            let value = expect_single_arg(args, "list_is_empty")?;
            let RuntimeValue::List(values) = value else {
                return Err("list_is_empty expects List".to_string());
            };
            Ok(RuntimeValue::Bool(values.is_empty()))
        }
        RuntimeIntrinsic::ListPush => {
            if args.len() != 2 {
                return Err("list_push expects two arguments".to_string());
            }
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "list_push on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "list_push on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "list_push on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let RuntimeValue::List(mut values) = read_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    host,
                )?
                else {
                    return Err("list_push expects List".to_string());
                };
                values.push(args[1].clone());
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::List(values),
                    host,
                )?;
                return Ok(RuntimeValue::Unit);
            }
            let Some(RuntimeValue::List(values)) = final_args.get_mut(0) else {
                return Err("list_push expects List".to_string());
            };
            values.push(args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ListPop => {
            if args.len() != 1 {
                return Err("list_pop expects one argument".to_string());
            }
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "list_pop on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "list_pop on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "list_pop on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let RuntimeValue::List(mut values) = read_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    host,
                )?
                else {
                    return Err("list_pop expects List".to_string());
                };
                let value = values
                    .pop()
                    .ok_or_else(|| "list_pop called on empty list".to_string())?;
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::List(values),
                    host,
                )?;
                return Ok(value);
            }
            let Some(RuntimeValue::List(values)) = final_args.get_mut(0) else {
                return Err("list_pop expects List".to_string());
            };
            values
                .pop()
                .ok_or_else(|| "list_pop called on empty list".to_string())
        }
        RuntimeIntrinsic::ListTryPopOr => {
            if args.len() != 2 {
                return Err("list_try_pop_or expects two arguments".to_string());
            }
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .ok_or_else(|| "list_try_pop_or on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id.ok_or_else(|| {
                    "list_try_pop_or on refs requires package context".to_string()
                })?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "list_try_pop_or on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let RuntimeValue::List(mut values) = read_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    host,
                )?
                else {
                    return Err("list_try_pop_or expects List".to_string());
                };
                let result = match values.pop() {
                    Some(value) => make_pair(RuntimeValue::Bool(true), value),
                    None => make_pair(RuntimeValue::Bool(false), args[1].clone()),
                };
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::List(values),
                    host,
                )?;
                return Ok(result);
            }
            let Some(RuntimeValue::List(values)) = final_args.get_mut(0) else {
                return Err("list_try_pop_or expects List".to_string());
            };
            Ok(match values.pop() {
                Some(value) => make_pair(RuntimeValue::Bool(true), value),
                None => make_pair(RuntimeValue::Bool(false), args[1].clone()),
            })
        }
        RuntimeIntrinsic::ArrayNew => {
            if args.len() != 2 {
                return Err("array_new expects two arguments".to_string());
            }
            let len = expect_int(args[0].clone(), "array_new")?;
            if len < 0 {
                return Err("array_new length must be non-negative".to_string());
            }
            Ok(RuntimeValue::Array(vec![args[1].clone(); len as usize]))
        }
        RuntimeIntrinsic::ArrayLen => {
            let value = expect_single_arg(args, "array_len")?;
            let RuntimeValue::Array(values) = value else {
                return Err("array_len expects Array".to_string());
            };
            Ok(RuntimeValue::Int(i64::try_from(values.len()).map_err(
                |_| "array length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::ArrayFromList => {
            let value = expect_single_arg(args, "array_from_list")?;
            let RuntimeValue::List(values) = value else {
                return Err("array_from_list expects List".to_string());
            };
            Ok(RuntimeValue::Array(values))
        }
        RuntimeIntrinsic::ArrayToList => {
            let value = expect_single_arg(args, "array_to_list")?;
            let RuntimeValue::Array(values) = value else {
                return Err("array_to_list expects Array".to_string());
            };
            Ok(RuntimeValue::List(values))
        }
        RuntimeIntrinsic::MapNew => {
            if !args.is_empty() {
                return Err("map_new expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Map(Vec::new()))
        }
        RuntimeIntrinsic::MapLen => {
            let value = expect_single_arg(args, "map_len")?;
            let RuntimeValue::Map(entries) = value else {
                return Err("map_len expects Map".to_string());
            };
            Ok(RuntimeValue::Int(i64::try_from(entries.len()).map_err(
                |_| "map length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::MapHas => {
            if args.len() != 2 {
                return Err("map_has expects two arguments".to_string());
            }
            let RuntimeValue::Map(entries) = args[0].clone() else {
                return Err("map_has expects Map".to_string());
            };
            Ok(RuntimeValue::Bool(
                entries.iter().any(|(entry_key, _)| *entry_key == args[1]),
            ))
        }
        RuntimeIntrinsic::MapGet => {
            if args.len() != 2 {
                return Err("map_get expects two arguments".to_string());
            }
            let RuntimeValue::Map(entries) = args[0].clone() else {
                return Err("map_get expects Map".to_string());
            };
            entries
                .into_iter()
                .find_map(|(entry_key, entry_value)| (entry_key == args[1]).then_some(entry_value))
                .ok_or_else(|| "map_get key was not present".to_string())
        }
        RuntimeIntrinsic::MapSet => {
            if args.len() != 3 {
                return Err("map_set expects three arguments".to_string());
            }
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "map_set on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "map_set on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "map_set on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let RuntimeValue::Map(mut entries) = read_runtime_value_if_ref(
                    RuntimeValue::Ref(reference.clone()),
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?
                else {
                    return Err("map_set expects Map".to_string());
                };
                if let Some((_, entry_value)) = entries
                    .iter_mut()
                    .find(|(entry_key, _)| *entry_key == args[1])
                {
                    *entry_value = args[2].clone();
                } else {
                    entries.push((args[1].clone(), args[2].clone()));
                }
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::Map(entries),
                    host,
                )?;
                return Ok(RuntimeValue::Unit);
            }
            let Some(RuntimeValue::Map(entries)) = final_args.get_mut(0) else {
                return Err("map_set expects Map".to_string());
            };
            if let Some((_, entry_value)) = entries
                .iter_mut()
                .find(|(entry_key, _)| *entry_key == args[1])
            {
                *entry_value = args[2].clone();
            } else {
                entries.push((args[1].clone(), args[2].clone()));
            }
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MapRemove => {
            if args.len() != 2 {
                return Err("map_remove expects two arguments".to_string());
            }
            if let Some(RuntimeValue::Ref(reference)) = final_args.first().cloned() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "map_remove on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "map_remove on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "map_remove on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                let RuntimeValue::Map(mut entries) = read_runtime_value_if_ref(
                    RuntimeValue::Ref(reference.clone()),
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?
                else {
                    return Err("map_remove expects Map".to_string());
                };
                let original_len = entries.len();
                entries.retain(|(entry_key, _)| *entry_key != args[1]);
                let removed = entries.len() != original_len;
                write_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    RuntimeValue::Map(entries),
                    host,
                )?;
                return Ok(RuntimeValue::Bool(removed));
            }
            let Some(RuntimeValue::Map(entries)) = final_args.get_mut(0) else {
                return Err("map_remove expects Map".to_string());
            };
            let original_len = entries.len();
            entries.retain(|(entry_key, _)| *entry_key != args[1]);
            Ok(RuntimeValue::Bool(entries.len() != original_len))
        }
        RuntimeIntrinsic::MapTryGetOr => {
            if args.len() != 3 {
                return Err("map_try_get_or expects three arguments".to_string());
            }
            let RuntimeValue::Map(entries) = args[0].clone() else {
                return Err("map_try_get_or expects Map".to_string());
            };
            Ok(
                match entries.into_iter().find_map(|(entry_key, entry_value)| {
                    (entry_key == args[1]).then_some(entry_value)
                }) {
                    Some(value) => make_pair(RuntimeValue::Bool(true), value),
                    None => make_pair(RuntimeValue::Bool(false), args[2].clone()),
                },
            )
        }
    }
}
