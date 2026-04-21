use std::{collections::BTreeMap, ffi::c_void};

use arcana_cabi::{
    ArcanaCabiApiBackendTargetKind, ArcanaCabiApiFieldContract, ArcanaCabiApiFieldMode,
    ArcanaCabiApiLaneKind, ArcanaCabiApiOwnedResultKind, ArcanaCabiApiReleaseFamily,
    ArcanaCabiApiTransferMode, ArcanaCabiBindingType, ArcanaCabiBindingValueTag,
    ArcanaCabiBindingValueV1, ArcanaCabiParamSourceMode, release_binding_output_value,
};
use serde::Serialize;

use super::{
    RuntimeApiDeclPlan, RuntimeBindingArgStorage, RuntimeBindingOpaqueValue,
    RuntimeBindingOwnedRelease, RuntimeCallArg, RuntimeCoreHost, RuntimeExecutionState,
    RuntimeOpaqueValue, RuntimePackagePlan, RuntimeScope, RuntimeTypeBindings, RuntimeValue,
    execute_routine_call_with_state, leak_runtime_binding_text,
    runtime_binding_callback_signatures_for_package, runtime_binding_callback_specs_for_package,
    runtime_binding_decode_layout_value, runtime_binding_import_signatures_for_package,
    runtime_binding_input_from_runtime_value, runtime_binding_layout_by_id, runtime_eval_message,
    runtime_is_hidden_bitfield_storage_field, runtime_value_from_binding_cabi_output,
    validate_runtime_requirements_supported, with_runtime_binding_callback_context,
    with_runtime_native_products,
};

pub const RUNTIME_API_ABI_FORMAT: &str = "arcana-runtime-api-abi-v1";

#[derive(Serialize)]
struct ApiAbiManifest<'a> {
    format: &'static str,
    package_name: &'a str,
    root_module_id: &'a str,
    apis: Vec<ApiAbiDecl<'a>>,
}

#[derive(Serialize)]
struct ApiAbiDecl<'a> {
    api_key: String,
    module_id: &'a str,
    name: &'a str,
    request_type: String,
    response_type: String,
    backend_target_kind: &'static str,
    backend_target: &'a str,
    fields: Vec<ApiAbiField<'a>>,
}

#[derive(Serialize)]
struct ApiAbiField<'a> {
    name: &'a str,
    mode: &'static str,
    lane_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_compat: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transfer_mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owned_result_kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_family: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_target: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    companion_fields: Vec<&'a str>,
    #[serde(skip_serializing_if = "api_abi_false")]
    partial_failure_cleanup: bool,
}

fn api_abi_false(value: &bool) -> bool {
    !*value
}

#[cfg(all(windows, test))]
#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemAlloc(cb: usize) -> *mut c_void;
}

#[cfg(windows)]
#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(pv: *mut c_void);
}

#[cfg(all(windows, test))]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalAlloc(flags: u32, bytes: usize) -> *mut c_void;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(h_mem: *mut c_void) -> *mut c_void;
}

#[cfg(windows)]
#[repr(C)]
struct RuntimeApiIUnknownVtable {
    query_interface: *const c_void,
    add_ref: *const c_void,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[cfg(windows)]
unsafe extern "system" fn runtime_api_owned_bytes_co_task_mem_free(ptr: *mut u8, _len: usize) {
    if !ptr.is_null() {
        unsafe {
            CoTaskMemFree(ptr.cast());
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn runtime_api_owned_str_co_task_mem_free(ptr: *mut u8, len: usize) {
    unsafe {
        runtime_api_owned_bytes_co_task_mem_free(ptr, len);
    }
}

#[cfg(not(windows))]
unsafe extern "system" fn runtime_api_owned_bytes_co_task_mem_free(_ptr: *mut u8, _len: usize) {}

#[cfg(not(windows))]
unsafe extern "system" fn runtime_api_owned_str_co_task_mem_free(_ptr: *mut u8, _len: usize) {}

#[cfg(windows)]
unsafe extern "system" fn runtime_api_owned_bytes_local_free(ptr: *mut u8, _len: usize) {
    if !ptr.is_null() {
        unsafe {
            let _ = LocalFree(ptr.cast());
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn runtime_api_owned_str_local_free(ptr: *mut u8, len: usize) {
    unsafe {
        runtime_api_owned_bytes_local_free(ptr, len);
    }
}

#[cfg(not(windows))]
unsafe extern "system" fn runtime_api_owned_bytes_local_free(_ptr: *mut u8, _len: usize) {}

#[cfg(not(windows))]
unsafe extern "system" fn runtime_api_owned_str_local_free(_ptr: *mut u8, _len: usize) {}

pub fn render_exported_api_abi_manifest(plan: &RuntimePackagePlan) -> Result<String, String> {
    let manifest = ApiAbiManifest {
        format: RUNTIME_API_ABI_FORMAT,
        package_name: &plan.package_name,
        root_module_id: &plan.root_module_id,
        apis: exported_api_decls(plan)
            .into_iter()
            .map(|decl| {
                arcana_cabi::validate_api_contract_fields(&decl.name, &decl.fields)?;
                Ok(ApiAbiDecl {
                    api_key: exported_api_key(decl),
                    module_id: &decl.module_id,
                    name: &decl.name,
                    request_type: decl.request_type.render(),
                    response_type: decl.response_type.render(),
                    backend_target_kind: decl.backend_target_kind.as_str(),
                    backend_target: &decl.backend_target,
                    fields: decl
                        .fields
                        .iter()
                        .map(|field| ApiAbiField {
                            name: &field.name,
                            mode: field.mode.as_str(),
                            lane_kind: field.lane_kind.as_str(),
                            slot: field.binding_slot.map(|slot| slot.as_str()),
                            input_type: field.input_type.as_deref(),
                            output_type: field.output_type.as_deref(),
                            callback_compat: field.callback_compat.as_deref(),
                            transfer_mode: field.transfer_mode.map(|mode| mode.as_str()),
                            owned_result_kind: field.owned_result_kind.map(|kind| kind.as_str()),
                            release_family: field.release_family.map(|family| family.as_str()),
                            release_target: field.release_target.as_deref(),
                            companion_fields: field
                                .companion_fields
                                .iter()
                                .map(String::as_str)
                                .collect(),
                            partial_failure_cleanup: field.partial_failure_cleanup,
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    };
    serde_json::to_string(&manifest)
        .map_err(|err| format!("failed to render runtime api abi manifest: {err}"))
}

pub fn execute_exported_api_abi(
    plan: &RuntimePackagePlan,
    api_ref: &str,
    request_json: &str,
    host: &mut dyn RuntimeCoreHost,
) -> Result<String, String> {
    let request_json = serde_json::from_str::<serde_json::Value>(request_json)
        .map_err(|err| format!("failed to parse runtime api abi request: {err}"))?;
    let api = resolve_exported_api_decl(plan, api_ref)?;
    let request_value = api_request_runtime_value_from_json(api, &request_json)?;
    let response_value = execute_runtime_api_call(plan, api, request_value, host)?;
    let response =
        api_response_json_from_runtime_value(&api.response_type.render(), response_value)?;
    serde_json::to_string(&response)
        .map_err(|err| format!("failed to render runtime api abi response: {err}"))
}

fn exported_api_decls(plan: &RuntimePackagePlan) -> Vec<&RuntimeApiDeclPlan> {
    plan.api_decls.iter().filter(|decl| decl.exported).collect()
}

fn exported_api_key(decl: &RuntimeApiDeclPlan) -> String {
    format!("{}.{}", decl.module_id, decl.name)
}

fn resolve_exported_api_decl<'a>(
    plan: &'a RuntimePackagePlan,
    api_ref: &str,
) -> Result<&'a RuntimeApiDeclPlan, String> {
    let exported = exported_api_decls(plan);
    let exact = exported
        .iter()
        .copied()
        .filter(|decl| exported_api_key(decl) == api_ref)
        .collect::<Vec<_>>();
    if let [decl] = exact.as_slice() {
        return Ok(*decl);
    }
    if exact.len() > 1 {
        return Err(format!(
            "runtime api abi reference `{api_ref}` resolved to multiple exported apis"
        ));
    }
    let by_name = exported
        .iter()
        .copied()
        .filter(|decl| decl.name == api_ref)
        .collect::<Vec<_>>();
    match by_name.as_slice() {
        [decl] => Ok(*decl),
        [] => Err(format!("runtime api abi has no exported api `{api_ref}`")),
        _ => Err(format!(
            "runtime api abi api name `{api_ref}` is ambiguous; use `<module>.<name>`"
        )),
    }
}

pub(super) fn resolve_runtime_api_decl_for_call<'a>(
    plan: &'a RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    callable_path: &[String],
) -> Result<Option<&'a RuntimeApiDeclPlan>, String> {
    let boundary_prefixed = callable_path.len() >= 2
        && callable_path
            .first()
            .and_then(|root| {
                super::resolve_visible_package_id_for_root(plan, current_package_id, root)
            })
            .is_some();
    let mut candidates = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (package_id, module_id, symbol_name) in super::resolve_routine_module_targets(
        plan,
        current_package_id,
        current_module_id,
        callable_path,
    ) {
        for decl in &plan.api_decls {
            if decl.package_id == package_id
                && decl.module_id == module_id
                && decl.name == symbol_name
                && (decl.package_id != current_package_id || boundary_prefixed)
                && seen.insert((
                    decl.package_id.clone(),
                    decl.module_id.clone(),
                    decl.name.clone(),
                ))
            {
                candidates.push(decl);
            }
        }
    }
    match candidates.as_slice() {
        [] => Ok(None),
        [decl] => Ok(Some(*decl)),
        _ => Err(format!(
            "runtime api call `{}` is ambiguous across visible boundary api declarations",
            callable_path.join(".")
        )),
    }
}

pub(super) fn bind_runtime_api_request_for_call(
    api: &RuntimeApiDeclPlan,
    call_args: &[RuntimeCallArg],
) -> Result<RuntimeValue, String> {
    let input_fields = api
        .fields
        .iter()
        .filter(|field| {
            matches!(
                field.mode,
                ArcanaCabiApiFieldMode::In | ArcanaCabiApiFieldMode::InWithWriteBack
            )
        })
        .collect::<Vec<_>>();
    let request_type = api.request_type.render();
    if let [arg] = call_args
        && arg.name.is_none()
        && runtime_api_request_value_matches_contract(&arg.value, &request_type)
    {
        return Ok(arg.value.clone());
    }
    let mut bound = BTreeMap::new();
    let mut next_positional = 0usize;
    for arg in call_args {
        let field = if let Some(name) = arg.name.as_deref() {
            input_fields
                .iter()
                .find(|field| field.name == name)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "runtime api `{}` request contract has no field `{name}`",
                        api.name
                    )
                })?
        } else {
            let Some(field) = input_fields.get(next_positional).copied() else {
                return Err(format!(
                    "runtime api `{}` request received too many positional arguments",
                    api.name
                ));
            };
            next_positional += 1;
            field
        };
        if bound
            .insert(field.name.clone(), arg.value.clone())
            .is_some()
        {
            return Err(format!(
                "runtime api `{}` request received duplicate argument for `{}`",
                api.name, field.name
            ));
        }
    }
    let missing = input_fields
        .iter()
        .filter_map(|field| (!bound.contains_key(&field.name)).then_some(field.name.clone()))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "runtime api `{}` request is missing fields {}",
            api.name,
            missing.join(", ")
        ));
    }
    Ok(RuntimeValue::Struct {
        name: request_type,
        fields: bound,
    })
}

pub(super) fn execute_runtime_api_call(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    request_value: RuntimeValue,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    arcana_cabi::validate_api_contract_fields(&api.name, &api.fields)?;
    validate_runtime_requirements_supported(plan, host)?;
    ensure_runtime_api_contract_name(&api.name, "request", &api.request_type.render())?;
    ensure_runtime_api_contract_name(&api.name, "response", &api.response_type.render())?;
    ensure_runtime_api_contract_value(
        &api.name,
        "request",
        &api.request_type.render(),
        &request_value,
    )?;
    let response_value = match api.backend_target_kind {
        ArcanaCabiApiBackendTargetKind::Arcana => {
            execute_runtime_arcana_api(plan, api, request_value, host)
        }
        ArcanaCabiApiBackendTargetKind::ForeignSymbol
        | ArcanaCabiApiBackendTargetKind::EmbeddedCShim => {
            execute_runtime_binding_api(plan, api, request_value, host)
        }
    }?;
    ensure_runtime_api_contract_value(
        &api.name,
        "response",
        &api.response_type.render(),
        &response_value,
    )?;
    Ok(response_value)
}

fn execute_runtime_arcana_api(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    request_value: RuntimeValue,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let (routine_index, _) = resolve_arcana_api_backend_routine(plan, api)?;
    let mut state = RuntimeExecutionState::default();
    let outcome = execute_routine_call_with_state(
        plan,
        routine_index,
        Vec::new(),
        vec![request_value],
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
                "unsupported runtime api control flow `{other:?}`"
            )),
        }));
    }
    Ok(outcome.value)
}

