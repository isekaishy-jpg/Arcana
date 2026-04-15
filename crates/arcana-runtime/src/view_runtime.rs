use super::*;

pub(super) fn runtime_byte_view_values(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    view: &RuntimeByteViewState,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<Vec<u8>, String> {
    match &view.backing {
        RuntimeByteViewBacking::Buffer(buffer) => {
            let values = &state
                .byte_view_buffers
                .get(buffer)
                .ok_or_else(|| format!("invalid byte view buffer `{}`", buffer.0))?
                .values;
            let (start, end) = runtime_view_range(view.start, view.len, values.len(), context)?;
            Ok(values[start..end].to_vec())
        }
        RuntimeByteViewBacking::Reference(reference) => {
            let values = runtime_reference_byte_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                reference,
                host,
                context,
            )?;
            let (start, end) = runtime_view_range(view.start, view.len, values.len(), context)?;
            Ok(values[start..end].to_vec())
        }
        RuntimeByteViewBacking::Foreign(backing) => {
            let backing_len = runtime_foreign_byte_len(plan, host, *backing, context)?;
            let (start, end) = runtime_view_range(view.start, view.len, backing_len, context)?;
            let mut bytes = Vec::with_capacity(end - start);
            let mut index = start;
            while index < end {
                bytes.push(runtime_foreign_byte_at(
                    plan, host, *backing, index, context,
                )?);
                index += 1;
            }
            Ok(bytes)
        }
    }
}

pub(super) fn runtime_byte_edit_view_values(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    view: &RuntimeByteEditViewState,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<Vec<u8>, String> {
    match &view.backing {
        RuntimeByteViewBacking::Buffer(buffer) => {
            let values = &state
                .byte_view_buffers
                .get(buffer)
                .ok_or_else(|| format!("invalid byte view buffer `{}`", buffer.0))?
                .values;
            let (start, end) = runtime_view_range(view.start, view.len, values.len(), context)?;
            Ok(values[start..end].to_vec())
        }
        RuntimeByteViewBacking::Reference(reference) => {
            let values = runtime_reference_byte_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                reference,
                host,
                context,
            )?;
            let (start, end) = runtime_view_range(view.start, view.len, values.len(), context)?;
            Ok(values[start..end].to_vec())
        }
        RuntimeByteViewBacking::Foreign(backing) => {
            let backing_len = runtime_foreign_byte_len(plan, host, *backing, context)?;
            let (start, end) = runtime_view_range(view.start, view.len, backing_len, context)?;
            let mut bytes = Vec::with_capacity(end - start);
            let mut index = start;
            while index < end {
                bytes.push(runtime_foreign_byte_at(
                    plan, host, *backing, index, context,
                )?);
                index += 1;
            }
            Ok(bytes)
        }
    }
}

pub(super) fn runtime_str_view_text(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    view: &RuntimeStrViewState,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<String, String> {
    match &view.backing {
        RuntimeStrViewBacking::Buffer(buffer) => {
            let text = &state
                .str_view_buffers
                .get(buffer)
                .ok_or_else(|| format!("invalid str view buffer `{}`", buffer.0))?
                .text;
            let (start, end) = runtime_view_range(view.start, view.len, text.len(), context)?;
            runtime_string_slice(text, start, end, context)
        }
        RuntimeStrViewBacking::Reference(reference) => {
            let text = runtime_reference_text_value(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                reference,
                host,
                context,
            )?;
            let (start, end) = runtime_view_range(view.start, view.len, text.len(), context)?;
            runtime_string_slice(&text, start, end, context)
        }
    }
}

pub(super) fn eval_runtime_index_value(
    base: RuntimeValue,
    index: RuntimeValue,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let base = read_runtime_value_if_ref(
        base,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let index = expect_int(index, "index")?;
    match base {
        RuntimeValue::List(values) => {
            let index = runtime_index_to_usize(index, values.len(), "list index")?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| format!("list index `{index}` is out of bounds"))
        }
        RuntimeValue::Array(values) => {
            let index = runtime_index_to_usize(index, values.len(), "array index")?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| format!("array index `{index}` is out of bounds"))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
            let view = state
                .read_views
                .get(&handle)
                .cloned()
                .ok_or_else(|| format!("invalid ReadView handle `{}`", handle.0))?;
            let values = runtime_read_view_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &view,
                host,
                "view index",
            )?;
            let index = runtime_index_to_usize(index, values.len(), "view index")?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| format!("view index `{index}` is out of bounds"))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
            let view = state
                .edit_views
                .get(&handle)
                .cloned()
                .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?;
            let read_view = RuntimeReadViewState {
                type_args: view.type_args,
                backing: view.backing,
                start: view.start,
                len: view.len,
            };
            let values = runtime_read_view_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &read_view,
                host,
                "view index",
            )?;
            let index = runtime_index_to_usize(index, values.len(), "view index")?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| format!("view index `{index}` is out of bounds"))
        }
        other => Err(format!(
            "runtime index expects List, Array, or View, got `{other:?}`"
        )),
    }
}

