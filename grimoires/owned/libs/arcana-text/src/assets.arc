import std.fs
import std.text
import std.result
use std.result.Result

export fn load_utf8(path: Str) -> Result[Str, Str]:
    return std.fs.read_text :: path :: call

export fn split_lines(text: Str) -> List[Str]:
    return std.text.split_lines :: text :: call
