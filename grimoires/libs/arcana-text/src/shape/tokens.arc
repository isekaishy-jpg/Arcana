import arcana_text.buffer
import arcana_text.text_units
import arcana_text.types
import std.text
import std.collections.list
import std.option
import std.text
use std.option.Option

export record TextToken:
    text: Str
    range: arcana_text.types.TextRange
    whitespace: Bool
    newline: Bool

export enum ShapeItemKind:
    Text
    Placeholder

export record ShapeItem:
    kind: arcana_text.shape.tokens.ShapeItemKind
    range: arcana_text.types.TextRange
    text: Str
    whitespace: Bool
    newline: Bool
    placeholder: Option[arcana_text.types.PlaceholderSpec]

export record ShapeLine:
    range: arcana_text.types.TextRange
    text: Str
    items: List[arcana_text.shape.tokens.ShapeItem]
    hard_break: Bool
    break_range: arcana_text.types.TextRange

record TextItemEmitRequest:
    buffer: arcana_text.buffer.TextBuffer
    token: arcana_text.shape.tokens.TextToken
    start: Int
    end: Int

fn empty_tokens() -> List[arcana_text.shape.tokens.TextToken]:
    return std.collections.list.empty[arcana_text.shape.tokens.TextToken] :: :: call

fn empty_items() -> List[arcana_text.shape.tokens.ShapeItem]:
    return std.collections.list.empty[arcana_text.shape.tokens.ShapeItem] :: :: call

fn empty_lines() -> List[arcana_text.shape.tokens.ShapeLine]:
    return std.collections.list.empty[arcana_text.shape.tokens.ShapeLine] :: :: call

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

export fn is_newline(read text: Str) -> Bool:
    let codepoint = arcana_text.text_units.codepoint_at :: text, 0 :: call
    return arcana_text.text_units.is_newline_codepoint :: codepoint :: call

export fn is_space(read text: Str) -> Bool:
    let codepoint = arcana_text.text_units.codepoint_at :: text, 0 :: call
    return arcana_text.text_units.is_spacing_or_separator_codepoint :: codepoint :: call

export fn text_token(text: Str, start: Int, end: Int) -> arcana_text.shape.tokens.TextToken:
    let mut out = arcana_text.shape.tokens.TextToken :: text = text, range = (arcana_text.types.TextRange :: start = start, end = end :: call) :: call
    out.whitespace = false
    out.newline = false
    return out

fn text_item(read token: arcana_text.shape.tokens.TextToken) -> arcana_text.shape.tokens.ShapeItem:
    let mut item = arcana_text.shape.tokens.ShapeItem :: kind = (arcana_text.shape.tokens.ShapeItemKind.Text :: :: call), range = token.range, text = token.text :: call
    item.whitespace = token.whitespace
    item.newline = token.newline
    item.placeholder = Option.None[arcana_text.types.PlaceholderSpec] :: :: call
    return item

fn placeholder_item(read spec: arcana_text.types.PlaceholderSpec) -> arcana_text.shape.tokens.ShapeItem:
    let mut item = arcana_text.shape.tokens.ShapeItem :: kind = (arcana_text.shape.tokens.ShapeItemKind.Placeholder :: :: call), range = spec.range, text = "" :: call
    item.whitespace = false
    item.newline = false
    item.placeholder = Option.Some[arcana_text.types.PlaceholderSpec] :: spec :: call
    return item

fn placeholder_seen(read seen: List[arcana_text.types.PlaceholderSpec], read spec: arcana_text.types.PlaceholderSpec) -> Bool:
    for existing in seen:
        if existing.range.start == spec.range.start and existing.range.end == spec.range.end and existing.size == spec.size:
            return true
    return false

fn placeholder_seen_parts(read seen: List[arcana_text.types.PlaceholderSpec], read range: arcana_text.types.TextRange, read size: (Int, Int)) -> Bool:
    for existing in seen:
        if existing.range.start == range.start and existing.range.end == range.end and existing.size == size:
            return true
    return false

fn placeholder_covering(read buffer: arcana_text.buffer.TextBuffer, offset: Int) -> Option[arcana_text.types.PlaceholderSpec]:
    for spec in buffer.placeholders:
        if spec.range.start <= offset and offset < spec.range.end:
            return Option.Some[arcana_text.types.PlaceholderSpec] :: spec :: call
    return Option.None[arcana_text.types.PlaceholderSpec] :: :: call

fn placeholder_starting(read buffer: arcana_text.buffer.TextBuffer, offset: Int) -> Option[arcana_text.types.PlaceholderSpec]:
    for spec in buffer.placeholders:
        if spec.range.start == offset:
            return Option.Some[arcana_text.types.PlaceholderSpec] :: spec :: call
    return Option.None[arcana_text.types.PlaceholderSpec] :: :: call