pub(super) fn eval_runtime_slice_value(
    base: RuntimeValue,
    start: Option<RuntimeValue>,
    end: Option<RuntimeValue>,
    inclusive_end: bool,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let base = read_runtime_value_if_ref(
        base,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let start = start
        .map(|value| expect_int(value, "slice start"))
        .transpose()?;
    let end = end
        .map(|value| expect_int(value, "slice end"))
        .transpose()?;
    match base {
        RuntimeValue::Array(values) => {
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, values.len(), "slice")?;
            let type_args = runtime_array_projection_type_args(&values[start..end], state);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(
                insert_runtime_read_view(state, &type_args, values[start..end].to_vec()),
            )))
        }
        RuntimeValue::Bytes(values) | RuntimeValue::ByteBuffer(values) => {
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, values.len(), "slice")?;
            let backing = insert_runtime_byte_view_buffer(state, values);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(
                insert_runtime_byte_view_from_buffer(state, backing, start, end - start),
            )))
        }
        RuntimeValue::Utf16(units) | RuntimeValue::Utf16Buffer(units) => {
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, units.len(), "slice")?;
            let values = units[start..end]
                .iter()
                .copied()
                .map(|value| RuntimeValue::Int(i64::from(value)))
                .collect::<Vec<_>>();
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(
                insert_runtime_read_view(state, &["U16".to_string()], values),
            )))
        }
        RuntimeValue::Str(text) => {
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, text.len(), "slice")?;
            let backing = insert_runtime_str_view_buffer(state, text);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(
                insert_runtime_str_view_from_buffer(state, backing, start, end - start),
            )))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
            let (type_args, backing, view_start, view_len) = {
                let view = state
                    .read_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid View handle `{}`", handle.0))?;
                (
                    view.type_args.clone(),
                    view.backing.clone(),
                    view.start,
                    view.len,
                )
            };
            let (start, end) = runtime_slice_window(start, end, inclusive_end, view_len, "slice")?;
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(
                insert_runtime_read_view_from_backing(
                    state,
                    &type_args,
                    backing,
                    view_start + start,
                    end - start,
                ),
            )))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
            let (type_args, backing, view_start, view_len) = {
                let view = state
                    .edit_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid View handle `{}`", handle.0))?;
                (
                    view.type_args.clone(),
                    view.backing.clone(),
                    view.start,
                    view.len,
                )
            };
            let (start, end) = runtime_slice_window(start, end, inclusive_end, view_len, "slice")?;
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(
                insert_runtime_read_view_from_backing(
                    state,
                    &type_args,
                    backing,
                    view_start + start,
                    end - start,
                ),
            )))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
            let view = state
                .byte_views
                .get(&handle)
                .ok_or_else(|| format!("invalid View handle `{}`", handle.0))?
                .clone();
            let (start, end) = runtime_slice_window(start, end, inclusive_end, view.len, "slice")?;
            let next = match view.backing {
                RuntimeByteViewBacking::Buffer(buffer) => insert_runtime_byte_view_from_buffer(
                    state,
                    buffer,
                    view.start + start,
                    end - start,
                ),
                RuntimeByteViewBacking::Reference(reference) => {
                    insert_runtime_byte_view_from_reference(
                        state,
                        reference,
                        view.start + start,
                        end - start,
                    )
                }
                RuntimeByteViewBacking::Foreign(backing) => insert_runtime_byte_view_from_foreign(
                    state,
                    backing.package_id,
                    backing.handle,
                    view.start + start,
                    end - start,
                ),
            };
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(next)))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid View handle `{}`", handle.0))?
                .clone();
            let (start, end) = runtime_slice_window(start, end, inclusive_end, view.len, "slice")?;
            let next = match view.backing {
                RuntimeByteViewBacking::Buffer(buffer) => insert_runtime_byte_view_from_buffer(
                    state,
                    buffer,
                    view.start + start,
                    end - start,
                ),
                RuntimeByteViewBacking::Reference(reference) => {
                    insert_runtime_byte_view_from_reference(
                        state,
                        reference,
                        view.start + start,
                        end - start,
                    )
                }
                RuntimeByteViewBacking::Foreign(backing) => insert_runtime_byte_view_from_foreign(
                    state,
                    backing.package_id,
                    backing.handle,
                    view.start + start,
                    end - start,
                ),
            };
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(next)))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
            let view = state
                .str_views
                .get(&handle)
                .ok_or_else(|| format!("invalid View handle `{}`", handle.0))?
                .clone();
            let (start, end) = runtime_slice_window(start, end, inclusive_end, view.len, "slice")?;
            let next = match view.backing {
                RuntimeStrViewBacking::Buffer(buffer) => insert_runtime_str_view_from_buffer(
                    state,
                    buffer,
                    view.start + start,
                    end - start,
                ),
                RuntimeStrViewBacking::Reference(reference) => {
                    insert_runtime_str_view_from_reference(
                        state,
                        reference,
                        view.start + start,
                        end - start,
                    )
                }
            };
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(next)))
        }
        other => Err(format!(
            "runtime slice requires array, bytes, utf16, string, or view backing, got `{other:?}`"
        )),
    }
}

