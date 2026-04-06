import arcana_text.buffer
import arcana_text.types
import std.bytes
import std.collections.list
import std.option
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

fn empty_tokens() -> List[arcana_text.shape.tokens.TextToken]:
    return std.collections.list.empty[arcana_text.shape.tokens.TextToken] :: :: call

fn empty_items() -> List[arcana_text.shape.tokens.ShapeItem]:
    return std.collections.list.empty[arcana_text.shape.tokens.ShapeItem] :: :: call

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
    return text == "\n"

export fn is_space(read text: Str) -> Bool:
    return text == " " or text == "\t" or text == "\r"

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

export fn tokenize_text(read text: Str) -> List[arcana_text.shape.tokens.TextToken]:
    let bytes = std.bytes.from_str_utf8 :: text :: call
    let total = std.bytes.len :: bytes :: call
    let mut out = empty_tokens :: :: call
    let mut index = 0
    let mut current = ""
    let mut current_start = 0
    let mut current_whitespace = false
    let mut active = false
    while index < total:
        let first = std.bytes.at :: bytes, index :: call
        let mut count = arcana_text.shape.tokens.utf8_char_len :: first :: call
        if index + count > total:
            count = 1
        let slice = std.bytes.slice :: bytes, index, index + count :: call
        let ch = std.bytes.to_str_utf8 :: slice :: call
        if arcana_text.shape.tokens.is_newline :: ch :: call:
            if active:
                let mut token = arcana_text.shape.tokens.text_token :: current, current_start, index :: call
                token.whitespace = current_whitespace
                out :: token :: push
                current = ""
                active = false
            let mut newline = arcana_text.shape.tokens.text_token :: ch, index, index + count :: call
            newline.newline = true
            out :: newline :: push
            index += count
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
    let bytes = std.bytes.from_str_utf8 :: text :: call
    let total = std.bytes.len :: bytes :: call
    let mut index = 0
    while index < total:
        let first = std.bytes.at :: bytes, index :: call
        let mut count = arcana_text.shape.tokens.utf8_char_len :: first :: call
        if index + count > total:
            count = 1
        let slice = std.bytes.slice :: bytes, index, index + count :: call
        let ch = std.bytes.to_str_utf8 :: slice :: call
        if not (arcana_text.shape.tokens.is_space :: ch :: call) and not (arcana_text.shape.tokens.is_newline :: ch :: call):
            return ch
        index += count
    return "M"

export fn collect_items(read buffer: arcana_text.buffer.TextBuffer) -> List[arcana_text.shape.tokens.ShapeItem]:
    let tokens = arcana_text.shape.tokens.tokenize_text :: buffer.text :: call
    let mut items = empty_items :: :: call
    let mut emitted = std.collections.list.empty[arcana_text.types.PlaceholderSpec] :: :: call
    for token in tokens:
        for spec in buffer.placeholders:
            if spec.range.start > token.range.start:
                continue
            if arcana_text.shape.tokens.placeholder_seen :: emitted, spec :: call:
                continue
            items :: (arcana_text.shape.tokens.placeholder_item :: spec :: call) :: push
            emitted :: spec :: push
        items :: (arcana_text.shape.tokens.text_item :: token :: call) :: push
    for spec in buffer.placeholders:
        if arcana_text.shape.tokens.placeholder_seen :: emitted, spec :: call:
            continue
        items :: (arcana_text.shape.tokens.placeholder_item :: spec :: call) :: push
    return items
