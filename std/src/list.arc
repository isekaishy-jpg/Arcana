import std.collections.list

export fn len[T](read values: List[T]) -> Int:
    return std.collections.list.len :: values :: call

export fn push[T](edit values: List[T], take value: T):
    std.collections.list.push :: values, value :: call

export fn pop_or[T](edit values: List[T], take fallback: T) -> (Bool, T):
    return std.collections.list.try_pop_or :: values, fallback :: call
