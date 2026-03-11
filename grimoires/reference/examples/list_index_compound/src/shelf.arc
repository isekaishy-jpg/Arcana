import std.io
use std.io as io
fn main() -> Int:
    let mut xs = [1, 2, 8]
    xs[0] += 4
    xs[1] <<= 1
    xs[2] shr= 1
    io.print[Int] :: xs[0] :: call
    io.print[Int] :: xs[1] :: call
    io.print[Int] :: xs[2] :: call

    let mut ss = ["Ar", "ca"]
    ss[0] += "c"
    ss[1] += "na"
    io.print[Str] :: ss[0] + ss[1] :: call
    return 0






