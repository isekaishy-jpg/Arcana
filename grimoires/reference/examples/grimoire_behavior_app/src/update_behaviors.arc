import jobs
use jobs.compute
import std.concurrent
use std.concurrent as concurrent
import std.io
use std.io as io

behavior[phase=update, affinity=worker] fn tick_job():
    let h = split compute :: 41 :: call
    (h :: :: join) :: :: io.print






