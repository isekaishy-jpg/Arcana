import std.result
use std.result.Result

intrinsic fn text_len_bytes(text: Str) -> Int = HostTextLenBytes
intrinsic fn text_byte_at(text: Str, index: Int) -> Int = HostTextByteAt
intrinsic fn text_slice_bytes(text: Str, start: Int, end: Int) -> Str = HostTextSliceBytes
intrinsic fn text_starts_with(text: Str, prefix: Str) -> Bool = HostTextStartsWith
intrinsic fn text_ends_with(text: Str, suffix: Str) -> Bool = HostTextEndsWith
intrinsic fn text_split_lines(text: Str) -> List[Str] = HostTextSplitLines
intrinsic fn text_from_int(value: Int) -> Str = HostTextFromInt

intrinsic fn bytes_from_str_utf8(text: Str) -> Bytes = HostBytesFromStrUtf8
intrinsic fn bytes_to_str_utf8(read bytes: Bytes) -> Str = HostBytesToStrUtf8
intrinsic fn bytes_len(read bytes: Bytes) -> Int = HostBytesLen
intrinsic fn bytes_at(read bytes: Bytes, index: Int) -> Int = HostBytesAt
intrinsic fn bytes_slice(read bytes: Bytes, start: Int, end: Int) -> Bytes = HostBytesSlice
intrinsic fn bytes_sha256_hex(read bytes: Bytes) -> Str = HostBytesSha256Hex
intrinsic fn bytes_thaw(read bytes: Bytes) -> ByteBuffer = HostBytesThaw
intrinsic fn byte_buffer_new() -> ByteBuffer = HostByteBufferNew
intrinsic fn byte_buffer_len(read buf: ByteBuffer) -> Int = HostByteBufferLen
intrinsic fn byte_buffer_at(read buf: ByteBuffer, index: Int) -> U8 = HostByteBufferAt
intrinsic fn byte_buffer_set(edit buf: ByteBuffer, index: Int, value: U8) = HostByteBufferSet
intrinsic fn byte_buffer_push(edit buf: ByteBuffer, value: U8) = HostByteBufferPush
intrinsic fn byte_buffer_freeze(read buf: ByteBuffer) -> Bytes = HostByteBufferFreeze

intrinsic fn utf16_from_str(text: Str) -> Utf16 = HostUtf16FromStr
intrinsic fn utf16_to_str(read text: Utf16) -> Result[Str, Str] = HostUtf16ToStr
intrinsic fn utf16_len(read text: Utf16) -> Int = HostUtf16Len
intrinsic fn utf16_at(read text: Utf16, index: Int) -> U16 = HostUtf16At
intrinsic fn utf16_slice(read text: Utf16, start: Int, end: Int) -> Utf16 = HostUtf16Slice
intrinsic fn utf16_thaw(read text: Utf16) -> Utf16Buffer = HostUtf16Thaw
intrinsic fn utf16_buffer_new() -> Utf16Buffer = HostUtf16BufferNew
intrinsic fn utf16_buffer_len(read buf: Utf16Buffer) -> Int = HostUtf16BufferLen
intrinsic fn utf16_buffer_at(read buf: Utf16Buffer, index: Int) -> U16 = HostUtf16BufferAt
intrinsic fn utf16_buffer_set(edit buf: Utf16Buffer, index: Int, value: U16) = HostUtf16BufferSet
intrinsic fn utf16_buffer_push(edit buf: Utf16Buffer, value: U16) = HostUtf16BufferPush
intrinsic fn utf16_buffer_freeze(read buf: Utf16Buffer) -> Utf16 = HostUtf16BufferFreeze
