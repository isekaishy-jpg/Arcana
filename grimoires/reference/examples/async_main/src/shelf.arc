import std.io
use std.io as io
import std.concurrent
use std.concurrent as concurrent
async fn worker(n: Int) -> Int:
    1 :: :: std.concurrent.sleep
    return n + 1

async fn main() -> Int:
    let a = weave worker :: 40 :: call
    let b = weave worker :: 1 :: call
    (a >> await) :: :: io.print
    (b >> await) :: :: io.print
    return 0
