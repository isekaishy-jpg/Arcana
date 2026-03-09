import std.io
import std.memory
use std.io as io
use std.memory as memory

record Node:
    value: Int

fn make_node(v: Int) -> Node:
    return Node :: value = v :: call

fn show_node(n: Node) -> Node:
    n.value :: :: io.print
    return n

fn bump_node(n: Node) -> Node:
    return Node :: value = n.value + 1 :: call

fn touch_id(id: ArenaId[Node]) -> ArenaId[Node]:
    return id

fn main() -> Int:
    let mut arena_nodes = memory.new[Node] :: 8 :: call

    Node :: :: call
        value = 10
        forward :=> show_node => bump_node => show_node

    arena: arena_nodes :> 21 <: make_node
        plan :=> touch_id
        forward :=> touch_id

    return 0
