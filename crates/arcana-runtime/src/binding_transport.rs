use super::*;

pub(super) fn with_runtime_binding_callback_context<R>(
    plan: &RuntimePackagePlan,
    host_data: *mut (),
    host_vtable: *mut (),
    action: impl FnOnce() -> Result<R, String>,
) -> Result<R, String> {
    ACTIVE_RUNTIME_BINDING_CALLBACK_CONTEXT.with(|slot| {
        let previous = slot.replace(Some(RuntimeBindingCallbackContext {
            plan: plan as *const RuntimePackagePlan,
            host_data,
            host_vtable,
        }));
        let result = action();
        let _ = slot.replace(previous);
        result
    })
}

pub(super) fn leak_runtime_binding_text(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

pub(super) unsafe extern "system" fn runtime_binding_owned_bytes_free(ptr: *mut u8, len: usize) {
    unsafe {
        arcana_cabi::free_owned_bytes(ptr, len);
    }
}

pub(super) unsafe extern "system" fn runtime_binding_owned_str_free(ptr: *mut u8, len: usize) {
    unsafe {
        arcana_cabi::free_owned_str(ptr, len);
    }
}

unsafe fn free_runtime_binding_callback_user_data(user_data: *mut c_void) {
    if !user_data.is_null() {
        unsafe {
            drop(Box::from_raw(
                user_data as *mut RuntimeBindingCallbackThunkData,
            ));
        }
    }
}

pub(super) fn runtime_binding_callback_specs_for_package(
    plan: &RuntimePackagePlan,
    package_id: &str,
) -> Vec<RuntimeBindingCallbackRegistrationSpec> {
    plan.native_callbacks
        .iter()
        .filter(|callback| callback.package_id == package_id)
        .map(|callback| RuntimeBindingCallbackRegistrationSpec {
            name: leak_runtime_binding_text(&callback.name),
            callback: runtime_binding_callback_trampoline,
            owned_bytes_free: runtime_binding_owned_bytes_free,
            owned_str_free: runtime_binding_owned_str_free,
            user_data: Box::into_raw(Box::new(RuntimeBindingCallbackThunkData {
                callback: callback.clone(),
            })) as *mut c_void,
            cleanup_user_data: free_runtime_binding_callback_user_data,
        })
        .collect()
}

pub(super) fn runtime_binding_param_metadata(
    param: &RuntimeParamPlan,
) -> Result<ArcanaCabiBindingParam, String> {
    let source_mode = ArcanaCabiParamSourceMode::from_param_mode_text(param.mode.as_deref())?;
    let input_type = ArcanaCabiBindingType::parse(&param.ty.render())?;
    validate_binding_transport_type(&input_type)?;
    Ok(ArcanaCabiBindingParam::binding(
        param.name.clone(),
        source_mode,
        input_type,
    ))
}

pub(super) fn runtime_binding_params_metadata(
    params: &[RuntimeParamPlan],
) -> Result<Vec<ArcanaCabiBindingParam>, String> {
    params
        .iter()
        .map(runtime_binding_param_metadata)
        .collect::<Result<Vec<_>, _>>()
}

pub(super) fn runtime_binding_return_type(
    return_type: Option<&IrRoutineType>,
) -> Result<ArcanaCabiBindingType, String> {
    let ty = ArcanaCabiBindingType::parse(
        &return_type
            .map(IrRoutineType::render)
            .unwrap_or_else(|| "Unit".to_string()),
    )?;
    validate_binding_transport_type(&ty)?;
    Ok(ty)
}

pub(super) fn runtime_binding_import_signatures_for_package(
    plan: &RuntimePackagePlan,
    package_id: &str,
) -> Result<Vec<ArcanaCabiBindingSignature>, String> {
    plan.routines
        .iter()
        .filter(|routine| routine.package_id == package_id)
        .filter_map(|routine| routine.native_impl.as_ref().map(|name| (routine, name)))
        .map(|(routine, name)| {
            Ok(ArcanaCabiBindingSignature {
                name: name.clone(),
                return_type: runtime_binding_return_type(routine.return_type.as_ref())?,
                params: runtime_binding_params_metadata(&routine.params)?,
            })
        })
        .collect()
}

pub(super) fn runtime_binding_callback_signatures_for_package(
    plan: &RuntimePackagePlan,
    package_id: &str,
) -> Result<Vec<ArcanaCabiBindingSignature>, String> {
    plan.native_callbacks
        .iter()
        .filter(|callback| callback.package_id == package_id)
        .map(|callback| {
            Ok(ArcanaCabiBindingSignature {
                name: callback.name.clone(),
                return_type: runtime_binding_return_type(callback.return_type.as_ref())?,
                params: runtime_binding_params_metadata(&callback.params)?,
            })
        })
        .collect()
}

pub(super) unsafe extern "system" fn runtime_binding_callback_trampoline(
    user_data: *mut c_void,
    args: *const ArcanaCabiBindingValueV1,
    arg_count: usize,
    out_write_backs: *mut ArcanaCabiBindingValueV1,
    out_result: *mut ArcanaCabiBindingValueV1,
) -> i32 {
    let Some(thunk_data) =
        (unsafe { (user_data as *mut RuntimeBindingCallbackThunkData).as_ref() })
    else {
        return 0;
    };
    let callback_args = if arg_count == 0 {
        &[]
    } else if args.is_null() {
        return 0;
    } else {
        unsafe { std::slice::from_raw_parts(args, arg_count) }
    };
    if arg_count != 0 && out_write_backs.is_null() {
        return 0;
    }
    let result = ACTIVE_RUNTIME_BINDING_CALLBACK_CONTEXT.with(|slot| {
        let borrow = slot.borrow();
        let Some(context) = *borrow else {
            return Err(
                "native binding callback invoked without an active runtime context".to_string(),
            );
        };
        let plan = unsafe { context.plan.as_ref() }
            .ok_or_else(|| "native binding callback lost runtime package plan".to_string())?;
        let host_ptr: *mut dyn RuntimeCoreHost =
            unsafe { std::mem::transmute((context.host_data, context.host_vtable)) };
        let host = unsafe { host_ptr.as_mut() }
            .ok_or_else(|| "native binding callback lost runtime core host".to_string())?;
        execute_runtime_binding_callback(plan, host, &thunk_data.callback, callback_args)
    });
    match result {
        Ok(outcome) => {
            if !out_write_backs.is_null() {
                let slots = unsafe { std::slice::from_raw_parts_mut(out_write_backs, arg_count) };
                slots.copy_from_slice(&outcome.write_backs);
            }
            if !out_result.is_null() {
                unsafe {
                    *out_result = outcome.result;
                }
            }
            1
        }
        Err(_) => {
            if !out_write_backs.is_null() {
                let slots = unsafe { std::slice::from_raw_parts_mut(out_write_backs, arg_count) };
                for slot in slots {
                    *slot = ArcanaCabiBindingValueV1::default();
                }
            }
            if !out_result.is_null() {
                unsafe {
                    *out_result = ArcanaCabiBindingValueV1::default();
                }
            }
            0
        }
    }
}

pub(super) fn execute_runtime_binding_callback(
    plan: &RuntimePackagePlan,
    host: &mut dyn RuntimeCoreHost,
    callback: &RuntimeNativeCallbackPlan,
    args: &[ArcanaCabiBindingValueV1],
) -> Result<RuntimeBindingCallbackOutcome, String> {
    if callback.params.len() != args.len() {
        return Err(format!(
            "native callback `{}` expected {} arguments, got {}",
            callback.name,
            callback.params.len(),
            args.len()
        ));
    }
    let routine_key = callback.target_routine_key.as_deref().ok_or_else(|| {
        format!(
            "native callback `{}` does not resolve to a runtime routine",
            callback.name
        )
    })?;
    let routine_index = plan
        .routines
        .iter()
        .enumerate()
        .find(|(_, routine)| routine.routine_key == routine_key)
        .map(|(index, _)| index)
        .ok_or_else(|| {
            format!(
                "native callback `{}` targets missing routine `{}`",
                callback.name, routine_key
            )
        })?;
    let mut state = RuntimeExecutionState::default();
    let runtime_args = callback
        .params
        .iter()
        .zip(args.iter())
        .map(|(param, value)| {
            let source_mode =
                ArcanaCabiParamSourceMode::from_param_mode_text(param.mode.as_deref())?;
            runtime_value_from_binding_input(
                &plan.binding_layouts,
                &callback.package_id,
                &param.ty.render(),
                source_mode,
                value,
                &mut state,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let outcome = execute_routine_call_with_state(
        plan,
        routine_index,
        Vec::new(),
        runtime_args,
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
            FlowSignal::OwnerExit {
                owner_key,
                exit_name,
            } => RuntimeEvalSignal::OwnerExit {
                owner_key,
                exit_name,
            },
            other => RuntimeEvalSignal::Message(format!(
                "unsupported native binding callback control flow `{other:?}`"
            )),
        }));
    }
    let write_backs = runtime_binding_callback_write_backs(
        &plan.binding_layouts,
        plan,
        callback,
        &outcome.final_args,
        &mut state,
        host,
    )?;
    let result = runtime_binding_output_from_runtime_value_with_context(
        &plan.binding_layouts,
        &callback.package_id,
        callback
            .return_type
            .as_ref()
            .map(IrRoutineType::render)
            .unwrap_or_else(|| "Unit".to_string())
            .as_str(),
        outcome.value,
        None,
        Some(plan),
        Some(&callback.package_id),
        Some(""),
        None,
        None,
        Some(&mut state),
        Some(host),
    )
    .inspect_err(|_err| {
        runtime_release_binding_values(write_backs.iter().copied());
    })?;
    Ok(RuntimeBindingCallbackOutcome {
        result,
        write_backs,
    })
}

pub(super) fn execute_runtime_native_binding_import(
    plan: &RuntimePackagePlan,
    routine: &RuntimeRoutinePlan,
    binding_name: &str,
    args: &[RuntimeValue],
    scopes: &mut Vec<RuntimeScope>,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RoutineExecutionOutcome, String> {
    let (host_data, host_vtable): (*mut (), *mut ()) =
        unsafe { std::mem::transmute::<&mut dyn RuntimeCoreHost, (*mut (), *mut ())>(host) };
    let mut storage = RuntimeBindingArgStorage::default();
    let binding_args = args
        .iter()
        .map(|value| {
            read_runtime_value_if_ref(
                value.clone(),
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cabi_args = routine
        .params
        .iter()
        .enumerate()
        .zip(binding_args.iter())
        .map(|((index, param), value)| {
            let source_mode =
                ArcanaCabiParamSourceMode::from_param_mode_text(param.mode.as_deref())?;
            runtime_binding_input_from_runtime_value(
                &plan.binding_layouts,
                &routine.package_id,
                &param.ty.render(),
                source_mode,
                value,
                &mut storage,
                index,
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let callback_specs = runtime_binding_callback_specs_for_package(plan, &routine.package_id);
    let expected_imports =
        runtime_binding_import_signatures_for_package(plan, &routine.package_id)?;
    let expected_callbacks =
        runtime_binding_callback_signatures_for_package(plan, &routine.package_id)?;
    let outcome = with_runtime_binding_callback_context(plan, host_data, host_vtable, || {
        let host_ptr: *mut dyn RuntimeCoreHost =
            unsafe { std::mem::transmute((host_data, host_vtable)) };
        let _ = unsafe { host_ptr.as_mut() }
            .ok_or_else(|| "native binding import lost runtime core host".to_string())?;
        with_runtime_native_products(|catalog| {
            catalog.invoke_binding_import(
                &routine.package_id,
                binding_name,
                &callback_specs,
                &expected_imports,
                &expected_callbacks,
                &plan.binding_layouts,
                &cabi_args,
            )
        })
    })?;
    let (final_args, skip_write_back_edit_indices) = runtime_final_args_from_binding_import(
        &plan.binding_layouts,
        &routine.package_id,
        &routine.params,
        args,
        &binding_args,
        &mut storage,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
        &outcome,
    )
    .inspect_err(|_err| {
        let _ = release_binding_output_value(
            outcome.result,
            outcome.owned_bytes_free,
            outcome.owned_str_free,
        );
    })?;
    let value = runtime_value_from_binding_cabi_output(
        &plan.binding_layouts,
        &routine.package_id,
        routine
            .return_type
            .as_ref()
            .map(IrRoutineType::render)
            .unwrap_or_else(|| "Unit".to_string())
            .as_str(),
        &outcome.result,
        state,
        outcome.owned_bytes_free,
        outcome.owned_str_free,
        "native binding import result",
    )?;
    Ok(RoutineExecutionOutcome {
        value,
        final_args,
        skip_write_back_edit_indices,
        control: None,
    })
}

pub(super) fn runtime_foreign_byte_len(
    plan: &RuntimePackagePlan,
    _host: &mut dyn RuntimeCoreHost,
    backing: RuntimeForeignByteViewBacking,
    context: &str,
) -> Result<usize, String> {
    let _ = context;
    let callback_specs = runtime_binding_callback_specs_for_package(plan, backing.package_id);
    let expected_imports = runtime_binding_import_signatures_for_package(plan, backing.package_id)?;
    let expected_callbacks =
        runtime_binding_callback_signatures_for_package(plan, backing.package_id)?;
    with_runtime_native_products(|catalog| {
        catalog.invoke_binding_mapped_view_len_bytes(
            backing.package_id,
            &callback_specs,
            &expected_imports,
            &expected_callbacks,
            &plan.binding_layouts,
            backing.handle,
        )
    })
}

pub(super) fn runtime_foreign_byte_at(
    plan: &RuntimePackagePlan,
    _host: &mut dyn RuntimeCoreHost,
    backing: RuntimeForeignByteViewBacking,
    index: usize,
    context: &str,
) -> Result<u8, String> {
    let _ = context;
    let callback_specs = runtime_binding_callback_specs_for_package(plan, backing.package_id);
    let expected_imports = runtime_binding_import_signatures_for_package(plan, backing.package_id)?;
    let expected_callbacks =
        runtime_binding_callback_signatures_for_package(plan, backing.package_id)?;
    with_runtime_native_products(|catalog| {
        catalog.invoke_binding_mapped_view_read_byte(
            backing.package_id,
            &callback_specs,
            &expected_imports,
            &expected_callbacks,
            &plan.binding_layouts,
            backing.handle,
            index,
        )
    })
}

pub(super) fn runtime_foreign_byte_set(
    plan: &RuntimePackagePlan,
    _host: &mut dyn RuntimeCoreHost,
    backing: RuntimeForeignByteViewBacking,
    index: usize,
    value: u8,
    context: &str,
) -> Result<(), String> {
    let _ = context;
    let callback_specs = runtime_binding_callback_specs_for_package(plan, backing.package_id);
    let expected_imports = runtime_binding_import_signatures_for_package(plan, backing.package_id)?;
    let expected_callbacks =
        runtime_binding_callback_signatures_for_package(plan, backing.package_id)?;
    with_runtime_native_products(|catalog| {
        catalog.invoke_binding_mapped_view_write_byte(
            backing.package_id,
            &callback_specs,
            &expected_imports,
            &expected_callbacks,
            &plan.binding_layouts,
            backing.handle,
            index,
            value,
        )
    })
}

pub(super) fn runtime_binding_input_from_runtime_value(
    layouts: &[ArcanaCabiBindingLayout],
    package_id: &str,
    expected_type: &str,
    source_mode: ArcanaCabiParamSourceMode,
    value: &RuntimeValue,
    storage: &mut RuntimeBindingArgStorage,
    param_index: usize,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<ArcanaCabiBindingValueV1, String> {
    let binding_type = ArcanaCabiBindingType::parse(expected_type)?;
    match (&binding_type, value) {
        (ArcanaCabiBindingType::ByteBuffer, value @ RuntimeValue::Bytes(_))
        | (ArcanaCabiBindingType::ByteBuffer, value @ RuntimeValue::ByteBuffer(_))
        | (ArcanaCabiBindingType::ByteBuffer, value @ RuntimeValue::Array(_))
            if source_mode == ArcanaCabiParamSourceMode::Edit =>
        {
            let bytes = runtime_binding_bytes_from_runtime_value(value, "binding byte argument")?;
            storage.bytes.push(bytes);
            let bytes_index = storage.bytes.len() - 1;
            storage.in_place_edits.insert(
                param_index,
                RuntimeBindingInPlaceEdit::ByteBuffer { bytes_index },
            );
            let stored = storage
                .bytes
                .last()
                .ok_or_else(|| "binding arg storage lost bytes value".to_string())?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Bytes as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: contiguous_u8_view(stored.as_ptr(), stored.len(), 0),
                },
            })
        }
        (ArcanaCabiBindingType::Utf16Buffer, RuntimeValue::Utf16(units))
        | (ArcanaCabiBindingType::Utf16Buffer, RuntimeValue::Utf16Buffer(units))
            if source_mode == ArcanaCabiParamSourceMode::Edit =>
        {
            storage
                .bytes
                .push(runtime_utf16_units_to_binding_bytes(units));
            let bytes_index = storage.bytes.len() - 1;
            storage.in_place_edits.insert(
                param_index,
                RuntimeBindingInPlaceEdit::Utf16Buffer { bytes_index },
            );
            let stored = storage
                .bytes
                .last()
                .ok_or_else(|| "binding arg storage lost utf16 value".to_string())?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Bytes as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: contiguous_u8_view(stored.as_ptr(), stored.len(), 0),
                },
            })
        }
        (ArcanaCabiBindingType::View(view_type), value) => {
            let snapshot = runtime_binding_snapshot_view_input(
                layouts,
                view_type,
                value,
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
                "native binding argument view",
            )?;
            storage.bytes.push(snapshot.bytes);
            let bytes_index = storage.bytes.len() - 1;
            if source_mode == ArcanaCabiParamSourceMode::Edit {
                storage.in_place_edits.insert(
                    param_index,
                    RuntimeBindingInPlaceEdit::View {
                        original: value.clone(),
                        bytes_index,
                        expected_type: expected_type.to_string(),
                    },
                );
            }
            let stored = storage
                .bytes
                .last()
                .ok_or_else(|| "binding arg storage lost view bytes".to_string())?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::View as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: raw_view(
                        stored.as_ptr(),
                        snapshot.len,
                        snapshot.stride_bytes,
                        snapshot.family.cabi_tag(),
                        u32::try_from(snapshot.element_size).map_err(|_| {
                            format!(
                                "binding view element size `{}` does not fit u32",
                                snapshot.element_size
                            )
                        })?,
                        snapshot.flags,
                    ),
                },
            })
        }
        _ => runtime_binding_input_from_runtime_value_legacy(
            layouts,
            package_id,
            expected_type,
            value,
            storage,
        ),
    }
}

