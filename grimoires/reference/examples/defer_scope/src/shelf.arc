import std.io
use std.io as io
fn main() -> Int:
    defer io.print[Str] :: "main-1" :: call
    defer io.print[Str] :: "main-2" :: call

    let mut i = 0
    while i < 4:
        defer io.print[Str] :: "loop" :: call
        i += 1
        if i == 2:
            continue
        if i == 4:
            break
        io.print[Int] :: i :: call

    io.print[Int] :: 99 :: call
    return 0






