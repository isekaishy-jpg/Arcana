import std.collections.array
import std.collections.list
import std.kernel.memory
import std.option
use std.option.Option

export opaque type TempArena[T] as move, boundary_unsafe
export opaque type TempId[T] as copy, boundary_unsafe
export opaque type SessionArena[T] as move, boundary_unsafe
export opaque type SessionId[T] as copy, boundary_unsafe
export opaque type RingBuffer[T] as move, boundary_unsafe
export opaque type RingId[T] as copy, boundary_unsafe
export opaque type Slab[T] as move, boundary_unsafe
export opaque type SlabId[T] as copy, boundary_unsafe

lang temp_arena_handle = TempArena
lang temp_id_handle = TempId
lang session_arena_handle = SessionArena
lang session_id_handle = SessionId
lang ring_buffer_handle = RingBuffer
lang ring_id_handle = RingId
lang slab_handle = Slab
lang slab_id_handle = SlabId

export record PoolRelocation[T]:
    old: PoolId[T]
    new: PoolId[T]

export trait Resettable[S]:
    fn reset_value(edit self: S)

export trait IdAllocating[S]:
    type Id
    fn has_id(read self: S, id: std.memory.IdAllocating[S].Id) -> Bool

export trait LiveIterable[S]:
    type Id
    fn live_ids_of(read self: S) -> List[std.memory.LiveIterable[S].Id]

export trait Compactable[S]:
    type Relocation
    fn compact_items(edit self: S) -> List[std.memory.Compactable[S].Relocation]

export trait SequenceBuffer[S]:
    type Item
    type Id
    fn push_item(edit self: S, take value: std.memory.SequenceBuffer[S].Item) -> std.memory.SequenceBuffer[S].Id
    fn pop_item(edit self: S) -> Option[std.memory.SequenceBuffer[S].Item]

export trait Sealable[S]:
    fn seal_state(edit self: S)
    fn unseal_state(edit self: S)
    fn state_is_sealed(read self: S) -> Bool

export fn new[T](capacity: Int) -> Arena[T]:
    return std.kernel.memory.arena_new[T] :: capacity :: call

export fn frame_new[T](capacity: Int) -> FrameArena[T]:
    return std.kernel.memory.frame_new[T] :: capacity :: call

export fn pool_new[T](capacity: Int) -> PoolArena[T]:
    return std.kernel.memory.pool_new[T] :: capacity :: call

export fn temp_new[T](capacity: Int) -> TempArena[T]:
    return std.kernel.memory.temp_new[T] :: capacity :: call

export fn session_new[T](capacity: Int) -> SessionArena[T]:
    return std.kernel.memory.session_new[T] :: capacity :: call

export fn ring_new[T](capacity: Int) -> RingBuffer[T]:
    return std.kernel.memory.ring_new[T] :: capacity :: call

export fn slab_new[T](capacity: Int) -> Slab[T]:
    return std.kernel.memory.slab_new[T] :: capacity :: call

