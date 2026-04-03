use std::cell::RefCell;
use std::collections::BTreeMap;

use arcana_aot::parse_package_artifact;
use arcana_cabi::{
    ArcanaCabiParamSourceMode, ArcanaCabiPassMode, ArcanaCabiProviderCallOutcome,
    ArcanaCabiProviderCallable, ArcanaCabiProviderDescriptorViewBackingKind,
    ArcanaCabiProviderDescriptorViewOwner, ArcanaCabiProviderOpaqueFamily, ArcanaCabiProviderParam,
    ArcanaCabiProviderValue, ArcanaCabiProviderWriteBack,
};
use arcana_ir::IrRoutineType;

use crate::{
    FlowSignal, RuntimeExecutionState, RuntimeHost, RuntimeImageHandle, RuntimeOpaqueValue,
    RuntimePackagePlan, RuntimeRoutinePlan, RuntimeValue, execute_routine_call_with_state,
    insert_runtime_byte_view, insert_runtime_read_view, insert_runtime_str_view,
    parse_runtime_package_image, plan_from_artifact, runtime_eval_message,
    runtime_substrate_opaque_from_family,
};

pub trait RuntimeSourceProviderCallbacks {
    fn package_asset_root(&mut self, package_id: &str) -> Result<String, String>;
    fn descriptor_view_values(
        &mut self,
        family: &str,
        view_id: u64,
        start: u64,
        len: u64,
    ) -> Result<Vec<ArcanaCabiProviderValue>, String>;
    fn descriptor_view_bytes(
        &mut self,
        family: &str,
        view_id: u64,
        start: u64,
        len: u64,
    ) -> Result<Vec<u8>, String>;
    fn canvas_image_create(&mut self, width: i64, height: i64) -> Result<u64, String>;
    fn canvas_image_replace_rgba(&mut self, image_id: u64, rgba: &[u8]) -> Result<(), String>;
    fn canvas_blit(&mut self, window_id: u64, image_id: u64, x: i64, y: i64) -> Result<(), String>;
}

