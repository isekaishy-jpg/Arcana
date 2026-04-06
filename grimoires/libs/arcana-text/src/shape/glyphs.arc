import arcana_text.font_leaf
import arcana_text.fonts
import arcana_text.shape.styles
import arcana_text.shape.tokens
import arcana_text.shape.types
import std.bytes
import std.collections.array
import std.collections.list
import std.option
import std.text
use std.option.Option

export record AppendGlyphRequest:
    text: Str
    range: arcana_text.types.TextRange

record MatchCharRequest:
    primary: arcana_text.types.FontMatch
    style: arcana_text.types.TextStyle
    text: Str

record ShapeGlyphRequest:
    style: arcana_text.types.TextStyle
    primary: arcana_text.types.FontMatch
    glyph: arcana_text.shape.glyphs.AppendGlyphRequest

record PlanKeyRequest:
    matched: arcana_text.types.FontMatch
    style: arcana_text.types.TextStyle
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection

export record PreparedRun:
    run: arcana_text.types.ShapedRun
    unresolved: List[arcana_text.types.UnresolvedGlyph]

record PreparedGlyph:
    glyph: arcana_text.types.ShapedGlyph
    unresolved: Bool

fn empty_glyphs() -> List[arcana_text.types.ShapedGlyph]:
    return std.collections.list.empty[arcana_text.types.ShapedGlyph] :: :: call

fn empty_bitmap(read style: arcana_text.types.TextStyle) -> arcana_text.font_leaf.GlyphBitmap:
    let mut bitmap = arcana_text.font_leaf.GlyphBitmap :: size = (0, 0), offset = (0, 0), advance = (arcana_text.shape.types.max_int :: style.size, 1 :: call) :: call
    bitmap.baseline = arcana_text.shape.types.max_int :: style.size, 1 :: call
    bitmap.line_height = arcana_text.shape.types.max_int :: (style.size + 6), style.size :: call
    bitmap.empty = true
    bitmap.alpha = std.collections.array.empty[Int] :: :: call
    return bitmap

fn script_for_codepoint(codepoint: Int) -> arcana_text.types.ScriptClass:
    if codepoint >= 1424 and codepoint <= 1535:
        return arcana_text.types.ScriptClass.Hebrew :: :: call
    if codepoint >= 1536 and codepoint <= 1791:
        return arcana_text.types.ScriptClass.Arabic :: :: call
    if codepoint >= 2304 and codepoint <= 2431:
        return arcana_text.types.ScriptClass.Devanagari :: :: call
    if codepoint >= 12352 and codepoint <= 12543:
        return arcana_text.types.ScriptClass.Han :: :: call
    if codepoint >= 19968 and codepoint <= 40959:
        return arcana_text.types.ScriptClass.Han :: :: call
    if codepoint >= 44032 and codepoint <= 55215:
        return arcana_text.types.ScriptClass.Hangul :: :: call
    if codepoint >= 1024 and codepoint <= 1279:
        return arcana_text.types.ScriptClass.Cyrillic :: :: call
    if codepoint >= 0 and codepoint <= 591:
        return arcana_text.types.ScriptClass.Latin :: :: call
    return arcana_text.types.ScriptClass.Common :: :: call

fn script_for_text(read text: Str) -> arcana_text.types.ScriptClass:
    let codepoint = arcana_text.fonts.utf8_codepoint :: text :: call
    return script_for_codepoint :: codepoint :: call

fn direction_for_script(read script: arcana_text.types.ScriptClass) -> arcana_text.types.TextDirection:
    return match script:
        arcana_text.types.ScriptClass.Arabic => arcana_text.types.TextDirection.RightToLeft :: :: call
        arcana_text.types.ScriptClass.Hebrew => arcana_text.types.TextDirection.RightToLeft :: :: call
        _ => arcana_text.types.TextDirection.LeftToRight :: :: call

fn plan_key(read request: arcana_text.shape.glyphs.PlanKeyRequest) -> arcana_text.types.ShapePlanKey:
    let matched = request.matched
    let style = request.style
    let script = request.script
    let direction = request.direction
    let traits = arcana_text.fonts.style_traits_for :: style :: call
    let mut key = arcana_text.types.ShapePlanKey :: face_id = matched.id, direction = direction, script = script :: call
    key.language_tag = ""
    key.font_size = style.size
    key.weight = traits.weight
    key.width_milli = traits.width_milli
    key.slant_milli = traits.slant_milli
    key.feature_signature = arcana_text.types.feature_signature :: style.features :: call
    key.axis_signature = arcana_text.types.axis_signature :: style.axes :: call
    return key

