import planning.graph
import std.result

export fn plan_local_workspace(read members: List[Str], read deps: Map[Str, List[Str]]) -> std.result.Result[List[Str], Str]:
    return planning.graph.topo_sort :: members, deps :: call

export fn plan_local_workspace_tuple(read members: List[Str], read deps: Map[Str, List[Str]]) -> (Bool, (List[Str], Str)):
    return planning.graph.topo_sort_status :: members, deps :: call
