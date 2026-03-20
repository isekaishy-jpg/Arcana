use crate::artifact::AotPackageArtifact;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NativeAbiType {
    Int,
    Bool,
    Str,
    Bytes,
    Pair(Box<NativeAbiType>, Box<NativeAbiType>),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeAbiParam {
    pub name: String,
    pub ty: NativeAbiType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeExport {
    pub routine_key: String,
    pub export_name: String,
    pub params: Vec<NativeAbiParam>,
    pub return_type: NativeAbiType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRoutineSignature {
    pub params: Vec<NativeAbiParam>,
    pub return_type: NativeAbiType,
}

pub fn collect_native_exports(artifact: &AotPackageArtifact) -> Result<Vec<NativeExport>, String> {
    let root_prefix = format!("{}.", artifact.root_module_id);
    let exported_surface = exported_function_surface_rows(artifact);
    let mut exports = Vec::new();
    let mut used_names = std::collections::BTreeSet::new();

    for routine in &artifact.routines {
        if !routine.exported {
            continue;
        }
        let surface_exported = exported_surface
            .as_ref()
            .map(|rows| {
                rows.contains(&(
                    routine.module_id.as_str(),
                    routine.symbol_kind.as_str(),
                    routine.signature_row.as_str(),
                ))
            })
            .unwrap_or_else(|| {
                routine.module_id == artifact.root_module_id
                    || routine.module_id.starts_with(&root_prefix)
            });
        if !surface_exported {
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
        if !routine.type_param_rows.is_empty() {
            return Err(format!(
                "windows-dll target does not support generic export `{}`",
                routine.routine_key
            ));
        }

        let NativeRoutineSignature {
            params,
            return_type,
        } = parse_native_routine_signature(&routine.param_rows, &routine.signature_row).map_err(
            |err| {
                format!(
                    "windows-dll target cannot export `{}`: {err}",
                    routine.routine_key
                )
            },
        )?;
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
            params,
            return_type,
        });
    }

    Ok(exports)
}

fn exported_function_surface_rows(
    artifact: &AotPackageArtifact,
) -> Option<BTreeSet<(&str, &str, &str)>> {
    let mut rows = BTreeSet::new();
    for row in &artifact.exported_surface_rows {
        let Some(payload) = row.strip_prefix("module=") else {
            continue;
        };
        let Some((module_id, surface_row)) = payload.split_once(':') else {
            continue;
        };
        let Some(surface_payload) = surface_row.strip_prefix("export:") else {
            continue;
        };
        let Some((kind, signature)) = surface_payload.split_once(':') else {
            continue;
        };
        rows.insert((module_id, kind, signature));
    }
    if rows.is_empty() { None } else { Some(rows) }
}

pub fn parse_native_routine_signature(
    param_rows: &[String],
    signature_row: &str,
) -> Result<NativeRoutineSignature, String> {
    Ok(NativeRoutineSignature {
        params: param_rows
            .iter()
            .map(|row| parse_native_param_row(row))
            .collect::<Result<Vec<_>, _>>()?,
        return_type: parse_native_return_type(signature_row)?,
    })
}

pub fn parse_native_param_row(text: &str) -> Result<NativeAbiParam, String> {
    let parts = text.splitn(3, ':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!("malformed runtime param row `{text}`"));
    }
    let name = parts[1]
        .strip_prefix("name=")
        .ok_or_else(|| format!("param row missing name in `{text}`"))?;
    let ty = parts[2]
        .strip_prefix("ty=")
        .ok_or_else(|| format!("param row missing ty in `{text}`"))?;
    Ok(NativeAbiParam {
        name: sanitize_name(name),
        ty: parse_native_type(ty)?,
    })
}

pub fn parse_native_return_type(signature_row: &str) -> Result<NativeAbiType, String> {
    let Some((_, tail)) = signature_row.rsplit_once("->") else {
        return Ok(NativeAbiType::Unit);
    };
    parse_native_type(tail.trim().trim_end_matches(':').trim())
}

fn parse_native_type(text: &str) -> Result<NativeAbiType, String> {
    let text = text.trim();
    if let Some(inner) = text
        .strip_prefix("Pair[")
        .and_then(|rest| rest.strip_suffix(']'))
    {
        let parts = split_top_level_type_items(inner, ',');
        let [left, right] = parts.as_slice() else {
            return Err(format!(
                "pair native abi type must have exactly two items: `{text}`"
            ));
        };
        return Ok(NativeAbiType::Pair(
            Box::new(parse_native_type(left)?),
            Box::new(parse_native_type(right)?),
        ));
    }
    if let Some(inner) = text
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let parts = split_top_level_type_items(inner, ',');
        let [left, right] = parts.as_slice() else {
            return Err(format!(
                "tuple native abi type must have exactly two items: `{text}`"
            ));
        };
        return Ok(NativeAbiType::Pair(
            Box::new(parse_native_type(left)?),
            Box::new(parse_native_type(right)?),
        ));
    }
    match text {
        "Int" => Ok(NativeAbiType::Int),
        "Bool" => Ok(NativeAbiType::Bool),
        "Str" => Ok(NativeAbiType::Str),
        "Array[Int]" => Ok(NativeAbiType::Bytes),
        "Unit" | "" => Ok(NativeAbiType::Unit),
        other => Err(format!("unsupported native abi type `{other}`")),
    }
}

fn split_top_level_type_items(text: &str, delimiter: char) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth_square = 0usize;
    let mut start = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => depth_square += 1,
            ']' => depth_square = depth_square.saturating_sub(1),
            _ if ch == delimiter && depth_square == 0 => {
                items.push(text[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    items.push(text[start..].trim());
    items
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
