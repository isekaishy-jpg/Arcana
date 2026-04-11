import std.collections.list
import std.option
import std.text
use std.option.Option

export enum FontSourceKind:
    Asset
    File
    Directory
    Bytes
    Installed

export record FontFaceId:
    source_index: Int
    face_index: Int

export record FontFeature:
    tag: Str
    value: Int
    enabled: Bool

export record FontAxis:
    tag: Str
    value: Int

export record FontSource:
    kind: arcana_text.types.FontSourceKind
    label: Str
    path: Str
    family: Str
    face: Str
    full_name: Str
    postscript_name: Str
    installed: Bool

export record FontMatch:
    id: arcana_text.types.FontFaceId
    source: arcana_text.types.FontSource

export record FontQuery:
    families: List[Str]
    size: Int
    weight: Int
    width_milli: Int
    slant_milli: Int
    features: List[arcana_text.types.FontFeature]
    axes: List[arcana_text.types.FontAxis]

export enum TextAlign:
    Left
    Center
    Right
    Justified
    End

export enum TextDirection:
    LeftToRight
    RightToLeft

export enum ScriptClass:
    Unknown
    Common
    Latin
    Cyrillic
    Arabic
    Hebrew
    Han
    Hangul
    Devanagari
    Adlam
    Bengali
    Bopomofo
    CanadianAboriginal
    Chakma
    Cherokee
    Ethiopic
    Gujarati
    Gurmukhi
    Hiragana
    Katakana
    Javanese
    Kannada
    Khmer
    Lao
    Malayalam
    Mongolian
    Myanmar
    Oriya
    Sinhala
    Tamil
    Telugu
    Thaana
    Thai
    Tibetan
    Tifinagh
    Vai
    Yi

export enum TextWrap:
    NoWrap
    Glyph
    Word
    WordOrGlyph

export enum EllipsizeMode:
    None
    Start
    Middle
    End

export enum EllipsizeLimitKind:
    None
    Lines
    Height

export record EllipsizeHeightLimit:
    kind: arcana_text.types.EllipsizeLimitKind
    value: Int

export enum Hinting:
    Disabled
    Enabled

export enum RasterMode:
    Alpha
    Lcd
    Color

export enum GlyphSurfaceFormat:
    Alpha8
    LcdSubpixel
    Rgba8

export enum PlaceholderAlignment:
    Baseline
    Middle
    Top
    Bottom

export enum TextBaseline:
    Alphabetic
    Ideographic

export enum UnderlineStyle:
    None
    Single
    Double

export record TextRange:
    start: Int
    end: Int

export record SpanStyle:
    color: Int
    background_enabled: Bool
    background_color: Int
    underline: arcana_text.types.UnderlineStyle
    underline_color_enabled: Bool
    underline_color: Int
    strikethrough_enabled: Bool
    strikethrough_color_enabled: Bool
    strikethrough_color: Int
    overline_enabled: Bool
    overline_color_enabled: Bool
    overline_color: Int
    size: Int
    letter_spacing: Int
    line_height: Int
    families: List[Str]
    features: List[arcana_text.types.FontFeature]
    axes: List[arcana_text.types.FontAxis]

export record TextStyle:
    color: Int
    background_enabled: Bool
    background_color: Int
    underline: arcana_text.types.UnderlineStyle
    underline_color_enabled: Bool
    underline_color: Int
    strikethrough_enabled: Bool
    strikethrough_color_enabled: Bool
    strikethrough_color: Int
    overline_enabled: Bool
    overline_color_enabled: Bool
    overline_color: Int
    size: Int
    letter_spacing: Int
    line_height: Int
    families: List[Str]
    features: List[arcana_text.types.FontFeature]
    axes: List[arcana_text.types.FontAxis]

export record ParagraphStyle:
    align: arcana_text.types.TextAlign
    max_lines: Int
    ellipsis: Str
    wrap: arcana_text.types.TextWrap
    ellipsize_mode: arcana_text.types.EllipsizeMode
    ellipsize_limit: arcana_text.types.EllipsizeHeightLimit
    hinting: arcana_text.types.Hinting

export record LayoutConfig:
    max_width: Int
    align: arcana_text.types.TextAlign
    max_lines: Int
    ellipsis: Str
    wrap: arcana_text.types.TextWrap
    tab_width: Int
    ellipsize_mode: arcana_text.types.EllipsizeMode
    ellipsize_limit: arcana_text.types.EllipsizeHeightLimit
    hinting: arcana_text.types.Hinting

export record RasterConfig:
    mode: arcana_text.types.RasterMode
    clip_range: arcana_text.types.TextRange
    draw_backgrounds: Bool
    hinting: arcana_text.types.Hinting

export record Cursor:
    offset: Int
    preferred_x: Int

export record Selection:
    anchor: Int
    focus: Int

export enum SelectionMode:
    Normal
    Word
    Line

export record CompositionRange:
    range: arcana_text.types.TextRange
    caret: Int