fn next_span_boundary(read buffer: arcana_text.buffer.TextBuffer, cursor: Int, limit: Int) -> Int:
    let mut next = limit
    for span in buffer.spans:
        if span.range.start > cursor and span.range.start < next:
            next = span.range.start
        if span.range.end > cursor and span.range.end < next:
            next = span.range.end
    return next

fn next_placeholder_boundary(read buffer: arcana_text.buffer.TextBuffer, cursor: Int, limit: Int) -> Int:
    let mut next = limit
    for spec in buffer.placeholders:
        if spec.range.start > cursor and spec.range.start < next:
            next = spec.range.start
    return next

fn next_item_boundary(read buffer: arcana_text.buffer.TextBuffer, read token: arcana_text.shape.tokens.TextToken, cursor: Int) -> Int:
    let mut next = token.range.end
    next = arcana_text.shape.tokens.next_span_boundary :: buffer, cursor, next :: call
    next = arcana_text.shape.tokens.next_placeholder_boundary :: buffer, cursor, next :: call
    return next

fn emit_text_item(edit items: List[arcana_text.shape.tokens.ShapeItem], read payload: arcana_text.shape.tokens.TextItemEmitRequest):
    let buffer = payload.buffer
    let token = payload.token
    let start = payload.start
    let end = payload.end
    if end <= start:
        return
    let text = std.text.slice_bytes :: buffer.text, start, end :: call
    let mut segment = arcana_text.shape.tokens.text_token :: text, start, end :: call
    segment.whitespace = token.whitespace
    segment.newline = token.newline
    items :: (arcana_text.shape.tokens.text_item :: segment :: call) :: push

fn fallback_placeholder(offset: Int) -> arcana_text.types.PlaceholderSpec:
    return arcana_text.shape.types.fallback_placeholder :: (arcana_text.types.TextRange :: start = offset, end = offset :: call) :: call

fn push_unseen_placeholder(edit items: List[arcana_text.shape.tokens.ShapeItem], edit emitted: List[arcana_text.types.PlaceholderSpec], read spec: arcana_text.types.PlaceholderSpec):
    if arcana_text.shape.tokens.placeholder_seen_parts :: emitted, spec.range, spec.size :: call:
        return
    items :: (arcana_text.shape.tokens.placeholder_item :: spec :: call) :: push
    emitted :: spec :: push

export fn tokenize_text(read text: Str) -> List[arcana_text.shape.tokens.TextToken]:
    let bytes = std.text.bytes_from_str_utf8 :: text :: call
    let total = std.text.bytes_len :: bytes :: call
    let mut out = empty_tokens :: :: call
    let mut index = 0
    let mut current = ""
    let mut current_start = 0
    let mut current_whitespace = false
    let mut active = false
    while index < total:
        let first = std.text.bytes_at :: bytes, index :: call
        let mut count = arcana_text.shape.tokens.utf8_char_len :: first :: call
        if index + count > total:
            count = 1
        let slice = std.text.bytes_slice :: bytes, index, index + count :: call
        let ch = std.text.bytes_to_str_utf8 :: slice :: call
        if arcana_text.shape.tokens.is_newline :: ch :: call:
            let codepoint = arcana_text.text_units.codepoint_at :: ch, 0 :: call
            let mut line_break_end = index + count
            if codepoint == 13 and line_break_end < total:
                let mut next_count = arcana_text.shape.tokens.utf8_char_len :: (std.text.bytes_at :: bytes, line_break_end :: call) :: call
                if line_break_end + next_count > total:
                    next_count = 1
                let next_slice = std.text.bytes_slice :: bytes, line_break_end, line_break_end + next_count :: call
                let next_text = std.text.bytes_to_str_utf8 :: next_slice :: call
                if next_text == "\n":
                    line_break_end += next_count
            if active:
                let mut token = arcana_text.shape.tokens.text_token :: current, current_start, index :: call
                token.whitespace = current_whitespace
                out :: token :: push
                current = ""
                active = false
            let newline_text = std.text.slice_bytes :: text, index, line_break_end :: call
            let mut newline = arcana_text.shape.tokens.text_token :: newline_text, index, line_break_end :: call
            newline.newline = true
            out :: newline :: push
            index = line_break_end
            continue
        let whitespace = arcana_text.shape.tokens.is_space :: ch :: call
        if not active:
            current = ch
            current_start = index
            current_whitespace = whitespace
            active = true
            index += count
            continue
        if whitespace == current_whitespace:
            current = current + ch
            index += count
            continue
        let mut token = arcana_text.shape.tokens.text_token :: current, current_start, index :: call
        token.whitespace = current_whitespace
        out :: token :: push
        current = ch
        current_start = index
        current_whitespace = whitespace
        active = true
        index += count
    if active:
        let mut token = arcana_text.shape.tokens.text_token :: current, current_start, total :: call
        token.whitespace = current_whitespace
        out :: token :: push
    return out

