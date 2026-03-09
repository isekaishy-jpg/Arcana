import std.memory
import std.io
use std.memory as memory
use std.io as io

record Node:
    lhs: Int
    rhs: Int
    op: Int

fn main() -> Int:
    let mut ast = memory.new[Node] :: 16 :: call
    let root = arena: ast :> lhs = 10, rhs = 32, op = 43 <: Node
    let node = ast :: root :: get
    node.lhs :: :: io.print
    node.rhs :: :: io.print
    node.op :: :: io.print
    let count = ast :: :: len
    count :: :: io.print
    return 0
