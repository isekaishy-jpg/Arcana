import arcana_text.font_leaf
import arcana_text.fonts
import arcana_text.shape.cache
import arcana_text.shape.styles
import arcana_text.shape.tokens
import arcana_text.shape.types
import arcana_text.text_units
import std.text
import std.collections.array
import std.collections.list
import arcana_process.fs
import std.option
import arcana_process.path
import std.text
use std.option.Option

fn shape_probe_flag_path() -> Str:
    return arcana_process.path.join :: (arcana_process.path.join :: (arcana_process.path.cwd :: :: call), "scratch" :: call), "enable_text_fonts_probe" :: call

fn shape_probe_log_path() -> Str:
    return arcana_process.path.join :: (arcana_process.path.join :: (arcana_process.path.cwd :: :: call), "scratch" :: call), "text_shape_probe.log" :: call

fn shape_probe_enabled() -> Bool:
    return arcana_process.fs.is_file :: (shape_probe_flag_path :: :: call) :: call

fn shape_probe_append(line: Str):
    if not (shape_probe_enabled :: :: call):
        return
    let _ = arcana_process.fs.mkdir_all :: (arcana_process.path.parent :: (shape_probe_log_path :: :: call) :: call) :: call
    let opened = arcana_process.fs.stream_open_write :: (shape_probe_log_path :: :: call), true :: call
    return match opened:
        std.result.Result.Ok(value) => shape_probe_append_ready :: value, line :: call
        std.result.Result.Err(_) => 0

fn shape_probe_append_ready(take value: arcana_winapi.process_handles.FileStream, line: Str):
    let mut stream = value
    let bytes = std.text.bytes_from_str_utf8 :: (line + "\n") :: call
    let _ = arcana_process.fs.stream_write :: stream, bytes :: call
    let _ = arcana_process.fs.stream_close :: stream :: call

export record AppendGlyphRequest:
    text: Str
    range: arcana_text.types.TextRange

record MatchTextRequest:
    primary: arcana_text.types.FontMatch
    style: arcana_text.types.TextStyle
    text: Str

record ShapeGlyphRequest:
    style: arcana_text.types.TextStyle
    primary: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    glyph: arcana_text.shape.glyphs.AppendGlyphRequest

record ShapeGlyphWithMatchRequest:
    style: arcana_text.types.TextStyle
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    glyph: arcana_text.shape.glyphs.AppendGlyphRequest

record PlanKeyRequest:
    matched: arcana_text.types.FontMatch
    style: arcana_text.types.TextStyle
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    language_tag: Str

record RestyledGlyphRequest:
    style: arcana_text.types.TextStyle
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    glyphs: List[arcana_text.types.ShapedGlyph]
    start_index: Int
    consumed: Int
    glyph_index: Int
    direction: arcana_text.types.TextDirection

record FinalizeActiveRunRequest:
    run: arcana_text.types.ShapedRun
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    style: arcana_text.types.TextStyle

record AppendClusterToRunRequest:
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    cluster: arcana_text.shape.glyphs.ClusterSeed
    style: arcana_text.types.TextStyle

record RunSeed:
    span_style: arcana_text.types.SpanStyle
    style: arcana_text.types.TextStyle
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    language_tag: Str
    bidi_level: Int
    start: Int
    whitespace: Bool

record RunShapeKey:
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    language_tag: Str
    bidi_level: Int

record ClusterSeed:
    text: Str
    range: arcana_text.types.TextRange
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    language_tag: Str
    bidi_level: Int

record BidiScalar:
    start: Int
    end: Int
    class: Str
    level: Int

record BidiFrame:
    level: Int
    override: Str
    isolate: Bool

record BidiBracketOpen:
    index: Int
    codepoint: Int
    level: Int

record BidiBracketMatch:
    start_index: Int
    end_index: Int
    resolved_class: Str

record BidiParagraphRange:
    start: Int
    end: Int
    base_level: Int

record ResolvedCluster:
    text: Str
    range: arcana_text.types.TextRange
    direction: arcana_text.types.TextDirection
    bidi_level: Int

export record ResolvedLine:
    text: Str
    range: arcana_text.types.TextRange
    scalars: List[arcana_text.shape.glyphs.BidiScalar]
    paragraph_level: Int
    signature: Int

record BidiNeutralRunRequest:
    source: List[arcana_text.shape.glyphs.BidiScalar]
    scalars: List[arcana_text.shape.glyphs.BidiScalar]
    start: Int
    end: Int
    paragraph_class: Str

record BidiClusterInfoRequest:
    text: Str
    start: Int
    end: Int
    scalars: List[arcana_text.shape.glyphs.BidiScalar]

record ShapeTokenRunsInLineRequest:
    token: arcana_text.shape.tokens.TextToken
    span_style: arcana_text.types.SpanStyle
    line: arcana_text.shape.glyphs.ResolvedLine

record SingleFeatureSubstituteRequest:
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    language_tag: Str
    feature: arcana_text.types.FontFeature
    glyph_index: Int

record ArabicJoinRequest:
    seeds: List[arcana_text.shape.glyphs.GlyphSeed]
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    units: List[arcana_text.font_leaf.GsubGlyphUnit]

record PreparedGlyph:
    glyph: arcana_text.types.ShapedGlyph
    match: arcana_text.types.FontMatch
    unresolved: Bool

record GlyphSeed:
    text: Str
    range: arcana_text.types.TextRange
    cluster_range: arcana_text.types.TextRange
    glyph_index: Int
    caret_stop_before: Bool
    caret_stop_after: Bool
    unresolved: Bool

record SeededRun:
    glyphs: List[arcana_text.shape.glyphs.GlyphSeed]
    unresolved: List[arcana_text.types.UnresolvedGlyph]

record FinalizeSegmentRequest:
    run: arcana_text.types.ShapedRun
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    style: arcana_text.types.TextStyle
    reshaped: Bool

record FallbackMatchRequest:
    style: arcana_text.types.TextStyle
    cluster: arcana_text.shape.glyphs.ResolvedCluster
    primary: arcana_text.types.FontMatch
    script_tag: Str

record SeedGlyphRequest:
    style: arcana_text.types.TextStyle
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    text: Str
    range: arcana_text.types.TextRange
    cluster_range: arcana_text.types.TextRange
    caret_stop_before: Bool
    caret_stop_after: Bool

record SeedGlyphBuildRequest:
    style: arcana_text.types.TextStyle
    matched: arcana_text.types.FontMatch
    script: arcana_text.types.ScriptClass
    direction: arcana_text.types.TextDirection
    start_index: Int
    consumed: Int
    glyph_index: Int

record SinglePositionRequest:
    script: arcana_text.types.ScriptClass
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    glyph: arcana_text.types.ShapedGlyph

record PairPositionRequest:
    script: arcana_text.types.ScriptClass
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    previous: List[arcana_text.types.ShapedGlyph]
    right: arcana_text.types.ShapedGlyph

record PositionLookupStep:
    lookup: arcana_text.font_leaf.GsubLookupRef
    lookup_type: Int

record ShapeRunFromUnitsRequest:
    run: arcana_text.types.ShapedRun
    style: arcana_text.types.TextStyle
    seeds: List[arcana_text.shape.glyphs.GlyphSeed]
    units: List[arcana_text.font_leaf.GsubGlyphUnit]

fn empty_glyphs() -> List[arcana_text.types.ShapedGlyph]:
    return std.collections.list.empty[arcana_text.types.ShapedGlyph] :: :: call

fn copy_features(read values: List[arcana_text.types.FontFeature]) -> List[arcana_text.types.FontFeature]:
    let mut out = std.collections.list.empty[arcana_text.types.FontFeature] :: :: call
    out :: values :: extend_list
    return out

fn copy_shaped_glyphs(read values: List[arcana_text.types.ShapedGlyph]) -> List[arcana_text.types.ShapedGlyph]:
    let mut out = std.collections.list.empty[arcana_text.types.ShapedGlyph] :: :: call
    out :: values :: extend_list
    return out

fn empty_position_lookup_steps() -> List[arcana_text.shape.glyphs.PositionLookupStep]:
    return std.collections.list.empty[arcana_text.shape.glyphs.PositionLookupStep] :: :: call

fn copy_seed_glyphs(read values: List[arcana_text.shape.glyphs.GlyphSeed]) -> List[arcana_text.shape.glyphs.GlyphSeed]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.GlyphSeed] :: :: call
    out :: values :: extend_list
    return out

export fn empty_prepared_runs() -> List[arcana_text.types.PreparedRun]:
    return std.collections.list.empty[arcana_text.types.PreparedRun] :: :: call

fn shift_range(read range: arcana_text.types.TextRange, delta: Int) -> arcana_text.types.TextRange:
    return arcana_text.types.TextRange :: start = range.start + delta, end = range.end + delta :: call

fn shift_unresolved(read unresolved: List[arcana_text.types.UnresolvedGlyph], delta: Int) -> List[arcana_text.types.UnresolvedGlyph]:
    let mut out = arcana_text.shape.types.empty_unresolved :: :: call
    for value in unresolved:
        let mut next = value
        next.index = next.index + delta
        out :: next :: push
    return out

fn shift_shaped_glyph(read glyph: arcana_text.types.ShapedGlyph, delta: Int) -> arcana_text.types.ShapedGlyph:
    let mut next = glyph
    next.range = arcana_text.shape.glyphs.shift_range :: glyph.range, delta :: call
    next.cluster_range = arcana_text.shape.glyphs.shift_range :: glyph.cluster_range, delta :: call
    return next

fn shift_shaped_glyphs(read glyphs: List[arcana_text.types.ShapedGlyph], delta: Int) -> List[arcana_text.types.ShapedGlyph]:
    let mut out = std.collections.list.empty[arcana_text.types.ShapedGlyph] :: :: call
    for glyph in glyphs:
        out :: (arcana_text.shape.glyphs.shift_shaped_glyph :: glyph, delta :: call) :: push
    return out

fn shift_shaped_run(read run: arcana_text.types.ShapedRun, delta: Int) -> arcana_text.types.ShapedRun:
    let mut next = run
    next.range = arcana_text.shape.glyphs.shift_range :: run.range, delta :: call
    next.glyphs = arcana_text.shape.glyphs.shift_shaped_glyphs :: run.glyphs, delta :: call
    return next

fn shift_prepared_run(read prepared: arcana_text.types.PreparedRun, delta: Int) -> arcana_text.types.PreparedRun:
    return arcana_text.types.PreparedRun :: run = (arcana_text.shape.glyphs.shift_shaped_run :: prepared.run, delta :: call), unresolved = (arcana_text.shape.glyphs.shift_unresolved :: prepared.unresolved, delta :: call) :: call

fn shift_prepared_runs(read runs: List[arcana_text.types.PreparedRun], delta: Int) -> List[arcana_text.types.PreparedRun]:
    let mut out = arcana_text.shape.glyphs.empty_prepared_runs :: :: call
    for prepared in runs:
        out :: (arcana_text.shape.glyphs.shift_prepared_run :: prepared, delta :: call) :: push
    return out

fn empty_bitmap(read style: arcana_text.types.TextStyle) -> arcana_text.font_leaf.GlyphBitmap:
    let mut bitmap = arcana_text.font_leaf.GlyphBitmap :: size = (0, 0), offset = (0, 0), advance = (arcana_text.shape.types.max_int :: style.size, 1 :: call) :: call
    bitmap.baseline = arcana_text.shape.types.max_int :: style.size, 1 :: call
    bitmap.line_height = arcana_text.shape.types.max_int :: (style.size + 6), style.size :: call
    bitmap.empty = true
    bitmap.alpha = std.collections.array.empty[Int] :: :: call
    bitmap.lcd = std.collections.array.empty[Int] :: :: call
    bitmap.rgba = std.collections.array.empty[Int] :: :: call
    return bitmap

fn empty_bidi_scalar() -> arcana_text.shape.glyphs.BidiScalar:
    let mut scalar = arcana_text.shape.glyphs.BidiScalar :: start = 0, end = 0, class = "L" :: call
    scalar.level = 0
    return scalar

fn empty_bidi_frame() -> arcana_text.shape.glyphs.BidiFrame:
    let mut frame = arcana_text.shape.glyphs.BidiFrame :: level = 0, override = "" :: call
    frame.isolate = false
    return frame

fn empty_bidi_bracket_opens() -> List[arcana_text.shape.glyphs.BidiBracketOpen]:
    return std.collections.list.empty[arcana_text.shape.glyphs.BidiBracketOpen] :: :: call

fn empty_bidi_bracket_matches() -> List[arcana_text.shape.glyphs.BidiBracketMatch]:
    return std.collections.list.empty[arcana_text.shape.glyphs.BidiBracketMatch] :: :: call

fn empty_bidi_paragraph_ranges() -> List[arcana_text.shape.glyphs.BidiParagraphRange]:
    return std.collections.list.empty[arcana_text.shape.glyphs.BidiParagraphRange] :: :: call

fn bidi_scalar_slice(read scalars: List[arcana_text.shape.glyphs.BidiScalar], start: Int, end: Int) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let total = scalars :: :: len
    let safe_start = clamp_int :: start, 0, total :: call
    let safe_end = clamp_int :: end, safe_start, total :: call
    let mut index = safe_start
    while index < safe_end:
        out :: (scalars)[index] :: push
        index += 1
    return out

fn utf8_scalar_from_codepoint(codepoint: Int) -> Str:
    if codepoint <= 0:
        return ""
    let mut buf = std.text.bytes_builder :: :: call
    if codepoint < 128:
        let _ = std.text.bytes_builder_push :: buf, codepoint :: call
        return std.text.bytes_to_str_utf8 :: (std.text.bytes_builder_freeze :: buf :: call) :: call
    if codepoint < 2048:
        let _ = std.text.bytes_builder_push :: buf, (192 + (codepoint / 64)) :: call
        let _ = std.text.bytes_builder_push :: buf, (128 + (codepoint % 64)) :: call
        return std.text.bytes_to_str_utf8 :: (std.text.bytes_builder_freeze :: buf :: call) :: call
    if codepoint < 65536:
        let _ = std.text.bytes_builder_push :: buf, (224 + (codepoint / 4096)) :: call
        let _ = std.text.bytes_builder_push :: buf, (128 + ((codepoint / 64) % 64)) :: call
        let _ = std.text.bytes_builder_push :: buf, (128 + (codepoint % 64)) :: call
        return std.text.bytes_to_str_utf8 :: (std.text.bytes_builder_freeze :: buf :: call) :: call
    let _ = std.text.bytes_builder_push :: buf, (240 + (codepoint / 262144)) :: call
    let _ = std.text.bytes_builder_push :: buf, (128 + ((codepoint / 4096) % 64)) :: call
    let _ = std.text.bytes_builder_push :: buf, (128 + ((codepoint / 64) % 64)) :: call
    let _ = std.text.bytes_builder_push :: buf, (128 + (codepoint % 64)) :: call
    return std.text.bytes_to_str_utf8 :: (std.text.bytes_builder_freeze :: buf :: call) :: call

fn mirrored_codepoint(codepoint: Int) -> Int:
    return match codepoint:
        40 => 41
        41 => 40
        60 => 62
        62 => 60
        91 => 93
        93 => 91
        123 => 125
        125 => 123
        171 => 187
        187 => 171
        8249 => 8250
        8250 => 8249
        8261 => 8262
        8262 => 8261
        12296 => 12297
        12297 => 12296
        12298 => 12299
        12299 => 12298
        12300 => 12301
        12301 => 12300
        12302 => 12303
        12303 => 12302
        12304 => 12305
        12305 => 12304
        12308 => 12309
        12309 => 12308
        12310 => 12311
        12311 => 12310
        12312 => 12313
        12313 => 12312
        12314 => 12315
        12315 => 12314
        _ => codepoint

fn bidi_open_bracket_for(codepoint: Int) -> Int:
    return match codepoint:
        40 => 41
        60 => 62
        91 => 93
        123 => 125
        171 => 187
        8249 => 8250
        12296 => 12297
        12298 => 12299
        12300 => 12301
        12302 => 12303
        12304 => 12305
        12308 => 12309
        12310 => 12311
        12312 => 12313
        12314 => 12315
        _ => 0

fn bidi_close_bracket_for(codepoint: Int) -> Int:
    return match codepoint:
        41 => 40
        62 => 60
        93 => 91
        125 => 123
        187 => 171
        8250 => 8249
        12297 => 12296
        12299 => 12298
        12301 => 12300
        12303 => 12302
        12305 => 12304
        12309 => 12308
        12311 => 12310
        12313 => 12312
        12315 => 12314
        _ => 0

