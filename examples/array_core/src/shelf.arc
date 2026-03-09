import std.io
use std.io as io
fn list_len_i(read xs: List[Int]) -> Int:
    let mut n = 0
    for _v in xs:
        n += 1
    return n

fn main() -> Int:
    let mut xs = [2, 2, 2, 2]
    let n0 = list_len_i :: xs :: call
    n0 :: :: io.print
    xs[1] = 9
    xs[2] += 3
    let b = xs[1..=2]
    let n1 = list_len_i :: b :: call
    n1 :: :: io.print
    xs[1] :: :: io.print
    xs[2] :: :: io.print
    return 0
