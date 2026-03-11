import std.io
use std.io as io
import std.concurrent
use std.concurrent as concurrent
async fn worker(n: Int) -> Int:
    1 :: :: std.concurrent.sleep
    return n + 1

async fn main() -> Int:
    let t = weave worker :: 41 :: call
    let x = t >> await
    x :: :: io.print
    return 0