struct RuntimeBindingViewSnapshot {
    bytes: Vec<u8>,
    len: usize,
    stride_bytes: usize,
    element_size: usize,
    family: ArcanaCabiViewFamily,
    flags: u32,
}

pub(super) fn runtime_binding_view_element_raw_type(
    view_type: &ArcanaCabiBindingViewType,
) -> Result<ArcanaCabiBindingRawType, String> {
    if let Some(scalar) = view_type.element_type.as_ref().clone().scalar() {
        return Ok(ArcanaCabiBindingRawType::Scalar(scalar));
    }
    match view_type.element_type.as_ref() {
        ArcanaCabiBindingType::Named(layout_id) => {
            Ok(ArcanaCabiBindingRawType::Named(layout_id.clone()))
        }
        other => Err(format!(
            "binding View element type `{}` is not raw-layout-safe",
            other.render()
        )),
    }
}

pub(super) fn runtime_binding_view_element_size(
    layouts: &[ArcanaCabiBindingLayout],
    view_type: &ArcanaCabiBindingViewType,
) -> Result<usize, String> {
    runtime_binding_raw_type_size(layouts, &runtime_binding_view_element_raw_type(view_type)?)
}

fn runtime_binding_snapshot_view_input(
    layouts: &[ArcanaCabiBindingLayout],
    view_type: &ArcanaCabiBindingViewType,
    value: &RuntimeValue,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<RuntimeBindingViewSnapshot, String> {
    let raw_element_type = runtime_binding_view_element_raw_type(view_type)?;
    let element_size = runtime_binding_view_element_size(layouts, view_type)?;
    if matches!(
        raw_element_type,
        ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U8)
    ) {
        match value {
            RuntimeValue::Bytes(bytes) | RuntimeValue::ByteBuffer(bytes) => {
                return Ok(RuntimeBindingViewSnapshot {
                    bytes: bytes.clone(),
                    len: bytes.len(),
                    stride_bytes: 1,
                    element_size,
                    family: view_type.family,
                    flags: 0,
                });
            }
            RuntimeValue::Array(values) => {
                let bytes =
                    runtime_binding_bytes_from_runtime_array(values, "binding view byte snapshot")?;
                return Ok(RuntimeBindingViewSnapshot {
                    len: bytes.len(),
                    bytes,
                    stride_bytes: 1,
                    element_size,
                    family: view_type.family,
                    flags: 0,
                });
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
                let view = state
                    .byte_views
                    .get(handle)
                    .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                    .clone();
                let len = view.len;
                let bytes = runtime_byte_view_values(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &view,
                    host,
                    context,
                )?;
                return Ok(RuntimeBindingViewSnapshot {
                    bytes,
                    len,
                    stride_bytes: 1,
                    element_size,
                    family: view_type.family,
                    flags: 0,
                });
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                let view = state
                    .byte_edit_views
                    .get(handle)
                    .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                    .clone();
                let len = view.len;
                let bytes = runtime_byte_edit_view_values(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &view,
                    host,
                    context,
                )?;
                return Ok(RuntimeBindingViewSnapshot {
                    bytes,
                    len,
                    stride_bytes: 1,
                    element_size,
                    family: view_type.family,
                    flags: 0,
                });
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
                let view = state
                    .str_views
                    .get(handle)
                    .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                    .clone();
                let text = runtime_str_view_text(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &view,
                    host,
                    context,
                )?;
                let bytes = text.into_bytes();
                return Ok(RuntimeBindingViewSnapshot {
                    len: bytes.len(),
                    bytes,
                    stride_bytes: 1,
                    element_size,
                    family: view_type.family,
                    flags: ARCANA_CABI_VIEW_FLAG_UTF8,
                });
            }
            _ => {}
        }
    }
    if matches!(
        raw_element_type,
        ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U16)
    ) && matches!(value, RuntimeValue::Utf16(_) | RuntimeValue::Utf16Buffer(_))
    {
        let units = match value {
            RuntimeValue::Utf16(units) | RuntimeValue::Utf16Buffer(units) => units,
            _ => unreachable!(),
        };
        return Ok(RuntimeBindingViewSnapshot {
            bytes: runtime_utf16_units_to_binding_bytes(units),
            len: units.len(),
            stride_bytes: element_size,
            element_size,
            family: view_type.family,
            flags: 0,
        });
    }
    let values = match value {
        RuntimeValue::Array(values) => values.clone(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
            let view = state
                .read_views
                .get(handle)
                .ok_or_else(|| format!("invalid ReadView handle `{}`", handle.0))?
                .clone();
            runtime_read_view_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &view,
                host,
                context,
            )?
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
            let view = state
                .edit_views
                .get(handle)
                .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?
                .clone();
            let read_view = RuntimeReadViewState {
                type_args: view.type_args.clone(),
                backing: view.backing.clone(),
                start: view.start,
                len: view.len,
            };
            runtime_read_view_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &read_view,
                host,
                context,
            )?
        }
        other => {
            return Err(format!(
                "{context} expected View-compatible value, got `{}`",
                runtime_value_type_root(other).unwrap_or_else(|| format!("{other:?}"))
            ));
        }
    };
    let mut bytes = Vec::with_capacity(values.len().saturating_mul(element_size));
    for (index, item) in values.iter().enumerate() {
        let encoded = runtime_binding_encode_raw_type(
            layouts,
            &raw_element_type,
            item,
            &format!("{context}[{index}]"),
        )?;
        bytes.extend_from_slice(&encoded);
    }
    Ok(RuntimeBindingViewSnapshot {
        len: values.len(),
        bytes,
        stride_bytes: element_size,
        element_size,
        family: view_type.family,
        flags: 0,
    })
}

