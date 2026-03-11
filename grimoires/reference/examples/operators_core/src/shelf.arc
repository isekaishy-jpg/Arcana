import std.io
use std.io as io
import std.concurrent
use std.concurrent as concurrent
fn side_effect(read counter: AtomicInt) -> Bool:
    "side" :: :: io.print
    counter :: 1 :: add
    return true

fn main() -> Int:
    let counter = concurrent.atomic_int :: 0 :: call
    let x = -3
    let paused = false

    (x % 2) :: :: io.print
    (not paused) :: :: io.print
    (x != 0) :: :: io.print
    (x <= 0) :: :: io.print
    (x >= -3) :: :: io.print

    if false and (side_effect :: counter :: call):
        999 :: :: io.print

    if true or (side_effect :: counter :: call):
        111 :: :: io.print

    if true and (side_effect :: counter :: call):
        222 :: :: io.print

    if false or (side_effect :: counter :: call):
        333 :: :: io.print

    (counter :: :: load) :: :: io.print
    return 0
