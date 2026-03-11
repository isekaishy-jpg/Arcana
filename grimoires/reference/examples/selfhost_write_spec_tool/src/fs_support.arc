import std.fs
import std.result
use std.result.Result

export fn read_text_or(path: Str, fallback: Str) -> Str:
    return match std.fs.read_text :: path :: call:
        Result.Ok(text) => text
        Result.Err(_) => fallback
