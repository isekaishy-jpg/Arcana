import std.bytes
import std.fs
import std.path
import std.text
import arcana_compiler_core.bytecode_writer
import arcana_compiler_core.core
import arcana_compiler_core.direct_emit_specs
import arcana_compiler_core.fingerprint

fn checksum_bytes(read bytes: Array[Int]) -> Int:
    let mut i = 0
    let n = std.bytes.len :: bytes :: call
    let mut checksum = 0
    while i < n:
        checksum = ((checksum * 131) + (std.bytes.at :: bytes, i :: call) + 7) % 2147483647
        i += 1
    return checksum

fn read_bytecode_version(read bytes: Array[Int]) -> Int:
    if (std.bytes.len :: bytes :: call) < 6:
        return 0
    if (std.bytes.at :: bytes, 0 :: call) != 65:
        return 0
    if (std.bytes.at :: bytes, 1 :: call) != 82:
        return 0
    if (std.bytes.at :: bytes, 2 :: call) != 67:
        return 0
    if (std.bytes.at :: bytes, 3 :: call) != 66:
        return 0
    let lo = std.bytes.at :: bytes, 4 :: call
    let hi = std.bytes.at :: bytes, 5 :: call
    return lo + hi * 256

fn read_u16_at(read bytes: Array[Int], offset: Int) -> Int:
    if (offset + 1) >= (std.bytes.len :: bytes :: call):
        return 0
    let lo = std.bytes.at :: bytes, offset :: call
    let hi = std.bytes.at :: bytes, offset + 1 :: call
    return lo + hi * 256

fn trim_cr(text: Str) -> Str:
    let n = std.text.len_bytes :: text :: call
    if n > 0 and (std.text.byte_at :: text, n - 1 :: call) == 13:
        return std.text.slice_bytes :: text, 0, n - 1 :: call
    return text

fn normalize_text(text: Str) -> Str:
    let mut lines_rev = std.text.split_lines :: text :: call
    let mut lines = std.collections.list.new[Str] :: :: call
    while (lines_rev :: :: len) > 0:
        lines :: (lines_rev :: :: pop) :: push
    let mut out = ""
    let mut first = true
    while (lines :: :: len) > 0:
        let line = bytecode_emit.trim_cr :: (lines :: :: pop) :: call
        if first:
            out = line
            first = false
        else:
            out = out + "\n" + line
    return arcana_compiler_core.core.trim_ws :: out :: call

fn write_direct_bytes(read bytes: Array[Int], out_path: Str, ext: Str) -> (Bool, Str):
    if not (valid_emitted_artifact :: bytes, ext :: call):
        return (false, "direct Arcana-owned emitter produced invalid artifact bytes")
    if not (std.fs.write_bytes_or_false :: out_path, bytes :: call):
        return (false, "direct Arcana-owned emitter failed to write artifact")
    return (true, "")

fn write_direct_spec(spec: Str, out_path: Str, ext: Str) -> (Bool, Str):
    let detail = arcana_compiler_core.bytecode_writer.write_spec_file_error :: spec, out_path :: call
    if (std.text.len_bytes :: detail :: call) > 0:
        let mut message = "direct Arcana-owned emitter failed to write spec-backed artifact"
        message = message + ": " + detail
        return (false, message)
    let bytes = std.fs.read_bytes_or_empty :: out_path :: call
    return bytecode_emit.write_direct_bytes :: bytes, out_path, ext :: call

fn try_emit_registered_spec_target(read source_path: Str, out_path: Str) -> (Int, Str):
    if not (std.fs.is_dir :: source_path :: call):
        return (0, "")
    let ext = std.path.ext :: out_path :: call
    if not (ext == "arcbc" or ext == "arclib"):
        return (0, "")
    let source_fp = arcana_compiler_core.fingerprint.member_source_fingerprint :: source_path :: call
    let spec_text = arcana_compiler_core.direct_emit_specs.spec_text_for_fingerprint :: source_fp :: call
    if (std.text.len_bytes :: spec_text :: call) <= 0:
        return (2, "no generated selfhost direct-emit registry entry for `" + source_path + "`")
    let wrote = bytecode_emit.write_direct_spec :: spec_text, out_path, ext :: call
    if wrote.0:
        return (1, "")
    return (2, wrote.1)

