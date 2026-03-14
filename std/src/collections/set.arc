import std.collections.list
import std.collections.map
import std.kernel.collections

export record Set[K]:
    entries: Map[K, Bool]

export fn new[K]() -> Set[K]:
    let entries = std.collections.map.new[K, Bool] :: :: call
    return std.collections.set.Set[K] :: entries = entries :: call

impl[K] Set[K]:
    fn len(read self: Set[K]) -> Int:
        return std.kernel.collections.map_len :: self.entries :: call

    fn is_empty(read self: Set[K]) -> Bool:
        return (self :: :: len) == 0

    fn has(read self: Set[K], key: K) -> Bool:
        return std.kernel.collections.map_has :: self.entries, key :: call

    fn insert(edit self: Set[K], key: K) -> Bool:
        let mut entries = self.entries
        if std.kernel.collections.map_has :: entries, key :: call:
            return false
        std.kernel.collections.map_set :: entries, key, true :: call
        self.entries = entries
        return true

    fn remove(edit self: Set[K], key: K) -> Bool:
        let mut entries = self.entries
        let removed = std.kernel.collections.map_remove :: entries, key :: call
        self.entries = entries
        return removed

    fn items(read self: Set[K]) -> List[K]:
        return std.collections.map.keys[K, Bool] :: self.entries :: call

export fn len[K](read set: Set[K]) -> Int:
    return set :: :: len

export fn is_empty[K](read set: Set[K]) -> Bool:
    return set :: :: is_empty

export fn has[K](read set: Set[K], key: K) -> Bool:
    return set :: key :: has

export fn insert[K](edit set: Set[K], key: K) -> Bool:
    return set :: key :: insert

export fn remove[K](edit set: Set[K], key: K) -> Bool:
    return set :: key :: remove

export fn items[K](read set: Set[K]) -> List[K]:
    return set :: :: items
