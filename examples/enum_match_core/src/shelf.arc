import std.io
use std.io as io
enum Color:
    Red
    Green
    Blue(Int)

fn main() -> Int:
    let c = Color.Blue :: 7 :: call
    let n = match c:
        Color.Red => 1
        Color.Green => 2
        Color.Blue(v) => v
    n :: :: io.print
    return 0
