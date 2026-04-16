import std.result
use std.result.Result

// `arcana_process.io` is runtime-owned host-core surface.
export fn print[T](read value: T):
    arcana_process.io.print[T] :: value :: call

export fn print_line[T](read value: T):
    arcana_process.io.print[T] :: value :: call
    arcana_process.io.print[Str] :: "\n" :: call

export fn eprint[T](read value: T):
    arcana_process.io.eprint[T] :: value :: call

export fn eprint_line[T](read value: T):
    arcana_process.io.eprint[T] :: value :: call
    arcana_process.io.eprint[Str] :: "\n" :: call

export fn flush_stdout():
    arcana_process.io.flush_stdout :: :: call

export fn flush_stderr():
    arcana_process.io.flush_stderr :: :: call

export fn read_line() -> Result[Str, Str]:
    return arcana_process.io.read_line :: :: call
