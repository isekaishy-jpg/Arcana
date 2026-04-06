use crate::artifact::{AotPackageArtifact, AotRoutineArtifact};
use arcana_cabi::{
    ArcanaCabiExport, ArcanaCabiExportParam, ArcanaCabiParamSourceMode, ArcanaCabiPassMode,
    ArcanaCabiType,
};
use arcana_ir::{IrRoutineParam, IrRoutineType, IrRoutineTypeKind, parse_routine_type_text};
use std::collections::BTreeSet;

pub type NativeAbiType = ArcanaCabiType;
pub type NativeAbiParam = ArcanaCabiExportParam;
pub type NativeExport = ArcanaCabiExport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRoutineSignature {
    pub params: Vec<NativeAbiParam>,
    pub return_type: NativeAbiType,
}

pub fn collect_native_exports(artifact: &AotPackageArtifact) -> Result<Vec<NativeExport>, String> {
    validate_declared_native_export_rows(artifact)?;
    let root_prefix = format!("{}.", artifact.root_module_id);
    let mut exports = Vec::new();
    let mut used_names = std::collections::BTreeSet::new();

    for routine in &artifact.routines {
        if !routine.exported {
            continue;
        }
        if routine.module_id != artifact.root_module_id
            && !routine.module_id.starts_with(&root_prefix)
        {
            continue;
        }
        if routine.symbol_kind != "fn" {
            continue;
        }
        if routine.impl_target_type.is_some() {
            return Err(format!(
                "windows-dll target does not yet support exported impl method `{}`",
                routine.routine_key
            ));
        }
        if routine.is_async {
            return Err(format!(
                "windows-dll target does not support async export `{}`",
                routine.routine_key
            ));
        }
        if !routine.type_params.is_empty() {
            return Err(format!(
                "windows-dll target does not support generic export `{}`",
                routine.routine_key
            ));
        }

        let NativeRoutineSignature {
            params,
            return_type,
        } = native_routine_signature(routine).map_err(|err| {
            format!(
                "windows-dll target cannot export `{}`: {err}",
                routine.routine_key
            )
        })?;
        let mut export_name = default_export_name(
            &artifact.root_module_id,
            &routine.module_id,
            &routine.symbol_name,
        );
        if !used_names.insert(export_name.clone()) {
            export_name = format!("{export_name}__{}", sanitize_name(&routine.routine_key));
            used_names.insert(export_name.clone());
        }
        exports.push(NativeExport {
            routine_key: routine.routine_key.clone(),
            export_name,
            symbol_name: routine.symbol_name.clone(),
            params,
            return_type,
        });
    }

    Ok(exports)
}

fn parse_package_function_export_row(row: &str) -> Option<(&str, &str, &str)> {
    let payload = row.strip_prefix("module=")?;
    let (module_id, module_row) = payload.split_once(':')?;
    let surface_payload = module_row.strip_prefix("export:")?;
    let (kind, signature) = surface_payload.split_once(':')?;
    Some((module_id, kind, signature))
}

