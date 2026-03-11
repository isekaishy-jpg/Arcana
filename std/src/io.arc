import std.kernel.io
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
    return std.kernel.io.stdin_read_line :: :: call
