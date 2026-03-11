import std.args
import std.fs
import std.io
import std.text
import fs_support
use std.io as io
import arcana_compiler_core.bytecode_writer

fn embedded_spec(name: Str) -> Str:
    if name == "__lib_fixture":
        let mut spec = "# selfhost_emit_spec_v1"
        spec += "\n# source_fingerprint=test"
        spec += "\nkind=lib"
        spec += "\nbytecode_version=29"
        spec += "\nstd_abi=std-abi-v1"
        spec += "\nexport=util|1|0|0|Int|Int"
        spec += "\ndep=core|sha256:abc"
        spec += "\nkind=module"
        spec += "\nversion=29"
        spec += "\nfunction=util|0|1|0|1|0|0"
        spec += "\ncode=3|0|0"
        spec += "\ncode=20|0|0"
        spec += "\nendfn"
        return spec
    if name == "__lib_nocomment":
        let mut spec = "kind=lib"
        spec += "\nbytecode_version=29"
        spec += "\nstd_abi=std-abi-v1"
        spec += "\nexport=util|1|0|0|Int|Int"
        spec += "\ndep=core|sha256:abc"
        spec += "\nkind=module"
        spec += "\nversion=29"
        spec += "\nfunction=util|0|1|0|1|0|0"
        spec += "\ncode=3|0|0"
        spec += "\ncode=20|0|0"
        spec += "\nendfn"
        return spec
    return ""

fn main() -> Int:
    if (std.args.count :: :: call) < 2:
        io.print[Str] :: "usage: selfhost_write_spec_tool <spec-file|__lib_fixture|__lib_nocomment> <output-file>" :: call
        return 1
    let spec_path = std.args.get :: 0 :: call
    let output_path = std.args.get :: 1 :: call
    let mut spec_text = embedded_spec :: spec_path :: call
    if (std.text.len_bytes :: spec_text :: call) <= 0:
        spec_text = fs_support.read_text_or :: spec_path, "" :: call
    if (std.text.len_bytes :: spec_text :: call) <= 0:
        let mut exists = "false"
        if std.fs.is_file :: spec_path :: call:
            exists = "true"
        io.print[Str] :: ("spec read failed: `" + spec_path + "` exists=" + exists) :: call
        return 1
    if output_path == "__print_kind":
        io.print[Str] :: (arcana_compiler_core.bytecode_writer.detect_spec_kind :: spec_text :: call) :: call
        return 0
    let detail = arcana_compiler_core.bytecode_writer.write_spec_file_error :: spec_text, output_path :: call
    if (std.text.len_bytes :: detail :: call) > 0:
        let mut message = "spec write failed"
        message = message + ": " + detail
        io.print[Str] :: message :: call
        return 1
    io.print[Str] :: "ok" :: call
    return 0
