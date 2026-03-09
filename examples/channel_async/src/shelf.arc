import std.io
use std.io as io
import std.concurrent
use std.concurrent as concurrent
async fn sender(read ch: Channel[Int]) -> Int:
    1 :: :: std.concurrent.sleep
    ch :: 7 :: send
    return 0

async fn receiver(read ch: Channel[Int]) -> Int:
    return ch :: :: recv

async fn main() -> Int:
    let ch = concurrent.channel[Int] :: 1 :: call
    let ts = weave sender :: ch :: call
    let tr = weave receiver :: ch :: call
    (tr >> await) :: :: io.print
    (ts >> await) :: :: io.print
    return 0
