import std.kernel.host

export fn count() -> Int:
    return std.kernel.host.arg_count :: :: call

export fn get(index: Int) -> Str:
    return std.kernel.host.arg_get :: index :: call
