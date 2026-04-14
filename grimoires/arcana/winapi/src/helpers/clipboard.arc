import std.result
use std.result.Result

native fn read_text_raw() -> Str = helpers.clipboard.read_text_raw
native fn write_text_raw(text: Str) -> Bool = helpers.clipboard.write_text_raw
native fn read_bytes_raw() -> Bytes = helpers.clipboard.read_bytes_raw
native fn write_bytes_raw(read bytes: Bytes) -> Bool = helpers.clipboard.write_bytes_raw
native fn take_last_error() -> Str = helpers.clipboard.take_last_error

export fn read_text() -> Result[Str, Str]:
    let text = arcana_winapi.helpers.clipboard.read_text_raw :: :: call
    let err = arcana_winapi.helpers.clipboard.take_last_error :: :: call
    if err != "":
        return Result.Err[Str, Str] :: err :: call
    return Result.Ok[Str, Str] :: text :: call

export fn write_text(text: Str) -> Result[Unit, Str]:
    let _ = arcana_winapi.helpers.clipboard.write_text_raw :: text :: call
    let err = arcana_winapi.helpers.clipboard.take_last_error :: :: call
    if err != "":
        return Result.Err[Unit, Str] :: err :: call
    return Result.Ok[Unit, Str] :: :: call

export fn read_bytes() -> Result[Bytes, Str]:
    let bytes = arcana_winapi.helpers.clipboard.read_bytes_raw :: :: call
    let err = arcana_winapi.helpers.clipboard.take_last_error :: :: call
    if err != "":
        return Result.Err[Bytes, Str] :: err :: call
    return Result.Ok[Bytes, Str] :: bytes :: call

export fn write_bytes(read bytes: Bytes) -> Result[Unit, Str]:
    let _ = arcana_winapi.helpers.clipboard.write_bytes_raw :: bytes :: call
    let err = arcana_winapi.helpers.clipboard.take_last_error :: :: call
    if err != "":
        return Result.Err[Unit, Str] :: err :: call
    return Result.Ok[Unit, Str] :: :: call
