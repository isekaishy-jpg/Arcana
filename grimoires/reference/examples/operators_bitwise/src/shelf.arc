import std.io
use std.io as io
fn main() -> Int:
    let flags = 5 | 8
    let masked = flags & 7
    let flipped = ~masked
    let hi = 1 << 10
    let lo = hi shr 3
    let mix = (flags ^ 3) & 15
    let precedence = 1 + 2 << 3
    let signed = -8 shr 1

    io.print[Int] :: flags :: call
    io.print[Int] :: masked :: call
    io.print[Int] :: flipped :: call
    io.print[Int] :: hi :: call
    io.print[Int] :: lo :: call
    io.print[Int] :: mix :: call
    io.print[Int] :: precedence :: call
    io.print[Int] :: signed :: call

    // Runtime error example (commented): shift count out of range
    // io.print[Int](1 << 64)

    return 0






