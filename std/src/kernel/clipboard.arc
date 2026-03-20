import std.result
use std.result.Result

intrinsic fn read_text() -> Result[Str, Str] = ClipboardReadTextTry
intrinsic fn write_text(text: Str) -> Result[Unit, Str] = ClipboardWriteTextTry
intrinsic fn read_bytes() -> Result[Array[Int], Str] = ClipboardReadBytesTry
intrinsic fn write_bytes(bytes: Array[Int]) -> Result[Unit, Str] = ClipboardWriteBytesTry