pub(super) fn runtime_slice_window(
    start: Option<i64>,
    end: Option<i64>,
    inclusive_end: bool,
    len: usize,
    context: &str,
) -> Result<(usize, usize), String> {
    let start = runtime_slice_bound_to_usize(start, 0, len, context, "start")?;
    let has_end = end.is_some();
    let raw_end = runtime_slice_bound_to_usize(end, len, len, context, "end")?;
    let end = if inclusive_end {
        if has_end {
            if raw_end >= len {
                return Err(format!(
                    "{context} inclusive end `{raw_end}` is out of bounds for length `{len}`"
                ));
            }
            raw_end + 1
        } else {
            len
        }
    } else {
        raw_end
    };
    if start > end {
        return Err(format!(
            "{context} start `{start}` must be less than or equal to end `{end}`"
        ));
    }
    Ok((start, end))
}

pub(super) fn runtime_strided_projection_window(
    start: Option<i64>,
    len: Option<i64>,
    stride: Option<i64>,
    context: &str,
) -> Result<(usize, usize, usize), String> {
    let start = runtime_non_negative_usize(start.unwrap_or(0), &format!("{context} start"))?;
    let len = runtime_non_negative_usize(len.unwrap_or(0), &format!("{context} len"))?;
    let stride = runtime_non_negative_usize(stride.unwrap_or(1), &format!("{context} stride"))?;
    if stride == 0 {
        return Err(format!("{context} stride must be >= 1"));
    }
    if stride != 1 {
        return Err(format!(
            "{context} stride `{stride}` is not supported yet; only stride 1 is currently allowed"
        ));
    }
    Ok((start, len, stride))
}

