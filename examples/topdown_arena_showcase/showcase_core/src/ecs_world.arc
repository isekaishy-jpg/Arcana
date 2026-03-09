import std.ecs

use std.ecs as ecs

export fn tick(seed: Int) -> Int:
    ecs.set_component[Int] :: seed :: call

    let e = ecs.spawn :: :: call
    ecs.set_component_at[Int] :: e, (seed + 3) :: call

    let mut out = 0
    if ecs.has_component_at[Int] :: e :: call:
        out += ecs.get_component_at[Int] :: e :: call

    if ecs.has_component[Int] :: :: call:
        out += ecs.get_component[Int] :: :: call

    ecs.remove_component_at[Int] :: e :: call
    ecs.despawn :: e :: call
    return out
