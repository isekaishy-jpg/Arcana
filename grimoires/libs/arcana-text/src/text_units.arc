import std.text
import std.text

export fn utf8_char_len(first: Int) -> Int:
    if first < 128:
        return 1
    if first < 224:
        return 2
    if first < 240:
        return 3
    if first < 248:
        return 4
    return 1

fn is_continuation_byte(value: Int) -> Bool:
    return value >= 128 and value < 192

export fn previous_scalar_start(read text: Str, offset: Int) -> Int:
    if offset <= 0:
        return 0
    let bytes = std.text.bytes_from_str_utf8 :: text :: call
    let mut index = offset - 1
    while index > 0 and (arcana_text.text_units.is_continuation_byte :: (std.text.bytes_at :: bytes, index :: call) :: call):
        index -= 1
    return index

export fn next_scalar_end(read text: Str, offset: Int) -> Int:
    let bytes = std.text.bytes_from_str_utf8 :: text :: call
    let total = std.text.bytes_len :: bytes :: call
    if offset >= total:
        return total
    let count = arcana_text.text_units.utf8_char_len :: (std.text.bytes_at :: bytes, offset :: call) :: call
    let mut next = offset + count
    if next > total:
        next = total
    if next <= offset:
        next = offset + 1
    return next

export fn scalar_text_at(read text: Str, offset: Int) -> Str:
    let next = arcana_text.text_units.next_scalar_end :: text, offset :: call
    return std.text.slice_bytes :: text, offset, next :: call

export fn codepoint_at(read text: Str, offset: Int) -> Int:
    let scalar = arcana_text.text_units.scalar_text_at :: text, offset :: call
    let bytes = std.text.bytes_from_str_utf8 :: scalar :: call
    let total = std.text.bytes_len :: bytes :: call
    if total <= 0:
        return 0
    let first = std.text.bytes_at :: bytes, 0 :: call
    if first < 128:
        return first
    if total >= 2 and first < 224:
        return ((first % 32) * 64) + ((std.text.bytes_at :: bytes, 1 :: call) % 64)
    if total >= 3 and first < 240:
        return ((first % 16) * 4096) + (((std.text.bytes_at :: bytes, 1 :: call) % 64) * 64) + ((std.text.bytes_at :: bytes, 2 :: call) % 64)
    if total >= 4 and first < 248:
        return ((first % 8) * 262144) + (((std.text.bytes_at :: bytes, 1 :: call) % 64) * 4096) + (((std.text.bytes_at :: bytes, 2 :: call) % 64) * 64) + ((std.text.bytes_at :: bytes, 3 :: call) % 64)
    return first

export fn is_combining_mark(codepoint: Int) -> Bool:
    if codepoint >= 768 and codepoint <= 879:
        return true
    if codepoint >= 1425 and codepoint <= 1479:
        return true
    if codepoint == 1523 or codepoint == 1524:
        return true
    if codepoint >= 1611 and codepoint <= 1631:
        return true
    if codepoint == 1648:
        return true
    if codepoint >= 1750 and codepoint <= 1773:
        return true
    if codepoint >= 2362 and codepoint <= 2381:
        return true
    if codepoint >= 2385 and codepoint <= 2403:
        return true
    if codepoint >= 6832 and codepoint <= 6911:
        return true
    if codepoint >= 7616 and codepoint <= 7679:
        return true
    if codepoint >= 8400 and codepoint <= 8447:
        return true
    if codepoint >= 65024 and codepoint <= 65039:
        return true
    if codepoint >= 65136 and codepoint <= 65151:
        return true
    if codepoint == 8419:
        return true
    return false

fn is_variation_selector(codepoint: Int) -> Bool:
    if codepoint >= 65024 and codepoint <= 65039:
        return true
    if codepoint >= 917760 and codepoint <= 917999:
        return true
    return false

fn is_joiner(codepoint: Int) -> Bool:
    return codepoint == 8204 or codepoint == 8205

