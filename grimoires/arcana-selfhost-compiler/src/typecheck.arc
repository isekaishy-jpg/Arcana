import arcana_compiler_core.semantics
import protocol
import std.text
import types

fn diag(path: Str, read meta: (Str, Str), read pos: (Int, Int)) -> types.Diag:
    let out_meta = (meta.0, "error")
    let start = (pos.0, pos.1)
    let loc = (path, start)
    let tail = (start, meta.1)
    return types.Diag :: meta = out_meta, loc = loc, tail = tail :: call

fn derive_check_target(path: Str) -> Str:
    return arcana_compiler_core.semantics.derive_check_target :: path :: call

fn validate_target_with_frontend(target: Str) -> (Int, Int):
    return arcana_compiler_core.semantics.validate_target_with_frontend :: target :: call

export fn validate_semantics_target(path: Str) -> (Int, Int):
    let target = typecheck.derive_check_target :: path :: call
    let sem = arcana_compiler_core.semantics.validate_semantics_target :: path :: call
    let mut checksum = protocol.fold_checksum :: 0, (std.text.len_bytes :: target :: call) :: call
    checksum = protocol.fold_checksum :: checksum, sem.1 :: call
    if sem.0 > 0:
        let d = typecheck.diag :: target, ("ARC-SHCOMP-CHECK-FAILED", "selfhost typecheck stage reported semantic errors"), (1, 1) :: call
        protocol.emit_diag :: d :: call
    return (sem.0, checksum)

export fn validate_semantics(path: Str, text: Str) -> (Int, Int):
    let n = std.text.len_bytes :: text :: call
    let base = protocol.fold_checksum :: 0, n :: call
    let sem = typecheck.validate_semantics_target :: path :: call
    let checksum = protocol.fold_checksum :: base, sem.1 :: call
    return (sem.0, checksum)
