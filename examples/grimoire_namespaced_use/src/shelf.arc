import foo.id
import bar.id
import std.io
use std.io as io

fn main() -> Int:
    let a = 2 :: :: foo.id.id
    let b = 3 :: :: bar.id.id
    (a + b) :: :: io.print
    return 0
