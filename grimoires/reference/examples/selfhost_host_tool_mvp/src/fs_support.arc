import std.collections.list
import std.fs
import std.result
use std.result.Result

export fn read_text_or(path: Str, fallback: Str) -> Str:
    return match std.fs.read_text :: path :: call:
        Result.Ok(text) => text
        Result.Err(_) => fallback

export fn list_dir_or_empty(path: Str) -> List[Str]:
    return match std.fs.list_dir :: path :: call:
        Result.Ok(entries) => entries
        Result.Err(_) => std.collections.list.new[Str] :: :: call

export fn mkdir_all_or_false(path: Str) -> Bool:
    return match std.fs.mkdir_all :: path :: call:
        Result.Ok(ok) => ok
        Result.Err(_) => false

export fn write_text_or_false(path: Str, text: Str) -> Bool:
    return match std.fs.write_text :: path, text :: call:
        Result.Ok(ok) => ok
        Result.Err(_) => false
