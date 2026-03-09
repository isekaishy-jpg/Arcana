import std.io
use std.io as io

fn cleanup(value: Int):
    io.print[Int] :: value :: call

fn run(seed: Int) -> Int:
    let local = seed
    while local > 0:
        let scratch = local
        local -= 1
    [scratch, cleanup]#cleanup
    return local
[seed, cleanup]#cleanup

fn main() -> Int:
    return run :: 3 :: call
