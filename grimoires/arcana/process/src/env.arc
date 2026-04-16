// `arcana_process.env` is runtime-owned host-core surface.
export fn has(name: Str) -> Bool:
    return arcana_process.env.has :: name :: call

export fn get(name: Str) -> Str:
    return arcana_process.env.get :: name :: call

export fn get_or(name: Str, fallback: Str) -> Str:
    if arcana_process.env.has :: name :: call:
        return arcana_process.env.get :: name :: call
    return fallback