pub(super) fn eval_runtime_strided_projection_value(
    base: RuntimeValue,
    start: Option<RuntimeValue>,
    len: Option<RuntimeValue>,
    stride: Option<RuntimeValue>,
    mutable: bool,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let base = read_runtime_value_if_ref(
        base,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let (start, len, _stride) = runtime_strided_projection_window(
        start
            .map(|value| expect_int(value, "strided projection start"))
            .transpose()?,
        len.map(|value| expect_int(value, "strided projection len"))
            .transpose()?,
        stride
            .map(|value| expect_int(value, "strided projection stride"))
            .transpose()?,
        "strided projection",
    )?;
    match base {
        RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)) => {
            let arena = state
                .ring_buffers
                .get(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            if len > arena.policy.window {
                return Err(format!(
                    "strided projection len `{len}` exceeds configured window `{}` for RingBuffer `{}`",
                    arena.policy.window, handle.0
                ));
            }
            let end = start
                .checked_add(len)
                .ok_or_else(|| "strided projection range overflowed".to_string())?;
            if end > arena.order.len() {
                return Err(format!(
                    "strided projection `{start}..{end}` is out of bounds for length `{}`",
                    arena.order.len()
                ));
            }
            let slots = arena
                .order
                .iter()
                .skip(start)
                .take(len)
                .copied()
                .collect::<Vec<_>>();
            if mutable {
                let ids = runtime_ring_ids_for_slots(state, handle, &slots)?;
                runtime_reject_live_reference_or_opaque_conflict(
                    Some(scopes.as_slice()),
                    None,
                    state,
                    |candidate| ids.iter().any(|id| runtime_reference_targets_ring_id(candidate, *id)),
                    |opaque, state| runtime_opaque_matches_ring_window_predicate(
                        opaque,
                        state,
                        &|candidate_arena, candidate_slots| {
                            runtime_ring_window_overlaps_slots(
                                handle,
                                &slots,
                                candidate_arena,
                                candidate_slots,
                            )
                        },
                    ),
                    None,
                    None,
                    |_| false,
                    "strided projection rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
                )?;
                let view =
                    insert_runtime_edit_view_from_ring_window(state, &[], handle, slots, 0, len);
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(view)))
            } else {
                let view =
                    insert_runtime_read_view_from_ring_window(state, &[], handle, slots, 0, len);
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(view)))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
            if mutable {
                return Err(
                    "runtime mutable strided projections require editable backing".to_string(),
                );
            }
            let (type_args, backing, view_start, view_len) = {
                let view = state
                    .read_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid View handle `{}`", handle.0))?;
                (
                    view.type_args.clone(),
                    view.backing.clone(),
                    view.start,
                    view.len,
                )
            };
            let end = start
                .checked_add(len)
                .ok_or_else(|| "strided projection range overflowed".to_string())?;
            let _ = runtime_view_range(start, len, view_len, "strided projection")?;
            let next = insert_runtime_read_view_from_backing(
                state,
                &type_args,
                backing,
                view_start + start,
                end - start,
            );
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(next)))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
            let (type_args, backing, view_start, view_len) = {
                let view = state
                    .edit_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid View handle `{}`", handle.0))?;
                (
                    view.type_args.clone(),
                    view.backing.clone(),
                    view.start,
                    view.len,
                )
            };
            let end = start
                .checked_add(len)
                .ok_or_else(|| "strided projection range overflowed".to_string())?;
            let _ = runtime_view_range(start, len, view_len, "strided projection")?;
            if mutable {
                let next = match backing {
                    RuntimeElementViewBacking::Buffer(buffer) => {
                        runtime_reject_live_reference_or_opaque_conflict(
                            Some(scopes.as_slice()),
                            None,
                            state,
                            |_| false,
                            |opaque, state| runtime_opaque_matches_element_buffer(opaque, state, buffer),
                            None,
                            Some(RuntimeOpaqueValue::EditView(handle)),
                            |_| false,
                            "strided projection rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
                        )?;
                        insert_runtime_edit_view_from_buffer(
                            state,
                            &type_args,
                            buffer,
                            view_start + start,
                            end - start,
                        )
                    }
                    RuntimeElementViewBacking::Reference(reference) => {
                        runtime_reject_live_reference_or_opaque_conflict(
                            Some(scopes.as_slice()),
                            None,
                            state,
                            |candidate| candidate.target == reference.target,
                            |opaque, state| runtime_opaque_matches_reference_predicate(
                                opaque,
                                state,
                                &|candidate| candidate.target == reference.target,
                            ),
                            None,
                            Some(RuntimeOpaqueValue::EditView(handle)),
                            |_| false,
                            "strided projection rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
                        )?;
                        insert_runtime_edit_view_from_reference(
                            state,
                            &type_args,
                            reference,
                            view_start + start,
                            end - start,
                        )
                    }
                    RuntimeElementViewBacking::RingWindow { arena, slots } => {
                        let active = runtime_ring_window_active_slots(&slots, view_start + start, end - start)
                            .ok_or_else(|| {
                                "runtime mutable strided projection range is out of bounds for ring window"
                                    .to_string()
                            })?
                            .to_vec();
                        let ids = runtime_ring_ids_for_slots(state, arena, &active)?;
                        runtime_reject_live_reference_or_opaque_conflict(
                            Some(scopes.as_slice()),
                            None,
                            state,
                            |candidate| ids.iter().any(|id| runtime_reference_targets_ring_id(candidate, *id)),
                            |opaque, state| runtime_opaque_matches_ring_window_predicate(
                                opaque,
                                state,
                                &|candidate_arena, candidate_slots| {
                                    runtime_ring_window_overlaps_slots(
                                        arena,
                                        &active,
                                        candidate_arena,
                                        candidate_slots,
                                    )
                                },
                            ),
                            None,
                            Some(RuntimeOpaqueValue::EditView(handle)),
                            |_| false,
                            "strided projection rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
                        )?;
                        insert_runtime_edit_view_from_ring_window(
                            state,
                            &type_args,
                            arena,
                            slots,
                            view_start + start,
                            end - start,
                        )
                    }
                };
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(next)))
            } else {
                let next = insert_runtime_read_view_from_backing(
                    state,
                    &type_args,
                    backing,
                    view_start + start,
                    end - start,
                );
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(next)))
            }
        }
        other => Err(format!(
            "runtime strided projection requires RingBuffer or View backing, got `{other:?}`"
        )),
    }
}