pub(super) fn runtime_binding_input_from_runtime_value_legacy(
    layouts: &[ArcanaCabiBindingLayout],
    package_id: &str,
    expected_type: &str,
    value: &RuntimeValue,
    storage: &mut RuntimeBindingArgStorage,
) -> Result<ArcanaCabiBindingValueV1, String> {
    let binding_type = ArcanaCabiBindingType::parse(expected_type)?;
    match (&binding_type, value) {
        (ArcanaCabiBindingType::Int, RuntimeValue::Int(value)) => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Int as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 { int_value: *value },
        }),
        (ArcanaCabiBindingType::Bool, RuntimeValue::Bool(value)) => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Bool as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                bool_value: if *value { 1 } else { 0 },
            },
        }),
        (ArcanaCabiBindingType::Str, RuntimeValue::Str(value)) => {
            storage.strings.push(value.clone());
            let stored = storage
                .strings
                .last()
                .ok_or_else(|| "binding arg storage lost string value".to_string())?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Str as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: contiguous_u8_view(
                        stored.as_bytes().as_ptr(),
                        stored.len(),
                        ARCANA_CABI_VIEW_FLAG_UTF8,
                    ),
                },
            })
        }
        (ArcanaCabiBindingType::Bytes, value @ RuntimeValue::Bytes(_))
        | (ArcanaCabiBindingType::Bytes, value @ RuntimeValue::ByteBuffer(_))
        | (ArcanaCabiBindingType::Bytes, value @ RuntimeValue::Array(_))
        | (ArcanaCabiBindingType::ByteBuffer, value @ RuntimeValue::Bytes(_))
        | (ArcanaCabiBindingType::ByteBuffer, value @ RuntimeValue::ByteBuffer(_))
        | (ArcanaCabiBindingType::ByteBuffer, value @ RuntimeValue::Array(_)) => {
            let bytes = runtime_binding_bytes_from_runtime_value(value, "binding byte argument")?;
            storage.bytes.push(bytes);
            let stored = storage
                .bytes
                .last()
                .ok_or_else(|| "binding arg storage lost bytes value".to_string())?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Bytes as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: contiguous_u8_view(stored.as_ptr(), stored.len(), 0),
                },
            })
        }
        (ArcanaCabiBindingType::Utf16, RuntimeValue::Utf16(units))
        | (ArcanaCabiBindingType::Utf16, RuntimeValue::Utf16Buffer(units))
        | (ArcanaCabiBindingType::Utf16Buffer, RuntimeValue::Utf16(units))
        | (ArcanaCabiBindingType::Utf16Buffer, RuntimeValue::Utf16Buffer(units)) => {
            storage
                .bytes
                .push(runtime_utf16_units_to_binding_bytes(units));
            let stored = storage
                .bytes
                .last()
                .ok_or_else(|| "binding arg storage lost utf16 value".to_string())?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Bytes as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: contiguous_u8_view(stored.as_ptr(), stored.len(), 0),
                },
            })
        }
        (ArcanaCabiBindingType::Unit, RuntimeValue::Unit) => {
            Ok(ArcanaCabiBindingValueV1::default())
        }
        (_, RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)))
            if binding.package_id == package_id && binding.type_name == expected_type =>
        {
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Opaque as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    opaque_value: binding.handle,
                },
            })
        }
        (ArcanaCabiBindingType::Named(layout_id), value) => {
            let bytes = runtime_binding_encode_layout_value(layouts, layout_id, value)?;
            storage.bytes.push(bytes);
            let stored = storage
                .bytes
                .last()
                .ok_or_else(|| "binding arg storage lost layout bytes".to_string())?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Layout as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: contiguous_u8_view(stored.as_ptr(), stored.len(), 0),
                },
            })
        }
        (ty, value) if ty.clone().scalar().is_some() => {
            runtime_binding_scalar_input_value(ty, value, expected_type)
        }
        _ => Err(format!(
            "native binding argument expected `{expected_type}`, got `{}`",
            runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
        )),
    }
}

pub(super) fn runtime_value_from_binding_input(
    layouts: &[ArcanaCabiBindingLayout],
    package_id: &str,
    expected_type: &str,
    source_mode: ArcanaCabiParamSourceMode,
    value: &ArcanaCabiBindingValueV1,
    state: &mut RuntimeExecutionState,
) -> Result<RuntimeValue, String> {
    let binding_type = ArcanaCabiBindingType::parse(expected_type)?;
    match (&binding_type, value.tag()?) {
        (ArcanaCabiBindingType::Int, ArcanaCabiBindingValueTag::Int) => {
            Ok(RuntimeValue::Int(unsafe { value.payload.int_value }))
        }
        (ArcanaCabiBindingType::Bool, ArcanaCabiBindingValueTag::Bool) => {
            Ok(RuntimeValue::Bool(unsafe { value.payload.bool_value != 0 }))
        }
        (ArcanaCabiBindingType::Str, ArcanaCabiBindingValueTag::Str) => {
            let view: ArcanaViewV1 = unsafe { value.payload.view_value };
            let bytes = runtime_binding_input_bytes_view(
                view.ptr.cast(),
                view.len,
                "native binding string arg",
            )?;
            Ok(RuntimeValue::Str(
                std::str::from_utf8(bytes)
                    .map_err(|err| format!("native binding string arg is not utf-8: {err}"))?
                    .to_string(),
            ))
        }
        (ArcanaCabiBindingType::Bytes, ArcanaCabiBindingValueTag::Bytes) => {
            let view: ArcanaViewV1 = unsafe { value.payload.view_value };
            let bytes =
                runtime_binding_input_bytes_view(view.ptr, view.len, "native binding bytes arg")?;
            Ok(RuntimeValue::Bytes(bytes.to_vec()))
        }
        (ArcanaCabiBindingType::ByteBuffer, ArcanaCabiBindingValueTag::Bytes) => {
            let view: ArcanaViewV1 = unsafe { value.payload.view_value };
            let bytes = runtime_binding_input_bytes_view(
                view.ptr,
                view.len,
                "native binding byte buffer arg",
            )?;
            Ok(RuntimeValue::ByteBuffer(bytes.to_vec()))
        }
        (ArcanaCabiBindingType::Utf16, ArcanaCabiBindingValueTag::Bytes) => {
            let view: ArcanaViewV1 = unsafe { value.payload.view_value };
            let bytes =
                runtime_binding_input_bytes_view(view.ptr, view.len, "native binding utf16 arg")?;
            Ok(RuntimeValue::Utf16(runtime_utf16_units_from_binding_bytes(
                bytes,
                "native binding utf16 arg",
            )?))
        }
        (ArcanaCabiBindingType::Utf16Buffer, ArcanaCabiBindingValueTag::Bytes) => {
            let view: ArcanaViewV1 = unsafe { value.payload.view_value };
            let bytes = runtime_binding_input_bytes_view(
                view.ptr,
                view.len,
                "native binding utf16 buffer arg",
            )?;
            Ok(RuntimeValue::Utf16Buffer(
                runtime_utf16_units_from_binding_bytes(bytes, "native binding utf16 buffer arg")?,
            ))
        }
        (ArcanaCabiBindingType::Unit, ArcanaCabiBindingValueTag::Unit) => Ok(RuntimeValue::Unit),
        (ArcanaCabiBindingType::Named(layout_id), ArcanaCabiBindingValueTag::Layout) => {
            let view: ArcanaViewV1 = unsafe { value.payload.view_value };
            let bytes =
                runtime_binding_input_bytes_view(view.ptr, view.len, "native binding layout arg")?;
            runtime_binding_decode_layout_value(layouts, layout_id, bytes)
        }
        (ArcanaCabiBindingType::Named(_), ArcanaCabiBindingValueTag::Opaque) => Ok(
            RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(RuntimeBindingOpaqueValue {
                package_id: leak_runtime_binding_text(package_id),
                type_name: leak_runtime_binding_text(expected_type),
                handle: unsafe { value.payload.opaque_value },
            })),
        ),
        (ArcanaCabiBindingType::View(view_type), ArcanaCabiBindingValueTag::View) => {
            runtime_value_from_binding_view_input(layouts, view_type, source_mode, value, state)
        }
        (ty, actual) if ty.clone().scalar().is_some() => {
            runtime_binding_runtime_value_from_scalar_tag(ty, actual, value)
        }
        (_, actual) => Err(format!(
            "native binding callback expected `{expected_type}`, got tag `{actual:?}`"
        )),
    }
}

pub(super) fn runtime_value_from_binding_view_input(
    layouts: &[ArcanaCabiBindingLayout],
    view_type: &ArcanaCabiBindingViewType,
    source_mode: ArcanaCabiParamSourceMode,
    value: &ArcanaCabiBindingValueV1,
    state: &mut RuntimeExecutionState,
) -> Result<RuntimeValue, String> {
    let view = unsafe { value.payload.view_value };
    if view.family != view_type.family.cabi_tag() {
        return Err(format!(
            "native binding view arg family mismatch: expected `{}`, got `{}`",
            view_type.family.as_str(),
            view.family
        ));
    }
    let element_size = runtime_binding_view_element_size(layouts, view_type)?;
    let actual_element_size = usize::try_from(view.element_size)
        .map_err(|_| "native binding view arg element size does not fit usize".to_string())?;
    if actual_element_size != element_size {
        return Err(format!(
            "native binding view arg element size mismatch: expected `{element_size}`, got `{actual_element_size}`"
        ));
    }
    let total = view_total_bytes(view)?;
    let bytes = if total == 0 {
        Vec::new()
    } else {
        if view.ptr.is_null() {
            return Err(format!(
                "native binding view arg returned null data with len {}",
                view.len
            ));
        }
        let raw = unsafe { std::slice::from_raw_parts(view.ptr, total) };
        raw.to_vec()
    };
    runtime_runtime_value_from_binding_view_bytes(
        layouts,
        view_type,
        source_mode,
        view,
        bytes,
        state,
        "native binding view arg",
    )
}

