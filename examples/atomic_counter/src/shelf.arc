import std.io
use std.io as io
import std.concurrent
use std.concurrent as concurrent
fn step(read a: AtomicInt) -> Int:
    return a :: 1 :: add

fn main() -> Int:
    let a = concurrent.atomic_int :: 0 :: call
    let h1 = split step :: a :: call
    let h2 = split step :: a :: call
    let p1 = h1 :: :: join
    let p2 = h2 :: :: join
    p1 :: :: io.print
    p2 :: :: io.print
    (a :: :: load) :: :: io.print
    return 0
