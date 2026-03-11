import std.args
import std.io
use std.io as io
import arcana_compiler_core.bytecode_writer

fn main() -> Int:
    if (std.args.count :: :: call) < 2:
        io.print[Str] :: "usage: selfhost_bytecode_writer_tool <module_hello|module_behavior|lib_util> <output-file>" :: call
        return 1
    let fixture = std.args.get :: 0 :: call
    let output_path = std.args.get :: 1 :: call
    let wrote = bytecode_writer.write_fixture_file :: fixture, output_path :: call
    if not wrote:
        io.print[Str] :: "fixture write failed" :: call
        return 1
    return 0