pub(super) fn runtime_runtime_value_from_binding_view_bytes(
    layouts: &[ArcanaCabiBindingLayout],
    view_type: &ArcanaCabiBindingViewType,
    source_mode: ArcanaCabiParamSourceMode,
    view: ArcanaViewV1,
    bytes: Vec<u8>,
    state: &mut RuntimeExecutionState,
    context: &str,
) -> Result<RuntimeValue, String> {
    let element_size = runtime_binding_view_element_size(layouts, view_type)?;
    if matches!(
        view_type.element_type.as_ref().clone().scalar(),
        Some(ArcanaCabiBindingScalarType::U8)
    ) {
        let stride = if view.stride_bytes == 0 {
            1
        } else {
            view.stride_bytes
        };
        let compact = if stride == 1 {
            bytes
        } else {
            let mut compact = Vec::with_capacity(view.len);
            for index in 0..view.len {
                let byte = *bytes
                    .get(index * stride)
                    .ok_or_else(|| format!("{context} index `{index}` is out of bounds"))?;
                compact.push(byte);
            }
            compact
        };
        let backing = insert_runtime_byte_view_buffer(state, compact);
        return Ok(match source_mode {
            ArcanaCabiParamSourceMode::Edit => {
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(
                    insert_runtime_byte_edit_view_from_buffer(state, backing, 0, view.len),
                ))
            }
            ArcanaCabiParamSourceMode::Read | ArcanaCabiParamSourceMode::Take => {
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(
                    insert_runtime_byte_view_from_buffer(state, backing, 0, view.len),
                ))
            }
        });
    }
    let raw_element_type = runtime_binding_view_element_raw_type(view_type)?;
    let mut values = Vec::with_capacity(view.len);
    let stride = if view.stride_bytes == 0 {
        element_size
    } else {
        view.stride_bytes
    };
    for index in 0..view.len {
        let start = index
            .checked_mul(stride)
            .ok_or_else(|| "native binding view arg byte offset overflowed usize".to_string())?;
        let end = start
            .checked_add(element_size)
            .ok_or_else(|| "native binding view arg byte range overflowed usize".to_string())?;
        let slice = bytes
            .get(start..end)
            .ok_or_else(|| format!("{context} index `{index}` is out of bounds"))?;
        values.push(runtime_binding_decode_raw_type(
            layouts,
            &raw_element_type,
            slice,
            &format!("{context}[{index}]"),
        )?);
    }
    let type_args = vec![view_type.element_type.render()];
    Ok(match source_mode {
        ArcanaCabiParamSourceMode::Edit => RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(
            insert_runtime_edit_view(state, &type_args, values),
        )),
        ArcanaCabiParamSourceMode::Read | ArcanaCabiParamSourceMode::Take => RuntimeValue::Opaque(
            RuntimeOpaqueValue::ReadView(insert_runtime_read_view(state, &type_args, values)),
        ),
    })
}

pub(super) fn runtime_apply_binding_view_in_place_edit(
    layouts: &[ArcanaCabiBindingLayout],
    expected_type: &str,
    original: &RuntimeValue,
    bytes: &[u8],
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    let binding_type = ArcanaCabiBindingType::parse(expected_type)?;
    let ArcanaCabiBindingType::View(view_type) = binding_type else {
        return Err(format!(
            "binding in-place view edit expected `View[...]`, got `{expected_type}`"
        ));
    };
    let element_size = runtime_binding_view_element_size(layouts, &view_type)?;
    match original {
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
            let view = state
                .byte_edit_views
                .get(handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            if element_size != 1 {
                return Err(format!(
                    "binding in-place ByteEditView edit expected U8 elements, got `{}`",
                    view_type.element_type.render()
                ));
            }
            if bytes.len() != view.len {
                return Err(format!(
                    "binding in-place ByteEditView edit length mismatch: expected {}, got {}",
                    view.len,
                    bytes.len()
                ));
            }
            for (index, byte) in bytes.iter().enumerate() {
                let mut args = vec![
                    original.clone(),
                    RuntimeValue::Int(index as i64),
                    RuntimeValue::Int(i64::from(*byte)),
                ];
                let _ = execute_runtime_intrinsic(
                    RuntimeIntrinsic::MemoryByteEditViewSet,
                    &[],
                    &mut args,
                    plan,
                    Some(scopes),
                    Some(current_package_id),
                    Some(current_module_id),
                    Some(aliases),
                    Some(type_bindings),
                    state,
                    host,
                )?;
            }
            Ok(())
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
            let view = state
                .edit_views
                .get(handle)
                .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?
                .clone();
            let expected_bytes = view.len.checked_mul(element_size).ok_or_else(|| {
                "binding in-place view edit byte length overflowed usize".to_string()
            })?;
            if bytes.len() != expected_bytes {
                return Err(format!(
                    "binding in-place EditView edit length mismatch: expected {}, got {}",
                    expected_bytes,
                    bytes.len()
                ));
            }
            let raw_element_type = runtime_binding_view_element_raw_type(&view_type)?;
            for index in 0..view.len {
                let start = index * element_size;
                let end = start + element_size;
                let value = runtime_binding_decode_raw_type(
                    layouts,
                    &raw_element_type,
                    &bytes[start..end],
                    &format!("binding in-place view edit[{index}]"),
                )?;
                let mut args = vec![original.clone(), RuntimeValue::Int(index as i64), value];
                let _ = execute_runtime_intrinsic(
                    RuntimeIntrinsic::MemoryEditViewSet,
                    &[],
                    &mut args,
                    plan,
                    Some(scopes),
                    Some(current_package_id),
                    Some(current_module_id),
                    Some(aliases),
                    Some(type_bindings),
                    state,
                    host,
                )?;
            }
            Ok(())
        }
        other => Err(format!(
            "binding in-place view edit expected editable view value, got `{}`",
            runtime_value_type_root(other).unwrap_or_else(|| format!("{other:?}"))
        )),
    }
}

pub(super) fn runtime_value_from_binding_cabi_output(
    layouts: &[ArcanaCabiBindingLayout],
    package_id: &str,
    expected_type: &str,
    value: &ArcanaCabiBindingValueV1,
    state: &mut RuntimeExecutionState,
    owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    owned_str_free: ArcanaCabiOwnedStrFreeFn,
    label: &str,
) -> Result<RuntimeValue, String> {
    let actual = value.tag()?;
    let binding_type = ArcanaCabiBindingType::parse(expected_type)?;
    match (&binding_type, actual) {
        (ArcanaCabiBindingType::Int, ArcanaCabiBindingValueTag::Int) => {
            Ok(RuntimeValue::Int(unsafe { value.payload.int_value }))
        }
        (ArcanaCabiBindingType::Bool, ArcanaCabiBindingValueTag::Bool) => {
            Ok(RuntimeValue::Bool(unsafe { value.payload.bool_value != 0 }))
        }
        (ArcanaCabiBindingType::Str, ArcanaCabiBindingValueTag::Str) => {
            let owned = unsafe { value.payload.owned_str_value };
            let text = clone_owned_binding_str(owned, owned_str_free)?;
            Ok(RuntimeValue::Str(text))
        }
        (ArcanaCabiBindingType::Bytes, ArcanaCabiBindingValueTag::Bytes) => {
            let owned = unsafe { value.payload.owned_bytes_value };
            let bytes = clone_owned_binding_bytes(owned, owned_bytes_free)?;
            Ok(RuntimeValue::Array(
                bytes
                    .into_iter()
                    .map(|byte| RuntimeValue::Int(i64::from(byte)))
                    .collect(),
            ))
        }
        (ArcanaCabiBindingType::ByteBuffer, ArcanaCabiBindingValueTag::Bytes) => {
            let owned = unsafe { value.payload.owned_bytes_value };
            let bytes = clone_owned_binding_bytes(owned, owned_bytes_free)?;
            Ok(RuntimeValue::ByteBuffer(bytes))
        }
        (ArcanaCabiBindingType::Utf16, ArcanaCabiBindingValueTag::Bytes) => {
            let owned = unsafe { value.payload.owned_bytes_value };
            let bytes = clone_owned_binding_bytes(owned, owned_bytes_free)?;
            Ok(RuntimeValue::Utf16(runtime_utf16_units_from_binding_bytes(
                &bytes, label,
            )?))
        }
        (ArcanaCabiBindingType::Utf16Buffer, ArcanaCabiBindingValueTag::Bytes) => {
            let owned = unsafe { value.payload.owned_bytes_value };
            let bytes = clone_owned_binding_bytes(owned, owned_bytes_free)?;
            Ok(RuntimeValue::Utf16Buffer(
                runtime_utf16_units_from_binding_bytes(&bytes, label)?,
            ))
        }
        (ArcanaCabiBindingType::Unit, ArcanaCabiBindingValueTag::Unit) => Ok(RuntimeValue::Unit),
        (ArcanaCabiBindingType::Named(layout_id), ArcanaCabiBindingValueTag::Layout) => {
            let owned = unsafe { value.payload.owned_bytes_value };
            let bytes = clone_owned_binding_bytes(owned, owned_bytes_free)?;
            runtime_binding_decode_layout_value(layouts, layout_id, &bytes)
        }
        (ArcanaCabiBindingType::Named(_), ArcanaCabiBindingValueTag::Opaque) => Ok(
            RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(RuntimeBindingOpaqueValue {
                package_id: leak_runtime_binding_text(package_id),
                type_name: leak_runtime_binding_text(expected_type),
                handle: unsafe { value.payload.opaque_value },
            })),
        ),
        (ArcanaCabiBindingType::View(view_type), ArcanaCabiBindingValueTag::View) => {
            let view = unsafe { value.payload.view_value };
            let bytes = clone_binding_view_bytes(view, owned_bytes_free)?;
            runtime_runtime_value_from_binding_view_bytes(
                layouts,
                view_type,
                ArcanaCabiParamSourceMode::Read,
                view,
                bytes,
                state,
                label,
            )
        }
        (ty, actual) if ty.clone().scalar().is_some() => {
            runtime_binding_runtime_value_from_scalar_tag(ty, actual, value)
        }
        (_, ArcanaCabiBindingValueTag::Str) => {
            let _ = release_binding_output_value(*value, owned_bytes_free, owned_str_free);
            Err(format!("{label} expected `{expected_type}`, got tag `Str`"))
        }
        (_, ArcanaCabiBindingValueTag::Bytes | ArcanaCabiBindingValueTag::Layout) => {
            let _ = release_binding_output_value(*value, owned_bytes_free, owned_str_free);
            Err(format!(
                "{label} expected `{expected_type}`, got tag `{actual:?}`"
            ))
        }
        (_, actual) => Err(format!(
            "{label} expected `{expected_type}`, got tag `{actual:?}`"
        )),
    }
}

