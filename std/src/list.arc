import std.collections.list

impl[T] List[T]:
    fn len(read self: List[T]) -> Int:
        return std.collections.list.len :: self :: call

    fn push(edit self: List[T], take value: T):
        std.collections.list.push :: self, value :: call

    fn pop_or(edit self: List[T], take fallback: T) -> (Bool, T):
        return std.collections.list.try_pop_or :: self, fallback :: call
