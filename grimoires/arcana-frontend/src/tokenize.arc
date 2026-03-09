import arcana_frontend.types
import std.collections.list
import std.text
use arcana_frontend.types.Span
use arcana_frontend.types.Token
use arcana_frontend.types.Pos

fn is_digit(byte: Int) -> Bool:
    return std.text.is_digit_byte :: byte :: call

fn is_ident_start(byte: Int) -> Bool:
    return std.text.is_ident_start_byte :: byte :: call

fn is_ident_continue(byte: Int) -> Bool:
    return std.text.is_ident_continue_byte :: byte :: call

fn is_punct(byte: Int) -> Bool:
    if byte == 123 or byte == 125:
        return true
    if byte == 40 or byte == 41:
        return true
    if byte == 91 or byte == 93:
        return true
    if byte == 44 or byte == 46 or byte == 58 or byte == 59:
        return true
    if byte == 43 or byte == 45 or byte == 42 or byte == 47 or byte == 37:
        return true
    if byte == 61 or byte == 33 or byte == 60 or byte == 62:
        return true
    if byte == 38 or byte == 124 or byte == 63:
        return true
    if byte == 35:
        return true
    return false

fn mk_span(path: Str, start: Pos, end: Pos) -> Span:
    return arcana_frontend.types.Span :: path = path, start = start, end = end :: call

fn mk_token(kind: Str, lexeme: Str, span: Span) -> Token:
    return arcana_frontend.types.Token :: kind = kind, lexeme = lexeme, span = span :: call

export fn scan(path: Str, text: Str) -> List[Token]:
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    let mut line = 1
    let mut column = 1
    let mut out = std.collections.list.new[Token] :: :: call

    while i < n:
        let b = std.text.byte_at :: text, i :: call

        if b == 10:
            i += 1
            line += 1
            column = 1
            continue
        if std.text.is_space_byte :: b :: call:
            i += 1
            column += 1
            continue

        let start_i = i
        let start_line = line
        let start_column = column

        if arcana_frontend.tokenize.is_ident_start :: b :: call:
            i += 1
            column += 1
            while i < n:
                let c = std.text.byte_at :: text, i :: call
                if not (arcana_frontend.tokenize.is_ident_continue :: c :: call):
                    break
                i += 1
                column += 1
            let lexeme = std.text.slice_bytes :: text, start_i, i :: call
            let start = arcana_frontend.types.Pos :: line = start_line, column = start_column :: call
            let end = arcana_frontend.types.Pos :: line = line, column = column :: call
            let span = arcana_frontend.tokenize.mk_span :: path, start, end :: call
            let token = arcana_frontend.tokenize.mk_token :: "ident", lexeme, span :: call
            out :: token :: push
            continue

        if arcana_frontend.tokenize.is_digit :: b :: call:
            i += 1
            column += 1
            while i < n:
                let c = std.text.byte_at :: text, i :: call
                if not (arcana_frontend.tokenize.is_digit :: c :: call):
                    break
                i += 1
                column += 1
            let lexeme = std.text.slice_bytes :: text, start_i, i :: call
            let start = arcana_frontend.types.Pos :: line = start_line, column = start_column :: call
            let end = arcana_frontend.types.Pos :: line = line, column = column :: call
            let span = arcana_frontend.tokenize.mk_span :: path, start, end :: call
            let token = arcana_frontend.tokenize.mk_token :: "int", lexeme, span :: call
            out :: token :: push
            continue

        if arcana_frontend.tokenize.is_punct :: b :: call:
            i += 1
            column += 1
            let lexeme = std.text.slice_bytes :: text, start_i, i :: call
            let start = arcana_frontend.types.Pos :: line = start_line, column = start_column :: call
            let end = arcana_frontend.types.Pos :: line = line, column = column :: call
            let span = arcana_frontend.tokenize.mk_span :: path, start, end :: call
            let token = arcana_frontend.tokenize.mk_token :: "punct", lexeme, span :: call
            out :: token :: push
            continue

        i += 1
        column += 1
        let lexeme = std.text.slice_bytes :: text, start_i, i :: call
        let start = arcana_frontend.types.Pos :: line = start_line, column = start_column :: call
        let end = arcana_frontend.types.Pos :: line = line, column = column :: call
        let span = arcana_frontend.tokenize.mk_span :: path, start, end :: call
        let token = arcana_frontend.tokenize.mk_token :: "other", lexeme, span :: call
        out :: token :: push

    return out
