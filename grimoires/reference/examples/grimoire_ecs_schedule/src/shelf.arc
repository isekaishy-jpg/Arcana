import std.ecs
use std.ecs as ecs
use std.ecs.step_startup
use std.ecs.step_fixed_update
use std.ecs.step_update
use std.ecs.step_render
import std.io
use std.io as io

record Position:
    x: Int
    y: Int

system[phase=startup, affinity=main] fn boot():
    let p = Position :: x = 0, y = 0 :: call
    ecs.set_component[Position] :: p :: call

system[phase=fixed_update, affinity=main] fn sim(edit p: Position):
    p.x += 1
    p.y += 2

system[phase=render, affinity=main] fn draw():
    let p = ecs.get_component[Position] :: :: call
    p.x :: :: io.print
    p.y :: :: io.print

fn main() -> Int:
    (step_startup :: :: call) :: :: io.print
    (step_fixed_update :: :: call) :: :: io.print
    (step_update :: :: call) :: :: io.print
    (step_render :: :: call) :: :: io.print
    return 0
