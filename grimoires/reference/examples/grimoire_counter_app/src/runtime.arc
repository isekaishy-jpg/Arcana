import types
use types.Counter
import std.io
use std.io as io

export fn run_counter(edit c: Counter):
    while c.value < c.limit:
        io.print[Int] :: c.value :: call
        c.value = c.value + 1





