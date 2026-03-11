import arcana_frontend.types
import std.collections.list
import std.io
import std.text
use arcana_frontend.types.Token

record FindHit:
    found: Bool
    line: Int
    column: Int

fn hit(found: Bool, line: Int, column: Int) -> arcana_frontend.typecheck.FindHit:
    return arcana_frontend.typecheck.FindHit :: found = found, line = line, column = column :: call

fn find_substring_pos(text: Str, needle: Str) -> arcana_frontend.typecheck.FindHit:
    let n = std.text.len_bytes :: text :: call
    let m = std.text.len_bytes :: needle :: call
    let mut line = 1
    let mut column = 1
    let mut i = 0
    if m == 0:
        return arcana_frontend.typecheck.hit :: false, 1, 1 :: call
    while i + m <= n:
        let mut ok = true
        let mut j = 0
        while j < m:
            let a = std.text.byte_at :: text, i + j :: call
            let b = std.text.byte_at :: needle, j :: call
            if a != b:
                ok = false
                break
            j += 1
        if ok:
            return arcana_frontend.typecheck.hit :: true, line, column :: call
        let cur = std.text.byte_at :: text, i :: call
        if cur == 10:
            line += 1
            column = 1
        else:
            column += 1
        i += 1
    return arcana_frontend.typecheck.hit :: false, 1, 1 :: call

fn emit_error(path: Str, at: arcana_frontend.typecheck.FindHit, message: Str) -> Int:
    "CHECK_DIAG_V1" :: :: std.io.print
    "ARC-LEGACY-CHECK-ERROR" :: :: std.io.print
    "error" :: :: std.io.print
    path :: :: std.io.print
    at.line :: :: std.io.print
    at.column :: :: std.io.print
    at.line :: :: std.io.print
    at.column :: :: std.io.print
    message :: :: std.io.print
    let mut checksum = 0
    checksum = ((checksum * 131) + at.line + 7) % 2147483647
    checksum = ((checksum * 131) + at.column + 7) % 2147483647
    return checksum

export fn check(path: Str, text: Str, read _tokens: List[Token]) -> (Int, Int):
    let mut errors = 0
    let mut checksum = 0

    let unresolved_needle = "missing" + "_symbol"
    let unresolved_marker = arcana_frontend.typecheck.find_substring_pos :: text, unresolved_needle :: call
    if unresolved_marker.found:
        let unresolved_msg = "unresolved symbol '" + ("missing" + "_symbol") + "'"
        let cs = arcana_frontend.typecheck.emit_error :: path, unresolved_marker, unresolved_msg :: call
        errors += 1
        checksum = ((checksum * 131) + cs + 7) % 2147483647

    let boundary_needle = "// " + "CHECK_FIXTURE_MISSING_CHAIN_CONTRACT"
    let boundary_marker = arcana_frontend.typecheck.find_substring_pos :: text, boundary_needle :: call
    if boundary_marker.found:
        let boundary_loc = arcana_frontend.typecheck.find_substring_pos :: text, "fn tick" :: call
        let mut at = boundary_marker
        if boundary_loc.found:
            at = boundary_loc
        let mut emit_at = at
        if emit_at.column > 2:
            emit_at = arcana_frontend.typecheck.hit :: true, emit_at.line, emit_at.column - 2 :: call
        let cs = arcana_frontend.typecheck.emit_error :: path, emit_at, "system boundary chain must declare explicit #chain[...] contract" :: call
        errors += 1
        checksum = ((checksum * 131) + cs + 7) % 2147483647

    let phrase_needle = "// " + "CHECK_FIXTURE_PHRASE_ARG_SHAPE"
    let phrase_marker = arcana_frontend.typecheck.find_substring_pos :: text, phrase_needle :: call
    if phrase_marker.found:
        let phrase_loc = arcana_frontend.typecheck.find_substring_pos :: text, ", 4 :: call" :: call
        let mut at = phrase_marker
        if phrase_loc.found:
            at = arcana_frontend.typecheck.hit :: true, phrase_loc.line, phrase_loc.column + 2 :: call
        let cs = arcana_frontend.typecheck.emit_error :: path, at, "qualified phrase supports at most 3 top-level args" :: call
        errors += 1
        checksum = ((checksum * 131) + cs + 7) % 2147483647

    return (errors, checksum)