thread_local! {
    static ACTIVE_SOURCE_PROVIDER_CALLBACKS: RefCell<Option<*mut (dyn RuntimeSourceProviderCallbacks + 'static)>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug)]
enum SourceBridgeKind {
    Direct,
    Opaque {
        family_key: String,
        type_path: String,
    },
}

#[derive(Clone, Debug)]
struct SourceCallableParamBridge {
    descriptor: ArcanaCabiProviderParam,
    kind: SourceBridgeKind,
}

#[derive(Clone, Debug)]
struct SourceCallableBridge {
    descriptor: ArcanaCabiProviderCallable,
    result_kind: SourceBridgeKind,
    routine_index: usize,
    params: Vec<SourceCallableParamBridge>,
}

#[derive(Clone, Debug)]
struct SourceOpaqueSlot {
    family_key: String,
    type_path: String,
    ref_count: u64,
    value: RuntimeValue,
}

#[derive(Clone, Debug)]
pub struct ArcanaSourceProvider {
    descriptor: arcana_cabi::ArcanaCabiProviderDescriptor,
    plan: RuntimePackagePlan,
    bridges: BTreeMap<String, SourceCallableBridge>,
    next_opaque_id: u64,
    opaque_slots: BTreeMap<u64, SourceOpaqueSlot>,
}

impl ArcanaSourceProvider {
    pub fn new(package_image_text: &str) -> Result<Self, String> {
        let plan = parse_runtime_package_image(package_image_text).or_else(|_| {
            let artifact = parse_package_artifact(package_image_text)?;
            plan_from_artifact(&artifact)
        })?;
        let (descriptor, bridges) = discover_source_provider_bridges(&plan)?;
        Ok(Self {
            descriptor,
            plan,
            bridges,
            next_opaque_id: 1,
            opaque_slots: BTreeMap::new(),
        })
    }

    pub fn descriptor(&self) -> &arcana_cabi::ArcanaCabiProviderDescriptor {
        &self.descriptor
    }

    pub fn retain_opaque(&mut self, family_key: &str, opaque_id: u64) -> Result<(), String> {
        let slot = self
            .opaque_slots
            .get_mut(&opaque_id)
            .ok_or_else(|| format!("unknown source provider opaque id `{opaque_id}`"))?;
        if slot.family_key != family_key {
            return Err(format!(
                "source provider opaque `{opaque_id}` belongs to `{}`, not `{family_key}`",
                slot.family_key
            ));
        }
        slot.ref_count = slot.ref_count.saturating_add(1);
        Ok(())
    }

    pub fn release_opaque(&mut self, family_key: &str, opaque_id: u64) -> Result<(), String> {
        let remove = {
            let slot = self
                .opaque_slots
                .get_mut(&opaque_id)
                .ok_or_else(|| format!("unknown source provider opaque id `{opaque_id}`"))?;
            if slot.family_key != family_key {
                return Err(format!(
                    "source provider opaque `{opaque_id}` belongs to `{}`, not `{family_key}`",
                    slot.family_key
                ));
            }
            if slot.ref_count > 1 {
                slot.ref_count -= 1;
                false
            } else {
                true
            }
        };
        if remove {
            self.opaque_slots.remove(&opaque_id);
        }
        Ok(())
    }

    pub fn invoke(
        &mut self,
        callable_key: &str,
        args: &[ArcanaCabiProviderValue],
        host: &mut dyn RuntimeHost,
        callbacks: &mut dyn RuntimeSourceProviderCallbacks,
    ) -> Result<ArcanaCabiProviderCallOutcome, String> {
        let bridge = self
            .bridges
            .get(callable_key)
            .ok_or_else(|| format!("source provider does not expose `{callable_key}`"))?
            .clone();
        if bridge.params.len() != args.len() {
            return Err(format!(
                "source provider callable `{callable_key}` expected {} args, got {}",
                bridge.params.len(),
                args.len()
            ));
        }

        let mut state = RuntimeExecutionState::default();
        let mut runtime_args = Vec::with_capacity(args.len());
        for (index, (arg, param)) in args.iter().zip(&bridge.params).enumerate() {
            runtime_args.push(match &param.kind {
                SourceBridgeKind::Direct => provider_value_to_runtime_direct(arg, &mut state)
                    .map_err(|err| {
                        format!(
                            "source provider arg {index} `{}` for `{callable_key}`: {err}",
                            param.descriptor.name
                        )
                    })?,
                SourceBridgeKind::Opaque {
                    family_key,
                    type_path,
                } => self.provider_opaque_arg_value(
                    arg,
                    family_key,
                    type_path,
                    callable_key,
                    &param.descriptor.name,
                )?,
            });
        }

        let outcome = with_active_source_provider_callbacks(callbacks, || {
            execute_routine_call_with_state(
                &self.plan,
                bridge.routine_index,
                Vec::new(),
                runtime_args,
                &Vec::new(),
                None,
                None,
                None,
                None,
                None,
                &mut state,
                host,
                false,
            )
            .map_err(runtime_eval_message)
        })?;
        if let Some(FlowSignal::OwnerExit {
            owner_key,
            exit_name,
        }) = outcome.control
        {
            return Err(format!(
                "source provider callable `{callable_key}` triggered owner exit `{owner_key}`:`{exit_name}`"
            ));
        }

        let result = self.runtime_value_to_provider_result(
            &bridge.result_kind,
            &bridge.descriptor.return_type,
            outcome.value,
            &state,
            None,
            callable_key,
        )?;
        let mut write_backs = Vec::new();
        for (index, param) in bridge.params.iter().enumerate() {
            if param.descriptor.pass_mode != ArcanaCabiPassMode::InWithWriteBack {
                continue;
            }
            let final_value = outcome.final_args.get(index).cloned().ok_or_else(|| {
                format!(
                    "source provider callable `{callable_key}` missing final arg {index} for write-back"
                )
            })?;
            let prior_opaque = match &param.kind {
                SourceBridgeKind::Opaque { family_key, .. } => Some(self.provider_opaque_arg_id(
                    &args[index],
                    family_key,
                    callable_key,
                    &param.descriptor.name,
                )?),
                SourceBridgeKind::Direct => None,
            };
            if !provider_write_back_needed(&param.kind, &final_value) {
                continue;
            }
            let value = self.runtime_value_to_provider_result(
                &param.kind,
                param
                    .descriptor
                    .write_back_type
                    .as_deref()
                    .unwrap_or(&param.descriptor.input_type),
                final_value,
                &state,
                prior_opaque,
                callable_key,
            )?;
            write_backs.push(ArcanaCabiProviderWriteBack {
                index,
                name: param.descriptor.name.clone(),
                value,
            });
        }
        Ok(ArcanaCabiProviderCallOutcome {
            result,
            write_backs,
        })
    }

    fn provider_opaque_arg_id(
        &self,
        arg: &ArcanaCabiProviderValue,
        expected_family: &str,
        callable_key: &str,
        param_name: &str,
    ) -> Result<u64, String> {
        let ArcanaCabiProviderValue::ProviderOpaque { family, id } = arg else {
            return Err(format!(
                "source provider callable `{callable_key}` param `{param_name}` expects opaque `{expected_family}`"
            ));
        };
        if family != expected_family {
            return Err(format!(
                "source provider callable `{callable_key}` param `{param_name}` expects opaque `{expected_family}`, got `{family}`"
            ));
        }
        Ok(*id)
    }

    fn provider_opaque_arg_value(
        &self,
        arg: &ArcanaCabiProviderValue,
        expected_family: &str,
        expected_type: &str,
        callable_key: &str,
        param_name: &str,
    ) -> Result<RuntimeValue, String> {
        let opaque_id =
            self.provider_opaque_arg_id(arg, expected_family, callable_key, param_name)?;
        let slot = self.opaque_slots.get(&opaque_id).ok_or_else(|| {
            format!(
                "source provider callable `{callable_key}` param `{param_name}` references unknown opaque `{opaque_id}`"
            )
        })?;
        if slot.type_path != expected_type {
            return Err(format!(
                "source provider callable `{callable_key}` param `{param_name}` expected type `{expected_type}`, got `{}`",
                slot.type_path
            ));
        }
        Ok(slot.value.clone())
    }

    fn runtime_value_to_provider_result(
        &mut self,
        kind: &SourceBridgeKind,
        type_path: &str,
        value: RuntimeValue,
        state: &RuntimeExecutionState,
        existing_opaque_id: Option<u64>,
        callable_key: &str,
    ) -> Result<ArcanaCabiProviderValue, String> {
        match kind {
            SourceBridgeKind::Direct => {
                runtime_value_to_provider_direct(value, state, &self.plan.package_id)
            }
            SourceBridgeKind::Opaque {
                family_key,
                type_path: expected_type,
            } => {
                if let Some(opaque_id) = existing_opaque_id {
                    let slot = self.opaque_slots.get_mut(&opaque_id).ok_or_else(|| {
                        format!(
                            "source provider callable `{callable_key}` cannot update missing opaque `{opaque_id}`"
                        )
                    })?;
                    slot.family_key = family_key.clone();
                    slot.type_path = expected_type.clone();
                    slot.value = value;
                    return Ok(ArcanaCabiProviderValue::ProviderOpaque {
                        family: family_key.clone(),
                        id: opaque_id,
                    });
                }
                let opaque_id = self.next_opaque_id.max(1);
                self.next_opaque_id = opaque_id + 1;
                self.opaque_slots.insert(
                    opaque_id,
                    SourceOpaqueSlot {
                        family_key: family_key.clone(),
                        type_path: expected_type.clone(),
                        ref_count: 1,
                        value,
                    },
                );
                Ok(ArcanaCabiProviderValue::ProviderOpaque {
                    family: family_key.clone(),
                    id: opaque_id,
                })
            }
        }
        .map_err(|err| {
            format!(
                "source provider `{callable_key}` could not encode `{type_path}`: {err}"
            )
        })
    }
}

pub(crate) fn source_provider_dispatch_active() -> bool {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| slot.borrow().is_some())
}

pub(crate) fn active_source_provider_package_asset_root(
    package_id: &str,
) -> Result<Option<String>, String> {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| {
        let Some(ptr) = *slot.borrow() else {
            return Ok(None);
        };
        let callbacks = unsafe { &mut *ptr };
        callbacks.package_asset_root(package_id).map(Some)
    })
}

