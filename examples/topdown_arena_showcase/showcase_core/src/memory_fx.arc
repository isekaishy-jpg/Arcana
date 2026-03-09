import std.memory

use std.memory as memory

fn id_int(v: Int) -> Int:
    return v

export fn probe(seed: Int) -> Int:
    let mut arena_store = memory.new[Int] :: 8 :: call
    let mut frame_store = memory.frame_new[Int] :: 8 :: call
    let mut pool_store = memory.pool_new[Int] :: 8 :: call

    let aid = arena: arena_store :> seed <: id_int
    let fid = frame: frame_store :> seed + 1 <: id_int
    let pid = pool: pool_store :> seed + 2 <: id_int

    let mut out = arena_store :: aid :: get
    out += frame_store :: fid :: get
    out += pool_store :: pid :: get

    pool_store :: pid :: remove
    frame_store :: :: reset
    return out
