import std.fs
import std.result
use std.result.Result

fn close_stream(take stream: std.fs.FileStream) -> Result[Int, Str]:
    return Result.Ok[Int, Str] :: 0 :: call
-cleanup

fn main() -> Int:
    return 0
