import std.memory
import std.io
use std.memory as memory
use std.io as io

record Entity:
    hp: Int

fn main() -> Int:
    let mut pool_store = memory.pool_new[Entity] :: 8 :: call

    let e1 = pool: pool_store :> hp = 100 <: Entity
    pool_store :: e1 :: remove

    let e2 = pool: pool_store :> hp = 250 <: Entity
    let alive = pool_store :: e2 :: get
    alive.hp :: :: io.print

    let live = pool_store :: :: len
    live :: :: io.print

    pool_store :: :: reset
    let after = pool_store :: :: len
    after :: :: io.print
    return 0