fn resolve_arcana_api_backend_routine<'a>(
    plan: &'a RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
) -> Result<(usize, &'a super::RuntimeRoutinePlan), String> {
    let target = api.backend_target.trim();
    let matches = plan
        .routines
        .iter()
        .enumerate()
        .filter(|(_, routine)| {
            routine.package_id == api.package_id
                && !routine.is_async
                && routine.type_params.is_empty()
                && arcana_api_backend_matches(api, routine, target)
        })
        .collect::<Vec<_>>();
    let (index, routine) = match matches.as_slice() {
        [matched] => *matched,
        [] => {
            return Err(format!(
                "runtime api `{}` backend target `{target}` does not resolve to a package routine",
                api.name
            ));
        }
        _ => {
            return Err(format!(
                "runtime api `{}` backend target `{target}` resolves to multiple routines",
                api.name
            ));
        }
    };
    if routine.params.len() != 1 {
        return Err(format!(
            "runtime api `{}` backend routine `{}` must take exactly one packed request argument",
            api.name, routine.routine_key
        ));
    }
    if routine.params[0].ty.render() != api.request_type.render() {
        return Err(format!(
            "runtime api `{}` backend routine `{}` request type `{}` does not match api request contract `{}`",
            api.name,
            routine.routine_key,
            routine.params[0].ty.render(),
            api.request_type.render()
        ));
    }
    let actual_response = routine
        .return_type
        .as_ref()
        .map(|ty| ty.render())
        .unwrap_or_else(|| "Unit".to_string());
    if actual_response != api.response_type.render() {
        return Err(format!(
            "runtime api `{}` backend routine `{}` response type `{actual_response}` does not match api response contract `{}`",
            api.name,
            routine.routine_key,
            api.response_type.render()
        ));
    }
    Ok((index, routine))
}

fn arcana_api_backend_matches(
    api: &RuntimeApiDeclPlan,
    routine: &super::RuntimeRoutinePlan,
    target: &str,
) -> bool {
    target == routine.routine_key
        || target == format!("{}.{}", routine.module_id, routine.symbol_name)
        || (routine.module_id == api.module_id && target == routine.symbol_name)
}

fn execute_runtime_binding_api(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    request_value: RuntimeValue,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    arcana_cabi::validate_api_contract_fields(&api.name, &api.fields)?;
    let resolution = arcana_cabi::resolve_api_binding_resolution(&api.name, &api.fields)?;
    let mut request_fields = api_binding_request_fields_from_runtime_value(api, request_value)?;

    let (host_data, host_vtable): (*mut (), *mut ()) =
        unsafe { std::mem::transmute::<&mut dyn RuntimeCoreHost, (*mut (), *mut ())>(host) };
    let mut state = RuntimeExecutionState::default();
    let mut scopes = Vec::<RuntimeScope>::new();
    let type_bindings = RuntimeTypeBindings::default();
    let aliases = BTreeMap::new();
    let mut storage = RuntimeBindingArgStorage::default();
    let cabi_args = resolution
        .param_field_indices
        .iter()
        .enumerate()
        .map(|(param_index, field_index)| {
            let field = &api.fields[*field_index];
            match field.mode {
                ArcanaCabiApiFieldMode::In | ArcanaCabiApiFieldMode::InWithWriteBack => {
                    let value = request_fields.get(&field.name).ok_or_else(|| {
                        format!(
                            "runtime api `{}` request lost field `{}` before binding conversion",
                            api.name, field.name
                        )
                    })?;
                    validate_api_binding_input_field_value(plan, api, field, value)?;
                    let input_type = field.input_type.as_deref().ok_or_else(|| {
                        format!(
                            "runtime api `{}` field `{}` is missing an input transport type",
                            api.name, field.name
                        )
                    })?;
                    let source_mode = match field.mode {
                        ArcanaCabiApiFieldMode::In => ArcanaCabiParamSourceMode::Read,
                        ArcanaCabiApiFieldMode::InWithWriteBack => ArcanaCabiParamSourceMode::Edit,
                        ArcanaCabiApiFieldMode::Out => unreachable!(),
                    };
                    runtime_binding_input_from_runtime_value(
                        &plan.binding_layouts,
                        &api.package_id,
                        input_type,
                        source_mode,
                        value,
                        &mut storage,
                        param_index,
                        &mut scopes,
                        plan,
                        &api.package_id,
                        &api.module_id,
                        &aliases,
                        &type_bindings,
                        &mut state,
                        host,
                    )
                }
                ArcanaCabiApiFieldMode::Out => {
                    let output_type = field.output_type.as_deref().ok_or_else(|| {
                        format!(
                            "runtime api `{}` field `{}` is missing an output transport type",
                            api.name, field.name
                        )
                    })?;
                    let value = api_binding_synthetic_out_runtime_value(
                        plan,
                        &api.package_id,
                        output_type,
                        &field.name,
                    )?;
                    runtime_binding_input_from_runtime_value(
                        &plan.binding_layouts,
                        &api.package_id,
                        output_type,
                        ArcanaCabiParamSourceMode::Edit,
                        &value,
                        &mut storage,
                        param_index,
                        &mut scopes,
                        plan,
                        &api.package_id,
                        &api.module_id,
                        &aliases,
                        &type_bindings,
                        &mut state,
                        host,
                    )
                }
            }
        })
        .collect::<Result<Vec<_>, String>>()?;

    let callback_specs = runtime_binding_callback_specs_for_package(plan, &api.package_id);
    let expected_imports = runtime_binding_import_signatures_for_package(plan, &api.package_id)?;
    let expected_callbacks =
        runtime_binding_callback_signatures_for_package(plan, &api.package_id)?;
    let outcome = with_runtime_binding_callback_context(plan, host_data, host_vtable, || {
        let host_ptr: *mut dyn RuntimeCoreHost =
            unsafe { std::mem::transmute((host_data, host_vtable)) };
        let _ = unsafe { host_ptr.as_mut() }
            .ok_or_else(|| "native api binding lost runtime core host".to_string())?;
        with_runtime_native_products(|catalog| {
            catalog.invoke_binding_import(
                &api.package_id,
                &api.backend_target,
                &callback_specs,
                &expected_imports,
                &expected_callbacks,
                &plan.binding_layouts,
                &cabi_args,
            )
        })
    })?;

    let response_fields = materialize_runtime_binding_api_response_fields(
        plan,
        api,
        &resolution,
        &mut request_fields,
        &outcome,
        &mut state,
    )?;
    runtime_api_response_value_from_fields(api, response_fields)
}

fn runtime_api_request_value_matches_contract(value: &RuntimeValue, contract_name: &str) -> bool {
    matches!(
        value,
        RuntimeValue::Record { name, .. }
            | RuntimeValue::Struct { name, .. }
            | RuntimeValue::Union { name, .. }
            if name == contract_name
    )
}

fn ensure_runtime_api_contract_name(
    api_name: &str,
    label: &str,
    contract_name: &str,
) -> Result<(), String> {
    if contract_name == "Unit" {
        return Err(format!(
            "runtime api `{api_name}` {label} contract must be a packed nominal type, found `Unit`"
        ));
    }
    Ok(())
}

fn ensure_runtime_api_contract_value(
    api_name: &str,
    label: &str,
    contract_name: &str,
    value: &RuntimeValue,
) -> Result<(), String> {
    if runtime_api_request_value_matches_contract(value, contract_name) {
        return Ok(());
    }
    Err(format!(
        "runtime api `{api_name}` {label} contract expects packed `{contract_name}`, found `{}`",
        api_runtime_value_label(value)
    ))
}

fn api_binding_request_fields_from_runtime_value(
    api: &RuntimeApiDeclPlan,
    request_value: RuntimeValue,
) -> Result<BTreeMap<String, RuntimeValue>, String> {
    let input_fields = api
        .fields
        .iter()
        .filter(|field| {
            matches!(
                field.mode,
                ArcanaCabiApiFieldMode::In | ArcanaCabiApiFieldMode::InWithWriteBack
            )
        })
        .map(|field| field.name.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let fields = match request_value {
        RuntimeValue::Record { name, fields }
        | RuntimeValue::Struct { name, fields }
        | RuntimeValue::Union { name, fields } => {
            if name != api.request_type.render() {
                return Err(format!(
                    "runtime api `{}` request contract expects `{}`, found `{name}`",
                    api.name,
                    api.request_type.render()
                ));
            }
            fields
        }
        RuntimeValue::Map(entries) => entries
            .into_iter()
            .map(|(key, value)| match key {
                RuntimeValue::Str(name) => Ok((name, value)),
                other => Err(format!(
                    "runtime api `{}` request map key must be Str, found `{}`",
                    api.name,
                    api_runtime_value_label(&other)
                )),
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?,
        other => {
            return Err(format!(
                "runtime api `{}` request contract expects a packed nominal value, found `{}`",
                api.name,
                api_runtime_value_label(&other)
            ));
        }
    };
    let extras = fields
        .keys()
        .filter(|name| !input_fields.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    if !extras.is_empty() {
        return Err(format!(
            "runtime api `{}` request includes unexpected fields {}",
            api.name,
            extras.join(", ")
        ));
    }
    let missing = input_fields
        .iter()
        .filter(|name| !fields.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "runtime api `{}` request is missing fields {}",
            api.name,
            missing.join(", ")
        ));
    }
    Ok(fields)
}

fn api_runtime_value_label(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Record { name, .. }
        | RuntimeValue::Struct { name, .. }
        | RuntimeValue::Union { name, .. } => name.clone(),
        RuntimeValue::Variant { name, .. } => name.clone(),
        RuntimeValue::Int(_) => "Int".to_string(),
        RuntimeValue::Float { .. } => "Float".to_string(),
        RuntimeValue::Bool(_) => "Bool".to_string(),
        RuntimeValue::Str(_) => "Str".to_string(),
        RuntimeValue::Bytes(_) => "Bytes".to_string(),
        RuntimeValue::ByteBuffer(_) => "ByteBuffer".to_string(),
        RuntimeValue::Utf16(_) => "Utf16".to_string(),
        RuntimeValue::Utf16Buffer(_) => "Utf16Buffer".to_string(),
        RuntimeValue::Tuple(_) => "Tuple".to_string(),
        RuntimeValue::Pair(_, _) => "Pair".to_string(),
        RuntimeValue::Array(_) => "Array".to_string(),
        RuntimeValue::List(_) => "List".to_string(),
        RuntimeValue::Map(_) => "Map".to_string(),
        RuntimeValue::Range { .. } => "Range".to_string(),
        RuntimeValue::OwnerHandle(_) => "Owner".to_string(),
        RuntimeValue::Ref(_) => "Ref".to_string(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) => binding.type_name.to_string(),
        RuntimeValue::Opaque(_) => "Opaque".to_string(),
        RuntimeValue::Unit => "Unit".to_string(),
    }
}

fn validate_api_binding_input_field_value(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    field: &ArcanaCabiApiFieldContract,
    value: &RuntimeValue,
) -> Result<(), String> {
    if !matches!(field.lane_kind, ArcanaCabiApiLaneKind::CallbackToken) {
        return Ok(());
    }
    let compat = field.callback_compat.as_deref().ok_or_else(|| {
        format!(
            "runtime api `{}` callback-token field `{}` is missing compatibility metadata",
            api.name, field.name
        )
    })?;
    if !plan
        .native_callbacks
        .iter()
        .any(|callback| callback.package_id == api.package_id && callback.name == compat)
    {
        return Err(format!(
            "runtime api `{}` callback-token field `{}` expects callback `{compat}`, but no native callback with that name is available in package `{}`",
            api.name, field.name, api.package_id
        ));
    }
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) = value else {
        return Err(format!(
            "runtime api `{}` callback-token field `{}` expects opaque callback `{compat}`, found `{}`",
            api.name,
            field.name,
            api_runtime_value_label(value)
        ));
    };
    if binding.type_name != compat {
        return Err(format!(
            "runtime api `{}` callback-token field `{}` expects `{compat}`, found `{}`",
            api.name, field.name, binding.type_name
        ));
    }
    Ok(())
}

fn runtime_api_response_value_from_fields(
    api: &RuntimeApiDeclPlan,
    fields: BTreeMap<String, RuntimeValue>,
) -> Result<RuntimeValue, String> {
    Ok(RuntimeValue::Struct {
        name: api.response_type.render(),
        fields,
    })
}

fn runtime_api_owned_release_for_field(
    field: &ArcanaCabiApiFieldContract,
) -> Result<Option<RuntimeBindingOwnedRelease>, String> {
    Ok(match field.release_family {
        None | Some(ArcanaCabiApiReleaseFamily::Unsupported) => None,
        Some(ArcanaCabiApiReleaseFamily::Release) => Some(RuntimeBindingOwnedRelease::Release),
        Some(ArcanaCabiApiReleaseFamily::CoTaskMemFree) => {
            Some(RuntimeBindingOwnedRelease::CoTaskMemFree)
        }
        Some(ArcanaCabiApiReleaseFamily::LocalFree) => Some(RuntimeBindingOwnedRelease::LocalFree),
        Some(ArcanaCabiApiReleaseFamily::Custom) => Some(RuntimeBindingOwnedRelease::Custom(
            leak_runtime_binding_text(
                field
                    .release_target
                    .as_deref()
                    .ok_or_else(|| "custom api release target is missing".to_string())?,
            ),
        )),
    })
}

fn runtime_api_owned_output_free_fns(
    field: &ArcanaCabiApiFieldContract,
    outcome: &crate::native_product_loader::RuntimeBindingImportOutcome,
) -> Result<
    (
        arcana_cabi::ArcanaCabiOwnedBytesFreeFn,
        arcana_cabi::ArcanaCabiOwnedStrFreeFn,
    ),
    String,
> {
    match field.release_family {
        Some(ArcanaCabiApiReleaseFamily::Custom)
            if field.release_target.as_deref() == Some("__binding.owned_str_free")
                || field.release_target.as_deref() == Some("__binding.owned_bytes_free") =>
        {
            Ok((outcome.owned_bytes_free, outcome.owned_str_free))
        }
        Some(ArcanaCabiApiReleaseFamily::CoTaskMemFree) => Ok((
            runtime_api_owned_bytes_co_task_mem_free,
            runtime_api_owned_str_co_task_mem_free,
        )),
        Some(ArcanaCabiApiReleaseFamily::LocalFree) => Ok((
            runtime_api_owned_bytes_local_free,
            runtime_api_owned_str_local_free,
        )),
        Some(ArcanaCabiApiReleaseFamily::Custom) => Err(format!(
            "runtime api field `{}` uses custom release target `{}` and must be copied before cleanup",
            field.name,
            field.release_target.as_deref().unwrap_or("<missing>")
        )),
        Some(ArcanaCabiApiReleaseFamily::Release) => Err(format!(
            "runtime api field `{}` cannot decode bytes/text through COM `Release`",
            field.name
        )),
        Some(ArcanaCabiApiReleaseFamily::Unsupported) | None => Err(format!(
            "runtime api field `{}` is missing a supported owned-result release family",
            field.name
        )),
    }
}