fn is_regional_indicator(codepoint: Int) -> Bool:
    return codepoint >= 127462 and codepoint <= 127487

fn is_skin_tone_modifier(codepoint: Int) -> Bool:
    return codepoint >= 127995 and codepoint <= 127999

export fn is_cluster_extension(codepoint: Int) -> Bool:
    return (arcana_text.text_units.is_combining_mark :: codepoint :: call) or (arcana_text.text_units.is_variation_selector :: codepoint :: call) or (arcana_text.text_units.is_skin_tone_modifier :: codepoint :: call)

fn is_ascii_word_byte(value: Int) -> Bool:
    return (std.text.is_alpha_byte :: value :: call) or (std.text.is_digit_byte :: value :: call) or value == 95

fn is_spacing_codepoint(codepoint: Int) -> Bool:
    return codepoint == 9 or codepoint == 10 or codepoint == 13 or codepoint == 32 or codepoint == 160 or codepoint == 5760 or codepoint == 12288

export fn is_newline_codepoint(codepoint: Int) -> Bool:
    return codepoint == 10 or codepoint == 11 or codepoint == 12 or codepoint == 13 or codepoint == 133 or codepoint == 8232 or codepoint == 8233

export fn is_spacing_or_separator_codepoint(codepoint: Int) -> Bool:
    if arcana_text.text_units.is_spacing_codepoint :: codepoint :: call:
        return true
    if codepoint == 8192 or codepoint == 8193 or codepoint == 8194 or codepoint == 8195 or codepoint == 8196 or codepoint == 8197 or codepoint == 8198 or codepoint == 8199 or codepoint == 8200 or codepoint == 8201 or codepoint == 8202 or codepoint == 8239 or codepoint == 8287:
        return true
    return false

export fn is_format_control_codepoint(codepoint: Int) -> Bool:
    if codepoint == 173 or codepoint == 8203 or codepoint == 8204 or codepoint == 8205 or codepoint == 8206 or codepoint == 8207 or codepoint == 8234 or codepoint == 8235 or codepoint == 8236 or codepoint == 8237 or codepoint == 8238 or codepoint == 8288 or codepoint == 8294 or codepoint == 8295 or codepoint == 8296 or codepoint == 8297 or codepoint == 65279:
        return true
    return false

fn is_ascii_punctuation(codepoint: Int) -> Bool:
    if codepoint >= 33 and codepoint <= 47:
        return true
    if codepoint >= 58 and codepoint <= 64:
        return true
    if codepoint >= 91 and codepoint <= 96:
        return true
    if codepoint >= 123 and codepoint <= 126:
        return true
    return false

fn is_common_punctuation(codepoint: Int) -> Bool:
    if arcana_text.text_units.is_ascii_punctuation :: codepoint :: call:
        return true
    if codepoint >= 8192 and codepoint <= 8303:
        return true
    if codepoint >= 12289 and codepoint <= 12351:
        return true
    if codepoint >= 64830 and codepoint <= 65023:
        return true
    return false

export fn is_word_codepoint(codepoint: Int) -> Bool:
    if codepoint <= 0:
        return false
    if arcana_text.text_units.is_spacing_codepoint :: codepoint :: call:
        return false
    if codepoint < 128:
        return arcana_text.text_units.is_ascii_word_byte :: codepoint :: call
    if arcana_text.text_units.is_common_punctuation :: codepoint :: call:
        return false
    if arcana_text.text_units.is_cluster_extension :: codepoint :: call:
        return true
    return true

