import planning
import std.collections.list
import std.collections.map
import std.io
use planning as tooling
use std.collections.list as list
use std.collections.map as map
use std.io as io

fn main() -> Int:
    let mut members = list.new[Str] :: :: call
    members :: "app" :: push
    members :: "core" :: push

    let mut deps = map.new[Str, List[Str]] :: :: call
    let mut app_deps = list.new[Str] :: :: call
    app_deps :: "core" :: push
    deps :: "app", app_deps :: set
    let empty = list.new[Str] :: :: call
    deps :: "core", empty :: set

    let _plan = tooling.plan_local_workspace :: members, deps :: call
    "planned" :: :: io.print
    return 0
