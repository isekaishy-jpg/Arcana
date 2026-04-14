use crate::runtime_intrinsics::RuntimeIntrinsic;

pub(super) fn resolve_path(parts: &[&str]) -> Option<RuntimeIntrinsic> {
    match parts {
        ["std", "concurrent", "channel"] | ["std", "kernel", "concurrency", "channel_new"] => {
            Some(RuntimeIntrinsic::ConcurrentChannelNew)
        }
        ["std", "concurrent", "mutex"] | ["std", "kernel", "concurrency", "mutex_new"] => {
            Some(RuntimeIntrinsic::ConcurrentMutexNew)
        }
        ["std", "concurrent", "atomic_int"]
        | ["std", "kernel", "concurrency", "atomic_int_new"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntNew)
        }
        ["std", "concurrent", "atomic_bool"]
        | ["std", "kernel", "concurrency", "atomic_bool_new"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolNew)
        }
        ["std", "concurrent", "sleep"] | ["std", "kernel", "concurrency", "sleep"] => {
            Some(RuntimeIntrinsic::ConcurrentSleep)
        }
        ["std", "concurrent", "thread_id"] | ["std", "kernel", "concurrency", "thread_id"] => {
            Some(RuntimeIntrinsic::ConcurrentThreadId)
        }
        ["std", "kernel", "concurrency", "task_done"] => Some(RuntimeIntrinsic::ConcurrentTaskDone),
        ["std", "kernel", "concurrency", "task_join"] => Some(RuntimeIntrinsic::ConcurrentTaskJoin),
        ["std", "kernel", "concurrency", "thread_done"] => {
            Some(RuntimeIntrinsic::ConcurrentThreadDone)
        }
        ["std", "kernel", "concurrency", "thread_join"] => {
            Some(RuntimeIntrinsic::ConcurrentThreadJoin)
        }
        ["std", "kernel", "concurrency", "channel_send"] => {
            Some(RuntimeIntrinsic::ConcurrentChannelSend)
        }
        ["std", "kernel", "concurrency", "channel_recv"] => {
            Some(RuntimeIntrinsic::ConcurrentChannelRecv)
        }
        ["std", "kernel", "concurrency", "mutex_take"] => {
            Some(RuntimeIntrinsic::ConcurrentMutexTake)
        }
        ["std", "kernel", "concurrency", "mutex_put"] => Some(RuntimeIntrinsic::ConcurrentMutexPut),
        ["std", "kernel", "concurrency", "atomic_int_load"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntLoad)
        }
        ["std", "kernel", "concurrency", "atomic_int_store"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntStore)
        }
        ["std", "kernel", "concurrency", "atomic_int_add"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntAdd)
        }
        ["std", "kernel", "concurrency", "atomic_int_sub"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntSub)
        }
        ["std", "kernel", "concurrency", "atomic_int_swap"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntSwap)
        }
        ["std", "kernel", "concurrency", "atomic_bool_load"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolLoad)
        }
        ["std", "kernel", "concurrency", "atomic_bool_store"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolStore)
        }
        ["std", "kernel", "concurrency", "atomic_bool_swap"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolSwap)
        }
        _ => None,
    }
}

pub(super) fn resolve_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    match intrinsic_impl {
        "ConcurrentSleep" => Some(RuntimeIntrinsic::ConcurrentSleep),
        "ConcurrentBehaviorStep" => Some(RuntimeIntrinsic::ConcurrentBehaviorStep),
        "ConcurrentThreadId" => Some(RuntimeIntrinsic::ConcurrentThreadId),
        "ConcurrentTaskDone" => Some(RuntimeIntrinsic::ConcurrentTaskDone),
        "ConcurrentTaskJoin" => Some(RuntimeIntrinsic::ConcurrentTaskJoin),
        "ConcurrentThreadDone" => Some(RuntimeIntrinsic::ConcurrentThreadDone),
        "ConcurrentThreadJoin" => Some(RuntimeIntrinsic::ConcurrentThreadJoin),
        "ConcurrentChannelNew" => Some(RuntimeIntrinsic::ConcurrentChannelNew),
        "ConcurrentChannelSend" => Some(RuntimeIntrinsic::ConcurrentChannelSend),
        "ConcurrentChannelRecv" => Some(RuntimeIntrinsic::ConcurrentChannelRecv),
        "ConcurrentMutexNew" => Some(RuntimeIntrinsic::ConcurrentMutexNew),
        "ConcurrentMutexTake" => Some(RuntimeIntrinsic::ConcurrentMutexTake),
        "ConcurrentMutexPut" => Some(RuntimeIntrinsic::ConcurrentMutexPut),
        "ConcurrentAtomicIntNew" => Some(RuntimeIntrinsic::ConcurrentAtomicIntNew),
        "ConcurrentAtomicIntLoad" => Some(RuntimeIntrinsic::ConcurrentAtomicIntLoad),
        "ConcurrentAtomicIntStore" => Some(RuntimeIntrinsic::ConcurrentAtomicIntStore),
        "ConcurrentAtomicIntAdd" => Some(RuntimeIntrinsic::ConcurrentAtomicIntAdd),
        "ConcurrentAtomicIntSub" => Some(RuntimeIntrinsic::ConcurrentAtomicIntSub),
        "ConcurrentAtomicIntSwap" => Some(RuntimeIntrinsic::ConcurrentAtomicIntSwap),
        "ConcurrentAtomicBoolNew" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolNew),
        "ConcurrentAtomicBoolLoad" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolLoad),
        "ConcurrentAtomicBoolStore" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolStore),
        "ConcurrentAtomicBoolSwap" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolSwap),
        _ => None,
    }
}