pub(super) fn runtime_binding_callback_write_backs(
    layouts: &[ArcanaCabiBindingLayout],
    plan: &RuntimePackagePlan,
    callback: &RuntimeNativeCallbackPlan,
    final_args: &[RuntimeValue],
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<Vec<ArcanaCabiBindingValueV1>, String> {
    if callback.params.len() != final_args.len() {
        return Err(format!(
            "native callback `{}` final arg count mismatch: expected {}, got {}",
            callback.name,
            callback.params.len(),
            final_args.len()
        ));
    }
    let metadata = runtime_binding_params_metadata(&callback.params)?;
    let mut write_backs = binding_write_back_slots(&metadata);
    for (index, (param, value)) in callback.params.iter().zip(final_args.iter()).enumerate() {
        if param.mode.as_deref() != Some("edit") {
            continue;
        }
        match runtime_binding_output_from_runtime_value_with_context(
            layouts,
            &callback.package_id,
            &param.ty.render(),
            value.clone(),
            None,
            Some(plan),
            Some(&callback.package_id),
            Some(""),
            None,
            None,
            Some(state),
            Some(host),
        ) {
            Ok(write_back) => write_backs[index] = write_back,
            Err(err) => {
                runtime_release_binding_values(write_backs.iter().copied());
                return Err(err);
            }
        }
    }
    Ok(write_backs)
}

pub(super) fn runtime_final_args_from_binding_import(
    layouts: &[ArcanaCabiBindingLayout],
    package_id: &str,
    params: &[RuntimeParamPlan],
    args: &[RuntimeValue],
    binding_args: &[RuntimeValue],
    storage: &mut RuntimeBindingArgStorage,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    outcome: &RuntimeBindingImportOutcome,
) -> Result<(Vec<RuntimeValue>, BTreeSet<usize>), String> {
    if params.len() != args.len() {
        return Err(format!(
            "native binding import arg count mismatch: expected {}, got {}",
            params.len(),
            args.len()
        ));
    }
    let mut final_args = args.to_vec();
    let mut skip_write_back_edit_indices = BTreeSet::new();
    for (&index, edit) in &storage.in_place_edits {
        match edit {
            RuntimeBindingInPlaceEdit::ByteBuffer { bytes_index } => {
                let bytes =
                    storage.bytes.get(*bytes_index).cloned().ok_or_else(|| {
                        format!("binding byte buffer edit `{index}` lost storage")
                    })?;
                final_args[index] = RuntimeValue::ByteBuffer(bytes);
            }
            RuntimeBindingInPlaceEdit::Utf16Buffer { bytes_index } => {
                let bytes = storage
                    .bytes
                    .get(*bytes_index)
                    .ok_or_else(|| format!("binding utf16 buffer edit `{index}` lost storage"))?;
                final_args[index] = RuntimeValue::Utf16Buffer(
                    runtime_utf16_units_from_binding_bytes(bytes, "native binding utf16 edit")?,
                );
            }
            RuntimeBindingInPlaceEdit::View {
                original,
                bytes_index,
                expected_type,
            } => {
                let bytes = storage
                    .bytes
                    .get(*bytes_index)
                    .cloned()
                    .ok_or_else(|| format!("binding view edit `{index}` lost storage"))?;
                runtime_apply_binding_view_in_place_edit(
                    layouts,
                    expected_type,
                    original,
                    &bytes,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                final_args[index] = args
                    .get(index)
                    .cloned()
                    .ok_or_else(|| format!("binding view edit `{index}` lost original arg"))?;
                skip_write_back_edit_indices.insert(index);
            }
        }
    }
    for (index, param) in params.iter().enumerate() {
        if param.mode.as_deref() != Some("edit") {
            continue;
        }
        if storage.in_place_edits.contains_key(&index) {
            continue;
        }
        let expected_type = param.ty.render();
        let write_back = &outcome.write_backs[index];
        if write_back.tag()? == ArcanaCabiBindingValueTag::Unit {
            if let Some(value) = runtime_binding_preserved_opaque_edit_arg(
                package_id,
                &expected_type,
                args.get(index),
                binding_args.get(index),
            ) {
                final_args[index] = value;
                continue;
            }
        }
        match runtime_value_from_binding_cabi_output(
            layouts,
            package_id,
            &expected_type,
            write_back,
            state,
            outcome.owned_bytes_free,
            outcome.owned_str_free,
            "native binding import write-back",
        ) {
            Ok(value) => final_args[index] = value,
            Err(err) => {
                runtime_release_binding_values(outcome.write_backs[index..].iter().copied());
                return Err(err);
            }
        }
    }
    let _ = binding_args;
    Ok((final_args, skip_write_back_edit_indices))
}

fn runtime_binding_preserved_opaque_edit_arg(
    package_id: &str,
    expected_type: &str,
    original_arg: Option<&RuntimeValue>,
    binding_arg: Option<&RuntimeValue>,
) -> Option<RuntimeValue> {
    let candidates = [binding_arg, original_arg];
    for candidate in candidates {
        let Some(RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding))) = candidate else {
            continue;
        };
        if binding.package_id == package_id && binding.type_name == expected_type {
            return Some(RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(
                binding.clone(),
            )));
        }
    }
    None
}

#[cfg(test)]
pub(super) fn runtime_binding_output_from_runtime_value(
    layouts: &[ArcanaCabiBindingLayout],
    package_id: &str,
    expected_type: &str,
    value: RuntimeValue,
) -> Result<ArcanaCabiBindingValueV1, String> {
    runtime_binding_output_from_runtime_value_with_context(
        layouts,
        package_id,
        expected_type,
        value,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

pub(super) fn runtime_binding_output_from_runtime_value_with_context(
    layouts: &[ArcanaCabiBindingLayout],
    package_id: &str,
    expected_type: &str,
    value: RuntimeValue,
    scopes: Option<&mut Vec<RuntimeScope>>,
    plan: Option<&RuntimePackagePlan>,
    current_package_id: Option<&str>,
    current_module_id: Option<&str>,
    aliases: Option<&BTreeMap<String, Vec<String>>>,
    type_bindings: Option<&RuntimeTypeBindings>,
    state: Option<&mut RuntimeExecutionState>,
    host: Option<&mut dyn RuntimeCoreHost>,
) -> Result<ArcanaCabiBindingValueV1, String> {
    let actual_type = runtime_value_type_root(&value).unwrap_or_else(|| format!("{value:?}"));
    let binding_type = ArcanaCabiBindingType::parse(expected_type)?;
    match (&binding_type, value) {
        (ArcanaCabiBindingType::Int, RuntimeValue::Int(value)) => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Int as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 { int_value: value },
        }),
        (ArcanaCabiBindingType::Bool, RuntimeValue::Bool(value)) => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Bool as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                bool_value: if value { 1 } else { 0 },
            },
        }),
        (ArcanaCabiBindingType::Str, RuntimeValue::Str(value)) => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Str as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                owned_str_value: into_owned_str(value),
            },
        }),
        (ArcanaCabiBindingType::Bytes, RuntimeValue::Bytes(bytes))
        | (ArcanaCabiBindingType::Bytes, RuntimeValue::ByteBuffer(bytes))
        | (ArcanaCabiBindingType::ByteBuffer, RuntimeValue::Bytes(bytes))
        | (ArcanaCabiBindingType::ByteBuffer, RuntimeValue::ByteBuffer(bytes)) => {
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Bytes as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    owned_bytes_value: into_owned_bytes(bytes),
                },
            })
        }
        (ArcanaCabiBindingType::Bytes, RuntimeValue::Array(values))
        | (ArcanaCabiBindingType::ByteBuffer, RuntimeValue::Array(values)) => {
            let bytes =
                runtime_binding_bytes_from_owned_runtime_array(values, "binding bytes output")?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Bytes as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    owned_bytes_value: into_owned_bytes(bytes),
                },
            })
        }
        (ArcanaCabiBindingType::Utf16, RuntimeValue::Utf16(units))
        | (ArcanaCabiBindingType::Utf16, RuntimeValue::Utf16Buffer(units))
        | (ArcanaCabiBindingType::Utf16Buffer, RuntimeValue::Utf16(units))
        | (ArcanaCabiBindingType::Utf16Buffer, RuntimeValue::Utf16Buffer(units)) => {
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Bytes as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    owned_bytes_value: into_owned_bytes(runtime_utf16_units_to_binding_bytes(
                        &units,
                    )),
                },
            })
        }
        (ArcanaCabiBindingType::Unit, RuntimeValue::Unit) => {
            Ok(ArcanaCabiBindingValueV1::default())
        }
        (_, RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)))
            if binding.package_id == package_id && binding.type_name == expected_type =>
        {
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Opaque as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    opaque_value: binding.handle,
                },
            })
        }
        (ArcanaCabiBindingType::Named(layout_id), value) => {
            let bytes = runtime_binding_encode_layout_value(layouts, layout_id, &value)?;
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Layout as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    owned_bytes_value: into_owned_bytes(bytes),
                },
            })
        }
        (ArcanaCabiBindingType::View(view_type), value) => {
            let plan = plan.ok_or_else(|| {
                format!("native binding output `{expected_type}` requires runtime package context")
            })?;
            let state = state.ok_or_else(|| {
                format!("native binding output `{expected_type}` requires runtime state")
            })?;
            let host = host.ok_or_else(|| {
                format!("native binding output `{expected_type}` requires runtime core host")
            })?;
            let mut empty_scopes = Vec::new();
            let scopes = match scopes {
                Some(scopes) => scopes,
                None => &mut empty_scopes,
            };
            let empty_aliases = BTreeMap::new();
            let aliases = aliases.unwrap_or(&empty_aliases);
            let empty_type_bindings = BTreeMap::new();
            let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
            let snapshot = runtime_binding_snapshot_view_input(
                layouts,
                view_type,
                &value,
                scopes,
                plan,
                current_package_id.unwrap_or(package_id),
                current_module_id.unwrap_or(""),
                aliases,
                type_bindings,
                state,
                host,
                "native binding output view",
            )?;
            let owned = into_owned_bytes(snapshot.bytes);
            Ok(ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::View as u32,
                reserved0: 0,
                reserved1: 0,
                payload: ArcanaCabiBindingPayloadV1 {
                    view_value: raw_view(
                        owned.ptr.cast_const(),
                        snapshot.len,
                        snapshot.stride_bytes,
                        snapshot.family.cabi_tag(),
                        u32::try_from(snapshot.element_size).map_err(|_| {
                            format!(
                                "native binding output view element size `{}` does not fit u32",
                                snapshot.element_size
                            )
                        })?,
                        snapshot.flags,
                    ),
                },
            })
        }
        (ty, value) if ty.clone().scalar().is_some() => {
            runtime_binding_scalar_output_value(ty, value, expected_type)
        }
        _ => Err(format!(
            "native binding output expected `{expected_type}`, got `{}`",
            actual_type
        )),
    }
}

pub(super) fn runtime_binding_bytes_from_runtime_array(
    values: &[RuntimeValue],
    label: &str,
) -> Result<Vec<u8>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| match value {
            RuntimeValue::Int(value) => u8::try_from(*value)
                .map_err(|_| format!("{label} index `{index}` is out of range 0..255: `{value}`")),
            other => Err(format!(
                "{label} expected Int elements for byte-compatible Array, found `{other:?}` at index `{index}`"
            )),
        })
        .collect()
}

pub(super) fn runtime_binding_bytes_from_runtime_value(
    value: &RuntimeValue,
    label: &str,
) -> Result<Vec<u8>, String> {
    match value {
        RuntimeValue::Bytes(bytes) => Ok(bytes.clone()),
        RuntimeValue::Array(values) => runtime_binding_bytes_from_runtime_array(values, label),
        other => Err(format!("{label} expected Bytes, got `{other:?}`")),
    }
}

pub(super) fn runtime_binding_bytes_from_owned_runtime_array(
    values: Vec<RuntimeValue>,
    label: &str,
) -> Result<Vec<u8>, String> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| match value {
            RuntimeValue::Int(value) => u8::try_from(value)
                .map_err(|_| format!("{label} index `{index}` is out of range 0..255: `{value}`")),
            other => Err(format!(
                "{label} expected Int elements for byte-compatible Array, found `{other:?}` at index `{index}`"
            )),
        })
        .collect()
}

