use super::*;

fn materialize_runtime_return_value(
    value: RuntimeValue,
    return_type: Option<&IrRoutineType>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let return_root = return_type.and_then(|ty| match &ty.kind {
        IrRoutineTypeKind::Path(path) => path.root_name(),
        IrRoutineTypeKind::Apply { base, .. } => base.root_name(),
        _ => None,
    });
    match (return_root, &value) {
        (Some("Bytes"), RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle))) => {
            let view = state
                .byte_views
                .get(handle)
                .cloned()
                .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?;
            return Ok(RuntimeValue::Bytes(runtime_byte_view_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &view,
                host,
                "return value",
            )?));
        }
        (Some("Bytes"), RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle))) => {
            let view = state
                .byte_edit_views
                .get(handle)
                .cloned()
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?;
            return Ok(RuntimeValue::Bytes(runtime_byte_edit_view_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &view,
                host,
                "return value",
            )?));
        }
        (Some("Str"), RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle))) => {
            let view = state
                .str_views
                .get(handle)
                .cloned()
                .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?;
            return Ok(RuntimeValue::Str(runtime_str_view_text(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &view,
                host,
                "return value",
            )?));
        }
        _ => {}
    }
    if matches!(
        return_type.map(|ty| &ty.kind),
        Some(IrRoutineTypeKind::Ref { .. })
    ) || (return_type.is_none() && matches!(value, RuntimeValue::Ref(_)))
    {
        return Ok(value);
    }
    read_runtime_value_if_ref(
        value,
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

pub(super) fn eval_expr(
    expr: &ParsedExpr,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    match expr {
        ParsedExpr::Int(value) => Ok(RuntimeValue::Int(*value)),
        ParsedExpr::Float { text, kind } => Ok(RuntimeValue::Float {
            text: text.clone(),
            kind: *kind,
        }),
        ParsedExpr::Bool(value) => Ok(RuntimeValue::Bool(*value)),
        ParsedExpr::Str(value) => Ok(RuntimeValue::Str(value.clone())),
        ParsedExpr::Pair { left, right } => Ok(RuntimeValue::Pair(
            Box::new(eval_expr(
                left,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?),
            Box::new(eval_expr(
                right,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?),
        )),
        ParsedExpr::Collection { items } => Ok(RuntimeValue::List(
            items
                .iter()
                .map(|item| {
                    eval_expr(
                        item,
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
                .collect::<RuntimeEvalResult<Vec<_>>>()?,
        )),
        ParsedExpr::Match { subject, arms } => eval_match_expr(
            subject,
            arms,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::ConstructRegion(region) => {
            let target_name = runtime_expr_path_name(&region.target).ok_or_else(|| {
                "construct target must be a path-like constructor reference".to_string()
            })?;
            let mut fields = BTreeMap::new();
            let mut payload = Vec::new();
            for line in &region.lines {
                if let Some(value) = eval_construct_contribution_value(
                    line,
                    region.default_modifier.as_ref(),
                    "construct",
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )? {
                    if line.name == "payload" {
                        payload.push(value);
                    } else {
                        fields.insert(line.name.clone(), value);
                    }
                }
            }
            if !payload.is_empty() && fields.is_empty() {
                Ok(RuntimeValue::Variant {
                    name: target_name,
                    payload,
                })
            } else {
                Ok(RuntimeValue::Record {
                    name: target_name,
                    fields,
                })
            }
        }
        ParsedExpr::RecordRegion(region) => eval_record_region_value(
            region,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::ArrayRegion(region) => eval_array_region_value(
            region,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::Chain {
            style,
            introducer,
            steps,
        } => eval_runtime_chain_expr(
            style,
            *introducer,
            steps,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::Path(segments) => {
            if segments.len() == 1 && lookup_local(scopes, &segments[0]).is_some() {
                return force_runtime_value(
                    read_runtime_local_value(scopes, state, &segments[0])?,
                    plan,
                    state,
                    host,
                )
                .map_err(Into::into);
            }
            if let Some(value) = try_eval_runtime_const_value_expr(
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                state,
                host,
            )? {
                return Ok(value);
            }
            Err(format!("unsupported runtime value path `{}`", segments.join(".")).into())
        }
        ParsedExpr::Member { expr, member } => {
            let full_expr = ParsedExpr::Member {
                expr: expr.clone(),
                member: member.clone(),
            };
            if let Some(value) = try_eval_runtime_const_value_expr(
                &full_expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                state,
                host,
            )? {
                return Ok(value);
            }
            if let ParsedExpr::Unary {
                op: ParsedUnaryOp::Deref,
                expr: capability_expr,
            } = expr.as_ref()
            {
                let token_expr = capability_expr.as_ref().clone();
                let reference = expect_reference(
                    eval_expr(
                        capability_expr,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    "deref",
                )?;
                return match reference.mode {
                    RuntimeReferenceMode::Take => {
                        let value = read_runtime_reference(
                            scopes,
                            plan,
                            current_package_id,
                            current_module_id,
                            aliases,
                            type_bindings,
                            state,
                            &reference,
                            host,
                        )?;
                        redeem_take_reference(scopes, &reference)
                            .map_err(RuntimeEvalSignal::from)?;
                        reclaim_hold_capability_root_local(scopes, &token_expr)
                            .map_err(RuntimeEvalSignal::from)?;
                        Ok(eval_member_value(value, member)?)
                    }
                    RuntimeReferenceMode::Read
                    | RuntimeReferenceMode::Edit
                    | RuntimeReferenceMode::Hold => read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        &RuntimeReferenceValue {
                            mode: reference.mode,
                            target: runtime_reference_with_member(
                                &reference.target,
                                member.clone(),
                            ),
                        },
                        host,
                    )
                    .map_err(Into::into),
                };
            }
            let base = eval_expr(
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            Ok(eval_runtime_member_value(base, member)?)
        }
        ParsedExpr::Index { expr, index } => Ok(eval_runtime_index_value(
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
            )?,
            eval_expr(
                index,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?,
            scopes,
            plan,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        )?),
        ParsedExpr::Slice {
            expr,
            family,
            start,
            end,
            len,
            stride,
            inclusive_end,
        } => {
            let base = eval_expr(
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let start_value = start
                .as_deref()
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
                .transpose()?;
            let end_value = end
                .as_deref()
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
                .transpose()?;
            let len_value = len
                .as_deref()
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
                .transpose()?;
            let stride_value = stride
                .as_deref()
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
                .transpose()?;
            Ok(match family {
                ParsedProjectionFamily::Strided => eval_runtime_strided_projection_value(
                    base,
                    start_value,
                    len_value,
                    stride_value,
                    false,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
                ParsedProjectionFamily::Inferred | ParsedProjectionFamily::Contiguous => {
                    eval_runtime_slice_value(
                        base,
                        start_value,
                        end_value,
                        *inclusive_end,
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?
                }
            })
        }
        ParsedExpr::Range {
            start,
            end,
            inclusive_end,
        } => Ok(RuntimeValue::Range {
            start: eval_optional_runtime_int_expr(
                start.as_deref(),
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                "range start",
            )?,
            end: eval_optional_runtime_int_expr(
                end.as_deref(),
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                "range end",
            )?,
            inclusive_end: *inclusive_end,
        }),
        ParsedExpr::Generic { expr, .. } => eval_expr(
            expr,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::Await { expr } => Ok(await_runtime_value(
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
            )?,
            plan,
            state,
            host,
        )?),
        ParsedExpr::Unary { op, expr } => match op {
            ParsedUnaryOp::Weave | ParsedUnaryOp::Split => eval_spawn_expr(
                *op,
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedUnaryOp::CapabilityRead
            | ParsedUnaryOp::CapabilityEdit
            | ParsedUnaryOp::CapabilityTake
            | ParsedUnaryOp::CapabilityHold => {
                let mode = runtime_reference_mode_from_unary(*op)
                    .ok_or_else(|| format!("unsupported capability op `{op:?}`"))?;
                if let ParsedExpr::Slice {
                    expr,
                    family,
                    start,
                    end,
                    len,
                    stride,
                    inclusive_end,
                } = expr.as_ref()
                {
                    if !matches!(
                        mode,
                        RuntimeReferenceMode::Read | RuntimeReferenceMode::Edit
                    ) {
                        return Err(format!(
                            "runtime capability `{}` does not support slice views",
                            mode.as_str()
                        )
                        .into());
                    }
                    return match family {
                        ParsedProjectionFamily::Strided => {
                            eval_runtime_borrowed_strided_projection_view(
                                expr,
                                start.as_deref(),
                                len.as_deref(),
                                stride.as_deref(),
                                matches!(mode, RuntimeReferenceMode::Edit),
                                plan,
                                current_package_id,
                                current_module_id,
                                scopes,
                                aliases,
                                type_bindings,
                                state,
                                host,
                            )
                            .map_err(RuntimeEvalSignal::from)
                        }
                        ParsedProjectionFamily::Inferred | ParsedProjectionFamily::Contiguous => {
                            eval_runtime_borrowed_slice_view(
                                expr,
                                start.as_deref(),
                                end.as_deref(),
                                *inclusive_end,
                                matches!(mode, RuntimeReferenceMode::Edit),
                                plan,
                                current_package_id,
                                current_module_id,
                                scopes,
                                aliases,
                                type_bindings,
                                state,
                                host,
                            )
                            .map_err(RuntimeEvalSignal::from)
                        }
                    };
                }
                let target = expr_to_assign_target(expr).ok_or_else(|| {
                    format!(
                        "runtime capability operand `{:?}` is not a writable place",
                        expr
                    )
                })?;
                let place = resolve_assign_target_place(scopes, &target)?;
                if mode.allows_write() && !place.mode.allows_write() {
                    return Err(format!(
                        "runtime capability `{}` operand `{:?}` is not mutable",
                        mode.as_str(),
                        expr
                    )
                    .into());
                }
                match mode {
                    RuntimeReferenceMode::Take => reserve_take_capability_root_local(scopes, expr)
                        .map_err(RuntimeEvalSignal::from)?,
                    RuntimeReferenceMode::Hold => reserve_hold_capability_root_local(scopes, expr)
                        .map_err(RuntimeEvalSignal::from)?,
                    RuntimeReferenceMode::Read | RuntimeReferenceMode::Edit => {}
                }
                Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                    mode,
                    target: place.target,
                }))
            }
            ParsedUnaryOp::Deref => {
                let token_expr = expr.as_ref().clone();
                let reference = expect_reference(
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
                    )?,
                    "deref",
                )?;
                let value = read_runtime_reference(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    host,
                )?;
                if reference.mode == RuntimeReferenceMode::Take {
                    redeem_take_reference(scopes, &reference).map_err(RuntimeEvalSignal::from)?;
                    reclaim_hold_capability_root_local(scopes, &token_expr)
                        .map_err(RuntimeEvalSignal::from)?;
                }
                Ok(value)
            }
            ParsedUnaryOp::Neg => {
                let value = read_runtime_value_if_ref(
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
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                match value {
                    RuntimeValue::Float { text, kind } => Ok(make_runtime_float(
                        kind,
                        -parse_runtime_float_text(&text, kind)?,
                    )),
                    other => Ok(RuntimeValue::Int(-expect_int(other, "unary -")?)),
                }
            }
            ParsedUnaryOp::Not => Ok(RuntimeValue::Bool(!expect_bool(
                read_runtime_value_if_ref(
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
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
                "not",
            )?)),
            ParsedUnaryOp::BitNot => Ok(RuntimeValue::Int(!expect_int(
                read_runtime_value_if_ref(
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
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
                "~",
            )?)),
        },
        ParsedExpr::Binary { left, op, right } => match op {
            ParsedBinaryOp::Or => {
                let left = expect_bool(
                    read_runtime_value_if_ref(
                        eval_expr(
                            left,
                            plan,
                            current_package_id,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    "or",
                )?;
                if left {
                    Ok(RuntimeValue::Bool(true))
                } else {
                    Ok(RuntimeValue::Bool(expect_bool(
                        read_runtime_value_if_ref(
                            eval_expr(
                                right,
                                plan,
                                current_package_id,
                                current_module_id,
                                scopes,
                                aliases,
                                type_bindings,
                                state,
                                host,
                            )?,
                            scopes,
                            plan,
                            current_package_id,
                            current_module_id,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        "or",
                    )?))
                }
            }
            ParsedBinaryOp::And => {
                let left = expect_bool(
                    read_runtime_value_if_ref(
                        eval_expr(
                            left,
                            plan,
                            current_package_id,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    "and",
                )?;
                if !left {
                    Ok(RuntimeValue::Bool(false))
                } else {
                    Ok(RuntimeValue::Bool(expect_bool(
                        read_runtime_value_if_ref(
                            eval_expr(
                                right,
                                plan,
                                current_package_id,
                                current_module_id,
                                scopes,
                                aliases,
                                type_bindings,
                                state,
                                host,
                            )?,
                            scopes,
                            plan,
                            current_package_id,
                            current_module_id,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        "and",
                    )?))
                }
            }
            other => {
                let left = read_runtime_value_if_ref(
                    eval_expr(
                        left,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let right = read_runtime_value_if_ref(
                    eval_expr(
                        right,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                Ok(apply_binary_op(*other, left, right)?)
            }
        },
        ParsedExpr::Phrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            qualifier_type_args,
            resolved_callable,
            resolved_routine,
            dynamic_dispatch,
            attached,
        } => match qualifier_kind {
            ParsedPhraseQualifierKind::Call => execute_runtime_apply_phrase(
                subject,
                args,
                attached,
                qualifier_type_args,
                resolved_callable.as_deref(),
                resolved_routine.as_deref(),
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                runtime_async_calls_allowed(state),
            ),
            ParsedPhraseQualifierKind::NamedPath => execute_runtime_named_qualifier_call(
                subject,
                args,
                attached,
                qualifier,
                qualifier_type_args,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedPhraseQualifierKind::BareMethod => eval_qualifier(
                subject,
                args,
                attached,
                qualifier,
                qualifier_type_args,
                resolved_callable.as_deref(),
                resolved_routine.as_deref(),
                dynamic_dispatch.as_ref(),
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedPhraseQualifierKind::Try => eval_try_qualifier(
                subject,
                args,
                attached,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedPhraseQualifierKind::Apply => execute_runtime_apply_phrase(
                subject,
                args,
                attached,
                &[],
                resolved_callable.as_deref(),
                None,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                runtime_async_calls_allowed(state),
            ),
            ParsedPhraseQualifierKind::AwaitApply => {
                if args.is_empty() && attached.is_empty() {
                    return Ok(await_runtime_value(
                        eval_expr(
                            subject,
                            plan,
                            current_package_id,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        plan,
                        state,
                        host,
                    )?);
                }
                let value = execute_runtime_apply_phrase(
                    subject,
                    args,
                    attached,
                    &[],
                    None,
                    None,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                    true,
                )?;
                match value {
                    RuntimeValue::Opaque(RuntimeOpaqueValue::Task(_))
                    | RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(_)) => {
                        Ok(await_runtime_value(value, plan, state, host)?)
                    }
                    other => Ok(other),
                }
            }
            ParsedPhraseQualifierKind::Await => {
                if !args.is_empty() {
                    return Err("`:: await` does not accept arguments".to_string().into());
                }
                if !attached.is_empty() {
                    return Err("`:: await` does not support an attached block"
                        .to_string()
                        .into());
                }
                Ok(await_runtime_value(
                    eval_expr(
                        subject,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    plan,
                    state,
                    host,
                )?)
            }
            ParsedPhraseQualifierKind::Weave => {
                if let Some(spawned) = capture_spawned_phrase_call(
                    ParsedUnaryOp::Weave,
                    subject,
                    args,
                    attached,
                    ParsedPhraseQualifierKind::Weave,
                    qualifier,
                    qualifier_type_args,
                    resolved_callable.as_deref(),
                    resolved_routine.as_deref(),
                    dynamic_dispatch.as_ref(),
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )? {
                    Ok(spawned)
                } else {
                    Err("`:: weave` expects a callable phrase target"
                        .to_string()
                        .into())
                }
            }
            ParsedPhraseQualifierKind::Split => {
                if let Some(spawned) = capture_spawned_phrase_call(
                    ParsedUnaryOp::Split,
                    subject,
                    args,
                    attached,
                    ParsedPhraseQualifierKind::Split,
                    qualifier,
                    qualifier_type_args,
                    resolved_callable.as_deref(),
                    resolved_routine.as_deref(),
                    dynamic_dispatch.as_ref(),
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )? {
                    Ok(spawned)
                } else {
                    Err("`:: split` expects a callable phrase target"
                        .to_string()
                        .into())
                }
            }
            ParsedPhraseQualifierKind::Must => {
                if !args.is_empty() {
                    return Err("`:: must` does not accept arguments".to_string().into());
                }
                if !attached.is_empty() {
                    return Err("`:: must` does not support an attached block"
                        .to_string()
                        .into());
                }
                must_unwrap_runtime_value(eval_expr(
                    subject,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?)
            }
            ParsedPhraseQualifierKind::Fallback => {
                if !attached.is_empty() {
                    return Err("`:: fallback` does not support an attached block"
                        .to_string()
                        .into());
                }
                let [fallback_arg] = args.as_slice() else {
                    return Err(
                        "`:: fallback` expects exactly one positional fallback argument"
                            .to_string()
                            .into(),
                    );
                };
                let Some(fallback_expr) =
                    (fallback_arg.name.is_none()).then_some(&fallback_arg.value)
                else {
                    return Err(
                        "`:: fallback` expects exactly one positional fallback argument"
                            .to_string()
                            .into(),
                    );
                };
                let subject_value = eval_expr(
                    subject,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let fallback_value = eval_expr(
                    fallback_expr,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                fallback_runtime_value(subject_value, fallback_value)
            }
        },
        ParsedExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => {
            let arena_value = resolve_runtime_memory_phrase_instance(
                family,
                arena,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let constructed = execute_runtime_apply_phrase(
                constructor,
                init_args,
                attached,
                &[],
                None,
                None,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                false,
            )?;
            let intrinsic = match family.as_str() {
                "arena" => RuntimeIntrinsic::MemoryArenaAlloc,
                "frame" => RuntimeIntrinsic::MemoryFrameAlloc,
                "pool" => RuntimeIntrinsic::MemoryPoolAlloc,
                "temp" => RuntimeIntrinsic::MemoryTempAlloc,
                "session" => RuntimeIntrinsic::MemorySessionAlloc,
                "ring" => RuntimeIntrinsic::MemoryRingPush,
                "slab" => RuntimeIntrinsic::MemorySlabAlloc,
                other => return Err(format!("unsupported runtime memory family `{other}`").into()),
            };
            let type_args = runtime_receiver_type_args(&arena_value, state);
            let mut values = vec![arena_value, constructed];
            Ok(execute_runtime_intrinsic(
                intrinsic,
                &type_args,
                &mut values,
                plan,
                None,
                None,
                None,
                None,
                None,
                state,
                host,
            )?)
        }
    }
}

pub(super) fn try_eval_runtime_const_value_expr(
    expr: &ParsedExpr,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
    let Some(callable) = resolve_named_qualifier_callable_path(expr, aliases) else {
        return Ok(None);
    };
    let Some(routine_index) =
        resolve_routine_index(plan, current_package_id, current_module_id, &callable)
    else {
        return Ok(None);
    };
    let Some(routine) = plan.routines.get(routine_index) else {
        return Err(format!("invalid routine index `{routine_index}`").into());
    };
    if routine.symbol_kind != "const" {
        return Ok(None);
    }
    let label = runtime_expr_path_name(expr).unwrap_or_else(|| callable.join("."));
    if routine.is_async {
        return Err(format!(
            "runtime const path `{}` cannot target async routine `{}`",
            label, routine.symbol_name
        )
        .into());
    }
    if !routine.params.is_empty() {
        return Err(format!(
            "runtime const path `{}` cannot target non-zero-arg const `{}`",
            label, routine.symbol_name
        )
        .into());
    }
    execute_call_by_path(
        &callable,
        Some(&routine.routine_key),
        None,
        current_package_id,
        current_module_id,
        Vec::new(),
        Vec::new(),
        false,
        plan,
        scopes,
        state,
        host,
        false,
    )
    .map(Some)
}

pub(super) fn apply_assign(
    target: &ParsedAssignTarget,
    op: ParsedAssignOp,
    value: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<()> {
    let updated = if matches!(op, ParsedAssignOp::Assign) {
        value
    } else {
        let current = read_assign_target_value_runtime(
            target,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        apply_assignment_op(current, op, value)?
    };
    Ok(write_assign_target_value_runtime(
        target,
        updated,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?)
}

pub(super) fn execute_reclaim_statement_expr(
    expr: &ParsedExpr,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<()> {
    let token_expr = expr.clone();
    let reference = expect_reference(
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
        )?,
        "reclaim",
    )?;
    if reference.mode != RuntimeReferenceMode::Hold {
        return Err("`reclaim` expects an `&hold[...]` capability"
            .to_string()
            .into());
    }
    reclaim_held_target_local(scopes, &reference.target).map_err(RuntimeEvalSignal::from)?;
    reclaim_hold_capability_root_local(scopes, &token_expr).map_err(RuntimeEvalSignal::from)?;
    Ok(())
}

pub(super) fn run_scope_defers(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<()> {
    let deferred = scopes
        .last_mut()
        .ok_or_else(|| "runtime scope stack is empty".to_string())?
        .deferred
        .drain(..)
        .rev()
        .collect::<Vec<_>>();
    for deferred_action in deferred {
        match deferred_action {
            ParsedDeferAction::Expr(expr) => {
                let _ = eval_expr(
                    &expr,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
            }
            ParsedDeferAction::Reclaim(expr) => {
                execute_reclaim_statement_expr(
                    &expr,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn execute_cleanup_footers(
    frame: RuntimeCleanupFooterFrame,
    plan: &RuntimePackagePlan,
    scopes: &mut Vec<RuntimeScope>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<()> {
    let current_package_id = frame.current_package_id.clone();
    let current_module_id = frame.current_module_id.clone();
    let cleanup_footers_by_binding_id = frame
        .cleanup_footers
        .iter()
        .filter(|cleanup_footer| cleanup_footer.binding_id != 0)
        .map(|cleanup_footer| (cleanup_footer.binding_id, cleanup_footer))
        .collect::<BTreeMap<_, _>>();
    let cleanup_footers_by_subject = frame
        .cleanup_footers
        .iter()
        .filter(|cleanup_footer| cleanup_footer.binding_id == 0)
        .map(|cleanup_footer| (cleanup_footer.subject.clone(), cleanup_footer))
        .collect::<BTreeMap<_, _>>();
    for subject in frame.activations.into_iter().rev() {
        let cleanup_footer = cleanup_footers_by_binding_id
            .get(&subject.binding_id)
            .copied()
            .or_else(|| cleanup_footers_by_subject.get(&subject.subject).copied());
        let Some(cleanup_footer) = cleanup_footer else {
            continue;
        };
        if cleanup_footer.kind != "cleanup" {
            return Err(
                format!("unsupported cleanup footer kind `{}`", cleanup_footer.kind).into(),
            );
        }
        let callable = if cleanup_footer.handler_path.is_empty() {
            vec!["cleanup".to_string()]
        } else {
            resolve_cleanup_footer_handler_callable_path(
                plan,
                &current_package_id,
                &current_module_id,
                &cleanup_footer.handler_path,
            )
        };
        let outcome = execute_call_by_path(
            &callable,
            cleanup_footer.resolved_routine.as_deref(),
            None,
            &current_package_id,
            &current_module_id,
            Vec::new(),
            vec![RuntimeCallArg {
                name: None,
                value: subject.value,
                source_expr: ParsedExpr::Path(vec![cleanup_footer.subject.clone()]),
            }],
            cleanup_footer.handler_path.is_empty(),
            plan,
            scopes,
            state,
            host,
            false,
        )?;
        expect_cleanup_outcome(outcome).map_err(RuntimeEvalSignal::Message)?;
    }
    Ok(())
}

pub(super) fn finish_runtime_cleanup_footers<T>(
    result: RuntimeEvalResult<T>,
    frame: Option<RuntimeCleanupFooterFrame>,
    plan: &RuntimePackagePlan,
    scopes: &mut Vec<RuntimeScope>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<T> {
    if let Some(frame) = frame
        && !matches!(result, Err(RuntimeEvalSignal::Message(_)))
    {
        execute_cleanup_footers(frame, plan, scopes, state, host)?;
    }
    result
}

pub(super) fn execute_scoped_block(
    statements: &[ParsedStmt],
    cleanup_footers: &[ParsedCleanupFooter],
    scopes: &mut Vec<RuntimeScope>,
    scope: RuntimeScope,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<FlowSignal> {
    push_runtime_cleanup_footer_frame(
        state,
        cleanup_footers,
        scopes,
        current_package_id,
        current_module_id,
    );
    scopes.push(scope);
    activate_attached_runtime_owners_for_current_scope(
        scopes,
        &[],
        plan,
        current_package_id,
        state,
    )?;
    activate_attached_runtime_objects_for_current_scope(
        scopes,
        &[],
        plan,
        current_package_id,
        state,
    )?;
    let result = execute_statements(
        statements,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    );
    let defer_result = run_scope_defers(
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    );
    let exited_scope = scopes
        .pop()
        .ok_or_else(|| "runtime scope stack is empty".to_string())?;
    validate_scope_hold_tokens(&exited_scope).map_err(RuntimeEvalSignal::from)?;
    let result = match defer_result {
        Ok(()) => result,
        Err(RuntimeEvalSignal::OwnerExit {
            owner_key,
            exit_name,
        }) => Ok(FlowSignal::OwnerExit {
            owner_key,
            exit_name,
        }),
        Err(other) => return Err(other),
    };
    let frame = if cleanup_footers.is_empty() {
        None
    } else {
        pop_runtime_cleanup_footer_frame(state)
    };
    let result = match finish_runtime_cleanup_footers(result, frame, plan, scopes, state, host) {
        Ok(signal) => signal,
        Err(RuntimeEvalSignal::OwnerExit {
            owner_key,
            exit_name,
        }) => FlowSignal::OwnerExit {
            owner_key,
            exit_name,
        },
        Err(other) => return Err(other),
    };
    let result = match result {
        FlowSignal::OwnerExit {
            owner_key,
            exit_name,
        } if exited_scope
            .activated_owner_keys
            .iter()
            .any(|active| active == &owner_key) =>
        {
            apply_explicit_owner_exit(plan, state, &owner_key, &exit_name, Some(scopes))
                .map_err(RuntimeEvalSignal::from)?;
            FlowSignal::Next
        }
        other => {
            if let Err(err) = evaluate_owner_exit_checkpoints(
                &exited_scope.activated_owner_keys,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
                Some(scopes),
            ) {
                return Err(err.into());
            }
            other
        }
    };
    release_scope_owner_activations(state, &exited_scope.activated_owner_keys);
    Ok(result)
}

pub(super) fn execute_statements(
    statements: &[ParsedStmt],
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<FlowSignal> {
    for statement in statements {
        let signal = match statement {
            ParsedStmt::Let {
                binding_id,
                mutable,
                name,
                value,
            } => {
                let value = eval_expr(
                    value,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let current_scope_depth = scopes.len().saturating_sub(1);
                let current_scope = scopes
                    .last_mut()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                insert_runtime_local(
                    state,
                    current_scope_depth,
                    current_scope,
                    *binding_id,
                    name.clone(),
                    *mutable,
                    value,
                );
                FlowSignal::Next
            }
            ParsedStmt::Expr {
                expr,
                cleanup_footers,
            } => {
                push_runtime_cleanup_footer_frame(
                    state,
                    cleanup_footers,
                    scopes,
                    current_package_id,
                    current_module_id,
                );
                finish_runtime_cleanup_footers(
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
                    .map(|_| FlowSignal::Next),
                    if cleanup_footers.is_empty() {
                        None
                    } else {
                        pop_runtime_cleanup_footer_frame(state)
                    },
                    plan,
                    scopes,
                    state,
                    host,
                )?;
                FlowSignal::Next
            }
            ParsedStmt::Reclaim(expr) => {
                execute_reclaim_statement_expr(
                    expr,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                FlowSignal::Next
            }
            ParsedStmt::ReturnVoid => FlowSignal::Return(RuntimeValue::Unit),
            ParsedStmt::ReturnValue { value } => FlowSignal::Return(eval_expr(
                value,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?),
            ParsedStmt::If {
                condition,
                then_branch,
                else_branch,
                cleanup_footers,
                availability,
            } => {
                let condition = read_runtime_value_if_ref(
                    eval_expr(
                        condition,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                if expect_bool(condition, "if condition")? {
                    let mut scope = RuntimeScope::default();
                    apply_runtime_availability_attachments(&mut scope, availability);
                    execute_scoped_block(
                        then_branch,
                        cleanup_footers,
                        scopes,
                        scope,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?
                } else if else_branch.is_empty() {
                    FlowSignal::Next
                } else {
                    let mut scope = RuntimeScope::default();
                    apply_runtime_availability_attachments(&mut scope, availability);
                    execute_scoped_block(
                        else_branch,
                        cleanup_footers,
                        scopes,
                        scope,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?
                }
            }
            ParsedStmt::While {
                condition,
                body,
                cleanup_footers,
                availability,
            } => loop {
                let condition = read_runtime_value_if_ref(
                    eval_expr(
                        condition,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                if !expect_bool(condition, "while condition")? {
                    break FlowSignal::Next;
                }
                let mut scope = RuntimeScope::default();
                apply_runtime_availability_attachments(&mut scope, availability);
                match execute_scoped_block(
                    body,
                    cleanup_footers,
                    scopes,
                    scope,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )? {
                    FlowSignal::Next | FlowSignal::Continue => {}
                    FlowSignal::Break => break FlowSignal::Next,
                    FlowSignal::Return(value) => break FlowSignal::Return(value),
                    FlowSignal::OwnerExit {
                        owner_key,
                        exit_name,
                    } => {
                        break FlowSignal::OwnerExit {
                            owner_key,
                            exit_name,
                        };
                    }
                }
            },
            ParsedStmt::For {
                binding_id,
                binding,
                iterable,
                body,
                cleanup_footers,
                availability,
            } => {
                let iterable = read_runtime_value_if_ref(
                    eval_expr(
                        iterable,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let values =
                    into_iterable_values(force_runtime_value(iterable, plan, state, host)?)?;
                let mut loop_signal = FlowSignal::Next;
                for value in values {
                    let mut scope = RuntimeScope::default();
                    apply_runtime_availability_attachments(&mut scope, availability);
                    insert_runtime_local(
                        state,
                        scopes.len(),
                        &mut scope,
                        *binding_id,
                        binding.clone(),
                        false,
                        value,
                    );
                    match execute_scoped_block(
                        body,
                        cleanup_footers,
                        scopes,
                        scope,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )? {
                        FlowSignal::Next | FlowSignal::Continue => {}
                        FlowSignal::Break => {
                            loop_signal = FlowSignal::Next;
                            break;
                        }
                        FlowSignal::Return(value) => {
                            loop_signal = FlowSignal::Return(value);
                            break;
                        }
                        FlowSignal::OwnerExit {
                            owner_key,
                            exit_name,
                        } => {
                            loop_signal = FlowSignal::OwnerExit {
                                owner_key,
                                exit_name,
                            };
                            break;
                        }
                    }
                }
                loop_signal
            }
            ParsedStmt::Defer(action) => {
                scopes
                    .last_mut()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?
                    .deferred
                    .push(action.clone());
                FlowSignal::Next
            }
            ParsedStmt::ActivateOwner { .. } => {
                let ParsedStmt::ActivateOwner {
                    owner_path,
                    owner_local_name: _,
                    binding,
                    object_binding_ids,
                    context,
                } = statement
                else {
                    unreachable!();
                };
                let owner_package_id =
                    resolve_visible_package_id_for_path(plan, current_package_id, owner_path)
                        .ok_or_else(|| {
                            format!(
                                "runtime owner activation `{}` resolves to an unknown owner",
                                owner_path.join(".")
                            )
                        })?;
                let owner = lookup_runtime_owner_plan(plan, owner_package_id, owner_path)
                    .ok_or_else(|| {
                        format!(
                            "runtime owner activation `{}` resolves to an unknown owner",
                            owner_path.join(".")
                        )
                    })?;
                let owner_key = owner_state_key(owner_package_id, owner_path);
                let had_prior_active_state = state
                    .owners
                    .get(&owner_key)
                    .map(|owner_state| owner_state.active_bindings > 0)
                    .unwrap_or(false);
                let context_value = context
                    .as_ref()
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
                    .transpose()?;
                if had_prior_active_state {
                    evaluate_owner_exit_checkpoints(
                        std::slice::from_ref(&owner_key),
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                        Some(scopes),
                    )
                    .map_err(RuntimeEvalSignal::from)?;
                }
                let owner_state = state.owners.entry(owner_key.clone()).or_default();
                if owner_state.active_bindings == 0 {
                    owner_state.activation_context = context_value;
                    owner_state.pending_init.clear();
                    owner_state.pending_resume = owner_state.objects.keys().cloned().collect();
                }
                activate_owner_scope_binding(
                    scopes,
                    state,
                    owner,
                    &owner_key,
                    binding.as_deref(),
                    object_binding_ids,
                )
                .map_err(RuntimeEvalSignal::from)?;
                FlowSignal::Next
            }
            ParsedStmt::Recycle {
                default_modifier,
                lines,
            } => {
                let mut signal = FlowSignal::Next;
                for line in lines {
                    let gate = match &line.kind {
                        ParsedRecycleLineKind::Expr { gate }
                        | ParsedRecycleLineKind::Let { gate, .. }
                        | ParsedRecycleLineKind::Assign { gate, .. } => gate,
                    };
                    let value = eval_expr(
                        gate,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?;
                    match runtime_gate_outcome(value, "recycle gate")? {
                        Ok(success_payload) => match (&line.kind, success_payload) {
                            (ParsedRecycleLineKind::Let { mutable, name, .. }, Some(payload)) => {
                                let current_scope_depth = scopes.len().saturating_sub(1);
                                let current_scope = scopes
                                    .last_mut()
                                    .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                                insert_runtime_local(
                                    state,
                                    current_scope_depth,
                                    current_scope,
                                    0,
                                    name.clone(),
                                    *mutable,
                                    payload,
                                );
                            }
                            (ParsedRecycleLineKind::Assign { name, .. }, Some(payload)) => {
                                apply_assign(
                                    &ParsedAssignTarget::Name(name.clone()),
                                    ParsedAssignOp::Assign,
                                    payload,
                                    plan,
                                    current_package_id,
                                    current_module_id,
                                    scopes,
                                    aliases,
                                    type_bindings,
                                    state,
                                    host,
                                )?;
                            }
                            (ParsedRecycleLineKind::Expr { .. }, _) => {}
                            _ => {
                                return Err(
                                    "payload-bearing recycle line requires Option/Result gate"
                                        .to_string()
                                        .into(),
                                );
                            }
                        },
                        Err(failure) => {
                            let modifier = line
                                .modifier
                                .as_ref()
                                .or(default_modifier.as_ref())
                                .ok_or_else(|| {
                                    "recycle failure requires an explicit exit modifier".to_string()
                                })?;
                            signal = match modifier.kind.as_str() {
                                "return" => {
                                    if let Some(payload) = &modifier.payload {
                                        FlowSignal::Return(eval_expr(
                                            payload,
                                            plan,
                                            current_package_id,
                                            current_module_id,
                                            scopes,
                                            aliases,
                                            type_bindings,
                                            state,
                                            host,
                                        )?)
                                    } else if let RuntimeValue::Variant { name, .. } = &failure {
                                        if variant_name_matches(name, "Result.Err") {
                                            FlowSignal::Return(failure)
                                        } else {
                                            return Err(
                                                "bare `-return` in recycle requires Result failure"
                                                    .to_string()
                                                    .into(),
                                            );
                                        }
                                    } else {
                                        return Err(
                                            "bare `-return` in recycle requires Result failure"
                                                .to_string()
                                                .into(),
                                        );
                                    }
                                }
                                "break" => FlowSignal::Break,
                                "continue" => FlowSignal::Continue,
                                other => {
                                    let owner_key =
                                        resolve_named_owner_exit_target(plan, scopes, other)
                                            .map_err(RuntimeEvalSignal::from)?;
                                    FlowSignal::OwnerExit {
                                        owner_key,
                                        exit_name: other.to_string(),
                                    }
                                }
                            };
                            break;
                        }
                    }
                }
                signal
            }
            ParsedStmt::Bind {
                default_modifier,
                lines,
            } => {
                let mut signal = FlowSignal::Next;
                for line in lines {
                    let modifier = line.modifier.as_ref().or(default_modifier.as_ref());
                    match &line.kind {
                        ParsedBindLineKind::Require { expr } => {
                            let value = read_runtime_value_if_ref(
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
                                )?,
                                scopes,
                                plan,
                                current_package_id,
                                current_module_id,
                                aliases,
                                type_bindings,
                                state,
                                host,
                            )?;
                            if !expect_bool(value, "bind require")? {
                                let modifier = modifier.ok_or_else(|| {
                                    "bind require failure requires an explicit modifier".to_string()
                                })?;
                                signal = match modifier.kind.as_str() {
                                    "return" => {
                                        let value = eval_headed_modifier_payload(
                                            modifier.payload.as_ref(),
                                            plan,
                                            current_package_id,
                                            current_module_id,
                                            scopes,
                                            aliases,
                                            type_bindings,
                                            state,
                                            host,
                                        )?
                                        .unwrap_or(RuntimeValue::Unit);
                                        FlowSignal::Return(value)
                                    }
                                    "break" => FlowSignal::Break,
                                    "continue" => FlowSignal::Continue,
                                    other => {
                                        return Err(format!(
                                            "unsupported bind require modifier `{other}`"
                                        )
                                        .into());
                                    }
                                };
                                break;
                            }
                        }
                        ParsedBindLineKind::Let {
                            mutable,
                            name,
                            gate,
                        } => {
                            let gate_value = eval_expr(
                                gate,
                                plan,
                                current_package_id,
                                current_module_id,
                                scopes,
                                aliases,
                                type_bindings,
                                state,
                                host,
                            )?;
                            match runtime_gate_outcome(gate_value, "bind gate")? {
                                Ok(Some(payload)) => {
                                    let current_scope_depth = scopes.len().saturating_sub(1);
                                    let current_scope = scopes.last_mut().ok_or_else(|| {
                                        "runtime scope stack is empty".to_string()
                                    })?;
                                    insert_runtime_local(
                                        state,
                                        current_scope_depth,
                                        current_scope,
                                        0,
                                        name.clone(),
                                        *mutable,
                                        payload,
                                    );
                                }
                                Ok(None) => {
                                    return Err("bind payload lines require Option/Result gates"
                                        .to_string()
                                        .into());
                                }
                                Err(failure) => {
                                    let modifier = modifier.ok_or_else(|| {
                                        "bind failure requires an explicit modifier".to_string()
                                    })?;
                                    match modifier.kind.as_str() {
                                        "return" => {
                                            signal = if let Some(payload) = &modifier.payload {
                                                FlowSignal::Return(eval_expr(
                                                    payload,
                                                    plan,
                                                    current_package_id,
                                                    current_module_id,
                                                    scopes,
                                                    aliases,
                                                    type_bindings,
                                                    state,
                                                    host,
                                                )?)
                                            } else if let RuntimeValue::Variant { name, .. } =
                                                &failure
                                            {
                                                if variant_name_matches(name, "Result.Err") {
                                                    FlowSignal::Return(failure)
                                                } else {
                                                    return Err("bare `-return` in bind requires Result failure".to_string().into());
                                                }
                                            } else {
                                                return Err("bare `-return` in bind requires Result failure".to_string().into());
                                            };
                                            break;
                                        }
                                        "default" => {
                                            let fallback = eval_headed_modifier_payload(
                                                modifier.payload.as_ref(),
                                                plan,
                                                current_package_id,
                                                current_module_id,
                                                scopes,
                                                aliases,
                                                type_bindings,
                                                state,
                                                host,
                                            )?
                                            .ok_or_else(|| {
                                                "bind `default` requires a payload".to_string()
                                            })?;
                                            let current_scope_depth =
                                                scopes.len().saturating_sub(1);
                                            let current_scope =
                                                scopes.last_mut().ok_or_else(|| {
                                                    "runtime scope stack is empty".to_string()
                                                })?;
                                            insert_runtime_local(
                                                state,
                                                current_scope_depth,
                                                current_scope,
                                                0,
                                                name.clone(),
                                                *mutable,
                                                fallback,
                                            );
                                        }
                                        other => {
                                            return Err(format!(
                                                "unsupported bind let modifier `{other}`"
                                            )
                                            .into());
                                        }
                                    }
                                }
                            }
                        }
                        ParsedBindLineKind::Assign { name, gate } => {
                            let gate_value = eval_expr(
                                gate,
                                plan,
                                current_package_id,
                                current_module_id,
                                scopes,
                                aliases,
                                type_bindings,
                                state,
                                host,
                            )?;
                            match runtime_gate_outcome(gate_value, "bind gate")? {
                                Ok(Some(payload)) => {
                                    apply_assign(
                                        &ParsedAssignTarget::Name(name.clone()),
                                        ParsedAssignOp::Assign,
                                        payload,
                                        plan,
                                        current_package_id,
                                        current_module_id,
                                        scopes,
                                        aliases,
                                        type_bindings,
                                        state,
                                        host,
                                    )?;
                                }
                                Ok(None) => {
                                    return Err("bind payload lines require Option/Result gates"
                                        .to_string()
                                        .into());
                                }
                                Err(failure) => {
                                    let modifier = modifier.ok_or_else(|| {
                                        "bind failure requires an explicit modifier".to_string()
                                    })?;
                                    match modifier.kind.as_str() {
                                        "return" => {
                                            signal = if let Some(payload) = &modifier.payload {
                                                FlowSignal::Return(eval_expr(
                                                    payload,
                                                    plan,
                                                    current_package_id,
                                                    current_module_id,
                                                    scopes,
                                                    aliases,
                                                    type_bindings,
                                                    state,
                                                    host,
                                                )?)
                                            } else if let RuntimeValue::Variant { name, .. } =
                                                &failure
                                            {
                                                if variant_name_matches(name, "Result.Err") {
                                                    FlowSignal::Return(failure)
                                                } else {
                                                    return Err("bare `-return` in bind requires Result failure".to_string().into());
                                                }
                                            } else {
                                                return Err("bare `-return` in bind requires Result failure".to_string().into());
                                            };
                                            break;
                                        }
                                        "preserve" => {}
                                        "replace" => {
                                            let fallback = eval_headed_modifier_payload(
                                                modifier.payload.as_ref(),
                                                plan,
                                                current_package_id,
                                                current_module_id,
                                                scopes,
                                                aliases,
                                                type_bindings,
                                                state,
                                                host,
                                            )?
                                            .ok_or_else(|| {
                                                "bind `replace` requires a payload".to_string()
                                            })?;
                                            apply_assign(
                                                &ParsedAssignTarget::Name(name.clone()),
                                                ParsedAssignOp::Assign,
                                                fallback,
                                                plan,
                                                current_package_id,
                                                current_module_id,
                                                scopes,
                                                aliases,
                                                type_bindings,
                                                state,
                                                host,
                                            )?;
                                        }
                                        other => {
                                            return Err(format!(
                                                "unsupported bind assign modifier `{other}`"
                                            )
                                            .into());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                signal
            }
            ParsedStmt::Construct(region) => {
                let value = eval_expr(
                    &ParsedExpr::ConstructRegion(Box::new(region.clone())),
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                match &region.destination {
                    Some(ParsedConstructDestination::Deliver { name }) => {
                        let current_scope_depth = scopes.len().saturating_sub(1);
                        let current_scope = scopes
                            .last_mut()
                            .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                        insert_runtime_local(
                            state,
                            current_scope_depth,
                            current_scope,
                            0,
                            name.clone(),
                            false,
                            value,
                        );
                    }
                    Some(ParsedConstructDestination::Place { target }) => {
                        apply_assign(
                            target,
                            ParsedAssignOp::Assign,
                            value,
                            plan,
                            current_package_id,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?;
                    }
                    None => {}
                }
                FlowSignal::Next
            }
            ParsedStmt::Record(region) => {
                let value = eval_expr(
                    &ParsedExpr::RecordRegion(Box::new(region.clone())),
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                match &region.destination {
                    Some(ParsedConstructDestination::Deliver { name }) => {
                        let current_scope_depth = scopes.len().saturating_sub(1);
                        let current_scope = scopes
                            .last_mut()
                            .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                        insert_runtime_local(
                            state,
                            current_scope_depth,
                            current_scope,
                            0,
                            name.clone(),
                            false,
                            value,
                        );
                    }
                    Some(ParsedConstructDestination::Place { target }) => {
                        apply_assign(
                            target,
                            ParsedAssignOp::Assign,
                            value,
                            plan,
                            current_package_id,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?;
                    }
                    None => {}
                }
                FlowSignal::Next
            }
            ParsedStmt::Array(region) => {
                let value = eval_expr(
                    &ParsedExpr::ArrayRegion(Box::new(region.clone())),
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                match &region.destination {
                    Some(ParsedConstructDestination::Deliver { name }) => {
                        let current_scope_depth = scopes.len().saturating_sub(1);
                        let current_scope = scopes
                            .last_mut()
                            .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                        insert_runtime_local(
                            state,
                            current_scope_depth,
                            current_scope,
                            0,
                            name.clone(),
                            false,
                            value,
                        );
                    }
                    Some(ParsedConstructDestination::Place { target }) => {
                        apply_assign(
                            target,
                            ParsedAssignOp::Assign,
                            value,
                            plan,
                            current_package_id,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?;
                    }
                    None => {}
                }
                FlowSignal::Next
            }
            ParsedStmt::MemorySpec(spec) => {
                let owner_keys = collect_active_owner_keys_from_scopes(scopes);
                let current_scope = scopes
                    .last_mut()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                current_scope.memory_specs.insert(
                    spec.name.clone(),
                    RuntimeMemorySpecState {
                        spec: spec.clone(),
                        handle: None,
                        handle_policy: None,
                        owner_keys,
                    },
                );
                FlowSignal::Next
            }
            ParsedStmt::Break => FlowSignal::Break,
            ParsedStmt::Continue => FlowSignal::Continue,
            ParsedStmt::Assign { target, op, value } => {
                let value = eval_expr(
                    value,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                apply_assign(
                    target,
                    *op,
                    value,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                FlowSignal::Next
            }
        };
        if signal != FlowSignal::Next {
            return Ok(signal);
        }
    }
    Ok(FlowSignal::Next)
}

pub(super) fn execute_routine_call_with_state(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    type_args: Vec<String>,
    mut args: Vec<RuntimeValue>,
    inherited_active_owner_keys: &[String],
    caller_scopes: Option<&mut Vec<RuntimeScope>>,
    current_package_id: Option<&str>,
    current_module_id: Option<&str>,
    aliases: Option<&BTreeMap<String, Vec<String>>>,
    type_bindings: Option<&RuntimeTypeBindings>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    allow_async: bool,
) -> RuntimeEvalResult<RoutineExecutionOutcome> {
    let routine = plan
        .routines
        .get(routine_index)
        .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
    push_runtime_call_frame(state, &routine.module_id, &routine.symbol_name)?;
    let execution_result = grow(16 * 1024 * 1024, || {
        (|| -> RuntimeEvalResult<RoutineExecutionOutcome> {
            if let Some(intrinsic_impl) = &routine.intrinsic_impl {
                let intrinsic =
                    resolve_runtime_intrinsic_impl(intrinsic_impl).ok_or_else(|| {
                        format!(
                            "unsupported runtime intrinsic `{intrinsic_impl}` for `{}`",
                            routine.symbol_name
                        )
                    })?;
                let value = execute_runtime_intrinsic(
                    intrinsic,
                    &type_args,
                    &mut args,
                    plan,
                    caller_scopes,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                return Ok(RoutineExecutionOutcome {
                    value,
                    final_args: args,
                    skip_write_back_edit_indices: BTreeSet::new(),
                    control: None,
                });
            }
            if let Some(native_impl) = &routine.native_impl {
                return execute_runtime_native_binding_import(
                    plan,
                    routine,
                    native_impl,
                    &args,
                    caller_scopes.ok_or_else(|| {
                        RuntimeEvalSignal::Message(
                            "native binding import missing caller scopes".to_string(),
                        )
                    })?,
                    current_package_id.ok_or_else(|| {
                        RuntimeEvalSignal::Message(
                            "native binding import missing current package id".to_string(),
                        )
                    })?,
                    current_module_id.ok_or_else(|| {
                        RuntimeEvalSignal::Message(
                            "native binding import missing current module id".to_string(),
                        )
                    })?,
                    aliases.ok_or_else(|| {
                        RuntimeEvalSignal::Message(
                            "native binding import missing alias map".to_string(),
                        )
                    })?,
                    type_bindings.ok_or_else(|| {
                        RuntimeEvalSignal::Message(
                            "native binding import missing type bindings".to_string(),
                        )
                    })?,
                    state,
                    host,
                )
                .map_err(RuntimeEvalSignal::from);
            }
            if routine.is_async && !allow_async {
                return Err(format!(
                    "async routine `{}` is not executable in the current runtime lane",
                    routine.symbol_name
                )
                .into());
            }
            if args.len() != routine.params.len() {
                return Err(format!(
                    "routine `{}` expected {} arguments, got {}",
                    routine.symbol_name,
                    routine.params.len(),
                    args.len()
                )
                .into());
            }
            let aliases = plan
                .module_aliases
                .get(&module_alias_scope_key(
                    &routine.package_id,
                    &routine.module_id,
                ))
                .cloned()
                .unwrap_or_default();
            let resolved_type_args = resolve_runtime_routine_type_args(
                &routine.symbol_name,
                &routine.type_params,
                type_args,
            )
            .map_err(RuntimeEvalSignal::from)?;
            let type_bindings = routine
                .type_params
                .iter()
                .cloned()
                .zip(resolved_type_args)
                .collect::<RuntimeTypeBindings>();
            let entered_async_context = routine.is_async;
            if entered_async_context {
                state.async_context_depth += 1;
            }
            let outcome = (|| -> RuntimeEvalResult<RoutineExecutionOutcome> {
                let mut initial_scope = RuntimeScope {
                    inherited_active_owner_keys: inherited_active_owner_keys.to_vec(),
                    ..Default::default()
                };
                apply_runtime_availability_attachments(&mut initial_scope, &routine.availability);
                push_runtime_cleanup_footer_frame(
                    state,
                    &routine.cleanup_footers,
                    &[],
                    &routine.package_id,
                    &routine.module_id,
                );
                for (param, value) in routine.params.iter().zip(args) {
                    insert_runtime_local(
                        state,
                        0,
                        &mut initial_scope,
                        param.binding_id,
                        param.name.clone(),
                        param.mode.as_deref() == Some("edit"),
                        value,
                    );
                }
                let mut scopes = Vec::new();
                scopes.push(initial_scope);
                activate_attached_runtime_owners_for_current_scope(
                    &mut scopes,
                    inherited_active_owner_keys,
                    plan,
                    &routine.package_id,
                    state,
                )?;
                activate_attached_runtime_objects_for_current_scope(
                    &mut scopes,
                    inherited_active_owner_keys,
                    plan,
                    &routine.package_id,
                    state,
                )?;
                let result = execute_statements(
                    &routine.statements,
                    &mut scopes,
                    plan,
                    &routine.package_id,
                    &routine.module_id,
                    &aliases,
                    &type_bindings,
                    state,
                    host,
                );
                let defer_result = run_scope_defers(
                    plan,
                    &routine.package_id,
                    &routine.module_id,
                    &mut scopes,
                    &aliases,
                    &type_bindings,
                    state,
                    host,
                );
                let routine_cleanup_footer_frame = pop_runtime_cleanup_footer_frame(state);
                let final_scope = scopes
                    .pop()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                validate_scope_hold_tokens(&final_scope).map_err(RuntimeEvalSignal::from)?;
                match defer_result {
                    Ok(()) => {}
                    Err(RuntimeEvalSignal::Message(message)) => return Err(message.into()),
                    Err(RuntimeEvalSignal::Return(value)) => {
                        if let Some(frame) = routine_cleanup_footer_frame.clone() {
                            execute_cleanup_footers(frame, plan, &mut scopes, state, host)
                                .map_err(runtime_eval_message)?;
                        }
                        let mut materialize_scopes = scopes.clone();
                        materialize_scopes.push(final_scope.clone());
                        let value = materialize_runtime_return_value(
                            value,
                            routine.return_type.as_ref(),
                            plan,
                            &routine.package_id,
                            &routine.module_id,
                            &mut materialize_scopes,
                            &aliases,
                            &type_bindings,
                            state,
                            host,
                        )
                        .map_err(RuntimeEvalSignal::from)?;
                        evaluate_owner_exit_checkpoints(
                            &final_scope.activated_owner_keys,
                            plan,
                            &routine.package_id,
                            &routine.module_id,
                            &aliases,
                            &type_bindings,
                            state,
                            host,
                            None,
                        )?;
                        release_scope_owner_activations(state, &final_scope.activated_owner_keys);
                        let final_args = routine
                            .params
                            .iter()
                            .map(|param| {
                                final_scope
                                    .locals
                                    .get(&param.name)
                                    .map(|local| local.value.clone())
                                    .ok_or_else(|| {
                                        format!(
                                            "runtime routine `{}` lost bound parameter `{}`",
                                            routine.symbol_name, param.name
                                        )
                                    })
                            })
                            .collect::<Result<Vec<_>, String>>()
                            .map_err(RuntimeEvalSignal::from)?;
                        return Ok(RoutineExecutionOutcome {
                            value,
                            final_args,
                            skip_write_back_edit_indices: BTreeSet::new(),
                            control: None,
                        });
                    }
                    Err(RuntimeEvalSignal::OwnerExit {
                        owner_key,
                        exit_name,
                    }) => {
                        if let Some(frame) = routine_cleanup_footer_frame.clone() {
                            execute_cleanup_footers(frame, plan, &mut scopes, state, host)
                                .map_err(runtime_eval_message)?;
                        }
                        evaluate_owner_exit_checkpoints(
                            &final_scope.activated_owner_keys,
                            plan,
                            &routine.package_id,
                            &routine.module_id,
                            &aliases,
                            &type_bindings,
                            state,
                            host,
                            None,
                        )
                        .map_err(RuntimeEvalSignal::from)?;
                        release_scope_owner_activations(state, &final_scope.activated_owner_keys);
                        let final_args = routine
                            .params
                            .iter()
                            .map(|param| {
                                final_scope
                                    .locals
                                    .get(&param.name)
                                    .map(|local| local.value.clone())
                                    .ok_or_else(|| {
                                        format!(
                                            "runtime routine `{}` lost bound parameter `{}`",
                                            routine.symbol_name, param.name
                                        )
                                    })
                            })
                            .collect::<Result<Vec<_>, String>>()
                            .map_err(RuntimeEvalSignal::from)?;
                        return Ok(RoutineExecutionOutcome {
                            value: RuntimeValue::Unit,
                            final_args,
                            skip_write_back_edit_indices: BTreeSet::new(),
                            control: Some(FlowSignal::OwnerExit {
                                owner_key,
                                exit_name,
                            }),
                        });
                    }
                }
                if let Some(frame) = routine_cleanup_footer_frame.clone()
                    && !matches!(result, Err(RuntimeEvalSignal::Message(_)))
                {
                    execute_cleanup_footers(frame, plan, &mut scopes, state, host)
                        .map_err(runtime_eval_message)?;
                }
                let result = match result {
                    Ok(signal) => signal,
                    Err(RuntimeEvalSignal::Message(message)) => return Err(message.into()),
                    Err(RuntimeEvalSignal::Return(value)) => FlowSignal::Return(value),
                    Err(RuntimeEvalSignal::OwnerExit {
                        owner_key,
                        exit_name,
                    }) => FlowSignal::OwnerExit {
                        owner_key,
                        exit_name,
                    },
                };
                let result = match result {
                    FlowSignal::Return(value) => {
                        let mut materialize_scopes = scopes.clone();
                        materialize_scopes.push(final_scope.clone());
                        FlowSignal::Return(
                            materialize_runtime_return_value(
                                value,
                                routine.return_type.as_ref(),
                                plan,
                                &routine.package_id,
                                &routine.module_id,
                                &mut materialize_scopes,
                                &aliases,
                                &type_bindings,
                                state,
                                host,
                            )
                            .map_err(RuntimeEvalSignal::from)?,
                        )
                    }
                    other => other,
                };
                evaluate_owner_exit_checkpoints(
                    &final_scope.activated_owner_keys,
                    plan,
                    &routine.package_id,
                    &routine.module_id,
                    &aliases,
                    &type_bindings,
                    state,
                    host,
                    None,
                )?;
                release_scope_owner_activations(state, &final_scope.activated_owner_keys);
                let result = match result {
                    FlowSignal::OwnerExit {
                        owner_key,
                        exit_name,
                    } if final_scope
                        .activated_owner_keys
                        .iter()
                        .any(|active| active == &owner_key) =>
                    {
                        apply_explicit_owner_exit(plan, state, &owner_key, &exit_name, None)?;
                        FlowSignal::Next
                    }
                    other => other,
                };
                let value = match result {
                    FlowSignal::Next => RuntimeValue::Unit,
                    FlowSignal::Return(value) => value,
                    FlowSignal::Break => {
                        return Err("break escaped the top-level routine".to_string().into());
                    }
                    FlowSignal::Continue => {
                        return Err("continue escaped the top-level routine".to_string().into());
                    }
                    FlowSignal::OwnerExit {
                        owner_key,
                        exit_name,
                    } => {
                        let final_args = routine
                            .params
                            .iter()
                            .map(|param| {
                                final_scope
                                    .locals
                                    .get(&param.name)
                                    .map(|local| local.value.clone())
                                    .ok_or_else(|| {
                                        format!(
                                            "runtime routine `{}` lost bound parameter `{}`",
                                            routine.symbol_name, param.name
                                        )
                                    })
                            })
                            .collect::<Result<Vec<_>, String>>()
                            .map_err(RuntimeEvalSignal::from)?;
                        return Ok(RoutineExecutionOutcome {
                            value: RuntimeValue::Unit,
                            final_args,
                            skip_write_back_edit_indices: BTreeSet::new(),
                            control: Some(FlowSignal::OwnerExit {
                                owner_key,
                                exit_name,
                            }),
                        });
                    }
                };
                let final_args = routine
                    .params
                    .iter()
                    .map(|param| {
                        final_scope
                            .locals
                            .get(&param.name)
                            .map(|local| local.value.clone())
                            .ok_or_else(|| {
                                format!(
                                    "runtime routine `{}` lost bound parameter `{}`",
                                    routine.symbol_name, param.name
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, String>>()
                    .map_err(RuntimeEvalSignal::from)?;
                Ok(RoutineExecutionOutcome {
                    value,
                    final_args,
                    skip_write_back_edit_indices: BTreeSet::new(),
                    control: None,
                })
            })();
            if entered_async_context {
                state.async_context_depth = state.async_context_depth.saturating_sub(1);
            }
            outcome
        })()
    });
    pop_runtime_call_frame(state);
    execution_result
}

fn runtime_type_param_is_lifetime(param: &str) -> bool {
    param.starts_with('\'')
}

fn resolve_runtime_routine_type_args(
    routine_name: &str,
    declared_type_params: &[String],
    supplied_type_args: Vec<String>,
) -> Result<Vec<String>, String> {
    if declared_type_params.is_empty() {
        if supplied_type_args.is_empty() {
            return Ok(Vec::new());
        }
        return Err(format!(
            "routine `{routine_name}` expected 0 type arguments, got {}",
            supplied_type_args.len()
        ));
    }
    if supplied_type_args.is_empty() {
        return Ok(declared_type_params.to_vec());
    }
    if supplied_type_args.len() == declared_type_params.len() {
        return Ok(supplied_type_args);
    }
    let non_lifetime_type_param_count = declared_type_params
        .iter()
        .filter(|param| !runtime_type_param_is_lifetime(param))
        .count();
    if supplied_type_args.len() == non_lifetime_type_param_count {
        let mut supplied = supplied_type_args.into_iter();
        return Ok(declared_type_params
            .iter()
            .map(|param| {
                if runtime_type_param_is_lifetime(param) {
                    param.clone()
                } else {
                    supplied
                        .next()
                        .expect("non-lifetime type arg count should match supplied args")
                }
            })
            .collect());
    }
    Err(format!(
        "routine `{routine_name}` expected {} type arguments, got {}",
        declared_type_params.len(),
        supplied_type_args.len()
    ))
}

pub(super) fn execute_routine_with_state(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    type_args: Vec<String>,
    args: Vec<RuntimeValue>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let outcome = execute_routine_call_with_state(
        plan,
        routine_index,
        type_args,
        args,
        &collect_active_owner_keys_from_state(state),
        None,
        None,
        None,
        None,
        None,
        state,
        host,
        false,
    )
    .map_err(runtime_eval_message)?;
    if let Some(FlowSignal::OwnerExit {
        owner_key,
        exit_name,
    }) = outcome.control
    {
        return Err(format!(
            "owner exit `{exit_name}` for `{owner_key}` escaped the top-level runtime"
        ));
    }
    Ok(outcome.value)
}

pub fn validate_runtime_requirements_supported(
    plan: &RuntimePackagePlan,
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    for requirement in &plan.runtime_requirements {
        if !host.supports_runtime_requirement(requirement) {
            return Err(format!(
                "runtime core host does not support required capability `{requirement}`"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn execute_routine(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    args: Vec<RuntimeValue>,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    validate_runtime_requirements_supported(plan, host)?;
    reset_runtime_native_products_cache();
    let mut state = RuntimeExecutionState::default();
    let result =
        execute_routine_with_state(plan, routine_index, Vec::new(), args, &mut state, host);
    reset_runtime_native_products_cache();
    result
}

pub fn execute_main(
    plan: &RuntimePackagePlan,
    host: &mut dyn RuntimeCoreHost,
) -> Result<i32, String> {
    validate_runtime_requirements_supported(plan, host)?;
    let entry = plan
        .main_entrypoint()
        .ok_or_else(|| format!("package `{}` has no main entrypoint", plan.package_name))?;
    let routine_key = plan
        .routines
        .get(entry.routine_index)
        .map(|routine| routine.routine_key.clone())
        .ok_or_else(|| format!("invalid routine index `{}`", entry.routine_index))?;
    execute_entrypoint_routine(plan, &routine_key, host)
}

pub fn execute_entrypoint_routine(
    plan: &RuntimePackagePlan,
    routine_key: &str,
    host: &mut dyn RuntimeCoreHost,
) -> Result<i32, String> {
    validate_runtime_requirements_supported(plan, host)?;
    reset_runtime_native_products_cache();
    let entry = plan
        .entrypoints
        .iter()
        .find(|entry| {
            plan.routines
                .get(entry.routine_index)
                .is_some_and(|routine| routine.routine_key == routine_key)
        })
        .ok_or_else(|| {
            format!(
                "entrypoint routine `{routine_key}` is not present in package `{}`",
                plan.package_name
            )
        })?;
    let routine = plan
        .routines
        .get(entry.routine_index)
        .ok_or_else(|| format!("invalid routine index `{}`", entry.routine_index))?;
    validate_runtime_main_entry_contract(routine.params.len(), routine.return_type.as_ref())?;
    let mut state = RuntimeExecutionState::default();
    if state.next_scheduler_thread_id <= 0 {
        state.next_scheduler_thread_id = 1;
    }
    state.current_thread_id = 0;
    let value = execute_routine_call_with_state(
        plan,
        entry.routine_index,
        Vec::new(),
        Vec::new(),
        &[],
        None,
        None,
        None,
        None,
        None,
        &mut state,
        host,
        true,
    );
    let value = match value {
        Ok(value) => value,
        Err(err) => {
            reset_runtime_native_products_cache();
            return Err(runtime_eval_message(err));
        }
    };
    reset_runtime_native_products_cache();
    if let Some(FlowSignal::OwnerExit {
        owner_key,
        exit_name,
    }) = value.control.clone()
    {
        return Err(format!(
            "owner exit `{exit_name}` for `{owner_key}` escaped entrypoint `{routine_key}`"
        ));
    }
    let value = value.value;
    match value {
        RuntimeValue::Int(value) => i32::try_from(value)
            .map_err(|_| format!("main return value `{value}` does not fit in i32")),
        RuntimeValue::Unit => Ok(0),
        RuntimeValue::Float { .. }
        | RuntimeValue::Bool(_)
        | RuntimeValue::Str(_)
        | RuntimeValue::Bytes(_)
        | RuntimeValue::ByteBuffer(_)
        | RuntimeValue::Utf16(_)
        | RuntimeValue::Utf16Buffer(_)
        | RuntimeValue::Pair(_, _)
        | RuntimeValue::Array(_)
        | RuntimeValue::List(_)
        | RuntimeValue::Map(_)
        | RuntimeValue::Range { .. }
        | RuntimeValue::OwnerHandle(_)
        | RuntimeValue::Ref(_)
        | RuntimeValue::Opaque(_)
        | RuntimeValue::Record { .. }
        | RuntimeValue::Variant { .. } => {
            Err("main must return Int or Unit in the current runtime lane".to_string())
        }
    }
}

#[cfg(all(test, windows))]
mod raw_binding_tests {
    use super::*;
    use crate::native_product_loader::RuntimeBindingImportOutcome;
    use std::collections::BTreeMap;

    unsafe extern "system" fn test_free_owned_bytes(ptr: *mut u8, len: usize) {
        unsafe {
            arcana_cabi::free_owned_bytes(ptr, len);
        }
    }

    unsafe extern "system" fn test_free_owned_str(ptr: *mut u8, len: usize) {
        unsafe {
            arcana_cabi::free_owned_str(ptr, len);
        }
    }

    fn sample_binding_layouts() -> Vec<ArcanaCabiBindingLayout> {
        vec![
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.Rect".to_string(),
                size: 12,
                align: 4,
                kind: ArcanaCabiBindingLayoutKind::Struct {
                    fields: vec![
                        arcana_cabi::ArcanaCabiBindingLayoutField {
                            name: "left".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                            offset: 0,
                            bit_width: None,
                            bit_offset: None,
                        },
                        arcana_cabi::ArcanaCabiBindingLayoutField {
                            name: "top".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                            offset: 4,
                            bit_width: None,
                            bit_offset: None,
                        },
                        arcana_cabi::ArcanaCabiBindingLayoutField {
                            name: "flags".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U32),
                            offset: 8,
                            bit_width: Some(3),
                            bit_offset: Some(0),
                        },
                    ],
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.Words".to_string(),
                size: 8,
                align: 2,
                kind: ArcanaCabiBindingLayoutKind::Array {
                    element_type: ArcanaCabiBindingRawType::Scalar(
                        ArcanaCabiBindingScalarType::U16,
                    ),
                    len: 4,
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.Mode".to_string(),
                size: 4,
                align: 4,
                kind: ArcanaCabiBindingLayoutKind::Enum {
                    repr: ArcanaCabiBindingScalarType::U32,
                    variants: vec![
                        arcana_cabi::ArcanaCabiBindingLayoutEnumVariant {
                            name: "Idle".to_string(),
                            value: 0,
                        },
                        arcana_cabi::ArcanaCabiBindingLayoutEnumVariant {
                            name: "Busy".to_string(),
                            value: 1,
                        },
                    ],
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.ValueUnion".to_string(),
                size: 4,
                align: 4,
                kind: ArcanaCabiBindingLayoutKind::Union {
                    fields: vec![
                        arcana_cabi::ArcanaCabiBindingLayoutField {
                            name: "as_int".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                            offset: 0,
                            bit_width: None,
                            bit_offset: None,
                        },
                        arcana_cabi::ArcanaCabiBindingLayoutField {
                            name: "as_word".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U16),
                            offset: 0,
                            bit_width: None,
                            bit_offset: None,
                        },
                    ],
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.WindowProc".to_string(),
                size: std::mem::size_of::<usize>(),
                align: std::mem::size_of::<usize>(),
                kind: ArcanaCabiBindingLayoutKind::Callback {
                    abi: "system".to_string(),
                    params: vec![ArcanaCabiBindingRawType::Scalar(
                        ArcanaCabiBindingScalarType::I32,
                    )],
                    return_type: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.IUnknownVTable".to_string(),
                size: std::mem::size_of::<usize>() * 3,
                align: std::mem::size_of::<usize>(),
                kind: ArcanaCabiBindingLayoutKind::Struct {
                    fields: vec![arcana_cabi::ArcanaCabiBindingLayoutField {
                        name: "query_interface".to_string(),
                        ty: ArcanaCabiBindingRawType::FunctionPointer {
                            abi: "system".to_string(),
                            nullable: false,
                            params: vec![ArcanaCabiBindingRawType::Pointer {
                                mutable: false,
                                inner: Box::new(ArcanaCabiBindingRawType::Void),
                            }],
                            return_type: Box::new(ArcanaCabiBindingRawType::Scalar(
                                ArcanaCabiBindingScalarType::I32,
                            )),
                        },
                        offset: 0,
                        bit_width: None,
                        bit_offset: None,
                    }],
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.IUnknown".to_string(),
                size: std::mem::size_of::<usize>(),
                align: std::mem::size_of::<usize>(),
                kind: ArcanaCabiBindingLayoutKind::Interface {
                    iid: Some("00000000-0000-0000-C000-000000000046".to_string()),
                    vtable_layout_id: Some("hostapi.raw.IUnknownVTable".to_string()),
                },
            },
        ]
    }

    fn rect_value(left: i64, top: i64, flags: i64) -> RuntimeValue {
        RuntimeValue::Record {
            name: "hostapi.raw.Rect".to_string(),
            fields: BTreeMap::from([
                ("left".to_string(), RuntimeValue::Int(left)),
                ("top".to_string(), RuntimeValue::Int(top)),
                ("flags".to_string(), RuntimeValue::Int(flags)),
            ]),
        }
    }

    #[test]
    fn raw_binding_runtime_round_trips_layout_values() {
        let layouts = sample_binding_layouts();
        let mut state = RuntimeExecutionState::default();

        let rect = rect_value(12, -8, 5);
        let encoded = runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.Rect",
            rect.clone(),
        )
        .expect("struct output should encode");
        assert_eq!(
            encoded.tag().expect("tag should parse"),
            ArcanaCabiBindingValueTag::Layout
        );
        let decoded = runtime_value_from_binding_cabi_output(
            &layouts,
            "hostapi",
            "hostapi.raw.Rect",
            &encoded,
            &mut state,
            test_free_owned_bytes,
            test_free_owned_str,
            "struct result",
        )
        .expect("struct output should decode");
        assert_eq!(decoded, rect);

        let words = RuntimeValue::Array(vec![
            RuntimeValue::Int(1),
            RuntimeValue::Int(2),
            RuntimeValue::Int(3),
            RuntimeValue::Int(4),
        ]);
        let encoded = runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.Words",
            words.clone(),
        )
        .expect("array output should encode");
        let decoded = runtime_value_from_binding_cabi_output(
            &layouts,
            "hostapi",
            "hostapi.raw.Words",
            &encoded,
            &mut state,
            test_free_owned_bytes,
            test_free_owned_str,
            "array result",
        )
        .expect("array output should decode");
        assert_eq!(decoded, words);

        let union = RuntimeValue::Record {
            name: "hostapi.raw.ValueUnion".to_string(),
            fields: BTreeMap::from([("as_word".to_string(), RuntimeValue::Int(7))]),
        };
        let encoded = runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.ValueUnion",
            union.clone(),
        )
        .expect("union output should encode");
        let decoded = runtime_value_from_binding_cabi_output(
            &layouts,
            "hostapi",
            "hostapi.raw.ValueUnion",
            &encoded,
            &mut state,
            test_free_owned_bytes,
            test_free_owned_str,
            "union result",
        )
        .expect("union output should decode");
        let RuntimeValue::Record { fields, .. } = decoded else {
            panic!("decoded union should remain a record");
        };
        assert_eq!(fields.get("as_word"), Some(&RuntimeValue::Int(7)));

        let callback_value = RuntimeValue::Int(0x1234);
        let encoded = runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.WindowProc",
            callback_value.clone(),
        )
        .expect("callback pointer should encode");
        let decoded = runtime_value_from_binding_cabi_output(
            &layouts,
            "hostapi",
            "hostapi.raw.WindowProc",
            &encoded,
            &mut state,
            test_free_owned_bytes,
            test_free_owned_str,
            "callback result",
        )
        .expect("callback pointer should decode");
        assert_eq!(decoded, callback_value);

        let enum_value = RuntimeValue::Int(1);
        let encoded = runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.Mode",
            enum_value.clone(),
        )
        .expect("enum value should encode");
        let decoded = runtime_value_from_binding_cabi_output(
            &layouts,
            "hostapi",
            "hostapi.raw.Mode",
            &encoded,
            &mut state,
            test_free_owned_bytes,
            test_free_owned_str,
            "enum result",
        )
        .expect("enum value should decode");
        assert_eq!(decoded, enum_value);

        let interface_value = RuntimeValue::Int(0x2468);
        let encoded = runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.IUnknown",
            interface_value.clone(),
        )
        .expect("interface pointer should encode");
        let decoded = runtime_value_from_binding_cabi_output(
            &layouts,
            "hostapi",
            "hostapi.raw.IUnknown",
            &encoded,
            &mut state,
            test_free_owned_bytes,
            test_free_owned_str,
            "interface result",
        )
        .expect("interface pointer should decode");
        assert_eq!(decoded, interface_value);
    }

    #[test]
    fn raw_binding_runtime_rejects_multi_field_union_values() {
        let layouts = sample_binding_layouts();
        let err = match runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.ValueUnion",
            RuntimeValue::Record {
                name: "hostapi.raw.ValueUnion".to_string(),
                fields: BTreeMap::from([
                    ("as_int".to_string(), RuntimeValue::Int(1)),
                    ("as_word".to_string(), RuntimeValue::Int(2)),
                ]),
            },
        ) {
            Ok(_) => panic!("union output should require exactly one active field"),
            Err(err) => err,
        };
        assert!(err.contains("exactly one field"), "{err}");
    }

    #[test]
    fn raw_binding_runtime_applies_named_layout_write_backs() {
        let layouts = sample_binding_layouts();
        let mut state = RuntimeExecutionState::default();
        let dummy_plan = RuntimePackagePlan {
            package_id: "hostapi".to_string(),
            package_name: "hostapi".to_string(),
            root_module_id: "hostapi".to_string(),
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
            shackle_decls: Vec::new(),
            binding_layouts: layouts.clone(),
            owners: Vec::new(),
        };
        let mut host = BufferedHost::default();
        let updated = rect_value(32, 48, 3);
        let write_back = runtime_binding_output_from_runtime_value(
            &layouts,
            "hostapi",
            "hostapi.raw.Rect",
            updated.clone(),
        )
        .expect("write-back value should encode");
        let outcome = RuntimeBindingImportOutcome {
            result: ArcanaCabiBindingValueV1::default(),
            write_backs: vec![write_back],
            owned_bytes_free: test_free_owned_bytes,
            owned_str_free: test_free_owned_str,
        };
        let params = vec![arcana_ir::IrRoutineParam {
            binding_id: 0,
            mode: Some("edit".to_string()),
            name: "rect".to_string(),
            ty: parse_routine_type_text("hostapi.raw.Rect").expect("type should parse"),
        }];
        let (final_args, skip_write_back_edit_indices) = runtime_final_args_from_binding_import(
            &layouts,
            "hostapi",
            &params,
            &[rect_value(1, 2, 0)],
            &[rect_value(1, 2, 0)],
            &mut RuntimeBindingArgStorage::default(),
            &mut Vec::new(),
            &dummy_plan,
            "hostapi",
            "hostapi.tests",
            &BTreeMap::new(),
            &BTreeMap::new(),
            &mut state,
            &mut host,
            &outcome,
        )
        .expect("named layout write-back should decode");
        assert_eq!(final_args, vec![updated]);
        assert!(skip_write_back_edit_indices.is_empty());
    }
}