fn validate_declared_native_export_rows(artifact: &AotPackageArtifact) -> Result<(), String> {
    let root_prefix = format!("{}.", artifact.root_module_id);
    let declared = artifact
        .exported_surface_rows
        .iter()
        .filter_map(|row| parse_package_function_export_row(row))
        .filter(|(module_id, kind, _)| {
            *kind == "fn"
                && (*module_id == artifact.root_module_id || module_id.starts_with(&root_prefix))
        })
        .map(|(module_id, _, signature)| {
            Ok(declared_native_signature_key(signature)?
                .map(|signature_key| (module_id.to_string(), signature_key)))
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let structured = artifact
        .routines
        .iter()
        .filter(|routine| {
            routine.exported
                && (routine.module_id == artifact.root_module_id
                    || routine.module_id.starts_with(&root_prefix))
                && routine.symbol_kind == "fn"
                && routine.impl_target_type.is_none()
                && !routine.is_async
                && routine.type_params.is_empty()
                && native_routine_signature(routine).is_ok()
        })
        .map(|routine| {
            (
                routine.module_id.clone(),
                native_signature_key(
                    &routine.symbol_name,
                    &native_routine_signature(routine)
                        .expect("native signature should be available for structured export"),
                ),
            )
        })
        .collect::<BTreeSet<_>>();

    if structured != declared {
        return Err(
            "backend artifact native export rows do not match structured routines".to_string(),
        );
    }
    Ok(())
}

fn declared_native_signature_key(signature: &str) -> Result<Option<String>, String> {
    let Some(rest) = signature.strip_prefix("fn ") else {
        return Ok(None);
    };
    let Some((head, tail)) = rest.split_once('(') else {
        return Err(format!("malformed native export signature `{signature}`"));
    };
    if head.contains('[') {
        return Ok(None);
    }
    let symbol_name = head.trim();
    let (params_text, after_params) = split_signature_param_section(tail)
        .ok_or_else(|| format!("malformed native export signature `{signature}`"))?;
    let after_params = after_params.trim();
    let return_type = if after_params.is_empty() || after_params == ":" {
        NativeAbiType::Unit
    } else {
        let Some(return_text) = after_params
            .strip_prefix("->")
            .map(str::trim)
            .map(|text| text.trim_end_matches(':').trim())
        else {
            return Err(format!("malformed native export signature `{signature}`"));
        };
        parse_native_return_type(Some(&parse_routine_type_text(return_text)?))?
    };
    let params = split_signature_params(&params_text)
        .into_iter()
        .map(|param_text| parse_declared_native_param(&param_text))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(native_signature_key(
        symbol_name,
        &NativeRoutineSignature {
            params,
            return_type,
        },
    )))
}

fn parse_declared_native_param(text: &str) -> Result<NativeAbiParam, String> {
    let Some((head, ty_text)) = text.split_once(':') else {
        return Err(format!("malformed native export param `{text}`"));
    };
    let head = head.trim();
    let ty = parse_routine_type_text(ty_text.trim())?;
    let parts = head.split_whitespace().collect::<Vec<_>>();
    let param = match parts.as_slice() {
        [name] => IrRoutineParam {
            binding_id: 0,
            mode: None,
            name: (*name).to_string(),
            ty,
        },
        [mode, name] => IrRoutineParam {
            binding_id: 0,
            mode: Some((*mode).to_string()),
            name: (*name).to_string(),
            ty,
        },
        _ => return Err(format!("malformed native export param `{text}`")),
    };
    parse_native_param(&param)
}

fn split_signature_params(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    for ch in text.chars() {
        match ch {
            '[' => {
                square_depth += 1;
                current.push(ch);
            }
            ']' => {
                square_depth = square_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if square_depth == 0 && paren_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

fn split_signature_param_section(text: &str) -> Option<(String, String)> {
    let mut nested_paren_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => nested_paren_depth += 1,
            ')' => {
                if nested_paren_depth == 0 {
                    return Some((text[..index].to_string(), text[index + 1..].to_string()));
                }
                nested_paren_depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn native_signature_key(symbol_name: &str, signature: &NativeRoutineSignature) -> String {
    let params = signature
        .params
        .iter()
        .map(|param| {
            format!(
                "{}:{}:{}:{}",
                param.source_mode.as_str(),
                param.name,
                canonical_native_type_name(&param.input_type),
                param
                    .write_back_type
                    .as_ref()
                    .map(canonical_native_type_name)
                    .unwrap_or_else(|| "-".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}({})->{}",
        symbol_name,
        params,
        canonical_native_type_name(&signature.return_type)
    )
}

fn canonical_native_type_name(ty: &NativeAbiType) -> String {
    match ty {
        NativeAbiType::Int => "Int".to_string(),
        NativeAbiType::Bool => "Bool".to_string(),
        NativeAbiType::Str => "Str".to_string(),
        NativeAbiType::Bytes => "Array[Int]".to_string(),
        NativeAbiType::Opaque(name) => name.clone(),
        NativeAbiType::Unit => "Unit".to_string(),
        NativeAbiType::Pair(left, right) => {
            format!(
                "Pair[{}, {}]",
                canonical_native_type_name(left),
                canonical_native_type_name(right)
            )
        }
    }
}

pub fn native_routine_signature(
    routine: &AotRoutineArtifact,
) -> Result<NativeRoutineSignature, String> {
    Ok(NativeRoutineSignature {
        params: routine
            .params
            .iter()
            .map(parse_native_param)
            .collect::<Result<Vec<_>, _>>()?,
        return_type: parse_native_return_type(routine.return_type.as_ref())?,
    })
}

pub fn parse_native_param(param: &IrRoutineParam) -> Result<NativeAbiParam, String> {
    let ty = parse_native_type(&param.ty)?;
    let source_mode = match param.mode.as_deref() {
        None | Some("read") => ArcanaCabiParamSourceMode::Read,
        Some("take") => ArcanaCabiParamSourceMode::Take,
        Some("edit") => ArcanaCabiParamSourceMode::Edit,
        Some(other) => {
            return Err(format!(
                "unsupported native abi parameter mode `{other}` for `{}`",
                param.name
            ));
        }
    };
    Ok(NativeAbiParam {
        name: sanitize_name(&param.name),
        input_type: ty.clone(),
        source_mode,
        pass_mode: match source_mode {
            ArcanaCabiParamSourceMode::Edit => ArcanaCabiPassMode::InWithWriteBack,
            ArcanaCabiParamSourceMode::Read | ArcanaCabiParamSourceMode::Take => {
                ArcanaCabiPassMode::In
            }
        },
        write_back_type: match source_mode {
            ArcanaCabiParamSourceMode::Edit => Some(ty),
            ArcanaCabiParamSourceMode::Read | ArcanaCabiParamSourceMode::Take => None,
        },
    })
}

pub fn parse_native_return_type(
    return_type: Option<&IrRoutineType>,
) -> Result<NativeAbiType, String> {
    return_type
        .map(parse_native_type)
        .transpose()
        .map(|ty| ty.unwrap_or(NativeAbiType::Unit))
}

fn parse_native_type(ty: &IrRoutineType) -> Result<NativeAbiType, String> {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => match path.root_name() {
            Some("Int") => Ok(NativeAbiType::Int),
            Some("Bool") => Ok(NativeAbiType::Bool),
            Some("Str") => Ok(NativeAbiType::Str),
            Some("Unit") => Ok(NativeAbiType::Unit),
            _ => Ok(NativeAbiType::Opaque(ty.render())),
        },
        IrRoutineTypeKind::Apply { base, args } => match base.root_name() {
            Some("Pair") if args.len() == 2 => Ok(NativeAbiType::Pair(
                Box::new(parse_native_type(&args[0])?),
                Box::new(parse_native_type(&args[1])?),
            )),
            Some("Array") if args.len() == 1 && args[0].root_name() == Some("Int") => {
                Ok(NativeAbiType::Bytes)
            }
            _ => Err(format!("unsupported native abi type `{}`", ty.render())),
        },
        IrRoutineTypeKind::Tuple(items) if items.len() == 2 => Ok(NativeAbiType::Pair(
            Box::new(parse_native_type(&items[0])?),
            Box::new(parse_native_type(&items[1])?),
        )),
        _ => Err(format!("unsupported native abi type `{}`", ty.render())),
    }
}

fn default_export_name(root_module_id: &str, module_id: &str, symbol_name: &str) -> String {
    if module_id == root_module_id {
        return sanitize_name(symbol_name);
    }
    let relative = module_id
        .strip_prefix(root_module_id)
        .unwrap_or(module_id)
        .trim_start_matches('.');
    if relative.is_empty() {
        sanitize_name(symbol_name)
    } else {
        format!(
            "{}__{}",
            relative
                .split('.')
                .map(sanitize_name)
                .collect::<Vec<_>>()
                .join("__"),
            sanitize_name(symbol_name)
        )
    }
}

fn sanitize_name(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "_".to_string()
    } else if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("_{out}")
    } else {
        out
    }
}
