import std.collections.list
import std.collections.map
import std.kernel.collections

export record Set[K]:
    entries: Map[K, Bool]

export fn new[K]() -> Set[K]:
    let entries = std.collections.map.new[K, Bool] :: :: call
    return std.collections.set.Set[K] :: entries = entries :: call

export fn len[K](read self: Set[K]) -> Int:
    return std.kernel.collections.map_len :: self.entries :: call

export fn has[K](read self: Set[K], key: K) -> Bool:
    return std.kernel.collections.map_has :: self.entries, key :: call

export fn insert[K](edit self: Set[K], key: K) -> Bool:
    let mut entries = self.entries
    if std.kernel.collections.map_has :: entries, key :: call:
        return false
    std.kernel.collections.map_set :: entries, key, true :: call
    self.entries = entries
    return true

export fn remove[K](edit self: Set[K], key: K) -> Bool:
    let mut entries = self.entries
    let removed = std.kernel.collections.map_remove :: entries, key :: call
    self.entries = entries
    return removed

export fn items[K](read self: Set[K]) -> List[K]:
    return std.collections.map.keys[K, Bool] :: self.entries :: call
