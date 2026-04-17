import arcana_process.fs
import std.result
use std.result.Result

fn close_stream(take stream: arcana_process.fs.FileStream) -> Result[Int, Str]:
    return Result.Ok[Int, Str] :: 0 :: call
-cleanup

fn main() -> Int:
    return 0
