use crate::runtime_intrinsics::RuntimeIntrinsic;

pub(super) fn resolve_path(parts: &[&str]) -> Option<RuntimeIntrinsic> {
    match parts {
        ["std", "memory", "new"] | ["std", "kernel", "memory", "arena_new"] => {
            Some(RuntimeIntrinsic::MemoryArenaNew)
        }
        ["std", "memory", "frame_new"] | ["std", "kernel", "memory", "frame_new"] => {
            Some(RuntimeIntrinsic::MemoryFrameNew)
        }
        ["std", "memory", "pool_new"] | ["std", "kernel", "memory", "pool_new"] => {
            Some(RuntimeIntrinsic::MemoryPoolNew)
        }
        ["std", "kernel", "memory", "arena_alloc"] => Some(RuntimeIntrinsic::MemoryArenaAlloc),
        ["std", "kernel", "memory", "arena_len"] => Some(RuntimeIntrinsic::MemoryArenaLen),
        ["std", "kernel", "memory", "arena_has"] => Some(RuntimeIntrinsic::MemoryArenaHas),
        ["std", "kernel", "memory", "arena_get"] => Some(RuntimeIntrinsic::MemoryArenaGet),
        ["std", "kernel", "memory", "arena_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryArenaBorrowRead)
        }
        ["std", "kernel", "memory", "arena_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryArenaBorrowEdit)
        }
        ["std", "kernel", "memory", "arena_set"] => Some(RuntimeIntrinsic::MemoryArenaSet),
        ["std", "kernel", "memory", "arena_remove"] => Some(RuntimeIntrinsic::MemoryArenaRemove),
        ["std", "kernel", "memory", "arena_reset"] => Some(RuntimeIntrinsic::MemoryArenaReset),
        ["std", "kernel", "memory", "frame_alloc"] => Some(RuntimeIntrinsic::MemoryFrameAlloc),
        ["std", "kernel", "memory", "frame_len"] => Some(RuntimeIntrinsic::MemoryFrameLen),
        ["std", "kernel", "memory", "frame_has"] => Some(RuntimeIntrinsic::MemoryFrameHas),
        ["std", "kernel", "memory", "frame_get"] => Some(RuntimeIntrinsic::MemoryFrameGet),
        ["std", "kernel", "memory", "frame_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryFrameBorrowRead)
        }
        ["std", "kernel", "memory", "frame_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryFrameBorrowEdit)
        }
        ["std", "kernel", "memory", "frame_set"] => Some(RuntimeIntrinsic::MemoryFrameSet),
        ["std", "kernel", "memory", "frame_reset"] => Some(RuntimeIntrinsic::MemoryFrameReset),
        ["std", "kernel", "memory", "pool_alloc"] => Some(RuntimeIntrinsic::MemoryPoolAlloc),
        ["std", "kernel", "memory", "pool_len"] => Some(RuntimeIntrinsic::MemoryPoolLen),
        ["std", "kernel", "memory", "pool_has"] => Some(RuntimeIntrinsic::MemoryPoolHas),
        ["std", "kernel", "memory", "pool_get"] => Some(RuntimeIntrinsic::MemoryPoolGet),
        ["std", "kernel", "memory", "pool_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryPoolBorrowRead)
        }
        ["std", "kernel", "memory", "pool_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryPoolBorrowEdit)
        }
        ["std", "kernel", "memory", "pool_set"] => Some(RuntimeIntrinsic::MemoryPoolSet),
        ["std", "kernel", "memory", "pool_remove"] => Some(RuntimeIntrinsic::MemoryPoolRemove),
        ["std", "kernel", "memory", "pool_reset"] => Some(RuntimeIntrinsic::MemoryPoolReset),
        ["std", "kernel", "memory", "pool_live_ids"] => Some(RuntimeIntrinsic::MemoryPoolLiveIds),
        ["std", "kernel", "memory", "pool_compact"] => Some(RuntimeIntrinsic::MemoryPoolCompact),
        ["std", "memory", "temp_new"] | ["std", "kernel", "memory", "temp_new"] => {
            Some(RuntimeIntrinsic::MemoryTempNew)
        }
        ["std", "kernel", "memory", "temp_alloc"] => Some(RuntimeIntrinsic::MemoryTempAlloc),
        ["std", "kernel", "memory", "temp_len"] => Some(RuntimeIntrinsic::MemoryTempLen),
        ["std", "kernel", "memory", "temp_has"] => Some(RuntimeIntrinsic::MemoryTempHas),
        ["std", "kernel", "memory", "temp_get"] => Some(RuntimeIntrinsic::MemoryTempGet),
        ["std", "kernel", "memory", "temp_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryTempBorrowRead)
        }
        ["std", "kernel", "memory", "temp_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryTempBorrowEdit)
        }
        ["std", "kernel", "memory", "temp_set"] => Some(RuntimeIntrinsic::MemoryTempSet),
        ["std", "kernel", "memory", "temp_reset"] => Some(RuntimeIntrinsic::MemoryTempReset),
        ["std", "memory", "session_new"] | ["std", "kernel", "memory", "session_new"] => {
            Some(RuntimeIntrinsic::MemorySessionNew)
        }
        ["std", "kernel", "memory", "session_alloc"] => Some(RuntimeIntrinsic::MemorySessionAlloc),
        ["std", "kernel", "memory", "session_len"] => Some(RuntimeIntrinsic::MemorySessionLen),
        ["std", "kernel", "memory", "session_has"] => Some(RuntimeIntrinsic::MemorySessionHas),
        ["std", "kernel", "memory", "session_get"] => Some(RuntimeIntrinsic::MemorySessionGet),
        ["std", "kernel", "memory", "session_borrow_read"] => {
            Some(RuntimeIntrinsic::MemorySessionBorrowRead)
        }
        ["std", "kernel", "memory", "session_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemorySessionBorrowEdit)
        }
        ["std", "kernel", "memory", "session_set"] => Some(RuntimeIntrinsic::MemorySessionSet),
        ["std", "kernel", "memory", "session_reset"] => Some(RuntimeIntrinsic::MemorySessionReset),
        ["std", "kernel", "memory", "session_seal"] => Some(RuntimeIntrinsic::MemorySessionSeal),
        ["std", "kernel", "memory", "session_unseal"] => {
            Some(RuntimeIntrinsic::MemorySessionUnseal)
        }
        ["std", "kernel", "memory", "session_is_sealed"] => {
            Some(RuntimeIntrinsic::MemorySessionIsSealed)
        }
        ["std", "kernel", "memory", "session_live_ids"] => {
            Some(RuntimeIntrinsic::MemorySessionLiveIds)
        }
        ["std", "memory", "ring_new"] | ["std", "kernel", "memory", "ring_new"] => {
            Some(RuntimeIntrinsic::MemoryRingNew)
        }
        ["std", "kernel", "memory", "ring_push"] => Some(RuntimeIntrinsic::MemoryRingPush),
        ["std", "kernel", "memory", "ring_try_pop"] => Some(RuntimeIntrinsic::MemoryRingTryPop),
        ["std", "kernel", "memory", "ring_len"] => Some(RuntimeIntrinsic::MemoryRingLen),
        ["std", "kernel", "memory", "ring_has"] => Some(RuntimeIntrinsic::MemoryRingHas),
        ["std", "kernel", "memory", "ring_get"] => Some(RuntimeIntrinsic::MemoryRingGet),
        ["std", "kernel", "memory", "ring_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryRingBorrowRead)
        }
        ["std", "kernel", "memory", "ring_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryRingBorrowEdit)
        }
        ["std", "kernel", "memory", "ring_set"] => Some(RuntimeIntrinsic::MemoryRingSet),
        ["std", "kernel", "memory", "ring_reset"] => Some(RuntimeIntrinsic::MemoryRingReset),
        ["std", "kernel", "memory", "ring_window_read"] => {
            Some(RuntimeIntrinsic::MemoryRingWindowRead)
        }
        ["std", "kernel", "memory", "ring_window_edit"] => {
            Some(RuntimeIntrinsic::MemoryRingWindowEdit)
        }
        ["std", "memory", "slab_new"] | ["std", "kernel", "memory", "slab_new"] => {
            Some(RuntimeIntrinsic::MemorySlabNew)
        }
        ["std", "kernel", "memory", "slab_alloc"] => Some(RuntimeIntrinsic::MemorySlabAlloc),
        ["std", "kernel", "memory", "slab_len"] => Some(RuntimeIntrinsic::MemorySlabLen),
        ["std", "kernel", "memory", "slab_has"] => Some(RuntimeIntrinsic::MemorySlabHas),
        ["std", "kernel", "memory", "slab_get"] => Some(RuntimeIntrinsic::MemorySlabGet),
        ["std", "kernel", "memory", "slab_borrow_read"] => {
            Some(RuntimeIntrinsic::MemorySlabBorrowRead)
        }
        ["std", "kernel", "memory", "slab_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemorySlabBorrowEdit)
        }
        ["std", "kernel", "memory", "slab_set"] => Some(RuntimeIntrinsic::MemorySlabSet),
        ["std", "kernel", "memory", "slab_remove"] => Some(RuntimeIntrinsic::MemorySlabRemove),
        ["std", "kernel", "memory", "slab_reset"] => Some(RuntimeIntrinsic::MemorySlabReset),
        ["std", "kernel", "memory", "slab_seal"] => Some(RuntimeIntrinsic::MemorySlabSeal),
        ["std", "kernel", "memory", "slab_unseal"] => Some(RuntimeIntrinsic::MemorySlabUnseal),
        ["std", "kernel", "memory", "slab_is_sealed"] => Some(RuntimeIntrinsic::MemorySlabIsSealed),
        ["std", "kernel", "memory", "slab_live_ids"] => Some(RuntimeIntrinsic::MemorySlabLiveIds),
        ["std", "kernel", "memory", "array_view_read"] => {
            Some(RuntimeIntrinsic::MemoryArrayViewRead)
        }
        ["std", "kernel", "memory", "array_view_edit"] => {
            Some(RuntimeIntrinsic::MemoryArrayViewEdit)
        }
        ["std", "kernel", "memory", "bytes_view"] => Some(RuntimeIntrinsic::MemoryBytesView),
        ["std", "kernel", "memory", "bytes_view_edit"] => {
            Some(RuntimeIntrinsic::MemoryBytesViewEdit)
        }
        ["std", "kernel", "memory", "mapped_view"] => Some(RuntimeIntrinsic::MemoryMappedView),
        ["std", "kernel", "memory", "mapped_view_edit"] => {
            Some(RuntimeIntrinsic::MemoryMappedViewEdit)
        }
        ["std", "kernel", "memory", "str_view"] => Some(RuntimeIntrinsic::MemoryStrView),
        ["std", "kernel", "memory", "view_len"] => Some(RuntimeIntrinsic::MemoryViewLen),
        ["std", "kernel", "memory", "view_get"] => Some(RuntimeIntrinsic::MemoryViewGet),
        ["std", "kernel", "memory", "view_subview"] => Some(RuntimeIntrinsic::MemoryViewSubview),
        ["std", "kernel", "memory", "edit_view_len"] => Some(RuntimeIntrinsic::MemoryEditViewLen),
        ["std", "kernel", "memory", "edit_view_get"] => Some(RuntimeIntrinsic::MemoryEditViewGet),
        ["std", "kernel", "memory", "edit_view_set"] => Some(RuntimeIntrinsic::MemoryEditViewSet),
        ["std", "kernel", "memory", "edit_view_subview_read"] => {
            Some(RuntimeIntrinsic::MemoryEditViewSubviewRead)
        }
        ["std", "kernel", "memory", "edit_view_subview_edit"] => {
            Some(RuntimeIntrinsic::MemoryEditViewSubviewEdit)
        }
        ["std", "kernel", "memory", "byte_view_len"] => Some(RuntimeIntrinsic::MemoryByteViewLen),
        ["std", "kernel", "memory", "byte_view_at"] => Some(RuntimeIntrinsic::MemoryByteViewAt),
        ["std", "kernel", "memory", "byte_view_subview"] => {
            Some(RuntimeIntrinsic::MemoryByteViewSubview)
        }
        ["std", "kernel", "memory", "byte_view_to_array"] => {
            Some(RuntimeIntrinsic::MemoryByteViewToArray)
        }
        ["std", "kernel", "memory", "byte_edit_view_len"] => {
            Some(RuntimeIntrinsic::MemoryByteEditViewLen)
        }
        ["std", "kernel", "memory", "byte_edit_view_at"] => {
            Some(RuntimeIntrinsic::MemoryByteEditViewAt)
        }
        ["std", "kernel", "memory", "byte_edit_view_set"] => {
            Some(RuntimeIntrinsic::MemoryByteEditViewSet)
        }
        ["std", "kernel", "memory", "byte_edit_view_subview_read"] => {
            Some(RuntimeIntrinsic::MemoryByteEditViewSubviewRead)
        }
        ["std", "kernel", "memory", "byte_edit_view_subview_edit"] => {
            Some(RuntimeIntrinsic::MemoryByteEditViewSubviewEdit)
        }
        ["std", "kernel", "memory", "byte_edit_view_to_array"] => {
            Some(RuntimeIntrinsic::MemoryByteEditViewToArray)
        }
        ["std", "kernel", "memory", "str_view_len_bytes"] => {
            Some(RuntimeIntrinsic::MemoryStrViewLenBytes)
        }
        ["std", "kernel", "memory", "str_view_byte_at"] => {
            Some(RuntimeIntrinsic::MemoryStrViewByteAt)
        }
        ["std", "kernel", "memory", "str_view_subview"] => {
            Some(RuntimeIntrinsic::MemoryStrViewSubview)
        }
        ["std", "kernel", "memory", "str_view_to_str"] => {
            Some(RuntimeIntrinsic::MemoryStrViewToStr)
        }
        _ => None,
    }
}

