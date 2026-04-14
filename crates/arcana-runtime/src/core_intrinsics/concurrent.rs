use super::*;
use crate::runtime_intrinsics::ConcurrentIntrinsic as RuntimeIntrinsic;

#[allow(unused_variables)]
pub(super) fn execute(
    intrinsic: RuntimeIntrinsic,
    type_args: &[String],
    final_args: &mut Vec<RuntimeValue>,
    plan: &RuntimePackagePlan,
    scopes: Option<&mut Vec<RuntimeScope>>,
    current_package_id: Option<&str>,
    current_module_id: Option<&str>,
    aliases: Option<&BTreeMap<String, Vec<String>>>,
    type_bindings: Option<&RuntimeTypeBindings>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let args = final_args.clone();
    match intrinsic {
        RuntimeIntrinsic::ConcurrentSleep => {
            let ms = expect_int(expect_single_arg(args, "sleep")?, "sleep")?;
            host.sleep_ms(ms)?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentBehaviorStep => {
            let phase = expect_str(expect_single_arg(args, "behavior_step")?, "behavior_step")?;
            Ok(RuntimeValue::Int(runtime_behavior_step(
                plan, &phase, state, host,
            )?))
        }
        RuntimeIntrinsic::ConcurrentThreadId => {
            if !args.is_empty() {
                return Err("thread_id expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Int(state.current_thread_id))
        }
        RuntimeIntrinsic::ConcurrentTaskDone => {
            let handle = expect_task(expect_single_arg(args, "task_done")?, "task_done")?;
            let task = state
                .tasks
                .get(&handle)
                .ok_or_else(|| format!("invalid Task handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(pending_state_is_done(&task.state)))
        }
        RuntimeIntrinsic::ConcurrentTaskJoin => {
            let handle = expect_task(expect_single_arg(args, "task_join")?, "task_join")?;
            drive_runtime_task(handle, plan, state, host)?;
            let task = state
                .tasks
                .get(&handle)
                .ok_or_else(|| format!("invalid Task handle `{}`", handle.0))?;
            pending_state_value(&task.state, &format!("Task `{}`", handle.0))
        }
        RuntimeIntrinsic::ConcurrentThreadDone => {
            let handle = expect_thread(expect_single_arg(args, "thread_done")?, "thread_done")?;
            let thread = state
                .threads
                .get(&handle)
                .ok_or_else(|| format!("invalid Thread handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(pending_state_is_done(&thread.state)))
        }
        RuntimeIntrinsic::ConcurrentThreadJoin => {
            let handle = expect_thread(expect_single_arg(args, "thread_join")?, "thread_join")?;
            drive_runtime_thread(handle, plan, state, host)?;
            let thread = state
                .threads
                .get(&handle)
                .ok_or_else(|| format!("invalid Thread handle `{}`", handle.0))?;
            pending_state_value(&thread.state, &format!("Thread `{}`", handle.0))
        }
        RuntimeIntrinsic::ConcurrentChannelNew => {
            let capacity = expect_int(expect_single_arg(args, "channel_new")?, "channel_new")?;
            if capacity < 0 {
                return Err("channel_new capacity must be non-negative".to_string());
            }
            let handle = insert_runtime_channel(state, type_args, capacity as usize);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(handle)))
        }
        RuntimeIntrinsic::ConcurrentChannelSend => {
            if args.len() != 2 {
                return Err("channel_send expects two arguments".to_string());
            }
            let handle = expect_channel(args[0].clone(), "channel_send")?;
            runtime_validate_split_value(&args[1], state, "channel_send payload")?;
            let channel = state
                .channels
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Channel handle `{}`", handle.0))?;
            if channel.queue.len() >= channel.capacity {
                return Err("channel_send would exceed channel capacity".to_string());
            }
            channel.queue.push_back(args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentChannelRecv => {
            let handle = expect_channel(expect_single_arg(args, "channel_recv")?, "channel_recv")?;
            let channel = state
                .channels
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Channel handle `{}`", handle.0))?;
            channel
                .queue
                .pop_front()
                .ok_or_else(|| "channel_recv called on empty channel".to_string())
        }
        RuntimeIntrinsic::ConcurrentMutexNew => {
            let value = expect_single_arg(args, "mutex_new")?;
            runtime_validate_split_value(&value, state, "mutex_new payload")?;
            let handle = insert_runtime_mutex(state, type_args, value);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(handle)))
        }
        RuntimeIntrinsic::ConcurrentMutexTake => {
            let handle = expect_mutex(expect_single_arg(args, "mutex_take")?, "mutex_take")?;
            let mutex = state
                .mutexes
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Mutex handle `{}`", handle.0))?;
            mutex
                .value
                .take()
                .ok_or_else(|| "mutex_take called on empty mutex".to_string())
        }
        RuntimeIntrinsic::ConcurrentMutexPut => {
            if args.len() != 2 {
                return Err("mutex_put expects two arguments".to_string());
            }
            let handle = expect_mutex(args[0].clone(), "mutex_put")?;
            runtime_validate_split_value(&args[1], state, "mutex_put payload")?;
            let mutex = state
                .mutexes
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Mutex handle `{}`", handle.0))?;
            if mutex.value.is_some() {
                return Err("mutex_put called while mutex already holds a value".to_string());
            }
            mutex.value = Some(args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentAtomicIntNew => {
            let value = expect_int(expect_single_arg(args, "atomic_int_new")?, "atomic_int_new")?;
            let handle = insert_runtime_atomic_int(state, value);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicInt(handle)))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntLoad => {
            let handle = expect_atomic_int(
                expect_single_arg(args, "atomic_int_load")?,
                "atomic_int_load",
            )?;
            let value = state
                .atomic_ints
                .get(&handle)
                .copied()
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(value))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntStore => {
            if args.len() != 2 {
                return Err("atomic_int_store expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_store")?;
            let value = expect_int(args[1].clone(), "atomic_int_store")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            *slot = value;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentAtomicIntAdd => {
            if args.len() != 2 {
                return Err("atomic_int_add expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_add")?;
            let delta = expect_int(args[1].clone(), "atomic_int_add")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            *slot += delta;
            Ok(RuntimeValue::Int(*slot))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntSub => {
            if args.len() != 2 {
                return Err("atomic_int_sub expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_sub")?;
            let delta = expect_int(args[1].clone(), "atomic_int_sub")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            *slot -= delta;
            Ok(RuntimeValue::Int(*slot))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntSwap => {
            if args.len() != 2 {
                return Err("atomic_int_swap expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_swap")?;
            let value = expect_int(args[1].clone(), "atomic_int_swap")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            let old = *slot;
            *slot = value;
            Ok(RuntimeValue::Int(old))
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolNew => {
            let value = expect_bool(
                expect_single_arg(args, "atomic_bool_new")?,
                "atomic_bool_new",
            )?;
            let handle = insert_runtime_atomic_bool(state, value);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicBool(handle)))
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolLoad => {
            let handle = expect_atomic_bool(
                expect_single_arg(args, "atomic_bool_load")?,
                "atomic_bool_load",
            )?;
            let value = state
                .atomic_bools
                .get(&handle)
                .copied()
                .ok_or_else(|| format!("invalid AtomicBool handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(value))
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolStore => {
            if args.len() != 2 {
                return Err("atomic_bool_store expects two arguments".to_string());
            }
            let handle = expect_atomic_bool(args[0].clone(), "atomic_bool_store")?;
            let value = expect_bool(args[1].clone(), "atomic_bool_store")?;
            let slot = state
                .atomic_bools
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicBool handle `{}`", handle.0))?;
            *slot = value;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolSwap => {
            if args.len() != 2 {
                return Err("atomic_bool_swap expects two arguments".to_string());
            }
            let handle = expect_atomic_bool(args[0].clone(), "atomic_bool_swap")?;
            let value = expect_bool(args[1].clone(), "atomic_bool_swap")?;
            let slot = state
                .atomic_bools
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicBool handle `{}`", handle.0))?;
            let old = *slot;
            *slot = value;
            Ok(RuntimeValue::Bool(old))
        }
    }
}
