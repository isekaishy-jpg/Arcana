import std.kernel.host

export fn has(name: Str) -> Bool:
    return std.kernel.host.env_has :: name :: call

export fn get(name: Str) -> Str:
    return std.kernel.host.env_get :: name :: call

export fn get_or(name: Str, fallback: Str) -> Str:
    if std.kernel.host.env_has :: name :: call:
        return std.kernel.host.env_get :: name :: call
    return fallback
