import std.io
use std.io as io
import std.concurrent
use std.concurrent as concurrent
fn sender(read ch: Channel[Int]) -> Int:
    ch :: 42 :: send
    return 0

fn main() -> Int:
    let ch = concurrent.channel[Int] :: 1 :: call
    let h = split sender :: ch :: call
    let v = ch :: :: recv
    v :: :: io.print
    let code = h :: :: join
    code :: :: io.print
    return 0
