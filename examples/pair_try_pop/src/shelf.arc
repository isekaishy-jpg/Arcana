import std.io
use std.io as io
fn find_or(read xs: List[Int], want: Int, fallback: Int) -> (Bool, Int):
    for v in xs:
        if v == want:
            return (true, v)
    return (false, fallback)

fn main() -> Int:
    let xs = [7, 8, 9]
    let p = find_or :: xs, 8, -1 :: call
    p.0 :: :: io.print
    p.1 :: :: io.print

    let q = find_or :: xs, 42, -1 :: call
    q.0 :: :: io.print
    q.1 :: :: io.print
    return 0