pub(super) fn runtime_binding_scalar_input_value(
    expected_type: &ArcanaCabiBindingType,
    value: &RuntimeValue,
    expected_text: &str,
) -> Result<ArcanaCabiBindingValueV1, String> {
    match expected_type {
        ArcanaCabiBindingType::I8 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::I8 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                i8_value: runtime_binding_expect_int_range(
                    value,
                    i8::MIN as i64,
                    i8::MAX as i64,
                    expected_text,
                )? as i8,
            },
        }),
        ArcanaCabiBindingType::U8 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::U8 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                u8_value: u8::try_from(runtime_binding_expect_int_range(
                    value,
                    0,
                    u8::MAX as i64,
                    expected_text,
                )?)
                .map_err(|_| format!("native binding argument expected `{expected_text}`"))?,
            },
        }),
        ArcanaCabiBindingType::I16 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::I16 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                i16_value: runtime_binding_expect_int_range(
                    value,
                    i16::MIN as i64,
                    i16::MAX as i64,
                    expected_text,
                )? as i16,
            },
        }),
        ArcanaCabiBindingType::U16 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::U16 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                u16_value: u16::try_from(runtime_binding_expect_int_range(
                    value,
                    0,
                    u16::MAX as i64,
                    expected_text,
                )?)
                .map_err(|_| format!("native binding argument expected `{expected_text}`"))?,
            },
        }),
        ArcanaCabiBindingType::I32 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::I32 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                i32_value: runtime_binding_expect_int_range(
                    value,
                    i32::MIN as i64,
                    i32::MAX as i64,
                    expected_text,
                )? as i32,
            },
        }),
        ArcanaCabiBindingType::U32 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::U32 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                u32_value: u32::try_from(runtime_binding_expect_int_range(
                    value,
                    0,
                    u32::MAX as i64,
                    expected_text,
                )?)
                .map_err(|_| format!("native binding argument expected `{expected_text}`"))?,
            },
        }),
        ArcanaCabiBindingType::I64 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::I64 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                i64_value: runtime_binding_expect_int_range(
                    value,
                    i64::MIN,
                    i64::MAX,
                    expected_text,
                )?,
            },
        }),
        ArcanaCabiBindingType::U64 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::U64 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                u64_value: u64::try_from(runtime_binding_expect_int_range(
                    value,
                    0,
                    i64::MAX,
                    expected_text,
                )?)
                .map_err(|_| format!("native binding argument expected `{expected_text}`"))?,
            },
        }),
        ArcanaCabiBindingType::ISize => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::ISize as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                isize_value: runtime_binding_expect_int_range(
                    value,
                    isize::MIN as i64,
                    isize::MAX as i64,
                    expected_text,
                )? as isize,
            },
        }),
        ArcanaCabiBindingType::USize => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::USize as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                usize_value: usize::try_from(runtime_binding_expect_int_range(
                    value,
                    0,
                    i64::MAX,
                    expected_text,
                )?)
                .map_err(|_| format!("native binding argument expected `{expected_text}`"))?,
            },
        }),
        ArcanaCabiBindingType::F32 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::F32 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                f32_value: runtime_binding_expect_float(value, ParsedFloatKind::F32, expected_text)?
                    as f32,
            },
        }),
        ArcanaCabiBindingType::F64 => Ok(ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::F64 as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 {
                f64_value: runtime_binding_expect_float(
                    value,
                    ParsedFloatKind::F64,
                    expected_text,
                )?,
            },
        }),
        _ => Err(format!(
            "native binding argument expected `{expected_text}`"
        )),
    }
}

pub(super) fn runtime_binding_scalar_output_value(
    expected_type: &ArcanaCabiBindingType,
    value: RuntimeValue,
    expected_text: &str,
) -> Result<ArcanaCabiBindingValueV1, String> {
    runtime_binding_scalar_input_value(expected_type, &value, expected_text)
}

pub(super) fn runtime_binding_runtime_value_from_scalar_tag(
    expected_type: &ArcanaCabiBindingType,
    actual_tag: ArcanaCabiBindingValueTag,
    value: &ArcanaCabiBindingValueV1,
) -> Result<RuntimeValue, String> {
    match (expected_type, actual_tag) {
        (ArcanaCabiBindingType::I8, ArcanaCabiBindingValueTag::I8) => {
            Ok(RuntimeValue::Int(i64::from(unsafe {
                value.payload.i8_value
            })))
        }
        (ArcanaCabiBindingType::U8, ArcanaCabiBindingValueTag::U8) => {
            Ok(RuntimeValue::Int(i64::from(unsafe {
                value.payload.u8_value
            })))
        }
        (ArcanaCabiBindingType::I16, ArcanaCabiBindingValueTag::I16) => {
            Ok(RuntimeValue::Int(i64::from(unsafe {
                value.payload.i16_value
            })))
        }
        (ArcanaCabiBindingType::U16, ArcanaCabiBindingValueTag::U16) => {
            Ok(RuntimeValue::Int(i64::from(unsafe {
                value.payload.u16_value
            })))
        }
        (ArcanaCabiBindingType::I32, ArcanaCabiBindingValueTag::I32) => {
            Ok(RuntimeValue::Int(i64::from(unsafe {
                value.payload.i32_value
            })))
        }
        (ArcanaCabiBindingType::U32, ArcanaCabiBindingValueTag::U32) => {
            Ok(RuntimeValue::Int(i64::from(unsafe {
                value.payload.u32_value
            })))
        }
        (ArcanaCabiBindingType::I64, ArcanaCabiBindingValueTag::I64) => {
            Ok(RuntimeValue::Int(unsafe { value.payload.i64_value }))
        }
        (ArcanaCabiBindingType::U64, ArcanaCabiBindingValueTag::U64) => {
            let raw = unsafe { value.payload.u64_value };
            let int = i64::try_from(raw).map_err(|_| {
                format!("native binding scalar `U64` value `{raw}` does not fit Arcana Int carrier")
            })?;
            Ok(RuntimeValue::Int(int))
        }
        (ArcanaCabiBindingType::ISize, ArcanaCabiBindingValueTag::ISize) => Ok(RuntimeValue::Int(
            unsafe { value.payload.isize_value } as i64,
        )),
        (ArcanaCabiBindingType::USize, ArcanaCabiBindingValueTag::USize) => {
            let raw = unsafe { value.payload.usize_value };
            let int = i64::try_from(raw).map_err(|_| {
                format!(
                    "native binding scalar `USize` value `{raw}` does not fit Arcana Int carrier"
                )
            })?;
            Ok(RuntimeValue::Int(int))
        }
        (ArcanaCabiBindingType::F32, ArcanaCabiBindingValueTag::F32) => Ok(make_runtime_float(
            ParsedFloatKind::F32,
            f64::from(unsafe { value.payload.f32_value }),
        )),
        (ArcanaCabiBindingType::F64, ArcanaCabiBindingValueTag::F64) => {
            Ok(make_runtime_float(ParsedFloatKind::F64, unsafe {
                value.payload.f64_value
            }))
        }
        _ => Err(format!(
            "native binding value expected `{}`, got tag `{actual_tag:?}`",
            expected_type.render()
        )),
    }
}

pub(super) fn runtime_binding_expect_int_range(
    value: &RuntimeValue,
    min: i64,
    max: i64,
    expected_text: &str,
) -> Result<i64, String> {
    let RuntimeValue::Int(value) = value else {
        return Err(format!(
            "native binding argument expected `{expected_text}`, got `{}`",
            runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
        ));
    };
    if *value < min || *value > max {
        return Err(format!(
            "native binding argument `{expected_text}` value `{value}` is out of range {min}..{max}"
        ));
    }
    Ok(*value)
}

pub(super) fn runtime_binding_expect_float(
    value: &RuntimeValue,
    kind: ParsedFloatKind,
    expected_text: &str,
) -> Result<f64, String> {
    let RuntimeValue::Float {
        text,
        kind: actual_kind,
    } = value
    else {
        return Err(format!(
            "native binding argument expected `{expected_text}`, got `{}`",
            runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
        ));
    };
    parse_runtime_float_text(text, *actual_kind).map(|value| match kind {
        ParsedFloatKind::F32 => f64::from(value as f32),
        ParsedFloatKind::F64 => value,
    })
}

pub(super) fn runtime_binding_input_bytes_view<'a>(
    ptr: *const u8,
    len: usize,
    label: &str,
) -> Result<&'a [u8], String> {
    if ptr.is_null() {
        if len == 0 {
            return Ok(&[]);
        }
        return Err(format!("{label} returned null data with len {len}"));
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

pub(super) fn runtime_utf16_units_to_binding_bytes(units: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(units.len().saturating_mul(2));
    for unit in units {
        bytes.extend_from_slice(&unit.to_ne_bytes());
    }
    bytes
}

pub(super) fn runtime_utf16_units_from_binding_bytes(
    bytes: &[u8],
    label: &str,
) -> Result<Vec<u16>, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err(format!(
            "{label} length {} is not a multiple of 2 for Utf16 transport",
            bytes.len()
        ));
    }
    Ok(bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_ne_bytes([chunk[0], chunk[1]]))
        .collect())
}

pub(super) fn runtime_binding_layout_by_id<'a>(
    layouts: &'a [ArcanaCabiBindingLayout],
    layout_id: &str,
) -> Result<&'a ArcanaCabiBindingLayout, String> {
    layouts
        .iter()
        .find(|layout| layout.layout_id == layout_id)
        .ok_or_else(|| format!("binding layout `{layout_id}` is not present in runtime plan"))
}

pub(super) fn runtime_binding_encode_layout_value(
    layouts: &[ArcanaCabiBindingLayout],
    layout_id: &str,
    value: &RuntimeValue,
) -> Result<Vec<u8>, String> {
    let layout = runtime_binding_layout_by_id(layouts, layout_id)?;
    let mut buffer = vec![0u8; layout.size];
    runtime_binding_encode_layout_into(layouts, layout, value, &mut buffer)?;
    Ok(buffer)
}

pub(super) fn runtime_binding_decode_layout_value(
    layouts: &[ArcanaCabiBindingLayout],
    layout_id: &str,
    bytes: &[u8],
) -> Result<RuntimeValue, String> {
    let layout = runtime_binding_layout_by_id(layouts, layout_id)?;
    if bytes.len() != layout.size {
        return Err(format!(
            "binding layout `{layout_id}` size mismatch: expected {}, got {}",
            layout.size,
            bytes.len()
        ));
    }
    runtime_binding_decode_layout_from(layouts, layout, bytes)
}

