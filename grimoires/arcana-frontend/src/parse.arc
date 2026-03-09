import arcana_frontend.types
import std.collections.list
import std.io
import std.text
use arcana_frontend.types.Token

record DelimMark:
    byte: Int
    line: Int
    column: Int

fn is_open(byte: Int) -> Bool:
    return byte == 40 or byte == 91 or byte == 123

fn is_close(byte: Int) -> Bool:
    return byte == 41 or byte == 93 or byte == 125

fn expected_open(byte: Int) -> Int:
    if byte == 41:
        return 40
    if byte == 93:
        return 91
    return 123

fn byte_name(byte: Int) -> Str:
    if byte == 40:
        return "("
    if byte == 41:
        return ")"
    if byte == 91:
        return "["
    if byte == 93:
        return "]"
    if byte == 123:
        return "{"
    if byte == 125:
        return "}"
    return "?"

export fn check(path: Str, text: Str, read _tokens: List[Token]) -> (Int, Int):
    let mut errors = 0
    let mut checksum = 0
    let mut stack = std.collections.list.new[arcana_frontend.parse.DelimMark] :: :: call
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    let mut line = 1
    let mut column = 1
    let mut in_string = false
    let mut in_comment = false

    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if in_comment:
            if b == 10:
                in_comment = false
                line += 1
                column = 1
            else:
                column += 1
            i += 1
            continue

        if in_string:
            if b == 34:
                in_string = false
            if b == 10:
                line += 1
                column = 1
            else:
                column += 1
            i += 1
            continue

        if b == 47 and (i + 1) < n:
            let next = std.text.byte_at :: text, i + 1 :: call
            if next == 47:
                in_comment = true
                i += 2
                column += 2
                continue

        if b == 34:
            in_string = true
            i += 1
            column += 1
            continue

        if arcana_frontend.parse.is_open :: b :: call:
            let mark = arcana_frontend.parse.DelimMark :: byte = b, line = line, column = column :: call
            stack :: mark :: push
        else:
            if arcana_frontend.parse.is_close :: b :: call:
                if (stack :: :: len) == 0:
                    let msg = "unexpected closing delimiter " + (arcana_frontend.parse.byte_name :: b :: call)
                    "CHECK_DIAG_V1" :: :: std.io.print
                    "ARC-LEGACY-CHECK-ERROR" :: :: std.io.print
                    "error" :: :: std.io.print
                    path :: :: std.io.print
                    line :: :: std.io.print
                    column :: :: std.io.print
                    line :: :: std.io.print
                    column :: :: std.io.print
                    msg :: :: std.io.print
                    errors += 1
                    checksum = ((checksum * 131) + line + 7) % 2147483647
                    checksum = ((checksum * 131) + column + 7) % 2147483647
                else:
                    let top = stack :: :: pop
                    let expected = arcana_frontend.parse.expected_open :: b :: call
                    if top.byte != expected:
                        let msg = "delimiter mismatch: expected close for " + (arcana_frontend.parse.byte_name :: top.byte :: call) + ", found " + (arcana_frontend.parse.byte_name :: b :: call)
                        "CHECK_DIAG_V1" :: :: std.io.print
                        "ARC-LEGACY-CHECK-ERROR" :: :: std.io.print
                        "error" :: :: std.io.print
                        path :: :: std.io.print
                        line :: :: std.io.print
                        column :: :: std.io.print
                        line :: :: std.io.print
                        column :: :: std.io.print
                        msg :: :: std.io.print
                        errors += 1
                        checksum = ((checksum * 131) + line + 7) % 2147483647
                        checksum = ((checksum * 131) + column + 7) % 2147483647
        if b == 10:
            line += 1
            column = 1
        else:
            column += 1
        i += 1

    while (stack :: :: len) > 0:
        let mark = stack :: :: pop
        let msg = "unclosed delimiter " + (arcana_frontend.parse.byte_name :: mark.byte :: call)
        "CHECK_DIAG_V1" :: :: std.io.print
        "ARC-LEGACY-CHECK-ERROR" :: :: std.io.print
        "error" :: :: std.io.print
        path :: :: std.io.print
        mark.line :: :: std.io.print
        mark.column :: :: std.io.print
        mark.line :: :: std.io.print
        mark.column :: :: std.io.print
        msg :: :: std.io.print
        errors += 1
        checksum = ((checksum * 131) + mark.line + 7) % 2147483647
        checksum = ((checksum * 131) + mark.column + 7) % 2147483647

    return (errors, checksum)