impl[T] Arena[T]:
    fn len(read self: Arena[T]) -> Int:
        return std.kernel.memory.arena_len :: self :: call

    fn has(read self: Arena[T], id: ArenaId[T]) -> Bool:
        return std.kernel.memory.arena_has :: self, id :: call

    fn get(read self: Arena[T], id: ArenaId[T]) -> T:
        return std.kernel.memory.arena_get :: self, id :: call

    fn set(edit self: Arena[T], id: ArenaId[T], take value: T):
        std.kernel.memory.arena_set :: self, id, value :: call

    fn remove(edit self: Arena[T], id: ArenaId[T]) -> Bool:
        return std.kernel.memory.arena_remove :: self, id :: call

    fn reset(edit self: Arena[T]):
        std.kernel.memory.arena_reset :: self :: call

    fn borrow_read['arena](read self: Arena[T], id: ArenaId[T]) -> &'arena T:
        return std.kernel.memory.arena_borrow_read :: self, id :: call

    fn borrow_edit['arena](edit self: Arena[T], id: ArenaId[T]) -> &'arena mut T:
        return std.kernel.memory.arena_borrow_edit :: self, id :: call

impl[T] FrameArena[T]:
    fn len(read self: FrameArena[T]) -> Int:
        return std.kernel.memory.frame_len :: self :: call

    fn has(read self: FrameArena[T], id: FrameId[T]) -> Bool:
        return std.kernel.memory.frame_has :: self, id :: call

    fn get(read self: FrameArena[T], id: FrameId[T]) -> T:
        return std.kernel.memory.frame_get :: self, id :: call

    fn set(edit self: FrameArena[T], id: FrameId[T], take value: T):
        std.kernel.memory.frame_set :: self, id, value :: call

    fn reset(edit self: FrameArena[T]):
        std.kernel.memory.frame_reset :: self :: call

    fn borrow_read['frame](read self: FrameArena[T], id: FrameId[T]) -> &'frame T:
        return std.kernel.memory.frame_borrow_read :: self, id :: call

    fn borrow_edit['frame](edit self: FrameArena[T], id: FrameId[T]) -> &'frame mut T:
        return std.kernel.memory.frame_borrow_edit :: self, id :: call

impl[T] PoolArena[T]:
    fn len(read self: PoolArena[T]) -> Int:
        return std.kernel.memory.pool_len :: self :: call

    fn has(read self: PoolArena[T], id: PoolId[T]) -> Bool:
        return std.kernel.memory.pool_has :: self, id :: call

    fn get(read self: PoolArena[T], id: PoolId[T]) -> T:
        return std.kernel.memory.pool_get :: self, id :: call

    fn set(edit self: PoolArena[T], id: PoolId[T], take value: T):
        std.kernel.memory.pool_set :: self, id, value :: call

    fn remove(edit self: PoolArena[T], id: PoolId[T]) -> Bool:
        return std.kernel.memory.pool_remove :: self, id :: call

    fn reset(edit self: PoolArena[T]):
        std.kernel.memory.pool_reset :: self :: call

    fn live_ids(read self: PoolArena[T]) -> List[PoolId[T]]:
        return std.kernel.memory.pool_live_ids[T] :: self :: call

    fn compact(edit self: PoolArena[T]) -> List[std.memory.PoolRelocation[T]]:
        return std.kernel.memory.pool_compact[T] :: self :: call

    fn borrow_read['pool](read self: PoolArena[T], id: PoolId[T]) -> &'pool T:
        return std.kernel.memory.pool_borrow_read :: self, id :: call

    fn borrow_edit['pool](edit self: PoolArena[T], id: PoolId[T]) -> &'pool mut T:
        return std.kernel.memory.pool_borrow_edit :: self, id :: call

impl[T] TempArena[T]:
    fn len(read self: TempArena[T]) -> Int:
        return std.kernel.memory.temp_len :: self :: call

    fn has(read self: TempArena[T], id: TempId[T]) -> Bool:
        return std.kernel.memory.temp_has :: self, id :: call

    fn get(read self: TempArena[T], id: TempId[T]) -> T:
        return std.kernel.memory.temp_get :: self, id :: call

    fn set(edit self: TempArena[T], id: TempId[T], take value: T):
        std.kernel.memory.temp_set :: self, id, value :: call

    fn reset(edit self: TempArena[T]):
        std.kernel.memory.temp_reset :: self :: call

    fn borrow_read['temp](read self: TempArena[T], id: TempId[T]) -> &'temp T:
        return std.kernel.memory.temp_borrow_read :: self, id :: call

    fn borrow_edit['temp](edit self: TempArena[T], id: TempId[T]) -> &'temp mut T:
        return std.kernel.memory.temp_borrow_edit :: self, id :: call

impl[T] SessionArena[T]:
    fn len(read self: SessionArena[T]) -> Int:
        return std.kernel.memory.session_len :: self :: call

    fn has(read self: SessionArena[T], id: SessionId[T]) -> Bool:
        return std.kernel.memory.session_has :: self, id :: call

    fn get(read self: SessionArena[T], id: SessionId[T]) -> T:
        return std.kernel.memory.session_get :: self, id :: call

    fn set(edit self: SessionArena[T], id: SessionId[T], take value: T):
        std.kernel.memory.session_set :: self, id, value :: call

    fn reset(edit self: SessionArena[T]):
        std.kernel.memory.session_reset :: self :: call

    fn seal(edit self: SessionArena[T]):
        std.kernel.memory.session_seal :: self :: call

    fn unseal(edit self: SessionArena[T]):
        std.kernel.memory.session_unseal :: self :: call

    fn is_sealed(read self: SessionArena[T]) -> Bool:
        return std.kernel.memory.session_is_sealed :: self :: call

    fn live_ids(read self: SessionArena[T]) -> List[SessionId[T]]:
        return std.kernel.memory.session_live_ids[T] :: self :: call

    fn borrow_read['session](read self: SessionArena[T], id: SessionId[T]) -> &'session T:
        return std.kernel.memory.session_borrow_read :: self, id :: call

    fn borrow_edit['session](edit self: SessionArena[T], id: SessionId[T]) -> &'session mut T:
        return std.kernel.memory.session_borrow_edit :: self, id :: call

impl[T] RingBuffer[T]:
    fn len(read self: RingBuffer[T]) -> Int:
        return std.kernel.memory.ring_len :: self :: call

    fn has(read self: RingBuffer[T], id: RingId[T]) -> Bool:
        return std.kernel.memory.ring_has :: self, id :: call

    fn get(read self: RingBuffer[T], id: RingId[T]) -> T:
        return std.kernel.memory.ring_get :: self, id :: call

    fn set(edit self: RingBuffer[T], id: RingId[T], take value: T):
        std.kernel.memory.ring_set :: self, id, value :: call

    fn push(edit self: RingBuffer[T], take value: T) -> RingId[T]:
        return std.kernel.memory.ring_push[T] :: self, value :: call

    fn pop(edit self: RingBuffer[T]) -> Option[T]:
        return std.kernel.memory.ring_try_pop[T] :: self :: call

    fn reset(edit self: RingBuffer[T]):
        std.kernel.memory.ring_reset :: self :: call

    fn window_read(read self: RingBuffer[T], start: Int, len: Int) -> View[T, Strided]:
        return std.kernel.memory.ring_window_read[T] :: self, start, len :: call

    fn window_edit(edit self: RingBuffer[T], start: Int, len: Int) -> View[T, Strided]:
        return std.kernel.memory.ring_window_edit[T] :: self, start, len :: call

    fn borrow_read['ring](read self: RingBuffer[T], id: RingId[T]) -> &'ring T:
        return std.kernel.memory.ring_borrow_read :: self, id :: call

    fn borrow_edit['ring](edit self: RingBuffer[T], id: RingId[T]) -> &'ring mut T:
        return std.kernel.memory.ring_borrow_edit :: self, id :: call

impl[T] Slab[T]:
    fn len(read self: Slab[T]) -> Int:
        return std.kernel.memory.slab_len :: self :: call

    fn has(read self: Slab[T], id: SlabId[T]) -> Bool:
        return std.kernel.memory.slab_has :: self, id :: call

    fn get(read self: Slab[T], id: SlabId[T]) -> T:
        return std.kernel.memory.slab_get :: self, id :: call

    fn set(edit self: Slab[T], id: SlabId[T], take value: T):
        std.kernel.memory.slab_set :: self, id, value :: call

    fn remove(edit self: Slab[T], id: SlabId[T]) -> Bool:
        return std.kernel.memory.slab_remove :: self, id :: call

    fn reset(edit self: Slab[T]):
        std.kernel.memory.slab_reset :: self :: call

    fn seal(edit self: Slab[T]):
        std.kernel.memory.slab_seal :: self :: call

    fn unseal(edit self: Slab[T]):
        std.kernel.memory.slab_unseal :: self :: call

    fn is_sealed(read self: Slab[T]) -> Bool:
        return std.kernel.memory.slab_is_sealed :: self :: call

    fn live_ids(read self: Slab[T]) -> List[SlabId[T]]:
        return std.kernel.memory.slab_live_ids[T] :: self :: call

    fn borrow_read['slab](read self: Slab[T], id: SlabId[T]) -> &'slab T:
        return std.kernel.memory.slab_borrow_read :: self, id :: call

    fn borrow_edit['slab](edit self: Slab[T], id: SlabId[T]) -> &'slab mut T:
        return std.kernel.memory.slab_borrow_edit :: self, id :: call

impl[T] View[T, Contiguous]:
    fn len(read self: View[T, Contiguous]) -> Int:
        return std.kernel.memory.view_len[T, Contiguous] :: self :: call

    fn get(read self: View[T, Contiguous], index: Int) -> T:
        return std.kernel.memory.view_get[T, Contiguous] :: self, index :: call

    fn set(edit self: View[T, Contiguous], index: Int, take value: T):
        std.kernel.memory.view_set[T, Contiguous] :: self, index, value :: call

    fn subview(read self: View[T, Contiguous], start: Int, end: Int) -> View[T, Contiguous]:
        return std.kernel.memory.view_subview[T, Contiguous] :: self, start, end :: call

    fn to_array(read self: View[T, Contiguous]) -> Array[T]:
        let mut items = std.collections.list.new[T] :: :: call
        let total = self :: :: len
        let mut index = 0
        while index < total:
            items :: (self :: index :: get) :: push
            index += 1
        return std.collections.array.from_list[T] :: items :: call

impl View[U8, Contiguous]:
    fn to_str(read self: View[U8, Contiguous]) -> Str:
        return std.kernel.memory.view_to_str :: self :: call

impl[T] View[T, Strided]:
    fn len(read self: View[T, Strided]) -> Int:
        return std.kernel.memory.view_len[T, Strided] :: self :: call

    fn get(read self: View[T, Strided], index: Int) -> T:
        return std.kernel.memory.view_get[T, Strided] :: self, index :: call

    fn set(edit self: View[T, Strided], index: Int, take value: T):
        std.kernel.memory.view_set[T, Strided] :: self, index, value :: call

    fn subview(read self: View[T, Strided], start: Int, end: Int) -> View[T, Strided]:
        return std.kernel.memory.view_subview[T, Strided] :: self, start, end :: call

impl View[U8, Mapped]:
    fn len(read self: View[U8, Mapped]) -> Int:
        return std.kernel.memory.view_len[U8, Mapped] :: self :: call

    fn get(read self: View[U8, Mapped], index: Int) -> Int:
        return Int :: (std.kernel.memory.view_get[U8, Mapped] :: self, index :: call) :: call

    fn set(edit self: View[U8, Mapped], index: Int, value: Int):
        std.kernel.memory.view_set[U8, Mapped] :: self, index, (U8 :: value :: call) :: call

    fn subview(read self: View[U8, Mapped], start: Int, end: Int) -> View[U8, Mapped]:
        return std.kernel.memory.view_subview[U8, Mapped] :: self, start, end :: call

impl[T] std.memory.Resettable[Arena[T]] for Arena[T]:
    fn reset_value(edit self: Arena[T]):
        std.kernel.memory.arena_reset :: self :: call

impl[T] std.memory.Resettable[FrameArena[T]] for FrameArena[T]:
    fn reset_value(edit self: FrameArena[T]):
        std.kernel.memory.frame_reset :: self :: call

impl[T] std.memory.Resettable[PoolArena[T]] for PoolArena[T]:
    fn reset_value(edit self: PoolArena[T]):
        std.kernel.memory.pool_reset :: self :: call

impl[T] std.memory.Resettable[TempArena[T]] for TempArena[T]:
    fn reset_value(edit self: TempArena[T]):
        std.kernel.memory.temp_reset :: self :: call

impl[T] std.memory.Resettable[SessionArena[T]] for SessionArena[T]:
    fn reset_value(edit self: SessionArena[T]):
        std.kernel.memory.session_reset :: self :: call

impl[T] std.memory.Resettable[RingBuffer[T]] for RingBuffer[T]:
    fn reset_value(edit self: RingBuffer[T]):
        std.kernel.memory.ring_reset :: self :: call

impl[T] std.memory.Resettable[Slab[T]] for Slab[T]:
    fn reset_value(edit self: Slab[T]):
        std.kernel.memory.slab_reset :: self :: call

impl[T] std.memory.IdAllocating[Arena[T]] for Arena[T]:
    type Id = ArenaId[T]
    fn has_id(read self: Arena[T], id: ArenaId[T]) -> Bool:
        return std.kernel.memory.arena_has :: self, id :: call

impl[T] std.memory.IdAllocating[FrameArena[T]] for FrameArena[T]:
    type Id = FrameId[T]
    fn has_id(read self: FrameArena[T], id: FrameId[T]) -> Bool:
        return std.kernel.memory.frame_has :: self, id :: call

impl[T] std.memory.IdAllocating[PoolArena[T]] for PoolArena[T]:
    type Id = PoolId[T]
    fn has_id(read self: PoolArena[T], id: PoolId[T]) -> Bool:
        return std.kernel.memory.pool_has :: self, id :: call

impl[T] std.memory.IdAllocating[TempArena[T]] for TempArena[T]:
    type Id = TempId[T]
    fn has_id(read self: TempArena[T], id: TempId[T]) -> Bool:
        return std.kernel.memory.temp_has :: self, id :: call

impl[T] std.memory.IdAllocating[SessionArena[T]] for SessionArena[T]:
    type Id = SessionId[T]
    fn has_id(read self: SessionArena[T], id: SessionId[T]) -> Bool:
        return std.kernel.memory.session_has :: self, id :: call

impl[T] std.memory.IdAllocating[RingBuffer[T]] for RingBuffer[T]:
    type Id = RingId[T]
    fn has_id(read self: RingBuffer[T], id: RingId[T]) -> Bool:
        return std.kernel.memory.ring_has :: self, id :: call

impl[T] std.memory.IdAllocating[Slab[T]] for Slab[T]:
    type Id = SlabId[T]
    fn has_id(read self: Slab[T], id: SlabId[T]) -> Bool:
        return std.kernel.memory.slab_has :: self, id :: call

impl[T] std.memory.LiveIterable[PoolArena[T]] for PoolArena[T]:
    type Id = PoolId[T]
    fn live_ids_of(read self: PoolArena[T]) -> List[PoolId[T]]:
        return std.kernel.memory.pool_live_ids[T] :: self :: call

impl[T] std.memory.LiveIterable[SessionArena[T]] for SessionArena[T]:
    type Id = SessionId[T]
    fn live_ids_of(read self: SessionArena[T]) -> List[SessionId[T]]:
        return std.kernel.memory.session_live_ids[T] :: self :: call

impl[T] std.memory.LiveIterable[Slab[T]] for Slab[T]:
    type Id = SlabId[T]
    fn live_ids_of(read self: Slab[T]) -> List[SlabId[T]]:
        return std.kernel.memory.slab_live_ids[T] :: self :: call

impl[T] std.memory.Compactable[PoolArena[T]] for PoolArena[T]:
    type Relocation = PoolRelocation[T]
    fn compact_items(edit self: PoolArena[T]) -> List[PoolRelocation[T]]:
        return std.kernel.memory.pool_compact[T] :: self :: call

impl[T] std.memory.SequenceBuffer[RingBuffer[T]] for RingBuffer[T]:
    type Item = T
    type Id = RingId[T]
    fn push_item(edit self: RingBuffer[T], take value: T) -> RingId[T]:
        return std.kernel.memory.ring_push[T] :: self, value :: call

    fn pop_item(edit self: RingBuffer[T]) -> Option[T]:
        return std.kernel.memory.ring_try_pop[T] :: self :: call

impl[T] std.memory.Sealable[SessionArena[T]] for SessionArena[T]:
    fn seal_state(edit self: SessionArena[T]):
        std.kernel.memory.session_seal :: self :: call

    fn unseal_state(edit self: SessionArena[T]):
        std.kernel.memory.session_unseal :: self :: call

    fn state_is_sealed(read self: SessionArena[T]) -> Bool:
        return std.kernel.memory.session_is_sealed :: self :: call

impl[T] std.memory.Sealable[Slab[T]] for Slab[T]:
    fn seal_state(edit self: Slab[T]):
        std.kernel.memory.slab_seal :: self :: call

    fn unseal_state(edit self: Slab[T]):
        std.kernel.memory.slab_unseal :: self :: call

    fn state_is_sealed(read self: Slab[T]) -> Bool:
        return std.kernel.memory.slab_is_sealed :: self :: call
