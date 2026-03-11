import std.kernel.env

export fn has(name: Str) -> Bool:
    return std.kernel.env.env_has :: name :: call

export fn get(name: Str) -> Str:
    return std.kernel.env.env_get :: name :: call

export fn get_or(name: Str, fallback: Str) -> Str:
    if std.kernel.env.env_has :: name :: call:
        return std.kernel.env.env_get :: name :: call
    return fallback
