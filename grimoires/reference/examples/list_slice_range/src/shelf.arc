import std.io
use std.io as io
fn list_len_i(read xs: List[Int]) -> Int:
    let mut n = 0
    for _v in xs:
        n += 1
    return n

fn main() -> Int:
    let xs = [1, 2, 3, 4]
    let a = xs[1..3]
    let b = xs[..=1]
    let c = xs[2..]
    let d = xs[..]

    let na = list_len_i :: a :: call
    let nb = list_len_i :: b :: call
    let nc = list_len_i :: c :: call
    na :: :: io.print
    nb :: :: io.print
    nc :: :: io.print
    (d == xs) :: :: io.print

    let r1 = 0..3
    let r2 = 0..3
    let r3 = ..=3
    let r4 = ..=3
    (r1 == r2) :: :: io.print
    (r3 == r4) :: :: io.print
    return 0
