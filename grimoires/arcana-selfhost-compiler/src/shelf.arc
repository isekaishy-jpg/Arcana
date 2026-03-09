import std.args
import std.collections.list
import protocol
import types
import workspace

fn usage_error(message: Str) -> Int:
    let start = (1, 1)
    let end = (1, 1)
    let meta = ("ARC-SELFHOST-USAGE", "error")
    let loc = (".", start)
    let tail = (end, message)
    let diag = types.Diag :: meta = meta, loc = loc, tail = tail :: call
    protocol.emit_diag :: diag :: call
    let checksum = protocol.fold_checksum :: 0, (std.args.count :: :: call) :: call
    protocol.emit_final :: 1, 0, checksum :: call
    return 1

fn collect_extra(start: Int) -> List[Str]:
    let mut out = std.collections.list.new[Str] :: :: call
    let argc = std.args.count :: :: call
    let mut i = start
    while i < argc:
        let value = std.args.get :: i :: call
        out :: value :: push
        i += 1
    return out

fn main() -> Int:
    let argc = std.args.count :: :: call
    if argc < 2:
        return usage_error :: "usage: <mode> <target...>" :: call
    let mode = std.args.get :: 0 :: call
    if mode == "compile":
        if argc < 3:
            return usage_error :: "compile mode requires <target> <out>" :: call
        let source_path = std.args.get :: 1 :: call
        let out_path = std.args.get :: 2 :: call
        let extra = collect_extra :: 3 :: call
        return workspace.run_compile :: source_path, out_path, extra :: call
    if mode == "build":
        let workspace_dir = std.args.get :: 1 :: call
        let extra = collect_extra :: 2 :: call
        return workspace.run_build :: workspace_dir, extra :: call
    return usage_error :: ("unknown selfhost compile mode '" + mode + "'") :: call
