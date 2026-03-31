import core.types

record Widget:
    value: Int
    maybe: Option[Int]

enum Option[T]:
    Some(T)
    None

enum Result[T, E]:
    Ok(T)
    Err(E)

enum Packet:
    Data(Int)

obj Counter:
    value: Int

create Session [Counter] scope-exit:
    done: when false hold [Counter]

fn main() -> Int:
    Memory frame:scratch -alloc
        capacity = 2
        pressure = bounded
    let mut refined = 10
    bind -return 0
        let value = Result.Ok[Int, Str] :: 1 :: call
        let fallback = Option.None[Int] :: :: call -default 2
        refined = Option.None[Int] :: :: call -preserve
        refined = Option.None[Int] :: :: call -replace 3
    let mut sentinel = 0
    while true:
        sentinel = sentinel + 1
        bind -continue
            require sentinel != 2
        bind -break
            require sentinel < 3
    if true:
        let active = Session :: :: call
        recycle -done
            false
    let built = construct yield Widget -return 0
        value = Result.Ok[Int, Str] :: (value + fallback + refined) :: call
        maybe = Option.None[Int] :: :: call
    construct deliver Widget -> delivered -return 0
        value = Result.Ok[Int, Str] :: (built.value + 1) :: call
        maybe = Option.Some[Int] :: fallback :: call
    let mut placed = delivered
    construct place Widget -> placed -return 0
        value = Result.Ok[Int, Str] :: (delivered.value + refined) :: call
        maybe = Option.None[Int] :: :: call
    let _cached = arena: core.types.cache :> value = placed.value <: core.types.Item
    let _scratch = frame: scratch :> value = placed.value <: core.types.Item
    construct deliver Packet.Data -> packet -return 0
        payload = Result.Ok[Int, Str] :: placed.value :: call
    let mut placed_packet = packet
    construct place Packet.Data -> placed_packet -return 0
        payload = Result.Ok[Int, Str] :: (placed.value + delivered.value) :: call
    let total = match placed_packet:
        Packet.Data(payload) => payload
    return total