pub(super) fn runtime_binding_encode_layout_into(
    layouts: &[ArcanaCabiBindingLayout],
    layout: &ArcanaCabiBindingLayout,
    value: &RuntimeValue,
    buffer: &mut [u8],
) -> Result<(), String> {
    if buffer.len() != layout.size {
        return Err(format!(
            "binding layout `{}` output buffer size mismatch: expected {}, got {}",
            layout.layout_id,
            layout.size,
            buffer.len()
        ));
    }
    match &layout.kind {
        ArcanaCabiBindingLayoutKind::Alias { target } => {
            let bytes = runtime_binding_encode_raw_type(layouts, target, value, &layout.layout_id)?;
            if bytes.len() != layout.size {
                return Err(format!(
                    "binding layout `{}` alias size mismatch: expected {}, got {}",
                    layout.layout_id,
                    layout.size,
                    bytes.len()
                ));
            }
            buffer.copy_from_slice(&bytes);
            Ok(())
        }
        ArcanaCabiBindingLayoutKind::Struct { fields } => {
            let RuntimeValue::Record {
                name,
                fields: record_fields,
            } = value
            else {
                return Err(format!(
                    "binding layout `{}` expected struct record value, got `{}`",
                    layout.layout_id,
                    runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
                ));
            };
            if name != &layout.layout_id {
                return Err(format!(
                    "binding layout `{}` expected record `{}`, got `{name}`",
                    layout.layout_id, layout.layout_id
                ));
            }
            for field in fields {
                let field_value = record_fields.get(&field.name).ok_or_else(|| {
                    format!(
                        "binding layout `{}` is missing field `{}`",
                        layout.layout_id, field.name
                    )
                })?;
                if let Some(bit_width) = field.bit_width {
                    runtime_binding_encode_bitfield(layout, field, bit_width, field_value, buffer)?;
                } else {
                    let field_size = runtime_binding_raw_type_size(layouts, &field.ty)?;
                    let end = field.offset + field_size;
                    let bytes = runtime_binding_encode_raw_type(
                        layouts,
                        &field.ty,
                        field_value,
                        &format!("{}::{}", layout.layout_id, field.name),
                    )?;
                    buffer[field.offset..end].copy_from_slice(&bytes);
                }
            }
            Ok(())
        }
        ArcanaCabiBindingLayoutKind::Union { fields } => {
            let RuntimeValue::Record {
                name,
                fields: record_fields,
            } = value
            else {
                return Err(format!(
                    "binding layout `{}` expected union record value, got `{}`",
                    layout.layout_id,
                    runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
                ));
            };
            if name != &layout.layout_id {
                return Err(format!(
                    "binding layout `{}` expected record `{}`, got `{name}`",
                    layout.layout_id, layout.layout_id
                ));
            }
            if record_fields.len() != 1 {
                return Err(format!(
                    "binding union `{}` must initialize exactly one field, got {}",
                    layout.layout_id,
                    record_fields.len()
                ));
            }
            let (field_name, field_value) = record_fields.iter().next().ok_or_else(|| {
                format!(
                    "binding union `{}` must initialize at least one field",
                    layout.layout_id
                )
            })?;
            let field = fields
                .iter()
                .find(|field| field.name == *field_name)
                .ok_or_else(|| {
                    format!(
                        "binding union `{}` has no field `{field_name}`",
                        layout.layout_id
                    )
                })?;
            let field_size = runtime_binding_raw_type_size(layouts, &field.ty)?;
            let bytes = runtime_binding_encode_raw_type(
                layouts,
                &field.ty,
                field_value,
                &format!("{}::{}", layout.layout_id, field.name),
            )?;
            buffer[field.offset..field.offset + field_size].copy_from_slice(&bytes);
            Ok(())
        }
        ArcanaCabiBindingLayoutKind::Array { element_type, len } => {
            let RuntimeValue::Array(values) = value else {
                return Err(format!(
                    "binding layout `{}` expected array value, got `{}`",
                    layout.layout_id,
                    runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
                ));
            };
            if values.len() != *len {
                return Err(format!(
                    "binding array `{}` expected {len} elements, got {}",
                    layout.layout_id,
                    values.len()
                ));
            }
            let element_size = runtime_binding_raw_type_size(layouts, element_type)?;
            for (index, item) in values.iter().enumerate() {
                let bytes = runtime_binding_encode_raw_type(
                    layouts,
                    element_type,
                    item,
                    &format!("{}[{index}]", layout.layout_id),
                )?;
                let start = index * element_size;
                buffer[start..start + element_size].copy_from_slice(&bytes);
            }
            Ok(())
        }
        ArcanaCabiBindingLayoutKind::Enum { repr, .. }
        | ArcanaCabiBindingLayoutKind::Flags { repr } => {
            let bytes = runtime_binding_encode_scalar_bytes(*repr, value, &layout.layout_id)?;
            buffer.copy_from_slice(&bytes);
            Ok(())
        }
        ArcanaCabiBindingLayoutKind::Callback { .. }
        | ArcanaCabiBindingLayoutKind::Interface { .. } => {
            let raw = runtime_binding_pointer_value(value, &layout.layout_id)?;
            let bytes = runtime_binding_encode_pointer_bytes(layout.size, raw, &layout.layout_id)?;
            buffer.copy_from_slice(&bytes);
            Ok(())
        }
    }
}

pub(super) fn runtime_binding_decode_layout_from(
    layouts: &[ArcanaCabiBindingLayout],
    layout: &ArcanaCabiBindingLayout,
    bytes: &[u8],
) -> Result<RuntimeValue, String> {
    match &layout.kind {
        ArcanaCabiBindingLayoutKind::Alias { target } => {
            runtime_binding_decode_raw_type(layouts, target, bytes, &layout.layout_id)
        }
        ArcanaCabiBindingLayoutKind::Struct { fields } => {
            let mut values = BTreeMap::new();
            for field in fields {
                let value = if let Some(bit_width) = field.bit_width {
                    runtime_binding_decode_bitfield(layout, field, bit_width, bytes)?
                } else {
                    let field_size = runtime_binding_raw_type_size(layouts, &field.ty)?;
                    runtime_binding_decode_raw_type(
                        layouts,
                        &field.ty,
                        &bytes[field.offset..field.offset + field_size],
                        &format!("{}::{}", layout.layout_id, field.name),
                    )?
                };
                values.insert(field.name.clone(), value);
            }
            Ok(RuntimeValue::Record {
                name: layout.layout_id.clone(),
                fields: values,
            })
        }
        ArcanaCabiBindingLayoutKind::Union { fields } => {
            let mut values = BTreeMap::new();
            for field in fields {
                let field_size = runtime_binding_raw_type_size(layouts, &field.ty)?;
                let value = runtime_binding_decode_raw_type(
                    layouts,
                    &field.ty,
                    &bytes[field.offset..field.offset + field_size],
                    &format!("{}::{}", layout.layout_id, field.name),
                )?;
                values.insert(field.name.clone(), value);
            }
            Ok(RuntimeValue::Record {
                name: layout.layout_id.clone(),
                fields: values,
            })
        }
        ArcanaCabiBindingLayoutKind::Array { element_type, len } => {
            let element_size = runtime_binding_raw_type_size(layouts, element_type)?;
            let mut values = Vec::with_capacity(*len);
            for index in 0..*len {
                let start = index * element_size;
                values.push(runtime_binding_decode_raw_type(
                    layouts,
                    element_type,
                    &bytes[start..start + element_size],
                    &format!("{}[{index}]", layout.layout_id),
                )?);
            }
            Ok(RuntimeValue::Array(values))
        }
        ArcanaCabiBindingLayoutKind::Enum { repr, .. }
        | ArcanaCabiBindingLayoutKind::Flags { repr } => {
            runtime_binding_decode_scalar_bytes(*repr, bytes, &layout.layout_id)
        }
        ArcanaCabiBindingLayoutKind::Callback { .. }
        | ArcanaCabiBindingLayoutKind::Interface { .. } => Ok(RuntimeValue::Int(
            runtime_binding_decode_pointer_bytes(bytes, &layout.layout_id)?,
        )),
    }
}

pub(super) fn runtime_binding_encode_raw_type(
    layouts: &[ArcanaCabiBindingLayout],
    ty: &ArcanaCabiBindingRawType,
    value: &RuntimeValue,
    context: &str,
) -> Result<Vec<u8>, String> {
    match ty {
        ArcanaCabiBindingRawType::Void => Ok(Vec::new()),
        ArcanaCabiBindingRawType::Scalar(scalar) => {
            runtime_binding_encode_scalar_bytes(*scalar, value, context)
        }
        ArcanaCabiBindingRawType::Named(layout_id) => {
            runtime_binding_encode_layout_value(layouts, layout_id, value)
        }
        ArcanaCabiBindingRawType::Pointer { .. }
        | ArcanaCabiBindingRawType::FunctionPointer { .. } => {
            let raw = runtime_binding_pointer_value(value, context)?;
            runtime_binding_encode_pointer_bytes(
                runtime_binding_raw_type_size(layouts, ty)?,
                raw,
                context,
            )
        }
    }
}

pub(super) fn runtime_binding_decode_raw_type(
    layouts: &[ArcanaCabiBindingLayout],
    ty: &ArcanaCabiBindingRawType,
    bytes: &[u8],
    context: &str,
) -> Result<RuntimeValue, String> {
    match ty {
        ArcanaCabiBindingRawType::Void => Ok(RuntimeValue::Unit),
        ArcanaCabiBindingRawType::Scalar(scalar) => {
            runtime_binding_decode_scalar_bytes(*scalar, bytes, context)
        }
        ArcanaCabiBindingRawType::Named(layout_id) => {
            runtime_binding_decode_layout_value(layouts, layout_id, bytes)
        }
        ArcanaCabiBindingRawType::Pointer { .. }
        | ArcanaCabiBindingRawType::FunctionPointer { .. } => Ok(RuntimeValue::Int(
            runtime_binding_decode_pointer_bytes(bytes, context)?,
        )),
    }
}

pub(super) fn runtime_binding_raw_type_size(
    layouts: &[ArcanaCabiBindingLayout],
    ty: &ArcanaCabiBindingRawType,
) -> Result<usize, String> {
    Ok(match ty {
        ArcanaCabiBindingRawType::Void => 0,
        ArcanaCabiBindingRawType::Scalar(scalar) => scalar.size_bytes(),
        ArcanaCabiBindingRawType::Named(layout_id) => {
            runtime_binding_layout_by_id(layouts, layout_id)?.size
        }
        ArcanaCabiBindingRawType::Pointer { .. }
        | ArcanaCabiBindingRawType::FunctionPointer { .. } => std::mem::size_of::<usize>(),
    })
}

pub(super) fn runtime_binding_encode_scalar_bytes(
    scalar: ArcanaCabiBindingScalarType,
    value: &RuntimeValue,
    context: &str,
) -> Result<Vec<u8>, String> {
    match scalar {
        ArcanaCabiBindingScalarType::Bool => match value {
            RuntimeValue::Bool(value) => Ok(vec![u8::from(*value)]),
            _ => Err(format!(
                "{context} expected Bool, got `{}`",
                runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
            )),
        },
        ArcanaCabiBindingScalarType::Int | ArcanaCabiBindingScalarType::I64 => Ok(
            runtime_binding_expect_int_range(value, i64::MIN, i64::MAX, context)?
                .to_ne_bytes()
                .to_vec(),
        ),
        ArcanaCabiBindingScalarType::I8 => {
            Ok(
                (runtime_binding_expect_int_range(value, i8::MIN as i64, i8::MAX as i64, context)?
                    as i8)
                    .to_ne_bytes()
                    .to_vec(),
            )
        }
        ArcanaCabiBindingScalarType::U8 => Ok([u8::try_from(runtime_binding_expect_int_range(
            value,
            0,
            u8::MAX as i64,
            context,
        )?)
        .map_err(|_| format!("{context} expected U8"))?]
        .to_vec()),
        ArcanaCabiBindingScalarType::I16 => Ok((runtime_binding_expect_int_range(
            value,
            i16::MIN as i64,
            i16::MAX as i64,
            context,
        )? as i16)
            .to_ne_bytes()
            .to_vec()),
        ArcanaCabiBindingScalarType::U16 => Ok((u16::try_from(runtime_binding_expect_int_range(
            value,
            0,
            u16::MAX as i64,
            context,
        )?)
        .map_err(|_| format!("{context} expected U16"))?)
        .to_ne_bytes()
        .to_vec()),
        ArcanaCabiBindingScalarType::I32 => Ok((runtime_binding_expect_int_range(
            value,
            i32::MIN as i64,
            i32::MAX as i64,
            context,
        )? as i32)
            .to_ne_bytes()
            .to_vec()),
        ArcanaCabiBindingScalarType::U32 => Ok((u32::try_from(runtime_binding_expect_int_range(
            value,
            0,
            u32::MAX as i64,
            context,
        )?)
        .map_err(|_| format!("{context} expected U32"))?)
        .to_ne_bytes()
        .to_vec()),
        ArcanaCabiBindingScalarType::U64 => Ok((u64::try_from(runtime_binding_expect_int_range(
            value,
            0,
            i64::MAX,
            context,
        )?)
        .map_err(|_| format!("{context} expected U64"))?)
        .to_ne_bytes()
        .to_vec()),
        ArcanaCabiBindingScalarType::ISize => Ok((runtime_binding_expect_int_range(
            value,
            isize::MIN as i64,
            isize::MAX as i64,
            context,
        )? as isize)
            .to_ne_bytes()
            .to_vec()),
        ArcanaCabiBindingScalarType::USize => Ok((usize::try_from(
            runtime_binding_expect_int_range(value, 0, i64::MAX, context)?,
        )
        .map_err(|_| format!("{context} expected USize"))?)
        .to_ne_bytes()
        .to_vec()),
        ArcanaCabiBindingScalarType::F32 => {
            Ok(
                (runtime_binding_expect_float(value, ParsedFloatKind::F32, context)? as f32)
                    .to_ne_bytes()
                    .to_vec(),
            )
        }
        ArcanaCabiBindingScalarType::F64 => {
            Ok(
                runtime_binding_expect_float(value, ParsedFloatKind::F64, context)?
                    .to_ne_bytes()
                    .to_vec(),
            )
        }
    }
}

