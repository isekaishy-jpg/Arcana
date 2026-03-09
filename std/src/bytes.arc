import std.collections.array
import std.collections.list
import std.kernel.text
import std.result
use std.result.Result

export fn from_str_utf8(text: Str) -> Array[Int]:
    return std.kernel.text.bytes_from_str_utf8 :: text :: call

export fn to_str_utf8(read bytes: Array[Int]) -> Str:
    return std.kernel.text.bytes_to_str_utf8 :: bytes :: call

export fn len(read bytes: Array[Int]) -> Int:
    return std.kernel.text.bytes_len :: bytes :: call

export fn at(read bytes: Array[Int], index: Int) -> Int:
    return std.kernel.text.bytes_at :: bytes, index :: call

export fn slice(read bytes: Array[Int], start: Int, end: Int) -> Array[Int]:
    return std.kernel.text.bytes_slice :: bytes, start, end :: call

export fn validate_byte(value: Int) -> Bool:
    return value >= 0 and value <= 255

export fn new_buf() -> List[Int]:
    return std.collections.list.new[Int] :: :: call

export fn buf_push(edit buf: List[Int], value: Int) -> Result[Bool, Str]:
    if not (std.bytes.validate_byte :: value :: call):
        return Result.Err[Bool, Str] :: "byte value out of range 0..255" :: call
    buf :: value :: push
    return Result.Ok[Bool, Str] :: true :: call

export fn buf_extend(edit buf: List[Int], read bytes: Array[Int]) -> Result[Int, Str]:
    let mut added = 0
    for b in bytes:
        if not (std.bytes.validate_byte :: b :: call):
            return Result.Err[Int, Str] :: "byte value out of range 0..255" :: call
        buf :: b :: push
        added += 1
    return Result.Ok[Int, Str] :: added :: call

export fn buf_to_array(read buf: List[Int]) -> Array[Int]:
    let mut out = std.collections.list.new[Int] :: :: call
    for b in buf:
        out :: b :: push
    return std.collections.array.from_list[Int] :: out :: call