fn runtime_api_clone_output_without_release(
    output_type: &str,
    value: &ArcanaCabiBindingValueV1,
    label: &str,
) -> Result<RuntimeValue, String> {
    let binding_type = ArcanaCabiBindingType::parse(output_type)?;
    match (binding_type, value.tag()?) {
        (ArcanaCabiBindingType::Str, ArcanaCabiBindingValueTag::Str) => {
            let owned = unsafe { value.payload.owned_str_value };
            if owned.ptr.is_null() {
                if owned.len == 0 {
                    return Ok(RuntimeValue::Str(String::new()));
                }
                return Err(format!(
                    "{label} returned null owned string with non-zero length {}",
                    owned.len
                ));
            }
            let bytes = unsafe { std::slice::from_raw_parts(owned.ptr, owned.len) }.to_vec();
            Ok(RuntimeValue::Str(String::from_utf8(bytes).map_err(
                |err| format!("{label} string is not utf-8: {err}"),
            )?))
        }
        (ArcanaCabiBindingType::Bytes, ArcanaCabiBindingValueTag::Bytes)
        | (ArcanaCabiBindingType::ByteBuffer, ArcanaCabiBindingValueTag::Bytes)
        | (ArcanaCabiBindingType::Utf16, ArcanaCabiBindingValueTag::Bytes)
        | (ArcanaCabiBindingType::Utf16Buffer, ArcanaCabiBindingValueTag::Bytes) => {
            let owned = unsafe { value.payload.owned_bytes_value };
            if owned.ptr.is_null() {
                if owned.len == 0 {
                    return Ok(match ArcanaCabiBindingType::parse(output_type)? {
                        ArcanaCabiBindingType::Bytes => RuntimeValue::Bytes(Vec::new()),
                        ArcanaCabiBindingType::ByteBuffer => RuntimeValue::ByteBuffer(Vec::new()),
                        ArcanaCabiBindingType::Utf16 => RuntimeValue::Utf16(Vec::new()),
                        ArcanaCabiBindingType::Utf16Buffer => RuntimeValue::Utf16Buffer(Vec::new()),
                        _ => unreachable!(),
                    });
                }
                return Err(format!(
                    "{label} returned null owned bytes with non-zero length {}",
                    owned.len
                ));
            }
            let bytes = unsafe { std::slice::from_raw_parts(owned.ptr, owned.len) }.to_vec();
            if matches!(
                ArcanaCabiBindingType::parse(output_type)?,
                ArcanaCabiBindingType::Utf16 | ArcanaCabiBindingType::Utf16Buffer
            ) && bytes.len() % 2 != 0
            {
                return Err(format!(
                    "{label} utf16 payload must contain an even number of bytes, found {}",
                    bytes.len()
                ));
            }
            Ok(match ArcanaCabiBindingType::parse(output_type)? {
                ArcanaCabiBindingType::Bytes => RuntimeValue::Bytes(bytes),
                ArcanaCabiBindingType::ByteBuffer => RuntimeValue::ByteBuffer(bytes),
                ArcanaCabiBindingType::Utf16 => RuntimeValue::Utf16(
                    bytes
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect(),
                ),
                ArcanaCabiBindingType::Utf16Buffer => RuntimeValue::Utf16Buffer(
                    bytes
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect(),
                ),
                _ => unreachable!(),
            })
        }
        (_, actual) => Err(format!(
            "{label} cannot clone `{output_type}` from binding tag `{actual:?}` without a built-in free function"
        )),
    }
}

fn runtime_api_materialize_binding_output_value(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    field: &ArcanaCabiApiFieldContract,
    output_type: &str,
    value: &ArcanaCabiBindingValueV1,
    state: &mut RuntimeExecutionState,
    outcome: &crate::native_product_loader::RuntimeBindingImportOutcome,
    label: &str,
) -> Result<RuntimeValue, String> {
    let materialized = match field.release_family {
        Some(ArcanaCabiApiReleaseFamily::Custom)
            if field.release_target.as_deref() != Some("__binding.owned_str_free")
                && field.release_target.as_deref() != Some("__binding.owned_bytes_free")
                && matches!(
                    field.owned_result_kind,
                    Some(ArcanaCabiApiOwnedResultKind::String)
                        | Some(ArcanaCabiApiOwnedResultKind::Buffer)
                        | Some(ArcanaCabiApiOwnedResultKind::Array)
                ) =>
        {
            let copied = runtime_api_clone_output_without_release(output_type, value, label)?;
            release_runtime_api_binding_output_slot(plan, api, field, value, outcome)?;
            copied
        }
        _ => {
            let output_binding_type = ArcanaCabiBindingType::parse(output_type)?;
            let needs_owned_free = matches!(
                output_binding_type,
                ArcanaCabiBindingType::Str
                    | ArcanaCabiBindingType::Bytes
                    | ArcanaCabiBindingType::ByteBuffer
                    | ArcanaCabiBindingType::Utf16
                    | ArcanaCabiBindingType::Utf16Buffer
            ) || matches!(
                value.tag()?,
                ArcanaCabiBindingValueTag::Layout | ArcanaCabiBindingValueTag::View
            );
            let (owned_bytes_free, owned_str_free) =
                if matches!(field.lane_kind, ArcanaCabiApiLaneKind::OwnedTransfer)
                    && needs_owned_free
                {
                    runtime_api_owned_output_free_fns(field, outcome)?
                } else {
                    (outcome.owned_bytes_free, outcome.owned_str_free)
                };
            runtime_value_from_binding_cabi_output(
                &plan.binding_layouts,
                &api.package_id,
                output_type,
                value,
                state,
                owned_bytes_free,
                owned_str_free,
                label,
            )?
        }
    };
    if !matches!(field.lane_kind, ArcanaCabiApiLaneKind::OwnedTransfer) {
        return Ok(materialized);
    }
    let owned_release = runtime_api_owned_release_for_field(field)?;
    match materialized {
        RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) => Ok(RuntimeValue::Opaque(
            RuntimeOpaqueValue::Binding(RuntimeBindingOpaqueValue {
                owned_release,
                ..binding
            }),
        )),
        other => Ok(other),
    }
}

fn materialize_runtime_binding_api_response_fields(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    resolution: &arcana_cabi::ArcanaCabiApiBindingResolution,
    request_fields: &mut BTreeMap<String, RuntimeValue>,
    outcome: &crate::native_product_loader::RuntimeBindingImportOutcome,
    state: &mut RuntimeExecutionState,
) -> Result<BTreeMap<String, RuntimeValue>, String> {
    let mut response = BTreeMap::new();
    let mut consumed_fields = std::collections::BTreeSet::new();
    let collected = (|| -> Result<BTreeMap<String, RuntimeValue>, String> {
        for (param_index, field_index) in resolution.param_field_indices.iter().enumerate() {
            let field = &api.fields[*field_index];
            match field.mode {
                ArcanaCabiApiFieldMode::In => {}
                ArcanaCabiApiFieldMode::InWithWriteBack => {
                    let output_type = field.output_type.as_deref().ok_or_else(|| {
                        format!(
                            "runtime api `{}` field `{}` is missing an output transport type",
                            api.name, field.name
                        )
                    })?;
                    let write_back = &outcome.write_backs[param_index];
                    let value = if write_back.tag()? == ArcanaCabiBindingValueTag::Unit {
                        request_fields.remove(&field.name).ok_or_else(|| {
                            format!(
                                "runtime api `{}` request lost write-back field `{}`",
                                api.name, field.name
                            )
                        })?
                    } else {
                        validate_api_binding_output_field_metadata(api, field, output_type)?;
                        runtime_api_materialize_binding_output_value(
                            plan,
                            api,
                            field,
                            output_type,
                            write_back,
                            state,
                            outcome,
                            "runtime api binding write-back",
                        )?
                    };
                    response.insert(field.name.clone(), value);
                    consumed_fields.insert(*field_index);
                }
                ArcanaCabiApiFieldMode::Out => {
                    let output_type = field.output_type.as_deref().ok_or_else(|| {
                        format!(
                            "runtime api `{}` field `{}` is missing an output transport type",
                            api.name, field.name
                        )
                    })?;
                    let write_back = &outcome.write_backs[param_index];
                    if write_back.tag()? == ArcanaCabiBindingValueTag::Unit {
                        return Err(format!(
                            "runtime api `{}` field `{}` expected a synthetic out-param write-back but the binding backend returned `Unit`",
                            api.name, field.name
                        ));
                    }
                    validate_api_binding_output_field_metadata(api, field, output_type)?;
                    let value = runtime_api_materialize_binding_output_value(
                        plan,
                        api,
                        field,
                        output_type,
                        write_back,
                        state,
                        outcome,
                        "runtime api binding synthetic out-param",
                    )?;
                    response.insert(field.name.clone(), value);
                    consumed_fields.insert(*field_index);
                }
            }
        }
        if let Some(field_index) = resolution.return_field_index {
            let field = &api.fields[field_index];
            let output_type = field.output_type.as_deref().ok_or_else(|| {
                format!(
                    "runtime api `{}` field `{}` is missing an output transport type",
                    api.name, field.name
                )
            })?;
            validate_api_binding_output_field_metadata(api, field, output_type)?;
            let value = runtime_api_materialize_binding_output_value(
                plan,
                api,
                field,
                output_type,
                &outcome.result,
                state,
                outcome,
                "runtime api binding result",
            )?;
            response.insert(field.name.clone(), value);
            consumed_fields.insert(field_index);
        }
        validate_api_binding_response_companions(api, &response)?;
        Ok(response)
    })();
    match collected {
        Ok(response) => Ok(response),
        Err(err) => {
            cleanup_runtime_api_binding_partial_failure(
                plan,
                api,
                resolution,
                outcome,
                &consumed_fields,
            );
            Err(err)
        }
    }
}

fn validate_api_binding_output_field_metadata(
    api: &RuntimeApiDeclPlan,
    field: &ArcanaCabiApiFieldContract,
    output_type: &str,
) -> Result<(), String> {
    if !matches!(field.lane_kind, ArcanaCabiApiLaneKind::OwnedTransfer) {
        return Ok(());
    }
    if field.transfer_mode != Some(ArcanaCabiApiTransferMode::CalleeOwned) {
        return Err(format!(
            "runtime api `{}` owned-transfer field `{}` must use transfer mode `callee-owned`",
            api.name, field.name
        ));
    }
    let owned_kind = field.owned_result_kind.ok_or_else(|| {
        format!(
            "runtime api `{}` owned-transfer field `{}` is missing owned-result metadata",
            api.name, field.name
        )
    })?;
    let binding_type = ArcanaCabiBindingType::parse(output_type)?;
    match owned_kind {
        ArcanaCabiApiOwnedResultKind::String => {
            if output_type != "Str" {
                return Err(format!(
                    "runtime api `{}` owned string field `{}` must use output type `Str`, found `{output_type}`",
                    api.name, field.name
                ));
            }
            if !matches!(
                field.release_family,
                Some(ArcanaCabiApiReleaseFamily::Custom)
                    | Some(ArcanaCabiApiReleaseFamily::CoTaskMemFree)
                    | Some(ArcanaCabiApiReleaseFamily::LocalFree)
            ) {
                return Err(format!(
                    "runtime api `{}` owned string field `{}` must use `custom`, `co-task-mem-free`, or `local-free` release metadata",
                    api.name, field.name
                ));
            }
        }
        ArcanaCabiApiOwnedResultKind::Buffer | ArcanaCabiApiOwnedResultKind::Array => {
            if !matches!(
                output_type,
                "Bytes" | "ByteBuffer" | "Utf16" | "Utf16Buffer"
            ) {
                return Err(format!(
                    "runtime api `{}` owned {} field `{}` uses unsupported output type `{output_type}`",
                    api.name,
                    owned_kind.as_str(),
                    field.name
                ));
            }
            if !matches!(
                field.release_family,
                Some(ArcanaCabiApiReleaseFamily::Custom)
                    | Some(ArcanaCabiApiReleaseFamily::CoTaskMemFree)
                    | Some(ArcanaCabiApiReleaseFamily::LocalFree)
            ) {
                return Err(format!(
                    "runtime api `{}` owned {} field `{}` must use `custom`, `co-task-mem-free`, or `local-free` release metadata",
                    api.name,
                    owned_kind.as_str(),
                    field.name
                ));
            }
        }
        ArcanaCabiApiOwnedResultKind::Opaque | ArcanaCabiApiOwnedResultKind::Interface => {
            if !matches!(binding_type, ArcanaCabiBindingType::Named(_)) {
                return Err(format!(
                    "runtime api `{}` owned {} field `{}` must use a named binding transport type, found `{output_type}`",
                    api.name,
                    owned_kind.as_str(),
                    field.name
                ));
            }
        }
    }
    Ok(())
}

fn validate_api_binding_response_companions(
    api: &RuntimeApiDeclPlan,
    response: &BTreeMap<String, RuntimeValue>,
) -> Result<(), String> {
    for field in &api.fields {
        if field.companion_fields.is_empty() {
            continue;
        }
        if !response.contains_key(&field.name) {
            continue;
        }
        let missing = field
            .companion_fields
            .iter()
            .filter(|name| !response.contains_key(*name))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "runtime api `{}` field `{}` is missing companion response fields {}",
                api.name,
                field.name,
                missing.join(", ")
            ));
        }
    }
    Ok(())
}

