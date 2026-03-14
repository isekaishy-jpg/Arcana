import std.collections.array

export fn len[T](read values: Array[T]) -> Int:
    return std.collections.array.len :: values :: call

export fn to_list[T](read values: Array[T]) -> List[T]:
    return std.collections.array.to_list :: values :: call
