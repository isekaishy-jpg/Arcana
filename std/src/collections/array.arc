import std.kernel.collections

export fn new[T](len: Int, fill: T) -> Array[T]:
    return std.kernel.collections.array_new[T] :: len, fill :: call

export fn from_list[T](take xs: List[T]) -> Array[T]:
    return std.kernel.collections.array_from_list[T] :: xs :: call

impl[T] Array[T]:
    fn len(read self: Array[T]) -> Int:
        return std.kernel.collections.array_len :: self :: call

    fn to_list(read self: Array[T]) -> List[T]:
        return std.kernel.collections.array_to_list :: self :: call