fn cleanup_runtime_api_binding_partial_failure(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    resolution: &arcana_cabi::ArcanaCabiApiBindingResolution,
    outcome: &crate::native_product_loader::RuntimeBindingImportOutcome,
    consumed_fields: &std::collections::BTreeSet<usize>,
) {
    for (param_index, field_index) in resolution.param_field_indices.iter().enumerate() {
        if consumed_fields.contains(field_index) {
            continue;
        }
        let field = &api.fields[*field_index];
        if !field.partial_failure_cleanup {
            continue;
        }
        let slot = &outcome.write_backs[param_index];
        let _ = release_runtime_api_binding_output_slot(plan, api, field, slot, outcome);
    }
    if let Some(field_index) = resolution.return_field_index
        && !consumed_fields.contains(&field_index)
    {
        let field = &api.fields[field_index];
        if field.partial_failure_cleanup {
            let _ =
                release_runtime_api_binding_output_slot(plan, api, field, &outcome.result, outcome);
        }
    }
}

fn release_runtime_api_binding_output_slot(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    field: &ArcanaCabiApiFieldContract,
    value: &ArcanaCabiBindingValueV1,
    outcome: &crate::native_product_loader::RuntimeBindingImportOutcome,
) -> Result<(), String> {
    if value.tag()? == ArcanaCabiBindingValueTag::Unit {
        return Ok(());
    }
    match field.release_family {
        Some(ArcanaCabiApiReleaseFamily::Custom)
            if field.release_target.as_deref() == Some("__binding.owned_str_free")
                || field.release_target.as_deref() == Some("__binding.owned_bytes_free") =>
        {
            unsafe {
                let _ = release_binding_output_value(
                    *value,
                    outcome.owned_bytes_free,
                    outcome.owned_str_free,
                );
            };
            Ok(())
        }
        Some(ArcanaCabiApiReleaseFamily::Custom) => {
            runtime_api_invoke_custom_release_target(plan, api, field, value)
        }
        Some(ArcanaCabiApiReleaseFamily::CoTaskMemFree) => {
            runtime_api_release_binding_value_with_fns(
                *value,
                runtime_api_owned_bytes_co_task_mem_free,
                runtime_api_owned_str_co_task_mem_free,
            )
        }
        Some(ArcanaCabiApiReleaseFamily::LocalFree) => runtime_api_release_binding_value_with_fns(
            *value,
            runtime_api_owned_bytes_local_free,
            runtime_api_owned_str_local_free,
        ),
        Some(ArcanaCabiApiReleaseFamily::Release) => {
            runtime_api_release_binding_opaque_with_release(value)
        }
        Some(ArcanaCabiApiReleaseFamily::Unsupported) | None => Ok(()),
    }
}

fn runtime_api_release_binding_value_with_fns(
    value: ArcanaCabiBindingValueV1,
    owned_bytes_free: arcana_cabi::ArcanaCabiOwnedBytesFreeFn,
    owned_str_free: arcana_cabi::ArcanaCabiOwnedStrFreeFn,
) -> Result<(), String> {
    unsafe { release_binding_output_value(value, owned_bytes_free, owned_str_free) }
}

#[cfg(windows)]
fn runtime_api_release_binding_opaque_with_release(
    value: &ArcanaCabiBindingValueV1,
) -> Result<(), String> {
    if value.tag()? != ArcanaCabiBindingValueTag::Opaque {
        return Err("COM `Release` expects an opaque binding output slot".to_string());
    }
    let handle = unsafe { value.payload.opaque_value };
    if handle == 0 {
        return Ok(());
    }
    let ptr = handle as *mut c_void;
    if ptr.is_null() {
        return Ok(());
    }
    let vtable = unsafe { *(ptr as *mut *mut RuntimeApiIUnknownVtable) };
    if vtable.is_null() {
        return Err("COM `Release` received an opaque pointer with a null vtable".to_string());
    }
    unsafe {
        ((*vtable).release)(ptr);
    }
    Ok(())
}

#[cfg(not(windows))]
fn runtime_api_release_binding_opaque_with_release(
    _value: &ArcanaCabiBindingValueV1,
) -> Result<(), String> {
    Err("COM `Release` cleanup is only available on Windows".to_string())
}

fn runtime_api_invoke_custom_release_target(
    plan: &RuntimePackagePlan,
    api: &RuntimeApiDeclPlan,
    field: &ArcanaCabiApiFieldContract,
    value: &ArcanaCabiBindingValueV1,
) -> Result<(), String> {
    let target = field.release_target.as_deref().ok_or_else(|| {
        format!(
            "runtime api `{}` field `{}` is missing a custom release target",
            api.name, field.name
        )
    })?;
    let callback_specs = runtime_binding_callback_specs_for_package(plan, &api.package_id);
    let expected_imports = runtime_binding_import_signatures_for_package(plan, &api.package_id)?;
    let expected_callbacks =
        runtime_binding_callback_signatures_for_package(plan, &api.package_id)?;
    let outcome = with_runtime_native_products(|catalog| {
        catalog.invoke_binding_import(
            &api.package_id,
            target,
            &callback_specs,
            &expected_imports,
            &expected_callbacks,
            &plan.binding_layouts,
            &[*value],
        )
    })?;
    runtime_api_release_binding_value_with_fns(
        outcome.result,
        outcome.owned_bytes_free,
        outcome.owned_str_free,
    )?;
    for write_back in outcome.write_backs {
        runtime_api_release_binding_value_with_fns(
            write_back,
            outcome.owned_bytes_free,
            outcome.owned_str_free,
        )?;
    }
    Ok(())
}

fn api_binding_synthetic_out_runtime_value(
    plan: &RuntimePackagePlan,
    package_id: &str,
    expected_type: &str,
    field_name: &str,
) -> Result<RuntimeValue, String> {
    let binding_type = ArcanaCabiBindingType::parse(expected_type)?;
    match binding_type {
        ArcanaCabiBindingType::Int
        | ArcanaCabiBindingType::I8
        | ArcanaCabiBindingType::U8
        | ArcanaCabiBindingType::I16
        | ArcanaCabiBindingType::U16
        | ArcanaCabiBindingType::I32
        | ArcanaCabiBindingType::U32
        | ArcanaCabiBindingType::I64
        | ArcanaCabiBindingType::U64
        | ArcanaCabiBindingType::ISize
        | ArcanaCabiBindingType::USize => Ok(RuntimeValue::Int(0)),
        ArcanaCabiBindingType::Bool => Ok(RuntimeValue::Bool(false)),
        ArcanaCabiBindingType::F32 => Ok(RuntimeValue::Float {
            text: "0".to_string(),
            kind: super::ParsedFloatKind::F32,
        }),
        ArcanaCabiBindingType::F64 => Ok(RuntimeValue::Float {
            text: "0".to_string(),
            kind: super::ParsedFloatKind::F64,
        }),
        ArcanaCabiBindingType::Str => Ok(RuntimeValue::Str(String::new())),
        ArcanaCabiBindingType::Bytes => Ok(RuntimeValue::Bytes(Vec::new())),
        ArcanaCabiBindingType::Utf16 => Ok(RuntimeValue::Utf16(Vec::new())),
        ArcanaCabiBindingType::Named(layout_id) => {
            match runtime_binding_layout_by_id(&plan.binding_layouts, &layout_id) {
                Ok(layout) => runtime_binding_decode_layout_value(
                    &plan.binding_layouts,
                    &layout_id,
                    &vec![0u8; layout.size],
                ),
                Err(_) => Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(
                    RuntimeBindingOpaqueValue {
                        package_id: leak_runtime_binding_text(package_id),
                        type_name: leak_runtime_binding_text(expected_type),
                        handle: 0,
                        owned_release: None,
                    },
                ))),
            }
        }
        ArcanaCabiBindingType::Unit => Ok(RuntimeValue::Unit),
        ArcanaCabiBindingType::ByteBuffer
        | ArcanaCabiBindingType::Utf16Buffer
        | ArcanaCabiBindingType::View(_) => Err(format!(
            "runtime api synthetic out-param `{field_name}` cannot use in-place transport type `{expected_type}`"
        )),
    }
}

fn api_request_runtime_value_from_json(
    api: &RuntimeApiDeclPlan,
    request_json: &serde_json::Value,
) -> Result<RuntimeValue, String> {
    ensure_runtime_api_contract_name(&api.name, "request", &api.request_type.render())?;
    let request_value = if let Some(entries) = request_json.as_object() {
        if !entries.keys().any(|key| key.starts_with('$')) {
            let mut fields = BTreeMap::new();
            for (key, value) in entries {
                let contract = api.fields.iter().find(|field| field.name == *key);
                fields.insert(
                    key.clone(),
                    api_runtime_value_from_json(
                        contract.and_then(|field| field.input_type.as_deref()),
                        &api.package_id,
                        value,
                    )?,
                );
            }
            RuntimeValue::Struct {
                name: api.request_type.render(),
                fields,
            }
        } else {
            api_runtime_value_from_json(None, &api.package_id, request_json)?
        }
    } else {
        api_runtime_value_from_json(None, &api.package_id, request_json)?
    };
    ensure_runtime_api_contract_value(
        &api.name,
        "request",
        &api.request_type.render(),
        &request_value,
    )?;
    Ok(request_value)
}

fn api_response_json_from_runtime_value(
    expected_type: &str,
    value: RuntimeValue,
) -> Result<serde_json::Value, String> {
    if expected_type == "Unit" {
        return Err(
            "runtime api abi response contract must be a packed nominal type, found `Unit`"
                .to_string(),
        );
    }
    ensure_runtime_api_contract_value("runtime api abi", "response", expected_type, &value)?;
    match value {
        RuntimeValue::Struct { fields, .. }
        | RuntimeValue::Record { fields, .. }
        | RuntimeValue::Union { fields, .. } => Ok(serde_json::Value::Object(
            fields
                .into_iter()
                .filter(|(key, _)| !runtime_is_hidden_bitfield_storage_field(key))
                .map(|(key, value)| Ok((key, api_runtime_value_to_json(value)?)))
                .collect::<Result<serde_json::Map<_, _>, String>>()?,
        )),
        other => api_runtime_value_to_json(other),
    }
}

