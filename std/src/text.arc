import std.kernel.text

export fn len_bytes(text: Str) -> Int:
    return std.kernel.text.text_len_bytes :: text :: call

export fn byte_at(text: Str, index: Int) -> Int:
    return std.kernel.text.text_byte_at :: text, index :: call

export fn slice_bytes(text: Str, start: Int, end: Int) -> Str:
    return std.kernel.text.text_slice_bytes :: text, start, end :: call

export fn starts_with(text: Str, prefix: Str) -> Bool:
    return std.kernel.text.text_starts_with :: text, prefix :: call

export fn ends_with(text: Str, suffix: Str) -> Bool:
    return std.kernel.text.text_ends_with :: text, suffix :: call

export fn split_lines(text: Str) -> List[Str]:
    return std.kernel.text.text_split_lines :: text :: call

export fn from_int(value: Int) -> Str:
    return std.kernel.text.text_from_int :: value :: call

export fn join(read parts: List[Str], sep: Str) -> Str:
    let mut out = ""
    let mut first = true
    for part in parts:
        if not first:
            out += sep
        out += part
        first = false
    return out

export fn join_lines(read lines: List[Str]) -> Str:
    return std.text.join :: lines, "\n" :: call

export fn is_space_byte(b: Int) -> Bool:
    return b == 32 or b == 9 or b == 10 or b == 13

export fn is_digit_byte(b: Int) -> Bool:
    return b >= 48 and b <= 57

export fn is_alpha_byte(b: Int) -> Bool:
    return (b >= 65 and b <= 90) or (b >= 97 and b <= 122)

export fn is_ident_start_byte(b: Int) -> Bool:
    return (std.text.is_alpha_byte :: b :: call) or b == 95

export fn is_ident_continue_byte(b: Int) -> Bool:
    return (std.text.is_ident_start_byte :: b :: call) or (std.text.is_digit_byte :: b :: call)

export fn find_byte(text: Str, start: Int, needle: Int) -> Int:
    let n = std.text.len_bytes :: text :: call
    let mut i = start
    while i < n:
        if (std.text.byte_at :: text, i :: call) == needle:
            return i
        i += 1
    return -1

export fn take_while(text: Str, start: Int, mode: Int) -> Int:
    let n = std.text.len_bytes :: text :: call
    let mut i = start
    while i < n:
        let b = std.text.byte_at :: text, i :: call
        let mut keep = false
        if mode == 0:
            keep = std.text.is_space_byte :: b :: call
        else:
            if mode == 1:
                keep = std.text.is_digit_byte :: b :: call
            else:
                if mode == 2:
                    keep = std.text.is_ident_start_byte :: b :: call
                else:
                    if mode == 3:
                        keep = std.text.is_ident_continue_byte :: b :: call
        if not keep:
            break
        i += 1
    return i
