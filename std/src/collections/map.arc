import std.kernel.collections

export fn new[K, V]() -> Map[K, V]:
    return std.kernel.collections.map_new[K, V] :: :: call

export fn items[K, V](read m: Map[K, V]) -> List[(K, V)]:
    let mut out = std.collections.list.new[(K, V)] :: :: call
    for pair in m:
        out :: pair :: push
    return out

export fn keys[K, V](read m: Map[K, V]) -> List[K]:
    let mut out = std.collections.list.new[K] :: :: call
    for pair in m:
        out :: pair.0 :: push
    return out

export fn values[K, V](read m: Map[K, V]) -> List[V]:
    let mut out = std.collections.list.new[V] :: :: call
    for pair in m:
        out :: pair.1 :: push
    return out

impl[K, V] Map[K, V]:
    fn len(read self: Map[K, V]) -> Int:
        return std.kernel.collections.map_len :: self :: call

    fn has(read self: Map[K, V], key: K) -> Bool:
        return std.kernel.collections.map_has :: self, key :: call

    fn get(read self: Map[K, V], key: K) -> V:
        return std.kernel.collections.map_get :: self, key :: call

    fn set(edit self: Map[K, V], key: K, take value: V):
        std.kernel.collections.map_set :: self, key, value :: call

    fn remove(edit self: Map[K, V], key: K) -> Bool:
        return std.kernel.collections.map_remove :: self, key :: call

    fn try_get_or(read self: Map[K, V], key: K, take fallback: V) -> (Bool, V):
        return std.kernel.collections.map_try_get_or :: self, key, fallback :: call
