import std.collections.array
import std.kernel.collections

export fn new[T]() -> List[T]:
    return std.kernel.collections.list_new[T] :: :: call

impl[T] List[T]:
    fn len(read self: List[T]) -> Int:
        return std.kernel.collections.list_len :: self :: call

    fn is_empty(read self: List[T]) -> Bool:
        return (self :: :: len) == 0

    fn push(edit self: List[T], take value: T):
        std.kernel.collections.list_push :: self, value :: call

    fn pop(edit self: List[T]) -> T:
        return std.kernel.collections.list_pop :: self :: call

    fn try_pop_or(edit self: List[T], take fallback: T) -> (Bool, T):
        return std.kernel.collections.list_try_pop_or :: self, fallback :: call

    fn clear(edit self: List[T]):
        while not (self :: :: is_empty):
            self :: :: pop

    fn extend_list(edit self: List[T], read other: List[T]):
        for value in other:
            self :: value :: push

    fn extend_array(edit self: List[T], read other: Array[T]):
        for value in other:
            self :: value :: push