fn invalid_match() -> arcana_text.types.FontMatch:
    let mut source = arcana_text.types.FontSource :: kind = (arcana_text.types.FontSourceKind.Bytes :: :: call), label = "", path = "" :: call
    source.family = ""
    source.face = ""
    source.full_name = ""
    source.postscript_name = ""
    source.installed = false
    source.bytes = std.collections.array.empty[Int] :: :: call
    return arcana_text.types.FontMatch :: id = (arcana_text.types.FontFaceId :: source_index = -1, face_index = -1 :: call), source = source :: call

fn primary_match(edit fonts: arcana_text.fonts.FontSystem, read style: arcana_text.types.TextStyle, read text: Str) -> arcana_text.types.FontMatch:
    return fonts :: style, (arcana_text.shape.tokens.first_visible_char :: text :: call) :: resolve_style_char

fn match_for_char(edit fonts: arcana_text.fonts.FontSystem, read payload: arcana_text.shape.glyphs.MatchCharRequest) -> arcana_text.types.FontMatch:
    let primary = payload.primary
    let style = payload.style
    let ch = payload.text
    if ch == " " or ch == "\t" or ch == "\n" or ch == "\r":
        return primary
    if primary.id.source_index >= 0:
        let glyph_index = fonts :: primary, ch :: glyph_index
        if glyph_index > 0:
            return primary
    return fonts :: style, ch :: resolve_style_char

fn shape_glyph(edit fonts: arcana_text.fonts.FontSystem, read payload: arcana_text.shape.glyphs.ShapeGlyphRequest) -> arcana_text.shape.glyphs.PreparedGlyph:
    let style = payload.style
    let primary = payload.primary
    let request = payload.glyph
    let matched = match_for_char :: fonts, (arcana_text.shape.glyphs.MatchCharRequest :: primary = primary, style = style, text = request.text :: call) :: call
    let traits = arcana_text.fonts.style_traits_for :: style :: call
    let line_height_milli = arcana_text.fonts.style_line_height_milli :: style :: call
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: request.text, style.size, line_height_milli :: call
    spec.traits = traits
    spec.feature_signature = arcana_text.types.feature_signature :: style.features :: call
    spec.axis_signature = arcana_text.types.axis_signature :: style.axes :: call
    let mut glyph_index = -1
    if matched.id.source_index >= 0:
        glyph_index = fonts :: matched, request.text :: glyph_index
    spec.glyph_index = glyph_index
    let mut bitmap = empty_bitmap :: style :: call
    if matched.id.source_index >= 0:
        bitmap = fonts :: matched.id, spec :: measure_face_glyph
    let mut family = ""
    if matched.id.source_index >= 0:
        family = arcana_text.fonts.family_or_label :: matched.source :: call
    let mut glyph = arcana_text.types.ShapedGlyph :: glyph = request.text, range = request.range, family = family :: call
    glyph.face_id = matched.id
    glyph.glyph_index = glyph_index
    glyph.cluster_range = request.range
    glyph.font_size = style.size
    glyph.line_height_milli = line_height_milli
    glyph.weight = traits.weight
    glyph.width_milli = traits.width_milli
    glyph.slant_milli = traits.slant_milli
    glyph.advance = bitmap.advance + style.letter_spacing
    glyph.x_advance = glyph.advance
    glyph.y_advance = 0
    glyph.offset = (0, 0)
    glyph.ink_offset = bitmap.offset
    glyph.ink_size = bitmap.size
    glyph.baseline = bitmap.baseline
    glyph.line_height = bitmap.line_height
    glyph.caret_stop_before = true
    glyph.caret_stop_after = true
    glyph.empty = bitmap.empty
    let unresolved = glyph_index <= 0 and request.text != " " and request.text != "\t" and request.text != "\r"
    return arcana_text.shape.glyphs.PreparedGlyph :: glyph = glyph, unresolved = unresolved :: call