fn api_runtime_value_from_json(
    expected_type: Option<&str>,
    package_id: &str,
    value: &serde_json::Value,
) -> Result<RuntimeValue, String> {
    match value {
        serde_json::Value::Null => Ok(RuntimeValue::Unit),
        serde_json::Value::Bool(value) => Ok(RuntimeValue::Bool(*value)),
        serde_json::Value::Number(value) => value
            .as_i64()
            .map(RuntimeValue::Int)
            .ok_or_else(|| "runtime api abi only supports signed 64-bit integers".to_string()),
        serde_json::Value::String(value) => Ok(RuntimeValue::Str(value.clone())),
        serde_json::Value::Array(values) => Ok(RuntimeValue::List(
            values
                .iter()
                .map(|value| api_runtime_value_from_json(None, package_id, value))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        serde_json::Value::Object(entries) => {
            if let Some(type_name) = entries.get("$opaque") {
                let type_name = type_name
                    .as_str()
                    .ok_or_else(|| "`$opaque` must be a string".to_string())?;
                let handle_value = entries
                    .get("handle")
                    .ok_or_else(|| "opaque values must include a `handle` field".to_string())?;
                let package_id = entries
                    .get("package_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_else(|| type_name.split('.').next().unwrap_or(package_id));
                return Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(
                    RuntimeBindingOpaqueValue {
                        package_id: super::leak_runtime_binding_text(package_id),
                        type_name: super::leak_runtime_binding_text(type_name),
                        handle: json_value_to_u64_handle(handle_value)?,
                        owned_release: None,
                    },
                )));
            }
            if let Some(values) = entries.get("$bytes") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$bytes` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::Bytes(
                    values
                        .iter()
                        .map(|value| {
                            let unit = expect_json_int(value, "byte value")?;
                            u8::try_from(unit).map_err(|_| {
                                format!(
                                    "runtime api abi byte value `{unit}` is out of range `0..=255`"
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            if let Some(values) = entries.get("$byte_buffer") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$byte_buffer` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::ByteBuffer(
                    values
                        .iter()
                        .map(|value| {
                            let unit = expect_json_int(value, "byte buffer value")?;
                            u8::try_from(unit).map_err(|_| {
                                format!(
                                    "runtime api abi byte buffer value `{unit}` is out of range `0..=255`"
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            if let Some(values) = entries.get("$utf16") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$utf16` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::Utf16(
                    values
                        .iter()
                        .map(|value| {
                            let unit = expect_json_int(value, "utf16 value")?;
                            u16::try_from(unit).map_err(|_| {
                                format!(
                                    "runtime api abi utf16 value `{unit}` is out of range `0..=65535`"
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            if let Some(values) = entries.get("$utf16_buffer") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$utf16_buffer` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::Utf16Buffer(
                    values
                        .iter()
                        .map(|value| {
                            let unit = expect_json_int(value, "utf16 buffer value")?;
                            u16::try_from(unit).map_err(|_| {
                                format!(
                                    "runtime api abi utf16 buffer value `{unit}` is out of range `0..=65535`"
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            if let Some(values) = entries.get("$array") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$array` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::Array(
                    values
                        .iter()
                        .map(|value| api_runtime_value_from_json(None, package_id, value))
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
                    Box::new(api_runtime_value_from_json(None, package_id, &values[0])?),
                    Box::new(api_runtime_value_from_json(None, package_id, &values[1])?),
                ));
            }
            if let Some(values) = entries.get("$tuple") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$tuple` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::Tuple(
                    values
                        .iter()
                        .map(|value| api_runtime_value_from_json(None, package_id, value))
                        .collect::<Result<Vec<_>, _>>()?,
                ));
            }
            if let Some(values) = entries.get("$map") {
                let values = values
                    .as_array()
                    .ok_or_else(|| "`$map` must contain a JSON array".to_string())?;
                return Ok(RuntimeValue::Map(
                    values
                        .iter()
                        .map(|entry| {
                            let pair = entry.as_array().ok_or_else(|| {
                                "map entries must be two-element arrays".to_string()
                            })?;
                            if pair.len() != 2 {
                                return Err(
                                    "map entries must contain exactly two elements".to_string()
                                );
                            }
                            Ok((
                                api_runtime_value_from_json(None, package_id, &pair[0])?,
                                api_runtime_value_from_json(None, package_id, &pair[1])?,
                            ))
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                ));
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
                return Ok(RuntimeValue::Record {
                    name: name.to_string(),
                    fields: fields
                        .iter()
                        .map(|(key, value)| {
                            Ok((
                                key.clone(),
                                api_runtime_value_from_json(None, package_id, value)?,
                            ))
                        })
                        .collect::<Result<BTreeMap<_, _>, String>>()?,
                });
            }
            if let Some(name) = entries.get("$struct") {
                let name = name
                    .as_str()
                    .ok_or_else(|| "`$struct` must be a string".to_string())?;
                let fields = entries
                    .get("fields")
                    .and_then(serde_json::Value::as_object)
                    .ok_or_else(|| "struct values must include a `fields` object".to_string())?;
                return Ok(RuntimeValue::Struct {
                    name: name.to_string(),
                    fields: fields
                        .iter()
                        .map(|(key, value)| {
                            Ok((
                                key.clone(),
                                api_runtime_value_from_json(None, package_id, value)?,
                            ))
                        })
                        .collect::<Result<BTreeMap<_, _>, String>>()?,
                });
            }
            if let Some(name) = entries.get("$union") {
                let name = name
                    .as_str()
                    .ok_or_else(|| "`$union` must be a string".to_string())?;
                let fields = entries
                    .get("fields")
                    .and_then(serde_json::Value::as_object)
                    .ok_or_else(|| "union values must include a `fields` object".to_string())?;
                return Ok(RuntimeValue::Union {
                    name: name.to_string(),
                    fields: fields
                        .iter()
                        .map(|(key, value)| {
                            Ok((
                                key.clone(),
                                api_runtime_value_from_json(None, package_id, value)?,
                            ))
                        })
                        .collect::<Result<BTreeMap<_, _>, String>>()?,
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
                        .map(|value| api_runtime_value_from_json(None, package_id, value))
                        .collect::<Result<Vec<_>, _>>()?,
                });
            }
            if let Some(expected_type) = expected_type
                && !entries.keys().any(|key| key.starts_with('$'))
            {
                return Ok(RuntimeValue::Struct {
                    name: expected_type.to_string(),
                    fields: entries
                        .iter()
                        .map(|(key, value)| {
                            Ok((
                                key.clone(),
                                api_runtime_value_from_json(None, package_id, value)?,
                            ))
                        })
                        .collect::<Result<BTreeMap<_, _>, String>>()?,
                });
            }
            Ok(RuntimeValue::Map(
                entries
                    .iter()
                    .map(|(key, value)| {
                        Ok((
                            RuntimeValue::Str(key.clone()),
                            api_runtime_value_from_json(None, package_id, value)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            ))
        }
    }
}

fn api_runtime_value_to_json(value: RuntimeValue) -> Result<serde_json::Value, String> {
    match value {
        RuntimeValue::Unit => Ok(serde_json::Value::Null),
        RuntimeValue::Int(value) => Ok(serde_json::Value::Number(value.into())),
        RuntimeValue::Float { text, kind } => {
            let value = match kind {
                arcana_ir::ExecFloatKind::F32 => {
                    text.parse::<f32>().map(f64::from).map_err(|err| {
                        format!("runtime api abi invalid F32 literal `{text}`: {err}")
                    })?
                }
                arcana_ir::ExecFloatKind::F64 => text.parse::<f64>().map_err(|err| {
                    format!("runtime api abi invalid F64 literal `{text}`: {err}")
                })?,
            };
            let number = serde_json::Number::from_f64(value).ok_or_else(|| {
                format!("runtime api abi float `{text}` is not representable as JSON number")
            })?;
            Ok(serde_json::Value::Number(number))
        }
        RuntimeValue::Bool(value) => Ok(serde_json::Value::Bool(value)),
        RuntimeValue::Str(value) => Ok(serde_json::Value::String(value)),
        RuntimeValue::Bytes(bytes) => Ok(serde_json::json!({ "$bytes": bytes })),
        RuntimeValue::ByteBuffer(bytes) => Ok(serde_json::json!({ "$byte_buffer": bytes })),
        RuntimeValue::Utf16(units) => Ok(serde_json::json!({ "$utf16": units })),
        RuntimeValue::Utf16Buffer(units) => Ok(serde_json::json!({ "$utf16_buffer": units })),
        RuntimeValue::List(values) => Ok(serde_json::Value::Array(
            values
                .into_iter()
                .map(api_runtime_value_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        RuntimeValue::Array(values) => Ok(serde_json::json!({
            "$array": values
                .into_iter()
                .map(api_runtime_value_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        })),
        RuntimeValue::Tuple(values) => Ok(serde_json::json!({
            "$tuple": values
                .into_iter()
                .map(api_runtime_value_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        })),
        RuntimeValue::Pair(left, right) => Ok(serde_json::json!({
            "$pair": [
                api_runtime_value_to_json(*left)?,
                api_runtime_value_to_json(*right)?,
            ],
        })),
        RuntimeValue::Map(entries) => {
            let mut object = serde_json::Map::new();
            let mut string_keys = true;
            for (key, value) in &entries {
                let RuntimeValue::Str(key) = key else {
                    string_keys = false;
                    break;
                };
                object.insert(key.clone(), api_runtime_value_to_json(value.clone())?);
            }
            if string_keys {
                Ok(serde_json::Value::Object(object))
            } else {
                Ok(serde_json::json!({
                    "$map": entries
                        .into_iter()
                        .map(|(key, value)| {
                            Ok(serde_json::Value::Array(vec![
                                api_runtime_value_to_json(key)?,
                                api_runtime_value_to_json(value)?,
                            ]))
                        })
                        .collect::<Result<Vec<_>, String>>()?,
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
        RuntimeValue::Record { name, fields } => Ok(serde_json::json!({
            "$record": name,
            "fields": runtime_nominal_fields_to_json(fields)?,
        })),
        RuntimeValue::Struct { name, fields } => Ok(serde_json::json!({
            "$struct": name,
            "fields": runtime_nominal_fields_to_json(fields)?,
        })),
        RuntimeValue::Union { name, fields } => Ok(serde_json::json!({
            "$union": name,
            "fields": runtime_nominal_fields_to_json(fields)?,
        })),
        RuntimeValue::Variant { name, payload } => Ok(serde_json::json!({
            "$variant": name,
            "payload": payload
                .into_iter()
                .map(api_runtime_value_to_json)
                .collect::<Result<Vec<_>, _>>()?,
        })),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) => Ok(serde_json::json!({
            "$opaque": binding.type_name,
            "package_id": binding.package_id,
            "handle": binding.handle.to_string(),
        })),
        RuntimeValue::OwnerHandle(_)
        | RuntimeValue::Ref(_)
        | RuntimeValue::Opaque(_) => Err(
            "runtime api abi does not support owner, reference, or non-binding opaque runtime values"
                .to_string(),
        ),
    }
}

fn runtime_nominal_fields_to_json(
    fields: BTreeMap<String, RuntimeValue>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Object(
        fields
            .into_iter()
            .filter(|(key, _)| !runtime_is_hidden_bitfield_storage_field(key))
            .map(|(key, value)| Ok((key, api_runtime_value_to_json(value)?)))
            .collect::<Result<serde_json::Map<_, _>, String>>()?,
    ))
}

fn json_value_to_u64_handle(value: &serde_json::Value) -> Result<u64, String> {
    match value {
        serde_json::Value::Number(number) => number
            .as_u64()
            .or_else(|| number.as_i64().map(|value| value as u64))
            .ok_or_else(|| "opaque handle must be a 64-bit integer or string".to_string()),
        serde_json::Value::String(text) => text
            .parse::<u64>()
            .or_else(|_| text.parse::<i64>().map(|value| value as u64))
            .map_err(|_| format!("opaque handle `{text}` is not a valid 64-bit integer")),
        _ => Err("opaque handle must be a 64-bit integer or string".to_string()),
    }
}

fn expect_json_int(value: &serde_json::Value, context: &str) -> Result<i64, String> {
    value
        .as_i64()
        .ok_or_else(|| format!("runtime api abi {context} must be a signed 64-bit integer"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BufferedHost, RuntimeParamPlan, RuntimeRoutinePlan};
    use arcana_cabi::ArcanaCabiApiFieldContract;
    use arcana_ir::{ExecExpr, ExecStmt, parse_routine_type_text};
    #[cfg(windows)]
    use std::fs;
    #[cfg(windows)]
    use std::path::{Path, PathBuf};
    #[cfg(windows)]
    use std::sync::{
        Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    };
    #[cfg(windows)]
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(windows)]
    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root should exist")
            .to_path_buf()
    }

    #[cfg(windows)]
    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let dir = repo_root()
            .join("target")
            .join("arcana-runtime-api-abi-tests")
            .join(format!("{label}_{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[cfg(windows)]
    fn runtime_api_abi_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn empty_plan() -> RuntimePackagePlan {
        RuntimePackagePlan {
            package_id: "tool".to_string(),
            package_name: "tool".to_string(),
            root_module_id: "tool".to_string(),
            direct_deps: Vec::new(),
            direct_dep_ids: Vec::new(),
            package_display_names: BTreeMap::new(),
            package_direct_dep_ids: BTreeMap::new(),
            runtime_requirements: Vec::new(),
            foreword_index: Vec::new(),
            foreword_registrations: Vec::new(),
            module_aliases: BTreeMap::new(),
            opaque_family_types: BTreeMap::new(),
            entrypoints: Vec::new(),
            routines: Vec::new(),
            native_callbacks: Vec::new(),
            api_decls: Vec::new(),
            shackle_decls: Vec::new(),
            binding_layouts: Vec::new(),
            owners: Vec::new(),
        }
    }

    #[cfg(windows)]
    fn binding_owned_str(text: &str) -> ArcanaCabiBindingValueV1 {
        ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Str as u32,
            reserved0: 0,
            reserved1: 0,
            payload: arcana_cabi::ArcanaCabiBindingPayloadV1 {
                owned_str_value: arcana_cabi::into_owned_str(text.to_string()),
            },
        }
    }

    #[test]
    fn runtime_api_abi_manifest_lists_exported_apis() {
        let mut plan = empty_plan();
        plan.api_decls.push(RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "GetProcessInfo".to_string(),
            request_type: parse_routine_type_text("ProcessInfoRequest").expect("request type"),
            response_type: parse_routine_type_text("ProcessInfoResponse").expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::ForeignSymbol,
            backend_target: "tool.raw.GetProcessInfo".to_string(),
            fields: vec![
                ArcanaCabiApiFieldContract {
                    name: "process".to_string(),
                    mode: ArcanaCabiApiFieldMode::In,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OpaqueHandle,
                    binding_slot: None,
                    callback_compat: None,
                    transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::Borrowed),
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                    input_type: Some("tool.raw.HANDLE".to_string()),
                    output_type: None,
                },
                ArcanaCabiApiFieldContract {
                    name: "on_event".to_string(),
                    mode: ArcanaCabiApiFieldMode::In,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::CallbackToken,
                    binding_slot: None,
                    callback_compat: Some("tool.callbacks.ProcessCallback".to_string()),
                    transfer_mode: None,
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                    input_type: Some("tool.raw.WNDPROC".to_string()),
                    output_type: None,
                },
                ArcanaCabiApiFieldContract {
                    name: "greeting".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                    binding_slot: None,
                    callback_compat: None,
                    transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                    owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::String),
                    release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::Custom),
                    release_target: Some("__binding.owned_str_free".to_string()),
                    companion_fields: vec!["status".to_string()],
                    partial_failure_cleanup: true,
                    input_type: None,
                    output_type: Some("Str".to_string()),
                },
                ArcanaCabiApiFieldContract {
                    name: "status".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                    binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Param),
                    callback_compat: None,
                    transfer_mode: None,
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                    input_type: None,
                    output_type: Some("Int".to_string()),
                },
            ],
            surface_text: "export api GetProcessInfo".to_string(),
        });

        let manifest =
            render_exported_api_abi_manifest(&plan).expect("api abi manifest should render");
        let manifest =
            serde_json::from_str::<serde_json::Value>(&manifest).expect("manifest should parse");
        assert_eq!(manifest["format"].as_str(), Some(RUNTIME_API_ABI_FORMAT));
        assert_eq!(
            manifest["apis"][0]["api_key"].as_str(),
            Some("tool.api.GetProcessInfo")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][0]["input_type"].as_str(),
            Some("tool.raw.HANDLE")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][1]["callback_compat"].as_str(),
            Some("tool.callbacks.ProcessCallback")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][0]["transfer_mode"].as_str(),
            Some("borrowed")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][2]["owned_result_kind"].as_str(),
            Some("string")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][2]["release_family"].as_str(),
            Some("custom")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][2]["release_target"].as_str(),
            Some("__binding.owned_str_free")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][2]["companion_fields"][0].as_str(),
            Some("status")
        );
        assert_eq!(
            manifest["apis"][0]["fields"][2]["partial_failure_cleanup"].as_bool(),
            Some(true)
        );
        assert_eq!(
            manifest["apis"][0]["fields"][3]["slot"].as_str(),
            Some("param")
        );
    }

    #[test]
    fn runtime_api_abi_executes_arcana_backend() {
        let mut plan = empty_plan();
        plan.routines.push(RuntimeRoutinePlan {
            package_id: "tool".to_string(),
            module_id: "tool".to_string(),
            routine_key: "tool#fn-0".to_string(),
            symbol_name: "double".to_string(),
            symbol_kind: "fn".to_string(),
            exported: true,
            is_async: false,
            type_params: Vec::new(),
            behavior_attrs: BTreeMap::new(),
            params: vec![RuntimeParamPlan {
                binding_id: 0,
                mode: None,
                name: "request".to_string(),
                ty: parse_routine_type_text("tool.api.DoubleEnvelope").expect("param type"),
            }],
            return_type: Some(
                parse_routine_type_text("tool.api.DoubleEnvelope").expect("return type"),
            ),
            intrinsic_impl: None,
            native_impl: None,
            impl_target_type: None,
            impl_trait_path: None,
            availability: Vec::new(),
            cleanup_footers: Vec::new(),
            statements: vec![ExecStmt::ReturnValue {
                value: ExecExpr::Path(vec!["request".to_string()]),
            }],
        });
        plan.api_decls.push(RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool".to_string(),
            exported: true,
            name: "Double".to_string(),
            request_type: parse_routine_type_text("tool.api.DoubleEnvelope").expect("request type"),
            response_type: parse_routine_type_text("tool.api.DoubleEnvelope")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::Arcana,
            backend_target: "double".to_string(),
            fields: vec![ArcanaCabiApiFieldContract {
                name: "value".to_string(),
                mode: ArcanaCabiApiFieldMode::In,
                lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                binding_slot: None,
                input_type: Some("Int".to_string()),
                output_type: None,
                callback_compat: None,
                transfer_mode: None,
                owned_result_kind: None,
                release_family: None,
                release_target: None,
                companion_fields: Vec::new(),
                partial_failure_cleanup: false,
            }],
            surface_text: "export api Double".to_string(),
        });

        let mut host = BufferedHost::default();
        let response =
            execute_exported_api_abi(&plan, "tool.Double", r#"{ "value": 5 }"#, &mut host)
                .expect("api invoke");
        let response =
            serde_json::from_str::<serde_json::Value>(&response).expect("response should parse");
        assert_eq!(response, serde_json::json!({ "value": 5 }));
    }

    #[test]
    fn runtime_api_abi_rejects_callback_token_type_mismatch_before_binding_invoke() {
        let mut plan = empty_plan();
        plan.native_callbacks
            .push(crate::RuntimeNativeCallbackPlan {
                package_id: "tool".to_string(),
                module_id: "tool.callbacks".to_string(),
                name: "tool.callbacks.ProcessCallback".to_string(),
                params: Vec::new(),
                return_type: None,
                target: vec![
                    "tool".to_string(),
                    "callbacks".to_string(),
                    "process_impl".to_string(),
                ],
                target_routine_key: None,
            });
        let api = RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "Invoke".to_string(),
            request_type: parse_routine_type_text("tool.api.InvokeRequest").expect("request type"),
            response_type: parse_routine_type_text("tool.api.InvokeResponse")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::ForeignSymbol,
            backend_target: "raw.callbacks.Invoke".to_string(),
            fields: vec![ArcanaCabiApiFieldContract {
                name: "callback".to_string(),
                mode: ArcanaCabiApiFieldMode::In,
                lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::CallbackToken,
                binding_slot: None,
                input_type: Some("tool.raw.ProcessCallback".to_string()),
                output_type: None,
                callback_compat: Some("tool.callbacks.ProcessCallback".to_string()),
                transfer_mode: None,
                owned_result_kind: None,
                release_family: None,
                release_target: None,
                companion_fields: Vec::new(),
                partial_failure_cleanup: false,
            }],
            surface_text: "export api Invoke".to_string(),
        };
        let request = RuntimeValue::Struct {
            name: "tool.api.InvokeRequest".to_string(),
            fields: BTreeMap::from([(
                "callback".to_string(),
                RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(RuntimeBindingOpaqueValue {
                    package_id: leak_runtime_binding_text("tool"),
                    type_name: leak_runtime_binding_text("tool.callbacks.OtherCallback"),
                    handle: 7,
                    owned_release: None,
                })),
            )]),
        };

        let mut host = BufferedHost::default();
        let err = execute_runtime_api_call(&plan, &api, request, &mut host)
            .expect_err("callback mismatch should fail before binding invocation");

        assert!(
            err.contains(
                "expects `tool.callbacks.ProcessCallback`, found `tool.callbacks.OtherCallback`"
            ),
            "{err}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_rejects_missing_companion_response_fields() {
        unsafe extern "system" fn test_owned_bytes_free(ptr: *mut u8, len: usize) {
            unsafe {
                arcana_cabi::free_owned_bytes(ptr, len);
            }
        }

        unsafe extern "system" fn test_owned_str_free(ptr: *mut u8, len: usize) {
            unsafe {
                arcana_cabi::free_owned_str(ptr, len);
            }
        }

        let api = RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "Greeting".to_string(),
            request_type: parse_routine_type_text("tool.api.GreetingRequest")
                .expect("request type"),
            response_type: parse_routine_type_text("tool.api.GreetingResponse")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
            backend_target: "api.greeting".to_string(),
            fields: vec![
                ArcanaCabiApiFieldContract {
                    name: "status".to_string(),
                    mode: ArcanaCabiApiFieldMode::In,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                    binding_slot: None,
                    input_type: Some("Int".to_string()),
                    output_type: None,
                    callback_compat: None,
                    transfer_mode: None,
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                },
                ArcanaCabiApiFieldContract {
                    name: "greeting".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                    binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Return),
                    input_type: None,
                    output_type: Some("Str".to_string()),
                    callback_compat: None,
                    transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                    owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::String),
                    release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::Custom),
                    release_target: Some("__binding.owned_str_free".to_string()),
                    companion_fields: vec!["status".to_string()],
                    partial_failure_cleanup: false,
                },
            ],
            surface_text: "export api Greeting".to_string(),
        };
        let resolution = arcana_cabi::resolve_api_binding_resolution(&api.name, &api.fields)
            .expect("resolution");
        let outcome = crate::native_product_loader::RuntimeBindingImportOutcome {
            result: binding_owned_str("hello"),
            write_backs: vec![ArcanaCabiBindingValueV1::default()],
            owned_bytes_free: test_owned_bytes_free,
            owned_str_free: test_owned_str_free,
        };
        let err = materialize_runtime_binding_api_response_fields(
            &empty_plan(),
            &api,
            &resolution,
            &mut BTreeMap::new(),
            &outcome,
            &mut RuntimeExecutionState::default(),
        )
        .expect_err("missing companion fields should fail response materialization");

        assert!(
            err.contains("missing companion response fields status"),
            "{err}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_partial_failure_cleanup_releases_owned_return_slot() {
        static RELEASES: AtomicUsize = AtomicUsize::new(0);

        unsafe extern "system" fn counted_owned_bytes_free(ptr: *mut u8, len: usize) {
            unsafe {
                arcana_cabi::free_owned_bytes(ptr, len);
            }
        }

        unsafe extern "system" fn counted_owned_str_free(ptr: *mut u8, len: usize) {
            RELEASES.fetch_add(1, Ordering::SeqCst);
            unsafe {
                arcana_cabi::free_owned_str(ptr, len);
            }
        }

        RELEASES.store(0, Ordering::SeqCst);
        let api = RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "GreetingWithStatus".to_string(),
            request_type: parse_routine_type_text("tool.api.GreetingRequest")
                .expect("request type"),
            response_type: parse_routine_type_text("tool.api.GreetingResponse")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
            backend_target: "api.greeting_with_status".to_string(),
            fields: vec![
                ArcanaCabiApiFieldContract {
                    name: "status".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                    binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Param),
                    input_type: None,
                    output_type: Some("Int".to_string()),
                    callback_compat: None,
                    transfer_mode: None,
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                },
                ArcanaCabiApiFieldContract {
                    name: "greeting".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                    binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Return),
                    input_type: None,
                    output_type: Some("Str".to_string()),
                    callback_compat: None,
                    transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                    owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::String),
                    release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::Custom),
                    release_target: Some("__binding.owned_str_free".to_string()),
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: true,
                },
            ],
            surface_text: "export api GreetingWithStatus".to_string(),
        };
        let resolution = arcana_cabi::resolve_api_binding_resolution(&api.name, &api.fields)
            .expect("resolution");
        let outcome = crate::native_product_loader::RuntimeBindingImportOutcome {
            result: binding_owned_str("hello"),
            write_backs: vec![ArcanaCabiBindingValueV1::default()],
            owned_bytes_free: counted_owned_bytes_free,
            owned_str_free: counted_owned_str_free,
        };
        let err = materialize_runtime_binding_api_response_fields(
            &empty_plan(),
            &api,
            &resolution,
            &mut BTreeMap::new(),
            &outcome,
            &mut RuntimeExecutionState::default(),
        )
        .expect_err("partial failure should surface the missing write-back error");

        assert!(
            err.contains("expected a synthetic out-param write-back"),
            "{err}"
        );
        assert_eq!(RELEASES.load(Ordering::SeqCst), 1);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_materializes_co_task_mem_owned_string_response() {
        let api = RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "Greeting".to_string(),
            request_type: parse_routine_type_text("tool.api.GreetingRequest")
                .expect("request type"),
            response_type: parse_routine_type_text("tool.api.GreetingResponse")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
            backend_target: "api.greeting".to_string(),
            fields: vec![ArcanaCabiApiFieldContract {
                name: "greeting".to_string(),
                mode: ArcanaCabiApiFieldMode::Out,
                lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Return),
                input_type: None,
                output_type: Some("Str".to_string()),
                callback_compat: None,
                transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::String),
                release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::CoTaskMemFree),
                release_target: None,
                companion_fields: Vec::new(),
                partial_failure_cleanup: false,
            }],
            surface_text: "export api Greeting".to_string(),
        };
        let resolution = arcana_cabi::resolve_api_binding_resolution(&api.name, &api.fields)
            .expect("resolution");
        let text = "hello from com";
        let ptr = unsafe { CoTaskMemAlloc(text.len()) as *mut u8 };
        assert!(!ptr.is_null(), "CoTaskMemAlloc should allocate");
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len());
        }
        let outcome = crate::native_product_loader::RuntimeBindingImportOutcome {
            result: ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Str as u32,
                reserved0: 0,
                reserved1: 0,
                payload: arcana_cabi::ArcanaCabiBindingPayloadV1 {
                    owned_str_value: arcana_cabi::ArcanaOwnedStr {
                        ptr,
                        len: text.len(),
                    },
                },
            },
            write_backs: Vec::new(),
            owned_bytes_free: runtime_api_owned_bytes_co_task_mem_free,
            owned_str_free: runtime_api_owned_str_co_task_mem_free,
        };
        let response = materialize_runtime_binding_api_response_fields(
            &empty_plan(),
            &api,
            &resolution,
            &mut BTreeMap::new(),
            &outcome,
            &mut RuntimeExecutionState::default(),
        )
        .expect("co-task-mem string should materialize");

        assert_eq!(
            response.get("greeting"),
            Some(&RuntimeValue::Str(text.to_string()))
        );
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_materializes_local_free_owned_string_response() {
        let api = RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "Greeting".to_string(),
            request_type: parse_routine_type_text("tool.api.GreetingRequest")
                .expect("request type"),
            response_type: parse_routine_type_text("tool.api.GreetingResponse")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
            backend_target: "api.greeting".to_string(),
            fields: vec![ArcanaCabiApiFieldContract {
                name: "greeting".to_string(),
                mode: ArcanaCabiApiFieldMode::Out,
                lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Return),
                input_type: None,
                output_type: Some("Str".to_string()),
                callback_compat: None,
                transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::String),
                release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::LocalFree),
                release_target: None,
                companion_fields: Vec::new(),
                partial_failure_cleanup: false,
            }],
            surface_text: "export api Greeting".to_string(),
        };
        let resolution = arcana_cabi::resolve_api_binding_resolution(&api.name, &api.fields)
            .expect("resolution");
        let text = "hello from local";
        let ptr = unsafe { LocalAlloc(0, text.len()) as *mut u8 };
        assert!(!ptr.is_null(), "LocalAlloc should allocate");
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len());
        }
        let outcome = crate::native_product_loader::RuntimeBindingImportOutcome {
            result: ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Str as u32,
                reserved0: 0,
                reserved1: 0,
                payload: arcana_cabi::ArcanaCabiBindingPayloadV1 {
                    owned_str_value: arcana_cabi::ArcanaOwnedStr {
                        ptr,
                        len: text.len(),
                    },
                },
            },
            write_backs: Vec::new(),
            owned_bytes_free: runtime_api_owned_bytes_local_free,
            owned_str_free: runtime_api_owned_str_local_free,
        };
        let response = materialize_runtime_binding_api_response_fields(
            &empty_plan(),
            &api,
            &resolution,
            &mut BTreeMap::new(),
            &outcome,
            &mut RuntimeExecutionState::default(),
        )
        .expect("local-free string should materialize");

        assert_eq!(
            response.get("greeting"),
            Some(&RuntimeValue::Str(text.to_string()))
        );
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_partial_failure_cleanup_releases_com_interface_return_slot() {
        static RELEASES: AtomicUsize = AtomicUsize::new(0);

        unsafe extern "system" fn fake_com_release(ptr: *mut std::ffi::c_void) -> u32 {
            RELEASES.fetch_add(1, Ordering::SeqCst);
            unsafe {
                drop(Box::from_raw(ptr as *mut FakeComObject));
            }
            0
        }

        #[repr(C)]
        struct FakeComObject {
            vtable: *const RuntimeApiIUnknownVtable,
        }

        let fake_com_vtable = RuntimeApiIUnknownVtable {
            query_interface: std::ptr::null(),
            add_ref: std::ptr::null(),
            release: fake_com_release,
        };

        RELEASES.store(0, Ordering::SeqCst);
        let api = RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "CreateInterface".to_string(),
            request_type: parse_routine_type_text("tool.api.InterfaceRequest")
                .expect("request type"),
            response_type: parse_routine_type_text("tool.api.InterfaceResponse")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
            backend_target: "api.create_interface".to_string(),
            fields: vec![
                ArcanaCabiApiFieldContract {
                    name: "status".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                    binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Param),
                    input_type: None,
                    output_type: Some("Int".to_string()),
                    callback_compat: None,
                    transfer_mode: None,
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                },
                ArcanaCabiApiFieldContract {
                    name: "iface".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                    binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Return),
                    input_type: None,
                    output_type: Some("tool.raw.IFace".to_string()),
                    callback_compat: None,
                    transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                    owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::Interface),
                    release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::Release),
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: true,
                },
            ],
            surface_text: "export api CreateInterface".to_string(),
        };
        let resolution = arcana_cabi::resolve_api_binding_resolution(&api.name, &api.fields)
            .expect("resolution");
        let object = Box::into_raw(Box::new(FakeComObject {
            vtable: &fake_com_vtable,
        }));
        let outcome = crate::native_product_loader::RuntimeBindingImportOutcome {
            result: ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Opaque as u32,
                reserved0: 0,
                reserved1: 0,
                payload: arcana_cabi::ArcanaCabiBindingPayloadV1 {
                    opaque_value: object as usize as u64,
                },
            },
            write_backs: vec![ArcanaCabiBindingValueV1::default()],
            owned_bytes_free: runtime_api_owned_bytes_local_free,
            owned_str_free: runtime_api_owned_str_local_free,
        };
        let err = materialize_runtime_binding_api_response_fields(
            &empty_plan(),
            &api,
            &resolution,
            &mut BTreeMap::new(),
            &outcome,
            &mut RuntimeExecutionState::default(),
        )
        .expect_err("missing status write-back should trigger interface cleanup");

        assert!(
            err.contains("expected a synthetic out-param write-back"),
            "{err}"
        );
        assert_eq!(RELEASES.load(Ordering::SeqCst), 1);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_materializes_owned_interface_with_release_metadata() {
        unsafe extern "system" fn fake_com_release(ptr: *mut std::ffi::c_void) -> u32 {
            unsafe {
                drop(Box::from_raw(ptr as *mut FakeComObject));
            }
            0
        }

        #[repr(C)]
        struct FakeComObject {
            vtable: *const RuntimeApiIUnknownVtable,
        }

        let fake_com_vtable = RuntimeApiIUnknownVtable {
            query_interface: std::ptr::null(),
            add_ref: std::ptr::null(),
            release: fake_com_release,
        };

        let api = RuntimeApiDeclPlan {
            package_id: "tool".to_string(),
            module_id: "tool.api".to_string(),
            exported: true,
            name: "CreateInterface".to_string(),
            request_type: parse_routine_type_text("tool.api.InterfaceRequest")
                .expect("request type"),
            response_type: parse_routine_type_text("tool.api.InterfaceResponse")
                .expect("response type"),
            backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
            backend_target: "api.create_interface".to_string(),
            fields: vec![ArcanaCabiApiFieldContract {
                name: "iface".to_string(),
                mode: ArcanaCabiApiFieldMode::Out,
                lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                binding_slot: Some(arcana_cabi::ArcanaCabiApiBindingSlot::Return),
                input_type: None,
                output_type: Some("tool.raw.IFace".to_string()),
                callback_compat: None,
                transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::Interface),
                release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::Release),
                release_target: None,
                companion_fields: Vec::new(),
                partial_failure_cleanup: false,
            }],
            surface_text: "export api CreateInterface".to_string(),
        };
        let resolution = arcana_cabi::resolve_api_binding_resolution(&api.name, &api.fields)
            .expect("resolution");
        let object = Box::into_raw(Box::new(FakeComObject {
            vtable: &fake_com_vtable,
        }));
        let outcome = crate::native_product_loader::RuntimeBindingImportOutcome {
            result: ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Opaque as u32,
                reserved0: 0,
                reserved1: 0,
                payload: arcana_cabi::ArcanaCabiBindingPayloadV1 {
                    opaque_value: object as usize as u64,
                },
            },
            write_backs: Vec::new(),
            owned_bytes_free: runtime_api_owned_bytes_local_free,
            owned_str_free: runtime_api_owned_str_local_free,
        };
        let response = materialize_runtime_binding_api_response_fields(
            &empty_plan(),
            &api,
            &resolution,
            &mut BTreeMap::new(),
            &outcome,
            &mut RuntimeExecutionState::default(),
        )
        .expect("owned interface should materialize");
        let binding = match response.get("iface") {
            Some(RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding))) => binding,
            other => panic!("expected binding opaque response, got {other:?}"),
        };
        assert_eq!(
            binding.owned_release,
            Some(RuntimeBindingOwnedRelease::Release)
        );

        let cleanup = ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Opaque as u32,
            reserved0: 0,
            reserved1: 0,
            payload: arcana_cabi::ArcanaCabiBindingPayloadV1 {
                opaque_value: binding.handle,
            },
        };
        release_runtime_api_binding_output_slot(
            &empty_plan(),
            &api,
            &api.fields[0],
            &cleanup,
            &outcome,
        )
        .expect("release metadata cleanup should succeed");
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_executes_binding_backend() {
        use arcana_aot::{
            AotInstanceProductSpec, AotShackleDeclArtifact, AotShackleImportTargetArtifact,
            NativeBindingImport, compile_instance_product,
            default_instance_product_cargo_target_dir,
        };
        use arcana_cabi::{
            ARCANA_CABI_BINDING_V2_CONTRACT_ID, ArcanaCabiBindingType, ArcanaCabiProductRole,
        };

        let _lock = runtime_api_abi_env_lock()
            .lock()
            .expect("runtime api abi env lock should acquire");
        let dir = temp_dir("binding_backend");
        let project_dir = dir.join("project");
        let artifact_dir = dir.join("artifact");
        let cargo_target_dir =
            default_instance_product_cargo_target_dir(ArcanaCabiProductRole::Binding);
        let compiled = compile_instance_product(
            &AotInstanceProductSpec {
                package_id: "hostapi".to_string(),
                package_name: "hostapi".to_string(),
                product_name: "shim".to_string(),
                role: ArcanaCabiProductRole::Binding,
                contract_id: ARCANA_CABI_BINDING_V2_CONTRACT_ID.to_string(),
                output_file_name: "hostapi_shim.dll".to_string(),
                package_image_text: None,
                binding_imports: vec![NativeBindingImport {
                    name: "raw.kernel32.GetCurrentProcessId".to_string(),
                    symbol_name: "arcana_binding_import_hostapi_raw_kernel32_getcurrentprocessid"
                        .to_string(),
                    return_type: ArcanaCabiBindingType::Int,
                    params: Vec::new(),
                }],
                binding_callbacks: Vec::new(),
                binding_layouts: Vec::new(),
                binding_shackle_decls: vec![AotShackleDeclArtifact {
                    package_id: "hostapi".to_string(),
                    module_id: "hostapi.raw.kernel32".to_string(),
                    exported: true,
                    kind: "import_fn".to_string(),
                    name: "GetCurrentProcessId".to_string(),
                    params: Vec::new(),
                    return_type: Some(
                        parse_routine_type_text("Int").expect("return type should parse"),
                    ),
                    callback_type: None,
                    binding: Some("raw.kernel32.GetCurrentProcessId".to_string()),
                    body_entries: Vec::new(),
                    raw_layout: None,
                    import_target: Some(AotShackleImportTargetArtifact {
                        library: "kernel32".to_string(),
                        symbol: "GetCurrentProcessId".to_string(),
                        abi: "system".to_string(),
                    }),
                    thunk_target: None,
                    surface_text: String::new(),
                }],
            },
            &project_dir,
            &artifact_dir,
            &cargo_target_dir,
        )
        .expect("binding instance product should compile");

        let bundle_dll = dir.join("hostapi_shim.dll");
        fs::copy(&compiled.output_path, &bundle_dll).expect("binding output should copy");
        let manifest_path = dir.join("arcana.bundle.toml");
        fs::write(
            &manifest_path,
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"hostapi_shim.dll\"]\n",
                "\n[[native_products]]\n",
                "package_id = \"hostapi\"\n",
                "package_name = \"hostapi\"\n",
                "product_name = \"shim\"\n",
                "role = \"binding\"\n",
                "contract_id = \"arcana.cabi.binding.v2\"\n",
                "file = \"hostapi_shim.dll\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let previous_manifest = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV);
        let previous_bundle_dir = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        unsafe {
            std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, &manifest_path);
            std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        }
        super::super::reset_runtime_native_products_cache();

        let invoke_result = {
            let mut plan = empty_plan();
            plan.package_id = "hostapi".to_string();
            plan.package_name = "hostapi".to_string();
            plan.root_module_id = "hostapi.api".to_string();
            plan.api_decls.push(RuntimeApiDeclPlan {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.api".to_string(),
                exported: true,
                name: "GetCurrentProcessId".to_string(),
                request_type: parse_routine_type_text("hostapi.api.EmptyRequest")
                    .expect("request type"),
                response_type: parse_routine_type_text("hostapi.api.PidResponse")
                    .expect("response type"),
                backend_target_kind: ArcanaCabiApiBackendTargetKind::ForeignSymbol,
                backend_target: "raw.kernel32.GetCurrentProcessId".to_string(),
                fields: vec![ArcanaCabiApiFieldContract {
                    name: "pid".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                    binding_slot: None,
                    input_type: None,
                    output_type: Some("Int".to_string()),
                    callback_compat: None,
                    transfer_mode: None,
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                }],
                surface_text: "export api GetCurrentProcessId".to_string(),
            });

            let mut host = BufferedHost::default();
            execute_exported_api_abi(&plan, "hostapi.api.GetCurrentProcessId", "{}", &mut host)
        };

        super::super::reset_runtime_native_products_cache();
        unsafe {
            match previous_manifest {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV),
            }
            match previous_bundle_dir {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV),
            }
        }

        let response = invoke_result.expect("binding api invoke should succeed");
        let response =
            serde_json::from_str::<serde_json::Value>(&response).expect("response should parse");
        assert!(
            response["pid"].as_i64().is_some_and(|pid| pid > 0),
            "expected positive pid response, got {response}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_executes_embedded_c_shim_write_back_backend() {
        use arcana_aot::{
            AotInstanceProductSpec, AotShackleDeclArtifact, NativeBindingImport,
            compile_instance_product, default_instance_product_cargo_target_dir,
        };
        use arcana_cabi::{
            ARCANA_CABI_BINDING_V2_CONTRACT_ID, ArcanaCabiBindingParam, ArcanaCabiBindingType,
            ArcanaCabiParamSourceMode, ArcanaCabiProductRole,
        };
        use arcana_ir::IrRoutineParam;

        let _lock = runtime_api_abi_env_lock()
            .lock()
            .expect("runtime api abi env lock should acquire");
        let dir = temp_dir("embedded_c_shim_write_back");
        let project_dir = dir.join("project");
        let artifact_dir = dir.join("artifact");
        let cargo_target_dir =
            default_instance_product_cargo_target_dir(ArcanaCabiProductRole::Binding);
        let compiled = compile_instance_product(
            &AotInstanceProductSpec {
                package_id: "hostapi".to_string(),
                package_name: "hostapi".to_string(),
                product_name: "api".to_string(),
                role: ArcanaCabiProductRole::Binding,
                contract_id: ARCANA_CABI_BINDING_V2_CONTRACT_ID.to_string(),
                output_file_name: "hostapi_api.dll".to_string(),
                package_image_text: None,
                binding_imports: vec![NativeBindingImport {
                    name: "api.touch".to_string(),
                    symbol_name: "arcana_binding_import_hostapi_api_touch".to_string(),
                    return_type: ArcanaCabiBindingType::Unit,
                    params: vec![ArcanaCabiBindingParam::binding(
                        "value",
                        ArcanaCabiParamSourceMode::Edit,
                        ArcanaCabiBindingType::Int,
                    )],
                }],
                binding_callbacks: Vec::new(),
                binding_layouts: Vec::new(),
                binding_shackle_decls: vec![AotShackleDeclArtifact {
                    package_id: "hostapi".to_string(),
                    module_id: "hostapi.api".to_string(),
                    exported: false,
                    kind: "fn".to_string(),
                    name: "touch_impl".to_string(),
                    params: vec![IrRoutineParam {
                        binding_id: 0,
                        mode: Some("edit".to_string()),
                        name: "value".to_string(),
                        ty: parse_routine_type_text("Int").expect("param type should parse"),
                    }],
                    return_type: Some(
                        parse_routine_type_text("Unit").expect("return type should parse"),
                    ),
                    callback_type: None,
                    binding: Some("api.touch".to_string()),
                    body_entries: vec!["Ok(binding_unit())".to_string()],
                    raw_layout: None,
                    import_target: None,
                    thunk_target: None,
                    surface_text: String::new(),
                }],
            },
            &project_dir,
            &artifact_dir,
            &cargo_target_dir,
        )
        .expect("binding instance product should compile");

        let bundle_dll = dir.join("hostapi_api.dll");
        fs::copy(&compiled.output_path, &bundle_dll).expect("binding output should copy");
        let manifest_path = dir.join("arcana.bundle.toml");
        fs::write(
            &manifest_path,
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"hostapi_api.dll\"]\n",
                "\n[[native_products]]\n",
                "package_id = \"hostapi\"\n",
                "package_name = \"hostapi\"\n",
                "product_name = \"api\"\n",
                "role = \"binding\"\n",
                "contract_id = \"arcana.cabi.binding.v2\"\n",
                "file = \"hostapi_api.dll\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let previous_manifest = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV);
        let previous_bundle_dir = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        unsafe {
            std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, &manifest_path);
            std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        }
        super::super::reset_runtime_native_products_cache();

        let invoke_result = {
            let mut plan = empty_plan();
            plan.package_id = "hostapi".to_string();
            plan.package_name = "hostapi".to_string();
            plan.root_module_id = "hostapi.api".to_string();
            plan.api_decls.push(RuntimeApiDeclPlan {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.api".to_string(),
                exported: true,
                name: "Touch".to_string(),
                request_type: parse_routine_type_text("hostapi.api.TouchRequest")
                    .expect("request type"),
                response_type: parse_routine_type_text("hostapi.api.TouchResponse")
                    .expect("response type"),
                backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
                backend_target: "api.touch".to_string(),
                fields: vec![ArcanaCabiApiFieldContract {
                    name: "value".to_string(),
                    mode: ArcanaCabiApiFieldMode::InWithWriteBack,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                    binding_slot: None,
                    input_type: Some("Int".to_string()),
                    output_type: Some("Int".to_string()),
                    callback_compat: None,
                    transfer_mode: None,
                    owned_result_kind: None,
                    release_family: None,
                    release_target: None,
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                }],
                surface_text: "export api Touch".to_string(),
            });

            let mut host = BufferedHost::default();
            execute_exported_api_abi(&plan, "hostapi.api.Touch", r#"{ "value": 7 }"#, &mut host)
        };

        super::super::reset_runtime_native_products_cache();
        unsafe {
            match previous_manifest {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV),
            }
            match previous_bundle_dir {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV),
            }
        }

        let response = invoke_result.expect("binding api invoke should succeed");
        let response =
            serde_json::from_str::<serde_json::Value>(&response).expect("response should parse");
        assert_eq!(response, serde_json::json!({ "value": 7 }));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_materializes_owned_string_response() {
        use arcana_aot::{
            AotInstanceProductSpec, AotShackleDeclArtifact, NativeBindingImport,
            compile_instance_product, default_instance_product_cargo_target_dir,
        };
        use arcana_cabi::{
            ARCANA_CABI_BINDING_V2_CONTRACT_ID, ArcanaCabiBindingType, ArcanaCabiProductRole,
        };

        let _lock = runtime_api_abi_env_lock()
            .lock()
            .expect("runtime api abi env lock should acquire");
        let dir = temp_dir("owned_string_response");
        let project_dir = dir.join("project");
        let artifact_dir = dir.join("artifact");
        let cargo_target_dir =
            default_instance_product_cargo_target_dir(ArcanaCabiProductRole::Binding);
        let compiled = compile_instance_product(
            &AotInstanceProductSpec {
                package_id: "hostapi".to_string(),
                package_name: "hostapi".to_string(),
                product_name: "greet".to_string(),
                role: ArcanaCabiProductRole::Binding,
                contract_id: ARCANA_CABI_BINDING_V2_CONTRACT_ID.to_string(),
                output_file_name: "hostapi_greet.dll".to_string(),
                package_image_text: None,
                binding_imports: vec![NativeBindingImport {
                    name: "api.greeting".to_string(),
                    symbol_name: "arcana_binding_import_hostapi_api_greeting".to_string(),
                    return_type: ArcanaCabiBindingType::Str,
                    params: Vec::new(),
                }],
                binding_callbacks: Vec::new(),
                binding_layouts: Vec::new(),
                binding_shackle_decls: vec![AotShackleDeclArtifact {
                    package_id: "hostapi".to_string(),
                    module_id: "hostapi.api".to_string(),
                    exported: false,
                    kind: "fn".to_string(),
                    name: "greeting_impl".to_string(),
                    params: Vec::new(),
                    return_type: Some(
                        parse_routine_type_text("Str").expect("return type should parse"),
                    ),
                    callback_type: None,
                    binding: Some("api.greeting".to_string()),
                    body_entries: vec!["Ok(binding_owned_str(\"hello\".to_string()))".to_string()],
                    raw_layout: None,
                    import_target: None,
                    thunk_target: None,
                    surface_text: String::new(),
                }],
            },
            &project_dir,
            &artifact_dir,
            &cargo_target_dir,
        )
        .expect("binding instance product should compile");

        let bundle_dll = dir.join("hostapi_greet.dll");
        fs::copy(&compiled.output_path, &bundle_dll).expect("binding output should copy");
        let manifest_path = dir.join("arcana.bundle.toml");
        fs::write(
            &manifest_path,
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"hostapi_greet.dll\"]\n",
                "\n[[native_products]]\n",
                "package_id = \"hostapi\"\n",
                "package_name = \"hostapi\"\n",
                "product_name = \"greet\"\n",
                "role = \"binding\"\n",
                "contract_id = \"arcana.cabi.binding.v2\"\n",
                "file = \"hostapi_greet.dll\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let previous_manifest = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV);
        let previous_bundle_dir = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        unsafe {
            std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, &manifest_path);
            std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        }
        super::super::reset_runtime_native_products_cache();

        let invoke_result = {
            let mut plan = empty_plan();
            plan.package_id = "hostapi".to_string();
            plan.package_name = "hostapi".to_string();
            plan.root_module_id = "hostapi.api".to_string();
            plan.api_decls.push(RuntimeApiDeclPlan {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.api".to_string(),
                exported: true,
                name: "Greeting".to_string(),
                request_type: parse_routine_type_text("hostapi.api.EmptyRequest")
                    .expect("request type"),
                response_type: parse_routine_type_text("hostapi.api.GreetingResponse")
                    .expect("response type"),
                backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
                backend_target: "api.greeting".to_string(),
                fields: vec![ArcanaCabiApiFieldContract {
                    name: "greeting".to_string(),
                    mode: ArcanaCabiApiFieldMode::Out,
                    lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::OwnedTransfer,
                    binding_slot: None,
                    input_type: None,
                    output_type: Some("Str".to_string()),
                    callback_compat: None,
                    transfer_mode: Some(arcana_cabi::ArcanaCabiApiTransferMode::CalleeOwned),
                    owned_result_kind: Some(arcana_cabi::ArcanaCabiApiOwnedResultKind::String),
                    release_family: Some(arcana_cabi::ArcanaCabiApiReleaseFamily::Custom),
                    release_target: Some("__binding.owned_str_free".to_string()),
                    companion_fields: Vec::new(),
                    partial_failure_cleanup: false,
                }],
                surface_text: "export api Greeting".to_string(),
            });

            let mut host = BufferedHost::default();
            execute_exported_api_abi(&plan, "hostapi.api.Greeting", "{}", &mut host)
        };

        super::super::reset_runtime_native_products_cache();
        unsafe {
            match previous_manifest {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV),
            }
            match previous_bundle_dir {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV),
            }
        }

        let response = invoke_result.expect("binding api invoke should succeed");
        let response =
            serde_json::from_str::<serde_json::Value>(&response).expect("response should parse");
        assert_eq!(response, serde_json::json!({ "greeting": "hello" }));

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_api_abi_collects_multi_out_binding_response() {
        use arcana_aot::{
            AotInstanceProductSpec, AotShackleDeclArtifact, NativeBindingImport,
            compile_instance_product, default_instance_product_cargo_target_dir,
        };
        use arcana_cabi::{
            ARCANA_CABI_BINDING_V2_CONTRACT_ID, ArcanaCabiApiBindingSlot, ArcanaCabiBindingParam,
            ArcanaCabiBindingType, ArcanaCabiParamSourceMode, ArcanaCabiProductRole,
        };
        use arcana_ir::IrRoutineParam;

        let _lock = runtime_api_abi_env_lock()
            .lock()
            .expect("runtime api abi env lock should acquire");
        let dir = temp_dir("multi_out_response");
        let project_dir = dir.join("project");
        let artifact_dir = dir.join("artifact");
        let cargo_target_dir =
            default_instance_product_cargo_target_dir(ArcanaCabiProductRole::Binding);
        let compiled = compile_instance_product(
            &AotInstanceProductSpec {
                package_id: "hostapi".to_string(),
                package_name: "hostapi".to_string(),
                product_name: "status".to_string(),
                role: ArcanaCabiProductRole::Binding,
                contract_id: ARCANA_CABI_BINDING_V2_CONTRACT_ID.to_string(),
                output_file_name: "hostapi_status.dll".to_string(),
                package_image_text: None,
                binding_imports: vec![NativeBindingImport {
                    name: "api.query".to_string(),
                    symbol_name: "arcana_binding_import_hostapi_api_query".to_string(),
                    return_type: ArcanaCabiBindingType::Int,
                    params: vec![ArcanaCabiBindingParam::binding(
                        "pid",
                        ArcanaCabiParamSourceMode::Edit,
                        ArcanaCabiBindingType::Int,
                    )],
                }],
                binding_callbacks: Vec::new(),
                binding_layouts: Vec::new(),
                binding_shackle_decls: vec![AotShackleDeclArtifact {
                    package_id: "hostapi".to_string(),
                    module_id: "hostapi.api".to_string(),
                    exported: false,
                    kind: "fn".to_string(),
                    name: "query_impl".to_string(),
                    params: vec![IrRoutineParam {
                        binding_id: 0,
                        mode: Some("edit".to_string()),
                        name: "pid".to_string(),
                        ty: parse_routine_type_text("Int").expect("param type should parse"),
                    }],
                    return_type: Some(
                        parse_routine_type_text("Int").expect("return type should parse"),
                    ),
                    callback_type: None,
                    binding: Some("api.query".to_string()),
                    body_entries: vec![
                        "*pid_write_back = binding_int(42);".to_string(),
                        "Ok(binding_int(7))".to_string(),
                    ],
                    raw_layout: None,
                    import_target: None,
                    thunk_target: None,
                    surface_text: String::new(),
                }],
            },
            &project_dir,
            &artifact_dir,
            &cargo_target_dir,
        )
        .expect("binding instance product should compile");

        let bundle_dll = dir.join("hostapi_status.dll");
        fs::copy(&compiled.output_path, &bundle_dll).expect("binding output should copy");
        let manifest_path = dir.join("arcana.bundle.toml");
        fs::write(
            &manifest_path,
            concat!(
                "format = \"arcana-distribution-bundle-v2\"\n",
                "member = \"app\"\n",
                "target = \"windows-exe\"\n",
                "target_format = \"arcana-native-exe-v1\"\n",
                "root_artifact = \"app.exe\"\n",
                "artifact_hash = \"sha256:test\"\n",
                "toolchain = \"toolchain\"\n",
                "support_files = [\"hostapi_status.dll\"]\n",
                "\n[[native_products]]\n",
                "package_id = \"hostapi\"\n",
                "package_name = \"hostapi\"\n",
                "product_name = \"status\"\n",
                "role = \"binding\"\n",
                "contract_id = \"arcana.cabi.binding.v2\"\n",
                "file = \"hostapi_status.dll\"\n",
            ),
        )
        .expect("bundle manifest should write");

        let previous_manifest = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV);
        let previous_bundle_dir = std::env::var_os(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        unsafe {
            std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, &manifest_path);
            std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV);
        }
        super::super::reset_runtime_native_products_cache();

        let invoke_result = {
            let mut plan = empty_plan();
            plan.package_id = "hostapi".to_string();
            plan.package_name = "hostapi".to_string();
            plan.root_module_id = "hostapi.api".to_string();
            plan.api_decls.push(RuntimeApiDeclPlan {
                package_id: "hostapi".to_string(),
                module_id: "hostapi.api".to_string(),
                exported: true,
                name: "Query".to_string(),
                request_type: parse_routine_type_text("hostapi.api.EmptyRequest")
                    .expect("request type"),
                response_type: parse_routine_type_text("hostapi.api.QueryResponse")
                    .expect("response type"),
                backend_target_kind: ArcanaCabiApiBackendTargetKind::EmbeddedCShim,
                backend_target: "api.query".to_string(),
                fields: vec![
                    ArcanaCabiApiFieldContract {
                        name: "status".to_string(),
                        mode: ArcanaCabiApiFieldMode::Out,
                        lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                        binding_slot: Some(ArcanaCabiApiBindingSlot::Return),
                        input_type: None,
                        output_type: Some("Int".to_string()),
                        callback_compat: None,
                        transfer_mode: None,
                        owned_result_kind: None,
                        release_family: None,
                        release_target: None,
                        companion_fields: Vec::new(),
                        partial_failure_cleanup: false,
                    },
                    ArcanaCabiApiFieldContract {
                        name: "pid".to_string(),
                        mode: ArcanaCabiApiFieldMode::Out,
                        lane_kind: arcana_cabi::ArcanaCabiApiLaneKind::Value,
                        binding_slot: Some(ArcanaCabiApiBindingSlot::Param),
                        input_type: None,
                        output_type: Some("Int".to_string()),
                        callback_compat: None,
                        transfer_mode: None,
                        owned_result_kind: None,
                        release_family: None,
                        release_target: None,
                        companion_fields: Vec::new(),
                        partial_failure_cleanup: false,
                    },
                ],
                surface_text: "export api Query".to_string(),
            });

            let mut host = BufferedHost::default();
            execute_exported_api_abi(&plan, "hostapi.api.Query", "{}", &mut host)
        };

        super::super::reset_runtime_native_products_cache();
        unsafe {
            match previous_manifest {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_MANIFEST_ENV),
            }
            match previous_bundle_dir {
                Some(value) => std::env::set_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV, value),
                None => std::env::remove_var(crate::ARCANA_NATIVE_BUNDLE_DIR_ENV),
            }
        }

        let response = invoke_result.expect("binding api invoke should succeed");
        let response =
            serde_json::from_str::<serde_json::Value>(&response).expect("response should parse");
        assert_eq!(response, serde_json::json!({ "pid": 42, "status": 7 }));

        let _ = fs::remove_dir_all(&dir);
    }
}
