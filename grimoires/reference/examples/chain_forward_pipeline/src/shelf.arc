import std.io
use std.io as io

fn seed() -> Int:
    return 4

fn add_three(x: Int) -> Int:
    return x + 3

fn mul_two(x: Int) -> Int:
    return x * 2

fn emit(x: Int) -> Int:
    x :: :: io.print
    return x

fn main() -> Int:
    forward :=> seed => add_three => mul_two => emit
    forward :=< add_three <= mul_two <= emit <= seed
    lazy :=> seed => add_three => mul_two => emit
    return 0
