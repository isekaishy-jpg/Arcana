import std.text
import std.collections.list
import protocol
import types

export fn trim_cr(line: Str) -> Str:
    let n = std.text.len_bytes :: line :: call
    if n <= 0:
        return line
    if (std.text.byte_at :: line, n - 1 :: call) == 13:
        return std.text.slice_bytes :: line, 0, n - 1 :: call
    return line

export fn is_digit(b: Int) -> Bool:
    return std.text.is_digit_byte :: b :: call

export fn parse_int_ascii(text: Str) -> (Bool, Int):
    let n = std.text.len_bytes :: text :: call
    if n <= 0:
        return (false, 0)
    let mut i = 0
    let mut out = 0
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if not (tokenize.is_digit :: b :: call):
            return (false, 0)
        out = (out * 10) + (b - 48)
        i += 1
    return (true, out)

export fn find_byte(text: Str, start: Int, needle: Int) -> Int:
    return std.text.find_byte :: text, start, needle :: call

record DelimMark:
    byte: Int
    line: Int
    column: Int

fn is_open_delim(byte: Int) -> Bool:
    return byte == 40 or byte == 91 or byte == 123

fn is_close_delim(byte: Int) -> Bool:
    return byte == 41 or byte == 93 or byte == 125

fn expected_open_delim(byte: Int) -> Int:
    if byte == 41:
        return 40
    if byte == 93:
        return 91
    return 123

fn delim_name(byte: Int) -> Str:
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

fn emit_diag(path: Str, read meta: (Str, Str), read pos: (Int, Int)):
    let out_meta = (meta.0, "error")
    let start = (pos.0, pos.1)
    let loc = (path, start)
    let tail = (start, meta.1)
    let diag = types.Diag :: meta = out_meta, loc = loc, tail = tail :: call
    protocol.emit_diag :: diag :: call

export fn validate_token_stream(path: Str, text: Str) -> (Int, Int):
    let mut errors = 0
    let mut checksum = 0
    let mut stack = std.collections.list.new[tokenize.DelimMark] :: :: call
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

        if tokenize.is_open_delim :: b :: call:
            let mark = tokenize.DelimMark :: byte = b, line = line, column = column :: call
            stack :: mark :: push
        else:
            if tokenize.is_close_delim :: b :: call:
                if (stack :: :: len) <= 0:
                    let msg = "unexpected closing delimiter " + (tokenize.delim_name :: b :: call)
                    let meta = ("ARC-SHCOMP-COMPILE-FAILED", msg)
                    tokenize.emit_diag :: path, meta, (line, column) :: call
                    errors += 1
                    checksum = protocol.fold_checksum :: checksum, line :: call
                    checksum = protocol.fold_checksum :: checksum, column :: call
                else:
                    let top = stack :: :: pop
                    let expected = tokenize.expected_open_delim :: b :: call
                    if top.byte != expected:
                        let msg = "delimiter mismatch: expected close for " + (tokenize.delim_name :: top.byte :: call) + ", found " + (tokenize.delim_name :: b :: call)
                        let meta = ("ARC-SHCOMP-COMPILE-FAILED", msg)
                        tokenize.emit_diag :: path, meta, (line, column) :: call
                        errors += 1
                        checksum = protocol.fold_checksum :: checksum, line :: call
                        checksum = protocol.fold_checksum :: checksum, column :: call

        if b == 10:
            line += 1
            column = 1
        else:
            column += 1
        i += 1

    while (stack :: :: len) > 0:
        let mark = stack :: :: pop
        let msg = "unclosed delimiter " + (tokenize.delim_name :: mark.byte :: call)
        let meta = ("ARC-SHCOMP-COMPILE-FAILED", msg)
        tokenize.emit_diag :: path, meta, (mark.line, mark.column) :: call
        errors += 1
        checksum = protocol.fold_checksum :: checksum, mark.line :: call
        checksum = protocol.fold_checksum :: checksum, mark.column :: call

    return (errors, checksum)
