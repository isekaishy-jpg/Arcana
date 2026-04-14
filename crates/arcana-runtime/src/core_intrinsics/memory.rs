use super::*;
use crate::runtime_intrinsics::MemoryIntrinsic as RuntimeIntrinsic;

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
        RuntimeIntrinsic::MemoryArenaNew => {
            let capacity = expect_int(expect_single_arg(args, "arena_new")?, "arena_new")?;
            let capacity = runtime_non_negative_usize(capacity, "arena_new capacity")?;
            let handle =
                insert_runtime_arena(state, type_args, default_runtime_arena_policy(capacity));
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(handle)))
        }
        RuntimeIntrinsic::MemoryArenaAlloc => {
            if args.len() != 2 {
                return Err("arena_alloc expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_alloc")?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            ensure_runtime_arena_capacity(arena)?;
            let slot = arena.next_slot;
            arena.next_slot += 1;
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(
                RuntimeArenaIdValue {
                    arena: handle,
                    slot,
                    generation: arena.generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryArenaLen => {
            let handle = expect_arena(expect_single_arg(args, "arena_len")?, "arena_len")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemoryArenaHas => {
            if args.len() != 2 {
                return Err("arena_has expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_has")?;
            let id = expect_arena_id(args[1].clone(), "arena_has")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(arena_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryArenaGet => {
            if args.len() != 2 {
                return Err("arena access expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_access")?;
            let id = expect_arena_id(args[1].clone(), "arena_access")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("Arena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryArenaBorrowRead | RuntimeIntrinsic::MemoryArenaBorrowEdit => {
            if args.len() != 2 {
                return Err("arena borrow expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_borrow")?;
            let id = expect_arena_id(args[1].clone(), "arena_borrow")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mode: runtime_reference_mode_for_place(matches!(
                    intrinsic,
                    RuntimeIntrinsic::MemoryArenaBorrowEdit
                )),
                target: RuntimeReferenceTarget::ArenaSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryArenaSet => {
            if args.len() != 3 {
                return Err("arena_set expects three arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_set")?;
            let id = expect_arena_id(args[1].clone(), "arena_set")?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryArenaRemove => {
            if args.len() != 2 {
                return Err("arena_remove expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_remove")?;
            let id = expect_arena_id(args[1].clone(), "arena_remove")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_arena_id(reference, id),
                format!(
                    "arena_remove rejects invalidation while borrowed views for ArenaId `{}` are live",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id)))
                ),
            )?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            arena.slots.remove(&id.slot);
            Ok(RuntimeValue::Bool(true))
        }
        RuntimeIntrinsic::MemoryArenaReset => {
            let handle = expect_arena(expect_single_arg(args, "arena_reset")?, "arena_reset")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_arena(reference, handle),
                format!(
                    "arena_reset rejects invalidation while borrowed views for Arena `{}` are live",
                    handle.0
                ),
            )?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            arena.generation += 1;
            if matches!(arena.policy.handle, RuntimeMemoryHandlePolicy::Unstable) {
                arena.next_slot = 0;
            }
            arena.slots.clear();
            arena.policy.current_limit = arena.policy.base_capacity;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryFrameNew => {
            let capacity = expect_int(expect_single_arg(args, "frame_new")?, "frame_new")?;
            let capacity = runtime_non_negative_usize(capacity, "frame_new capacity")?;
            let handle = insert_runtime_frame_arena(
                state,
                type_args,
                default_runtime_frame_policy(capacity),
            );
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle)))
        }
        RuntimeIntrinsic::MemoryFrameAlloc => {
            if args.len() != 2 {
                return Err("frame_alloc expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_alloc")?;
            let arena = state
                .frame_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            ensure_runtime_frame_capacity(arena)?;
            let slot = arena.next_slot;
            arena.next_slot += 1;
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(
                RuntimeFrameIdValue {
                    arena: handle,
                    slot,
                    generation: arena.generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryFrameLen => {
            let handle = expect_frame_arena(expect_single_arg(args, "frame_len")?, "frame_len")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemoryFrameHas => {
            if args.len() != 2 {
                return Err("frame_has expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_has")?;
            let id = expect_frame_id(args[1].clone(), "frame_has")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(frame_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryFrameGet => {
            if args.len() != 2 {
                return Err("frame access expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_access")?;
            let id = expect_frame_id(args[1].clone(), "frame_access")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            if !frame_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("FrameArena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryFrameBorrowRead | RuntimeIntrinsic::MemoryFrameBorrowEdit => {
            if args.len() != 2 {
                return Err("frame borrow expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_borrow")?;
            let id = expect_frame_id(args[1].clone(), "frame_borrow")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            if !frame_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mode: runtime_reference_mode_for_place(matches!(
                    intrinsic,
                    RuntimeIntrinsic::MemoryFrameBorrowEdit
                )),
                target: RuntimeReferenceTarget::FrameSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryFrameSet => {
            if args.len() != 3 {
                return Err("frame_set expects three arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_set")?;
            let id = expect_frame_id(args[1].clone(), "frame_set")?;
            let arena = state
                .frame_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            if !frame_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryFrameReset => {
            let handle =
                expect_frame_arena(expect_single_arg(args, "frame_reset")?, "frame_reset")?;
            runtime_reset_frame_arena_handle(state, handle, "frame_reset")?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryPoolNew => {
            let capacity = expect_int(expect_single_arg(args, "pool_new")?, "pool_new")?;
            let capacity = runtime_non_negative_usize(capacity, "pool_new capacity")?;
            let handle =
                insert_runtime_pool_arena(state, type_args, default_runtime_pool_policy(capacity));
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(handle)))
        }
        RuntimeIntrinsic::MemoryPoolAlloc => {
            if args.len() != 2 {
                return Err("pool_alloc expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_alloc")?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            let slot = if matches!(arena.policy.recycle, RuntimePoolRecyclePolicy::FreeList) {
                arena.free_slots.pop()
            } else {
                None
            }
            .unwrap_or_else(|| {
                if arena.slots.len() >= arena.policy.current_limit
                    && matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic)
                {
                    let _ = runtime_try_grow_limit(
                        &mut arena.policy.current_limit,
                        arena.policy.growth_step,
                    );
                }
                let has_capacity = arena.slots.len() < arena.policy.current_limit;
                if !has_capacity {
                    return u64::MAX;
                }
                let slot = arena.next_slot;
                arena.next_slot += 1;
                arena.generations.entry(slot).or_insert(0);
                slot
            });
            if slot == u64::MAX {
                return Err(format!(
                    "pool capacity exhausted at {}; growth={} pressure={:?} recycle={:?}",
                    arena.policy.current_limit,
                    arena.policy.growth_step,
                    arena.policy.pressure,
                    arena.policy.recycle
                ));
            }
            let generation = pool_slot_generation(arena, slot);
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(
                RuntimePoolIdValue {
                    arena: handle,
                    slot,
                    generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryPoolLen => {
            let handle = expect_pool_arena(expect_single_arg(args, "pool_len")?, "pool_len")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemoryPoolHas => {
            if args.len() != 2 {
                return Err("pool_has expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_has")?;
            let id = expect_pool_id(args[1].clone(), "pool_has")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(pool_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryPoolGet => {
            if args.len() != 2 {
                return Err("pool access expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_access")?;
            let id = expect_pool_id(args[1].clone(), "pool_access")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("PoolArena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryPoolBorrowRead | RuntimeIntrinsic::MemoryPoolBorrowEdit => {
            if args.len() != 2 {
                return Err("pool borrow expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_borrow")?;
            let id = expect_pool_id(args[1].clone(), "pool_borrow")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mode: runtime_reference_mode_for_place(matches!(
                    intrinsic,
                    RuntimeIntrinsic::MemoryPoolBorrowEdit
                )),
                target: RuntimeReferenceTarget::PoolSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryPoolSet => {
            if args.len() != 3 {
                return Err("pool_set expects three arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_set")?;
            let id = expect_pool_id(args[1].clone(), "pool_set")?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryPoolRemove => {
            if args.len() != 2 {
                return Err("pool_remove expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_remove")?;
            let id = expect_pool_id(args[1].clone(), "pool_remove")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_pool_id(reference, id),
                format!(
                    "pool_remove rejects invalidation while borrowed views for PoolId `{}` are live",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id)))
                ),
            )?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            arena.slots.remove(&id.slot);
            *arena.generations.entry(id.slot).or_insert(0) += 1;
            if matches!(arena.policy.recycle, RuntimePoolRecyclePolicy::FreeList) {
                arena.free_slots.push(id.slot);
            }
            Ok(RuntimeValue::Bool(true))
        }
        RuntimeIntrinsic::MemoryPoolReset => {
            let handle = expect_pool_arena(expect_single_arg(args, "pool_reset")?, "pool_reset")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_pool_arena(reference, handle),
                format!(
                    "pool_reset rejects invalidation while borrowed views for PoolArena `{}` are live",
                    handle.0
                ),
            )?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            arena.slots.clear();
            for generation in arena.generations.values_mut() {
                *generation += 1;
            }
            match arena.policy.recycle {
                RuntimePoolRecyclePolicy::FreeList => {
                    arena.free_slots = arena.generations.keys().copied().rev().collect();
                }
                RuntimePoolRecyclePolicy::Strict => {
                    arena.next_slot = 0;
                    arena.free_slots.clear();
                }
            }
            arena.policy.current_limit = arena.policy.base_capacity;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryPoolLiveIds => {
            let handle =
                expect_pool_arena(expect_single_arg(args, "pool_live_ids")?, "pool_live_ids")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::List(
                arena
                    .slots
                    .keys()
                    .copied()
                    .map(|slot| {
                        RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(RuntimePoolIdValue {
                            arena: handle,
                            slot,
                            generation: pool_slot_generation(arena, slot),
                        }))
                    })
                    .collect(),
            ))
        }
        RuntimeIntrinsic::MemoryPoolCompact => {
            let handle =
                expect_pool_arena(expect_single_arg(args, "pool_compact")?, "pool_compact")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_pool_arena(reference, handle),
                format!(
                    "pool_compact rejects invalidation while borrowed views for PoolArena `{}` are live",
                    handle.0
                ),
            )?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            let old_slots = arena.slots.keys().copied().collect::<Vec<_>>();
            let mut new_slots = BTreeMap::new();
            let mut new_generations = BTreeMap::new();
            let mut relocations = Vec::new();
            for (next_index, old_slot) in old_slots.iter().copied().enumerate() {
                let generation = pool_slot_generation(arena, old_slot);
                let next_generation = generation + 1;
                let value = arena
                    .slots
                    .remove(&old_slot)
                    .ok_or_else(|| format!("PoolArena slot `{old_slot}` is missing"))?;
                let new_slot = next_index as u64;
                new_slots.insert(new_slot, value);
                new_generations.insert(new_slot, next_generation);
                let mut fields = BTreeMap::new();
                fields.insert(
                    "old".to_string(),
                    RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(RuntimePoolIdValue {
                        arena: handle,
                        slot: old_slot,
                        generation,
                    })),
                );
                fields.insert(
                    "new".to_string(),
                    RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(RuntimePoolIdValue {
                        arena: handle,
                        slot: new_slot,
                        generation: next_generation,
                    })),
                );
                relocations.push(RuntimeValue::Record {
                    name: "std.memory.PoolRelocation".to_string(),
                    fields,
                });
            }
            arena.slots = new_slots;
            arena.generations = new_generations;
            arena.next_slot = arena.slots.len() as u64;
            arena.free_slots.clear();
            Ok(RuntimeValue::List(relocations))
        }
        RuntimeIntrinsic::MemoryTempNew => {
            let capacity = expect_int(expect_single_arg(args, "temp_new")?, "temp_new")?;
            let capacity = runtime_non_negative_usize(capacity, "temp_new capacity")?;
            let handle =
                insert_runtime_temp_arena(state, type_args, default_runtime_temp_policy(capacity));
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(handle)))
        }
        RuntimeIntrinsic::MemoryTempAlloc => {
            if args.len() != 2 {
                return Err("temp_alloc expects two arguments".to_string());
            }
            let handle = expect_temp_arena(args[0].clone(), "temp_alloc")?;
            let arena = state
                .temp_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", handle.0))?;
            ensure_runtime_temp_capacity(arena)?;
            let slot = arena.next_slot;
            arena.next_slot += 1;
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(
                RuntimeTempIdValue {
                    arena: handle,
                    slot,
                    generation: arena.generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryTempLen => {
            let handle = expect_temp_arena(expect_single_arg(args, "temp_len")?, "temp_len")?;
            let arena = state
                .temp_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemoryTempHas => {
            if args.len() != 2 {
                return Err("temp_has expects two arguments".to_string());
            }
            let handle = expect_temp_arena(args[0].clone(), "temp_has")?;
            let id = expect_temp_id(args[1].clone(), "temp_has")?;
            let arena = state
                .temp_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(temp_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryTempGet => {
            if args.len() != 2 {
                return Err("temp_get expects two arguments".to_string());
            }
            let handle = expect_temp_arena(args[0].clone(), "temp_get")?;
            let id = expect_temp_id(args[1].clone(), "temp_get")?;
            let arena = state
                .temp_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", handle.0))?;
            if !temp_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid TempId `{}` for TempArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("TempArena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryTempBorrowRead | RuntimeIntrinsic::MemoryTempBorrowEdit => {
            if args.len() != 2 {
                return Err("temp borrow expects two arguments".to_string());
            }
            let handle = expect_temp_arena(args[0].clone(), "temp_borrow")?;
            let id = expect_temp_id(args[1].clone(), "temp_borrow")?;
            let arena = state
                .temp_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", handle.0))?;
            if !temp_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid TempId `{}` for TempArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mode: runtime_reference_mode_for_place(matches!(
                    intrinsic,
                    RuntimeIntrinsic::MemoryTempBorrowEdit
                )),
                target: RuntimeReferenceTarget::TempSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryTempSet => {
            if args.len() != 3 {
                return Err("temp_set expects three arguments".to_string());
            }
            let handle = expect_temp_arena(args[0].clone(), "temp_set")?;
            let id = expect_temp_id(args[1].clone(), "temp_set")?;
            let arena = state
                .temp_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", handle.0))?;
            if !temp_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid TempId `{}` for TempArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryTempReset => {
            let handle = expect_temp_arena(expect_single_arg(args, "temp_reset")?, "temp_reset")?;
            runtime_reset_temp_arena_handle(state, handle, "temp_reset")?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySessionNew => {
            let capacity = expect_int(expect_single_arg(args, "session_new")?, "session_new")?;
            let capacity = runtime_non_negative_usize(capacity, "session_new capacity")?;
            let handle = insert_runtime_session_arena(
                state,
                type_args,
                default_runtime_session_policy(capacity),
            );
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(
                handle,
            )))
        }
        RuntimeIntrinsic::MemorySessionAlloc => {
            if args.len() != 2 {
                return Err("session_alloc expects two arguments".to_string());
            }
            let handle = expect_session_arena(args[0].clone(), "session_alloc")?;
            let arena = state
                .session_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            if arena.sealed {
                return Err("session_alloc rejects mutation while sealed".to_string());
            }
            ensure_runtime_session_capacity(arena)?;
            let slot = arena.next_slot;
            arena.next_slot += 1;
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                RuntimeSessionIdValue {
                    arena: handle,
                    slot,
                    generation: arena.generation,
                },
            )))
        }
        RuntimeIntrinsic::MemorySessionLen => {
            let handle =
                expect_session_arena(expect_single_arg(args, "session_len")?, "session_len")?;
            let arena = state
                .session_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemorySessionHas => {
            if args.len() != 2 {
                return Err("session_has expects two arguments".to_string());
            }
            let handle = expect_session_arena(args[0].clone(), "session_has")?;
            let id = expect_session_id(args[1].clone(), "session_has")?;
            let arena = state
                .session_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(session_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemorySessionGet => {
            if args.len() != 2 {
                return Err("session_get expects two arguments".to_string());
            }
            let handle = expect_session_arena(args[0].clone(), "session_get")?;
            let id = expect_session_id(args[1].clone(), "session_get")?;
            let arena = state
                .session_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            if !session_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid SessionId `{}` for SessionArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                        id
                    ))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("SessionArena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemorySessionBorrowRead | RuntimeIntrinsic::MemorySessionBorrowEdit => {
            if args.len() != 2 {
                return Err("session borrow expects two arguments".to_string());
            }
            let handle = expect_session_arena(args[0].clone(), "session_borrow")?;
            let id = expect_session_id(args[1].clone(), "session_borrow")?;
            let arena = state
                .session_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            if !session_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid SessionId `{}` for SessionArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                        id
                    ))),
                    handle.0
                ));
            }
            if arena.sealed && matches!(intrinsic, RuntimeIntrinsic::MemorySessionBorrowEdit) {
                return Err("session_borrow_edit rejects mutation while sealed".to_string());
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mode: runtime_reference_mode_for_place(matches!(
                    intrinsic,
                    RuntimeIntrinsic::MemorySessionBorrowEdit
                )),
                target: RuntimeReferenceTarget::SessionSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemorySessionSet => {
            if args.len() != 3 {
                return Err("session_set expects three arguments".to_string());
            }
            let handle = expect_session_arena(args[0].clone(), "session_set")?;
            let id = expect_session_id(args[1].clone(), "session_set")?;
            let arena = state
                .session_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            if arena.sealed {
                return Err("session_set rejects mutation while sealed".to_string());
            }
            if !session_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid SessionId `{}` for SessionArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                        id
                    ))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySessionReset => {
            let handle =
                expect_session_arena(expect_single_arg(args, "session_reset")?, "session_reset")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_session_arena(reference, handle),
                format!(
                    "session_reset rejects invalidation while borrowed views for SessionArena `{}` are live",
                    handle.0
                ),
            )?;
            let arena = state
                .session_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            if arena.sealed {
                return Err("session_reset rejects mutation while sealed".to_string());
            }
            arena.generation += 1;
            arena.next_slot = 0;
            arena.slots.clear();
            arena.policy.current_limit = arena.policy.base_capacity;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySessionSeal => {
            let handle =
                expect_session_arena(expect_single_arg(args, "session_seal")?, "session_seal")?;
            let arena = state
                .session_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            arena.sealed = true;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySessionUnseal => {
            let handle =
                expect_session_arena(expect_single_arg(args, "session_unseal")?, "session_unseal")?;
            runtime_reject_live_reference_or_opaque_conflict(
                scopes.as_ref().map(|scopes| scopes.as_slice()),
                Some(final_args.as_slice()),
                state,
                |reference| runtime_reference_targets_session_arena(reference, handle),
                |opaque, state| {
                    runtime_opaque_matches_reference_predicate(opaque, state, &|reference| {
                        runtime_reference_targets_session_arena(reference, handle)
                    })
                },
                None,
                None,
                |state| {
                    runtime_any_live_element_view_reference(state, |reference| {
                        runtime_reference_targets_session_arena(reference, handle)
                    })
                },
                format!(
                    "session_unseal rejects publication rollback while borrowed views or borrows for SessionArena `{}` are live",
                    handle.0
                ),
            )?;
            if state
                .exported_descriptor_counts
                .contains_key(&RuntimeExportedDescriptorTarget::SessionArena(handle))
            {
                return Err(format!(
                    "session_unseal rejects publication rollback while exported descriptor views for SessionArena `{}` are live",
                    handle.0
                ));
            }
            let arena = state
                .session_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            arena.sealed = false;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySessionIsSealed => {
            let handle = expect_session_arena(
                expect_single_arg(args, "session_is_sealed")?,
                "session_is_sealed",
            )?;
            let arena = state
                .session_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(arena.sealed))
        }
        RuntimeIntrinsic::MemorySessionLiveIds => {
            let handle = expect_session_arena(
                expect_single_arg(args, "session_live_ids")?,
                "session_live_ids",
            )?;
            let arena = state
                .session_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::List(
                arena
                    .slots
                    .keys()
                    .copied()
                    .map(|slot| {
                        RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(RuntimeSessionIdValue {
                            arena: handle,
                            slot,
                            generation: arena.generation,
                        }))
                    })
                    .collect(),
            ))
        }
        RuntimeIntrinsic::MemoryRingNew => {
            let capacity = expect_int(expect_single_arg(args, "ring_new")?, "ring_new")?;
            let capacity = runtime_non_negative_usize(capacity, "ring_new capacity")?;
            let handle =
                insert_runtime_ring_buffer(state, type_args, default_runtime_ring_policy(capacity));
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)))
        }
        RuntimeIntrinsic::MemoryRingPush => {
            if args.len() != 2 {
                return Err("ring_push expects two arguments".to_string());
            }
            let handle = expect_ring_buffer(args[0].clone(), "ring_push")?;
            if let Some((oldest_id, oldest_slot)) =
                state.ring_buffers.get(&handle).and_then(|arena| {
                    if arena.slots.len() < arena.policy.current_limit {
                        return None;
                    }
                    if matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic) {
                        return None;
                    }
                    if !matches!(arena.policy.overwrite, RuntimeRingOverwritePolicy::Oldest) {
                        return None;
                    }
                    let oldest_slot = *arena.order.front()?;
                    Some((
                        RuntimeRingIdValue {
                            arena: handle,
                            slot: oldest_slot,
                            generation: ring_slot_generation(arena, oldest_slot),
                        },
                        oldest_slot,
                    ))
                })
            {
                runtime_reject_live_reference_or_opaque_conflict(
                    scopes.as_ref().map(|scopes| scopes.as_slice()),
                    Some(args.as_slice()),
                    state,
                    |reference| runtime_reference_targets_ring_id(reference, oldest_id),
                    |opaque, state| {
                        runtime_opaque_matches_ring_window_predicate(
                            opaque,
                            state,
                            &|candidate_arena, candidate_slots| {
                                runtime_ring_window_overlaps_slots(
                                    handle,
                                    &[oldest_slot],
                                    candidate_arena,
                                    candidate_slots,
                                )
                            },
                        )
                    },
                    None,
                    None,
                    |_| false,
                    format!(
                        "ring_push rejects overwrite while borrowed views for RingId `{}` are live",
                        runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(
                            oldest_id
                        )))
                    ),
                )?;
            }
            let arena = state
                .ring_buffers
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            ensure_runtime_ring_capacity(arena)?;
            let slot = arena.next_slot;
            arena.next_slot += 1;
            let generation = ring_slot_generation(arena, slot);
            arena.slots.insert(slot, args[1].clone());
            arena.order.push_back(slot);
            arena.generations.entry(slot).or_insert(generation);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(
                RuntimeRingIdValue {
                    arena: handle,
                    slot,
                    generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryRingTryPop => {
            let handle = expect_ring_buffer(
                expect_single_arg(args.clone(), "ring_try_pop")?,
                "ring_try_pop",
            )?;
            if let Some((oldest_id, oldest_slot)) =
                state.ring_buffers.get(&handle).and_then(|arena| {
                    let oldest_slot = *arena.order.front()?;
                    Some((
                        RuntimeRingIdValue {
                            arena: handle,
                            slot: oldest_slot,
                            generation: ring_slot_generation(arena, oldest_slot),
                        },
                        oldest_slot,
                    ))
                })
            {
                runtime_reject_live_reference_or_opaque_conflict(
                    scopes.as_ref().map(|scopes| scopes.as_slice()),
                    Some(args.as_slice()),
                    state,
                    |reference| runtime_reference_targets_ring_id(reference, oldest_id),
                    |opaque, state| {
                        runtime_opaque_matches_ring_window_predicate(
                            opaque,
                            state,
                            &|candidate_arena, candidate_slots| {
                                runtime_ring_window_overlaps_slots(
                                    handle,
                                    &[oldest_slot],
                                    candidate_arena,
                                    candidate_slots,
                                )
                            },
                        )
                    },
                    None,
                    None,
                    |_| false,
                    format!(
                        "ring_try_pop rejects invalidation while borrowed views for RingId `{}` are live",
                        runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(
                            oldest_id
                        )))
                    ),
                )?;
            }
            let arena = state
                .ring_buffers
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            let Some(slot) = arena.order.pop_front() else {
                return Ok(none_variant());
            };
            let value = arena
                .slots
                .remove(&slot)
                .ok_or_else(|| format!("RingBuffer slot `{slot}` is missing"))?;
            *arena.generations.entry(slot).or_insert(0) += 1;
            Ok(some_variant(value))
        }
        RuntimeIntrinsic::MemoryRingLen => {
            let handle = expect_ring_buffer(expect_single_arg(args, "ring_len")?, "ring_len")?;
            let arena = state
                .ring_buffers
                .get(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.order.len() as i64))
        }
        RuntimeIntrinsic::MemoryRingHas => {
            if args.len() != 2 {
                return Err("ring_has expects two arguments".to_string());
            }
            let handle = expect_ring_buffer(args[0].clone(), "ring_has")?;
            let id = expect_ring_id(args[1].clone(), "ring_has")?;
            let arena = state
                .ring_buffers
                .get(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(ring_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryRingGet => {
            if args.len() != 2 {
                return Err("ring_get expects two arguments".to_string());
            }
            let handle = expect_ring_buffer(args[0].clone(), "ring_get")?;
            let id = expect_ring_id(args[1].clone(), "ring_get")?;
            let arena = state
                .ring_buffers
                .get(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            if !ring_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid RingId `{}` for RingBuffer `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("RingBuffer slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryRingBorrowRead | RuntimeIntrinsic::MemoryRingBorrowEdit => {
            if args.len() != 2 {
                return Err("ring borrow expects two arguments".to_string());
            }
            let handle = expect_ring_buffer(args[0].clone(), "ring_borrow")?;
            let id = expect_ring_id(args[1].clone(), "ring_borrow")?;
            let arena = state
                .ring_buffers
                .get(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            if !ring_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid RingId `{}` for RingBuffer `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mode: runtime_reference_mode_for_place(matches!(
                    intrinsic,
                    RuntimeIntrinsic::MemoryRingBorrowEdit
                )),
                target: RuntimeReferenceTarget::RingSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryRingSet => {
            if args.len() != 3 {
                return Err("ring_set expects three arguments".to_string());
            }
            let handle = expect_ring_buffer(args[0].clone(), "ring_set")?;
            let id = expect_ring_id(args[1].clone(), "ring_set")?;
            let arena = state
                .ring_buffers
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            if !ring_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid RingId `{}` for RingBuffer `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryRingReset => {
            let handle =
                expect_ring_buffer(expect_single_arg(args.clone(), "ring_reset")?, "ring_reset")?;
            runtime_reject_live_reference_or_opaque_conflict(
                scopes.as_ref().map(|scopes| scopes.as_slice()),
                Some(args.as_slice()),
                state,
                |reference| runtime_reference_targets_ring_arena(reference, handle),
                |opaque, state| {
                    runtime_opaque_matches_ring_window_predicate(
                        opaque,
                        state,
                        &|candidate_arena, _| candidate_arena == handle,
                    )
                },
                None,
                None,
                |_| false,
                format!(
                    "ring_reset rejects invalidation while borrowed views for RingBuffer `{}` are live",
                    handle.0
                ),
            )?;
            let arena = state
                .ring_buffers
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            for slot in arena.slots.keys().copied().collect::<Vec<_>>() {
                *arena.generations.entry(slot).or_insert(0) += 1;
            }
            arena.slots.clear();
            arena.order.clear();
            arena.policy.current_limit = arena.policy.base_capacity;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySlabNew => {
            let capacity = expect_int(expect_single_arg(args, "slab_new")?, "slab_new")?;
            let capacity = runtime_non_negative_usize(capacity, "slab_new capacity")?;
            let handle =
                insert_runtime_slab(state, type_args, default_runtime_slab_policy(capacity));
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle)))
        }
        RuntimeIntrinsic::MemorySlabAlloc => {
            if args.len() != 2 {
                return Err("slab_alloc expects two arguments".to_string());
            }
            let handle = expect_slab(args[0].clone(), "slab_alloc")?;
            let arena = state
                .slabs
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            if arena.sealed {
                return Err("slab_alloc rejects mutation while sealed".to_string());
            }
            ensure_runtime_slab_capacity(arena)?;
            let slot = arena.free_slots.pop().unwrap_or_else(|| {
                let next = arena.next_slot;
                arena.next_slot += 1;
                arena.generations.entry(next).or_insert(0);
                next
            });
            let generation = slab_slot_generation(arena, slot);
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(
                RuntimeSlabIdValue {
                    arena: handle,
                    slot,
                    generation,
                },
            )))
        }
        RuntimeIntrinsic::MemorySlabLen => {
            let handle = expect_slab(expect_single_arg(args, "slab_len")?, "slab_len")?;
            let arena = state
                .slabs
                .get(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemorySlabHas => {
            if args.len() != 2 {
                return Err("slab_has expects two arguments".to_string());
            }
            let handle = expect_slab(args[0].clone(), "slab_has")?;
            let id = expect_slab_id(args[1].clone(), "slab_has")?;
            let arena = state
                .slabs
                .get(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(slab_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemorySlabGet => {
            if args.len() != 2 {
                return Err("slab_get expects two arguments".to_string());
            }
            let handle = expect_slab(args[0].clone(), "slab_get")?;
            let id = expect_slab_id(args[1].clone(), "slab_get")?;
            let arena = state
                .slabs
                .get(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            if !slab_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("Slab slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemorySlabBorrowRead | RuntimeIntrinsic::MemorySlabBorrowEdit => {
            if args.len() != 2 {
                return Err("slab borrow expects two arguments".to_string());
            }
            let handle = expect_slab(args[0].clone(), "slab_borrow")?;
            let id = expect_slab_id(args[1].clone(), "slab_borrow")?;
            let arena = state
                .slabs
                .get(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            if !slab_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id))),
                    handle.0
                ));
            }
            if arena.sealed && matches!(intrinsic, RuntimeIntrinsic::MemorySlabBorrowEdit) {
                return Err("slab_borrow_edit rejects mutation while sealed".to_string());
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mode: runtime_reference_mode_for_place(matches!(
                    intrinsic,
                    RuntimeIntrinsic::MemorySlabBorrowEdit
                )),
                target: RuntimeReferenceTarget::SlabSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemorySlabSet => {
            if args.len() != 3 {
                return Err("slab_set expects three arguments".to_string());
            }
            let handle = expect_slab(args[0].clone(), "slab_set")?;
            let id = expect_slab_id(args[1].clone(), "slab_set")?;
            let arena = state
                .slabs
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            if arena.sealed {
                return Err("slab_set rejects mutation while sealed".to_string());
            }
            if !slab_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySlabRemove => {
            if args.len() != 2 {
                return Err("slab_remove expects two arguments".to_string());
            }
            let handle = expect_slab(args[0].clone(), "slab_remove")?;
            let id = expect_slab_id(args[1].clone(), "slab_remove")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_slab_id(reference, id),
                format!(
                    "slab_remove rejects invalidation while borrowed views for SlabId `{}` are live",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id)))
                ),
            )?;
            let arena = state
                .slabs
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            if arena.sealed {
                return Err("slab_remove rejects mutation while sealed".to_string());
            }
            if !slab_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id))),
                    handle.0
                ));
            }
            arena.slots.remove(&id.slot);
            *arena.generations.entry(id.slot).or_insert(0) += 1;
            arena.free_slots.push(id.slot);
            Ok(RuntimeValue::Bool(true))
        }
        RuntimeIntrinsic::MemorySlabReset => {
            let handle = expect_slab(expect_single_arg(args, "slab_reset")?, "slab_reset")?;
            runtime_reject_live_view_conflict(
                state,
                |reference| runtime_reference_targets_slab_arena(reference, handle),
                format!(
                    "slab_reset rejects invalidation while borrowed views for Slab `{}` are live",
                    handle.0
                ),
            )?;
            let arena = state
                .slabs
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            if arena.sealed {
                return Err("slab_reset rejects mutation while sealed".to_string());
            }
            arena.slots.clear();
            for generation in arena.generations.values_mut() {
                *generation += 1;
            }
            arena.free_slots = arena.generations.keys().copied().rev().collect();
            arena.policy.current_limit = arena.policy.base_capacity;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySlabSeal => {
            let handle = expect_slab(expect_single_arg(args, "slab_seal")?, "slab_seal")?;
            let arena = state
                .slabs
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            arena.sealed = true;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySlabUnseal => {
            let handle = expect_slab(expect_single_arg(args, "slab_unseal")?, "slab_unseal")?;
            runtime_reject_live_reference_or_opaque_conflict(
                scopes.as_ref().map(|scopes| scopes.as_slice()),
                Some(final_args.as_slice()),
                state,
                |reference| runtime_reference_targets_slab_arena(reference, handle),
                |opaque, state| {
                    runtime_opaque_matches_reference_predicate(opaque, state, &|reference| {
                        runtime_reference_targets_slab_arena(reference, handle)
                    })
                },
                None,
                None,
                |state| {
                    runtime_any_live_element_view_reference(state, |reference| {
                        runtime_reference_targets_slab_arena(reference, handle)
                    })
                },
                format!(
                    "slab_unseal rejects publication rollback while borrowed views or borrows for Slab `{}` are live",
                    handle.0
                ),
            )?;
            if state
                .exported_descriptor_counts
                .contains_key(&RuntimeExportedDescriptorTarget::Slab(handle))
            {
                return Err(format!(
                    "slab_unseal rejects publication rollback while exported descriptor views for Slab `{}` are live",
                    handle.0
                ));
            }
            let arena = state
                .slabs
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            arena.sealed = false;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemorySlabIsSealed => {
            let handle = expect_slab(expect_single_arg(args, "slab_is_sealed")?, "slab_is_sealed")?;
            let arena = state
                .slabs
                .get(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(arena.sealed))
        }
        RuntimeIntrinsic::MemorySlabLiveIds => {
            let handle = expect_slab(expect_single_arg(args, "slab_live_ids")?, "slab_live_ids")?;
            let arena = state
                .slabs
                .get(&handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            Ok(RuntimeValue::List(
                arena
                    .slots
                    .keys()
                    .copied()
                    .map(|slot| {
                        RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(RuntimeSlabIdValue {
                            arena: handle,
                            slot,
                            generation: slab_slot_generation(arena, slot),
                        }))
                    })
                    .collect(),
            ))
        }
        RuntimeIntrinsic::MemoryRingWindowRead | RuntimeIntrinsic::MemoryRingWindowEdit => {
            if args.len() != 3 {
                return Err("ring_window expects three arguments".to_string());
            }
            let handle = expect_ring_buffer(args[0].clone(), "ring_window")?;
            let start = expect_int(args[1].clone(), "ring_window start")?;
            let len = expect_int(args[2].clone(), "ring_window len")?;
            if len < 0 {
                return Err("ring_window len must be non-negative".to_string());
            }
            let arena = state
                .ring_buffers
                .get(&handle)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", handle.0))?;
            let start = runtime_non_negative_usize(start, "ring_window start")?;
            let count = runtime_non_negative_usize(len, "ring_window len")?;
            if count > arena.policy.window {
                return Err(format!(
                    "ring_window len `{count}` exceeds configured window `{}` for RingBuffer `{}`",
                    arena.policy.window, handle.0
                ));
            }
            let end = start
                .checked_add(count)
                .ok_or_else(|| "ring_window range overflowed".to_string())?;
            if end > arena.order.len() {
                return Err(format!(
                    "ring_window `{start}..{end}` is out of bounds for length `{}`",
                    arena.order.len()
                ));
            }
            let slots = arena
                .order
                .iter()
                .skip(start)
                .take(count)
                .copied()
                .collect::<Vec<_>>();
            if matches!(intrinsic, RuntimeIntrinsic::MemoryRingWindowEdit) {
                let ids = runtime_ring_ids_for_slots(state, handle, &slots)?;
                runtime_reject_live_reference_or_opaque_conflict(
                    scopes.as_ref().map(|scopes| scopes.as_slice()),
                    Some(args.as_slice()),
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
                    "ring_window_edit rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
                )?;
                let view = insert_runtime_edit_view_from_ring_window(
                    state, type_args, handle, slots, 0, count,
                );
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(view)))
            } else {
                let view = insert_runtime_read_view_from_ring_window(
                    state, type_args, handle, slots, 0, count,
                );
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(view)))
            }
        }
        RuntimeIntrinsic::MemoryArrayViewRead | RuntimeIntrinsic::MemoryArrayViewEdit => {
            if args.len() != 3 {
                return Err("array_view expects three arguments".to_string());
            }
            let reference = match &args[0] {
                RuntimeValue::Ref(reference) => Some(reference.clone()),
                _ => None,
            };
            let values = if let Some(reference) = reference.as_ref() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "array_view on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "array_view on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "array_view on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                expect_runtime_array(
                    read_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        reference,
                        host,
                    )?,
                    "array_view",
                )?
            } else {
                expect_runtime_array(args[0].clone(), "array_view")?
            };
            let start = expect_int(args[1].clone(), "array_view start")?;
            let end = expect_int(args[2].clone(), "array_view end")?;
            let (start, end) = runtime_view_bounds(start, end, values.len(), "array_view")?;
            if let Some(reference) = reference {
                if matches!(intrinsic, RuntimeIntrinsic::MemoryArrayViewEdit) {
                    runtime_reject_live_reference_or_opaque_conflict(
                        scopes.as_ref().map(|scopes| scopes.as_slice()),
                        Some(args.as_slice()),
                        state,
                        |candidate| candidate.target == reference.target,
                        |opaque, state| runtime_opaque_matches_reference_predicate(
                            opaque,
                            state,
                            &|candidate| candidate.target == reference.target,
                        ),
                        Some(&reference),
                        None,
                        |_| false,
                        "array_view_edit rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
                    )?;
                    let view = insert_runtime_edit_view_from_reference(
                        state,
                        type_args,
                        reference,
                        start,
                        end - start,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(view)))
                } else {
                    let view = insert_runtime_read_view_from_reference(
                        state,
                        type_args,
                        reference,
                        start,
                        end - start,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(view)))
                }
            } else {
                let backing = insert_runtime_element_view_buffer(state, type_args, values);
                if matches!(intrinsic, RuntimeIntrinsic::MemoryArrayViewEdit) {
                    let view = insert_runtime_edit_view_from_buffer(
                        state,
                        type_args,
                        backing,
                        start,
                        end - start,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(view)))
                } else {
                    let view = insert_runtime_read_view_from_buffer(
                        state,
                        type_args,
                        backing,
                        start,
                        end - start,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(view)))
                }
            }
        }
        RuntimeIntrinsic::MemoryBytesView | RuntimeIntrinsic::MemoryBytesViewEdit => {
            if args.len() != 3 {
                return Err("bytes_view expects three arguments".to_string());
            }
            let reference = match &args[0] {
                RuntimeValue::Ref(reference) => Some(reference.clone()),
                _ => None,
            };
            let values = if let Some(reference) = reference.as_ref() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "bytes_view on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "bytes_view on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "bytes_view on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                runtime_reference_array_values(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    reference,
                    host,
                    "bytes_view",
                )?
            } else {
                expect_runtime_array(args[0].clone(), "bytes_view")?
            };
            let byte_count = values
                .into_iter()
                .map(|value| {
                    let value = expect_int(value, "bytes_view")?;
                    if !(0..=255).contains(&value) {
                        return Err(format!(
                            "bytes_view byte `{value}` is out of range `0..=255`"
                        ));
                    }
                    Ok(())
                })
                .collect::<Result<Vec<_>, _>>()?
                .len();
            let start = expect_int(args[1].clone(), "bytes_view start")?;
            let end = expect_int(args[2].clone(), "bytes_view end")?;
            let (start, end) = runtime_view_bounds(start, end, byte_count, "bytes_view")?;
            if let Some(reference) = reference {
                if matches!(intrinsic, RuntimeIntrinsic::MemoryBytesViewEdit) {
                    runtime_reject_live_reference_or_opaque_conflict(
                        scopes.as_ref().map(|scopes| scopes.as_slice()),
                        Some(args.as_slice()),
                        state,
                        |candidate| candidate.target == reference.target,
                        |opaque, state| runtime_opaque_matches_reference_predicate(
                            opaque,
                            state,
                            &|candidate| candidate.target == reference.target,
                        ),
                        Some(&reference),
                        None,
                        |_| false,
                        "bytes_view_edit rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
                    )?;
                    let view = insert_runtime_byte_edit_view_from_reference(
                        state,
                        reference,
                        start,
                        end - start,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(view)))
                } else {
                    let view = insert_runtime_byte_view_from_reference(
                        state,
                        reference,
                        start,
                        end - start,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(view)))
                }
            } else {
                let bytes = expect_runtime_array(args[0].clone(), "bytes_view")?
                    .into_iter()
                    .map(|value| {
                        let value = expect_int(value, "bytes_view")?;
                        if !(0..=255).contains(&value) {
                            return Err(format!(
                                "bytes_view byte `{value}` is out of range `0..=255`"
                            ));
                        }
                        Ok(value as u8)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let backing = insert_runtime_byte_view_buffer(state, bytes);
                if matches!(intrinsic, RuntimeIntrinsic::MemoryBytesViewEdit) {
                    let view = insert_runtime_byte_edit_view_from_buffer(
                        state,
                        backing,
                        start,
                        end - start,
                    );
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(view)))
                } else {
                    let view =
                        insert_runtime_byte_view_from_buffer(state, backing, start, end - start);
                    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(view)))
                }
            }
        }
        RuntimeIntrinsic::MemoryMappedView | RuntimeIntrinsic::MemoryMappedViewEdit => {
            if args.len() != 3 {
                return Err("mapped_view expects three arguments".to_string());
            }
            let package_id = expect_str(args[0].clone(), "mapped_view package")?;
            let handle = expect_int(args[1].clone(), "mapped_view handle")?;
            if handle < 0 {
                return Err(format!("mapped_view handle `{handle}` must be >= 0"));
            }
            let len = expect_int(args[2].clone(), "mapped_view len")?;
            if len < 0 {
                return Err(format!("mapped_view len `{len}` must be >= 0"));
            }
            let foreign = RuntimeForeignByteViewBacking {
                package_id: leak_runtime_binding_text(&package_id),
                handle: handle as u64,
            };
            let backing_len = runtime_foreign_byte_len(plan, host, foreign, "mapped_view")?;
            let len = len as usize;
            if len > backing_len {
                return Err(format!(
                    "mapped_view len `{len}` exceeds foreign backing length `{backing_len}`"
                ));
            }
            if matches!(intrinsic, RuntimeIntrinsic::MemoryMappedViewEdit) {
                runtime_reject_live_reference_or_opaque_conflict(
                    scopes.as_ref().map(|scopes| scopes.as_slice()),
                    Some(args.as_slice()),
                    state,
                    |_| false,
                    |opaque, state| {
                        runtime_opaque_matches_foreign_byte_handle(
                            opaque,
                            state,
                            foreign.package_id,
                            foreign.handle,
                        )
                    },
                    None,
                    None,
                    |_| false,
                    "mapped_view_edit rejects exclusive view acquisition while conflicting views are live".to_string(),
                )?;
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(
                    insert_runtime_byte_edit_view_from_foreign(
                        state,
                        foreign.package_id,
                        foreign.handle,
                        0,
                        len,
                    ),
                )))
            } else {
                Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(
                    insert_runtime_byte_view_from_foreign(
                        state,
                        foreign.package_id,
                        foreign.handle,
                        0,
                        len,
                    ),
                )))
            }
        }
        RuntimeIntrinsic::MemoryStrView => {
            if args.len() != 3 {
                return Err("str_view expects three arguments".to_string());
            }
            let reference = match &args[0] {
                RuntimeValue::Ref(reference) => Some(reference.clone()),
                _ => None,
            };
            let text = if let Some(reference) = reference.as_ref() {
                let scopes = scopes
                    .as_deref_mut()
                    .ok_or_else(|| "str_view on refs requires runtime scopes".to_string())?;
                let current_package_id = current_package_id
                    .ok_or_else(|| "str_view on refs requires package context".to_string())?;
                let current_module_id = current_module_id
                    .ok_or_else(|| "str_view on refs requires module context".to_string())?;
                let empty_aliases = BTreeMap::new();
                let aliases = aliases.unwrap_or(&empty_aliases);
                let empty_type_bindings = BTreeMap::new();
                let type_bindings = type_bindings.unwrap_or(&empty_type_bindings);
                runtime_reference_text_value(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    reference,
                    host,
                    "str_view",
                )?
            } else {
                expect_str(args[0].clone(), "str_view")?
            };
            let start = expect_int(args[1].clone(), "str_view start")?;
            let end = expect_int(args[2].clone(), "str_view end")?;
            let (start, end) = runtime_view_bounds(start, end, text.len(), "str_view")?;
            let view = if let Some(reference) = reference {
                insert_runtime_str_view_from_reference(state, reference, start, end - start)
            } else {
                let backing = insert_runtime_str_view_buffer(state, text);
                insert_runtime_str_view_from_buffer(state, backing, start, end - start)
            };
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(view)))
        }
        RuntimeIntrinsic::MemoryViewLen => match expect_single_arg(args, "view_len")? {
            RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
                let (backing, view_start, view_len) = {
                    let view = state
                        .read_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ReadView handle `{}`", handle.0))?;
                    (view.backing.clone(), view.start, view.len)
                };
                match &backing {
                    RuntimeElementViewBacking::Buffer(buffer) => {
                        let values = &state
                            .element_view_buffers
                            .get(buffer)
                            .ok_or_else(|| format!("invalid element view buffer `{}`", buffer.0))?
                            .values;
                        let _ = runtime_view_range(view_start, view_len, values.len(), "view_len")?;
                    }
                    RuntimeElementViewBacking::Reference(reference) => {
                        let scopes = scopes.as_deref_mut().ok_or_else(|| {
                            "view_len on reference-backed View requires runtime call context"
                                .to_string()
                        })?;
                        let values = runtime_reference_array_values(
                            scopes,
                            plan,
                            current_package_id.ok_or_else(|| {
                                "view_len is missing current package context".to_string()
                            })?,
                            current_module_id.ok_or_else(|| {
                                "view_len is missing current module context".to_string()
                            })?,
                            aliases
                                .ok_or_else(|| "view_len is missing alias context".to_string())?,
                            type_bindings.ok_or_else(|| {
                                "view_len is missing type binding context".to_string()
                            })?,
                            state,
                            reference,
                            host,
                            "view_len",
                        )?;
                        let _ = runtime_view_range(view_start, view_len, values.len(), "view_len")?;
                    }
                    RuntimeElementViewBacking::RingWindow { slots, .. } => {
                        let _ = runtime_view_range(view_start, view_len, slots.len(), "view_len")?;
                    }
                }
                Ok(RuntimeValue::Int(view_len as i64))
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
                let (backing, view_start, view_len) = {
                    let view = state
                        .edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?;
                    (view.backing.clone(), view.start, view.len)
                };
                match &backing {
                    RuntimeElementViewBacking::Buffer(buffer) => {
                        let values = &state
                            .element_view_buffers
                            .get(buffer)
                            .ok_or_else(|| format!("invalid element view buffer `{}`", buffer.0))?
                            .values;
                        let _ = runtime_view_range(view_start, view_len, values.len(), "view_len")?;
                    }
                    RuntimeElementViewBacking::Reference(reference) => {
                        let scopes = scopes.as_deref_mut().ok_or_else(|| {
                            "view_len on reference-backed View requires runtime call context"
                                .to_string()
                        })?;
                        let values = runtime_reference_array_values(
                            scopes,
                            plan,
                            current_package_id.ok_or_else(|| {
                                "view_len is missing current package context".to_string()
                            })?,
                            current_module_id.ok_or_else(|| {
                                "view_len is missing current module context".to_string()
                            })?,
                            aliases
                                .ok_or_else(|| "view_len is missing alias context".to_string())?,
                            type_bindings.ok_or_else(|| {
                                "view_len is missing type binding context".to_string()
                            })?,
                            state,
                            reference,
                            host,
                            "view_len",
                        )?;
                        let _ = runtime_view_range(view_start, view_len, values.len(), "view_len")?;
                    }
                    RuntimeElementViewBacking::RingWindow { slots, .. } => {
                        let _ = runtime_view_range(view_start, view_len, slots.len(), "view_len")?;
                    }
                }
                Ok(RuntimeValue::Int(view_len as i64))
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
                let view = state
                    .byte_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                    .clone();
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "view_len on mapped views requires runtime call context".to_string()
                })?;
                let _ = runtime_byte_view_values(
                    scopes,
                    plan,
                    current_package_id
                        .ok_or_else(|| "view_len is missing current package context".to_string())?,
                    current_module_id
                        .ok_or_else(|| "view_len is missing current module context".to_string())?,
                    aliases.ok_or_else(|| "view_len is missing alias context".to_string())?,
                    type_bindings
                        .ok_or_else(|| "view_len is missing type binding context".to_string())?,
                    state,
                    &view,
                    host,
                    "view_len",
                )?;
                Ok(RuntimeValue::Int(view.len as i64))
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                let view = state
                    .byte_edit_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                    .clone();
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "view_len on mapped edit views requires runtime call context".to_string()
                })?;
                let _ = runtime_byte_edit_view_values(
                    scopes,
                    plan,
                    current_package_id
                        .ok_or_else(|| "view_len is missing current package context".to_string())?,
                    current_module_id
                        .ok_or_else(|| "view_len is missing current module context".to_string())?,
                    aliases.ok_or_else(|| "view_len is missing alias context".to_string())?,
                    type_bindings
                        .ok_or_else(|| "view_len is missing type binding context".to_string())?,
                    state,
                    &view,
                    host,
                    "view_len",
                )?;
                Ok(RuntimeValue::Int(view.len as i64))
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
                let view = state
                    .str_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                    .clone();
                let scopes = scopes.as_deref_mut().ok_or_else(|| {
                    "view_len on borrowed text view requires runtime call context".to_string()
                })?;
                let _ = runtime_str_view_text(
                    scopes,
                    plan,
                    current_package_id
                        .ok_or_else(|| "view_len is missing current package context".to_string())?,
                    current_module_id
                        .ok_or_else(|| "view_len is missing current module context".to_string())?,
                    aliases.ok_or_else(|| "view_len is missing alias context".to_string())?,
                    type_bindings
                        .ok_or_else(|| "view_len is missing type binding context".to_string())?,
                    state,
                    &view,
                    host,
                    "view_len",
                )?;
                Ok(RuntimeValue::Int(view.len as i64))
            }
            other => Err(format!("view_len expected View, got `{other:?}`")),
        },
        RuntimeIntrinsic::MemoryViewGet => {
            if args.len() != 2 {
                return Err("view_get expects two arguments".to_string());
            }
            match args[0].clone() {
                RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
                    let view = state
                        .read_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ReadView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "view_get index")?,
                        view.len,
                        "view_get",
                    )?;
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "view_get on borrowed views requires runtime call context".to_string()
                    })?;
                    Ok(runtime_read_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "view_get is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "view_get is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| "view_get is missing alias context".to_string())?,
                        type_bindings.ok_or_else(|| {
                            "view_get is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "view_get",
                    )?[index]
                        .clone())
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
                    let view = state
                        .edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "view_get index")?,
                        view.len,
                        "view_get",
                    )?;
                    let read_view = RuntimeReadViewState {
                        type_args: view.type_args.clone(),
                        backing: view.backing.clone(),
                        start: view.start,
                        len: view.len,
                    };
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "view_get on borrowed views requires runtime call context".to_string()
                    })?;
                    Ok(runtime_read_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "view_get is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "view_get is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| "view_get is missing alias context".to_string())?,
                        type_bindings.ok_or_else(|| {
                            "view_get is missing type binding context".to_string()
                        })?,
                        state,
                        &read_view,
                        host,
                        "view_get",
                    )?[index]
                        .clone())
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
                    let view = state
                        .byte_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "view_get index")?,
                        view.len,
                        "view_get",
                    )?;
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "view_get on mapped views requires runtime call context".to_string()
                    })?;
                    let value = runtime_byte_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "view_get is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "view_get is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| "view_get is missing alias context".to_string())?,
                        type_bindings.ok_or_else(|| {
                            "view_get is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "view_get",
                    )?[index];
                    Ok(RuntimeValue::Int(i64::from(value)))
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                    let view = state
                        .byte_edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "view_get index")?,
                        view.len,
                        "view_get",
                    )?;
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "view_get on mapped edit views requires runtime call context".to_string()
                    })?;
                    let value = runtime_byte_edit_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "view_get is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "view_get is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| "view_get is missing alias context".to_string())?,
                        type_bindings.ok_or_else(|| {
                            "view_get is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "view_get",
                    )?[index];
                    Ok(RuntimeValue::Int(i64::from(value)))
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
                    let view = state
                        .str_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "view_get index")?,
                        view.len,
                        "view_get",
                    )?;
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "view_get on borrowed text views requires runtime call context".to_string()
                    })?;
                    let text = runtime_str_view_text(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "view_get is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "view_get is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| "view_get is missing alias context".to_string())?,
                        type_bindings.ok_or_else(|| {
                            "view_get is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "view_get",
                    )?;
                    let value = text
                        .as_bytes()
                        .get(index)
                        .copied()
                        .ok_or_else(|| format!("view_get index `{index}` is out of bounds"))?;
                    Ok(RuntimeValue::Int(i64::from(value)))
                }
                other => Err(format!("view_get expected View, got `{other:?}`")),
            }
        }
        RuntimeIntrinsic::MemoryViewSubview => {
            if args.len() != 3 {
                return Err("view_subview expects three arguments".to_string());
            }
            let start = expect_int(args[1].clone(), "view_subview start")?;
            let end = expect_int(args[2].clone(), "view_subview end")?;
            match args[0].clone() {
                RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
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
                    let (start, end) = runtime_view_bounds(start, end, view_len, "view_subview")?;
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
                    let (start, end) = runtime_view_bounds(start, end, view_len, "view_subview")?;
                    let next = match backing {
                        RuntimeElementViewBacking::Buffer(buffer) => {
                            insert_runtime_edit_view_from_buffer(
                                state,
                                &type_args,
                                buffer,
                                view_start + start,
                                end - start,
                            )
                        }
                        RuntimeElementViewBacking::Reference(reference) => {
                            insert_runtime_edit_view_from_reference(
                                state,
                                &type_args,
                                reference,
                                view_start + start,
                                end - start,
                            )
                        }
                        RuntimeElementViewBacking::RingWindow { arena, slots } => {
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
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
                    let view = state
                        .byte_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                        .clone();
                    let (start, end) = runtime_view_bounds(start, end, view.len, "view_subview")?;
                    let next = match view.backing {
                        RuntimeByteViewBacking::Buffer(buffer) => {
                            insert_runtime_byte_view_from_buffer(
                                state,
                                buffer,
                                view.start + start,
                                end - start,
                            )
                        }
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
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                    let view = state
                        .byte_edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                        .clone();
                    let (start, end) = runtime_view_bounds(start, end, view.len, "view_subview")?;
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
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
                    let view = state
                        .str_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                        .clone();
                    let (start, end) = runtime_view_bounds(start, end, view.len, "view_subview")?;
                    let next = match view.backing {
                        RuntimeStrViewBacking::Buffer(buffer) => {
                            insert_runtime_str_view_from_buffer(
                                state,
                                buffer,
                                view.start + start,
                                end - start,
                            )
                        }
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
                other => Err(format!("view_subview expected View, got `{other:?}`")),
            }
        }
        RuntimeIntrinsic::MemoryEditViewLen => {
            let handle =
                expect_edit_view(expect_single_arg(args, "edit_view_len")?, "edit_view_len")?;
            let (backing, view_start, view_len) = {
                let view = state
                    .edit_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?;
                (view.backing.clone(), view.start, view.len)
            };
            match &backing {
                RuntimeElementViewBacking::Buffer(buffer) => {
                    let values = &state
                        .element_view_buffers
                        .get(buffer)
                        .ok_or_else(|| format!("invalid element view buffer `{}`", buffer.0))?
                        .values;
                    let _ =
                        runtime_view_range(view_start, view_len, values.len(), "edit_view_len")?;
                }
                RuntimeElementViewBacking::Reference(reference) => {
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "edit_view_len on reference-backed EditView requires runtime call context"
                            .to_string()
                    })?;
                    let values = runtime_reference_array_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "edit_view_len is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "edit_view_len is missing current module context".to_string()
                        })?,
                        aliases
                            .ok_or_else(|| "edit_view_len is missing alias context".to_string())?,
                        type_bindings.ok_or_else(|| {
                            "edit_view_len is missing type binding context".to_string()
                        })?,
                        state,
                        reference,
                        host,
                        "edit_view_len",
                    )?;
                    let _ =
                        runtime_view_range(view_start, view_len, values.len(), "edit_view_len")?;
                }
                RuntimeElementViewBacking::RingWindow { slots, .. } => {
                    let _ = runtime_view_range(view_start, view_len, slots.len(), "edit_view_len")?;
                }
            }
            Ok(RuntimeValue::Int(view_len as i64))
        }
        RuntimeIntrinsic::MemoryEditViewGet => {
            if args.len() != 2 {
                return Err("edit_view_get expects two arguments".to_string());
            }
            let handle = expect_edit_view(args[0].clone(), "edit_view_get")?;
            let view = state
                .edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?
                .clone();
            let index = runtime_index_to_usize(
                expect_int(args[1].clone(), "edit_view_get index")?,
                view.len,
                "edit_view_get",
            )?;
            let read_view = RuntimeReadViewState {
                type_args: view.type_args.clone(),
                backing: view.backing.clone(),
                start: view.start,
                len: view.len,
            };
            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                "edit_view_get on borrowed views requires runtime call context".to_string()
            })?;
            Ok(runtime_read_view_values(
                scopes,
                plan,
                current_package_id.ok_or_else(|| {
                    "edit_view_get is missing current package context".to_string()
                })?,
                current_module_id
                    .ok_or_else(|| "edit_view_get is missing current module context".to_string())?,
                aliases.ok_or_else(|| "edit_view_get is missing alias context".to_string())?,
                type_bindings
                    .ok_or_else(|| "edit_view_get is missing type binding context".to_string())?,
                state,
                &read_view,
                host,
                "edit_view_get",
            )?[index]
                .clone())
        }
        RuntimeIntrinsic::MemoryEditViewSet => {
            if args.len() != 3 {
                return Err("edit_view_set expects three arguments".to_string());
            }
            match args[0].clone() {
                RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
                    let view = state
                        .edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid EditView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "edit_view_set index")?,
                        view.len,
                        "edit_view_set",
                    )?;
                    match view.backing {
                        RuntimeElementViewBacking::Buffer(buffer) => {
                            let values = &mut state
                                .element_view_buffers
                                .get_mut(&buffer)
                                .ok_or_else(|| {
                                    format!("invalid element view buffer `{}`", buffer.0)
                                })?
                                .values;
                            let absolute = view.start + index;
                            if absolute >= values.len() {
                                return Err(format!(
                                    "edit_view_set index `{index}` is out of bounds for length `{}`",
                                    view.len
                                ));
                            }
                            values[absolute] = args[2].clone();
                        }
                        RuntimeElementViewBacking::Reference(reference) => {
                            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                                "edit_view_set on borrowed views requires runtime call context"
                                    .to_string()
                            })?;
                            let mut values = runtime_reference_array_values(
                                scopes,
                                plan,
                                current_package_id.ok_or_else(|| {
                                    "edit_view_set is missing current package context".to_string()
                                })?,
                                current_module_id.ok_or_else(|| {
                                    "edit_view_set is missing current module context".to_string()
                                })?,
                                aliases.ok_or_else(|| {
                                    "edit_view_set is missing alias context".to_string()
                                })?,
                                type_bindings.ok_or_else(|| {
                                    "edit_view_set is missing type binding context".to_string()
                                })?,
                                state,
                                &reference,
                                host,
                                "edit_view_set",
                            )?;
                            let absolute = view.start + index;
                            if absolute >= values.len() {
                                return Err(format!(
                                    "edit_view_set index `{index}` is out of bounds for length `{}`",
                                    view.len
                                ));
                            }
                            values[absolute] = args[2].clone();
                            write_runtime_reference(
                                scopes,
                                plan,
                                current_package_id.ok_or_else(|| {
                                    "edit_view_set is missing current package context".to_string()
                                })?,
                                current_module_id.ok_or_else(|| {
                                    "edit_view_set is missing current module context".to_string()
                                })?,
                                aliases.ok_or_else(|| {
                                    "edit_view_set is missing alias context".to_string()
                                })?,
                                type_bindings.ok_or_else(|| {
                                    "edit_view_set is missing type binding context".to_string()
                                })?,
                                state,
                                &reference,
                                RuntimeValue::Array(values),
                                host,
                            )?;
                        }
                        RuntimeElementViewBacking::RingWindow { arena, slots } => {
                            let absolute = view.start + index;
                            let slot = *slots.get(absolute).ok_or_else(|| {
                                format!(
                                    "edit_view_set index `{index}` is out of bounds for length `{}`",
                                    view.len
                                )
                            })?;
                            let ring = state.ring_buffers.get_mut(&arena).ok_or_else(|| {
                                format!("invalid RingBuffer handle `{}`", arena.0)
                            })?;
                            let entry = ring
                                .slots
                                .get_mut(&slot)
                                .ok_or_else(|| format!("RingBuffer slot `{slot}` is missing"))?;
                            *entry = args[2].clone();
                        }
                    }
                    Ok(RuntimeValue::Unit)
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                    let view = state
                        .byte_edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "view_set index")?,
                        view.len,
                        "view_set",
                    )?;
                    let byte = expect_int(args[2].clone(), "view_set value")?;
                    if !(0..=255).contains(&byte) {
                        return Err(format!("view_set value `{byte}` is out of range `0..=255`"));
                    }
                    match &view.backing {
                        RuntimeByteViewBacking::Buffer(buffer) => {
                            let values = &mut state
                                .byte_view_buffers
                                .get_mut(buffer)
                                .ok_or_else(|| format!("invalid byte view buffer `{}`", buffer.0))?
                                .values;
                            let absolute = view.start + index;
                            if absolute >= values.len() {
                                return Err(format!(
                                    "view_set index `{index}` is out of bounds for length `{}`",
                                    view.len
                                ));
                            }
                            values[absolute] = byte as u8;
                        }
                        RuntimeByteViewBacking::Reference(reference) => {
                            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                                "view_set on mapped edit views requires runtime call context"
                                    .to_string()
                            })?;
                            let current_package_id = current_package_id.ok_or_else(|| {
                                "view_set is missing current package context".to_string()
                            })?;
                            let current_module_id = current_module_id.ok_or_else(|| {
                                "view_set is missing current module context".to_string()
                            })?;
                            let aliases = aliases
                                .ok_or_else(|| "view_set is missing alias context".to_string())?;
                            let type_bindings = type_bindings.ok_or_else(|| {
                                "view_set is missing type binding context".to_string()
                            })?;
                            let (carrier, mut values) = runtime_reference_byte_carrier(
                                scopes,
                                plan,
                                current_package_id,
                                current_module_id,
                                aliases,
                                type_bindings,
                                state,
                                reference,
                                host,
                                "view_set",
                            )?;
                            let absolute = view.start + index;
                            if absolute >= values.len() {
                                return Err(format!(
                                    "view_set index `{index}` is out of bounds for length `{}`",
                                    view.len
                                ));
                            }
                            values[absolute] = byte as u8;
                            runtime_write_byte_reference(
                                scopes,
                                plan,
                                current_package_id,
                                current_module_id,
                                aliases,
                                type_bindings,
                                state,
                                reference,
                                carrier,
                                values,
                                host,
                            )?;
                        }
                        RuntimeByteViewBacking::Foreign(backing) => {
                            let backing_len =
                                runtime_foreign_byte_len(plan, host, *backing, "view_set")?;
                            let _ =
                                runtime_view_range(view.start, view.len, backing_len, "view_set")?;
                            runtime_foreign_byte_set(
                                plan,
                                host,
                                *backing,
                                view.start + index,
                                byte as u8,
                                "view_set",
                            )?;
                        }
                    }
                    Ok(RuntimeValue::Unit)
                }
                other => Err(format!("view_set expected edit View, got `{other:?}`")),
            }
        }
        RuntimeIntrinsic::MemoryEditViewSubviewRead => {
            if args.len() != 3 {
                return Err("edit_view_subview_read expects three arguments".to_string());
            }
            let handle = expect_edit_view(args[0].clone(), "edit_view_subview_read")?;
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
            let start = expect_int(args[1].clone(), "edit_view_subview_read start")?;
            let end = expect_int(args[2].clone(), "edit_view_subview_read end")?;
            let (start, end) = runtime_view_bounds(start, end, view_len, "edit_view_subview_read")?;
            let next = insert_runtime_read_view_from_backing(
                state,
                &type_args,
                backing,
                view_start + start,
                end - start,
            );
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(next)))
        }
        RuntimeIntrinsic::MemoryEditViewSubviewEdit => {
            if args.len() != 3 {
                return Err("edit_view_subview_edit expects three arguments".to_string());
            }
            let handle = expect_edit_view(args[0].clone(), "edit_view_subview_edit")?;
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
            let start = expect_int(args[1].clone(), "edit_view_subview_edit start")?;
            let end = expect_int(args[2].clone(), "edit_view_subview_edit end")?;
            let (start, end) = runtime_view_bounds(start, end, view_len, "edit_view_subview_edit")?;
            let next = match backing {
                RuntimeElementViewBacking::Buffer(buffer) => {
                    runtime_reject_live_reference_or_opaque_conflict(
                        scopes.as_ref().map(|scopes| scopes.as_slice()),
                        Some(args.as_slice()),
                        state,
                        |_| false,
                        |opaque, state| runtime_opaque_matches_element_buffer(opaque, state, buffer),
                        None,
                        Some(RuntimeOpaqueValue::EditView(handle)),
                        |_| false,
                        "edit_view_subview_edit rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
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
                        scopes.as_ref().map(|scopes| scopes.as_slice()),
                        Some(args.as_slice()),
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
                        "edit_view_subview_edit rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
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
                        "edit_view_subview_edit range is out of bounds for RingBuffer-backed EditView"
                            .to_string()
                    })?
                    .to_vec();
                    let ids = runtime_ring_ids_for_slots(state, arena, &active)?;
                    runtime_reject_live_reference_or_opaque_conflict(
                        scopes.as_ref().map(|scopes| scopes.as_slice()),
                        Some(args.as_slice()),
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
                        "edit_view_subview_edit rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
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
        }
        RuntimeIntrinsic::MemoryByteViewLen => match expect_single_arg(args, "byte_view_len")? {
            RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
                let view = state
                    .byte_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                    .clone();
                if let RuntimeByteViewBacking::Reference(reference) = &view.backing {
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "byte_view_len on borrowed ByteView requires runtime call context"
                            .to_string()
                    })?;
                    let _ = runtime_reference_array_len(
                        scopes.as_slice(),
                        state,
                        reference,
                        "byte_view_len",
                    )?;
                } else if let RuntimeByteViewBacking::Foreign(backing) = &view.backing {
                    let backing_len =
                        runtime_foreign_byte_len(plan, host, *backing, "byte_view_len")?;
                    let _ = runtime_view_range(view.start, view.len, backing_len, "byte_view_len")?;
                }
                Ok(RuntimeValue::Int(view.len as i64))
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                let view = state
                    .byte_edit_views
                    .get(&handle)
                    .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                    .clone();
                match &view.backing {
                    RuntimeByteViewBacking::Foreign(backing) => {
                        let backing_len =
                            runtime_foreign_byte_len(plan, host, *backing, "byte_view_len")?;
                        let _ =
                            runtime_view_range(view.start, view.len, backing_len, "byte_view_len")?;
                    }
                    RuntimeByteViewBacking::Buffer(_) | RuntimeByteViewBacking::Reference(_) => {
                        let scopes = scopes.as_deref_mut().ok_or_else(|| {
                                "byte_view_len on borrowed ByteEditView requires runtime call context".to_string()
                            })?;
                        let _ = runtime_byte_edit_view_values(
                            scopes,
                            plan,
                            current_package_id.ok_or_else(|| {
                                "byte_view_len is missing current package context".to_string()
                            })?,
                            current_module_id.ok_or_else(|| {
                                "byte_view_len is missing current module context".to_string()
                            })?,
                            aliases.ok_or_else(|| {
                                "byte_view_len is missing alias context".to_string()
                            })?,
                            type_bindings.ok_or_else(|| {
                                "byte_view_len is missing type binding context".to_string()
                            })?,
                            state,
                            &view,
                            host,
                            "byte_view_len",
                        )?;
                    }
                }
                Ok(RuntimeValue::Int(view.len as i64))
            }
            other => Err(format!(
                "byte_view_len expected mapped view, got `{other:?}`"
            )),
        },
        RuntimeIntrinsic::MemoryByteViewAt => {
            if args.len() != 2 {
                return Err("byte_view_at expects two arguments".to_string());
            }
            match args[0].clone() {
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
                    let view = state
                        .byte_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "byte_view_at index")?,
                        view.len,
                        "byte_view_at",
                    )?;
                    let value = match &view.backing {
                        RuntimeByteViewBacking::Buffer(buffer) => {
                            let values = &state
                                .byte_view_buffers
                                .get(buffer)
                                .ok_or_else(|| format!("invalid byte view buffer `{}`", buffer.0))?
                                .values;
                            values.get(view.start + index).copied().ok_or_else(|| {
                                format!("byte_view_at index `{index}` is out of bounds")
                            })?
                        }
                        RuntimeByteViewBacking::Reference(reference) => {
                            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                                "byte_view_at on borrowed ByteView requires runtime call context"
                                    .to_string()
                            })?;
                            runtime_reference_array_byte_at(
                                scopes.as_slice(),
                                state,
                                reference,
                                view.start + index,
                                "byte_view_at",
                            )?
                            .ok_or_else(|| {
                                "byte_view_at could not resolve borrowed ByteView backing"
                                    .to_string()
                            })?
                        }
                        RuntimeByteViewBacking::Foreign(backing) => {
                            let backing_len =
                                runtime_foreign_byte_len(plan, host, *backing, "byte_view_at")?;
                            let _ = runtime_view_range(
                                view.start,
                                view.len,
                                backing_len,
                                "byte_view_at",
                            )?;
                            runtime_foreign_byte_at(
                                plan,
                                host,
                                *backing,
                                view.start + index,
                                "byte_view_at",
                            )?
                        }
                    };
                    Ok(RuntimeValue::Int(i64::from(value)))
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                    let view = state
                        .byte_edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                        .clone();
                    let index = runtime_index_to_usize(
                        expect_int(args[1].clone(), "byte_view_at index")?,
                        view.len,
                        "byte_view_at",
                    )?;
                    let value = match &view.backing {
                        RuntimeByteViewBacking::Buffer(buffer) => {
                            let values = &state
                                .byte_view_buffers
                                .get(buffer)
                                .ok_or_else(|| format!("invalid byte view buffer `{}`", buffer.0))?
                                .values;
                            values.get(view.start + index).copied().ok_or_else(|| {
                                format!("byte_view_at index `{index}` is out of bounds")
                            })?
                        }
                        RuntimeByteViewBacking::Reference(reference) => {
                            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                                "byte_view_at on borrowed ByteEditView requires runtime call context"
                                    .to_string()
                            })?;
                            runtime_reference_array_byte_at(
                                scopes.as_slice(),
                                state,
                                reference,
                                view.start + index,
                                "byte_view_at",
                            )?
                            .ok_or_else(|| {
                                "byte_view_at could not resolve borrowed ByteEditView backing"
                                    .to_string()
                            })?
                        }
                        RuntimeByteViewBacking::Foreign(backing) => {
                            let backing_len =
                                runtime_foreign_byte_len(plan, host, *backing, "byte_view_at")?;
                            let _ = runtime_view_range(
                                view.start,
                                view.len,
                                backing_len,
                                "byte_view_at",
                            )?;
                            runtime_foreign_byte_at(
                                plan,
                                host,
                                *backing,
                                view.start + index,
                                "byte_view_at",
                            )?
                        }
                    };
                    Ok(RuntimeValue::Int(i64::from(value)))
                }
                other => Err(format!(
                    "byte_view_at expected mapped view, got `{other:?}`"
                )),
            }
        }
        RuntimeIntrinsic::MemoryByteViewSubview => {
            if args.len() != 3 {
                return Err("byte_view_subview expects three arguments".to_string());
            }
            let start = expect_int(args[1].clone(), "byte_view_subview start")?;
            let end = expect_int(args[2].clone(), "byte_view_subview end")?;
            match args[0].clone() {
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
                    let view = state
                        .byte_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                        .clone();
                    let (start, end) =
                        runtime_view_bounds(start, end, view.len, "byte_view_subview")?;
                    let next = match view.backing {
                        RuntimeByteViewBacking::Buffer(buffer) => {
                            insert_runtime_byte_view_from_buffer(
                                state,
                                buffer,
                                view.start + start,
                                end - start,
                            )
                        }
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
                RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
                    let view = state
                        .byte_edit_views
                        .get(&handle)
                        .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                        .clone();
                    let (start, end) =
                        runtime_view_bounds(start, end, view.len, "byte_view_subview")?;
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
                }
                other => Err(format!(
                    "byte_view_subview expected mapped view, got `{other:?}`"
                )),
            }
        }
        RuntimeIntrinsic::MemoryByteViewToArray => {
            let handle = expect_byte_view(
                expect_single_arg(args, "byte_view_to_array")?,
                "byte_view_to_array",
            )?;
            let view = state
                .byte_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?
                .clone();
            match &view.backing {
                RuntimeByteViewBacking::Foreign(_) => {
                    Ok(bytes_to_runtime_array(runtime_byte_view_values(
                        &mut Vec::new(),
                        plan,
                        "",
                        "",
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        state,
                        &view,
                        host,
                        "byte_view_to_array",
                    )?))
                }
                RuntimeByteViewBacking::Buffer(_) | RuntimeByteViewBacking::Reference(_) => {
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "byte_view_to_array on borrowed ByteView requires runtime call context"
                            .to_string()
                    })?;
                    Ok(bytes_to_runtime_array(runtime_byte_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "byte_view_to_array is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "byte_view_to_array is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| {
                            "byte_view_to_array is missing alias context".to_string()
                        })?,
                        type_bindings.ok_or_else(|| {
                            "byte_view_to_array is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "byte_view_to_array",
                    )?))
                }
            }
        }
        RuntimeIntrinsic::MemoryByteEditViewLen => {
            let handle = expect_byte_edit_view(
                expect_single_arg(args, "byte_edit_view_len")?,
                "byte_edit_view_len",
            )?;
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            match &view.backing {
                RuntimeByteViewBacking::Foreign(backing) => {
                    let backing_len =
                        runtime_foreign_byte_len(plan, host, *backing, "byte_edit_view_len")?;
                    let _ = runtime_view_range(
                        view.start,
                        view.len,
                        backing_len,
                        "byte_edit_view_len",
                    )?;
                }
                RuntimeByteViewBacking::Buffer(_) | RuntimeByteViewBacking::Reference(_) => {
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "byte_edit_view_len on borrowed ByteEditView requires runtime call context"
                            .to_string()
                    })?;
                    let _ = runtime_byte_edit_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "byte_edit_view_len is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "byte_edit_view_len is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| {
                            "byte_edit_view_len is missing alias context".to_string()
                        })?,
                        type_bindings.ok_or_else(|| {
                            "byte_edit_view_len is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "byte_edit_view_len",
                    )?;
                }
            }
            Ok(RuntimeValue::Int(view.len as i64))
        }
        RuntimeIntrinsic::MemoryByteEditViewAt => {
            if args.len() != 2 {
                return Err("byte_edit_view_at expects two arguments".to_string());
            }
            let handle = expect_byte_edit_view(args[0].clone(), "byte_edit_view_at")?;
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            let index = runtime_index_to_usize(
                expect_int(args[1].clone(), "byte_edit_view_at index")?,
                view.len,
                "byte_edit_view_at",
            )?;
            let value = match &view.backing {
                RuntimeByteViewBacking::Foreign(backing) => {
                    let backing_len =
                        runtime_foreign_byte_len(plan, host, *backing, "byte_edit_view_at")?;
                    let _ =
                        runtime_view_range(view.start, view.len, backing_len, "byte_edit_view_at")?;
                    runtime_foreign_byte_at(
                        plan,
                        host,
                        *backing,
                        view.start + index,
                        "byte_edit_view_at",
                    )?
                }
                RuntimeByteViewBacking::Buffer(_) | RuntimeByteViewBacking::Reference(_) => {
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "byte_edit_view_at on borrowed ByteEditView requires runtime call context"
                            .to_string()
                    })?;
                    runtime_byte_edit_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "byte_edit_view_at is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "byte_edit_view_at is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| {
                            "byte_edit_view_at is missing alias context".to_string()
                        })?,
                        type_bindings.ok_or_else(|| {
                            "byte_edit_view_at is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "byte_edit_view_at",
                    )?[index]
                }
            };
            Ok(RuntimeValue::Int(i64::from(value)))
        }
        RuntimeIntrinsic::MemoryByteEditViewSet => {
            if args.len() != 3 {
                return Err("byte_edit_view_set expects three arguments".to_string());
            }
            let handle = expect_byte_edit_view(args[0].clone(), "byte_edit_view_set")?;
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            let index = runtime_index_to_usize(
                expect_int(args[1].clone(), "byte_edit_view_set index")?,
                view.len,
                "byte_edit_view_set",
            )?;
            let byte = expect_int(args[2].clone(), "byte_edit_view_set value")?;
            if !(0..=255).contains(&byte) {
                return Err(format!(
                    "byte_edit_view_set value `{byte}` is out of range `0..=255`"
                ));
            }
            match &view.backing {
                RuntimeByteViewBacking::Buffer(buffer) => {
                    let values = &mut state
                        .byte_view_buffers
                        .get_mut(buffer)
                        .ok_or_else(|| format!("invalid byte view buffer `{}`", buffer.0))?
                        .values;
                    let absolute = view.start + index;
                    if absolute >= values.len() {
                        return Err(format!(
                            "byte_edit_view_set index `{index}` is out of bounds for length `{}`",
                            view.len
                        ));
                    }
                    values[absolute] = byte as u8;
                }
                RuntimeByteViewBacking::Reference(reference) => {
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "byte_edit_view_set on borrowed ByteEditView requires runtime call context"
                            .to_string()
                    })?;
                    let current_package_id = current_package_id.ok_or_else(|| {
                        "byte_edit_view_set is missing current package context".to_string()
                    })?;
                    let current_module_id = current_module_id.ok_or_else(|| {
                        "byte_edit_view_set is missing current module context".to_string()
                    })?;
                    let aliases = aliases
                        .ok_or_else(|| "byte_edit_view_set is missing alias context".to_string())?;
                    let type_bindings = type_bindings.ok_or_else(|| {
                        "byte_edit_view_set is missing type binding context".to_string()
                    })?;
                    let mut values = runtime_reference_array_values(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        reference,
                        host,
                        "byte_edit_view_set",
                    )?;
                    let absolute = view.start + index;
                    if absolute >= values.len() {
                        return Err(format!(
                            "byte_edit_view_set index `{index}` is out of bounds for length `{}`",
                            view.len
                        ));
                    }
                    values[absolute] = RuntimeValue::Int(byte);
                    write_runtime_reference(
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        reference,
                        RuntimeValue::Array(values),
                        host,
                    )?;
                }
                RuntimeByteViewBacking::Foreign(backing) => {
                    let backing_len =
                        runtime_foreign_byte_len(plan, host, *backing, "byte_edit_view_set")?;
                    let _ = runtime_view_range(
                        view.start,
                        view.len,
                        backing_len,
                        "byte_edit_view_set",
                    )?;
                    runtime_foreign_byte_set(
                        plan,
                        host,
                        *backing,
                        view.start + index,
                        byte as u8,
                        "byte_edit_view_set",
                    )?;
                }
            }
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryByteEditViewSubviewRead => {
            if args.len() != 3 {
                return Err("byte_edit_view_subview_read expects three arguments".to_string());
            }
            let handle = expect_byte_edit_view(args[0].clone(), "byte_edit_view_subview_read")?;
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            let start = expect_int(args[1].clone(), "byte_edit_view_subview_read start")?;
            let end = expect_int(args[2].clone(), "byte_edit_view_subview_read end")?;
            let (start, end) =
                runtime_view_bounds(start, end, view.len, "byte_edit_view_subview_read")?;
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
        RuntimeIntrinsic::MemoryByteEditViewSubviewEdit => {
            if args.len() != 3 {
                return Err("byte_edit_view_subview_edit expects three arguments".to_string());
            }
            let handle = expect_byte_edit_view(args[0].clone(), "byte_edit_view_subview_edit")?;
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            let start = expect_int(args[1].clone(), "byte_edit_view_subview_edit start")?;
            let end = expect_int(args[2].clone(), "byte_edit_view_subview_edit end")?;
            let (start, end) =
                runtime_view_bounds(start, end, view.len, "byte_edit_view_subview_edit")?;
            runtime_reject_live_reference_or_opaque_conflict(
                scopes.as_ref().map(|scopes| scopes.as_slice()),
                Some(args.as_slice()),
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
                "byte_edit_view_subview_edit rejects exclusive view acquisition while conflicting borrows or views are live".to_string(),
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
        }
        RuntimeIntrinsic::MemoryByteEditViewToArray => {
            let handle = expect_byte_edit_view(
                expect_single_arg(args, "byte_edit_view_to_array")?,
                "byte_edit_view_to_array",
            )?;
            let view = state
                .byte_edit_views
                .get(&handle)
                .ok_or_else(|| format!("invalid ByteEditView handle `{}`", handle.0))?
                .clone();
            match &view.backing {
                RuntimeByteViewBacking::Foreign(_) => {
                    Ok(bytes_to_runtime_array(runtime_byte_edit_view_values(
                        &mut Vec::new(),
                        plan,
                        "",
                        "",
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        state,
                        &view,
                        host,
                        "byte_edit_view_to_array",
                    )?))
                }
                RuntimeByteViewBacking::Buffer(_) | RuntimeByteViewBacking::Reference(_) => {
                    let scopes = scopes.as_deref_mut().ok_or_else(|| {
                        "byte_edit_view_to_array on borrowed ByteEditView requires runtime call context"
                            .to_string()
                    })?;
                    Ok(bytes_to_runtime_array(runtime_byte_edit_view_values(
                        scopes,
                        plan,
                        current_package_id.ok_or_else(|| {
                            "byte_edit_view_to_array is missing current package context".to_string()
                        })?,
                        current_module_id.ok_or_else(|| {
                            "byte_edit_view_to_array is missing current module context".to_string()
                        })?,
                        aliases.ok_or_else(|| {
                            "byte_edit_view_to_array is missing alias context".to_string()
                        })?,
                        type_bindings.ok_or_else(|| {
                            "byte_edit_view_to_array is missing type binding context".to_string()
                        })?,
                        state,
                        &view,
                        host,
                        "byte_edit_view_to_array",
                    )?))
                }
            }
        }
        RuntimeIntrinsic::MemoryStrViewLenBytes => {
            let handle = expect_str_view(
                expect_single_arg(args, "str_view_len_bytes")?,
                "str_view_len_bytes",
            )?;
            let view = state
                .str_views
                .get(&handle)
                .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                .clone();
            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                "str_view_len_bytes on borrowed StrView requires runtime call context".to_string()
            })?;
            let _ = runtime_str_view_text(
                scopes,
                plan,
                current_package_id.ok_or_else(|| {
                    "str_view_len_bytes is missing current package context".to_string()
                })?,
                current_module_id.ok_or_else(|| {
                    "str_view_len_bytes is missing current module context".to_string()
                })?,
                aliases.ok_or_else(|| "str_view_len_bytes is missing alias context".to_string())?,
                type_bindings.ok_or_else(|| {
                    "str_view_len_bytes is missing type binding context".to_string()
                })?,
                state,
                &view,
                host,
                "str_view_len_bytes",
            )?;
            Ok(RuntimeValue::Int(view.len as i64))
        }
        RuntimeIntrinsic::MemoryStrViewByteAt => {
            if args.len() != 2 {
                return Err("str_view_byte_at expects two arguments".to_string());
            }
            let handle = expect_str_view(args[0].clone(), "str_view_byte_at")?;
            let view = state
                .str_views
                .get(&handle)
                .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                .clone();
            let index = runtime_index_to_usize(
                expect_int(args[1].clone(), "str_view_byte_at index")?,
                view.len,
                "str_view_byte_at",
            )?;
            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                "str_view_byte_at on borrowed StrView requires runtime call context".to_string()
            })?;
            Ok(RuntimeValue::Int(i64::from(
                runtime_str_view_text(
                    scopes,
                    plan,
                    current_package_id.ok_or_else(|| {
                        "str_view_byte_at is missing current package context".to_string()
                    })?,
                    current_module_id.ok_or_else(|| {
                        "str_view_byte_at is missing current module context".to_string()
                    })?,
                    aliases
                        .ok_or_else(|| "str_view_byte_at is missing alias context".to_string())?,
                    type_bindings.ok_or_else(|| {
                        "str_view_byte_at is missing type binding context".to_string()
                    })?,
                    state,
                    &view,
                    host,
                    "str_view_byte_at",
                )?
                .as_bytes()[index],
            )))
        }
        RuntimeIntrinsic::MemoryStrViewSubview => {
            if args.len() != 3 {
                return Err("str_view_subview expects three arguments".to_string());
            }
            let handle = expect_str_view(args[0].clone(), "str_view_subview")?;
            let view = state
                .str_views
                .get(&handle)
                .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                .clone();
            let start = expect_int(args[1].clone(), "str_view_subview start")?;
            let end = expect_int(args[2].clone(), "str_view_subview end")?;
            let (start, end) = runtime_view_bounds(start, end, view.len, "str_view_subview")?;
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
        RuntimeIntrinsic::MemoryStrViewToStr => {
            let handle = expect_str_view(
                expect_single_arg(args, "str_view_to_str")?,
                "str_view_to_str",
            )?;
            let view = state
                .str_views
                .get(&handle)
                .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?
                .clone();
            let scopes = scopes.as_deref_mut().ok_or_else(|| {
                "str_view_to_str on borrowed StrView requires runtime call context".to_string()
            })?;
            Ok(RuntimeValue::Str(runtime_str_view_text(
                scopes,
                plan,
                current_package_id.ok_or_else(|| {
                    "str_view_to_str is missing current package context".to_string()
                })?,
                current_module_id.ok_or_else(|| {
                    "str_view_to_str is missing current module context".to_string()
                })?,
                aliases.ok_or_else(|| "str_view_to_str is missing alias context".to_string())?,
                type_bindings
                    .ok_or_else(|| "str_view_to_str is missing type binding context".to_string())?,
                state,
                &view,
                host,
                "str_view_to_str",
            )?))
        }
    }
}
