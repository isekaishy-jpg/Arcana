import std.memory
import std.io
use std.memory as memory
use std.io as io

record Temp:
    value: Int

fn main() -> Int:
    let mut scratch = memory.frame_new[Temp] :: 8 :: call

    let a = frame: scratch :> value = 10 <: Temp
    let b = frame: scratch :> value = 20 <: Temp
    let before = scratch :: :: len
    before :: :: io.print
    let va = scratch :: a :: get
    let vb = scratch :: b :: get
    va.value :: :: io.print
    vb.value :: :: io.print

    scratch :: :: reset
    let after = scratch :: :: len
    after :: :: io.print

    let c = frame: scratch :> value = 7 <: Temp
    let vc = scratch :: c :: get
    vc.value :: :: io.print
    return 0

