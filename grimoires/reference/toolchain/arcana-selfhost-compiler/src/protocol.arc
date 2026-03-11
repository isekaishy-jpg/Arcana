import std.io
import types

export fn fold_checksum(acc: Int, delta: Int) -> Int:
    return ((acc * 131) + delta + 7) % 2147483647

export fn emit_diag(diag: types.Diag):
    "COMPILE_DIAG_V1" :: :: std.io.print
    diag.meta.0 :: :: std.io.print
    diag.meta.1 :: :: std.io.print
    diag.loc.0 :: :: std.io.print
    diag.loc.1.0 :: :: std.io.print
    diag.loc.1.1 :: :: std.io.print
    diag.tail.0.0 :: :: std.io.print
    diag.tail.0.1 :: :: std.io.print
    diag.tail.1 :: :: std.io.print

export fn emit_artifact(artifact: types.Artifact):
    "COMPILE_ARTIFACT_V1" :: :: std.io.print
    artifact.left.0 :: :: std.io.print
    artifact.left.1 :: :: std.io.print
    artifact.right.0 :: :: std.io.print
    artifact.right.1.0 :: :: std.io.print
    artifact.right.1.1 :: :: std.io.print

export fn emit_build_event(member: Str, status: Str, artifact_path: Str):
    "BUILD_EVENT_V1" :: :: std.io.print
    member :: :: std.io.print
    status :: :: std.io.print
    artifact_path :: :: std.io.print

export fn emit_final(error_count: Int, warning_count: Int, checksum: Int):
    "COMPILE_FINAL_V1" :: :: std.io.print
    error_count :: :: std.io.print
    warning_count :: :: std.io.print
    checksum :: :: std.io.print
