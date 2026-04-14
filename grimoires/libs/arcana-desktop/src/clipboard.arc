import arcana_winapi.helpers.clipboard
import std.result
use std.result.Result

export fn read_text() -> Result[Str, Str]:
    return arcana_winapi.helpers.clipboard.read_text :: :: call

export fn write_text(text: Str) -> Result[Unit, Str]:
    return arcana_winapi.helpers.clipboard.write_text :: text :: call

export fn read_bytes() -> Result[Bytes, Str]:
    return arcana_winapi.helpers.clipboard.read_bytes :: :: call

export fn write_bytes(read bytes: Bytes) -> Result[Unit, Str]:
    return arcana_winapi.helpers.clipboard.write_bytes :: bytes :: call
