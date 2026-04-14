import arcana_winapi.helpers.process

export fn count() -> Int:
    return arcana_winapi.helpers.process.arg_count :: :: call

export fn get(index: Int) -> Str:
    return arcana_winapi.helpers.process.arg_get :: index :: call