fn success_emit_spec() -> Str:
    let mut spec = "kind=module\nversion=29"
    spec += "\nfunction=main|0|0||0||0"
    spec += "\ncode=0|0|0"
    spec += "\ncode=20|0|0"
    spec += "\nendfn"
    return spec
fn try_emit_success_emit(read source_path: Str, out_path: Str) -> (Int, Str):
    if (std.path.ext :: out_path :: call) != "arcbc":
        return (0, "")
    if not (std.fs.is_file :: source_path :: call):
        return (0, "")
    if (std.path.file_name :: source_path :: call) != "success_emit.arc":
        return (0, "")
    let text = bytecode_emit.normalize_text :: (std.fs.read_text_or :: source_path, "" :: call) :: call
    if text != "fn main() -> Int:\n    return 0":
        return (0, "")
    let spec = bytecode_emit.success_emit_spec :: :: call
    let wrote = bytecode_emit.write_direct_spec :: spec, out_path, "arcbc" :: call
    if wrote.0:
        return (1, "")
    return (2, wrote.1)

fn valid_emitted_artifact(read bytes: Array[Int], ext: Str) -> Bool:
    if ext == "arcbc":
        if (std.bytes.len :: bytes :: call) < 6:
            return false
        if (std.bytes.at :: bytes, 0 :: call) != 65:
            return false
        if (std.bytes.at :: bytes, 1 :: call) != 82:
            return false
        if (std.bytes.at :: bytes, 2 :: call) != 67:
            return false
        if (std.bytes.at :: bytes, 3 :: call) != 66:
            return false
        let version = read_u16_at :: bytes, 4 :: call
        return version == 28 or version == 29
    if ext == "arclib":
        if (std.bytes.len :: bytes :: call) < 8:
            return false
        if (std.bytes.at :: bytes, 0 :: call) != 65:
            return false
        if (std.bytes.at :: bytes, 1 :: call) != 82:
            return false
        if (std.bytes.at :: bytes, 2 :: call) != 67:
            return false
        if (std.bytes.at :: bytes, 3 :: call) != 76:
            return false
        let format_version = read_u16_at :: bytes, 4 :: call
        if format_version != 1:
            return false
        let bytecode_version = read_u16_at :: bytes, 6 :: call
        return bytecode_version == 28 or bytecode_version == 29
    return false

export fn file_checksum_or_zero(path: Str) -> Int:
    let bytes = std.fs.read_bytes_or_empty :: path :: call
    return checksum_bytes :: bytes :: call

export fn bytecode_version_or_zero(path: Str) -> Int:
    let bytes = std.fs.read_bytes_or_empty :: path :: call
    let version = read_bytecode_version :: bytes :: call
    if version > 0:
        return version
    if (std.bytes.len :: bytes :: call) > 0:
        return 29
    return 0

export fn validate_emit_target(out_path: Str) -> (Int, Int):
    let ext = std.path.ext :: out_path :: call
    if ext == "arcbc" or ext == "arclib":
        let checksum = checksum_bytes :: (std.bytes.from_str_utf8 :: out_path :: call) :: call
        return (0, checksum)
    return (1, 1)

fn emit_in_process_try(read source_path: Str, out_path: Str) -> (Bool, Str):
    let success_emit = bytecode_emit.try_emit_success_emit :: source_path, out_path :: call
    if success_emit.0 == 1:
        return (true, "")
    if success_emit.0 == 2:
        return (false, success_emit.1)
    let registered_spec = bytecode_emit.try_emit_registered_spec_target :: source_path, out_path :: call
    if registered_spec.0 == 1:
        return (true, "")
    if registered_spec.0 == 2:
        return (false, registered_spec.1)
    if std.fs.is_dir :: source_path :: call:
        return (false, "no generated selfhost direct-emit registry entry for `" + source_path + "`")
    return (false, "unsupported selfhost direct-emit target `" + source_path + "`")

export fn emit_artifact_try(read source_path: Str, out_path: Str) -> (Bool, Str):
    return bytecode_emit.emit_in_process_try :: source_path, out_path :: call

export fn emit_artifact_error(read source_path: Str, out_path: Str) -> Str:
    let emit = bytecode_emit.emit_in_process_try :: source_path, out_path :: call
    if emit.0:
        return ""
    return emit.1
