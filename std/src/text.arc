import std.kernel.text
import std.result
use std.result.Result

export fn len_bytes(read text: Str) -> Int:
    return std.kernel.text.text_len_bytes :: text :: call

export fn byte_at(read text: Str, index: Int) -> Int:
    return std.kernel.text.text_byte_at :: text, index :: call

export fn slice_bytes(read text: Str, start: Int, end: Int) -> Str:
    return std.kernel.text.text_slice_bytes :: text, start, end :: call

export fn starts_with(read text: Str, read prefix: Str) -> Bool:
    return std.kernel.text.text_starts_with :: text, prefix :: call

export fn ends_with(read text: Str, read suffix: Str) -> Bool:
    return std.kernel.text.text_ends_with :: text, suffix :: call

export fn split_lines(read text: Str) -> List[Str]:
    return std.kernel.text.text_split_lines :: text :: call

export fn from_int(value: Int) -> Str:
    return std.kernel.text.text_from_int :: value :: call

export fn bytes_from_str_utf8(read text: Str) -> Bytes:
    return std.kernel.text.bytes_from_str_utf8 :: text :: call

export fn bytes_to_str_utf8(read bytes: Bytes) -> Str:
    return std.kernel.text.bytes_to_str_utf8 :: bytes :: call

export fn bytes_validate(value: Int) -> Bool:
    return value >= 0 and value <= 255

export fn bytes_builder() -> ByteBuffer:
    return std.text.byte_buffer_new :: :: call

export fn bytes_builder_push(edit buf: ByteBuffer, value: Int) -> Result[Unit, Str]:
    if not (std.text.bytes_validate :: value :: call):
        return Result.Err[Unit, Str] :: "byte value out of range 0..255" :: call
    buf :: value :: push
    return Result.Ok[Unit, Str] :: :: call

export fn bytes_builder_freeze(read buf: ByteBuffer) -> Bytes:
    return buf :: :: freeze

export fn find(read text: Str, start: Int, read needle: Str) -> Int:
    let total = std.text.len_bytes :: text :: call
    let needle_len = std.text.len_bytes :: needle :: call
    let mut i = start
    if i < 0:
        i = 0
    if needle_len == 0:
        if i > total:
            return total
        return i
    while i + needle_len <= total:
        if (std.text.slice_bytes :: text, i, i + needle_len :: call) == needle:
            return i
        i += 1
    return -1

export fn contains(read text: Str, read needle: Str) -> Bool:
    return (std.text.find :: text, 0, needle :: call) >= 0

export fn is_space_byte(b: Int) -> Bool:
    return b == 32 or b == 9 or b == 10 or b == 13

export fn is_digit_byte(b: Int) -> Bool:
    return b >= 48 and b <= 57

export fn is_alpha_byte(b: Int) -> Bool:
    return (b >= 65 and b <= 90) or (b >= 97 and b <= 122)

export fn is_ident_start_byte(b: Int) -> Bool:
    return (std.text.is_alpha_byte :: b :: call) or b == 95

export fn is_ident_continue_byte(b: Int) -> Bool:
    return (std.text.is_ident_start_byte :: b :: call) or (std.text.is_digit_byte :: b :: call)

export fn find_byte(read text: Str, start: Int, needle: Int) -> Int:
    let n = std.text.len_bytes :: text :: call
    let mut i = start
    while i < n:
        if (std.text.byte_at :: text, i :: call) == needle:
            return i
        i += 1
    return -1

