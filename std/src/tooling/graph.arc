import std.collections.list
import std.collections.map
import std.collections.set
import std.result
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
    return "workspace dependency references unknown member: " + name

fn visit(read name: Str, read ctx: GraphCtx, take state: VisitState) -> VisitOutcome:
    let mut marks = state.marks
    let mut out = state.out
    if marks :: name :: has:
        let mark = marks :: name :: get
        if mark == 2:
            let next_state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
            return std.tooling.graph.VisitOutcome :: ok = true, err = "", state = next_state :: call
        if mark == 1:
            let next_state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
            return std.tooling.graph.VisitOutcome :: ok = false, err = "", state = next_state :: call

    marks :: name, 1 :: set
    let pair = ctx.deps :: name, std.collections.list.new[Str] :: :: call :: try_get_or
    let mut children = pair.1
    for child in children:
        if not (std.collections.set.has[Str] :: ctx.members, child :: call):
            let next_state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
            return std.tooling.graph.VisitOutcome :: ok = false, err = (unknown_member_err :: child :: call), state = next_state :: call
        let child_state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
        let child_outcome = std.tooling.graph.visit :: child, ctx, child_state :: call
        marks = child_outcome.state.marks
        out = child_outcome.state.out
        if child_outcome.err != "":
            let next_state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
            return std.tooling.graph.VisitOutcome :: ok = false, err = child_outcome.err, state = next_state :: call
        if not child_outcome.ok:
            let next_state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
            return std.tooling.graph.VisitOutcome :: ok = false, err = "", state = next_state :: call

    marks :: name, 2 :: set
    out :: name :: push
    let next_state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
    return std.tooling.graph.VisitOutcome :: ok = true, err = "", state = next_state :: call

export fn topo_sort_status(read members: List[Str], read deps: Map[Str, List[Str]]) -> (Bool, (List[Str], Str)):
    let mut marks = std.collections.map.new[Str, Int] :: :: call
    let mut out = std.collections.list.new[Str] :: :: call
    let mut member_set = std.collections.set.new[Str] :: :: call
    for name in members:
        std.collections.set.insert[Str] :: member_set, name :: call

    let ctx = std.tooling.graph.GraphCtx :: deps = deps, members = member_set :: call

    for name in members:
        let state = std.tooling.graph.VisitState :: marks = marks, out = out :: call
        let outcome = std.tooling.graph.visit :: name, ctx, state :: call
        marks = outcome.state.marks
        out = outcome.state.out
        if outcome.err != "":
            return (false, (std.collections.list.new[Str] :: :: call, outcome.err))
        if not outcome.ok:
            return (false, (std.collections.list.new[Str] :: :: call, "workspace dependency cycle detected"))

    return (true, (out, ""))

export fn topo_sort(read members: List[Str], read deps: Map[Str, List[Str]]) -> std.result.Result[List[Str], Str]:
    let status = std.tooling.graph.topo_sort_status :: members, deps :: call
    if status.0:
        return Result.Ok[List[Str], Str] :: status.1.0 :: call
    return Result.Err[List[Str], Str] :: status.1.1 :: call