pub(crate) fn active_source_provider_descriptor_view_values(
    family: &str,
    view_id: u64,
    start: u64,
    len: u64,
) -> Result<Option<Vec<ArcanaCabiProviderValue>>, String> {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| {
        let Some(ptr) = *slot.borrow() else {
            return Ok(None);
        };
        let callbacks = unsafe { &mut *ptr };
        callbacks
            .descriptor_view_values(family, view_id, start, len)
            .map(Some)
    })
}

pub(crate) fn active_source_provider_descriptor_view_bytes(
    family: &str,
    view_id: u64,
    start: u64,
    len: u64,
) -> Result<Option<Vec<u8>>, String> {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| {
        let Some(ptr) = *slot.borrow() else {
            return Ok(None);
        };
        let callbacks = unsafe { &mut *ptr };
        callbacks
            .descriptor_view_bytes(family, view_id, start, len)
            .map(Some)
    })
}

pub(crate) fn active_source_provider_canvas_image_create(
    width: i64,
    height: i64,
) -> Result<Option<RuntimeImageHandle>, String> {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| {
        let Some(ptr) = *slot.borrow() else {
            return Ok(None);
        };
        let callbacks = unsafe { &mut *ptr };
        callbacks
            .canvas_image_create(width, height)
            .map(|id| Some(RuntimeImageHandle(id)))
    })
}

