import arcana_winapi.helpers.process

export fn has(name: Str) -> Bool:
    return arcana_winapi.helpers.process.env_has :: name :: call

export fn get(name: Str) -> Str:
    return arcana_winapi.helpers.process.env_get :: name :: call

export fn get_or(name: Str, fallback: Str) -> Str:
    if arcana_winapi.helpers.process.env_has :: name :: call:
        return arcana_winapi.helpers.process.env_get :: name :: call
    return fallback
