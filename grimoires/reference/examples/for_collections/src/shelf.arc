import std.io
use std.io as io
fn main() -> Int:
    for x in 0..5:
        if x == 1:
            continue
        io.print[Int] :: x :: call
        if x == 3:
            break

    let xs = [7, 8, 9]
    for v in xs:
        io.print[Int] :: v :: call

    let ys = [4, 4, 4]
    for v in ys:
        io.print[Int] :: v :: call

    let m = {"a": 1, "b": 2}
    for e in m:
        io.print[Str] :: e.0 :: call
        io.print[Int] :: e.1 :: call
    return 0






