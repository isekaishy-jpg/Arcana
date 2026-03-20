import std.clipboard
import std.result
use std.result.Result

export fn read_text() -> Result[Str, Str]:
    return std.clipboard.read_text :: :: call

export fn write_text(text: Str) -> Result[Unit, Str]:
    return std.clipboard.write_text :: text :: call

export fn read_bytes() -> Result[Array[Int], Str]:
    return std.clipboard.read_bytes :: :: call

export fn write_bytes(read bytes: Array[Int]) -> Result[Unit, Str]:
    return std.clipboard.write_bytes :: bytes :: call
