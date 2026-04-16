// `arcana_process.args` is runtime-owned host-core surface.
export fn count() -> Int:
    return arcana_process.args.count :: :: call

export fn get(index: Int) -> Str:
    return arcana_process.args.get :: index :: call