export record PlaceholderSpec:
    range: arcana_text.types.TextRange
    size: (Int, Int)
    alignment: arcana_text.types.PlaceholderAlignment
    baseline: arcana_text.types.TextBaseline
    baseline_offset: Int

export record ShapePlanKey:
    face_id: arcana_text.types.FontFaceId
    direction: arcana_text.types.TextDirection
    script: arcana_text.types.ScriptClass
    language_tag: Str
    font_size: Int
    weight: Int
    width_milli: Int
    slant_milli: Int
    feature_signature: Int
    axis_signature: Int

export record TextSpan:
    range: arcana_text.types.TextRange
    style: arcana_text.types.SpanStyle

export record LineMetrics:
    index: Int
    range: arcana_text.types.TextRange
    position: (Int, Int)
    size: (Int, Int)
    baseline: Int

export record SnapshotLine:
    metrics: arcana_text.types.LineMetrics
    text: Str

export enum ShapedRunKind:
    Text
    Placeholder

export record ShapedGlyph:
    glyph: Str
    range: arcana_text.types.TextRange
    cluster_range: arcana_text.types.TextRange
    family: Str
    face_id: arcana_text.types.FontFaceId
    glyph_index: Int
    font_size: Int
    line_height_milli: Int
    weight: Int
    width_milli: Int
    slant_milli: Int
    feature_signature: Int
    axis_signature: Int
    advance: Int
    x_advance: Int
    y_advance: Int
    offset: (Int, Int)
    ink_offset: (Int, Int)
    ink_size: (Int, Int)
    baseline: Int
    line_height: Int
    caret_stop_before: Bool
    caret_stop_after: Bool
    empty: Bool

export record ShapedRun:
    kind: arcana_text.types.ShapedRunKind
    range: arcana_text.types.TextRange
    text: Str
    style: arcana_text.types.SpanStyle
    direction: arcana_text.types.TextDirection
    script: arcana_text.types.ScriptClass
    bidi_level: Int
    language_tag: Str
    plan_key: arcana_text.types.ShapePlanKey
    match: arcana_text.types.FontMatch
    glyphs: List[arcana_text.types.ShapedGlyph]
    width: Int
    whitespace: Bool
    hard_break: Bool
    placeholder: Option[arcana_text.types.PlaceholderSpec]

export record LayoutGlyph:
    glyph: Str
    range: arcana_text.types.TextRange
    cluster_range: arcana_text.types.TextRange
    position: (Int, Int)
    size: (Int, Int)
    advance: Int
    x_advance: Int
    y_advance: Int
    offset: (Int, Int)
    color: Int
    background_enabled: Bool
    background_color: Int
    family: Str
    face_id: arcana_text.types.FontFaceId
    glyph_index: Int
    line_index: Int
    direction: arcana_text.types.TextDirection
    baseline: Int
    font_size: Int
    line_height_milli: Int
    weight: Int
    width_milli: Int
    slant_milli: Int
    feature_signature: Int
    axis_signature: Int
    ink_offset: (Int, Int)
    ink_size: (Int, Int)
    caret_stop_before: Bool
    caret_stop_after: Bool
    empty: Bool

export record LayoutRun:
    kind: arcana_text.types.ShapedRunKind
    range: arcana_text.types.TextRange
    position: (Int, Int)
    size: (Int, Int)
    direction: arcana_text.types.TextDirection
    script: arcana_text.types.ScriptClass
    bidi_level: Int
    language_tag: Str
    color: Int
    baseline: Int
    font_size: Int
    line_height_milli: Int
    underline: arcana_text.types.UnderlineStyle
    underline_color_enabled: Bool
    underline_color: Int
    strikethrough_enabled: Bool
    strikethrough_color_enabled: Bool
    strikethrough_color: Int
    overline_enabled: Bool
    overline_color_enabled: Bool
    overline_color: Int
    family: Str
    face_id: arcana_text.types.FontFaceId
    glyphs: List[arcana_text.types.LayoutGlyph]
    placeholder: Option[arcana_text.types.PlaceholderSpec]

export record HitTest:
    index: Int
    line_index: Int
    position: (Int, Int)
    size: (Int, Int)

export record CaretBox:
    index: Int
    position: (Int, Int)
    size: (Int, Int)

export record RangeBox:
    position: (Int, Int)
    size: (Int, Int)
    range: arcana_text.types.TextRange
    direction: arcana_text.types.TextDirection

export record UnresolvedGlyph:
    index: Int
    glyph: Str
    reason: Str

export record PreparedRun:
    run: arcana_text.types.ShapedRun
    unresolved: List[arcana_text.types.UnresolvedGlyph]

export record PreparedLayoutLine:
    start: Int
    end: Int
    size: (Int, Int)
    signature: Int
    stopped: Bool
    lines: List[arcana_text.types.SnapshotLine]
    runs: List[arcana_text.types.LayoutRun]
    glyphs: List[arcana_text.types.LayoutGlyph]
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    fonts_used: List[arcana_text.types.FontMatch]

