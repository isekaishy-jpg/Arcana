import std.kernel.collections

export fn new[T]() -> List[T]:
    return std.kernel.collections.list_new[T] :: :: call

impl[T] List[T]:
    fn len(read self: List[T]) -> Int:
        return std.kernel.collections.list_len :: self :: call

    fn push(edit self: List[T], take value: T):
        std.kernel.collections.list_push :: self, value :: call

    fn pop(edit self: List[T]) -> T:
        return std.kernel.collections.list_pop :: self :: call

    fn try_pop_or(edit self: List[T], take fallback: T) -> (Bool, T):
        return std.kernel.collections.list_try_pop_or :: self, fallback :: call
