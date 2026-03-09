import std.ecs
import std.iter
use std.ecs as ecs

system[phase=startup, affinity=main] fn boot():
    ecs.set_component[Int] :: 7 :: call

fn main() -> Int:
    ecs.step_startup :: :: call

    let mut r = std.iter.range :: 0, 5 :: call
    let counted = std.iter.count[std.iter.RangeIter] :: r :: call

    let mut cur = ecs.singleton_int_cursor :: :: call
    let present = std.iter.count[std.ecs.SingletonIntCursor] :: cur :: call

    if present == 1:
        return counted
    return 0
