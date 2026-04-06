import std.collections.array

export fn empty[T]() -> Array[T]:
    return std.collections.array.empty[T] :: :: call

export fn len[T](read values: Array[T]) -> Int:
    return std.collections.array.len :: values :: call

export fn to_list[T](read values: Array[T]) -> List[T]:
    return std.collections.array.to_list :: values :: call
