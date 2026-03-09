import std.kernel.memory

export fn new[T](capacity: Int) -> Arena[T]:
    return std.kernel.memory.arena_new[T] :: capacity :: call

export fn frame_new[T](capacity: Int) -> FrameArena[T]:
    return std.kernel.memory.frame_new[T] :: capacity :: call

export fn pool_new[T](capacity: Int) -> PoolArena[T]:
    return std.kernel.memory.pool_new[T] :: capacity :: call

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

    fn borrow_read['pool](read self: PoolArena[T], id: PoolId[T]) -> &'pool T:
        return std.kernel.memory.pool_borrow_read :: self, id :: call

    fn borrow_edit['pool](edit self: PoolArena[T], id: PoolId[T]) -> &'pool mut T:
        return std.kernel.memory.pool_borrow_edit :: self, id :: call
