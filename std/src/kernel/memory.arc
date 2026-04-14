import std.memory
import std.option

intrinsic fn arena_new[T](capacity: Int) -> Arena[T] = MemoryArenaNew
intrinsic fn arena_alloc[T](edit arena: Arena[T], take value: T) -> ArenaId[T] = MemoryArenaAlloc
intrinsic fn arena_len[T](read arena: Arena[T]) -> Int = MemoryArenaLen
intrinsic fn arena_has[T](read arena: Arena[T], id: ArenaId[T]) -> Bool = MemoryArenaHas
intrinsic fn arena_get[T](read arena: Arena[T], id: ArenaId[T]) -> T = MemoryArenaGet
intrinsic fn arena_borrow_read['arena, T](read arena: Arena[T], id: ArenaId[T]) -> &'arena T = MemoryArenaBorrowRead
intrinsic fn arena_borrow_edit['arena, T](edit arena: Arena[T], id: ArenaId[T]) -> &'arena mut T = MemoryArenaBorrowEdit
intrinsic fn arena_set[T](edit arena: Arena[T], id: ArenaId[T], take value: T) = MemoryArenaSet
intrinsic fn arena_remove[T](edit arena: Arena[T], id: ArenaId[T]) -> Bool = MemoryArenaRemove
intrinsic fn arena_reset[T](edit arena: Arena[T]) = MemoryArenaReset

intrinsic fn frame_new[T](capacity: Int) -> FrameArena[T] = MemoryFrameNew
intrinsic fn frame_alloc[T](edit arena: FrameArena[T], take value: T) -> FrameId[T] = MemoryFrameAlloc
intrinsic fn frame_len[T](read arena: FrameArena[T]) -> Int = MemoryFrameLen
intrinsic fn frame_has[T](read arena: FrameArena[T], id: FrameId[T]) -> Bool = MemoryFrameHas
intrinsic fn frame_get[T](read arena: FrameArena[T], id: FrameId[T]) -> T = MemoryFrameGet
intrinsic fn frame_borrow_read['frame, T](read arena: FrameArena[T], id: FrameId[T]) -> &'frame T = MemoryFrameBorrowRead
intrinsic fn frame_borrow_edit['frame, T](edit arena: FrameArena[T], id: FrameId[T]) -> &'frame mut T = MemoryFrameBorrowEdit
intrinsic fn frame_set[T](edit arena: FrameArena[T], id: FrameId[T], take value: T) = MemoryFrameSet
intrinsic fn frame_reset[T](edit arena: FrameArena[T]) = MemoryFrameReset

intrinsic fn pool_new[T](capacity: Int) -> PoolArena[T] = MemoryPoolNew
intrinsic fn pool_alloc[T](edit arena: PoolArena[T], take value: T) -> PoolId[T] = MemoryPoolAlloc
intrinsic fn pool_len[T](read arena: PoolArena[T]) -> Int = MemoryPoolLen
intrinsic fn pool_has[T](read arena: PoolArena[T], id: PoolId[T]) -> Bool = MemoryPoolHas
intrinsic fn pool_get[T](read arena: PoolArena[T], id: PoolId[T]) -> T = MemoryPoolGet
intrinsic fn pool_borrow_read['pool, T](read arena: PoolArena[T], id: PoolId[T]) -> &'pool T = MemoryPoolBorrowRead
intrinsic fn pool_borrow_edit['pool, T](edit arena: PoolArena[T], id: PoolId[T]) -> &'pool mut T = MemoryPoolBorrowEdit
intrinsic fn pool_set[T](edit arena: PoolArena[T], id: PoolId[T], take value: T) = MemoryPoolSet
intrinsic fn pool_remove[T](edit arena: PoolArena[T], id: PoolId[T]) -> Bool = MemoryPoolRemove
intrinsic fn pool_reset[T](edit arena: PoolArena[T]) = MemoryPoolReset
intrinsic fn pool_live_ids[T](read arena: PoolArena[T]) -> List[PoolId[T]] = MemoryPoolLiveIds
intrinsic fn pool_compact[T](edit arena: PoolArena[T]) -> List[std.memory.PoolRelocation[T]] = MemoryPoolCompact

intrinsic fn temp_new[T](capacity: Int) -> std.memory.TempArena[T] = MemoryTempNew
intrinsic fn temp_alloc[T](edit arena: std.memory.TempArena[T], take value: T) -> std.memory.TempId[T] = MemoryTempAlloc
intrinsic fn temp_len[T](read arena: std.memory.TempArena[T]) -> Int = MemoryTempLen
intrinsic fn temp_has[T](read arena: std.memory.TempArena[T], id: std.memory.TempId[T]) -> Bool = MemoryTempHas
intrinsic fn temp_get[T](read arena: std.memory.TempArena[T], id: std.memory.TempId[T]) -> T = MemoryTempGet
intrinsic fn temp_borrow_read['temp, T](read arena: std.memory.TempArena[T], id: std.memory.TempId[T]) -> &'temp T = MemoryTempBorrowRead
intrinsic fn temp_borrow_edit['temp, T](edit arena: std.memory.TempArena[T], id: std.memory.TempId[T]) -> &'temp mut T = MemoryTempBorrowEdit
intrinsic fn temp_set[T](edit arena: std.memory.TempArena[T], id: std.memory.TempId[T], take value: T) = MemoryTempSet
intrinsic fn temp_reset[T](edit arena: std.memory.TempArena[T]) = MemoryTempReset

intrinsic fn session_new[T](capacity: Int) -> std.memory.SessionArena[T] = MemorySessionNew
intrinsic fn session_alloc[T](edit arena: std.memory.SessionArena[T], take value: T) -> std.memory.SessionId[T] = MemorySessionAlloc
intrinsic fn session_len[T](read arena: std.memory.SessionArena[T]) -> Int = MemorySessionLen
intrinsic fn session_has[T](read arena: std.memory.SessionArena[T], id: std.memory.SessionId[T]) -> Bool = MemorySessionHas
intrinsic fn session_get[T](read arena: std.memory.SessionArena[T], id: std.memory.SessionId[T]) -> T = MemorySessionGet
intrinsic fn session_borrow_read['session, T](read arena: std.memory.SessionArena[T], id: std.memory.SessionId[T]) -> &'session T = MemorySessionBorrowRead
intrinsic fn session_borrow_edit['session, T](edit arena: std.memory.SessionArena[T], id: std.memory.SessionId[T]) -> &'session mut T = MemorySessionBorrowEdit
intrinsic fn session_set[T](edit arena: std.memory.SessionArena[T], id: std.memory.SessionId[T], take value: T) = MemorySessionSet
intrinsic fn session_reset[T](edit arena: std.memory.SessionArena[T]) = MemorySessionReset
intrinsic fn session_seal[T](edit arena: std.memory.SessionArena[T]) = MemorySessionSeal
intrinsic fn session_unseal[T](edit arena: std.memory.SessionArena[T]) = MemorySessionUnseal
intrinsic fn session_is_sealed[T](read arena: std.memory.SessionArena[T]) -> Bool = MemorySessionIsSealed
intrinsic fn session_live_ids[T](read arena: std.memory.SessionArena[T]) -> List[std.memory.SessionId[T]] = MemorySessionLiveIds

intrinsic fn ring_new[T](capacity: Int) -> std.memory.RingBuffer[T] = MemoryRingNew
intrinsic fn ring_push[T](edit arena: std.memory.RingBuffer[T], take value: T) -> std.memory.RingId[T] = MemoryRingPush
intrinsic fn ring_try_pop[T](edit arena: std.memory.RingBuffer[T]) -> std.option.Option[T] = MemoryRingTryPop
intrinsic fn ring_len[T](read arena: std.memory.RingBuffer[T]) -> Int = MemoryRingLen
intrinsic fn ring_has[T](read arena: std.memory.RingBuffer[T], id: std.memory.RingId[T]) -> Bool = MemoryRingHas
intrinsic fn ring_get[T](read arena: std.memory.RingBuffer[T], id: std.memory.RingId[T]) -> T = MemoryRingGet
intrinsic fn ring_borrow_read['ring, T](read arena: std.memory.RingBuffer[T], id: std.memory.RingId[T]) -> &'ring T = MemoryRingBorrowRead
intrinsic fn ring_borrow_edit['ring, T](edit arena: std.memory.RingBuffer[T], id: std.memory.RingId[T]) -> &'ring mut T = MemoryRingBorrowEdit
intrinsic fn ring_set[T](edit arena: std.memory.RingBuffer[T], id: std.memory.RingId[T], take value: T) = MemoryRingSet
intrinsic fn ring_reset[T](edit arena: std.memory.RingBuffer[T]) = MemoryRingReset
intrinsic fn ring_window_read[T](read arena: std.memory.RingBuffer[T], start: Int, len: Int) -> View[T, Strided] = MemoryRingWindowRead
intrinsic fn ring_window_edit[T](edit arena: std.memory.RingBuffer[T], start: Int, len: Int) -> View[T, Strided] = MemoryRingWindowEdit

intrinsic fn slab_new[T](capacity: Int) -> std.memory.Slab[T] = MemorySlabNew
intrinsic fn slab_alloc[T](edit arena: std.memory.Slab[T], take value: T) -> std.memory.SlabId[T] = MemorySlabAlloc
intrinsic fn slab_len[T](read arena: std.memory.Slab[T]) -> Int = MemorySlabLen
intrinsic fn slab_has[T](read arena: std.memory.Slab[T], id: std.memory.SlabId[T]) -> Bool = MemorySlabHas
intrinsic fn slab_get[T](read arena: std.memory.Slab[T], id: std.memory.SlabId[T]) -> T = MemorySlabGet
intrinsic fn slab_borrow_read['slab, T](read arena: std.memory.Slab[T], id: std.memory.SlabId[T]) -> &'slab T = MemorySlabBorrowRead
intrinsic fn slab_borrow_edit['slab, T](edit arena: std.memory.Slab[T], id: std.memory.SlabId[T]) -> &'slab mut T = MemorySlabBorrowEdit
intrinsic fn slab_set[T](edit arena: std.memory.Slab[T], id: std.memory.SlabId[T], take value: T) = MemorySlabSet
intrinsic fn slab_remove[T](edit arena: std.memory.Slab[T], id: std.memory.SlabId[T]) -> Bool = MemorySlabRemove
intrinsic fn slab_reset[T](edit arena: std.memory.Slab[T]) = MemorySlabReset
intrinsic fn slab_seal[T](edit arena: std.memory.Slab[T]) = MemorySlabSeal
intrinsic fn slab_unseal[T](edit arena: std.memory.Slab[T]) = MemorySlabUnseal
intrinsic fn slab_is_sealed[T](read arena: std.memory.Slab[T]) -> Bool = MemorySlabIsSealed
intrinsic fn slab_live_ids[T](read arena: std.memory.Slab[T]) -> List[std.memory.SlabId[T]] = MemorySlabLiveIds

intrinsic fn mapped_view(package: Str, handle: Int, len: Int) -> View[U8, Mapped] = MemoryMappedView
intrinsic fn mapped_view_edit(package: Str, handle: Int, len: Int) -> View[U8, Mapped] = MemoryMappedViewEdit

intrinsic fn view_len[T, F](read view: View[T, F]) -> Int = MemoryViewLen
intrinsic fn view_get[T, F](read view: View[T, F], index: Int) -> T = MemoryViewGet
intrinsic fn view_set[T, F](edit view: View[T, F], index: Int, take value: T) = MemoryEditViewSet
intrinsic fn view_subview[T, F](read view: View[T, F], start: Int, end: Int) -> View[T, F] = MemoryViewSubview
intrinsic fn view_to_str(read view: View[U8, Contiguous]) -> Str = MemoryStrViewToStr
