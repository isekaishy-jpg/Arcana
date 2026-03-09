import std.io
use std.io as io
fn map_len_i(read m: Map[Str, Int]) -> Int:
    let mut n = 0
    for _e in m:
        n += 1
    return n

fn map_has_i(read m: Map[Str, Int], key: Str) -> Bool:
    for e in m:
        if e.0 == key:
            return true
    return false

fn map_try_get_or_i(read m: Map[Str, Int], key: Str, fallback: Int) -> (Bool, Int):
    for e in m:
        if e.0 == key:
            return (true, e.1)
    return (false, fallback)

fn main() -> Int:
    let mut m = {"hp": 10, "mana": 4}
    let n = map_len_i :: m :: call
    n :: :: io.print
    let has_hp = map_has_i :: m, "hp" :: call
    has_hp :: :: io.print
    m["hp"] :: :: io.print
    m["hp"] += 5
    m["hp"] :: :: io.print
    let p = map_try_get_or_i :: m, "stam", 99 :: call
    p.0 :: :: io.print
    p.1 :: :: io.print
    return 0
