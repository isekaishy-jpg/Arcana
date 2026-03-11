import std.collections.list
import std.collections.map
import std.collections.set
import std.result
use std.collections.list as list
use std.collections.map as map
use std.collections.set as set
use std.result.Result

record GraphCtx:
    deps: Map[Str, List[Str]]
    members: std.collections.set.Set[Str]

record VisitState:
    marks: Map[Str, Int]
    out: List[Str]

record VisitOutcome:
    ok: Bool
    err: Str
    state: VisitState

fn unknown_member_err(name: Str) -> Str:
    return "unknown member `" + name + "`"

fn visit(name: Str, read ctx: GraphCtx, state: VisitState) -> VisitOutcome:
    let mut marks = state.marks
    let mut out = state.out
    let mark_pair = marks :: name, 0 :: try_get_or
    let mark = mark_pair.1
    if mark == 2:
        let next_state = planning.graph.VisitState :: marks = marks, out = out :: call
        return planning.graph.VisitOutcome :: ok = true, err = "", state = next_state :: call
    if mark == 1:
        let next_state = planning.graph.VisitState :: marks = marks, out = out :: call
        return planning.graph.VisitOutcome :: ok = false, err = "", state = next_state :: call
    marks :: name, 1 :: set
    let children_pair = ctx.deps :: name, (list.new[Str] :: :: call) :: try_get_or
    let children = children_pair.1
    for child in children:
        if not (set.has[Str] :: ctx.members, child :: call):
            let next_state = planning.graph.VisitState :: marks = marks, out = out :: call
            return planning.graph.VisitOutcome :: ok = false, err = (unknown_member_err :: child :: call), state = next_state :: call
        let child_state = planning.graph.VisitState :: marks = marks, out = out :: call
        let child_outcome = planning.graph.visit :: child, ctx, child_state :: call
        marks = child_outcome.state.marks
        out = child_outcome.state.out
        if not child_outcome.ok:
            let next_state = planning.graph.VisitState :: marks = marks, out = out :: call
            return planning.graph.VisitOutcome :: ok = false, err = child_outcome.err, state = next_state :: call
        let child_mark_pair = marks :: child, 0 :: try_get_or
        let child_mark = child_mark_pair.1
        if child_mark == 1:
            let next_state = planning.graph.VisitState :: marks = marks, out = out :: call
            return planning.graph.VisitOutcome :: ok = false, err = "", state = next_state :: call
    marks :: name, 2 :: set
    out :: name :: push
    let next_state = planning.graph.VisitState :: marks = marks, out = out :: call
    return planning.graph.VisitOutcome :: ok = true, err = "", state = next_state :: call

export fn topo_sort_status(read members: List[Str], read deps: Map[Str, List[Str]]) -> (Bool, (List[Str], Str)):
    let mut member_set = set.new[Str] :: :: call
    for name in members:
        member_set :: name :: insert
    let ctx = planning.graph.GraphCtx :: deps = deps, members = member_set :: call
    let mut marks = map.new[Str, Int] :: :: call
    let mut out = list.new[Str] :: :: call
    for name in members:
        let state = planning.graph.VisitState :: marks = marks, out = out :: call
        let outcome = planning.graph.visit :: name, ctx, state :: call
        marks = outcome.state.marks
        out = outcome.state.out
        if not outcome.ok:
            return (false, (out, outcome.err))
    return (true, (out, ""))

export fn topo_sort(read members: List[Str], read deps: Map[Str, List[Str]]) -> Result[List[Str], Str]:
    let status = planning.graph.topo_sort_status :: members, deps :: call
    if status.0:
        return Result.Ok[List[Str], Str] :: status.1.0 :: call
    let err = status.1.1
    if err == "":
        return Result.Err[List[Str], Str] :: "workspace dependency cycle detected" :: call
    return Result.Err[List[Str], Str] :: err :: call
