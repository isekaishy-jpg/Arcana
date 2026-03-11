import std.collections.array
import std.fs
import std.result
use std.result.Result

export fn read_bytes_or_empty(path: Str) -> Array[Int]:
    return match std.fs.read_bytes :: path :: call:
        Result.Ok(bytes) => bytes
        Result.Err(_) => std.collections.array.new[Int] :: 0, 0 :: call

export fn write_bytes_or_false(path: Str, read bytes: Array[Int]) -> Bool:
    return match std.fs.write_bytes :: path, bytes :: call:
        Result.Ok(ok) => ok
        Result.Err(_) => false
