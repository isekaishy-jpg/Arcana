import std.memory
import std.io
use std.memory as memory
use std.io as io

record Node:
    value: Int

fn main() -> Int:
    let mut ast = memory.new[Node] :: 8 :: call
    let mut frame_nodes = memory.frame_new[Node] :: 8 :: call
    let mut pool_nodes = memory.pool_new[Node] :: 8 :: call

    let aid = arena: ast :> value = 1 <: Node
    let fid = frame: frame_nodes :> value = 2 <: Node
    let pid = pool: pool_nodes :> value = 3 <: Node

    let av = ast :: aid :: get
    let fv = frame_nodes :: fid :: get
    let pv = pool_nodes :: pid :: get

    av.value :: :: io.print
    fv.value :: :: io.print
    pv.value :: :: io.print
    return 0

