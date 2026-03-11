import std.kernel.io
import std.kernel.error
import std.result
use std.result.Result

export fn print[T](read value: T):
    std.kernel.io.print[T] :: value :: call

export fn print_line[T](read value: T):
    std.kernel.io.print[T] :: value :: call
    std.kernel.io.print[Str] :: "\n" :: call

export fn eprint[T](read value: T):
    std.kernel.io.eprint[T] :: value :: call

export fn eprint_line[T](read value: T):
    std.kernel.io.eprint[T] :: value :: call
    std.kernel.io.eprint[Str] :: "\n" :: call

export fn flush_stdout():
    std.kernel.io.flush_stdout :: :: call

export fn flush_stderr():
    std.kernel.io.flush_stderr :: :: call

export fn read_line() -> Result[Str, Str]:
    let pair = std.kernel.io.stdin_read_line_try :: :: call
    if pair.0:
        return Result.Ok[Str, Str] :: pair.1 :: call
    return Result.Err[Str, Str] :: (std.kernel.error.last_error_take :: :: call) :: call
