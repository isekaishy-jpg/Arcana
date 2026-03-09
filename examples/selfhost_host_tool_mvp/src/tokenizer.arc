import std.text

fn is_space(byte: Int) -> Bool:
    if byte == 9:
        return true
    if byte == 10:
        return true
    if byte == 13:
        return true
    if byte == 32:
        return true
    return false

fn is_digit(byte: Int) -> Bool:
    if byte >= 48:
        if byte <= 57:
            return true
    return false

fn is_ident_start(byte: Int) -> Bool:
    if byte == 95:
        return true
    if byte >= 65:
        if byte <= 90:
            return true
    if byte >= 97:
        if byte <= 122:
            return true
    return false

fn is_ident_continue(byte: Int) -> Bool:
    if is_ident_start :: byte :: call:
        return true
    return is_digit :: byte :: call

fn fold_checksum(acc: Int, delta: Int) -> Int:
    return (acc + delta) % 2147483647

export fn tokenize_subset(text: Str) -> (Int, Int):
    let n = std.text.len_bytes :: text :: call
    let mut i = 0
    let mut count = 0
    let mut checksum = 0

    while i < n:
        let b = std.text.byte_at :: text, i :: call
        if is_space :: b :: call:
            i += 1
            continue

        if is_ident_start :: b :: call:
            let mut len = 0
            while i < n:
                let c = std.text.byte_at :: text, i :: call
                if not (is_ident_continue :: c :: call):
                    break
                len += 1
                i += 1
            count += 1
            checksum = fold_checksum :: checksum, 101 + len :: call
            continue

        if is_digit :: b :: call:
            let mut len = 0
            while i < n:
                let c = std.text.byte_at :: text, i :: call
                if not (is_digit :: c :: call):
                    break
                len += 1
                i += 1
            count += 1
            checksum = fold_checksum :: checksum, 211 + len :: call
            continue

        count += 1
        checksum = fold_checksum :: checksum, 307 + b :: call
        i += 1

    return (count, checksum)