pub(crate) fn active_source_provider_canvas_image_replace_rgba(
    image: RuntimeImageHandle,
    rgba: &[u8],
) -> Result<Option<()>, String> {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| {
        let Some(ptr) = *slot.borrow() else {
            return Ok(None);
        };
        let callbacks = unsafe { &mut *ptr };
        callbacks.canvas_image_replace_rgba(image.0, rgba).map(Some)
    })
}

pub(crate) fn active_source_provider_canvas_blit(
    window_id: u64,
    image_id: u64,
    x: i64,
    y: i64,
) -> Result<Option<()>, String> {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| {
        let Some(ptr) = *slot.borrow() else {
            return Ok(None);
        };
        let callbacks = unsafe { &mut *ptr };
        callbacks.canvas_blit(window_id, image_id, x, y).map(Some)
    })
}

fn with_active_source_provider_callbacks<R>(
    callbacks: &mut dyn RuntimeSourceProviderCallbacks,
    action: impl FnOnce() -> Result<R, String>,
) -> Result<R, String> {
    ACTIVE_SOURCE_PROVIDER_CALLBACKS.with(|slot| {
        let callbacks_ptr = unsafe {
            std::mem::transmute::<
                *mut dyn RuntimeSourceProviderCallbacks,
                *mut (dyn RuntimeSourceProviderCallbacks + 'static),
            >(callbacks as *mut dyn RuntimeSourceProviderCallbacks)
        };
        let previous = slot.replace(Some(callbacks_ptr));
        let result = action();
        slot.replace(previous);
        result
    })
}

fn discover_source_provider_bridges(
    plan: &RuntimePackagePlan,
) -> Result<
    (
        arcana_cabi::ArcanaCabiProviderDescriptor,
        BTreeMap<String, SourceCallableBridge>,
    ),
    String,
> {
    let mut by_key = BTreeMap::<String, (usize, &RuntimeRoutinePlan)>::new();
    for (index, routine) in plan.routines.iter().enumerate() {
        if routine.package_id != plan.package_id {
            continue;
        }
        by_key.insert(runtime_routine_callable_key(routine), (index, routine));
    }

    let mut bridges = BTreeMap::new();
    let mut families = BTreeMap::<String, ArcanaCabiProviderOpaqueFamily>::new();
    for (public_key, (_, routine)) in &by_key {
        if !routine.exported
            || routine.symbol_kind != "fn"
            || public_key.starts_with(&format!("{}.provider_impl.", plan.package_name))
        {
            continue;
        }
        let internal_key = provider_impl_callable_key(&plan.package_name, public_key)?;
        let Some((internal_index, internal_routine)) = by_key.get(&internal_key) else {
            continue;
        };
        if routine.params.len() != internal_routine.params.len() {
            return Err(format!(
                "source provider bridge `{public_key}` param count {} does not match internal `{internal_key}` count {}",
                routine.params.len(),
                internal_routine.params.len()
            ));
        }
        let public_return_type = routine_type_text(routine.return_type.as_ref());
        let internal_return_type = routine_type_text(internal_routine.return_type.as_ref());
        let result_kind = bridge_kind(&public_return_type, &internal_return_type);
        collect_bridge_family(&result_kind, &mut families);

        let mut params = Vec::with_capacity(routine.params.len());
        let mut descriptor_params = Vec::with_capacity(routine.params.len());
        for (public_param, internal_param) in routine.params.iter().zip(&internal_routine.params) {
            let descriptor = ArcanaCabiProviderParam {
                name: public_param.name.clone(),
                source_mode: source_mode_for_param(public_param.mode.as_deref()),
                pass_mode: pass_mode_for_param(public_param.mode.as_deref()),
                input_type: public_param.ty.render(),
                write_back_type: write_back_type_for_mode(
                    public_param.mode.as_deref(),
                    &public_param.ty,
                ),
            };
            let kind = bridge_kind(&descriptor.input_type, &internal_param.ty.render());
            collect_bridge_family(&kind, &mut families);
            descriptor_params.push(descriptor.clone());
            params.push(SourceCallableParamBridge { descriptor, kind });
        }

        bridges.insert(
            public_key.clone(),
            SourceCallableBridge {
                descriptor: ArcanaCabiProviderCallable {
                    callable_key: public_key.clone(),
                    path: public_key.clone(),
                    routine_key: Some(internal_routine.routine_key.clone()),
                    return_type: public_return_type,
                    params: descriptor_params,
                },
                result_kind,
                routine_index: *internal_index,
                params,
            },
        );
    }

    let mut callables = bridges
        .values()
        .map(|bridge| bridge.descriptor.clone())
        .collect::<Vec<_>>();
    callables.sort_by(|left, right| left.callable_key.cmp(&right.callable_key));
    let mut opaque_families = families.into_values().collect::<Vec<_>>();
    opaque_families.sort_by(|left, right| left.family_key.cmp(&right.family_key));
    Ok((
        arcana_cabi::ArcanaCabiProviderDescriptor {
            format: "arcana.cabi.provider.source.v1".to_string(),
            package_name: plan.package_name.clone(),
            product_name: "default".to_string(),
            callables,
            opaque_families,
        },
        bridges,
    ))
}

fn collect_bridge_family(
    kind: &SourceBridgeKind,
    families: &mut BTreeMap<String, ArcanaCabiProviderOpaqueFamily>,
) {
    let SourceBridgeKind::Opaque {
        family_key,
        type_path,
    } = kind
    else {
        return;
    };
    families
        .entry(family_key.clone())
        .or_insert_with(|| ArcanaCabiProviderOpaqueFamily {
            family_key: family_key.clone(),
            type_path: type_path.clone(),
        });
}

fn bridge_kind(public_type: &str, internal_type: &str) -> SourceBridgeKind {
    if public_type == internal_type {
        SourceBridgeKind::Direct
    } else {
        SourceBridgeKind::Opaque {
            family_key: public_type.to_string(),
            type_path: public_type.to_string(),
        }
    }
}

fn runtime_routine_callable_key(routine: &RuntimeRoutinePlan) -> String {
    format!("{}.{}", routine.module_id, routine.symbol_name)
}

fn provider_impl_callable_key(package_name: &str, public_key: &str) -> Result<String, String> {
    let prefix = format!("{package_name}.");
    let suffix = public_key.strip_prefix(&prefix).ok_or_else(|| {
        format!("source provider callable `{public_key}` is outside package `{package_name}`")
    })?;
    Ok(format!("{package_name}.provider_impl.{suffix}"))
}

fn routine_type_text(ty: Option<&IrRoutineType>) -> String {
    ty.map(IrRoutineType::render)
        .unwrap_or_else(|| "Unit".to_string())
}

fn write_back_type_for_mode(mode: Option<&str>, ty: &IrRoutineType) -> Option<String> {
    if matches!(mode, Some("edit")) {
        Some(ty.render())
    } else {
        None
    }
}

fn source_mode_for_param(mode: Option<&str>) -> ArcanaCabiParamSourceMode {
    match mode {
        Some("take") | Some("move") => ArcanaCabiParamSourceMode::Take,
        Some("edit") => ArcanaCabiParamSourceMode::Edit,
        _ => ArcanaCabiParamSourceMode::Read,
    }
}

fn pass_mode_for_param(mode: Option<&str>) -> ArcanaCabiPassMode {
    if matches!(mode, Some("edit")) {
        ArcanaCabiPassMode::InWithWriteBack
    } else {
        ArcanaCabiPassMode::In
    }
}

fn provider_value_to_runtime_direct(
    value: &ArcanaCabiProviderValue,
    state: &mut RuntimeExecutionState,
) -> Result<RuntimeValue, String> {
    match value {
        ArcanaCabiProviderValue::Int(value) => Ok(RuntimeValue::Int(*value)),
        ArcanaCabiProviderValue::Bool(value) => Ok(RuntimeValue::Bool(*value)),
        ArcanaCabiProviderValue::Str(value) => Ok(RuntimeValue::Str(value.clone())),
        ArcanaCabiProviderValue::Bytes(bytes) => Ok(RuntimeValue::Array(
            bytes
                .iter()
                .map(|byte| RuntimeValue::Int(i64::from(*byte)))
                .collect(),
        )),
        ArcanaCabiProviderValue::Pair(left, right) => Ok(RuntimeValue::Pair(
            Box::new(provider_value_to_runtime_direct(left, state)?),
            Box::new(provider_value_to_runtime_direct(right, state)?),
        )),
        ArcanaCabiProviderValue::List(values) => Ok(RuntimeValue::List(
            values
                .iter()
                .map(|value| provider_value_to_runtime_direct(value, state))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        ArcanaCabiProviderValue::Map(entries) => {
            let mut map = Vec::new();
            for (key, value) in entries {
                map.push((
                    provider_value_to_runtime_direct(key, state)?,
                    provider_value_to_runtime_direct(value, state)?,
                ));
            }
            Ok(RuntimeValue::Map(map))
        }
        ArcanaCabiProviderValue::Range {
            start,
            end,
            inclusive_end,
        } => Ok(RuntimeValue::Range {
            start: *start,
            end: *end,
            inclusive_end: *inclusive_end,
        }),
        ArcanaCabiProviderValue::Record { name, fields } => Ok(RuntimeValue::Record {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(key, value)| {
                    Ok((key.clone(), provider_value_to_runtime_direct(value, state)?))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?,
        }),
        ArcanaCabiProviderValue::Variant { name, payload } => Ok(RuntimeValue::Variant {
            name: name.clone(),
            payload: payload
                .iter()
                .map(|value| provider_value_to_runtime_direct(value, state))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        ArcanaCabiProviderValue::DescriptorView(view) => match &view.owner {
            ArcanaCabiProviderDescriptorViewOwner::Runtime { .. } => match view.backing_kind {
                ArcanaCabiProviderDescriptorViewBackingKind::ReadElements => {
                    let values = active_source_provider_descriptor_view_values(
                        &view.family,
                        view.id,
                        view.start,
                        view.len,
                    )?
                    .ok_or_else(|| {
                        "descriptor element views require active source provider callbacks"
                            .to_string()
                    })?;
                    let runtime_values = values
                        .iter()
                        .map(|value| provider_value_to_runtime_direct(value, state))
                        .collect::<Result<Vec<_>, _>>()?;
                    let handle = insert_runtime_read_view(
                        state,
                        &[view.element_layout.clone()],
                        runtime_values,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)))
                }
                ArcanaCabiProviderDescriptorViewBackingKind::ReadBytes => {
                    let bytes = active_source_provider_descriptor_view_bytes(
                        &view.family,
                        view.id,
                        view.start,
                        view.len,
                    )?
                    .ok_or_else(|| {
                        "descriptor byte views require active source provider callbacks".to_string()
                    })?;
                    let handle = insert_runtime_byte_view(state, bytes);
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)))
                }
                ArcanaCabiProviderDescriptorViewBackingKind::ReadUtf8 => {
                    let bytes = active_source_provider_descriptor_view_bytes(
                        &view.family,
                        view.id,
                        view.start,
                        view.len,
                    )?
                    .ok_or_else(|| {
                        "descriptor string views require active source provider callbacks"
                            .to_string()
                    })?;
                    let text = String::from_utf8(bytes).map_err(|err| {
                        format!("descriptor string view is not valid utf-8: {err}")
                    })?;
                    let handle = insert_runtime_str_view(state, text);
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)))
                }
            },
            ArcanaCabiProviderDescriptorViewOwner::ProviderBinding { .. } => Err(format!(
                "provider-owned descriptor view `{}` cannot cross a direct provider value bridge",
                view.family
            )),
        },
        ArcanaCabiProviderValue::SubstrateOpaque { family, id } => Ok(RuntimeValue::Opaque(
            runtime_substrate_opaque_from_family(family, *id)?,
        )),
        ArcanaCabiProviderValue::ProviderOpaque { family, .. } => Err(format!(
            "provider opaque `{family}` cannot cross a direct provider value bridge"
        )),
        ArcanaCabiProviderValue::Unit => Ok(RuntimeValue::Unit),
    }
}

