import std.memory
import std.io
use std.memory as memory
use std.io as io

record Item:
    value: Int

fn main() -> Int:
    let mut a = memory.new[Item] :: 4 :: call
    let id = arena: a :> value = 99 <: Item
    a :: :: reset
    "about to trigger stale arena id error" :: :: io.print
    let _x = a :: id :: get
    return 0