fn bidi_text_is_simple_ascii(read text: Str) -> Bool:
    let bytes = std.text.bytes_from_str_utf8 :: text :: call
    let total = std.text.bytes_len :: bytes :: call
    let mut index = 0
    while index < total:
        let value = std.text.bytes_at :: bytes, index :: call
        if value >= 128:
            return false
        if value < 32 and value != 9 and value != 10 and value != 13:
            return false
        index += 1
    return true

fn bidi_ascii_class_for_byte(value: Int) -> Str:
    if value < 32 and value != 9 and value != 10 and value != 13:
        return "BN"
    if value == 10 or value == 13:
        return "B"
    if value == 9 or value == 11 or value == 12:
        return "S"
    if value == 32:
        return "WS"
    if value >= 48 and value <= 57:
        return "EN"
    if value == 43 or value == 45:
        return "ES"
    if value == 44 or value == 46 or value == 47 or value == 58:
        return "CS"
    if value == 35 or value == 36 or value == 37:
        return "ET"
    if (std.text.is_alpha_byte :: value :: call):
        return "L"
    return "ON"

fn bidi_ascii_scalars(read text: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let bytes = std.text.bytes_from_str_utf8 :: text :: call
    let total = std.text.bytes_len :: bytes :: call
    let mut index = 0
    while index < total:
        let value = std.text.bytes_at :: bytes, index :: call
        let end = match value == 13 and (index + 1) < total and (std.text.bytes_at :: bytes, index + 1 :: call) == 10:
            true => index + 2
            false => index + 1
        let mut scalar = arcana_text.shape.glyphs.BidiScalar :: start = index, end = end, class = (arcana_text.shape.glyphs.bidi_ascii_class_for_byte :: value :: call) :: call
        scalar.level = 0
        out :: scalar :: push
        index = end
    return out

fn bidi_format_class(codepoint: Int) -> Str:
    if codepoint == 8206:
        return "L"
    if codepoint == 8207:
        return "R"
    if codepoint == 8234:
        return "LRE"
    if codepoint == 8235:
        return "RLE"
    if codepoint == 8236:
        return "PDF"
    if codepoint == 8237:
        return "LRO"
    if codepoint == 8238:
        return "RLO"
    if codepoint == 8294:
        return "LRI"
    if codepoint == 8295:
        return "RLI"
    if codepoint == 8296:
        return "FSI"
    if codepoint == 8297:
        return "PDI"
    if arcana_text.text_units.is_format_control_codepoint :: codepoint :: call:
        return "BN"
    return ""

fn bidi_embedding_class(level: Int) -> Str:
    return match (level % 2) == 1:
        true => "R"
        false => "L"

fn bidi_class_for_codepoint(codepoint: Int) -> Str:
    if codepoint == 10 or codepoint == 13 or codepoint == 8232 or codepoint == 8233:
        return "B"
    if codepoint == 9 or codepoint == 11 or codepoint == 12 or codepoint == 133:
        return "S"
    let format_class = arcana_text.shape.glyphs.bidi_format_class :: codepoint :: call
    if format_class != "":
        return format_class
    if codepoint == 1564:
        return "AL"
    if arcana_text.text_units.is_spacing_or_separator_codepoint :: codepoint :: call:
        return "WS"
    if codepoint >= 48 and codepoint <= 57:
        return "EN"
    if (codepoint >= 1632 and codepoint <= 1641) or (codepoint >= 1776 and codepoint <= 1785):
        return "AN"
    if arcana_text.text_units.is_combining_mark :: codepoint :: call:
        return "NSM"
    if codepoint >= 1424 and codepoint <= 1535:
        return "R"
    if (codepoint >= 1536 and codepoint <= 1791) or (codepoint >= 1872 and codepoint <= 1919) or (codepoint >= 2208 and codepoint <= 2303) or (codepoint >= 64336 and codepoint <= 65023) or (codepoint >= 65136 and codepoint <= 65279):
        return "AL"
    if codepoint == 43 or codepoint == 45:
        return "ES"
    if codepoint == 44 or codepoint == 46 or codepoint == 58 or codepoint == 47:
        return "CS"
    if codepoint == 35 or codepoint == 36 or codepoint == 37:
        return "ET"
    if (codepoint >= 65 and codepoint <= 90) or (codepoint >= 97 and codepoint <= 122):
        return "L"
    let script_class = arcana_text.shape.glyphs.bidi_class_for_script :: (arcana_text.shape.glyphs.script_for_codepoint :: codepoint :: call) :: call
    if script_class != "":
        return script_class
    return "ON"

fn bidi_class_for_script(read script: arcana_text.types.ScriptClass) -> Str:
    return match script:
        arcana_text.types.ScriptClass.Hebrew => "R"
        arcana_text.types.ScriptClass.Adlam => "R"
        arcana_text.types.ScriptClass.Arabic => "AL"
        arcana_text.types.ScriptClass.Thaana => "AL"
        arcana_text.types.ScriptClass.Unknown => ""
        arcana_text.types.ScriptClass.Common => ""
        _ => "L"

fn bidi_is_isolate_initiator(read class: Str) -> Bool:
    return class == "LRI" or class == "RLI" or class == "FSI"

fn bidi_is_isolate_terminator(read class: Str) -> Bool:
    return class == "PDI"

fn bidi_is_segment_separator(read class: Str) -> Bool:
    return class == "B" or class == "S"

fn bidi_is_sequence_boundary(read class: Str) -> Bool:
    return (arcana_text.shape.glyphs.bidi_is_segment_separator :: class :: call) or (arcana_text.shape.glyphs.bidi_is_isolate_initiator :: class :: call) or (arcana_text.shape.glyphs.bidi_is_isolate_terminator :: class :: call)

fn bidi_is_strong(read class: Str) -> Bool:
    return class == "L" or class == "R" or class == "AL"

fn bidi_is_neutral(read class: Str) -> Bool:
    return class == "WS" or class == "ON" or class == "BN" or class == "ES" or class == "ET" or class == "CS" or class == "S" or class == "B"

fn bidi_surrounding_class(read class: Str) -> Str:
    if class == "L":
        return "L"
    if class == "R" or class == "AL" or class == "EN" or class == "AN":
        return "R"
    return ""

fn bidi_direction_from_class(read class: Str) -> arcana_text.types.TextDirection:
    if class == "R" or class == "AL":
        return arcana_text.types.TextDirection.RightToLeft :: :: call
    return arcana_text.types.TextDirection.LeftToRight :: :: call

fn bidi_direction_from_level(level: Int) -> arcana_text.types.TextDirection:
    if (level % 2) == 1:
        return arcana_text.types.TextDirection.RightToLeft :: :: call
    return arcana_text.types.TextDirection.LeftToRight :: :: call

fn bidi_scalar_is(read scalar: arcana_text.shape.glyphs.BidiScalar, read class: Str) -> Bool:
    return scalar.class == class

fn bidi_class_code(read class: Str) -> Int:
    if class == "BN":
        return 1
    if class == "B":
        return 2
    if class == "S":
        return 3
    if class == "R":
        return 4
    if class == "AN":
        return 5
    if class == "EN":
        return 6
    if class == "L":
        return 7
    if class == "NSM":
        return 8
    if class == "LRE":
        return 9
    if class == "RLE":
        return 10
    if class == "LRO":
        return 11
    if class == "RLO":
        return 12
    if class == "LRI":
        return 13
    if class == "RLI":
        return 14
    if class == "FSI":
        return 15
    if class == "PDF":
        return 16
    if class == "PDI":
        return 17
    return 0

fn bidi_next_even_level(level: Int) -> Int:
    let mut next = level + 1
    if (next % 2) == 1:
        next += 1
    return next

fn bidi_next_odd_level(level: Int) -> Int:
    let mut next = level + 1
    if (next % 2) == 0:
        next += 1
    return next

fn bidi_scalar_end(read text: Str, start: Int) -> Int:
    let end = arcana_text.text_units.next_scalar_end :: text, start :: call
    let codepoint = arcana_text.text_units.codepoint_at :: text, start :: call
    if codepoint == 13 and end < (std.text.len_bytes :: text :: call) and (arcana_text.text_units.codepoint_at :: text, end :: call) == 10:
        return arcana_text.text_units.next_scalar_end :: text, end :: call
    return end

fn bidi_scalars(read text: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let total = std.text.len_bytes :: text :: call
    let mut index = 0
    while index < total:
        let end = arcana_text.shape.glyphs.bidi_scalar_end :: text, index :: call
        let codepoint = arcana_text.text_units.codepoint_at :: text, index :: call
        let mut scalar = arcana_text.shape.glyphs.BidiScalar :: start = index, end = end, class = (arcana_text.shape.glyphs.bidi_class_for_codepoint :: codepoint :: call) :: call
        scalar.level = 0
        out :: scalar :: push
        index = end
    return out

fn bidi_base_level(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> Int:
    for scalar in scalars:
        if scalar.class == "L":
            return 0
        if scalar.class == "R" or scalar.class == "AL":
            return 1
    return 0

fn bidi_paragraph_ranges(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> List[arcana_text.shape.glyphs.BidiParagraphRange]:
    let mut out = arcana_text.shape.glyphs.empty_bidi_paragraph_ranges :: :: call
    let total = scalars :: :: len
    if total <= 0:
        return out
    let mut start = 0
    let mut index = 0
    while index < total:
        if (scalars)[index].class == "B":
            let end = index + 1
            let base_level = arcana_text.shape.glyphs.bidi_base_level :: (arcana_text.shape.glyphs.bidi_scalar_slice :: scalars, start, end :: call) :: call
            out :: (arcana_text.shape.glyphs.BidiParagraphRange :: start = start, end = end, base_level = base_level :: call) :: push
            start = end
        index += 1
    if start < total:
        let base_level = arcana_text.shape.glyphs.bidi_base_level :: (arcana_text.shape.glyphs.bidi_scalar_slice :: scalars, start, total :: call) :: call
        out :: (arcana_text.shape.glyphs.BidiParagraphRange :: start = start, end = total, base_level = base_level :: call) :: push
    return out

fn bidi_first_paragraph_level(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> Int:
    let paragraphs = arcana_text.shape.glyphs.bidi_paragraph_ranges :: scalars :: call
    if paragraphs :: :: is_empty:
        return 0
    return (paragraphs)[0].base_level

fn bidi_paragraph_class(base_level: Int) -> Str:
    if (base_level % 2) == 1:
        return "R"
    return "L"

fn bidi_resolve_nsm(read source: List[arcana_text.shape.glyphs.BidiScalar], read scalars: List[arcana_text.shape.glyphs.BidiScalar], read paragraph_class: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let mut previous = paragraph_class
    let total = scalars :: :: len
    let mut index = 0
    while index < total:
        let scalar = (scalars)[index]
        let source_class = (source)[index].class
        let mut next = scalar
        if next.class == "NSM":
            next.class = previous
        if arcana_text.shape.glyphs.bidi_is_sequence_boundary :: source_class :: call:
            previous = paragraph_class
        else:
            if next.class != "BN":
                previous = next.class
        out :: next :: push
        index += 1
    return out

fn bidi_resolve_fsi_direction(read scalars: List[arcana_text.shape.glyphs.BidiScalar], start: Int, read paragraph_class: Str) -> Str:
    let mut depth = 0
    let mut index = start + 1
    while index < (scalars :: :: len):
        let class = (scalars)[index].class
        if class == "LRI" or class == "RLI" or class == "FSI":
            depth += 1
        else:
            if class == "PDI":
                if depth <= 0:
                    break
                depth -= 1
            else:
                if depth <= 0:
                    if class == "L":
                        return "L"
                    if class == "R" or class == "AL":
                        return "R"
        index += 1
    return paragraph_class

fn bidi_resolve_explicit(read scalars: List[arcana_text.shape.glyphs.BidiScalar], base_level: Int, read paragraph_class: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let mut stack = std.collections.list.empty[arcana_text.shape.glyphs.BidiFrame] :: :: call
    let mut current_level = base_level
    let mut override_class = ""
    let mut index = 0
    while index < (scalars :: :: len):
        let scalar = (scalars)[index]
        let start = scalar.start
        let end = scalar.end
        let class = scalar.class
        let class_code = arcana_text.shape.glyphs.bidi_class_code :: class :: call
        if class_code == 2:
            let mut next = arcana_text.shape.glyphs.BidiScalar :: start = start, end = end, class = "B" :: call
            next.level = base_level
            out :: next :: push
            current_level = base_level
            override_class = ""
            stack = std.collections.list.empty[arcana_text.shape.glyphs.BidiFrame] :: :: call
        else:
            if class_code >= 9 and class_code <= 15:
                let mut next = arcana_text.shape.glyphs.BidiScalar :: start = start, end = end, class = "BN" :: call
                let mut frame = arcana_text.shape.glyphs.empty_bidi_frame :: :: call
                frame.level = current_level
                frame.override = override_class
                frame.isolate = class_code == 13 or class_code == 14 or class_code == 15
                stack :: frame :: push
                let mut resolved = "L"
                if class_code == 10 or class_code == 12 or class_code == 14:
                    resolved = "R"
                else:
                    if class_code == 15:
                        resolved = arcana_text.shape.glyphs.bidi_resolve_fsi_direction :: scalars, index, paragraph_class :: call
                if resolved == "R":
                    current_level = arcana_text.shape.glyphs.bidi_next_odd_level :: current_level :: call
                else:
                    current_level = arcana_text.shape.glyphs.bidi_next_even_level :: current_level :: call
                if class_code == 11:
                    override_class = "L"
                else:
                    if class_code == 12:
                        override_class = "R"
                    else:
                        override_class = ""
                next.level = current_level
                out :: next :: push
            else:
                if class_code == 16:
                    let mut next = arcana_text.shape.glyphs.BidiScalar :: start = start, end = end, class = "BN" :: call
                    if not (stack :: :: is_empty):
                        let top = stack :: :: pop
                        if not top.isolate:
                            current_level = top.level
                            override_class = top.override
                        else:
                            stack :: top :: push
                    next.level = current_level
                    out :: next :: push
                else:
                    if class_code == 17:
                        let mut next = arcana_text.shape.glyphs.BidiScalar :: start = start, end = end, class = "BN" :: call
                        while not (stack :: :: is_empty):
                            let top = stack :: :: pop
                            current_level = top.level
                            override_class = top.override
                            if top.isolate:
                                break
                        next.level = current_level
                        out :: next :: push
                    else:
                        let mut next = arcana_text.shape.glyphs.BidiScalar :: start = start, end = end, class = class :: call
                        next.level = current_level
                        if override_class != "" and class_code != 1 and class_code != 8:
                            next.class = override_class
                        out :: next :: push
        index += 1
    return out

fn bidi_resolve_w2(read scalars: List[arcana_text.shape.glyphs.BidiScalar], read paragraph_class: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let mut previous_strong = paragraph_class
    for scalar in scalars:
        let mut next = scalar
        if next.class == "EN" and previous_strong == "AL":
            next.class = "AN"
        if arcana_text.shape.glyphs.bidi_is_strong :: next.class :: call:
            previous_strong = next.class
        out :: next :: push
    return out

fn bidi_resolve_al(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    for scalar in scalars:
        let mut next = scalar
        if next.class == "AL":
            next.class = "R"
        out :: next :: push
    return out

fn bidi_scalar_class_or(read scalars: List[arcana_text.shape.glyphs.BidiScalar], index: Int, read fallback: Str) -> Str:
    if index < 0 or index >= (scalars :: :: len):
        return fallback
    return (scalars)[index].class

fn bidi_resolve_w4(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let total = scalars :: :: len
    let mut index = 0
    while index < total:
        let scalar = (scalars)[index]
        let mut next = scalar
        let prev_class = arcana_text.shape.glyphs.bidi_scalar_class_or :: scalars, index - 1, "" :: call
        let next_class = arcana_text.shape.glyphs.bidi_scalar_class_or :: scalars, index + 1, "" :: call
        if next.class == "ES" and prev_class == "EN" and next_class == "EN":
            next.class = "EN"
        if next.class == "CS" and prev_class == "EN" and next_class == "EN":
            next.class = "EN"
        if next.class == "CS" and prev_class == "AN" and next_class == "AN":
            next.class = "AN"
        out :: next :: push
        index += 1
    return out

fn bidi_resolve_w5(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let total = scalars :: :: len
    let mut index = 0
    while index < total:
        let scalar = (scalars)[index]
        if scalar.class != "ET":
            out :: scalar :: push
            index += 1
            continue
        let start = index
        while index < total and ((scalars)[index].class == "ET"):
            index += 1
        let left = arcana_text.shape.glyphs.bidi_scalar_class_or :: scalars, start - 1, "" :: call
        let right = arcana_text.shape.glyphs.bidi_scalar_class_or :: scalars, index, "" :: call
        let promote = left == "EN" or right == "EN"
        let mut fill = start
        while fill < index:
            let mut next = (scalars)[fill]
            if promote:
                next.class = "EN"
            out :: next :: push
            fill += 1
    return out

fn bidi_resolve_w6(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    for scalar in scalars:
        let mut next = scalar
        if next.class == "ES" or next.class == "ET" or next.class == "CS":
            next.class = "ON"
        out :: next :: push
    return out

fn bidi_resolve_w7(read scalars: List[arcana_text.shape.glyphs.BidiScalar], read paragraph_class: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let mut previous_strong = paragraph_class
    for scalar in scalars:
        let mut next = scalar
        if next.class == "EN" and previous_strong == "L":
            next.class = "L"
        if next.class == "L" or next.class == "R":
            previous_strong = next.class
        out :: next :: push
    return out

fn bidi_level_run_embedding(read scalars: List[arcana_text.shape.glyphs.BidiScalar], start: Int, read paragraph_class: Str) -> Str:
    if start < 0 or start >= (scalars :: :: len):
        return paragraph_class
    return arcana_text.shape.glyphs.bidi_embedding_class :: ((scalars)[start].level) :: call

fn bidi_level_run_class_left(read payload: (List[arcana_text.shape.glyphs.BidiScalar], List[arcana_text.shape.glyphs.BidiScalar], Int, Int, Str)) -> Str:
    let source = payload.0
    let scalars = payload.1
    let start = payload.2
    let level = payload.3
    let fallback = payload.4
    let mut index = start - 1
    while index >= 0:
        let source_class = (source)[index].class
        if arcana_text.shape.glyphs.bidi_is_sequence_boundary :: source_class :: call:
            return fallback
        let scalar = (scalars)[index]
        if scalar.level != level:
            return fallback
        let value = arcana_text.shape.glyphs.bidi_surrounding_class :: scalar.class :: call
        if value != "":
            return value
        index -= 1
    return fallback

fn bidi_level_run_class_right(read payload: (List[arcana_text.shape.glyphs.BidiScalar], List[arcana_text.shape.glyphs.BidiScalar], Int, Int, Str)) -> Str:
    let source = payload.0
    let scalars = payload.1
    let start = payload.2
    let level = payload.3
    let fallback = payload.4
    let total = scalars :: :: len
    let mut index = start
    while index < total:
        let source_class = (source)[index].class
        if arcana_text.shape.glyphs.bidi_is_sequence_boundary :: source_class :: call:
            return fallback
        let scalar = (scalars)[index]
        if scalar.level != level:
            return fallback
        let value = arcana_text.shape.glyphs.bidi_surrounding_class :: scalar.class :: call
        if value != "":
            return value
        index += 1
    return fallback

fn bidi_bracket_resolved_class(read payload: (List[arcana_text.shape.glyphs.BidiScalar], List[arcana_text.shape.glyphs.BidiScalar], Int, Int, Str)) -> Str:
    let source = payload.0
    let scalars = payload.1
    let start_index = payload.2
    let end_index = payload.3
    let paragraph_class = payload.4
    let embedding = arcana_text.shape.glyphs.bidi_level_run_embedding :: scalars, start_index, paragraph_class :: call
    let level = (scalars)[start_index].level
    let mut saw_embedding = false
    let mut first_strong = ""
    let mut index = start_index + 1
    while index < end_index:
        let source_class = (source)[index].class
        if arcana_text.shape.glyphs.bidi_is_sequence_boundary :: source_class :: call:
            index = end_index
        else:
            let scalar = (scalars)[index]
            if scalar.level == level:
                let value = arcana_text.shape.glyphs.bidi_surrounding_class :: scalar.class :: call
                if value != "":
                    if first_strong == "":
                        first_strong = value
                    if value == embedding:
                        saw_embedding = true
                        index = end_index
                    else:
                        index += 1
                else:
                    index += 1
            else:
                index += 1
    if saw_embedding:
        return embedding
    if first_strong != "":
        return first_strong
    return embedding

fn bidi_bracket_match_class(read matches: List[arcana_text.shape.glyphs.BidiBracketMatch], index: Int) -> Str:
    for value in matches:
        if value.start_index == index or value.end_index == index:
            return value.resolved_class
    return ""

fn bidi_resolve_brackets(read payload: (Str, List[arcana_text.shape.glyphs.BidiScalar], List[arcana_text.shape.glyphs.BidiScalar], Str)) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let text = payload.0
    let source = payload.1
    let scalars = payload.2
    let paragraph_class = payload.3
    let mut opens = arcana_text.shape.glyphs.empty_bidi_bracket_opens :: :: call
    let mut matches = arcana_text.shape.glyphs.empty_bidi_bracket_matches :: :: call
    let total = scalars :: :: len
    let mut isolate_depth = 0
    let mut index = 0
    while index < total:
        let scalar = (scalars)[index]
        let source_class = (source)[index].class
        if arcana_text.shape.glyphs.bidi_is_isolate_initiator :: source_class :: call:
            opens = arcana_text.shape.glyphs.empty_bidi_bracket_opens :: :: call
            isolate_depth += 1
            index += 1
            continue
        if arcana_text.shape.glyphs.bidi_is_isolate_terminator :: source_class :: call:
            opens = arcana_text.shape.glyphs.empty_bidi_bracket_opens :: :: call
            if isolate_depth > 0:
                isolate_depth -= 1
            index += 1
            continue
        if isolate_depth > 0:
            index += 1
            continue
        let codepoint = arcana_text.text_units.codepoint_at :: text, scalar.start :: call
        if arcana_text.shape.glyphs.bidi_is_segment_separator :: source_class :: call:
            opens = arcana_text.shape.glyphs.empty_bidi_bracket_opens :: :: call
            index += 1
            continue
        let close = arcana_text.shape.glyphs.bidi_close_bracket_for :: codepoint :: call
        if close != 0:
            let mut pending = arcana_text.shape.glyphs.empty_bidi_bracket_opens :: :: call
            let mut matched = false
            while not (opens :: :: is_empty):
                let open = opens :: :: pop
                if not matched and open.codepoint == close and open.level == scalar.level:
                    let resolved = arcana_text.shape.glyphs.bidi_bracket_resolved_class :: (source, scalars, open.index, index, paragraph_class) :: call
                    matches :: (arcana_text.shape.glyphs.BidiBracketMatch :: start_index = open.index, end_index = index, resolved_class = resolved :: call) :: push
                    matched = true
                else:
                    pending :: open :: push
            while not (pending :: :: is_empty):
                opens :: (pending :: :: pop) :: push
        else:
            let open = arcana_text.shape.glyphs.bidi_open_bracket_for :: codepoint :: call
            if open != 0:
                opens :: (arcana_text.shape.glyphs.BidiBracketOpen :: index = index, codepoint = open, level = scalar.level :: call) :: push
        index += 1
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    index = 0
    while index < total:
        let mut next = (scalars)[index]
        let resolved = arcana_text.shape.glyphs.bidi_bracket_match_class :: matches, index :: call
        if resolved == "L" or resolved == "R":
            next.class = resolved
        out :: next :: push
        index += 1
    return out

fn bidi_neutral_run_value(read request: arcana_text.shape.glyphs.BidiNeutralRunRequest) -> Str:
    let source = request.source
    let scalars = request.scalars
    let start = request.start
    let end = request.end
    let paragraph_class = request.paragraph_class
    let embedding = arcana_text.shape.glyphs.bidi_level_run_embedding :: scalars, start, paragraph_class :: call
    let level = match start < (scalars :: :: len):
        true => (scalars)[start].level
        false => 0
    let left = arcana_text.shape.glyphs.bidi_level_run_class_left :: (source, scalars, start, level, embedding) :: call
    let right = arcana_text.shape.glyphs.bidi_level_run_class_right :: (source, scalars, end, level, embedding) :: call
    if left == right:
        return left
    return embedding

fn bidi_resolve_neutrals(read source: List[arcana_text.shape.glyphs.BidiScalar], read scalars: List[arcana_text.shape.glyphs.BidiScalar], read paragraph_class: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let total = scalars :: :: len
    let mut index = 0
    while index < total:
        let scalar = (scalars)[index]
        if not (arcana_text.shape.glyphs.bidi_is_neutral :: scalar.class :: call):
            out :: scalar :: push
            index += 1
            continue
        let start = index
        while index < total and (arcana_text.shape.glyphs.bidi_is_neutral :: ((scalars)[index].class) :: call):
            index += 1
        let mut neutral_request = arcana_text.shape.glyphs.BidiNeutralRunRequest :: source = source, scalars = scalars, start = start, end = index :: call
        neutral_request.paragraph_class = paragraph_class
        let resolved = arcana_text.shape.glyphs.bidi_neutral_run_value :: neutral_request :: call
        let mut fill = start
        while fill < index:
            let mut next = (scalars)[fill]
            next.class = resolved
            out :: next :: push
            fill += 1
    return out

fn bidi_levels(read scalars: List[arcana_text.shape.glyphs.BidiScalar]) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    for scalar in scalars:
        let class = scalar.class
        let class_code = arcana_text.shape.glyphs.bidi_class_code :: class :: call
        if class_code == 1 or class_code == 2 or class_code == 3:
            let mut next = arcana_text.shape.glyphs.BidiScalar :: start = scalar.start, end = scalar.end, class = class :: call
            next.level = scalar.level
            out :: next :: push
        else:
            let mut next = arcana_text.shape.glyphs.BidiScalar :: start = scalar.start, end = scalar.end, class = class :: call
            next.level = scalar.level
            if class_code == 4:
                if (next.level % 2) == 0:
                    next.level += 1
            else:
                if class_code == 5 or class_code == 6:
                    if (next.level % 2) == 0:
                        next.level += 2
                    else:
                        next.level += 1
                else:
                    if class_code == 7 and (next.level % 2) == 1:
                        next.level += 1
            out :: next :: push
    return out

fn bidi_reset_levels(read source: List[arcana_text.shape.glyphs.BidiScalar], read resolved: List[arcana_text.shape.glyphs.BidiScalar], paragraph_level: Int) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let total = source :: :: len
    let mut reset_flags = std.kernel.collections.array_new[Bool] :: total, false :: call
    let mut reset_from = -1
    let mut index = 0
    while index < total:
        let class = (source)[index].class
        let ignored = class == "RLE" or class == "LRE" or class == "RLO" or class == "LRO" or class == "PDF" or class == "BN"
        let separator = class == "B" or class == "S"
        let whitespace = class == "WS" or class == "FSI" or class == "LRI" or class == "RLI" or class == "PDI"
        if ignored:
            index += 1
            continue
        if separator:
            if reset_from < 0:
                reset_from = index
            let mut fill = reset_from
            while fill <= index:
                reset_flags[fill] = true
                fill += 1
            reset_from = -1
        else:
            if whitespace:
                if reset_from < 0:
                    reset_from = index
            else:
                reset_from = -1
        index += 1
    if reset_from >= 0:
        let mut fill = reset_from
        while fill < total:
            reset_flags[fill] = true
            fill += 1
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    index = 0
    while index < (resolved :: :: len):
        let mut next = (resolved)[index]
        if reset_flags[index]:
            next.level = paragraph_level
        out :: next :: push
        index += 1
    return out

fn bidi_resolved_paragraph_scalars(read text: Str, read source: List[arcana_text.shape.glyphs.BidiScalar], read range: arcana_text.shape.glyphs.BidiParagraphRange) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let paragraph_source = arcana_text.shape.glyphs.bidi_scalar_slice :: source, range.start, range.end :: call
    let paragraph_class = arcana_text.shape.glyphs.bidi_paragraph_class :: range.base_level :: call
    let explicit = arcana_text.shape.glyphs.bidi_resolve_explicit :: paragraph_source, range.base_level, paragraph_class :: call
    let nsm = arcana_text.shape.glyphs.bidi_resolve_nsm :: paragraph_source, explicit, paragraph_class :: call
    let w2 = arcana_text.shape.glyphs.bidi_resolve_w2 :: nsm, paragraph_class :: call
    let al = arcana_text.shape.glyphs.bidi_resolve_al :: w2 :: call
    let w4 = arcana_text.shape.glyphs.bidi_resolve_w4 :: al :: call
    let w5 = arcana_text.shape.glyphs.bidi_resolve_w5 :: w4 :: call
    let w6 = arcana_text.shape.glyphs.bidi_resolve_w6 :: w5 :: call
    let w7 = arcana_text.shape.glyphs.bidi_resolve_w7 :: w6, paragraph_class :: call
    let brackets = arcana_text.shape.glyphs.bidi_resolve_brackets :: (text, paragraph_source, w7, paragraph_class) :: call
    let neutrals = arcana_text.shape.glyphs.bidi_resolve_neutrals :: paragraph_source, brackets, paragraph_class :: call
    let levels = arcana_text.shape.glyphs.bidi_levels :: neutrals :: call
    return arcana_text.shape.glyphs.bidi_reset_levels :: paragraph_source, levels, range.base_level :: call

fn bidi_resolved_scalars_from_source(read text: Str, read source: List[arcana_text.shape.glyphs.BidiScalar]) -> List[arcana_text.shape.glyphs.BidiScalar]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.BidiScalar] :: :: call
    let paragraphs = arcana_text.shape.glyphs.bidi_paragraph_ranges :: source :: call
    for paragraph in paragraphs:
        out :: (arcana_text.shape.glyphs.bidi_resolved_paragraph_scalars :: text, source, paragraph :: call) :: extend_list
    return out

fn bidi_resolved_scalars(read text: Str) -> List[arcana_text.shape.glyphs.BidiScalar]:
    if arcana_text.shape.glyphs.bidi_text_is_simple_ascii :: text :: call:
        return arcana_text.shape.glyphs.bidi_ascii_scalars :: text :: call
    let source = arcana_text.shape.glyphs.bidi_scalars :: text :: call
    return arcana_text.shape.glyphs.bidi_resolved_scalars_from_source :: text, source :: call

fn bidi_signature(read scalars: List[arcana_text.shape.glyphs.BidiScalar], paragraph_level: Int, read text: Str) -> Int:
    let mut signature = 41
    signature = arcana_text.shape.types.mix_signature :: signature, paragraph_level :: call
    signature = arcana_text.shape.types.mix_signature_text :: signature, text :: call
    for scalar in scalars:
        signature = arcana_text.shape.types.mix_signature :: signature, scalar.start :: call
        signature = arcana_text.shape.types.mix_signature :: signature, scalar.end :: call
        signature = arcana_text.shape.types.mix_signature :: signature, scalar.level :: call
        signature = arcana_text.shape.types.mix_signature_text :: signature, scalar.class :: call
    return signature

fn bidi_cluster_info(read request: arcana_text.shape.glyphs.BidiClusterInfoRequest) -> arcana_text.shape.glyphs.ResolvedCluster:
    let text = request.text
    let start = request.start
    let end = request.end
    let scalars = request.scalars
    let mut info = arcana_text.shape.glyphs.ResolvedCluster :: text = (std.text.slice_bytes :: text, start, end :: call), range = (arcana_text.types.TextRange :: start = start, end = end :: call) :: call
    info.direction = arcana_text.types.TextDirection.LeftToRight :: :: call
    info.bidi_level = 0
    let mut saw = false
    for scalar in scalars:
        if scalar.end <= start or scalar.start >= end:
            continue
        saw = true
        if scalar.level > info.bidi_level:
            info.bidi_level = scalar.level
    if saw:
        info.direction = arcana_text.shape.glyphs.bidi_direction_from_level :: info.bidi_level :: call
    return info

fn resolved_clusters_for_line(read line: arcana_text.shape.glyphs.ResolvedLine) -> List[arcana_text.shape.glyphs.ResolvedCluster]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.ResolvedCluster] :: :: call
    let text = line.text
    let total = std.text.len_bytes :: text :: call
    let mut index = 0
    while index < total:
        let end = arcana_text.text_units.next_cluster_end :: text, index :: call
        let mut request = arcana_text.shape.glyphs.BidiClusterInfoRequest :: text = text, start = index, end = end :: call
        request.scalars = line.scalars
        let mut info = arcana_text.shape.glyphs.bidi_cluster_info :: request :: call
        info.range = arcana_text.types.TextRange :: start = line.range.start + index, end = line.range.start + end :: call
        out :: info :: push
        index = end
    return out

export fn resolve_line(read text: Str, absolute_start: Int) -> arcana_text.shape.glyphs.ResolvedLine:
    let simple_ascii = arcana_text.shape.glyphs.bidi_text_is_simple_ascii :: text :: call
    let source = match simple_ascii:
        true => (arcana_text.shape.glyphs.bidi_ascii_scalars :: text :: call)
        false => (arcana_text.shape.glyphs.bidi_scalars :: text :: call)
    let scalars = match simple_ascii:
        true => source
        false => (arcana_text.shape.glyphs.bidi_resolved_scalars_from_source :: text, source :: call)
    let paragraph_level = match simple_ascii:
        true => 0
        false => arcana_text.shape.glyphs.bidi_first_paragraph_level :: source :: call
    let mut line = arcana_text.shape.glyphs.ResolvedLine :: text = text, range = (arcana_text.types.TextRange :: start = absolute_start, end = absolute_start + (std.text.len_bytes :: text :: call) :: call), scalars = scalars :: call
    line.paragraph_level = paragraph_level
    line.signature = arcana_text.shape.glyphs.bidi_signature :: scalars, paragraph_level, text :: call
    return line

export fn resolved_clusters_for_range(read line: arcana_text.shape.glyphs.ResolvedLine, read range: arcana_text.types.TextRange) -> List[arcana_text.shape.glyphs.ResolvedCluster]:
    let mut out = std.collections.list.empty[arcana_text.shape.glyphs.ResolvedCluster] :: :: call
    let total = std.text.len_bytes :: line.text :: call
    let mut index = range.start - line.range.start
    let limit = range.end - line.range.start
    while index < limit and index < total:
        let mut end = arcana_text.text_units.next_cluster_end :: line.text, index :: call
        if end > limit:
            end = limit
        let mut request = arcana_text.shape.glyphs.BidiClusterInfoRequest :: text = line.text, start = index, end = end :: call
        request.scalars = line.scalars
        let mut info = arcana_text.shape.glyphs.bidi_cluster_info :: request :: call
        info.range = arcana_text.types.TextRange :: start = line.range.start + index, end = line.range.start + end :: call
        info.text = std.text.slice_bytes :: line.text, index, end :: call
        out :: info :: push
        index = end
    return out

export fn resolved_cluster_for_range(read line: arcana_text.shape.glyphs.ResolvedLine, read range: arcana_text.types.TextRange) -> arcana_text.shape.glyphs.ResolvedCluster:
    let mut request = arcana_text.shape.glyphs.BidiClusterInfoRequest :: text = line.text, start = range.start - line.range.start, end = range.end - line.range.start :: call
    request.scalars = line.scalars
    let mut info = arcana_text.shape.glyphs.bidi_cluster_info :: request :: call
    info.range = range
    info.text = std.text.slice_bytes :: line.text, request.start, request.end :: call
    return info

fn resolved_clusters_for_text(read text: Str, absolute_start: Int) -> List[arcana_text.shape.glyphs.ResolvedCluster]:
    let line = arcana_text.shape.glyphs.resolve_line :: text, absolute_start :: call
    return arcana_text.shape.glyphs.resolved_clusters_for_line :: line :: call

fn script_for_codepoint(codepoint: Int) -> arcana_text.types.ScriptClass:
    if codepoint >= 125184 and codepoint <= 125279:
        return arcana_text.types.ScriptClass.Adlam :: :: call
    if codepoint >= 1424 and codepoint <= 1535:
        return arcana_text.types.ScriptClass.Hebrew :: :: call
    if (codepoint >= 1536 and codepoint <= 1791) or (codepoint >= 1872 and codepoint <= 1919) or (codepoint >= 2208 and codepoint <= 2303) or (codepoint >= 64336 and codepoint <= 65023):
        return arcana_text.types.ScriptClass.Arabic :: :: call
    if codepoint >= 1920 and codepoint <= 1983:
        return arcana_text.types.ScriptClass.Thaana :: :: call
    if codepoint >= 2304 and codepoint <= 2431:
        return arcana_text.types.ScriptClass.Devanagari :: :: call
    if codepoint >= 2432 and codepoint <= 2559:
        return arcana_text.types.ScriptClass.Bengali :: :: call
    if codepoint >= 2560 and codepoint <= 2687:
        return arcana_text.types.ScriptClass.Gurmukhi :: :: call
    if codepoint >= 2688 and codepoint <= 2815:
        return arcana_text.types.ScriptClass.Gujarati :: :: call
    if codepoint >= 2816 and codepoint <= 2943:
        return arcana_text.types.ScriptClass.Oriya :: :: call
    if codepoint >= 2944 and codepoint <= 3071:
        return arcana_text.types.ScriptClass.Tamil :: :: call
    if codepoint >= 3072 and codepoint <= 3199:
        return arcana_text.types.ScriptClass.Telugu :: :: call
    if codepoint >= 3200 and codepoint <= 3327:
        return arcana_text.types.ScriptClass.Kannada :: :: call
    if codepoint >= 3328 and codepoint <= 3455:
        return arcana_text.types.ScriptClass.Malayalam :: :: call
    if codepoint >= 3456 and codepoint <= 3583:
        return arcana_text.types.ScriptClass.Sinhala :: :: call
    if codepoint >= 3584 and codepoint <= 3711:
        return arcana_text.types.ScriptClass.Thai :: :: call
    if codepoint >= 3712 and codepoint <= 3839:
        return arcana_text.types.ScriptClass.Lao :: :: call
    if codepoint >= 3840 and codepoint <= 4095:
        return arcana_text.types.ScriptClass.Tibetan :: :: call
    if codepoint >= 4096 and codepoint <= 4255:
        return arcana_text.types.ScriptClass.Myanmar :: :: call
    if codepoint >= 4352 and codepoint <= 4607:
        return arcana_text.types.ScriptClass.Hangul :: :: call
    if codepoint >= 4608 and codepoint <= 4991:
        return arcana_text.types.ScriptClass.Ethiopic :: :: call
    if codepoint >= 5024 and codepoint <= 5119:
        return arcana_text.types.ScriptClass.Cherokee :: :: call
    if codepoint >= 5120 and codepoint <= 5759:
        return arcana_text.types.ScriptClass.CanadianAboriginal :: :: call
    if codepoint >= 6016 and codepoint <= 6143:
        return arcana_text.types.ScriptClass.Khmer :: :: call
    if codepoint >= 6144 and codepoint <= 6319:
        return arcana_text.types.ScriptClass.Mongolian :: :: call
    if codepoint >= 11568 and codepoint <= 11647:
        return arcana_text.types.ScriptClass.Tifinagh :: :: call
    if codepoint >= 11648 and codepoint <= 11743:
        return arcana_text.types.ScriptClass.Ethiopic :: :: call
    if codepoint >= 12352 and codepoint <= 12447:
        return arcana_text.types.ScriptClass.Hiragana :: :: call
    if codepoint >= 12448 and codepoint <= 12543:
        return arcana_text.types.ScriptClass.Katakana :: :: call
    if codepoint >= 12544 and codepoint <= 12591:
        return arcana_text.types.ScriptClass.Bopomofo :: :: call
    if codepoint >= 12704 and codepoint <= 12735:
        return arcana_text.types.ScriptClass.Bopomofo :: :: call
    if codepoint >= 12784 and codepoint <= 12799:
        return arcana_text.types.ScriptClass.Katakana :: :: call
    if (codepoint >= 13312 and codepoint <= 19903) or (codepoint >= 19968 and codepoint <= 40959) or (codepoint >= 63744 and codepoint <= 64255):
        return arcana_text.types.ScriptClass.Han :: :: call
    if codepoint >= 44032 and codepoint <= 55215:
        return arcana_text.types.ScriptClass.Hangul :: :: call
    if (codepoint >= 1024 and codepoint <= 1327) or (codepoint >= 42560 and codepoint <= 42655):
        return arcana_text.types.ScriptClass.Cyrillic :: :: call
    if codepoint >= 42240 and codepoint <= 42559:
        return arcana_text.types.ScriptClass.Vai :: :: call
    if codepoint >= 43392 and codepoint <= 43487:
        return arcana_text.types.ScriptClass.Javanese :: :: call
    if codepoint >= 43888 and codepoint <= 43967:
        return arcana_text.types.ScriptClass.Cherokee :: :: call
    if codepoint >= 40960 and codepoint <= 42127:
        return arcana_text.types.ScriptClass.Yi :: :: call
    if codepoint >= 69888 and codepoint <= 70015:
        return arcana_text.types.ScriptClass.Chakma :: :: call
    if codepoint >= 0 and codepoint <= 591:
        return arcana_text.types.ScriptClass.Latin :: :: call
    return arcana_text.types.ScriptClass.Common :: :: call

fn script_for_text(read text: Str) -> arcana_text.types.ScriptClass:
    let total = std.text.len_bytes :: text :: call
    let mut index = 0
    while index < total:
        let codepoint = arcana_text.text_units.codepoint_at :: text, index :: call
        let script = script_for_codepoint :: codepoint :: call
        if script != (arcana_text.types.ScriptClass.Common :: :: call):
            return script
        index = arcana_text.text_units.next_scalar_end :: text, index :: call
    return arcana_text.types.ScriptClass.Common :: :: call

fn direction_for_script(read script: arcana_text.types.ScriptClass) -> arcana_text.types.TextDirection:
    return match script:
        arcana_text.types.ScriptClass.Adlam => arcana_text.types.TextDirection.RightToLeft :: :: call
        arcana_text.types.ScriptClass.Arabic => arcana_text.types.TextDirection.RightToLeft :: :: call
        arcana_text.types.ScriptClass.Hebrew => arcana_text.types.TextDirection.RightToLeft :: :: call
        arcana_text.types.ScriptClass.Thaana => arcana_text.types.TextDirection.RightToLeft :: :: call
        _ => arcana_text.types.TextDirection.LeftToRight :: :: call

fn mirrored_scalar_text(text: Str, read direction: arcana_text.types.TextDirection) -> Str:
    if direction != (arcana_text.types.TextDirection.RightToLeft :: :: call):
        return text
    if (std.text.len_bytes :: text :: call) > 0:
        let codepoint = arcana_text.text_units.codepoint_at :: text, 0 :: call
        let mirrored = arcana_text.shape.glyphs.mirrored_codepoint :: codepoint :: call
        if mirrored != codepoint:
            return arcana_text.shape.glyphs.utf8_scalar_from_codepoint :: mirrored :: call
    return text
    return match text:
        "(" => ")"
        ")" => "("
        "[" => "]"
        "]" => "["
        "{" => "}"
        "}" => "{"
        "<" => ">"
        ">" => "<"
        "Ã‚Â«" => "Ã‚Â»"
        "Ã‚Â»" => "Ã‚Â«"
        "Ã¢â‚¬Â¹" => "Ã¢â‚¬Âº"
        "Ã¢â‚¬Âº" => "Ã¢â‚¬Â¹"
        _ => text

fn default_feature(tag: Str) -> arcana_text.types.FontFeature:
    let mut feature = arcana_text.types.FontFeature :: tag = tag, value = 1, enabled = true :: call
    return feature

fn mirrored_scalar_text_resolved(text: Str, read direction: arcana_text.types.TextDirection) -> Str:
    if direction != (arcana_text.types.TextDirection.RightToLeft :: :: call):
        return text
    if (std.text.len_bytes :: text :: call) > 0:
        let codepoint = arcana_text.text_units.codepoint_at :: text, 0 :: call
        let mirrored = arcana_text.shape.glyphs.mirrored_codepoint :: codepoint :: call
        if mirrored != codepoint:
            return arcana_text.shape.glyphs.utf8_scalar_from_codepoint :: mirrored :: call
    return text

fn feature_setting(read features: List[arcana_text.types.FontFeature], tag: Str) -> Int:
    for feature in features:
        if feature.tag == tag:
            if feature.enabled:
                return feature.value
            return 0
    return 0

fn hinted_advance_for_spec(edit fonts: arcana_text.fonts.FontSystem, read face_id: arcana_text.types.FontFaceId, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> Int:
    let mut hinted = spec
    hinted.hinting = arcana_text.types.Hinting.Enabled :: :: call
    return fonts :: face_id, hinted :: advance_face_glyph

fn hinted_measure_for_spec(edit fonts: arcana_text.fonts.FontSystem, read face_id: arcana_text.types.FontFaceId, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    let mut hinted = spec
    hinted.hinting = arcana_text.types.Hinting.Enabled :: :: call
    return fonts :: face_id, hinted :: measure_face_glyph

fn single_feature(read tag: Str, value: Int) -> arcana_text.types.FontFeature:
    let mut feature = arcana_text.types.FontFeature :: tag = tag, value = value, enabled = true :: call
    return feature

fn is_join_feature(read tag: Str) -> Bool:
    return tag == "isol" or tag == "init" or tag == "medi" or tag == "fina"

fn shaping_features(read script: arcana_text.types.ScriptClass, read features: List[arcana_text.types.FontFeature]) -> List[arcana_text.types.FontFeature]:
    let effective = arcana_text.shape.glyphs.effective_features :: script, features :: call
    if script != (arcana_text.types.ScriptClass.Arabic :: :: call):
        return effective
    let mut out = std.collections.list.empty[arcana_text.types.FontFeature] :: :: call
    for feature in effective:
        if not (arcana_text.shape.glyphs.is_join_feature :: feature.tag :: call):
            out :: feature :: push
    return out

fn vertical_features_enabled(read features: List[arcana_text.types.FontFeature]) -> Bool:
    return (arcana_text.shape.glyphs.feature_setting :: features, "vert" :: call) > 0 or (arcana_text.shape.glyphs.feature_setting :: features, "vrt2" :: call) > 0

fn arabic_joining_type(read text: Str) -> Int:
    let codepoint = arcana_text.text_units.codepoint_at :: text, 0 :: call
    if codepoint == 0:
        return 0
    if codepoint == 8205 or codepoint == 1600:
        return 4
    if codepoint == 8204:
        return 0
    if codepoint >= 1611 and codepoint <= 1631:
        return 5
    if codepoint == 1648:
        return 5
    if codepoint >= 1750 and codepoint <= 1773:
        return 5
    if codepoint >= 1425 and codepoint <= 1479:
        return 5
    if codepoint == 1523 or codepoint == 1524:
        return 5
    if codepoint == 1569:
        return 0
    if codepoint == 1570 or codepoint == 1571 or codepoint == 1572 or codepoint == 1573 or codepoint == 1575:
        return 1
    if codepoint >= 1583 and codepoint <= 1586:
        return 1
    if codepoint == 1608:
        return 1
    if codepoint >= 1649 and codepoint <= 1651:
        return 1
    if codepoint >= 1653 and codepoint <= 1655:
        return 1
    if codepoint >= 1672 and codepoint <= 1688:
        return 1
    if codepoint >= 1689 and codepoint <= 1698:
        return 1
    if codepoint == 1728 or codepoint == 1729:
        return 1
    if codepoint >= 1730 and codepoint <= 1731:
        return 1
    if codepoint >= 1733 and codepoint <= 1741:
        return 1
    if codepoint == 1743 or codepoint == 1745:
        return 1
    if codepoint >= 1574 and codepoint <= 1582:
        return 3
    if codepoint >= 1587 and codepoint <= 1607:
        return 3
    if codepoint == 1609 or codepoint == 1610:
        return 3
    if codepoint >= 1662 and codepoint <= 1671:
        return 3
    if codepoint >= 1699 and codepoint <= 1727:
        return 3
    if codepoint >= 1746 and codepoint <= 1747:
        return 3
    return 0

fn joins_left(kind: Int) -> Bool:
    return kind == 2 or kind == 3 or kind == 4

fn joins_right(kind: Int) -> Bool:
    return kind == 1 or kind == 3 or kind == 4

fn transparent_joining(kind: Int) -> Bool:
    return kind == 5

fn previous_joining_index(read glyphs: List[arcana_text.shape.glyphs.GlyphSeed], index: Int) -> Int:
    let mut cursor = index - 1
    while cursor >= 0:
        let kind = arcana_text.shape.glyphs.arabic_joining_type :: (glyphs)[cursor].text :: call
        if not (arcana_text.shape.glyphs.transparent_joining :: kind :: call):
            return cursor
        cursor -= 1
    return -1

fn next_joining_index(read glyphs: List[arcana_text.shape.glyphs.GlyphSeed], index: Int) -> Int:
    let total = glyphs :: :: len
    let mut cursor = index + 1
    while cursor < total:
        let kind = arcana_text.shape.glyphs.arabic_joining_type :: (glyphs)[cursor].text :: call
        if not (arcana_text.shape.glyphs.transparent_joining :: kind :: call):
            return cursor
        cursor += 1
    return -1

fn join_feature_for_glyph(read glyphs: List[arcana_text.shape.glyphs.GlyphSeed], index: Int, read features: List[arcana_text.types.FontFeature]) -> Option[arcana_text.types.FontFeature]:
    let current_kind = arcana_text.shape.glyphs.arabic_joining_type :: (glyphs)[index].text :: call
    if current_kind == 0 or current_kind == 5:
        return Option.None[arcana_text.types.FontFeature] :: :: call
    let previous_index = arcana_text.shape.glyphs.previous_joining_index :: glyphs, index :: call
    let next_index = arcana_text.shape.glyphs.next_joining_index :: glyphs, index :: call
    let mut joins_previous = false
    let mut joins_next = false
    if previous_index >= 0:
        let previous_kind = arcana_text.shape.glyphs.arabic_joining_type :: (glyphs)[previous_index].text :: call
        joins_previous = (arcana_text.shape.glyphs.joins_left :: previous_kind :: call) and (arcana_text.shape.glyphs.joins_right :: current_kind :: call)
    if next_index >= 0:
        let next_kind = arcana_text.shape.glyphs.arabic_joining_type :: (glyphs)[next_index].text :: call
        joins_next = (arcana_text.shape.glyphs.joins_left :: current_kind :: call) and (arcana_text.shape.glyphs.joins_right :: next_kind :: call)
    let tag = match joins_previous:
        true => match joins_next:
            true => "medi"
            false => "fina"
        false => match joins_next:
            true => "init"
            false => "isol"
    let value = arcana_text.shape.glyphs.feature_setting :: features, tag :: call
    if value <= 0:
        return Option.None[arcana_text.types.FontFeature] :: :: call
    return Option.Some[arcana_text.types.FontFeature] :: (arcana_text.shape.glyphs.single_feature :: tag, value :: call) :: call

fn substitute_single_feature(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.SingleFeatureSubstituteRequest) -> Int:
    let glyph_index = request.glyph_index
    let matched = request.matched
    let script = request.script
    let language_tag = request.language_tag
    let feature = request.feature
    if glyph_index <= 0:
        return glyph_index
    let mut features = std.collections.list.empty[arcana_text.types.FontFeature] :: :: call
    features :: feature :: push
    let mut glyphs = std.collections.list.empty[Int] :: :: call
    glyphs :: glyph_index :: push
    let mut gsub_request = arcana_text.fonts.GsubUnitsRequest :: matched = matched, script_tag = (arcana_text.shape.glyphs.script_tag :: script :: call), language_tag = language_tag :: call
    gsub_request.features = features
    gsub_request.glyphs = glyphs
    let units = fonts :: gsub_request :: gsub_units
    if units :: :: is_empty:
        return glyph_index
    return (units)[0].glyph_index

fn apply_arabic_join_forms(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.ArabicJoinRequest) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let mut next = std.collections.list.empty[arcana_text.font_leaf.GsubGlyphUnit] :: :: call
    let total = request.units :: :: len
    let mut index = 0
    while index < total:
        let current = (request.units)[index]
        let feature = arcana_text.shape.glyphs.join_feature_for_glyph :: request.seeds, index, request.features :: call
        let mut adjusted = current
        if feature :: :: is_some:
            let mut substitute_request = arcana_text.shape.glyphs.SingleFeatureSubstituteRequest :: matched = request.matched, script = request.script, language_tag = request.language_tag :: call
            substitute_request.feature = feature :: (arcana_text.shape.glyphs.default_feature :: "isol" :: call) :: unwrap_or
            substitute_request.glyph_index = current.glyph_index
            adjusted.glyph_index = arcana_text.shape.glyphs.substitute_single_feature :: fonts, substitute_request :: call
        next :: adjusted :: push
        index += 1
    return next

fn push_feature_if_missing(edit out: List[arcana_text.types.FontFeature], read feature: arcana_text.types.FontFeature):
    for existing in out:
        if existing.tag == feature.tag:
            return
    out :: feature :: push

fn effective_features(read script: arcana_text.types.ScriptClass, read features: List[arcana_text.types.FontFeature]) -> List[arcana_text.types.FontFeature]:
    let mut out = std.collections.list.empty[arcana_text.types.FontFeature] :: :: call
    out :: features :: extend_list
    arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "ccmp" :: call) :: call
    arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "locl" :: call) :: call
    arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "rlig" :: call) :: call
    arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "liga" :: call) :: call
    arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "calt" :: call) :: call
    arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "kern" :: call) :: call
    if script == (arcana_text.types.ScriptClass.Arabic :: :: call):
        arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "isol" :: call) :: call
        arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "init" :: call) :: call
        arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "medi" :: call) :: call
        arcana_text.shape.glyphs.push_feature_if_missing :: out, (arcana_text.shape.glyphs.default_feature :: "fina" :: call) :: call
    return out

fn feature_signature_for(read script: arcana_text.types.ScriptClass, read features: List[arcana_text.types.FontFeature]) -> Int:
    return arcana_text.types.feature_signature :: (arcana_text.shape.glyphs.effective_features :: script, features :: call) :: call

fn script_tag(read script: arcana_text.types.ScriptClass) -> Str:
    return match script:
        arcana_text.types.ScriptClass.Adlam => "adlm"
        arcana_text.types.ScriptClass.Arabic => "arab"
        arcana_text.types.ScriptClass.Bengali => "beng"
        arcana_text.types.ScriptClass.Bopomofo => "bopo"
        arcana_text.types.ScriptClass.CanadianAboriginal => "cans"
        arcana_text.types.ScriptClass.Chakma => "cakm"
        arcana_text.types.ScriptClass.Cherokee => "cher"
        arcana_text.types.ScriptClass.Cyrillic => "cyrl"
        arcana_text.types.ScriptClass.Devanagari => "deva"
        arcana_text.types.ScriptClass.Ethiopic => "ethi"
        arcana_text.types.ScriptClass.Gujarati => "gujr"
        arcana_text.types.ScriptClass.Gurmukhi => "guru"
        arcana_text.types.ScriptClass.Hangul => "hang"
        arcana_text.types.ScriptClass.Hiragana => "hira"
        arcana_text.types.ScriptClass.Hebrew => "hebr"
        arcana_text.types.ScriptClass.Han => "hani"
        arcana_text.types.ScriptClass.Javanese => "java"
        arcana_text.types.ScriptClass.Kannada => "knda"
        arcana_text.types.ScriptClass.Katakana => "kana"
        arcana_text.types.ScriptClass.Khmer => "khmr"
        arcana_text.types.ScriptClass.Lao => "laoo"
        arcana_text.types.ScriptClass.Latin => "latn"
        arcana_text.types.ScriptClass.Malayalam => "mlym"
        arcana_text.types.ScriptClass.Mongolian => "mong"
        arcana_text.types.ScriptClass.Myanmar => "mymr"
        arcana_text.types.ScriptClass.Oriya => "orya"
        arcana_text.types.ScriptClass.Sinhala => "sinh"
        arcana_text.types.ScriptClass.Tamil => "taml"
        arcana_text.types.ScriptClass.Telugu => "telu"
        arcana_text.types.ScriptClass.Thaana => "thaa"
        arcana_text.types.ScriptClass.Thai => "thai"
        arcana_text.types.ScriptClass.Tibetan => "tibt"
        arcana_text.types.ScriptClass.Tifinagh => "tfng"
        arcana_text.types.ScriptClass.Vai => "vaii"
        arcana_text.types.ScriptClass.Yi => "yiii"
        _ => "DFLT"

fn locale_matches(read locale: Str, read prefix: Str) -> Bool:
    if locale == prefix:
        return true
    return std.text.starts_with :: locale, (prefix + "-") :: call

fn cjk_language_tag(read locale: Str) -> Str:
    if arcana_text.shape.glyphs.locale_matches :: locale, "ja" :: call:
        return "JAN "
    if arcana_text.shape.glyphs.locale_matches :: locale, "ko" :: call:
        return "KOR "
    if arcana_text.shape.glyphs.locale_matches :: locale, "zh-TW" :: call:
        return "ZHT "
    if arcana_text.shape.glyphs.locale_matches :: locale, "zh-HK" :: call:
        return "ZHT "
    if arcana_text.shape.glyphs.locale_matches :: locale, "zh" :: call:
        return "ZHS "
    return ""

fn default_language_tag(read locale: Str, read script: arcana_text.types.ScriptClass) -> Str:
    return match script:
        arcana_text.types.ScriptClass.Adlam => "ADL "
        arcana_text.types.ScriptClass.Arabic => "ARA "
        arcana_text.types.ScriptClass.Bengali => "BEN "
        arcana_text.types.ScriptClass.Bopomofo => arcana_text.shape.glyphs.cjk_language_tag :: locale :: call
        arcana_text.types.ScriptClass.Devanagari => "HIN "
        arcana_text.types.ScriptClass.Gujarati => "GUJ "
        arcana_text.types.ScriptClass.Gurmukhi => "PAN "
        arcana_text.types.ScriptClass.Hangul => "KOR "
        arcana_text.types.ScriptClass.Hebrew => "IWR "
        arcana_text.types.ScriptClass.Han => arcana_text.shape.glyphs.cjk_language_tag :: locale :: call
        arcana_text.types.ScriptClass.Hiragana => "JAN "
        arcana_text.types.ScriptClass.Katakana => "JAN "
        arcana_text.types.ScriptClass.Kannada => "KAN "
        arcana_text.types.ScriptClass.Malayalam => "MAL "
        arcana_text.types.ScriptClass.Oriya => "ORI "
        arcana_text.types.ScriptClass.Sinhala => "SNH "
        arcana_text.types.ScriptClass.Tamil => "TAM "
        arcana_text.types.ScriptClass.Telugu => "TEL "
        arcana_text.types.ScriptClass.Thai => "THA "
        arcana_text.types.ScriptClass.Tibetan => "TIB "
        _ => ""

fn plan_key(read request: arcana_text.shape.glyphs.PlanKeyRequest) -> arcana_text.types.ShapePlanKey:
    let matched = request.matched
    let style = request.style
    let script = request.script
    let direction = request.direction
    let features = arcana_text.shape.glyphs.effective_features :: script, style.features :: call
    let traits = arcana_text.fonts.style_traits_for :: style :: call
    let mut key = arcana_text.types.ShapePlanKey :: face_id = matched.id, direction = direction, script = script :: call
    key.language_tag = request.language_tag
    key.font_size = style.size
    key.weight = traits.weight
    key.width_milli = traits.width_milli
    key.slant_milli = traits.slant_milli
    key.feature_signature = arcana_text.types.feature_signature :: features :: call
    key.axis_signature = arcana_text.types.axis_signature :: style.axes :: call
    return key

fn primary_match(edit fonts: arcana_text.fonts.FontSystem, read style: arcana_text.types.TextStyle, read text: Str) -> arcana_text.types.FontMatch:
    return fonts :: style, (arcana_text.shape.tokens.first_visible_char :: text :: call) :: resolve_style_char

fn match_for_text(edit fonts: arcana_text.fonts.FontSystem, read payload: arcana_text.shape.glyphs.MatchTextRequest) -> arcana_text.types.FontMatch:
    let primary = payload.primary
    let style = payload.style
    let text = payload.text
    if arcana_text.shape.glyphs.text_is_empty_glyph :: text :: call:
        return primary
    if primary.id.source_index >= 0 and (fonts :: primary, text :: supports_text):
        return primary
    let resolved = fonts :: style, text :: resolve_style_text
    if resolved.id.source_index >= 0:
        return resolved
    return primary

fn shape_glyph_with_match(edit fonts: arcana_text.fonts.FontSystem, read payload: arcana_text.shape.glyphs.ShapeGlyphWithMatchRequest) -> arcana_text.shape.glyphs.PreparedGlyph:
    let style = payload.style
    let matched = payload.matched
    let request = payload.glyph
    let script = payload.script
    let shape_text = arcana_text.shape.glyphs.mirrored_scalar_text_resolved :: request.text, payload.direction :: call
    shape_probe_append :: ("shape_glyph:start `" + shape_text + "`") :: call
    shape_probe_append :: ("shape_glyph:matched source=" + (std.text.from_int :: matched.id.source_index :: call) + " face=" + (std.text.from_int :: matched.id.face_index :: call)) :: call
    let traits = arcana_text.fonts.style_traits_for :: style :: call
    let line_height_milli = arcana_text.fonts.style_line_height_milli :: style :: call
    let vertical = arcana_text.shape.glyphs.vertical_features_enabled :: style.features :: call
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: shape_text, style.size, line_height_milli :: call
    spec.traits = traits
    spec.feature_signature = arcana_text.shape.glyphs.feature_signature_for :: script, style.features :: call
    spec.axis_signature = arcana_text.types.axis_signature :: style.axes :: call
    spec.vertical = vertical
    let mut glyph_index = -1
    if matched.id.source_index >= 0:
        glyph_index = fonts :: matched, shape_text :: glyph_index
    shape_probe_append :: ("shape_glyph:glyph_index=" + (std.text.from_int :: glyph_index :: call)) :: call
    spec.glyph_index = glyph_index
    let fallback = empty_bitmap :: style :: call
    let mut advance = fallback.advance
    let mut hinted_advance = fallback.advance
    let mut baseline = fallback.baseline
    let mut line_height = fallback.line_height
    let mut ink_offset = (0, 0)
    let mut ink_size = (0, 0)
    let mut empty = fallback.empty
    if matched.id.source_index >= 0:
        baseline = fonts :: matched, style :: baseline
        line_height = fonts :: matched, style :: line_height
        shape_probe_append :: "shape_glyph:advance_start" :: call
        advance = fonts :: matched.id, spec :: advance_face_glyph
        hinted_advance = arcana_text.shape.glyphs.hinted_advance_for_spec :: fonts, matched.id, spec :: call
        let measured = arcana_text.shape.glyphs.hinted_measure_for_spec :: fonts, matched.id, spec :: call
        ink_offset = measured.offset
        ink_size = measured.size
        baseline = measured.baseline
        line_height = measured.line_height
        shape_probe_append :: ("shape_glyph:advance_done advance=" + (std.text.from_int :: advance :: call)) :: call
        empty = measured.empty or glyph_index <= 0 or (arcana_text.shape.glyphs.text_is_empty_glyph :: request.text :: call)
    if arcana_text.shape.glyphs.text_is_zero_advance_glyph :: request.text :: call:
        advance = 0
        hinted_advance = 0
    let mut family = ""
    if matched.id.source_index >= 0:
        family = arcana_text.fonts.match_family_or_label :: fonts, matched :: call
    let mut glyph = arcana_text.types.ShapedGlyph :: glyph = request.text, range = request.range, family = family :: call
    glyph.face_id = matched.id
    glyph.glyph_index = glyph_index
    glyph.cluster_range = request.range
    glyph.font_size = style.size
    glyph.line_height_milli = line_height_milli
    glyph.weight = traits.weight
    glyph.width_milli = traits.width_milli
    glyph.slant_milli = traits.slant_milli
    glyph.feature_signature = spec.feature_signature
    glyph.axis_signature = spec.axis_signature
    glyph.advance = advance + style.letter_spacing
    glyph.x_advance = match vertical:
        true => 0
        false => hinted_advance + style.letter_spacing
    glyph.y_advance = match vertical:
        true => hinted_advance + style.letter_spacing
        false => 0
    glyph.offset = (0, 0)
    glyph.ink_offset = ink_offset
    glyph.ink_size = ink_size
    glyph.baseline = baseline
    glyph.line_height = line_height
    glyph.caret_stop_before = true
    glyph.caret_stop_after = true
    glyph.empty = empty
    let unresolved = glyph_index <= 0 and not (arcana_text.shape.glyphs.text_is_empty_glyph :: request.text :: call)
    let unresolved_code = match unresolved:
        true => 1
        false => 0
    shape_probe_append :: ("shape_glyph:done unresolved=" + (std.text.from_int :: unresolved_code :: call)) :: call
    return arcana_text.shape.glyphs.PreparedGlyph :: glyph = glyph, match = matched, unresolved = unresolved :: call

fn shape_glyph(edit fonts: arcana_text.fonts.FontSystem, read payload: arcana_text.shape.glyphs.ShapeGlyphRequest) -> arcana_text.shape.glyphs.PreparedGlyph:
    let matched = match_for_text :: fonts, (arcana_text.shape.glyphs.MatchTextRequest :: primary = payload.primary, style = payload.style, text = payload.glyph.text :: call) :: call
    let mut request = arcana_text.shape.glyphs.ShapeGlyphWithMatchRequest :: style = payload.style, matched = matched, script = payload.script :: call
    request.direction = payload.direction
    request.glyph = payload.glyph
    return arcana_text.shape.glyphs.shape_glyph_with_match :: fonts, request :: call

export fn shape_placeholder_with_context(read span_style: arcana_text.types.SpanStyle, read spec: arcana_text.types.PlaceholderSpec, read context: arcana_text.shape.glyphs.ResolvedCluster) -> arcana_text.types.PreparedRun:
    let style = arcana_text.shape.styles.text_style_from_span :: span_style :: call
    let script = arcana_text.types.ScriptClass.Common :: :: call
    let vertical = arcana_text.shape.glyphs.vertical_features_enabled :: style.features :: call
    let empty_match = arcana_text.fonts.invalid_match :: :: call
    let mut key_request = arcana_text.shape.glyphs.PlanKeyRequest :: matched = empty_match, style = style, script = script :: call
    key_request.direction = context.direction
    key_request.language_tag = ""
    let key = arcana_text.shape.glyphs.plan_key :: key_request :: call
    let mut run = arcana_text.types.ShapedRun :: kind = (arcana_text.types.ShapedRunKind.Placeholder :: :: call), range = spec.range, text = "" :: call
    run.style = span_style
    run.direction = context.direction
    run.script = script
    run.bidi_level = context.bidi_level
    run.language_tag = ""
    run.plan_key = key
    run.match = empty_match
    run.glyphs = empty_glyphs :: :: call
    run.width = match vertical:
        true => spec.size.1
        false => spec.size.0
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
    glyph.feature_signature = 0
    glyph.axis_signature = 0
    glyph.advance = match vertical:
        true => spec.size.1
        false => spec.size.0
    glyph.x_advance = match vertical:
        true => 0
        false => glyph.advance
    glyph.y_advance = match vertical:
        true => glyph.advance
        false => 0
    glyph.offset = (0, 0)
    glyph.ink_offset = (0, 0)
    glyph.ink_size = spec.size
    glyph.baseline = spec.baseline_offset
    glyph.line_height = spec.size.1
    glyph.caret_stop_before = true
    glyph.caret_stop_after = true
    glyph.empty = true
    run.glyphs :: glyph :: push
    return arcana_text.types.PreparedRun :: run = run, unresolved = (arcana_text.shape.types.empty_unresolved :: :: call) :: call

export fn shape_placeholder(read span_style: arcana_text.types.SpanStyle, read spec: arcana_text.types.PlaceholderSpec) -> arcana_text.types.PreparedRun:
    let mut context = arcana_text.shape.glyphs.ResolvedCluster :: text = "", range = spec.range :: call
    context.direction = arcana_text.types.TextDirection.LeftToRight :: :: call
    context.bidi_level = 0
    return arcana_text.shape.glyphs.shape_placeholder_with_context :: span_style, spec, context :: call

export fn shape_inline(edit fonts: arcana_text.fonts.FontSystem, read style: arcana_text.types.SpanStyle, read payload: (Str, Int)) -> arcana_text.types.ShapedRun:
    let text = payload.0
    let start = payload.1
    let token = arcana_text.shape.tokens.text_token :: text, start, (start + (std.text.len_bytes :: text :: call)) :: call
    let prepared = shape_token :: fonts, token, style :: call
    return prepared.run

fn bidi_level_for(read direction: arcana_text.types.TextDirection) -> Int:
    if direction == (arcana_text.types.TextDirection.RightToLeft :: :: call):
        return 1
    return 0

fn text_run(read seed: arcana_text.shape.glyphs.RunSeed) -> arcana_text.types.ShapedRun:
    let mut key_request = arcana_text.shape.glyphs.PlanKeyRequest :: matched = seed.matched, style = seed.style, script = seed.script :: call
    key_request.direction = seed.direction
    key_request.language_tag = seed.language_tag
    let mut run = arcana_text.types.ShapedRun :: kind = (arcana_text.types.ShapedRunKind.Text :: :: call), range = (arcana_text.types.TextRange :: start = seed.start, end = seed.start :: call), text = "" :: call
    run.style = seed.span_style
    run.direction = seed.direction
    run.script = seed.script
    run.bidi_level = seed.bidi_level
    run.language_tag = seed.language_tag
    run.plan_key = arcana_text.shape.glyphs.plan_key :: key_request :: call
    run.match = seed.matched
    run.glyphs = empty_glyphs :: :: call
    run.width = 0
    run.whitespace = seed.whitespace
    run.hard_break = false
    run.placeholder = Option.None[arcana_text.types.PlaceholderSpec] :: :: call
    return run

fn same_run_shape(read run: arcana_text.types.ShapedRun, read key: arcana_text.shape.glyphs.RunShapeKey) -> Bool:
    return run.match.id.source_index == key.matched.id.source_index and run.match.id.face_index == key.matched.id.face_index and run.script == key.script and run.direction == key.direction and run.language_tag == key.language_tag and run.bidi_level == key.bidi_level

fn same_match(read left: arcana_text.types.FontMatch, read right: arcana_text.types.FontMatch) -> Bool:
    return left.id.source_index == right.id.source_index and left.id.face_index == right.id.face_index

fn copy_resolved_cluster(read value: arcana_text.shape.glyphs.ResolvedCluster) -> arcana_text.shape.glyphs.ResolvedCluster:
    return value

fn finalize_text_run(edit out: List[arcana_text.types.PreparedRun], read run: arcana_text.types.ShapedRun, read unresolved: List[arcana_text.types.UnresolvedGlyph]):
    if run.range.end <= run.range.start and (run.glyphs :: :: is_empty):
        return
    out :: (arcana_text.types.PreparedRun :: run = run, unresolved = unresolved :: call) :: push

fn append_existing_cluster_glyphs(edit run: arcana_text.types.ShapedRun, read source: arcana_text.types.ShapedRun, read cluster: arcana_text.shape.glyphs.ResolvedCluster):
    run.range = arcana_text.types.TextRange :: start = run.range.start, end = cluster.range.end :: call
    run.text = run.text + cluster.text
    for glyph in source.glyphs:
        if glyph.cluster_range.start == cluster.range.start and glyph.cluster_range.end == cluster.range.end:
            let advance = arcana_text.shape.glyphs.glyph_primary_advance :: glyph :: call
            run.glyphs :: glyph :: push
            run.width += advance

fn append_cluster_unresolved(edit out: List[arcana_text.types.UnresolvedGlyph], read unresolved: List[arcana_text.types.UnresolvedGlyph], read cluster: arcana_text.shape.glyphs.ResolvedCluster):
    for value in unresolved:
        if value.index >= cluster.range.start and value.index < cluster.range.end:
            out :: value :: push

fn empty_seed_glyphs() -> List[arcana_text.shape.glyphs.GlyphSeed]:
    return std.collections.list.empty[arcana_text.shape.glyphs.GlyphSeed] :: :: call

fn text_is_empty_glyph(read text: Str) -> Bool:
    if text == " " or text == "\t" or text == "\n" or text == "\r":
        return true
    if (std.text.len_bytes :: text :: call) <= 0:
        return true
    let codepoint = arcana_text.text_units.codepoint_at :: text, 0 :: call
    if arcana_text.text_units.is_newline_codepoint :: codepoint :: call:
        return true
    return arcana_text.text_units.is_format_control_codepoint :: codepoint :: call

fn text_is_zero_advance_glyph(read text: Str) -> Bool:
    if (std.text.len_bytes :: text :: call) <= 0:
        return true
    let codepoint = arcana_text.text_units.codepoint_at :: text, 0 :: call
    return arcana_text.text_units.is_format_control_codepoint :: codepoint :: call

fn seed_glyph_for_scalar(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.SeedGlyphRequest) -> arcana_text.shape.glyphs.GlyphSeed:
    let shape_text = arcana_text.shape.glyphs.mirrored_scalar_text_resolved :: request.text, request.direction :: call
    let mut glyph_index = -1
    if request.matched.id.source_index >= 0:
        glyph_index = fonts :: request.matched, shape_text :: glyph_index
    let unresolved = glyph_index <= 0 and not (arcana_text.shape.glyphs.text_is_empty_glyph :: request.text :: call)
    let mut glyph = arcana_text.shape.glyphs.GlyphSeed :: text = request.text, range = request.range, cluster_range = request.cluster_range :: call
    glyph.glyph_index = glyph_index
    glyph.caret_stop_before = request.caret_stop_before
    glyph.caret_stop_after = request.caret_stop_after
    glyph.unresolved = unresolved
    return glyph

fn seed_run_glyphs(edit fonts: arcana_text.fonts.FontSystem, read run: arcana_text.types.ShapedRun, read style: arcana_text.types.TextStyle) -> arcana_text.shape.glyphs.SeededRun:
    let mut glyphs = arcana_text.shape.glyphs.empty_seed_glyphs :: :: call
    let mut unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    let total = std.text.len_bytes :: run.text :: call
    let mut cluster_index = 0
    while cluster_index < total:
        let cluster_end = arcana_text.text_units.next_cluster_end :: run.text, cluster_index :: call
        let cluster_range = arcana_text.types.TextRange :: start = run.range.start + cluster_index, end = run.range.start + cluster_end :: call
        let mut scalar = cluster_index
        while scalar < cluster_end:
            let scalar_end = arcana_text.text_units.next_scalar_end :: run.text, scalar :: call
            let ch = std.text.slice_bytes :: run.text, scalar, scalar_end :: call
            let mut request = arcana_text.shape.glyphs.SeedGlyphRequest :: style = style, matched = run.match, script = run.script :: call
            request.direction = run.direction
            request.text = ch
            request.range = arcana_text.types.TextRange :: start = run.range.start + scalar, end = run.range.start + scalar_end :: call
            request.cluster_range = cluster_range
            request.caret_stop_before = scalar == cluster_index
            request.caret_stop_after = scalar_end >= cluster_end
            let seed = arcana_text.shape.glyphs.seed_glyph_for_scalar :: fonts, request :: call
            let seed_start = seed.range.start
            let seed_text = seed.text
            if seed.unresolved:
                unresolved :: (arcana_text.types.UnresolvedGlyph :: index = seed_start, glyph = seed_text, reason = "missing glyph" :: call) :: push
            glyphs :: seed :: push
            scalar = scalar_end
        cluster_index = cluster_end
    return arcana_text.shape.glyphs.SeededRun :: glyphs = glyphs, unresolved = unresolved :: call

fn seed_glyph_indexes(read glyphs: List[arcana_text.shape.glyphs.GlyphSeed]) -> List[Int]:
    let mut out = arcana_text.shape.glyphs.empty_indexes :: :: call
    for glyph in glyphs:
        out :: glyph.glyph_index :: push
    return out

fn seed_span_text(read glyphs: List[arcana_text.shape.glyphs.GlyphSeed], start_index: Int, consumed: Int) -> Str:
    let mut text = ""
    let mut index = 0
    while index < consumed:
        text = text + (glyphs)[start_index + index].text
        index += 1
    return text

fn shaped_glyph_from_seed_span(edit fonts: arcana_text.fonts.FontSystem, read seeds: List[arcana_text.shape.glyphs.GlyphSeed], read request: arcana_text.shape.glyphs.SeedGlyphBuildRequest) -> arcana_text.types.ShapedGlyph:
    let first = (seeds)[request.start_index]
    let last = (seeds)[request.start_index + request.consumed - 1]
    let text = arcana_text.shape.glyphs.seed_span_text :: seeds, request.start_index, request.consumed :: call
    let features = arcana_text.shape.glyphs.effective_features :: request.script, request.style.features :: call
    let traits = arcana_text.fonts.style_traits_for :: request.style :: call
    let line_height_milli = arcana_text.fonts.style_line_height_milli :: request.style :: call
    let vertical = arcana_text.shape.glyphs.vertical_features_enabled :: features :: call
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: text, request.style.size, line_height_milli :: call
    spec.glyph_index = request.glyph_index
    spec.traits = traits
    spec.feature_signature = arcana_text.types.feature_signature :: features :: call
    spec.axis_signature = arcana_text.types.axis_signature :: request.style.axes :: call
    spec.vertical = vertical
    let fallback = empty_bitmap :: request.style :: call
    let mut advance = fallback.advance
    let mut hinted_advance = fallback.advance
    let mut baseline = fallback.baseline
    let mut line_height = fallback.line_height
    let mut ink_offset = (0, 0)
    let mut ink_size = (0, 0)
    let mut empty = fallback.empty
    if request.matched.id.source_index >= 0:
        baseline = fonts :: request.matched, request.style :: baseline
        line_height = fonts :: request.matched, request.style :: line_height
        advance = fonts :: request.matched.id, spec :: advance_face_glyph
        hinted_advance = arcana_text.shape.glyphs.hinted_advance_for_spec :: fonts, request.matched.id, spec :: call
        let measured = arcana_text.shape.glyphs.hinted_measure_for_spec :: fonts, request.matched.id, spec :: call
        ink_offset = measured.offset
        ink_size = measured.size
        baseline = measured.baseline
        line_height = measured.line_height
        empty = measured.empty or request.glyph_index <= 0 or (arcana_text.shape.glyphs.text_is_empty_glyph :: text :: call)
    if arcana_text.shape.glyphs.text_is_zero_advance_glyph :: text :: call:
        advance = 0
        hinted_advance = 0
    let mut family = ""
    if request.matched.id.source_index >= 0:
        family = arcana_text.fonts.match_family_or_label :: fonts, request.matched :: call
    let mut glyph = arcana_text.types.ShapedGlyph :: glyph = text, range = (arcana_text.types.TextRange :: start = first.range.start, end = last.range.end :: call), family = family :: call
    glyph.face_id = request.matched.id
    glyph.glyph_index = request.glyph_index
    glyph.cluster_range = arcana_text.types.TextRange :: start = first.cluster_range.start, end = last.cluster_range.end :: call
    glyph.font_size = request.style.size
    glyph.line_height_milli = line_height_milli
    glyph.weight = traits.weight
    glyph.width_milli = traits.width_milli
    glyph.slant_milli = traits.slant_milli
    glyph.feature_signature = spec.feature_signature
    glyph.axis_signature = spec.axis_signature
    glyph.advance = advance + request.style.letter_spacing
    glyph.x_advance = match vertical:
        true => 0
        false => hinted_advance + request.style.letter_spacing
    glyph.y_advance = match vertical:
        true => hinted_advance + request.style.letter_spacing
        false => 0
    glyph.offset = (0, 0)
    glyph.ink_offset = ink_offset
    glyph.ink_size = ink_size
    glyph.baseline = baseline
    glyph.line_height = line_height
    glyph.caret_stop_before = first.caret_stop_before
    glyph.caret_stop_after = last.caret_stop_after
    glyph.empty = empty
    return glyph

fn apply_single_position(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.SinglePositionRequest) -> arcana_text.types.ShapedGlyph:
    let glyph = request.glyph
    if glyph.face_id.source_index < 0 or glyph.glyph_index <= 0:
        return glyph
    let mut single_request = arcana_text.fonts.SingleAdjustRequest :: id = glyph.face_id, glyph = glyph.glyph_index, script_tag = (arcana_text.shape.glyphs.script_tag :: request.script :: call) :: call
    single_request.language_tag = request.language_tag
    single_request.features = request.features
    single_request.font_size = glyph.font_size
    single_request.width_milli = glyph.width_milli
    let adjust = fonts :: single_request :: single_adjust
    let mut next = glyph
    next.offset = (next.offset.0 + adjust.x_offset, next.offset.1 + adjust.y_offset)
    next.advance += adjust.x_advance
    next.x_advance += adjust.x_advance
    next.y_advance += adjust.y_advance
    if adjust.zero_advance:
        next.advance = 0
        next.x_advance = 0
        next.y_advance = 0
    return next

fn placement_has_effect(read value: arcana_text.font_leaf.PairPlacement) -> Bool:
    return value.zero_advance or value.attach_to_left_origin or value.x_offset != 0 or value.y_offset != 0 or value.x_advance != 0 or value.y_advance != 0

fn same_cluster_range(read left: arcana_text.types.TextRange, read right: arcana_text.types.TextRange) -> Bool:
    return left.start == right.start and left.end == right.end

fn copy_position_lookup_steps(read values: List[arcana_text.shape.glyphs.PositionLookupStep]) -> List[arcana_text.shape.glyphs.PositionLookupStep]:
    let mut out = arcana_text.shape.glyphs.empty_position_lookup_steps :: :: call
    out :: values :: extend_list
    return out

fn glyph_primary_advance(read glyph: arcana_text.types.ShapedGlyph) -> Int:
    if glyph.y_advance > 0 and glyph.x_advance == 0:
        return glyph.y_advance
    if glyph.x_advance > 0:
        return glyph.x_advance
    return glyph.advance

fn glyph_class(read fonts: arcana_text.fonts.FontSystem, read glyph: arcana_text.types.ShapedGlyph) -> Int:
    if glyph.face_id.source_index < 0 or glyph.glyph_index <= 0:
        return 0
    return fonts :: (arcana_text.fonts.GlyphClassRequest :: id = glyph.face_id, glyph_index = glyph.glyph_index :: call) :: glyph_class

fn glyph_prefers_attachment_search(read glyph: arcana_text.types.ShapedGlyph) -> Bool:
    if glyph.range.start > glyph.cluster_range.start:
        return true
    if not glyph.caret_stop_before:
        return true
    if glyph.glyph == "":
        return false
    let codepoint = arcana_text.text_units.codepoint_at :: glyph.glyph, 0 :: call
    return arcana_text.text_units.is_cluster_extension :: codepoint :: call

fn lookup_prefers_attachment_search(lookup_type: Int, read glyph: arcana_text.types.ShapedGlyph) -> Bool:
    if lookup_type == 4 or lookup_type == 5 or lookup_type == 6:
        return arcana_text.shape.glyphs.glyph_prefers_attachment_search :: glyph :: call
    return false

fn lookup_accepts_left_class(lookup_type: Int, glyph_class: Int) -> Bool:
    if lookup_type == 4:
        return glyph_class == 1
    if lookup_type == 5:
        return glyph_class == 2
    if lookup_type == 6:
        return glyph_class == 3
    return true

fn ordered_position_steps(edit fonts: arcana_text.fonts.FontSystem, read run: arcana_text.types.ShapedRun, read features: List[arcana_text.types.FontFeature]) -> List[arcana_text.shape.glyphs.PositionLookupStep]:
    let mut lookup_request = arcana_text.fonts.PositionLookupsRequest :: id = run.match.id, script_tag = (arcana_text.shape.glyphs.script_tag :: run.script :: call) :: call
    lookup_request.language_tag = run.language_tag
    lookup_request.features = features
    let lookups = fonts :: lookup_request :: position_lookups
    let mut steps = arcana_text.shape.glyphs.empty_position_lookup_steps :: :: call
    for lookup in lookups:
        let lookup_type = fonts :: (arcana_text.fonts.LookupTypeRequest :: id = run.match.id, lookup = lookup :: call) :: lookup_type
        steps :: (arcana_text.shape.glyphs.PositionLookupStep :: lookup = lookup, lookup_type = lookup_type :: call) :: push
    return steps

fn apply_scaled_adjustment(read glyph: arcana_text.types.ShapedGlyph, read left: arcana_text.types.ShapedGlyph, read adjust: arcana_text.font_leaf.PairPlacement) -> arcana_text.types.ShapedGlyph:
    let mut next = glyph
    let left_advance = arcana_text.shape.glyphs.glyph_primary_advance :: left :: call
    let offset_x = match adjust.attach_to_left_origin:
        true => next.offset.0 + adjust.x_offset - left_advance
        false => next.offset.0 + adjust.x_offset
    next.offset = (offset_x, next.offset.1 + adjust.y_offset)
    next.advance += adjust.x_advance
    next.x_advance += adjust.x_advance
    next.y_advance += adjust.y_advance
    if adjust.zero_advance:
        next.advance = 0
        next.x_advance = 0
        next.y_advance = 0
    return next

fn apply_ordered_lookup(edit fonts: arcana_text.fonts.FontSystem, read payload: (arcana_text.shape.glyphs.PositionLookupStep, List[arcana_text.types.ShapedGlyph], arcana_text.types.ShapedGlyph)) -> arcana_text.types.ShapedGlyph:
    let step = payload.0
    let previous = payload.1
    let glyph = payload.2
    if glyph.face_id.source_index < 0 or glyph.glyph_index <= 0:
        return glyph
    let lookup_type = step.lookup_type
    if lookup_type == 1:
        let adjust = fonts :: (arcana_text.fonts.LookupPlacementRequest :: id = glyph.face_id, lookup = step.lookup, left_glyph = glyph.glyph_index, right_glyph = 0, font_size = glyph.font_size, width_milli = glyph.width_milli :: call) :: lookup_placement
        if not (arcana_text.shape.glyphs.placement_has_effect :: adjust :: call):
            return glyph
        return arcana_text.shape.glyphs.apply_scaled_adjustment :: glyph, glyph, adjust :: call
    let total = previous :: :: len
    if total <= 0:
        return glyph
    let right_class = arcana_text.shape.glyphs.glyph_class :: fonts, glyph :: call
    let mut allow_backward = arcana_text.shape.glyphs.lookup_prefers_attachment_search :: lookup_type, glyph :: call
    if not allow_backward and right_class == 3:
        if lookup_type == 4 or lookup_type == 5 or lookup_type == 6:
            allow_backward = true
    let mut candidate_index = total - 1
    while candidate_index >= 0:
        let left = (previous)[candidate_index]
        if allow_backward and not (arcana_text.shape.glyphs.same_cluster_range :: left.cluster_range, glyph.cluster_range :: call):
            break
        if left.face_id.source_index >= 0 and left.face_id.source_index == glyph.face_id.source_index and left.face_id.face_index == glyph.face_id.face_index:
            let left_class = arcana_text.shape.glyphs.glyph_class :: fonts, left :: call
            if arcana_text.shape.glyphs.lookup_accepts_left_class :: lookup_type, left_class :: call:
                let adjust = fonts :: (arcana_text.fonts.LookupPlacementRequest :: id = glyph.face_id, lookup = step.lookup, left_glyph = left.glyph_index, right_glyph = glyph.glyph_index, font_size = glyph.font_size, width_milli = glyph.width_milli :: call) :: lookup_placement
                if arcana_text.shape.glyphs.placement_has_effect :: adjust :: call:
                    return arcana_text.shape.glyphs.apply_scaled_adjustment :: glyph, left, adjust :: call
        if lookup_type == 2 or lookup_type == 3:
            break
        if not allow_backward:
            break
        candidate_index -= 1
    return glyph

fn apply_pair_position(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.PairPositionRequest) -> arcana_text.types.ShapedGlyph:
    let right = request.right
    if right.face_id.source_index < 0:
        return right
    let total = request.previous :: :: len
    if total <= 0:
        return right
    let allow_backward = arcana_text.shape.glyphs.glyph_prefers_attachment_search :: right :: call
    let mut candidate_index = total - 1
    while candidate_index >= 0:
        let left = (request.previous)[candidate_index]
        if left.face_id.source_index >= 0 and left.face_id.source_index == right.face_id.source_index and left.face_id.face_index == right.face_id.face_index:
            let mut pair_request = arcana_text.fonts.PairAdjustRequest :: id = right.face_id, left_glyph = left.glyph_index, right_glyph = right.glyph_index, script_tag = (arcana_text.shape.glyphs.script_tag :: request.script :: call) :: call
            pair_request.language_tag = request.language_tag
            pair_request.features = request.features
            pair_request.font_size = right.font_size
            pair_request.width_milli = right.width_milli
            let adjust = fonts :: pair_request :: pair_adjust
            if arcana_text.shape.glyphs.placement_has_effect :: adjust :: call:
                let mut next = right
                let left_advance = arcana_text.shape.glyphs.glyph_primary_advance :: left :: call
                let offset_x = match adjust.attach_to_left_origin:
                    true => next.offset.0 + adjust.x_offset - left_advance
                    false => next.offset.0 + adjust.x_offset
                next.offset = (offset_x, next.offset.1 + adjust.y_offset)
                next.advance += adjust.x_advance
                next.x_advance += adjust.x_advance
                next.y_advance += adjust.y_advance
                if adjust.zero_advance:
                    next.advance = 0
                    next.x_advance = 0
                    next.y_advance = 0
                return next
        if not allow_backward:
            break
        if not (arcana_text.shape.glyphs.same_cluster_range :: left.cluster_range, right.cluster_range :: call):
            break
        candidate_index -= 1
    return right

fn position_run_glyphs(edit fonts: arcana_text.fonts.FontSystem, read run: arcana_text.types.ShapedRun, read style: arcana_text.types.TextStyle) -> arcana_text.types.ShapedRun:
    let features = arcana_text.shape.glyphs.effective_features :: run.script, style.features :: call
    let steps = arcana_text.shape.glyphs.ordered_position_steps :: fonts, run, (arcana_text.shape.glyphs.copy_features :: features :: call) :: call
    let mut has_pair_like_lookup = false
    let pair_like_scan = arcana_text.shape.glyphs.copy_position_lookup_steps :: steps :: call
    for step in pair_like_scan:
        if step.lookup_type == 2 or step.lookup_type == 3 or step.lookup_type == 4 or step.lookup_type == 5 or step.lookup_type == 6:
            has_pair_like_lookup = true
    let mut next = run
    next.width = 0
    next.glyphs = empty_glyphs :: :: call
    let mut index = 0
    while index < (run.glyphs :: :: len):
        let raw = (run.glyphs)[index]
        let mut positioned = raw
        let previous = arcana_text.shape.glyphs.copy_shaped_glyphs :: next.glyphs :: call
        let step_values = arcana_text.shape.glyphs.copy_position_lookup_steps :: steps :: call
        for step in step_values:
            positioned = arcana_text.shape.glyphs.apply_ordered_lookup :: fonts, (step, previous, positioned) :: call
        if index > 0:
            if (steps :: :: is_empty) or not has_pair_like_lookup:
                positioned = arcana_text.shape.glyphs.apply_pair_position :: fonts, (arcana_text.shape.glyphs.PairPositionRequest :: script = run.script, language_tag = run.language_tag, features = (arcana_text.shape.glyphs.copy_features :: features :: call), previous = previous, right = positioned :: call) :: call
        next.width += arcana_text.shape.glyphs.glyph_primary_advance :: positioned :: call
        next.glyphs :: positioned :: push
        index += 1
    return next

fn shape_run_from_units(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.ShapeRunFromUnitsRequest) -> arcana_text.types.ShapedRun:
    let run = request.run
    let style = request.style
    let seeds = request.seeds
    let units = request.units
    if seeds :: :: is_empty:
        let mut empty_run = run
        empty_run.glyphs = empty_glyphs :: :: call
        empty_run.width = 0
        empty_run.text = ""
        return empty_run
    let mut next = run
    next.glyphs = empty_glyphs :: :: call
    next.text = ""
    next.width = 0
    let mut source_index = 0
    let mut active_start = 0
    let mut active_consumed = 0
    for unit in units:
        if unit.consumed > 0:
            if source_index + unit.consumed > (seeds :: :: len):
                break
            active_start = source_index
            active_consumed = unit.consumed
            source_index += unit.consumed
        else:
            if active_consumed <= 0:
                continue
        let mut glyph_request = arcana_text.shape.glyphs.SeedGlyphBuildRequest :: style = style, matched = run.match, script = run.script :: call
        glyph_request.direction = run.direction
        glyph_request.start_index = active_start
        glyph_request.consumed = arcana_text.shape.types.max_int :: active_consumed, 1 :: call
        glyph_request.glyph_index = unit.glyph_index
        let glyph = arcana_text.shape.glyphs.shaped_glyph_from_seed_span :: fonts, seeds, glyph_request :: call
        next.text = next.text + glyph.glyph
        next.range = arcana_text.types.TextRange :: start = next.range.start, end = glyph.range.end :: call
        next.glyphs :: glyph :: push
    return arcana_text.shape.glyphs.position_run_glyphs :: fonts, next, style :: call

fn cluster_has_unresolved(read unresolved: List[arcana_text.types.UnresolvedGlyph], read cluster: arcana_text.shape.glyphs.ResolvedCluster) -> Bool:
    for value in unresolved:
        if value.index >= cluster.range.start and value.index < cluster.range.end:
            return true
    return false

fn finalize_segment(edit fonts: arcana_text.fonts.FontSystem, edit out: List[arcana_text.types.PreparedRun], read request: arcana_text.shape.glyphs.FinalizeSegmentRequest):
    let run = request.run
    if run.range.end <= run.range.start and run.text == "":
        return
    if request.reshaped:
        out :: (arcana_text.shape.glyphs.prepare_text_run :: fonts, run, request.style :: call) :: push
        return
    out :: (arcana_text.types.PreparedRun :: run = run, unresolved = request.unresolved :: call) :: push

fn fallback_match_for_cluster(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.FallbackMatchRequest) -> arcana_text.types.FontMatch:
        let matches = fonts :: (request.style, request.cluster.text, request.primary, request.script_tag) :: resolve_style_text_script_fallbacks
    for matched in matches:
        if not (arcana_text.shape.glyphs.same_match :: matched, request.primary :: call):
            return matched
    return request.primary

fn fallback_prepared_runs(edit fonts: arcana_text.fonts.FontSystem, read prepared: arcana_text.types.PreparedRun, read style: arcana_text.types.TextStyle) -> List[arcana_text.types.PreparedRun]:
    let unresolved_all = prepared.unresolved
    let base = prepared.run
    if unresolved_all :: :: is_empty:
        let mut out = arcana_text.shape.glyphs.empty_prepared_runs :: :: call
        out :: (arcana_text.types.PreparedRun :: run = base, unresolved = unresolved_all :: call) :: push
        return out
    let clusters = arcana_text.shape.glyphs.resolved_clusters_for_text :: base.text, base.range.start :: call
    let mut out = arcana_text.shape.glyphs.empty_prepared_runs :: :: call
    let mut active = false
    let mut current = base
    current.range = arcana_text.types.TextRange :: start = base.range.start, end = base.range.start :: call
    current.text = ""
    current.glyphs = empty_glyphs :: :: call
    current.width = 0
    let mut current_unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    let mut current_reshaped = false
    for cluster in clusters:
        let cluster_missing = arcana_text.shape.glyphs.cluster_has_unresolved :: unresolved_all, cluster :: call
        let cluster_script = arcana_text.shape.glyphs.script_for_text :: cluster.text :: call
        let language_tag = arcana_text.shape.glyphs.default_language_tag :: (fonts :: :: locale), cluster_script :: call
        let cluster_copy = arcana_text.shape.glyphs.copy_resolved_cluster :: cluster :: call
        let cluster_match = match cluster_missing:
            true => arcana_text.shape.glyphs.fallback_match_for_cluster :: fonts, (arcana_text.shape.glyphs.FallbackMatchRequest :: style = style, cluster = cluster_copy, primary = base.match, script_tag = (arcana_text.shape.glyphs.script_tag :: cluster_script :: call) :: call) :: call
            false => base.match
        let cluster_reshaped = cluster_missing and not (arcana_text.shape.glyphs.same_match :: cluster_match, base.match :: call)
        let mut shape_key = arcana_text.shape.glyphs.RunShapeKey :: matched = cluster_match, script = cluster_script, direction = cluster.direction :: call
        shape_key.language_tag = language_tag
        shape_key.bidi_level = cluster.bidi_level
        if not active or current_reshaped != cluster_reshaped or not (arcana_text.shape.glyphs.same_run_shape :: current, shape_key :: call):
            if active:
                let mut request = arcana_text.shape.glyphs.FinalizeSegmentRequest :: run = current, unresolved = current_unresolved, style = style :: call
                request.reshaped = current_reshaped
                arcana_text.shape.glyphs.finalize_segment :: fonts, out, request :: call
            let mut seed = arcana_text.shape.glyphs.RunSeed :: span_style = base.style, style = style, matched = cluster_match :: call
            seed.script = cluster_script
            seed.direction = cluster.direction
            seed.language_tag = language_tag
            seed.bidi_level = cluster.bidi_level
            seed.start = cluster.range.start
            seed.whitespace = base.whitespace
            current = arcana_text.shape.glyphs.text_run :: seed :: call
            current_unresolved = arcana_text.shape.types.empty_unresolved :: :: call
            current_reshaped = cluster_reshaped
            active = true
        if cluster_reshaped:
            let mut cluster_seed = arcana_text.shape.glyphs.ClusterSeed :: text = cluster.text, range = cluster.range, matched = cluster_match :: call
            cluster_seed.script = cluster_script
            cluster_seed.direction = cluster.direction
            cluster_seed.language_tag = language_tag
            cluster_seed.bidi_level = cluster.bidi_level
            let request = arcana_text.shape.glyphs.AppendClusterToRunRequest :: unresolved = current_unresolved, cluster = cluster_seed, style = style :: call
            current_unresolved = arcana_text.shape.glyphs.append_cluster_to_run :: fonts, current, request :: call
        else:
            arcana_text.shape.glyphs.append_existing_cluster_glyphs :: current, base, cluster :: call
            arcana_text.shape.glyphs.append_cluster_unresolved :: current_unresolved, unresolved_all, cluster :: call
    if active:
        let mut request = arcana_text.shape.glyphs.FinalizeSegmentRequest :: run = current, unresolved = current_unresolved, style = style :: call
        request.reshaped = current_reshaped
        arcana_text.shape.glyphs.finalize_segment :: fonts, out, request :: call
    return out

fn append_cluster_text(edit run: arcana_text.types.ShapedRun, read cluster: arcana_text.shape.glyphs.ClusterSeed):
    run.range = arcana_text.types.TextRange :: start = run.range.start, end = cluster.range.end :: call
    run.text = run.text + cluster.text

fn prepare_text_run(edit fonts: arcana_text.fonts.FontSystem, read run: arcana_text.types.ShapedRun, read style: arcana_text.types.TextStyle) -> arcana_text.types.PreparedRun:
    let seeded = arcana_text.shape.glyphs.seed_run_glyphs :: fonts, run, style :: call
    let seed_glyphs = arcana_text.shape.glyphs.copy_seed_glyphs :: seeded.glyphs :: call
    let mut units = arcana_text.font_leaf.default_gsub_units :: (arcana_text.shape.glyphs.seed_glyph_indexes :: seed_glyphs :: call) :: call
    let features = arcana_text.shape.glyphs.effective_features :: run.script, style.features :: call
    let shaping_features = arcana_text.shape.glyphs.shaping_features :: run.script, style.features :: call
    if run.script == (arcana_text.types.ScriptClass.Arabic :: :: call):
        let mut join_request = arcana_text.shape.glyphs.ArabicJoinRequest :: seeds = (arcana_text.shape.glyphs.copy_seed_glyphs :: seed_glyphs :: call), matched = run.match, script = run.script :: call
        join_request.language_tag = run.language_tag
        join_request.features = arcana_text.shape.glyphs.copy_features :: features :: call
        join_request.units = units
        units = arcana_text.shape.glyphs.apply_arabic_join_forms :: fonts, join_request :: call
    let mut gsub_request = arcana_text.fonts.GsubUnitsRequest :: matched = run.match, script_tag = (arcana_text.shape.glyphs.script_tag :: run.script :: call), language_tag = run.language_tag :: call
    gsub_request.features = shaping_features
    gsub_request.glyphs = arcana_text.shape.glyphs.seed_glyph_indexes :: seed_glyphs :: call
    if run.script == (arcana_text.types.ScriptClass.Arabic :: :: call):
        let mut prepared = std.collections.list.empty[Int] :: :: call
        for unit in units:
            prepared :: unit.glyph_index :: push
        gsub_request.glyphs = prepared
    let units = fonts :: gsub_request :: gsub_units
    let next = arcana_text.shape.glyphs.shape_run_from_units :: fonts, (arcana_text.shape.glyphs.ShapeRunFromUnitsRequest :: run = run, style = style, seeds = seed_glyphs, units = units :: call) :: call
    return arcana_text.types.PreparedRun :: run = next, unresolved = seeded.unresolved :: call

fn finalize_active_text_run(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.FinalizeActiveRunRequest) -> Option[arcana_text.types.PreparedRun]:
    let run = request.run
    if run.range.end <= run.range.start and run.text == "":
        return Option.None[arcana_text.types.PreparedRun] :: :: call
    let prepared = arcana_text.shape.glyphs.prepare_text_run :: fonts, run, request.style :: call
    return Option.Some[arcana_text.types.PreparedRun] :: prepared :: call

fn append_cluster_to_run(edit fonts: arcana_text.fonts.FontSystem, edit run: arcana_text.types.ShapedRun, read request: arcana_text.shape.glyphs.AppendClusterToRunRequest) -> List[arcana_text.types.UnresolvedGlyph]:
    let _ = fonts
    let _ = request.style
    arcana_text.shape.glyphs.append_cluster_text :: run, request.cluster :: call
    return request.unresolved

fn close_cluster_in_run(edit run: arcana_text.types.ShapedRun):
    if run.glyphs :: :: is_empty:
        return
    let mut previous = run.glyphs :: :: pop
    previous.caret_stop_after = false
    run.glyphs :: previous :: push

fn empty_indexes() -> List[Int]:
    return std.collections.list.empty[Int] :: :: call

fn glyph_indexes(read glyphs: List[arcana_text.types.ShapedGlyph]) -> List[Int]:
    let mut out = arcana_text.shape.glyphs.empty_indexes :: :: call
    for glyph in glyphs:
        out :: glyph.glyph_index :: push
    return out

fn substitution_matches_run(read units: List[arcana_text.font_leaf.GsubGlyphUnit], read glyphs: List[arcana_text.types.ShapedGlyph]) -> Bool:
    if (units :: :: len) != (glyphs :: :: len):
        return false
    let mut index = 0
    while index < (glyphs :: :: len):
        let unit = (units)[index]
        let glyph = (glyphs)[index]
        if unit.consumed != 1:
            return false
        if unit.glyph_index != glyph.glyph_index:
            return false
        index += 1
    return true

fn span_glyph_text(read glyphs: List[arcana_text.types.ShapedGlyph], start_index: Int, consumed: Int) -> Str:
    let mut text = ""
    let mut index = 0
    while index < consumed:
        text = text + (glyphs)[start_index + index].glyph
        index += 1
    return text

fn restyled_glyph(edit fonts: arcana_text.fonts.FontSystem, read request: arcana_text.shape.glyphs.RestyledGlyphRequest) -> arcana_text.types.ShapedGlyph:
    let first = (request.glyphs)[request.start_index]
    let last = (request.glyphs)[request.start_index + request.consumed - 1]
    let text = arcana_text.shape.glyphs.span_glyph_text :: request.glyphs, request.start_index, request.consumed :: call
    let features = arcana_text.shape.glyphs.effective_features :: request.script, request.style.features :: call
    let traits = arcana_text.fonts.style_traits_for :: request.style :: call
    let line_height_milli = arcana_text.fonts.style_line_height_milli :: request.style :: call
    let vertical = arcana_text.shape.glyphs.vertical_features_enabled :: features :: call
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: text, request.style.size, line_height_milli :: call
    spec.glyph_index = request.glyph_index
    spec.traits = traits
    spec.feature_signature = arcana_text.types.feature_signature :: features :: call
    spec.axis_signature = arcana_text.types.axis_signature :: request.style.axes :: call
    spec.vertical = vertical
    let fallback = empty_bitmap :: request.style :: call
    let mut advance = fallback.advance
    let mut hinted_advance = fallback.advance
    let mut baseline = fallback.baseline
    let mut line_height = fallback.line_height
    let mut ink_offset = (0, 0)
    let mut ink_size = (0, 0)
    let mut empty = fallback.empty
    if request.matched.id.source_index >= 0:
        baseline = fonts :: request.matched, request.style :: baseline
        line_height = fonts :: request.matched, request.style :: line_height
        advance = fonts :: request.matched.id, spec :: advance_face_glyph
        hinted_advance = arcana_text.shape.glyphs.hinted_advance_for_spec :: fonts, request.matched.id, spec :: call
        let measured = arcana_text.shape.glyphs.hinted_measure_for_spec :: fonts, request.matched.id, spec :: call
        ink_offset = measured.offset
        ink_size = measured.size
        baseline = measured.baseline
        line_height = measured.line_height
        empty = measured.empty or request.glyph_index <= 0 or (arcana_text.shape.glyphs.text_is_empty_glyph :: text :: call)
    if arcana_text.shape.glyphs.text_is_zero_advance_glyph :: text :: call:
        advance = 0
        hinted_advance = 0
    let mut glyph = first
    glyph.glyph = text
    glyph.range = arcana_text.types.TextRange :: start = first.range.start, end = last.range.end :: call
    glyph.cluster_range = arcana_text.types.TextRange :: start = first.cluster_range.start, end = last.cluster_range.end :: call
    glyph.family = arcana_text.fonts.match_family_or_label :: fonts, request.matched :: call
    glyph.face_id = request.matched.id
    glyph.glyph_index = request.glyph_index
    glyph.font_size = request.style.size
    glyph.line_height_milli = line_height_milli
    glyph.weight = traits.weight
    glyph.width_milli = traits.width_milli
    glyph.slant_milli = traits.slant_milli
    glyph.feature_signature = spec.feature_signature
    glyph.axis_signature = spec.axis_signature
    glyph.advance = advance + request.style.letter_spacing
    glyph.x_advance = match vertical:
        true => 0
        false => hinted_advance + request.style.letter_spacing
    glyph.y_advance = match vertical:
        true => hinted_advance + request.style.letter_spacing
        false => 0
    glyph.offset = (0, 0)
    glyph.ink_offset = ink_offset
    glyph.ink_size = ink_size
    glyph.baseline = baseline
    glyph.line_height = line_height
    glyph.caret_stop_before = first.caret_stop_before
    glyph.caret_stop_after = last.caret_stop_after
    glyph.empty = empty
    return glyph

fn substituted_run(edit fonts: arcana_text.fonts.FontSystem, read run: arcana_text.types.ShapedRun, read style: arcana_text.types.TextStyle) -> arcana_text.types.ShapedRun:
    let prepared = arcana_text.shape.glyphs.prepare_text_run :: fonts, run, style :: call
    return prepared.run

export fn shape_token_runs_in_line(edit fonts: arcana_text.fonts.FontSystem, read payload: arcana_text.shape.glyphs.ShapeTokenRunsInLineRequest) -> List[arcana_text.types.PreparedRun]:
    let token = payload.token
    let span_style = payload.span_style
    let line = payload.line
    shape_probe_append :: ("shape_token_runs:start bytes=" + (std.text.from_int :: (std.text.len_bytes :: token.text :: call) :: call)) :: call
    let cache_key = arcana_text.shape.cache.prepared_run_key :: token, span_style, line.signature :: call
    let cached_runs = fonts.shape_cache :: cache_key :: cached_prepared_runs
    if cached_runs :: :: is_some:
        shape_probe_append :: ("shape_token_runs:cache_hit bytes=" + (std.text.from_int :: (std.text.len_bytes :: token.text :: call) :: call)) :: call
        return arcana_text.shape.glyphs.shift_prepared_runs :: (cached_runs :: (arcana_text.shape.glyphs.empty_prepared_runs :: :: call) :: unwrap_or), token.range.start :: call
    let style = arcana_text.shape.styles.text_style_from_span :: span_style :: call
    let primary = primary_match :: fonts, style, token.text :: call
    shape_probe_append :: ("shape_token_runs:primary source=" + (std.text.from_int :: primary.id.source_index :: call) + " face=" + (std.text.from_int :: primary.id.face_index :: call)) :: call
    let mut runs = arcana_text.shape.glyphs.empty_prepared_runs :: :: call
    if token.newline:
        let mut seed = arcana_text.shape.glyphs.RunSeed :: span_style = span_style, style = style, matched = primary :: call
        seed.script = arcana_text.types.ScriptClass.Common :: :: call
        seed.direction = arcana_text.types.TextDirection.LeftToRight :: :: call
        seed.language_tag = ""
        seed.bidi_level = 0
        seed.start = token.range.start
        seed.whitespace = token.whitespace
        let mut run = arcana_text.shape.glyphs.text_run :: seed :: call
        run.range = token.range
        run.text = token.text
        run.hard_break = true
        runs :: (arcana_text.types.PreparedRun :: run = run, unresolved = (arcana_text.shape.types.empty_unresolved :: :: call) :: call) :: push
        fonts.shape_cache :: cache_key, (arcana_text.shape.glyphs.shift_prepared_runs :: runs, (0 - token.range.start) :: call) :: remember_prepared_runs
        return runs
    let mut active = false
    let mut seed = arcana_text.shape.glyphs.RunSeed :: span_style = span_style, style = style, matched = primary :: call
    seed.script = arcana_text.types.ScriptClass.Common :: :: call
    seed.direction = arcana_text.types.TextDirection.LeftToRight :: :: call
    seed.language_tag = ""
    seed.bidi_level = 0
    seed.start = token.range.start
    seed.whitespace = token.whitespace
    let mut current = arcana_text.shape.glyphs.text_run :: seed :: call
    let mut unresolved = arcana_text.shape.types.empty_unresolved :: :: call
    let clusters = arcana_text.shape.glyphs.resolved_clusters_for_range :: line, token.range :: call
    for resolved in clusters:
        let cluster_text = resolved.text
        shape_probe_append :: ("shape_token_runs:cluster bytes=" + (std.text.from_int :: (std.text.len_bytes :: cluster_text :: call) :: call)) :: call
        let cluster_range = resolved.range
        let cluster_script = arcana_text.shape.glyphs.script_for_text :: cluster_text :: call
        let cluster_match = primary
        shape_probe_append :: ("shape_token_runs:cluster_primary source=" + (std.text.from_int :: cluster_match.id.source_index :: call) + " face=" + (std.text.from_int :: cluster_match.id.face_index :: call)) :: call
        let language_tag = arcana_text.shape.glyphs.default_language_tag :: (fonts :: :: locale), cluster_script :: call
        let mut cluster = arcana_text.shape.glyphs.ClusterSeed :: text = cluster_text, range = cluster_range, matched = cluster_match :: call
        cluster.script = cluster_script
        cluster.direction = resolved.direction
        cluster.language_tag = language_tag
        cluster.bidi_level = resolved.bidi_level
        if not active:
            seed = arcana_text.shape.glyphs.RunSeed :: span_style = span_style, style = style, matched = cluster.matched :: call
            seed.script = cluster.script
            seed.direction = cluster.direction
            seed.language_tag = cluster.language_tag
            seed.bidi_level = cluster.bidi_level
            seed.start = cluster.range.start
            seed.whitespace = token.whitespace
            current = arcana_text.shape.glyphs.text_run :: seed :: call
            unresolved = arcana_text.shape.types.empty_unresolved :: :: call
            active = true
        else:
            let mut shape_key = arcana_text.shape.glyphs.RunShapeKey :: matched = cluster.matched, script = cluster.script, direction = cluster.direction :: call
            shape_key.language_tag = cluster.language_tag
            shape_key.bidi_level = cluster.bidi_level
            if not (arcana_text.shape.glyphs.same_run_shape :: current, shape_key :: call):
                let finalized = arcana_text.shape.glyphs.finalize_active_text_run :: fonts, (arcana_text.shape.glyphs.FinalizeActiveRunRequest :: run = current, unresolved = unresolved, style = style :: call) :: call
                if finalized :: :: is_some:
                    let prepared = finalized :: (arcana_text.types.PreparedRun :: run = current, unresolved = unresolved :: call) :: unwrap_or
                    runs :: (arcana_text.shape.glyphs.fallback_prepared_runs :: fonts, prepared, style :: call) :: extend_list
                seed = arcana_text.shape.glyphs.RunSeed :: span_style = span_style, style = style, matched = cluster.matched :: call
                seed.script = cluster.script
                seed.direction = cluster.direction
                seed.language_tag = cluster.language_tag
                seed.bidi_level = cluster.bidi_level
                seed.start = cluster.range.start
                seed.whitespace = token.whitespace
                current = arcana_text.shape.glyphs.text_run :: seed :: call
                unresolved = arcana_text.shape.types.empty_unresolved :: :: call
        arcana_text.shape.glyphs.append_cluster_text :: current, cluster :: call
    if active:
        let finalized = arcana_text.shape.glyphs.finalize_active_text_run :: fonts, (arcana_text.shape.glyphs.FinalizeActiveRunRequest :: run = current, unresolved = unresolved, style = style :: call) :: call
        if finalized :: :: is_some:
            let prepared = finalized :: (arcana_text.types.PreparedRun :: run = current, unresolved = unresolved :: call) :: unwrap_or
            runs :: (arcana_text.shape.glyphs.fallback_prepared_runs :: fonts, prepared, style :: call) :: extend_list
    shape_probe_append :: ("shape_token_runs:done runs=" + (std.text.from_int :: (runs :: :: len) :: call)) :: call
    fonts.shape_cache :: cache_key, (arcana_text.shape.glyphs.shift_prepared_runs :: runs, (0 - token.range.start) :: call) :: remember_prepared_runs
    return runs

export fn shape_token_runs(edit fonts: arcana_text.fonts.FontSystem, read token: arcana_text.shape.tokens.TextToken, read span_style: arcana_text.types.SpanStyle) -> List[arcana_text.types.PreparedRun]:
    let line = arcana_text.shape.glyphs.resolve_line :: token.text, token.range.start :: call
    return arcana_text.shape.glyphs.shape_token_runs_in_line :: fonts, (arcana_text.shape.glyphs.ShapeTokenRunsInLineRequest :: token = token, span_style = span_style, line = line :: call) :: call

export fn shape_token(edit fonts: arcana_text.fonts.FontSystem, read token: arcana_text.shape.tokens.TextToken, read span_style: arcana_text.types.SpanStyle) -> arcana_text.types.PreparedRun:
    let runs = arcana_text.shape.glyphs.shape_token_runs :: fonts, token, span_style :: call
    for run in runs:
        return run
    return arcana_text.shape.glyphs.shape_placeholder :: span_style, (arcana_text.shape.types.fallback_placeholder :: token.range :: call) :: call