fn runtime_value_to_provider_direct(
    value: RuntimeValue,
    state: &RuntimeExecutionState,
    package_id: &str,
) -> Result<ArcanaCabiProviderValue, String> {
    match value {
        RuntimeValue::Int(value) => Ok(ArcanaCabiProviderValue::Int(value)),
        RuntimeValue::Bool(value) => Ok(ArcanaCabiProviderValue::Bool(value)),
        RuntimeValue::Str(value) => Ok(ArcanaCabiProviderValue::Str(value)),
        RuntimeValue::Array(values) => {
            let mut bytes = Vec::with_capacity(values.len());
            for value in values {
                let RuntimeValue::Int(byte) = value else {
                    return Err("direct provider Array values must contain Int items".to_string());
                };
                bytes
                    .push(u8::try_from(byte).map_err(|_| {
                        format!("direct provider byte `{byte}` does not fit in u8")
                    })?);
            }
            Ok(ArcanaCabiProviderValue::Bytes(bytes))
        }
        RuntimeValue::Pair(left, right) => Ok(ArcanaCabiProviderValue::Pair(
            Box::new(runtime_value_to_provider_direct(*left, state, package_id)?),
            Box::new(runtime_value_to_provider_direct(*right, state, package_id)?),
        )),
        RuntimeValue::List(values) => Ok(ArcanaCabiProviderValue::List(
            values
                .into_iter()
                .map(|value| runtime_value_to_provider_direct(value, state, package_id))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        RuntimeValue::Map(entries) => Ok(ArcanaCabiProviderValue::Map(
            entries
                .into_iter()
                .map(|(key, value)| {
                    Ok((
                        runtime_value_to_provider_direct(key, state, package_id)?,
                        runtime_value_to_provider_direct(value, state, package_id)?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?,
        )),
        RuntimeValue::Range {
            start,
            end,
            inclusive_end,
        } => Ok(ArcanaCabiProviderValue::Range {
            start,
            end,
            inclusive_end,
        }),
        RuntimeValue::Record { name, fields } => Ok(ArcanaCabiProviderValue::Record {
            name,
            fields: fields
                .into_iter()
                .map(|(key, value)| {
                    Ok((key, runtime_value_to_provider_direct(value, state, package_id)?))
                })
                .collect::<Result<Vec<_>, String>>()?,
        }),
        RuntimeValue::Variant { name, payload } => Ok(ArcanaCabiProviderValue::Variant {
            name,
            payload: payload
                .into_iter()
                .map(|value| runtime_value_to_provider_direct(value, state, package_id))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Image(handle)) => {
            Ok(ArcanaCabiProviderValue::SubstrateOpaque {
                family: "std.canvas.Image".to_string(),
                id: handle.0,
            })
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Window(handle)) => {
            Ok(ArcanaCabiProviderValue::SubstrateOpaque {
                family: "std.window.Window".to_string(),
                id: handle.0,
            })
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(_)) => Err(
            "direct provider conversion does not support provider-originated view results across the provider boundary"
                .to_string(),
        ),
        RuntimeValue::Opaque(other) => Err(format!(
            "direct provider conversion does not support substrate opaque `{}`",
            crate::opaque_type_name(&other)
        )),
        RuntimeValue::OwnerHandle(_) | RuntimeValue::Ref(_) => {
            Err("direct provider conversion does not support owner/ref values".to_string())
        }
        RuntimeValue::Unit => Ok(ArcanaCabiProviderValue::Unit),
    }
}

fn provider_write_back_needed(kind: &SourceBridgeKind, value: &RuntimeValue) -> bool {
    match kind {
        SourceBridgeKind::Direct => !matches!(value, RuntimeValue::Opaque(_)),
        SourceBridgeKind::Opaque { .. } => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeSourceProviderCallbacks, provider_value_to_runtime_direct,
        with_active_source_provider_callbacks,
    };
    use arcana_cabi::{
        ArcanaCabiProviderDescriptorView, ArcanaCabiProviderDescriptorViewBackingKind,
        ArcanaCabiProviderDescriptorViewOwner, ArcanaCabiProviderValue,
    };

    use crate::{
        RuntimeExecutionState, RuntimeOpaqueValue, RuntimeValue, runtime_byte_view_snapshot,
        runtime_read_view_snapshot,
    };

    struct TestCallbacks;

    impl RuntimeSourceProviderCallbacks for TestCallbacks {
        fn package_asset_root(&mut self, _package_id: &str) -> Result<String, String> {
            Ok(String::new())
        }

        fn descriptor_view_values(
            &mut self,
            family: &str,
            view_id: u64,
            start: u64,
            len: u64,
        ) -> Result<Vec<ArcanaCabiProviderValue>, String> {
            assert_eq!(family, "std.memory.ReadView");
            assert_eq!(view_id, 77);
            assert_eq!(start, 0);
            assert_eq!(len, 2);
            Ok(vec![
                ArcanaCabiProviderValue::Int(11),
                ArcanaCabiProviderValue::Int(29),
            ])
        }

        fn descriptor_view_bytes(
            &mut self,
            family: &str,
            view_id: u64,
            start: u64,
            len: u64,
        ) -> Result<Vec<u8>, String> {
            assert_eq!(family, "std.memory.ByteView");
            assert_eq!(view_id, 41);
            assert_eq!(start, 0);
            assert_eq!(len, 3);
            Ok(vec![10, 20, 30])
        }

        fn canvas_image_create(&mut self, _width: i64, _height: i64) -> Result<u64, String> {
            Err("unused".to_string())
        }

        fn canvas_image_replace_rgba(
            &mut self,
            _image_id: u64,
            _rgba: &[u8],
        ) -> Result<(), String> {
            Err("unused".to_string())
        }

        fn canvas_blit(
            &mut self,
            _window_id: u64,
            _image_id: u64,
            _x: i64,
            _y: i64,
        ) -> Result<(), String> {
            Err("unused".to_string())
        }
    }

    #[test]
    fn descriptor_byte_view_materializes_local_runtime_handle() {
        let descriptor =
            ArcanaCabiProviderValue::DescriptorView(ArcanaCabiProviderDescriptorView {
                owner: ArcanaCabiProviderDescriptorViewOwner::Runtime {
                    package_id: "path:app".to_string(),
                },
                backing_kind: ArcanaCabiProviderDescriptorViewBackingKind::ReadBytes,
                family: "std.memory.ByteView".to_string(),
                id: 41,
                element_type: "Int".to_string(),
                element_layout: "Int".to_string(),
                start: 0,
                len: 3,
                mutable: false,
            });
        let mut callbacks = TestCallbacks;
        let mut state = RuntimeExecutionState::default();
        let value = with_active_source_provider_callbacks(&mut callbacks, || {
            provider_value_to_runtime_direct(&descriptor, &mut state)
        })
        .expect("descriptor byte view should materialize");
        let RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) = value else {
            panic!("expected ByteView runtime value");
        };
        assert_eq!(
            runtime_byte_view_snapshot(&state, handle).expect("byte view should exist"),
            vec![10, 20, 30]
        );
    }

    #[test]
    fn descriptor_element_view_materializes_local_runtime_handle() {
        let descriptor =
            ArcanaCabiProviderValue::DescriptorView(ArcanaCabiProviderDescriptorView {
                owner: ArcanaCabiProviderDescriptorViewOwner::Runtime {
                    package_id: "path:app".to_string(),
                },
                backing_kind: ArcanaCabiProviderDescriptorViewBackingKind::ReadElements,
                family: "std.memory.ReadView".to_string(),
                id: 77,
                element_type: "Int".to_string(),
                element_layout: "Int".to_string(),
                start: 0,
                len: 2,
                mutable: false,
            });
        let mut callbacks = TestCallbacks;
        let mut state = RuntimeExecutionState::default();
        let value = with_active_source_provider_callbacks(&mut callbacks, || {
            provider_value_to_runtime_direct(&descriptor, &mut state)
        })
        .expect("descriptor element view should materialize");
        let RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) = value else {
            panic!("expected ReadView runtime value");
        };
        assert_eq!(
            runtime_read_view_snapshot(&state, handle).expect("read view should exist"),
            vec![RuntimeValue::Int(11), RuntimeValue::Int(29)]
        );
    }
}