export fn trim_start(read text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    while i < n and (std.text.is_space_byte :: (std.text.byte_at :: text, i :: call) :: call):
        i += 1
    return std.text.slice_bytes :: text, i, n :: call

export fn trim_end(read text: Str) -> Str:
    let mut end = std.text.len_bytes :: text :: call
    while end > 0 and (std.text.is_space_byte :: (std.text.byte_at :: text, end - 1 :: call) :: call):
        end -= 1
    return std.text.slice_bytes :: text, 0, end :: call

export fn trim(read text: Str) -> Str:
    return std.text.trim_end :: (std.text.trim_start :: text :: call) :: call

export fn split(read text: Str, read delim: Str) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let n = std.text.len_bytes :: text :: call
    let delim_len = std.text.len_bytes :: delim :: call
    if delim_len == 0:
        out :: (std.text.slice_bytes :: text, 0, n :: call) :: push
        return out
    let mut start = 0
    while start <= n:
        let next = std.text.find :: text, start, delim :: call
        if next < 0:
            out :: (std.text.slice_bytes :: text, start, n :: call) :: push
            return out
        out :: (std.text.slice_bytes :: text, start, next :: call) :: push
        start = next + delim_len
    return out

export fn join(read parts: List[Str], read delim: Str) -> Str:
    let mut out = ""
    let mut first = true
    for part in parts:
        if first:
            out = part
            first = false
        else:
            out = out + delim + part
    return out

impl Str:
    fn encode_utf8(read self: Str) -> Bytes:
        return std.kernel.text.bytes_from_str_utf8 :: self :: call

    fn encode_utf16(read self: Str) -> Utf16:
        return std.kernel.text.utf16_from_str :: self :: call

export fn bytes_len(read bytes: Bytes) -> Int:
    return std.kernel.text.bytes_len :: bytes :: call

export fn bytes_at(read bytes: Bytes, index: Int) -> Int:
    return std.kernel.text.bytes_at :: bytes, index :: call

export fn bytes_slice(read bytes: Bytes, start: Int, end: Int) -> Bytes:
    return std.kernel.text.bytes_slice :: bytes, start, end :: call

export fn bytes_sha256_hex(read bytes: Bytes) -> Str:
    return std.kernel.text.bytes_sha256_hex :: bytes :: call

export fn byte_buffer_new() -> ByteBuffer:
    return std.kernel.text.byte_buffer_new :: :: call

export fn utf16_len(read text: Utf16) -> Int:
    return std.kernel.text.utf16_len :: text :: call

export fn utf16_at(read text: Utf16, index: Int) -> U16:
    return std.kernel.text.utf16_at :: text, index :: call

export fn utf16_slice(read text: Utf16, start: Int, end: Int) -> Utf16:
    return std.kernel.text.utf16_slice :: text, start, end :: call

impl Bytes:
    fn decode_utf8(read self: Bytes) -> Result[Str, Str]:
        return Result.Ok[Str, Str] :: (std.kernel.text.bytes_to_str_utf8 :: self :: call) :: call

    fn thaw(read self: Bytes) -> ByteBuffer:
        return std.kernel.text.bytes_thaw :: self :: call

impl ByteBuffer:
    fn len(read self: ByteBuffer) -> Int:
        return std.kernel.text.byte_buffer_len :: self :: call

    fn at(read self: ByteBuffer, index: Int) -> Int:
        return Int :: (std.kernel.text.byte_buffer_at :: self, index :: call) :: call

    fn set(edit self: ByteBuffer, index: Int, value: Int):
        std.kernel.text.byte_buffer_set :: self, index, (U8 :: value :: call) :: call

    fn push(edit self: ByteBuffer, value: Int):
        std.kernel.text.byte_buffer_push :: self, (U8 :: value :: call) :: call

    fn freeze(read self: ByteBuffer) -> Bytes:
        return std.kernel.text.byte_buffer_freeze :: self :: call

impl Utf16:
    fn to_str(read self: Utf16) -> Result[Str, Str]:
        return std.kernel.text.utf16_to_str :: self :: call

    fn thaw(read self: Utf16) -> Utf16Buffer:
        return std.kernel.text.utf16_thaw :: self :: call

export fn utf16_buffer_new() -> Utf16Buffer:
    return std.kernel.text.utf16_buffer_new :: :: call

impl Utf16Buffer:
    fn len(read self: Utf16Buffer) -> Int:
        return std.kernel.text.utf16_buffer_len :: self :: call

    fn at(read self: Utf16Buffer, index: Int) -> U16:
        return std.kernel.text.utf16_buffer_at :: self, index :: call

    fn set(edit self: Utf16Buffer, index: Int, read value: U16):
        std.kernel.text.utf16_buffer_set :: self, index, value :: call

    fn push(edit self: Utf16Buffer, read value: U16):
        std.kernel.text.utf16_buffer_push :: self, value :: call

    fn freeze(read self: Utf16Buffer) -> Utf16:
        return std.kernel.text.utf16_buffer_freeze :: self :: call

export fn repeat(read text: Str, count: Int) -> Str:
    let mut out = ""
    let mut i = 0
    while i < count:
        out = out + text
        i += 1
    return out

export fn to_int(read text: Str) -> Result[Int, Str]:
    let value = std.text.trim :: text :: call
    let n = std.text.len_bytes :: value :: call
    if n == 0:
        return Result.Err[Int, Str] :: "expected integer text" :: call
    let mut sign = 1
    let mut i = 0
    let first = std.text.byte_at :: value, 0 :: call
    if first == 45:
        sign = -1
        i = 1
    else:
        if first == 43:
            i = 1
    if i >= n:
        return Result.Err[Int, Str] :: "expected integer digits" :: call
    let mut out = 0
    while i < n:
        let b = std.text.byte_at :: value, i :: call
        if not (std.text.is_digit_byte :: b :: call):
            return Result.Err[Int, Str] :: "invalid digit in integer text" :: call
        out = out * 10 + (b - 48)
        i += 1
    return Result.Ok[Int, Str] :: out * sign :: call
