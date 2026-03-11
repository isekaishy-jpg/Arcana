import std.io
use std.io as io
import std.behaviors
use std.behaviors as behaviors
import std.behavior_traits
use std.behavior_traits as phase
import std.types

record PhaseCounter:
    tick: std.types.core.Tick

impl std.behavior_traits.UpdatePhase[PhaseCounter] for PhaseCounter:
    fn on_update(edit self: PhaseCounter) -> Int:
        self.tick.value = self.tick.value + 1
        return self.tick.value

#stage[pure=true, deterministic=true, thread=worker, authority=local, rollback_safe=true]
fn tick_seed() -> Str:
    return "tick"

#stage[pure=true, deterministic=true, thread=worker, authority=local, rollback_safe=true]
fn tick_suffix(v: Str) -> Str:
    return v + "_phase"

behavior[phase=update, affinity=worker] fn tick():
    #chain[phase=update, deterministic=true, thread=worker, authority=local, rollback_safe=true]
    forward :=> tick_seed => tick_suffix
    "tick" :: :: io.print

#stage[pure=true, deterministic=true, thread=main, authority=local, rollback_safe=true]
fn draw_seed() -> Str:
    return "draw"

#stage[pure=true, deterministic=true, thread=main, authority=local, rollback_safe=true]
fn draw_suffix(v: Str) -> Str:
    return v + "_phase"

behavior[phase=render, affinity=main] fn draw():
    #chain[phase=render, deterministic=true, thread=main, authority=local, rollback_safe=true]
    forward :=> draw_seed => draw_suffix
    "draw" :: :: io.print

fn main() -> Int:
    let mut counter = PhaseCounter :: tick = (std.types.core.Tick :: value = 0 :: call) :: call
    let p1 = phase.run_update[PhaseCounter] :: counter :: call
    p1 :: :: io.print
    let u = behaviors.step :: "update" :: call
    let r = behaviors.step :: "render" :: call
    u :: :: io.print
    r :: :: io.print
    return 0