pub(super) fn eval_runtime_borrowed_strided_projection_view(
    base_expr: &ParsedExpr,
    start: Option<&ParsedExpr>,
    len: Option<&ParsedExpr>,
    stride: Option<&ParsedExpr>,
    mutable: bool,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let base = eval_expr(
        base_expr,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )
    .map_err(runtime_eval_message)?;
    let start = start
        .map(|expr| {
            eval_expr(
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
        })
        .transpose()
        .map_err(runtime_eval_message)?;
    let len = len
        .map(|expr| {
            eval_expr(
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
        })
        .transpose()
        .map_err(runtime_eval_message)?;
    let stride = stride
        .map(|expr| {
            eval_expr(
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
        })
        .transpose()
        .map_err(runtime_eval_message)?;
    eval_runtime_strided_projection_value(
        base,
        start,
        len,
        stride,
        mutable,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )
}

pub(super) fn eval_runtime_borrowed_slice_view(
    base_expr: &ParsedExpr,
    start: Option<&ParsedExpr>,
    end: Option<&ParsedExpr>,
    inclusive_end: bool,
    mutable: bool,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let borrowed_place = expr_to_assign_target(base_expr)
        .and_then(|target| resolve_assign_target_place(scopes, &target).ok());
    if mutable
        && let Some(place) = &borrowed_place
        && !place.mode.allows_write()
    {
        return Err("runtime mutable borrowed slices require writable backing".to_string());
    }
    let base = read_runtime_value_if_ref(
        eval_expr(
            base_expr,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )
        .map_err(runtime_eval_message)?,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let start = eval_optional_runtime_int_expr(
        start,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
        "borrowed slice start",
    )
    .map_err(runtime_eval_message)?;
    let end = eval_optional_runtime_int_expr(
        end,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
        "borrowed slice end",
    )
    .map_err(runtime_eval_message)?;
    match base {
        RuntimeValue::List(_) => Err(
            "runtime borrowed slices require contiguous backing; `List` is not supported"
                .to_string(),
        ),
        RuntimeValue::Array(values) => {
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, values.len(), "borrowed slice")?;
            let len = end - start;
            let type_args = runtime_array_projection_type_args(&values[start..end], state);
            if let Some(place) = borrowed_place {
                if let RuntimeReferenceTarget::Local { local, .. } = &place.target
                    && let Some((_, runtime_local)) =
                        lookup_local_with_name_by_handle(scopes, *local)
                {
                    state
                        .captured_local_values
                        .entry(*local)
                        .or_insert_with(|| runtime_local.value.clone());
                }
                let reference = RuntimeReferenceValue {
                    mode: place.mode,
                    target: place.target,
                };
                if mutable {
                    runtime_reject_live_reference_or_opaque_conflict(
                        Some(scopes.as_slice()),
                        None,
                        state,
                        |candidate| candidate.target == reference.target,
                        |opaque, state| runtime_opaque_matches_reference_predicate(
                            opaque,
                            state,
                            &|candidate| candidate.target == reference.target,
                        ),
                        None,
                        None,
                        |_| false,
                        "runtime mutable borrowed slices require exclusive view access while conflicting borrows or views are live".to_string(),
                    )?;
                    let handle = insert_runtime_edit_view_from_reference(
                        state, &type_args, reference, start, len,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)))
                } else {
                    let handle = insert_runtime_read_view_from_reference(
                        state, &type_args, reference, start, len,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)))
                }
            } else if mutable {
                let handle =
                    insert_runtime_edit_view(state, &type_args, values[start..end].to_vec());
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)))
            } else {
                let handle =
                    insert_runtime_read_view(state, &type_args, values[start..end].to_vec());
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)))
            }
        }
        RuntimeValue::Bytes(values) => {
            if mutable {
                return Err(
                    "runtime byte slices are read-only on Bytes; use ByteBuffer for `&edit` access"
                        .to_string(),
                );
            }
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, values.len(), "borrowed slice")?;
            let len = end - start;
            if let Some(place) = borrowed_place {
                if let RuntimeReferenceTarget::Local { local, .. } = &place.target
                    && let Some((_, runtime_local)) =
                        lookup_local_with_name_by_handle(scopes, *local)
                {
                    state
                        .captured_local_values
                        .entry(*local)
                        .or_insert_with(|| runtime_local.value.clone());
                }
                let reference = RuntimeReferenceValue {
                    mode: place.mode,
                    target: place.target,
                };
                let handle = insert_runtime_byte_view_from_reference(state, reference, start, len);
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)))
            } else {
                let backing = insert_runtime_byte_view_buffer(state, values);
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(
                    insert_runtime_byte_view_from_buffer(state, backing, start, len),
                )))
            }
        }
        RuntimeValue::ByteBuffer(values) => {
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, values.len(), "borrowed slice")?;
            let len = end - start;
            if let Some(place) = borrowed_place {
                if let RuntimeReferenceTarget::Local { local, .. } = &place.target
                    && let Some((_, runtime_local)) =
                        lookup_local_with_name_by_handle(scopes, *local)
                {
                    state
                        .captured_local_values
                        .entry(*local)
                        .or_insert_with(|| runtime_local.value.clone());
                }
                let reference = RuntimeReferenceValue {
                    mode: place.mode,
                    target: place.target,
                };
                if mutable {
                    runtime_reject_live_reference_or_opaque_conflict(
                        Some(scopes.as_slice()),
                        None,
                        state,
                        |candidate| candidate.target == reference.target,
                        |opaque, state| runtime_opaque_matches_reference_predicate(
                            opaque,
                            state,
                            &|candidate| candidate.target == reference.target,
                        ),
                        None,
                        None,
                        |_| false,
                        "runtime mutable borrowed slices require exclusive view access while conflicting borrows or views are live".to_string(),
                    )?;
                    let handle =
                        insert_runtime_byte_edit_view_from_reference(state, reference, start, len);
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(
                        handle,
                    )))
                } else {
                    let handle =
                        insert_runtime_byte_view_from_reference(state, reference, start, len);
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)))
                }
            } else {
                let backing = insert_runtime_byte_view_buffer(state, values);
                if mutable {
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(
                        insert_runtime_byte_edit_view_from_buffer(state, backing, start, len),
                    )))
                } else {
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(
                        insert_runtime_byte_view_from_buffer(state, backing, start, len),
                    )))
                }
            }
        }
        RuntimeValue::Utf16(units) => {
            if mutable {
                return Err(
                    "runtime utf16 slices are read-only on Utf16; use Utf16Buffer for `&edit` access"
                        .to_string(),
                );
            }
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, units.len(), "borrowed slice")?;
            let len = end - start;
            let type_args = vec!["U16".to_string()];
            if let Some(place) = borrowed_place {
                if let RuntimeReferenceTarget::Local { local, .. } = &place.target
                    && let Some((_, runtime_local)) =
                        lookup_local_with_name_by_handle(scopes, *local)
                {
                    state
                        .captured_local_values
                        .entry(*local)
                        .or_insert_with(|| runtime_local.value.clone());
                }
                let reference = RuntimeReferenceValue {
                    mode: place.mode,
                    target: place.target,
                };
                let handle = insert_runtime_read_view_from_reference(
                    state, &type_args, reference, start, len,
                );
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)))
            } else {
                let values = units[start..end]
                    .iter()
                    .copied()
                    .map(|value| RuntimeValue::Int(i64::from(value)))
                    .collect::<Vec<_>>();
                let handle = insert_runtime_read_view(state, &type_args, values);
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)))
            }
        }
        RuntimeValue::Utf16Buffer(units) => {
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, units.len(), "borrowed slice")?;
            let len = end - start;
            let type_args = vec!["U16".to_string()];
            if let Some(place) = borrowed_place {
                if let RuntimeReferenceTarget::Local { local, .. } = &place.target
                    && let Some((_, runtime_local)) =
                        lookup_local_with_name_by_handle(scopes, *local)
                {
                    state
                        .captured_local_values
                        .entry(*local)
                        .or_insert_with(|| runtime_local.value.clone());
                }
                let reference = RuntimeReferenceValue {
                    mode: place.mode,
                    target: place.target,
                };
                if mutable {
                    runtime_reject_live_reference_or_opaque_conflict(
                        Some(scopes.as_slice()),
                        None,
                        state,
                        |candidate| candidate.target == reference.target,
                        |opaque, state| runtime_opaque_matches_reference_predicate(
                            opaque,
                            state,
                            &|candidate| candidate.target == reference.target,
                        ),
                        None,
                        None,
                        |_| false,
                        "runtime mutable borrowed slices require exclusive view access while conflicting borrows or views are live".to_string(),
                    )?;
                    let handle = insert_runtime_edit_view_from_reference(
                        state, &type_args, reference, start, len,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)))
                } else {
                    let handle = insert_runtime_read_view_from_reference(
                        state, &type_args, reference, start, len,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)))
                }
            } else {
                let values = units[start..end]
                    .iter()
                    .copied()
                    .map(|value| RuntimeValue::Int(i64::from(value)))
                    .collect::<Vec<_>>();
                if mutable {
                    let handle = insert_runtime_edit_view(state, &type_args, values);
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)))
                } else {
                    let handle = insert_runtime_read_view(state, &type_args, values);
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)))
                }
            }
        }
        RuntimeValue::Str(text) => {
            if mutable {
                return Err(
                    "runtime string slices are read-only; `&edit x[a..b]` is not allowed"
                        .to_string(),
                );
            }
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, text.len(), "borrowed slice")?;
            let handle = {
                let backing = insert_runtime_str_view_buffer(state, text);
                insert_runtime_str_view_from_buffer(state, backing, start, end - start)
            };
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
            if mutable {
                return Err("runtime mutable borrowed slices require editable backing".to_string());
            }
            let (type_args, backing, view_start, view_len) = {
                let view = state
                    .read_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid ReadView handle `{}`", handle.0))?;
                (
                    view.type_args.clone(),
                    view.backing.clone(),
                    view.start,
                    view.len,
                )
            };
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, view_len, "borrowed slice")?;
            let next = insert_runtime_read_view_from_backing(
                state,
                &type_args,
                backing,
                view_start + start,
                end - start,
            );
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(next)))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
            let (type_args, backing, view_start, view_len) = {
                let view = state
                    .edit_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?;
                (
                    view.type_args.clone(),
                    view.backing.clone(),
                    view.start,
                    view.len,
                )
            };
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, view_len, "borrowed slice")?;
            if mutable {
                let next = match backing {
                    RuntimeElementViewBacking::Buffer(buffer) => {
                        runtime_reject_live_reference_or_opaque_conflict(
                            Some(scopes.as_slice()),
                            None,
                            state,
                            |_| false,
                            |opaque, state| {
                                runtime_opaque_matches_element_buffer(opaque, state, buffer)
                            },
                            None,
                            Some(RuntimeOpaqueValue::EditView(handle)),
                            |_| false,
                            "runtime mutable borrowed slices require exclusive view access while conflicting borrows or views are live".to_string(),
                        )?;
                        insert_runtime_edit_view_from_buffer(
                            state,
                            &type_args,
                            buffer,
                            view_start + start,
                            end - start,
                        )
                    }
                    RuntimeElementViewBacking::Reference(reference) => {
                        runtime_reject_live_reference_or_opaque_conflict(
                            Some(scopes.as_slice()),
                            None,
                            state,
                            |candidate| candidate.target == reference.target,
                            |opaque, state| runtime_opaque_matches_reference_predicate(
                                opaque,
                                state,
                                &|candidate| candidate.target == reference.target,
                            ),
                            None,
                            Some(RuntimeOpaqueValue::EditView(handle)),
                            |_| false,
                            "runtime mutable borrowed slices require exclusive view access while conflicting borrows or views are live".to_string(),
                        )?;
                        insert_runtime_edit_view_from_reference(
                            state,
                            &type_args,
                            reference,
                            view_start + start,
                            end - start,
                        )
                    }
                    RuntimeElementViewBacking::RingWindow { arena, slots } => {
                        let active = runtime_ring_window_active_slots(
                            &slots,
                            view_start + start,
                            end - start,
                        )
                        .ok_or_else(|| {
                            "runtime mutable borrowed slice range is out of bounds for ring window"
                                .to_string()
                        })?
                        .to_vec();
                        let ids = runtime_ring_ids_for_slots(state, arena, &active)?;
                        runtime_reject_live_reference_or_opaque_conflict(
                            Some(scopes.as_slice()),
                            None,
                            state,
                            |candidate| ids.iter().any(|id| runtime_reference_targets_ring_id(candidate, *id)),
                            |opaque, state| runtime_opaque_matches_ring_window_predicate(
                                opaque,
                                state,
                                &|candidate_arena, candidate_slots| {
                                    runtime_ring_window_overlaps_slots(
                                        arena,
                                        &active,
                                        candidate_arena,
                                        candidate_slots,
                                    )
                                },
                            ),
                            None,
                            Some(RuntimeOpaqueValue::EditView(handle)),
                            |_| false,
                            "runtime mutable borrowed slices require exclusive view access while conflicting borrows or views are live".to_string(),
                        )?;
                        insert_runtime_edit_view_from_ring_window(
                            state,
                            &type_args,
                            arena,
                            slots,
                            view_start + start,
                            end - start,
                        )
                    }
                };
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(next)))
            } else {
                let next = insert_runtime_read_view_from_backing(
                    state,
                    &type_args,
                    backing,
                    view_start + start,
                    end - start,
                );
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(next)))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
            if mutable {
                return Err("runtime mutable borrowed slices require editable backing".to_string());
            }
            let view = state
                .byte_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                .clone();
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, view.len, "borrowed slice")?;
            let next = match view.backing {
                RuntimeByteViewBacking::Buffer(buffer) => insert_runtime_byte_view_from_buffer(
                    state,
                    buffer,
                    view.start + start,
                    end - start,
                ),
                RuntimeByteViewBacking::Reference(reference) => {
                    insert_runtime_byte_view_from_reference(
                        state,
                        reference,
                        view.start + start,
                        end - start,
                    )
                }
                RuntimeByteViewBacking::Foreign(backing) => insert_runtime_byte_view_from_foreign(
                    state,
                    backing.package_id,
                    backing.handle,
                    view.start + start,
                    end - start,
                ),
            };
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(next)))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, view.len, "borrowed slice")?;
            if mutable {
                runtime_reject_live_reference_or_opaque_conflict(
                    Some(scopes.as_slice()),
                    None,
                    state,
                    |candidate| match &view.backing {
                        RuntimeByteViewBacking::Buffer(_) => false,
                        RuntimeByteViewBacking::Reference(reference) => {
                            candidate.target == reference.target
                        }
                        RuntimeByteViewBacking::Foreign(_) => false,
                    },
                    |opaque, state| match &view.backing {
                        RuntimeByteViewBacking::Buffer(buffer) => {
                            runtime_opaque_matches_byte_buffer(opaque, state, *buffer)
                        }
                        RuntimeByteViewBacking::Reference(reference) => {
                            runtime_opaque_matches_reference_predicate(
                                opaque,
                                state,
                                &|candidate| candidate.target == reference.target,
                            )
                        }
                        RuntimeByteViewBacking::Foreign(backing) => {
                            runtime_opaque_matches_foreign_byte_handle(
                                opaque,
                                state,
                                backing.package_id,
                                backing.handle,
                            )
                        }
                    },
                    None,
                    Some(RuntimeOpaqueValue::ByteEditView(handle)),
                    |_| false,
                    "runtime mutable borrowed slices require exclusive view access while conflicting borrows or views are live".to_string(),
                )?;
                let next = match view.backing {
                    RuntimeByteViewBacking::Buffer(buffer) => {
                        insert_runtime_byte_edit_view_from_buffer(
                            state,
                            buffer,
                            view.start + start,
                            end - start,
                        )
                    }
                    RuntimeByteViewBacking::Reference(reference) => {
                        insert_runtime_byte_edit_view_from_reference(
                            state,
                            reference,
                            view.start + start,
                            end - start,
                        )
                    }
                    RuntimeByteViewBacking::Foreign(backing) => {
                        insert_runtime_byte_edit_view_from_foreign(
                            state,
                            backing.package_id,
                            backing.handle,
                            view.start + start,
                            end - start,
                        )
                    }
                };
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(next)))
            } else {
                let next = match view.backing {
                    RuntimeByteViewBacking::Buffer(buffer) => insert_runtime_byte_view_from_buffer(
                        state,
                        buffer,
                        view.start + start,
                        end - start,
                    ),
                    RuntimeByteViewBacking::Reference(reference) => {
                        insert_runtime_byte_view_from_reference(
                            state,
                            reference,
                            view.start + start,
                            end - start,
                        )
                    }
                    RuntimeByteViewBacking::Foreign(backing) => {
                        insert_runtime_byte_view_from_foreign(
                            state,
                            backing.package_id,
                            backing.handle,
                            view.start + start,
                            end - start,
                        )
                    }
                };
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(next)))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
            if mutable {
                return Err(
                    "runtime string slices are read-only; `&edit x[a..b]` is not allowed"
                        .to_string(),
                );
            }
            let view = state
                .str_views
                .get(&handle)
                .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                .clone();
            let (start, end) =
                runtime_slice_window(start, end, inclusive_end, view.len, "borrowed slice")?;
            let next = match view.backing {
                RuntimeStrViewBacking::Buffer(buffer) => insert_runtime_str_view_from_buffer(
                    state,
                    buffer,
                    view.start + start,
                    end - start,
                ),
                RuntimeStrViewBacking::Reference(reference) => {
                    insert_runtime_str_view_from_reference(
                        state,
                        reference,
                        view.start + start,
                        end - start,
                    )
                }
            };
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(next)))
        }
        other => Err(format!(
            "runtime borrowed slice expects contiguous array, bytes, utf16, view, or string backing, got `{other:?}`"
        )),
    }
}
