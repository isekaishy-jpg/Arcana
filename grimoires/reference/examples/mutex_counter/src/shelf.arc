import std.io
use std.io as io
import std.concurrent
use std.concurrent as concurrent
record Counter:
    n: Int

fn bump(read m: Mutex[Counter]) -> Int:
    let mut c = m :: :: pull
    c.n = c.n + 1
    let out = c.n
    m :: c :: put
    return out

fn main() -> Int:
    let seed = Counter :: n = 0 :: call
    let m = concurrent.mutex[Counter] :: seed :: call
    let h1 = split bump :: m :: call
    let h2 = split bump :: m :: call
    let r1 = h1 :: :: join
    let r2 = h2 :: :: join
    r1 :: :: io.print
    r2 :: :: io.print
    let c = m :: :: pull
    c.n :: :: io.print
    return 0
