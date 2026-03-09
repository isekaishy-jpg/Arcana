import std.kernel.io

export fn print[T](read value: T):
    std.kernel.io.print[T] :: value :: call
