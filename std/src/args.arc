import std.kernel.args

export fn count() -> Int:
    return std.kernel.args.arg_count :: :: call

export fn get(index: Int) -> Str:
    return std.kernel.args.arg_get :: index :: call
