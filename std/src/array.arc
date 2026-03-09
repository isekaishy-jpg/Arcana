import std.collections.array

impl[T] Array[T]:
    fn len(read self: Array[T]) -> Int:
        return std.collections.array.len :: self :: call

    fn to_list(read self: Array[T]) -> List[T]:
        return std.collections.array.to_list :: self :: call
