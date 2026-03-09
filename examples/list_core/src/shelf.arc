import std.io
use std.io as io
fn list_len_i(read xs: List[Int]) -> Int:
    let mut n = 0
    for _v in xs:
        n += 1
    return n

fn main() -> Int:
    let xs = [1, 2, 3]
    let n0 = list_len_i :: xs :: call
    n0 :: :: io.print
    xs[0] :: :: io.print
    let tail = xs[1..]
    let n1 = list_len_i :: tail :: call
    n1 :: :: io.print
    tail[0] :: :: io.print
    return 0
