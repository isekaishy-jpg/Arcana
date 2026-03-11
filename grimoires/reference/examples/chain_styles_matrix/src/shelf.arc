import std.io
use std.io as io

fn seed() -> Int:
    return 3

fn inc(x: Int) -> Int:
    return x + 1

fn dec(x: Int) -> Int:
    return x - 1

fn emit(x: Int) -> Int:
    x :: :: io.print
    return x

fn main() -> Int:
    forward :=> seed => inc => emit
    forward :=< inc <= emit <= seed
    forward :=> seed => inc <= dec <= emit
    lazy :=> seed => inc => emit
    parallel :=> seed => inc => dec => emit
    broadcast :=> seed => inc => dec => emit
    collect :=> seed => inc => dec => emit
    async :=> seed => inc => emit
    plan :=> seed => inc => emit
    return 0
