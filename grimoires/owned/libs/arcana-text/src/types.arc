import arcana_graphics.paint
import arcana_graphics.types
import std.collections.list

export opaque type FontCollection as move, boundary_unsafe
export opaque type ParagraphBuilder as move, boundary_unsafe
export opaque type Paragraph as move, boundary_unsafe

export record FontFeature:
    tag: Str
    value: Int

export record FontAxis:
    tag: Str
    value_milli: Int

export record Shadow:
    offset: (Int, Int)
    blur: Int
    paint: arcana_graphics.types.Paint

export enum TextAlign:
    Left
    Center
    Right
    Justify
    Start
    End

export enum TextDirection:
    LeftToRight
    RightToLeft
    Auto

export enum TextBaseline:
    Alphabetic
    Ideographic

export enum TextHeightBehavior:
    All
    DisableFirstAscent
    DisableLastDescent
    DisableAll

export enum RectHeightStyle:
    Tight
    Max

export enum RectWidthStyle:
    Tight
    Max

export enum Affinity:
    Upstream
    Downstream

export enum TextDecoration:
    Underline
    Overline
    LineThrough

export enum TextDecorationStyle:
    Solid
    Double
    Dotted
    Dashed
    Wavy

export enum PlaceholderAlignment:
    Baseline
    AboveBaseline
    BelowBaseline
    Top
    Bottom
    Middle

fn empty_decorations() -> List[arcana_text.types.TextDecoration]:
    return std.collections.list.new[arcana_text.types.TextDecoration] :: :: call

fn empty_shadows() -> List[arcana_text.types.Shadow]:
    return std.collections.list.new[arcana_text.types.Shadow] :: :: call

export record StrutStyle:
    enabled: Bool
    font_size: Int
    line_height_milli: Int
    force_height: Bool
    families: List[Str]

export record PlaceholderStyle:
    size: (Int, Int)
    alignment: arcana_text.types.PlaceholderAlignment
    baseline: arcana_text.types.TextBaseline
    baseline_offset: Int

export record TextRange:
    start: Int
    end: Int

export record PositionWithAffinity:
    index: Int
    affinity: arcana_text.types.Affinity

export record TextBox:
    position: (Int, Int)
    size: (Int, Int)
    range: arcana_text.types.TextRange
    direction: arcana_text.types.TextDirection

export record LineMetrics:
    start: Int
    end: Int
    baseline: Int
    ascent: Int
    descent: Int
    height: Int
    width: Int
    left: Int
    top: Int

export record TextStyle:
    families: List[Str]
    font_size: Int
    weight: Int
    width: Int
    slant: Int
    foreground: arcana_graphics.types.Paint
    background_enabled: Bool
    background: arcana_graphics.types.Paint
    decorations: List[arcana_text.types.TextDecoration]
    decoration_style: arcana_text.types.TextDecorationStyle
    decoration_paint: arcana_graphics.types.Paint
    shadows: List[arcana_text.types.Shadow]
    letter_spacing_milli: Int
    word_spacing_milli: Int
    line_height_milli: Int
    locale: Str
    features: List[arcana_text.types.FontFeature]
    axes: List[arcana_text.types.FontAxis]

export record ParagraphStyle:
    align: arcana_text.types.TextAlign
    direction: arcana_text.types.TextDirection
    max_lines: Int
    ellipsis: Str
    replace_tab_characters: Bool
    text_height_behavior: arcana_text.types.TextHeightBehavior
    strut: arcana_text.types.StrutStyle

fn empty_strings() -> List[Str]:
    return std.collections.list.new[Str] :: :: call

fn empty_features() -> List[arcana_text.types.FontFeature]:
    return std.collections.list.new[arcana_text.types.FontFeature] :: :: call

fn empty_axes() -> List[arcana_text.types.FontAxis]:
    return std.collections.list.new[arcana_text.types.FontAxis] :: :: call

export fn default_strut_style() -> arcana_text.types.StrutStyle:
    let mut style = arcana_text.types.StrutStyle :: enabled = false, font_size = 16, line_height_milli = 1000 :: call
    style.force_height = false
    style.families = arcana_text.types.empty_strings :: :: call
    return style

export fn default_text_style(color: Int) -> arcana_text.types.TextStyle:
    let foreground = arcana_graphics.paint.solid :: color :: call
    let background = arcana_graphics.paint.solid :: 0 :: call
    let mut families = arcana_text.types.empty_strings :: :: call
    families :: "Monaspace Neon" :: push
    let mut style = arcana_text.types.TextStyle :: families = families, font_size = 16, weight = 400 :: call
    style.width = 100
    style.slant = 0
    style.foreground = foreground
    style.background_enabled = false
    style.background = background
    style.decorations = arcana_text.types.empty_decorations :: :: call
    style.decoration_style = arcana_text.types.TextDecorationStyle.Solid :: :: call
    style.decoration_paint = foreground
    style.shadows = arcana_text.types.empty_shadows :: :: call
    style.letter_spacing_milli = 0
    style.word_spacing_milli = 0
    style.line_height_milli = 1000
    style.locale = ""
    style.features = arcana_text.types.empty_features :: :: call
    style.axes = arcana_text.types.empty_axes :: :: call
    return style

export fn default_paragraph_style() -> arcana_text.types.ParagraphStyle:
    let mut style = arcana_text.types.ParagraphStyle :: align = (arcana_text.types.TextAlign.Left :: :: call), direction = (arcana_text.types.TextDirection.LeftToRight :: :: call), max_lines = 0 :: call
    style.ellipsis = "..."
    style.replace_tab_characters = true
    style.text_height_behavior = arcana_text.types.TextHeightBehavior.All :: :: call
    style.strut = arcana_text.types.default_strut_style :: :: call
    return style
