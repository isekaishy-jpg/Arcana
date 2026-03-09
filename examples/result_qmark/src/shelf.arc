import std.io
use std.io as io
lang result = Result

enum Result:
    Ok(Int)
    Err(Str)

fn plus_one_if_even(x: Int) -> Result:
    defer "leave-plus" :: :: io.print
    if x % 2 != 0:
        return Result.Err :: "odd" :: call
    return Result.Ok :: x + 1 :: call

fn demo() -> Result:
    defer "leave-demo" :: :: io.print
    let t = plus_one_if_even :: 4 :: call
    let a = t :: :: ?
    return Result.Ok :: a + 10 :: call

fn main() -> Int:
    let out = demo :: :: call
    let v = match out:
        Result.Ok(n) => n
        Result.Err(_) => 0
    v :: :: io.print
    return 0
