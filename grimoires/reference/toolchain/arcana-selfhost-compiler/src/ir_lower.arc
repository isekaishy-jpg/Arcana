import std.fs
import std.collections.list
import std.path
import std.text
import arcana_compiler_core.sources
import protocol
import types

export fn artifact_fingerprint(checksum: Int) -> Str:
    return "fold:" + (std.text.from_int :: checksum :: call)

export fn ir_fingerprint(checksum: Int) -> Str:
    return "ir:" + (std.text.from_int :: checksum :: call)

fn emit_diag(path: Str, code: Str, message: Str):
    let meta = (code, "error")
    let start = (1, 1)
    let loc = (path, start)
    let tail = (start, message)
    let diag = types.Diag :: meta = meta, loc = loc, tail = tail :: call
    protocol.emit_diag :: diag :: call

export fn validate_lowering(path: Str, text: Str) -> (Int, Int):
    let n = std.text.len_bytes :: text :: call
    if n > 0:
        let checksum = protocol.fold_checksum :: 0, n :: call
        return (0, checksum)
    ir_lower.emit_diag :: path, "ARC-SHCOMP-LOWER-FAILED", "selfhost lower stage received empty source text" :: call
    return (1, 1)

export fn validate_lowering_target(target: Str) -> (Int, Int):
    let files = arcana_compiler_core.sources.count_files_and_bytes :: target :: call
    if files.0 <= 0:
        ir_lower.emit_diag :: target, "ARC-SHCOMP-LOWER-FAILED", "selfhost lower stage found no .arc sources to lower" :: call
        return (1, 1)
    if files.1 <= 0:
        ir_lower.emit_diag :: target, "ARC-SHCOMP-LOWER-FAILED", "selfhost lower stage received empty source corpus" :: call
        return (1, 1)
    let mut checksum = 0
    checksum = protocol.fold_checksum :: checksum, (std.text.len_bytes :: target :: call) :: call
    checksum = protocol.fold_checksum :: checksum, files.0 :: call
    checksum = protocol.fold_checksum :: checksum, files.1 :: call
    return (0, checksum)
