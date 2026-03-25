use crate::artifact::{AotPackageArtifact, AotRoutineArtifact};
use arcana_ir::{IrRoutineParam, IrRoutineType, IrRoutineTypeKind, render_routine_signature_text};
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
    pub is_edit: bool,
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

fn declared_native_signature_eligible(signature: &str) -> bool {
    let Some(rest) = signature.strip_prefix("fn ") else {
        return false;
    };
    let Some((head, _)) = rest.split_once('(') else {
        return false;
    };
    !head.contains('[')
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
        .map(|(module_id, _, signature)| (module_id.to_string(), signature.to_string()))
        .collect::<BTreeSet<_>>();
    let declared_eligible = declared
        .iter()
        .filter(|(_, signature)| declared_native_signature_eligible(signature))
        .cloned()
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
                render_routine_signature_text(
                    &routine.symbol_kind,
                    &routine.symbol_name,
                    routine.is_async,
                    &routine.type_params,
                    &routine.params,
                    routine.return_type.as_ref(),
                ),
            )
        })
        .collect::<BTreeSet<_>>();

    if structured != declared_eligible {
        return Err(
            "backend artifact native export rows do not match structured routines".to_string(),
        );
    }
    Ok(())
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
    let is_edit = match param.mode.as_deref() {
        None | Some("read") | Some("take") => false,
        Some("edit") => true,
        Some(other) => {
            return Err(format!(
                "unsupported native abi parameter mode `{other}` for `{}`",
                param.name
            ));
        }
    };
    Ok(NativeAbiParam {
        name: sanitize_name(&param.name),
        ty: parse_native_type(&param.ty)?,
        is_edit,
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
            _ => Err(format!("unsupported native abi type `{}`", ty.render())),
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
