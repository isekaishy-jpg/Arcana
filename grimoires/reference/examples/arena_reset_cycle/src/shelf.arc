import std.memory
import std.io
use std.memory as memory
use std.io as io

record Item:
    value: Int

fn main() -> Int:
    let mut frame = memory.new[Item] :: 8 :: call
    let _first = arena: frame :> value = 1 <: Item
    let _second = arena: frame :> value = 2 <: Item
    let before = frame :: :: len
    before :: :: io.print
    frame :: :: reset
    let after = frame :: :: len
    after :: :: io.print
    let fresh = arena: frame :> value = 7 <: Item
    let again = frame :: :: len
    again :: :: io.print
    let current = frame :: fresh :: get
    current.value :: :: io.print
    return 0