pub(super) fn runtime_binding_decode_scalar_bytes(
    scalar: ArcanaCabiBindingScalarType,
    bytes: &[u8],
    context: &str,
) -> Result<RuntimeValue, String> {
    if bytes.len() != scalar.size_bytes() {
        return Err(format!(
            "{context} expected {} bytes for {}, got {}",
            scalar.size_bytes(),
            scalar.render(),
            bytes.len()
        ));
    }
    Ok(match scalar {
        ArcanaCabiBindingScalarType::Bool => RuntimeValue::Bool(bytes[0] != 0),
        ArcanaCabiBindingScalarType::Int | ArcanaCabiBindingScalarType::I64 => {
            RuntimeValue::Int(i64::from_ne_bytes(bytes.try_into().expect("checked len")))
        }
        ArcanaCabiBindingScalarType::I8 => RuntimeValue::Int(i64::from(i8::from_ne_bytes(
            bytes.try_into().expect("checked len"),
        ))),
        ArcanaCabiBindingScalarType::U8 => RuntimeValue::Int(i64::from(u8::from_ne_bytes(
            bytes.try_into().expect("checked len"),
        ))),
        ArcanaCabiBindingScalarType::I16 => RuntimeValue::Int(i64::from(i16::from_ne_bytes(
            bytes.try_into().expect("checked len"),
        ))),
        ArcanaCabiBindingScalarType::U16 => RuntimeValue::Int(i64::from(u16::from_ne_bytes(
            bytes.try_into().expect("checked len"),
        ))),
        ArcanaCabiBindingScalarType::I32 => RuntimeValue::Int(i64::from(i32::from_ne_bytes(
            bytes.try_into().expect("checked len"),
        ))),
        ArcanaCabiBindingScalarType::U32 => RuntimeValue::Int(i64::from(u32::from_ne_bytes(
            bytes.try_into().expect("checked len"),
        ))),
        ArcanaCabiBindingScalarType::U64 => {
            let raw = u64::from_ne_bytes(bytes.try_into().expect("checked len"));
            RuntimeValue::Int(i64::try_from(raw).map_err(|_| {
                format!("{context} U64 value `{raw}` does not fit Arcana Int carrier")
            })?)
        }
        ArcanaCabiBindingScalarType::ISize => {
            RuntimeValue::Int(isize::from_ne_bytes(bytes.try_into().expect("checked len")) as i64)
        }
        ArcanaCabiBindingScalarType::USize => {
            let raw = usize::from_ne_bytes(bytes.try_into().expect("checked len"));
            RuntimeValue::Int(i64::try_from(raw).map_err(|_| {
                format!("{context} USize value `{raw}` does not fit Arcana Int carrier")
            })?)
        }
        ArcanaCabiBindingScalarType::F32 => make_runtime_float(
            ParsedFloatKind::F32,
            f64::from(f32::from_ne_bytes(bytes.try_into().expect("checked len"))),
        ),
        ArcanaCabiBindingScalarType::F64 => make_runtime_float(
            ParsedFloatKind::F64,
            f64::from_ne_bytes(bytes.try_into().expect("checked len")),
        ),
    })
}

pub(super) fn runtime_binding_encode_bitfield(
    layout: &ArcanaCabiBindingLayout,
    field: &arcana_cabi::ArcanaCabiBindingLayoutField,
    bit_width: u16,
    value: &RuntimeValue,
    buffer: &mut [u8],
) -> Result<(), String> {
    let ArcanaCabiBindingRawType::Scalar(scalar) = &field.ty else {
        return Err(format!(
            "binding layout `{}` bitfield `{}` must use a scalar base type",
            layout.layout_id, field.name
        ));
    };
    let storage_size = scalar.size_bytes();
    let storage = &buffer[field.offset..field.offset + storage_size];
    let mut current = runtime_binding_decode_integer_storage(*scalar, storage)?;
    let raw_value = runtime_binding_integer_from_value(value, &field.name)?;
    let bit_offset = usize::from(field.bit_offset.unwrap_or(0));
    let mask = if bit_width == 64 {
        u64::MAX
    } else {
        ((1u128 << bit_width) - 1) as u64
    };
    current &= !(mask << bit_offset);
    current |= (raw_value & mask) << bit_offset;
    let encoded = runtime_binding_encode_integer_storage(*scalar, current)?;
    buffer[field.offset..field.offset + storage_size].copy_from_slice(&encoded);
    Ok(())
}

pub(super) fn runtime_binding_decode_bitfield(
    layout: &ArcanaCabiBindingLayout,
    field: &arcana_cabi::ArcanaCabiBindingLayoutField,
    bit_width: u16,
    bytes: &[u8],
) -> Result<RuntimeValue, String> {
    let ArcanaCabiBindingRawType::Scalar(scalar) = &field.ty else {
        return Err(format!(
            "binding layout `{}` bitfield `{}` must use a scalar base type",
            layout.layout_id, field.name
        ));
    };
    let storage = &bytes[field.offset..field.offset + scalar.size_bytes()];
    let raw = runtime_binding_decode_integer_storage(*scalar, storage)?;
    let bit_offset = usize::from(field.bit_offset.unwrap_or(0));
    let mask = if bit_width == 64 {
        u64::MAX
    } else {
        ((1u128 << bit_width) - 1) as u64
    };
    let value = (raw >> bit_offset) & mask;
    if runtime_binding_scalar_is_signed(*scalar) {
        let shift = 64usize.saturating_sub(bit_width as usize);
        Ok(RuntimeValue::Int(((value << shift) as i64) >> shift))
    } else {
        Ok(RuntimeValue::Int(i64::try_from(value).map_err(|_| {
            format!(
                "binding layout `{}` bitfield `{}` value `{value}` does not fit Arcana Int carrier",
                layout.layout_id, field.name
            )
        })?))
    }
}

pub(super) fn runtime_binding_integer_from_value(
    value: &RuntimeValue,
    context: &str,
) -> Result<u64, String> {
    let RuntimeValue::Int(value) = value else {
        return Err(format!(
            "{context} expected Int-compatible bitfield value, got `{}`",
            runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
        ));
    };
    Ok(*value as u64)
}

pub(super) fn runtime_binding_encode_integer_storage(
    scalar: ArcanaCabiBindingScalarType,
    value: u64,
) -> Result<Vec<u8>, String> {
    Ok(match scalar {
        ArcanaCabiBindingScalarType::I8
        | ArcanaCabiBindingScalarType::U8
        | ArcanaCabiBindingScalarType::Bool => vec![value as u8],
        ArcanaCabiBindingScalarType::I16 | ArcanaCabiBindingScalarType::U16 => {
            (value as u16).to_ne_bytes().to_vec()
        }
        ArcanaCabiBindingScalarType::I32 | ArcanaCabiBindingScalarType::U32 => {
            (value as u32).to_ne_bytes().to_vec()
        }
        ArcanaCabiBindingScalarType::Int
        | ArcanaCabiBindingScalarType::I64
        | ArcanaCabiBindingScalarType::U64 => value.to_ne_bytes().to_vec(),
        ArcanaCabiBindingScalarType::ISize | ArcanaCabiBindingScalarType::USize => {
            (value as usize).to_ne_bytes().to_vec()
        }
        ArcanaCabiBindingScalarType::F32 | ArcanaCabiBindingScalarType::F64 => {
            return Err("floating-point scalars cannot back bitfield storage".to_string());
        }
    })
}

pub(super) fn runtime_binding_decode_integer_storage(
    scalar: ArcanaCabiBindingScalarType,
    bytes: &[u8],
) -> Result<u64, String> {
    Ok(match scalar {
        ArcanaCabiBindingScalarType::I8
        | ArcanaCabiBindingScalarType::U8
        | ArcanaCabiBindingScalarType::Bool => u64::from(bytes[0]),
        ArcanaCabiBindingScalarType::I16 | ArcanaCabiBindingScalarType::U16 => {
            u64::from(u16::from_ne_bytes(bytes.try_into().expect("checked len")))
        }
        ArcanaCabiBindingScalarType::I32 | ArcanaCabiBindingScalarType::U32 => {
            u64::from(u32::from_ne_bytes(bytes.try_into().expect("checked len")))
        }
        ArcanaCabiBindingScalarType::Int
        | ArcanaCabiBindingScalarType::I64
        | ArcanaCabiBindingScalarType::U64 => {
            u64::from_ne_bytes(bytes.try_into().expect("checked len"))
        }
        ArcanaCabiBindingScalarType::ISize | ArcanaCabiBindingScalarType::USize => {
            usize::from_ne_bytes(bytes.try_into().expect("checked len")) as u64
        }
        ArcanaCabiBindingScalarType::F32 | ArcanaCabiBindingScalarType::F64 => {
            return Err("floating-point scalars cannot back bitfield storage".to_string());
        }
    })
}

pub(super) fn runtime_binding_scalar_is_signed(scalar: ArcanaCabiBindingScalarType) -> bool {
    matches!(
        scalar,
        ArcanaCabiBindingScalarType::Int
            | ArcanaCabiBindingScalarType::I8
            | ArcanaCabiBindingScalarType::I16
            | ArcanaCabiBindingScalarType::I32
            | ArcanaCabiBindingScalarType::I64
            | ArcanaCabiBindingScalarType::ISize
    )
}

pub(super) fn runtime_binding_pointer_value(
    value: &RuntimeValue,
    context: &str,
) -> Result<u64, String> {
    match value {
        RuntimeValue::Int(value) if *value >= 0 => Ok(*value as u64),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) => Ok(binding.handle),
        _ => Err(format!(
            "{context} expected pointer-compatible Int or binding opaque, got `{}`",
            runtime_value_type_root(value).unwrap_or_else(|| format!("{value:?}"))
        )),
    }
}

pub(super) fn runtime_binding_encode_pointer_bytes(
    size: usize,
    value: u64,
    context: &str,
) -> Result<Vec<u8>, String> {
    match size {
        4 => Ok((u32::try_from(value).map_err(|_| {
            format!("{context} pointer value `{value}` does not fit 32-bit pointer")
        })?)
        .to_ne_bytes()
        .to_vec()),
        8 => Ok(value.to_ne_bytes().to_vec()),
        other => Err(format!("{context} uses unsupported pointer size `{other}`")),
    }
}

pub(super) fn runtime_binding_decode_pointer_bytes(
    bytes: &[u8],
    context: &str,
) -> Result<i64, String> {
    match bytes.len() {
        4 => Ok(i64::from(u32::from_ne_bytes(
            bytes.try_into().expect("checked len"),
        ))),
        8 => {
            let raw = u64::from_ne_bytes(bytes.try_into().expect("checked len"));
            i64::try_from(raw).map_err(|_| {
                format!("{context} pointer value `{raw}` does not fit Arcana Int carrier")
            })
        }
        other => Err(format!("{context} uses unsupported pointer size `{other}`")),
    }
}

pub(super) fn runtime_release_binding_values(
    values: impl IntoIterator<Item = ArcanaCabiBindingValueV1>,
) {
    for value in values {
        let _ = release_binding_output_value(
            value,
            runtime_binding_owned_bytes_free,
            runtime_binding_owned_str_free,
        );
    }
}