export record GlyphDraw:
    text: Str
    position: (Int, Int)
    size: (Int, Int)
    color: Int
    background_enabled: Bool
    background_color: Int
    family: Str

export record DecorationDraw:
    position: (Int, Int)
    size: (Int, Int)
    color: Int

export record GlyphSurface:
    size: (Int, Int)
    stride: Int
    format: arcana_text.types.GlyphSurfaceFormat
    pixels: Array[Int]

export record GlyphImageDraw:
    position: (Int, Int)
    size: (Int, Int)
    mode: arcana_text.types.RasterMode
    color: Int
    surface: arcana_text.types.GlyphSurface

export fn default_font_query() -> arcana_text.types.FontQuery:
    let mut query = arcana_text.types.FontQuery :: families = (std.collections.list.new[Str] :: :: call), size = 16, weight = 400000 :: call
    query.width_milli = 100000
    query.slant_milli = 0
    query.features = std.collections.list.new[arcana_text.types.FontFeature] :: :: call
    query.axes = std.collections.list.new[arcana_text.types.FontAxis] :: :: call
    return query

export fn feature_signature(read features: List[arcana_text.types.FontFeature]) -> Int:
    let mut signature = 17
    for feature in features:
        signature = (signature * 131 + (std.text.len_bytes :: feature.tag :: call) + feature.value + 29) % 2147483629
        if feature.enabled:
            signature = (signature * 131 + 1) % 2147483629
        else:
            signature = (signature * 131 + 2) % 2147483629
    return signature

export fn axis_signature(read axes: List[arcana_text.types.FontAxis]) -> Int:
    let mut signature = 23
    for axis in axes:
        signature = (signature * 131 + (std.text.len_bytes :: axis.tag :: call) + axis.value + 43) % 2147483629
    return signature

export fn default_text_style(color: Int) -> arcana_text.types.TextStyle:
    let mut style = arcana_text.types.TextStyle :: color = color, background_enabled = false, background_color = 0 :: call
    style.underline = arcana_text.types.UnderlineStyle.None :: :: call
    style.underline_color_enabled = false
    style.underline_color = color
    style.strikethrough_enabled = false
    style.strikethrough_color_enabled = false
    style.strikethrough_color = color
    style.overline_enabled = false
    style.overline_color_enabled = false
    style.overline_color = color
    style.size = 18
    style.letter_spacing = 0
    style.line_height = 0
    style.families = std.collections.list.new[Str] :: :: call
    style.features = std.collections.list.new[arcana_text.types.FontFeature] :: :: call
    style.axes = std.collections.list.new[arcana_text.types.FontAxis] :: :: call
    return style

export fn default_paragraph_style() -> arcana_text.types.ParagraphStyle:
    let mut paragraph = arcana_text.types.ParagraphStyle :: align = (arcana_text.types.TextAlign.Left :: :: call), max_lines = 0, ellipsis = "..." :: call
    paragraph.wrap = arcana_text.types.TextWrap.WordOrGlyph :: :: call
    paragraph.ellipsize_mode = arcana_text.types.EllipsizeMode.None :: :: call
    paragraph.ellipsize_limit = (arcana_text.types.EllipsizeHeightLimit :: kind = (arcana_text.types.EllipsizeLimitKind.None :: :: call), value = 0 :: call)
    paragraph.hinting = arcana_text.types.Hinting.Disabled :: :: call
    return paragraph

export fn default_layout_config(max_width: Int, read paragraph: arcana_text.types.ParagraphStyle) -> arcana_text.types.LayoutConfig:
    let mut config = arcana_text.types.LayoutConfig :: max_width = max_width, align = paragraph.align, max_lines = paragraph.max_lines :: call
    config.ellipsis = paragraph.ellipsis
    config.wrap = paragraph.wrap
    config.tab_width = 8
    config.ellipsize_mode = paragraph.ellipsize_mode
    config.ellipsize_limit = paragraph.ellipsize_limit
    config.hinting = paragraph.hinting
    return config

export fn default_raster_config() -> arcana_text.types.RasterConfig:
    let mut config = arcana_text.types.RasterConfig :: mode = (arcana_text.types.RasterMode.Alpha :: :: call), clip_range = (arcana_text.types.TextRange :: start = 0, end = 0 :: call), draw_backgrounds = true :: call
    config.hinting = arcana_text.types.Hinting.Disabled :: :: call
    return config

export fn default_cursor() -> arcana_text.types.Cursor:
    return arcana_text.types.Cursor :: offset = 0, preferred_x = 0 :: call

export fn default_selection() -> arcana_text.types.Selection:
    return arcana_text.types.Selection :: anchor = 0, focus = 0 :: call

export fn default_selection_mode() -> arcana_text.types.SelectionMode:
    return arcana_text.types.SelectionMode.Normal :: :: call