export fn first_visible_char(read text: Str) -> Str:
    let bytes = std.text.bytes_from_str_utf8 :: text :: call
    let total = std.text.bytes_len :: bytes :: call
    let mut index = 0
    while index < total:
        let first = std.text.bytes_at :: bytes, index :: call
        let mut count = arcana_text.shape.tokens.utf8_char_len :: first :: call
        if index + count > total:
            count = 1
        let slice = std.text.bytes_slice :: bytes, index, index + count :: call
        let ch = std.text.bytes_to_str_utf8 :: slice :: call
        let codepoint = arcana_text.text_units.codepoint_at :: ch, 0 :: call
        if not (arcana_text.shape.tokens.is_space :: ch :: call) and not (arcana_text.shape.tokens.is_newline :: ch :: call) and not (arcana_text.text_units.is_format_control_codepoint :: codepoint :: call):
            return ch
        index += count
    return "M"

export fn collect_items(read buffer: arcana_text.buffer.TextBuffer) -> List[arcana_text.shape.tokens.ShapeItem]:
    let tokens = arcana_text.shape.tokens.tokenize_text :: buffer.text :: call
    let mut items = empty_items :: :: call
    let mut emitted = std.collections.list.empty[arcana_text.types.PlaceholderSpec] :: :: call
    for token in tokens:
        let mut cursor = token.range.start
        while cursor < token.range.end:
            let starting = arcana_text.shape.tokens.placeholder_starting :: buffer, cursor :: call
            if starting :: :: is_some:
                let spec = starting :: (arcana_text.shape.tokens.fallback_placeholder :: cursor :: call) :: unwrap_or
                arcana_text.shape.tokens.push_unseen_placeholder :: items, emitted, spec :: call
                if spec.range.end > cursor:
                    cursor = spec.range.end
                    continue
            let covering = arcana_text.shape.tokens.placeholder_covering :: buffer, cursor :: call
            if covering :: :: is_some:
                let spec = covering :: (arcana_text.shape.tokens.fallback_placeholder :: cursor :: call) :: unwrap_or
                let spec_end = spec.range.end
                arcana_text.shape.tokens.push_unseen_placeholder :: items, emitted, spec :: call
                if spec_end <= cursor:
                    break
                cursor = spec_end
                continue
            let next = arcana_text.shape.tokens.next_item_boundary :: buffer, token, cursor :: call
            if next <= cursor:
                break
            let mut payload = arcana_text.shape.tokens.TextItemEmitRequest :: buffer = buffer, token = token, start = cursor :: call
            payload.end = next
            arcana_text.shape.tokens.emit_text_item :: items, payload :: call
            cursor = next
        let trailing = arcana_text.shape.tokens.placeholder_starting :: buffer, token.range.end :: call
        if trailing :: :: is_some:
            let spec = trailing :: (arcana_text.shape.tokens.fallback_placeholder :: token.range.end :: call) :: unwrap_or
            arcana_text.shape.tokens.push_unseen_placeholder :: items, emitted, spec :: call
    let total = std.text.len_bytes :: buffer.text :: call
    let trailing = arcana_text.shape.tokens.placeholder_starting :: buffer, total :: call
    if trailing :: :: is_some:
        let spec = trailing :: (arcana_text.shape.tokens.fallback_placeholder :: total :: call) :: unwrap_or
        arcana_text.shape.tokens.push_unseen_placeholder :: items, emitted, spec :: call
    for spec in buffer.placeholders:
        arcana_text.shape.tokens.push_unseen_placeholder :: items, emitted, spec :: call
    return items

export fn collect_lines(read buffer: arcana_text.buffer.TextBuffer) -> List[arcana_text.shape.tokens.ShapeLine]:
    let items = arcana_text.shape.tokens.collect_items :: buffer :: call
    let total = std.text.len_bytes :: buffer.text :: call
    let mut out = arcana_text.shape.tokens.empty_lines :: :: call
    let mut current = arcana_text.shape.tokens.empty_items :: :: call
    let mut line_start = 0
    let mut line_end = 0
    for item in items:
        if item.newline:
            let mut line = arcana_text.shape.tokens.ShapeLine :: range = (arcana_text.types.TextRange :: start = line_start, end = item.range.start :: call), text = (std.text.slice_bytes :: buffer.text, line_start, item.range.start :: call), items = current :: call
            line.hard_break = true
            line.break_range = item.range
            out :: line :: push
            current = arcana_text.shape.tokens.empty_items :: :: call
            line_start = item.range.end
            line_end = line_start
            continue
        let item_end = item.range.end
        current :: item :: push
        if item_end > line_end:
            line_end = item_end
    if not (current :: :: is_empty) or line_start < total or (out :: :: is_empty):
        let break_range = arcana_text.types.TextRange :: start = line_end, end = line_end :: call
        let mut line = arcana_text.shape.tokens.ShapeLine :: range = (arcana_text.types.TextRange :: start = line_start, end = line_end :: call), text = (std.text.slice_bytes :: buffer.text, line_start, line_end :: call), items = current :: call
        line.hard_break = false
        line.break_range = break_range
        out :: line :: push
    return out