fn seek_word_cluster_start(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    if total <= 0:
        return 0
    let mut cursor = arcana_text.text_units.clamp_offset :: text, offset :: call
    if cursor >= total:
        cursor = arcana_text.text_units.previous_cluster_start :: text, total :: call
    if cursor < total:
        let codepoint = arcana_text.text_units.codepoint_at :: text, cursor :: call
        if not (arcana_text.text_units.is_word_codepoint :: codepoint :: call):
            let mut forward = cursor
            while forward < total:
                let next_codepoint = arcana_text.text_units.codepoint_at :: text, forward :: call
                if arcana_text.text_units.is_word_codepoint :: next_codepoint :: call:
                    return forward
                forward = arcana_text.text_units.next_cluster_end :: text, forward :: call
            if cursor > 0:
                let mut backward = arcana_text.text_units.previous_cluster_start :: text, cursor :: call
                while backward >= 0:
                    let prior_codepoint = arcana_text.text_units.codepoint_at :: text, backward :: call
                    if arcana_text.text_units.is_word_codepoint :: prior_codepoint :: call:
                        return backward
                    if backward <= 0:
                        break
                    let next = arcana_text.text_units.previous_cluster_start :: text, backward :: call
                    if next == backward:
                        break
                    backward = next
            return cursor
    return cursor

export fn clamp_offset(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    if offset < 0:
        return 0
    if offset > total:
        return total
    return offset

export fn next_cluster_end(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    if offset >= total:
        return total
    let first_codepoint = arcana_text.text_units.codepoint_at :: text, offset :: call
    let mut regional_count = match arcana_text.text_units.is_regional_indicator :: first_codepoint :: call:
        true => 1
        false => 0
    let mut cursor = arcana_text.text_units.next_scalar_end :: text, offset :: call
    let mut join_next = false
    while cursor < total:
        let codepoint = arcana_text.text_units.codepoint_at :: text, cursor :: call
        if join_next:
            cursor = arcana_text.text_units.next_scalar_end :: text, cursor :: call
            join_next = false
            continue
        if arcana_text.text_units.is_joiner :: codepoint :: call:
            join_next = true
            cursor = arcana_text.text_units.next_scalar_end :: text, cursor :: call
            continue
        if arcana_text.text_units.is_cluster_extension :: codepoint :: call:
            cursor = arcana_text.text_units.next_scalar_end :: text, cursor :: call
            continue
        if regional_count > 0 and regional_count < 2 and (arcana_text.text_units.is_regional_indicator :: codepoint :: call):
            regional_count += 1
            cursor = arcana_text.text_units.next_scalar_end :: text, cursor :: call
            continue
        break
    return cursor

export fn previous_cluster_start(read text: Str, offset: Int) -> Int:
    if offset <= 0:
        return 0
    let mut start = arcana_text.text_units.previous_scalar_start :: text, offset :: call
    while start > 0:
        let prior = arcana_text.text_units.previous_scalar_start :: text, start :: call
        let prior_end = arcana_text.text_units.next_cluster_end :: text, prior :: call
        if prior_end > start:
            start = prior
        else:
            break
    return start

export fn word_boundary(read text: Str, offset: Int) -> (Int, Int):
    let total = std.text.len_bytes :: text :: call
    if total <= 0:
        return (0, 0)
    let mut start = arcana_text.text_units.seek_word_cluster_start :: text, offset :: call
    if start < 0:
        start = 0
    if start >= total:
        return (total, total)
    let codepoint = arcana_text.text_units.codepoint_at :: text, start :: call
    if not (arcana_text.text_units.is_word_codepoint :: codepoint :: call):
        let end = arcana_text.text_units.next_cluster_end :: text, start :: call
        return (start, end)
    let mut left = start
    while left > 0:
        let prior = arcana_text.text_units.previous_cluster_start :: text, left :: call
        let prior_codepoint = arcana_text.text_units.codepoint_at :: text, prior :: call
        if not (arcana_text.text_units.is_word_codepoint :: prior_codepoint :: call):
            break
        left = prior
    let mut right = arcana_text.text_units.next_cluster_end :: text, start :: call
    while right < total:
        let next_codepoint = arcana_text.text_units.codepoint_at :: text, right :: call
        if not (arcana_text.text_units.is_word_codepoint :: next_codepoint :: call):
            break
        right = arcana_text.text_units.next_cluster_end :: text, right :: call
    return (left, right)
