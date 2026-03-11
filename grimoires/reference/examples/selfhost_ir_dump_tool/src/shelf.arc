import std.args
import std.io
import arcana_compiler_core.ir_dump

fn main() -> Int:
    if (std.args.count :: :: call) <= 0:
        std.io.print[Str] :: "usage: selfhost_ir_dump_tool <artifact.arcbc|artifact.arclib>" :: call
        return 1
    let artifact = std.args.get :: 0 :: call
    let rendered = arcana_compiler_core.ir_dump.render_artifact_ir_dump :: artifact :: call
    if not rendered.ok:
        std.io.print[Str] :: rendered.message :: call
        return 1
    std.io.print[Str] :: rendered.value :: call
    return 0
