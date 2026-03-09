import std.args
import std.io
import arcana_compiler_core.fingerprint

fn main() -> Int:
    let mut target = "."
    let argc = std.args.count :: :: call
    if argc > 0:
        target = std.args.get :: 0 :: call
    let fp = arcana_compiler_core.fingerprint.member_source_fingerprint :: target :: call
    std.io.print[Str] :: fp :: call
    return 0
