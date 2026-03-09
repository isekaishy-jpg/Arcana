import std.memory
import std.io
use std.memory as memory
use std.io as io

record Node:
    value: Int

fn main() -> Int:
    let mut ast = memory.new[Node] :: 16 :: call

    let s1 = arena: ast :> value = 11 <: Node
    let s2 = arena: ast :> value = 22 <: Node
    let s3 = arena: ast :> value = 33 <: Node
    let s4 = arena: ast :> value = 44 <: Node
    let s5 = arena: ast :> value = 55 <: Node
    let s6 = arena: ast :> value = 66 <: Node
    let s7 = arena: ast :> value = 77 <: Node
    let s8 = arena: ast :> value = 88 <: Node
    let sf = arena: ast :> value = 462 <: Node

    "S1_BOOT_IO" :: :: io.print
    (ast :: s1 :: get).value :: :: io.print
    "S2_MOVE_BORROW" :: :: io.print
    (ast :: s2 :: get).value :: :: io.print
    "S3_ECS_TRAITS" :: :: io.print
    (ast :: s3 :: get).value :: :: io.print
    "S4_MEMORY_MIX" :: :: io.print
    (ast :: s4 :: get).value :: :: io.print
    "S5_CHAIN_SCORE" :: :: io.print
    (ast :: s5 :: get).value :: :: io.print
    "S6_CONCURRENCY_TELEMETRY" :: :: io.print
    (ast :: s6 :: get).value :: :: io.print
    "S7_STRESS_BURST" :: :: io.print
    (ast :: s7 :: get).value :: :: io.print
    "S8_FINAL" :: :: io.print
    (ast :: s8 :: get).value :: :: io.print
    "FINAL" :: :: io.print
    (ast :: sf :: get).value :: :: io.print
    return 0