export fn shape_placeholder(read span_style: arcana_text.types.SpanStyle, read spec: arcana_text.types.PlaceholderSpec) -> arcana_text.shape.glyphs.PreparedRun:
    let style = arcana_text.shape.styles.text_style_from_span :: span_style :: call
    let direction = arcana_text.types.TextDirection.LeftToRight :: :: call
    let script = arcana_text.types.ScriptClass.Common :: :: call
    let empty_match = invalid_match :: :: call
    let mut key_request = arcana_text.shape.glyphs.PlanKeyRequest :: matched = empty_match, style = style, script = script :: call
    key_request.direction = direction
    let key = arcana_text.shape.glyphs.plan_key :: key_request :: call
    let mut run = arcana_text.types.ShapedRun :: kind = (arcana_text.types.ShapedRunKind.Placeholder :: :: call), range = spec.range, text = "" :: call
    run.style = span_style
    run.direction = direction
    run.script = script
    run.bidi_level = 0
    run.language_tag = ""
    run.plan_key = key
    run.match = empty_match
    run.glyphs = empty_glyphs :: :: call
    run.width = spec.size.0
    run.whitespace = false
    run.hard_break = false
    run.placeholder = Option.Some[arcana_text.types.PlaceholderSpec] :: spec :: call
    let mut glyph = arcana_text.types.ShapedGlyph :: glyph = "", range = spec.range, family = "" :: call
    glyph.cluster_range = spec.range
    glyph.face_id = empty_match.id
    glyph.glyph_index = 0
    glyph.font_size = style.size
    glyph.line_height_milli = arcana_text.fonts.style_line_height_milli :: style :: call
    glyph.weight = 0
    glyph.width_milli = 0
    glyph.slant_milli = 0
    glyph.advance = spec.size.0
    glyph.x_advance = spec.size.0
    glyph.y_advance = 0
    glyph.offset = (0, 0)
    glyph.ink_offset = (0, 0)
    glyph.ink_size = spec.size
    glyph.baseline = spec.baseline_offset
    glyph.line_height = spec.size.1
    glyph.caret_stop_before = true
    glyph.caret_stop_after = true
    glyph.empty = true
    run.glyphs :: glyph :: push
    return arcana_text.shape.glyphs.PreparedRun :: run = run, unresolved = (arcana_text.shape.types.empty_unresolved :: :: call) :: call

export fn shape_inline(edit fonts: arcana_text.fonts.FontSystem, read style: arcana_text.types.SpanStyle, read payload: (Str, Int)) -> arcana_text.types.ShapedRun:
    let text = payload.0
    let start = payload.1
    let token = arcana_text.shape.tokens.text_token :: text, start, (start + (std.text.len_bytes :: text :: call)) :: call
    let prepared = shape_token :: fonts, token, style :: call
    return prepared.run

export fn shape_token(edit fonts: arcana_text.fonts.FontSystem, read token: arcana_text.shape.tokens.TextToken, read span_style: arcana_text.types.SpanStyle) -> arcana_text.shape.glyphs.PreparedRun:
    let style = arcana_text.shape.styles.text_style_from_span :: span_style :: call
    let primary = primary_match :: fonts, style, token.text :: call
    let script = script_for_text :: token.text :: call
    let direction = direction_for_script :: script :: call
    let mut key_request = arcana_text.shape.glyphs.PlanKeyRequest :: matched = primary, style = style, script = script :: call
    key_request.direction = direction
    let key = arcana_text.shape.glyphs.plan_key :: key_request :: call
    let mut run = arcana_text.types.ShapedRun :: kind = (arcana_text.types.ShapedRunKind.Text :: :: call), range = token.range, text = token.text :: call
    run.style = span_style
    run.direction = direction
    run.script = script
    run.bidi_level = 0
    if direction == (arcana_text.types.TextDirection.RightToLeft :: :: call):
        run.bidi_level = 1
    run.language_tag = ""
    run.plan_key = key
    run.match = primary
    run.glyphs = empty_glyphs :: :: call
    run.width = 0
    run.whitespace = token.whitespace
    run.hard_break = token.newline
    run.placeholder = Option.None[arcana_text.types.PlaceholderSpec] :: :: call
    let mut unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    if token.newline:
        return arcana_text.shape.glyphs.PreparedRun :: run = run, unresolved = unresolved :: call
    let bytes = std.bytes.from_str_utf8 :: token.text :: call
    let total = std.bytes.len :: bytes :: call
    let mut index = 0
    while index < total:
        let first = std.bytes.at :: bytes, index :: call
        let mut count = arcana_text.shape.tokens.utf8_char_len :: first :: call
        if index + count > total:
            count = 1
        let slice = std.bytes.slice :: bytes, index, index + count :: call
        let ch = std.bytes.to_str_utf8 :: slice :: call
        let request = arcana_text.shape.glyphs.AppendGlyphRequest :: text = ch, range = (arcana_text.types.TextRange :: start = token.range.start + index, end = token.range.start + index + count :: call) :: call
        let prepared = shape_glyph :: fonts, (arcana_text.shape.glyphs.ShapeGlyphRequest :: style = style, primary = primary, glyph = request :: call) :: call
        let glyph = prepared.glyph
        let glyph_start = glyph.range.start
        let glyph_text = glyph.glyph
        run.width += glyph.advance
        if run.match.id.source_index < 0 and glyph.face_id.source_index >= 0:
            run.match = arcana_text.types.FontMatch :: id = glyph.face_id, source = (fonts :: glyph.face_id.source_index :: source_at) :: call
        run.glyphs :: glyph :: push
        if prepared.unresolved:
            unresolved :: (arcana_text.types.UnresolvedGlyph :: index = glyph_start, glyph = glyph_text, reason = "missing glyph" :: call) :: push
        index += count
    return arcana_text.shape.glyphs.PreparedRun :: run = run, unresolved = unresolved :: call
