import std.result
import std.option
import std.collections.list
import std.collections.map
import std.collections.array
use std.result.Result
use std.collections.map as map

fn compute_score() -> Result[Int, Str]:
    let mut xs = [3, 4]
    xs :: 8 :: push
    let pair = xs :: 0 :: try_pop_or
    if pair.0:
        let mut m = map.new[Str, Int] :: :: call
        let len = xs :: :: len
        let score = pair.1 + len
        m :: "score", score :: set
        if m :: "score" :: has:
            let got = m :: "score" :: get
            return Result.Ok[Int, Str] :: got :: call
    return Result.Err[Int, Str] :: "missing" :: call

fn main() -> Int:
    let out = compute_score :: :: call
    return match out:
        Result.Ok(v) => v
        Result.Err(_) => 0
