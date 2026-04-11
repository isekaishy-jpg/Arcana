use crate::artifact::{
    AotNativeCallbackArtifact, AotPackageArtifact, AotRoutineArtifact, AotShackleDeclArtifact,
    AotShackleImportTargetArtifact, AotShackleThunkTargetArtifact,
};
use arcana_cabi::{
    ArcanaCabiBindingCallback, ArcanaCabiBindingImport, ArcanaCabiBindingLayout,
    ArcanaCabiBindingLayoutField, ArcanaCabiBindingLayoutKind, ArcanaCabiBindingParam,
    ArcanaCabiBindingRawType, ArcanaCabiBindingScalarType, ArcanaCabiBindingType, ArcanaCabiExport,
    ArcanaCabiExportParam, ArcanaCabiType, validate_binding_callbacks, validate_binding_imports,
    validate_binding_layouts,
};
use arcana_ir::{IrRoutineParam, IrRoutineType, IrRoutineTypeKind, parse_routine_type_text};
use std::collections::{BTreeMap, BTreeSet};

pub type NativeAbiType = ArcanaCabiType;
pub type NativeAbiParam = ArcanaCabiExportParam;
pub type NativeExport = ArcanaCabiExport;
pub type NativeBindingType = ArcanaCabiBindingType;
pub type NativeBindingParam = ArcanaCabiBindingParam;
pub type NativeBindingImport = ArcanaCabiBindingImport;
pub type NativeBindingCallback = ArcanaCabiBindingCallback;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRoutineSignature {
    pub params: Vec<NativeAbiParam>,
    pub return_type: NativeAbiType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeBindingRoutineSignature {
    pub params: Vec<NativeBindingParam>,
    pub return_type: NativeBindingType,
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

pub fn collect_native_binding_imports(
    artifact: &AotPackageArtifact,
) -> Result<Vec<NativeBindingImport>, String> {
    let imports = artifact
        .routines
        .iter()
        .filter(|routine| routine.package_id == artifact.package_id)
        .filter_map(|routine| {
            routine
                .native_impl
                .as_ref()
                .map(|binding_name| (routine, binding_name))
        })
        .map(|(routine, binding_name)| {
            let NativeBindingRoutineSignature {
                params,
                return_type,
            } = native_binding_routine_signature(routine).map_err(|err| {
                format!(
                    "binding import `{}` cannot lower routine `{}`: {err}",
                    binding_name, routine.routine_key
                )
            })?;
            Ok(NativeBindingImport {
                name: binding_name.clone(),
                symbol_name: default_binding_import_symbol_name(&artifact.package_id, binding_name),
                return_type,
                params,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    validate_binding_imports(&imports)?;
    Ok(imports)
}

pub fn collect_native_binding_callbacks(
    artifact: &AotPackageArtifact,
) -> Result<Vec<NativeBindingCallback>, String> {
    let callbacks = artifact
        .native_callbacks
        .iter()
        .filter(|callback| callback.package_id == artifact.package_id)
        .map(|callback| {
            let NativeBindingRoutineSignature {
                params,
                return_type,
            } = native_binding_callback_signature(callback).map_err(|err| {
                format!(
                    "binding callback `{}` cannot lower callback target metadata: {err}",
                    callback.name
                )
            })?;
            Ok(NativeBindingCallback {
                name: callback.name.clone(),
                return_type,
                params,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    validate_binding_callbacks(&callbacks)?;
    Ok(callbacks)
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

#[allow(dead_code)]
pub fn native_callback_signature(
    callback: &AotNativeCallbackArtifact,
) -> Result<NativeRoutineSignature, String> {
    Ok(NativeRoutineSignature {
        params: callback
            .params
            .iter()
            .map(parse_native_param)
            .collect::<Result<Vec<_>, _>>()?,
        return_type: parse_native_return_type(callback.return_type.as_ref())?,
    })
}

pub fn native_binding_routine_signature(
    routine: &AotRoutineArtifact,
) -> Result<NativeBindingRoutineSignature, String> {
    Ok(NativeBindingRoutineSignature {
        params: routine
            .params
            .iter()
            .map(parse_native_binding_param)
            .collect::<Result<Vec<_>, _>>()?,
        return_type: parse_native_binding_return_type(routine.return_type.as_ref())?,
    })
}

pub fn native_binding_callback_signature(
    callback: &AotNativeCallbackArtifact,
) -> Result<NativeBindingRoutineSignature, String> {
    Ok(NativeBindingRoutineSignature {
        params: callback
            .params
            .iter()
            .map(parse_native_binding_param)
            .collect::<Result<Vec<_>, _>>()?,
        return_type: parse_native_binding_return_type(callback.return_type.as_ref())?,
    })
}

pub fn parse_native_param(param: &IrRoutineParam) -> Result<NativeAbiParam, String> {
    let ty = parse_native_type(&param.ty)?;
    Ok(ArcanaCabiExportParam::binding(
        sanitize_name(&param.name),
        arcana_cabi::ArcanaCabiParamSourceMode::from_param_mode_text(param.mode.as_deref())?,
        ty,
    ))
}

pub fn parse_native_binding_param(param: &IrRoutineParam) -> Result<NativeBindingParam, String> {
    let ty = parse_native_binding_type(&param.ty)?;
    Ok(ArcanaCabiBindingParam::binding(
        sanitize_name(&param.name),
        arcana_cabi::ArcanaCabiParamSourceMode::from_param_mode_text(param.mode.as_deref())?,
        ty,
    ))
}

pub fn parse_native_return_type(
    return_type: Option<&IrRoutineType>,
) -> Result<NativeAbiType, String> {
    return_type
        .map(parse_native_type)
        .transpose()
        .map(|ty| ty.unwrap_or(NativeAbiType::Unit))
}

pub fn parse_native_binding_return_type(
    return_type: Option<&IrRoutineType>,
) -> Result<NativeBindingType, String> {
    return_type
        .map(parse_native_binding_type)
        .transpose()
        .map(|ty| ty.unwrap_or(NativeBindingType::Unit))
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

fn parse_native_binding_type(ty: &IrRoutineType) -> Result<NativeBindingType, String> {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => match path.root_name() {
            Some("Int") => Ok(NativeBindingType::Int),
            Some("Bool") => Ok(NativeBindingType::Bool),
            Some("Str") => Ok(NativeBindingType::Str),
            Some("Unit") => Ok(NativeBindingType::Unit),
            Some("I8") => Ok(NativeBindingType::I8),
            Some("U8") => Ok(NativeBindingType::U8),
            Some("I16") => Ok(NativeBindingType::I16),
            Some("U16") => Ok(NativeBindingType::U16),
            Some("I32") => Ok(NativeBindingType::I32),
            Some("U32") => Ok(NativeBindingType::U32),
            Some("I64") => Ok(NativeBindingType::I64),
            Some("U64") => Ok(NativeBindingType::U64),
            Some("ISize") => Ok(NativeBindingType::ISize),
            Some("USize") => Ok(NativeBindingType::USize),
            Some("F32") => Ok(NativeBindingType::F32),
            Some("F64") => Ok(NativeBindingType::F64),
            _ => Ok(NativeBindingType::Named(ty.render())),
        },
        IrRoutineTypeKind::Apply { base, args } => match base.root_name() {
            Some("Array") if args.len() == 1 && args[0].root_name() == Some("Int") => {
                Ok(NativeBindingType::Bytes)
            }
            _ => Ok(NativeBindingType::Named(ty.render())),
        },
        IrRoutineTypeKind::Tuple(_)
        | IrRoutineTypeKind::Ref { .. }
        | IrRoutineTypeKind::Projection(_) => Ok(NativeBindingType::Named(ty.render())),
    }
}

pub fn collect_binding_layouts(
    artifact: &AotPackageArtifact,
) -> Result<Vec<ArcanaCabiBindingLayout>, String> {
    let available = artifact
        .shackle_decls
        .iter()
        .filter_map(|decl| {
            decl.raw_layout
                .as_ref()
                .cloned()
                .map(|layout| (layout.layout_id.clone(), layout))
        })
        .collect::<BTreeMap<_, _>>();
    let mut collected = BTreeMap::<String, ArcanaCabiBindingLayout>::new();
    let mut visiting = BTreeSet::<String>::new();
    for import in collect_native_binding_imports(artifact)? {
        collect_named_binding_type_ids(
            &import.return_type,
            &available,
            &mut collected,
            &mut visiting,
        )?;
        for param in &import.params {
            collect_named_binding_type_ids(
                &param.input_type,
                &available,
                &mut collected,
                &mut visiting,
            )?;
            if let Some(write_back_type) = &param.write_back_type {
                collect_named_binding_type_ids(
                    write_back_type,
                    &available,
                    &mut collected,
                    &mut visiting,
                )?;
            }
        }
    }
    for callback in collect_native_binding_callbacks(artifact)? {
        collect_named_binding_type_ids(
            &callback.return_type,
            &available,
            &mut collected,
            &mut visiting,
        )?;
        for param in &callback.params {
            collect_named_binding_type_ids(
                &param.input_type,
                &available,
                &mut collected,
                &mut visiting,
            )?;
            if let Some(write_back_type) = &param.write_back_type {
                collect_named_binding_type_ids(
                    write_back_type,
                    &available,
                    &mut collected,
                    &mut visiting,
                )?;
            }
        }
    }
    let layouts = collected.into_values().collect::<Vec<_>>();
    validate_binding_layouts(&layouts)?;
    Ok(layouts)
}

pub fn populate_typed_shackle_metadata(artifact: &mut AotPackageArtifact) -> Result<(), String> {
    let snapshot = artifact.clone();
    let decls_by_id = snapshot
        .shackle_decls
        .iter()
        .filter_map(|decl| binding_layout_id_for_decl(decl).map(|layout_id| (layout_id, decl)))
        .collect::<BTreeMap<_, _>>();
    let mut builder = BindingLayoutBuilder {
        decls_by_id,
        built: BTreeMap::new(),
        building: BTreeSet::new(),
    };
    let mut raw_layouts = BTreeMap::<usize, ArcanaCabiBindingLayout>::new();
    for (index, decl) in snapshot.shackle_decls.iter().enumerate() {
        let Some(layout_id) = binding_layout_id_for_decl(decl) else {
            continue;
        };
        builder.build(&layout_id)?;
        let layout = builder
            .built
            .get(&layout_id)
            .cloned()
            .ok_or_else(|| format!("missing typed raw layout `{layout_id}` after lowering"))?;
        raw_layouts.insert(index, layout);
    }
    for (index, decl) in artifact.shackle_decls.iter_mut().enumerate() {
        decl.raw_layout = raw_layouts.get(&index).cloned();
        decl.import_target = match decl.kind.as_str() {
            "import fn" | "import_fn" => decl
                .binding
                .as_deref()
                .map(parse_shackle_import_target)
                .transpose()?,
            _ => None,
        };
        decl.thunk_target = match decl.kind.as_str() {
            "thunk" => decl
                .binding
                .as_ref()
                .map(|target| AotShackleThunkTargetArtifact {
                    target: target.clone(),
                    abi: "system".to_string(),
                }),
            _ => None,
        };
    }
    Ok(())
}

struct BindingLayoutBuilder<'a> {
    decls_by_id: BTreeMap<String, &'a crate::artifact::AotShackleDeclArtifact>,
    built: BTreeMap<String, ArcanaCabiBindingLayout>,
    building: BTreeSet<String>,
}

fn binding_layout_id_for_decl(decl: &AotShackleDeclArtifact) -> Option<String> {
    match decl.kind.as_str() {
        "type" | "struct" | "union" | "callback" => {
            Some(binding_layout_id(&decl.module_id, &decl.name))
        }
        "flags" if decl.binding.is_some() => Some(binding_layout_id(&decl.module_id, &decl.name)),
        _ => None,
    }
}

fn collect_named_binding_type_ids(
    ty: &ArcanaCabiBindingType,
    available: &BTreeMap<String, ArcanaCabiBindingLayout>,
    collected: &mut BTreeMap<String, ArcanaCabiBindingLayout>,
    visiting: &mut BTreeSet<String>,
) -> Result<(), String> {
    if let ArcanaCabiBindingType::Named(layout_id) = ty
        && available.contains_key(layout_id)
    {
        collect_binding_layout_id(layout_id, available, collected, visiting)?;
    }
    Ok(())
}

fn collect_binding_layout_id(
    layout_id: &str,
    available: &BTreeMap<String, ArcanaCabiBindingLayout>,
    collected: &mut BTreeMap<String, ArcanaCabiBindingLayout>,
    visiting: &mut BTreeSet<String>,
) -> Result<(), String> {
    if collected.contains_key(layout_id) {
        return Ok(());
    }
    let Some(layout) = available.get(layout_id).cloned() else {
        return Ok(());
    };
    if !visiting.insert(layout_id.to_string()) {
        return Err(format!(
            "recursive raw binding layout cycle at `{layout_id}`"
        ));
    }
    collect_binding_layout_refs(&layout, available, collected, visiting)?;
    visiting.remove(layout_id);
    collected.insert(layout_id.to_string(), layout);
    Ok(())
}

fn collect_binding_layout_refs(
    layout: &ArcanaCabiBindingLayout,
    available: &BTreeMap<String, ArcanaCabiBindingLayout>,
    collected: &mut BTreeMap<String, ArcanaCabiBindingLayout>,
    visiting: &mut BTreeSet<String>,
) -> Result<(), String> {
    match &layout.kind {
        ArcanaCabiBindingLayoutKind::Alias { target } => {
            collect_binding_raw_type_ids(target, available, collected, visiting)
        }
        ArcanaCabiBindingLayoutKind::Struct { fields }
        | ArcanaCabiBindingLayoutKind::Union { fields } => {
            for field in fields {
                collect_binding_raw_type_ids(&field.ty, available, collected, visiting)?;
            }
            Ok(())
        }
        ArcanaCabiBindingLayoutKind::Array { element_type, .. } => {
            collect_binding_raw_type_ids(element_type, available, collected, visiting)
        }
        ArcanaCabiBindingLayoutKind::Enum { .. } | ArcanaCabiBindingLayoutKind::Flags { .. } => {
            Ok(())
        }
        ArcanaCabiBindingLayoutKind::Callback {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_binding_raw_type_ids(param, available, collected, visiting)?;
            }
            collect_binding_raw_type_ids(return_type, available, collected, visiting)
        }
        ArcanaCabiBindingLayoutKind::Interface {
            vtable_layout_id, ..
        } => {
            if let Some(layout_id) = vtable_layout_id {
                collect_binding_layout_id(layout_id, available, collected, visiting)?;
            }
            Ok(())
        }
    }
}

fn collect_binding_raw_type_ids(
    ty: &ArcanaCabiBindingRawType,
    available: &BTreeMap<String, ArcanaCabiBindingLayout>,
    collected: &mut BTreeMap<String, ArcanaCabiBindingLayout>,
    visiting: &mut BTreeSet<String>,
) -> Result<(), String> {
    match ty {
        ArcanaCabiBindingRawType::Void | ArcanaCabiBindingRawType::Scalar(_) => Ok(()),
        ArcanaCabiBindingRawType::Named(layout_id) => {
            collect_binding_layout_id(layout_id, available, collected, visiting)
        }
        ArcanaCabiBindingRawType::Pointer { inner, .. } => {
            collect_binding_raw_type_ids(inner, available, collected, visiting)
        }
        ArcanaCabiBindingRawType::FunctionPointer {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_binding_raw_type_ids(param, available, collected, visiting)?;
            }
            collect_binding_raw_type_ids(return_type, available, collected, visiting)
        }
    }
}

impl BindingLayoutBuilder<'_> {
    fn build(&mut self, layout_id: &str) -> Result<(), String> {
        if self.built.contains_key(layout_id) {
            return Ok(());
        }
        let Some(decl) = self.decls_by_id.get(layout_id).copied() else {
            return Ok(());
        };
        if !self.building.insert(layout_id.to_string()) {
            return Err(format!(
                "recursive shackle raw layout cycle at `{layout_id}`"
            ));
        }
        let layout = match decl.kind.as_str() {
            "type" => self.build_type_layout(decl)?,
            "struct" => self.build_struct_layout(decl)?,
            "union" => self.build_union_layout(decl)?,
            "callback" => self.build_callback_layout(decl)?,
            "flags" => self.build_flags_layout(decl)?,
            other => {
                return Err(format!(
                    "unsupported shackle raw layout declaration kind `{other}` for `{layout_id}`"
                ));
            }
        };
        self.building.remove(layout_id);
        self.built.insert(layout_id.to_string(), layout);
        Ok(())
    }

    fn build_type_layout(
        &mut self,
        decl: &crate::artifact::AotShackleDeclArtifact,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let layout_id = binding_layout_id(&decl.module_id, &decl.name);
        let binding = decl
            .binding
            .as_deref()
            .ok_or_else(|| format!("shackle type `{layout_id}` is missing a raw binding target"))?;
        if let Some((element_type, len)) =
            parse_fixed_array_type_expr(binding, &decl.module_id, self)?
        {
            let (element_size, element_align) = self.raw_type_size_align(&element_type)?;
            return Ok(ArcanaCabiBindingLayout {
                layout_id,
                size: element_size.saturating_mul(len),
                align: element_align,
                kind: ArcanaCabiBindingLayoutKind::Array { element_type, len },
            });
        }
        let target = parse_shackle_raw_type(binding, &decl.module_id, self)?;
        let vtable_layout_id = companion_vtable_layout_id(&decl.module_id, &decl.name);
        if self.decls_by_id.contains_key(&vtable_layout_id)
            && matches!(target, ArcanaCabiBindingRawType::Pointer { .. })
        {
            self.build(&vtable_layout_id)?;
            let size = std::mem::size_of::<usize>();
            return Ok(ArcanaCabiBindingLayout {
                layout_id,
                size,
                align: size,
                kind: ArcanaCabiBindingLayoutKind::Interface {
                    iid: None,
                    vtable_layout_id: Some(vtable_layout_id),
                },
            });
        }
        let (size, align) = self.raw_type_size_align(&target)?;
        Ok(ArcanaCabiBindingLayout {
            layout_id,
            size,
            align,
            kind: ArcanaCabiBindingLayoutKind::Alias { target },
        })
    }

    fn build_struct_layout(
        &mut self,
        decl: &crate::artifact::AotShackleDeclArtifact,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let mut fields = Vec::new();
        let mut offset = 0usize;
        let mut max_align = 1usize;
        let mut active_bitfield: Option<(ArcanaCabiBindingScalarType, usize, usize, usize)> = None;
        for line in &decl.body_entries {
            let parsed = parse_shackle_struct_field(line, &decl.module_id, self)?;
            if let Some(bit_width) = parsed.bit_width {
                let scalar = parsed.scalar.ok_or_else(|| {
                    format!(
                        "bitfield `{}` on `{}` must use a fixed-width integer base type",
                        parsed.name, decl.name
                    )
                })?;
                let storage_size = scalar.size_bytes();
                let storage_align = scalar.align_bytes();
                let storage_bits = storage_size * 8;
                let (storage_offset, next_bit_offset, total_used_bits) =
                    if let Some((active_scalar, current_offset, current_bit_offset, used_bits)) =
                        active_bitfield
                    {
                        if active_scalar == scalar
                            && current_bit_offset + usize::from(bit_width) <= storage_bits
                        {
                            (
                                current_offset,
                                current_bit_offset,
                                used_bits + usize::from(bit_width),
                            )
                        } else {
                            offset = align_up(offset, storage_align);
                            let start = offset;
                            offset += storage_size;
                            (start, 0, usize::from(bit_width))
                        }
                    } else {
                        offset = align_up(offset, storage_align);
                        let start = offset;
                        offset += storage_size;
                        (start, 0, usize::from(bit_width))
                    };
                max_align = max_align.max(storage_align);
                fields.push(ArcanaCabiBindingLayoutField {
                    name: parsed.name,
                    ty: parsed.ty,
                    offset: storage_offset,
                    bit_width: Some(bit_width),
                    bit_offset: Some(
                        u16::try_from(next_bit_offset)
                            .map_err(|_| format!("bitfield offset overflow on `{}`", decl.name))?,
                    ),
                });
                let next = next_bit_offset + usize::from(bit_width);
                active_bitfield = Some((scalar, storage_offset, next, total_used_bits));
                if total_used_bits >= storage_bits {
                    active_bitfield = None;
                }
                continue;
            }
            active_bitfield = None;
            let (field_size, field_align) = self.raw_type_size_align(&parsed.ty)?;
            offset = align_up(offset, field_align);
            fields.push(ArcanaCabiBindingLayoutField {
                name: parsed.name,
                ty: parsed.ty,
                offset,
                bit_width: None,
                bit_offset: None,
            });
            offset += field_size;
            max_align = max_align.max(field_align);
        }
        let size = align_up(offset, max_align);
        Ok(ArcanaCabiBindingLayout {
            layout_id: binding_layout_id(&decl.module_id, &decl.name),
            size,
            align: max_align,
            kind: ArcanaCabiBindingLayoutKind::Struct { fields },
        })
    }

    fn build_union_layout(
        &mut self,
        decl: &crate::artifact::AotShackleDeclArtifact,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let mut fields = Vec::new();
        let mut size = 0usize;
        let mut align = 1usize;
        for line in &decl.body_entries {
            let parsed = parse_shackle_struct_field(line, &decl.module_id, self)?;
            if parsed.bit_width.is_some() {
                return Err(format!(
                    "shackle union `{}` does not support bitfields in the raw binding substrate",
                    decl.name
                ));
            }
            let (field_size, field_align) = self.raw_type_size_align(&parsed.ty)?;
            size = size.max(field_size);
            align = align.max(field_align);
            fields.push(ArcanaCabiBindingLayoutField {
                name: parsed.name,
                ty: parsed.ty,
                offset: 0,
                bit_width: None,
                bit_offset: None,
            });
        }
        Ok(ArcanaCabiBindingLayout {
            layout_id: binding_layout_id(&decl.module_id, &decl.name),
            size: align_up(size, align),
            align,
            kind: ArcanaCabiBindingLayoutKind::Union { fields },
        })
    }

    fn build_callback_layout(
        &mut self,
        decl: &crate::artifact::AotShackleDeclArtifact,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let params = decl
            .params
            .iter()
            .map(|param| parse_shackle_ir_raw_type(&param.ty, &decl.module_id, self))
            .collect::<Result<Vec<_>, _>>()?;
        let return_type = decl
            .return_type
            .as_ref()
            .map(|ty| parse_shackle_ir_raw_type(ty, &decl.module_id, self))
            .transpose()?
            .unwrap_or(ArcanaCabiBindingRawType::Void);
        Ok(ArcanaCabiBindingLayout {
            layout_id: binding_layout_id(&decl.module_id, &decl.name),
            size: std::mem::size_of::<usize>(),
            align: std::mem::size_of::<usize>(),
            kind: ArcanaCabiBindingLayoutKind::Callback {
                abi: "system".to_string(),
                params,
                return_type,
            },
        })
    }

    fn build_flags_layout(
        &mut self,
        decl: &crate::artifact::AotShackleDeclArtifact,
    ) -> Result<ArcanaCabiBindingLayout, String> {
        let layout_id = binding_layout_id(&decl.module_id, &decl.name);
        let binding = decl.binding.as_deref().ok_or_else(|| {
            format!("shackle flags `{layout_id}` is missing a repr binding target")
        })?;
        let Some(repr) = ArcanaCabiBindingScalarType::parse(binding) else {
            return Err(format!(
                "shackle flags `{layout_id}` repr `{binding}` must be a scalar integer type"
            ));
        };
        Ok(ArcanaCabiBindingLayout {
            layout_id,
            size: repr.size_bytes(),
            align: repr.align_bytes(),
            kind: ArcanaCabiBindingLayoutKind::Flags { repr },
        })
    }

    fn raw_type_size_align(
        &mut self,
        ty: &ArcanaCabiBindingRawType,
    ) -> Result<(usize, usize), String> {
        match ty {
            ArcanaCabiBindingRawType::Void => Ok((0, 1)),
            ArcanaCabiBindingRawType::Scalar(scalar) => {
                Ok((scalar.size_bytes(), scalar.align_bytes()))
            }
            ArcanaCabiBindingRawType::Pointer { .. }
            | ArcanaCabiBindingRawType::FunctionPointer { .. } => {
                let size = std::mem::size_of::<usize>();
                Ok((size, size))
            }
            ArcanaCabiBindingRawType::Named(layout_id) => {
                self.build(layout_id)?;
                let layout = self.built.get(layout_id).ok_or_else(|| {
                    format!("missing referenced raw binding layout `{layout_id}`")
                })?;
                Ok((layout.size, layout.align))
            }
        }
    }
}

fn binding_layout_id(module_id: &str, name: &str) -> String {
    format!("{module_id}.{name}")
}

fn companion_vtable_layout_id(module_id: &str, name: &str) -> String {
    binding_layout_id(module_id, &format!("{name}VTable"))
}

fn align_up(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + (align - 1)) & !(align - 1)
    }
}

fn parse_shackle_import_target(binding: &str) -> Result<AotShackleImportTargetArtifact, String> {
    let (library, symbol) = binding.split_once('.').ok_or_else(|| {
        format!("shackle import binding `{binding}` must be `<library>.<symbol>`")
    })?;
    if library.trim().is_empty() || symbol.trim().is_empty() {
        return Err(format!(
            "shackle import binding `{binding}` must use non-empty library and symbol names"
        ));
    }
    Ok(AotShackleImportTargetArtifact {
        library: library.trim().to_string(),
        symbol: symbol.trim().to_string(),
        abi: "system".to_string(),
    })
}

struct ParsedShackleField {
    name: String,
    ty: ArcanaCabiBindingRawType,
    scalar: Option<ArcanaCabiBindingScalarType>,
    bit_width: Option<u16>,
}

fn parse_shackle_struct_field(
    line: &str,
    module_id: &str,
    builder: &mut BindingLayoutBuilder<'_>,
) -> Result<ParsedShackleField, String> {
    let trimmed = line.trim().trim_end_matches(',');
    let (name, ty_text) = trimmed
        .split_once(':')
        .ok_or_else(|| format!("malformed shackle struct field `{trimmed}`"))?;
    let name = sanitize_name(name.trim());
    let ty_text = ty_text.trim();
    if let Some((base_text, width_text)) = ty_text.rsplit_once(" bits ") {
        let raw = parse_shackle_raw_type(base_text.trim(), module_id, builder)?;
        let scalar = raw_scalar_from_raw_type(&raw);
        let bit_width = width_text
            .trim()
            .parse::<u16>()
            .map_err(|err| format!("invalid shackle bitfield width `{width_text}`: {err}"))?;
        return Ok(ParsedShackleField {
            name,
            ty: raw,
            scalar,
            bit_width: Some(bit_width),
        });
    }
    let raw = parse_shackle_raw_type(ty_text, module_id, builder)?;
    Ok(ParsedShackleField {
        name,
        scalar: raw_scalar_from_raw_type(&raw),
        ty: raw,
        bit_width: None,
    })
}

fn raw_scalar_from_raw_type(ty: &ArcanaCabiBindingRawType) -> Option<ArcanaCabiBindingScalarType> {
    match ty {
        ArcanaCabiBindingRawType::Scalar(scalar) => Some(*scalar),
        _ => None,
    }
}

fn parse_fixed_array_type_expr(
    text: &str,
    module_id: &str,
    builder: &mut BindingLayoutBuilder<'_>,
) -> Result<Option<(ArcanaCabiBindingRawType, usize)>, String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Ok(None);
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let Some((element_text, len_text)) = inner.rsplit_once(';') else {
        return Ok(None);
    };
    let len = len_text
        .trim()
        .parse::<usize>()
        .map_err(|err| format!("invalid fixed array length `{}`: {err}", len_text.trim()))?;
    let element_type = parse_shackle_raw_type(element_text.trim(), module_id, builder)?;
    Ok(Some((element_type, len)))
}

fn parse_shackle_ir_raw_type(
    ty: &IrRoutineType,
    module_id: &str,
    builder: &mut BindingLayoutBuilder<'_>,
) -> Result<ArcanaCabiBindingRawType, String> {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => {
            let rendered = ty.render();
            if let Some(scalar) =
                ArcanaCabiBindingScalarType::parse(path.root_name().unwrap_or(&rendered))
            {
                return Ok(ArcanaCabiBindingRawType::Scalar(scalar));
            }
            Ok(ArcanaCabiBindingRawType::Named(
                resolve_shackle_named_layout_id(module_id, &rendered, builder),
            ))
        }
        IrRoutineTypeKind::Apply { .. }
        | IrRoutineTypeKind::Tuple(_)
        | IrRoutineTypeKind::Ref { .. }
        | IrRoutineTypeKind::Projection(_) => Err(format!(
            "unsupported raw shackle signature type `{}`",
            ty.render()
        )),
    }
}

fn parse_shackle_raw_type(
    text: &str,
    module_id: &str,
    builder: &mut BindingLayoutBuilder<'_>,
) -> Result<ArcanaCabiBindingRawType, String> {
    let trimmed = text.trim();
    if trimmed == "c_void" || trimmed == "()" {
        return Ok(ArcanaCabiBindingRawType::Void);
    }
    if let Some(scalar) = ArcanaCabiBindingScalarType::parse(trimmed) {
        return Ok(ArcanaCabiBindingRawType::Scalar(scalar));
    }
    if let Some(rest) = trimmed.strip_prefix("*mut ") {
        return Ok(ArcanaCabiBindingRawType::Pointer {
            mutable: true,
            inner: Box::new(parse_shackle_raw_type(rest.trim(), module_id, builder)?),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("*const ") {
        return Ok(ArcanaCabiBindingRawType::Pointer {
            mutable: false,
            inner: Box::new(parse_shackle_raw_type(rest.trim(), module_id, builder)?),
        });
    }
    if let Some(function_pointer) = parse_shackle_function_pointer(trimmed, module_id, builder)? {
        return Ok(function_pointer);
    }
    Ok(ArcanaCabiBindingRawType::Named(
        resolve_shackle_named_layout_id(module_id, trimmed, builder),
    ))
}

fn parse_shackle_function_pointer(
    text: &str,
    module_id: &str,
    builder: &mut BindingLayoutBuilder<'_>,
) -> Result<Option<ArcanaCabiBindingRawType>, String> {
    let (nullable, inner) = if text.starts_with("Option<") && text.ends_with('>') {
        (true, &text["Option<".len()..text.len() - 1])
    } else {
        (false, text)
    };
    let inner = inner.trim();
    let inner = inner.strip_prefix("unsafe ").unwrap_or(inner).trim();
    let Some(after_extern) = inner.strip_prefix("extern ") else {
        return Ok(None);
    };
    let Some((abi_text, after_abi)) = after_extern.split_once(" fn(") else {
        return Ok(None);
    };
    let abi = abi_text.trim().trim_matches('"').to_string();
    let Some((params_text, return_text)) = split_signature_param_section(after_abi) else {
        return Err(format!("malformed shackle function pointer `{text}`"));
    };
    let params = split_signature_params(&params_text)
        .into_iter()
        .map(|param_text| parse_shackle_raw_type(&param_text, module_id, builder))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = if return_text.trim().is_empty() {
        ArcanaCabiBindingRawType::Void
    } else {
        let return_text = return_text
            .trim()
            .strip_prefix("->")
            .map(str::trim)
            .unwrap_or("");
        if return_text.is_empty() {
            ArcanaCabiBindingRawType::Void
        } else {
            parse_shackle_raw_type(return_text, module_id, builder)?
        }
    };
    Ok(Some(ArcanaCabiBindingRawType::FunctionPointer {
        abi,
        nullable,
        params,
        return_type: Box::new(return_type),
    }))
}

fn resolve_shackle_named_layout_id(
    module_id: &str,
    name: &str,
    builder: &BindingLayoutBuilder<'_>,
) -> String {
    if name.contains('.') {
        return name.to_string();
    }
    let local = binding_layout_id(module_id, name);
    if builder.decls_by_id.contains_key(&local) {
        return local;
    }
    name.to_string()
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

fn default_binding_import_symbol_name(package_id: &str, binding_name: &str) -> String {
    format!(
        "arcana_binding_import_{}_{}",
        sanitize_name(package_id),
        sanitize_name(binding_name)
    )
}

#[cfg(test)]
mod tests {
    use super::{collect_binding_layouts, populate_typed_shackle_metadata};
    use crate::artifact::{
        AotNativeCallbackArtifact, AotPackageArtifact, AotPackageModuleArtifact,
        AotRoutineArtifact, AotShackleDeclArtifact,
    };
    use arcana_ir::{IrRoutineParam, parse_routine_type_text};
    use std::collections::BTreeMap;

    fn empty_artifact() -> AotPackageArtifact {
        AotPackageArtifact {
            format: "arcana-aot-v9".to_string(),
            package_id: "hostapi".to_string(),
            package_name: "hostapi".to_string(),
            root_module_id: "hostapi".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: BTreeMap::from([("hostapi".to_string(), "hostapi".to_string())]),
            package_direct_dep_ids: BTreeMap::from([("hostapi".to_string(), BTreeMap::new())]),
            module_count: 1,
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
            binding_layouts: Vec::new(),
            owners: Vec::new(),
            modules: vec![AotPackageModuleArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi".to_string(),
                symbol_count: 0,
                item_count: 0,
                line_count: 0,
                non_empty_line_count: 0,
                directive_rows: Vec::new(),
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
        }
    }

    fn test_param(name: &str, ty: &str) -> IrRoutineParam {
        IrRoutineParam {
            binding_id: 0,
            mode: Some("read".to_string()),
            name: name.to_string(),
            ty: parse_routine_type_text(ty).expect("type should parse"),
        }
    }

    #[test]
    fn collect_binding_layouts_builds_typed_raw_layout_metadata() {
        let mut artifact = empty_artifact();
        artifact.routines.push(AotRoutineArtifact {
            package_id: "hostapi".to_string(),
            module_id: "hostapi".to_string(),
            routine_key: "hostapi#fn-0".to_string(),
            symbol_name: "draw".to_string(),
            symbol_kind: "fn".to_string(),
            exported: false,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![
                IrRoutineParam {
                    binding_id: 0,
                    mode: Some("edit".to_string()),
                    name: "rect".to_string(),
                    ty: parse_routine_type_text("hostapi.raw.Rect")
                        .expect("rect type should parse"),
                },
                test_param("words", "hostapi.raw.Words"),
                test_param("proc", "hostapi.raw.WindowProc"),
                test_param("value", "hostapi.raw.ValueUnion"),
            ],
            return_type: Some(
                parse_routine_type_text("hostapi.raw.Rect").expect("return type should parse"),
            ),
            intrinsic_impl: None,
            native_impl: Some("raw.draw".to_string()),
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: Vec::new(),
        });
        artifact.native_callbacks.push(AotNativeCallbackArtifact {
            package_id: "hostapi".to_string(),
            module_id: "hostapi.callbacks".to_string(),
            name: "report".to_string(),
            params: vec![test_param("rect", "hostapi.raw.Rect")],
            return_type: Some(parse_routine_type_text("I32").expect("type should parse")),
            callback_type: None,
            target: vec![
                "hostapi".to_string(),
                "callbacks".to_string(),
                "report".to_string(),
            ],
            target_routine_key: Some("hostapi.callbacks#fn-0".to_string()),
        });
        artifact.shackle_decls = vec![
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "type".to_string(),
                name: "Words".to_string(),
                params: Vec::new(),
                return_type: None,
                callback_type: None,
                binding: Some("[U16; 4]".to_string()),
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "struct".to_string(),
                name: "Rect".to_string(),
                params: Vec::new(),
                return_type: None,
                callback_type: None,
                binding: None,
                body_entries: vec![
                    "left: I32".to_string(),
                    "top: I32".to_string(),
                    "flags: U32 bits 3".to_string(),
                ],
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "union".to_string(),
                name: "ValueUnion".to_string(),
                params: Vec::new(),
                return_type: None,
                callback_type: None,
                binding: None,
                body_entries: vec!["as_int: I32".to_string(), "as_word: U16".to_string()],
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "callback".to_string(),
                name: "WindowProc".to_string(),
                params: vec![test_param("code", "I32")],
                return_type: Some(parse_routine_type_text("I32").expect("type should parse")),
                callback_type: None,
                binding: None,
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "flags".to_string(),
                name: "WindowFlags".to_string(),
                params: Vec::new(),
                return_type: None,
                callback_type: None,
                binding: Some("U32".to_string()),
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
        ];
        populate_typed_shackle_metadata(&mut artifact)
            .expect("typed shackle metadata should populate");
        let layouts = collect_binding_layouts(&artifact).expect("binding layouts should collect");
        let by_id = layouts
            .iter()
            .map(|layout| (layout.layout_id.as_str(), layout))
            .collect::<BTreeMap<_, _>>();

        let rect = by_id
            .get("hostapi.raw.Rect")
            .expect("rect layout should exist");
        match &rect.kind {
            arcana_cabi::ArcanaCabiBindingLayoutKind::Struct { fields } => {
                assert_eq!(rect.size, 12);
                assert_eq!(rect.align, 4);
                assert_eq!(fields.len(), 3);
                assert_eq!(fields[2].bit_width, Some(3));
                assert_eq!(fields[2].bit_offset, Some(0));
            }
            other => panic!("expected struct layout, got {other:?}"),
        }

        let words = by_id
            .get("hostapi.raw.Words")
            .expect("array alias layout should exist");
        match &words.kind {
            arcana_cabi::ArcanaCabiBindingLayoutKind::Array { len, .. } => {
                assert_eq!(*len, 4);
                assert_eq!(words.size, 8);
            }
            other => panic!("expected array layout, got {other:?}"),
        }

        let union = by_id
            .get("hostapi.raw.ValueUnion")
            .expect("union layout should exist");
        match &union.kind {
            arcana_cabi::ArcanaCabiBindingLayoutKind::Union { fields } => {
                assert_eq!(fields.len(), 2);
                assert!(fields.iter().all(|field| field.offset == 0));
            }
            other => panic!("expected union layout, got {other:?}"),
        }

        let callback = by_id
            .get("hostapi.raw.WindowProc")
            .expect("callback layout should exist");
        match &callback.kind {
            arcana_cabi::ArcanaCabiBindingLayoutKind::Callback { abi, params, .. } => {
                assert_eq!(abi, "system");
                assert_eq!(params.len(), 1);
            }
            other => panic!("expected callback layout, got {other:?}"),
        }
    }

    #[test]
    fn populate_typed_shackle_metadata_lowers_import_targets_and_interface_layouts() {
        let mut artifact = empty_artifact();
        artifact.routines.push(AotRoutineArtifact {
            package_id: "hostapi".to_string(),
            module_id: "hostapi".to_string(),
            routine_key: "hostapi#fn-1".to_string(),
            symbol_name: "query".to_string(),
            symbol_kind: "fn".to_string(),
            exported: false,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![test_param("unknown", "hostapi.raw.IUnknown")],
            return_type: Some(
                parse_routine_type_text("hostapi.raw.IUnknown").expect("return type should parse"),
            ),
            intrinsic_impl: None,
            native_impl: Some("raw.query".to_string()),
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: Vec::new(),
        });
        artifact.shackle_decls = vec![
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "struct".to_string(),
                name: "IUnknownVTable".to_string(),
                params: Vec::new(),
                return_type: None,
                callback_type: None,
                binding: None,
                body_entries: vec![
                    "QueryInterface: Option<unsafe extern \"system\" fn(*mut c_void) -> i32>"
                        .to_string(),
                ],
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "type".to_string(),
                name: "IUnknown".to_string(),
                params: Vec::new(),
                return_type: None,
                callback_type: None,
                binding: Some("*mut c_void".to_string()),
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
            AotShackleDeclArtifact {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.raw".to_string(),
                exported: true,
                kind: "import_fn".to_string(),
                name: "CoInitializeEx".to_string(),
                params: Vec::new(),
                return_type: Some(parse_routine_type_text("I32").expect("type should parse")),
                callback_type: None,
                binding: Some("ole32.CoInitializeEx".to_string()),
                body_entries: Vec::new(),
                raw_layout: None,
                import_target: None,
                thunk_target: None,
                surface_text: String::new(),
            },
        ];

        populate_typed_shackle_metadata(&mut artifact)
            .expect("typed shackle metadata should populate");

        let interface_layout = artifact.shackle_decls[1]
            .raw_layout
            .as_ref()
            .expect("interface layout should populate");
        match &interface_layout.kind {
            arcana_cabi::ArcanaCabiBindingLayoutKind::Interface {
                iid,
                vtable_layout_id,
            } => {
                assert_eq!(iid, &None);
                assert_eq!(
                    vtable_layout_id.as_deref(),
                    Some("hostapi.raw.IUnknownVTable")
                );
            }
            other => panic!("expected interface layout, got {other:?}"),
        }
        let import = artifact.shackle_decls[2]
            .import_target
            .as_ref()
            .expect("typed import target should populate");
        assert_eq!(import.library, "ole32");
        assert_eq!(import.symbol, "CoInitializeEx");
        assert_eq!(import.abi, "system");
    }
}