pub(super) fn resolve_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    match intrinsic_impl {
        "MemoryArenaNew" => Some(RuntimeIntrinsic::MemoryArenaNew),
        "MemoryArenaAlloc" => Some(RuntimeIntrinsic::MemoryArenaAlloc),
        "MemoryArenaLen" => Some(RuntimeIntrinsic::MemoryArenaLen),
        "MemoryArenaHas" => Some(RuntimeIntrinsic::MemoryArenaHas),
        "MemoryArenaGet" => Some(RuntimeIntrinsic::MemoryArenaGet),
        "MemoryArenaBorrowRead" => Some(RuntimeIntrinsic::MemoryArenaBorrowRead),
        "MemoryArenaBorrowEdit" => Some(RuntimeIntrinsic::MemoryArenaBorrowEdit),
        "MemoryArenaSet" => Some(RuntimeIntrinsic::MemoryArenaSet),
        "MemoryArenaRemove" => Some(RuntimeIntrinsic::MemoryArenaRemove),
        "MemoryArenaReset" => Some(RuntimeIntrinsic::MemoryArenaReset),
        "MemoryFrameNew" => Some(RuntimeIntrinsic::MemoryFrameNew),
        "MemoryFrameAlloc" => Some(RuntimeIntrinsic::MemoryFrameAlloc),
        "MemoryFrameLen" => Some(RuntimeIntrinsic::MemoryFrameLen),
        "MemoryFrameHas" => Some(RuntimeIntrinsic::MemoryFrameHas),
        "MemoryFrameGet" => Some(RuntimeIntrinsic::MemoryFrameGet),
        "MemoryFrameBorrowRead" => Some(RuntimeIntrinsic::MemoryFrameBorrowRead),
        "MemoryFrameBorrowEdit" => Some(RuntimeIntrinsic::MemoryFrameBorrowEdit),
        "MemoryFrameSet" => Some(RuntimeIntrinsic::MemoryFrameSet),
        "MemoryFrameReset" => Some(RuntimeIntrinsic::MemoryFrameReset),
        "MemoryPoolNew" => Some(RuntimeIntrinsic::MemoryPoolNew),
        "MemoryPoolAlloc" => Some(RuntimeIntrinsic::MemoryPoolAlloc),
        "MemoryPoolLen" => Some(RuntimeIntrinsic::MemoryPoolLen),
        "MemoryPoolHas" => Some(RuntimeIntrinsic::MemoryPoolHas),
        "MemoryPoolGet" => Some(RuntimeIntrinsic::MemoryPoolGet),
        "MemoryPoolBorrowRead" => Some(RuntimeIntrinsic::MemoryPoolBorrowRead),
        "MemoryPoolBorrowEdit" => Some(RuntimeIntrinsic::MemoryPoolBorrowEdit),
        "MemoryPoolSet" => Some(RuntimeIntrinsic::MemoryPoolSet),
        "MemoryPoolRemove" => Some(RuntimeIntrinsic::MemoryPoolRemove),
        "MemoryPoolReset" => Some(RuntimeIntrinsic::MemoryPoolReset),
        "MemoryPoolLiveIds" => Some(RuntimeIntrinsic::MemoryPoolLiveIds),
        "MemoryPoolCompact" => Some(RuntimeIntrinsic::MemoryPoolCompact),
        "MemoryTempNew" => Some(RuntimeIntrinsic::MemoryTempNew),
        "MemoryTempAlloc" => Some(RuntimeIntrinsic::MemoryTempAlloc),
        "MemoryTempLen" => Some(RuntimeIntrinsic::MemoryTempLen),
        "MemoryTempHas" => Some(RuntimeIntrinsic::MemoryTempHas),
        "MemoryTempGet" => Some(RuntimeIntrinsic::MemoryTempGet),
        "MemoryTempBorrowRead" => Some(RuntimeIntrinsic::MemoryTempBorrowRead),
        "MemoryTempBorrowEdit" => Some(RuntimeIntrinsic::MemoryTempBorrowEdit),
        "MemoryTempSet" => Some(RuntimeIntrinsic::MemoryTempSet),
        "MemoryTempReset" => Some(RuntimeIntrinsic::MemoryTempReset),
        "MemorySessionNew" => Some(RuntimeIntrinsic::MemorySessionNew),
        "MemorySessionAlloc" => Some(RuntimeIntrinsic::MemorySessionAlloc),
        "MemorySessionLen" => Some(RuntimeIntrinsic::MemorySessionLen),
        "MemorySessionHas" => Some(RuntimeIntrinsic::MemorySessionHas),
        "MemorySessionGet" => Some(RuntimeIntrinsic::MemorySessionGet),
        "MemorySessionBorrowRead" => Some(RuntimeIntrinsic::MemorySessionBorrowRead),
        "MemorySessionBorrowEdit" => Some(RuntimeIntrinsic::MemorySessionBorrowEdit),
        "MemorySessionSet" => Some(RuntimeIntrinsic::MemorySessionSet),
        "MemorySessionReset" => Some(RuntimeIntrinsic::MemorySessionReset),
        "MemorySessionSeal" => Some(RuntimeIntrinsic::MemorySessionSeal),
        "MemorySessionUnseal" => Some(RuntimeIntrinsic::MemorySessionUnseal),
        "MemorySessionIsSealed" => Some(RuntimeIntrinsic::MemorySessionIsSealed),
        "MemorySessionLiveIds" => Some(RuntimeIntrinsic::MemorySessionLiveIds),
        "MemoryRingNew" => Some(RuntimeIntrinsic::MemoryRingNew),
        "MemoryRingPush" => Some(RuntimeIntrinsic::MemoryRingPush),
        "MemoryRingTryPop" => Some(RuntimeIntrinsic::MemoryRingTryPop),
        "MemoryRingLen" => Some(RuntimeIntrinsic::MemoryRingLen),
        "MemoryRingHas" => Some(RuntimeIntrinsic::MemoryRingHas),
        "MemoryRingGet" => Some(RuntimeIntrinsic::MemoryRingGet),
        "MemoryRingBorrowRead" => Some(RuntimeIntrinsic::MemoryRingBorrowRead),
        "MemoryRingBorrowEdit" => Some(RuntimeIntrinsic::MemoryRingBorrowEdit),
        "MemoryRingSet" => Some(RuntimeIntrinsic::MemoryRingSet),
        "MemoryRingReset" => Some(RuntimeIntrinsic::MemoryRingReset),
        "MemoryRingWindowRead" => Some(RuntimeIntrinsic::MemoryRingWindowRead),
        "MemoryRingWindowEdit" => Some(RuntimeIntrinsic::MemoryRingWindowEdit),
        "MemorySlabNew" => Some(RuntimeIntrinsic::MemorySlabNew),
        "MemorySlabAlloc" => Some(RuntimeIntrinsic::MemorySlabAlloc),
        "MemorySlabLen" => Some(RuntimeIntrinsic::MemorySlabLen),
        "MemorySlabHas" => Some(RuntimeIntrinsic::MemorySlabHas),
        "MemorySlabGet" => Some(RuntimeIntrinsic::MemorySlabGet),
        "MemorySlabBorrowRead" => Some(RuntimeIntrinsic::MemorySlabBorrowRead),
        "MemorySlabBorrowEdit" => Some(RuntimeIntrinsic::MemorySlabBorrowEdit),
        "MemorySlabSet" => Some(RuntimeIntrinsic::MemorySlabSet),
        "MemorySlabRemove" => Some(RuntimeIntrinsic::MemorySlabRemove),
        "MemorySlabReset" => Some(RuntimeIntrinsic::MemorySlabReset),
        "MemorySlabSeal" => Some(RuntimeIntrinsic::MemorySlabSeal),
        "MemorySlabUnseal" => Some(RuntimeIntrinsic::MemorySlabUnseal),
        "MemorySlabIsSealed" => Some(RuntimeIntrinsic::MemorySlabIsSealed),
        "MemorySlabLiveIds" => Some(RuntimeIntrinsic::MemorySlabLiveIds),
        "MemoryArrayViewRead" => Some(RuntimeIntrinsic::MemoryArrayViewRead),
        "MemoryArrayViewEdit" => Some(RuntimeIntrinsic::MemoryArrayViewEdit),
        "MemoryBytesView" => Some(RuntimeIntrinsic::MemoryBytesView),
        "MemoryBytesViewEdit" => Some(RuntimeIntrinsic::MemoryBytesViewEdit),
        "MemoryMappedView" => Some(RuntimeIntrinsic::MemoryMappedView),
        "MemoryMappedViewEdit" => Some(RuntimeIntrinsic::MemoryMappedViewEdit),
        "MemoryStrView" => Some(RuntimeIntrinsic::MemoryStrView),
        "MemoryViewLen" => Some(RuntimeIntrinsic::MemoryViewLen),
        "MemoryViewGet" => Some(RuntimeIntrinsic::MemoryViewGet),
        "MemoryViewSubview" => Some(RuntimeIntrinsic::MemoryViewSubview),
        "MemoryEditViewLen" => Some(RuntimeIntrinsic::MemoryEditViewLen),
        "MemoryEditViewGet" => Some(RuntimeIntrinsic::MemoryEditViewGet),
        "MemoryEditViewSet" => Some(RuntimeIntrinsic::MemoryEditViewSet),
        "MemoryEditViewSubviewRead" => Some(RuntimeIntrinsic::MemoryEditViewSubviewRead),
        "MemoryEditViewSubviewEdit" => Some(RuntimeIntrinsic::MemoryEditViewSubviewEdit),
        "MemoryByteViewLen" => Some(RuntimeIntrinsic::MemoryByteViewLen),
        "MemoryByteViewAt" => Some(RuntimeIntrinsic::MemoryByteViewAt),
        "MemoryByteViewSubview" => Some(RuntimeIntrinsic::MemoryByteViewSubview),
        "MemoryByteViewToArray" => Some(RuntimeIntrinsic::MemoryByteViewToArray),
        "MemoryByteEditViewLen" => Some(RuntimeIntrinsic::MemoryByteEditViewLen),
        "MemoryByteEditViewAt" => Some(RuntimeIntrinsic::MemoryByteEditViewAt),
        "MemoryByteEditViewSet" => Some(RuntimeIntrinsic::MemoryByteEditViewSet),
        "MemoryByteEditViewSubviewRead" => Some(RuntimeIntrinsic::MemoryByteEditViewSubviewRead),
        "MemoryByteEditViewSubviewEdit" => Some(RuntimeIntrinsic::MemoryByteEditViewSubviewEdit),
        "MemoryByteEditViewToArray" => Some(RuntimeIntrinsic::MemoryByteEditViewToArray),
        "MemoryStrViewLenBytes" => Some(RuntimeIntrinsic::MemoryStrViewLenBytes),
        "MemoryStrViewByteAt" => Some(RuntimeIntrinsic::MemoryStrViewByteAt),
        "MemoryStrViewSubview" => Some(RuntimeIntrinsic::MemoryStrViewSubview),
        "MemoryStrViewToStr" => Some(RuntimeIntrinsic::MemoryStrViewToStr),
        _ => None,
    }
}
