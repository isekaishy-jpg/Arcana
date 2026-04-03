import std.bytes
import std.canvas
import std.collections.array
import std.collections.list
import std.collections.map
import std.fs
import std.option
import std.result
import std.text
import std.window
use std.option.Option
use std.result.Result

import arcana_graphics.paint
import arcana_text.provider_impl.font

record RegisteredFace:
    family_alias: Str
    source_label: Str
    source_path: Str
    source_bytes: Array[Int]
    traits: arcana_text.provider_impl.font.FaceTraits
    face: Option[arcana_text.provider_impl.font.FontFaceState]
    load_error: Str

export record FontCollectionState:
    registered_families: List[Str]
    registered_sources: List[Str]
    faces: List[arcana_text.provider_impl.engine.RegisteredFace]
    host_fallback_enabled: Bool

export record TextRunState:
    style: arcana_text.types.TextStyle
    text: Str

export enum BuilderItemState:
    Text(arcana_text.provider_impl.engine.TextRunState)
    Placeholder(arcana_text.types.PlaceholderStyle)

export record ParagraphBuilderState:
    collection: arcana_text.provider_impl.engine.FontCollectionState
    paragraph_style: arcana_text.types.ParagraphStyle
    style_stack: List[arcana_text.types.TextStyle]
    items: List[arcana_text.provider_impl.engine.BuilderItemState]

export record LayoutAtom:
    position: (Int, Int)
    size: (Int, Int)
    range: arcana_text.types.TextRange
    direction: arcana_text.types.TextDirection
    foreground: Int
    background_enabled: Bool
    background: Int
    decorations: List[arcana_text.types.TextDecoration]
    decoration_style: arcana_text.types.TextDecorationStyle
    decoration_color: Int
    shadows: List[arcana_text.types.Shadow]
    family_name: Str
    unresolved: Bool
    text: Str
    glyph_index: Int
    font_size: Int
    line_height_milli: Int
    traits: arcana_text.provider_impl.font.FaceTraits
    placeholder: Bool
    placeholder_alignment: arcana_text.types.PlaceholderAlignment
    placeholder_baseline: arcana_text.types.TextBaseline
    placeholder_baseline_offset: Int
    bitmap: arcana_text.provider_impl.font.GlyphBitmap

export record LayoutState:
    requested_width: Int
    width: Int
    height: Int
    longest_line: Int
    exceeded_max_lines: Bool
    alphabetic_baseline: Int
    ideographic_baseline: Int
    line_metrics: List[arcana_text.types.LineMetrics]
    atoms: List[arcana_text.provider_impl.engine.LayoutAtom]
    placeholder_boxes: List[arcana_text.types.TextBox]
    unresolved_glyphs: List[Int]
    fonts_used: List[Str]
    flattened_text: List[Str]

export record ParagraphState:
    collection: arcana_text.provider_impl.engine.FontCollectionState
    paragraph_style: arcana_text.types.ParagraphStyle
    items: List[arcana_text.provider_impl.engine.BuilderItemState]
    layout: Option[arcana_text.provider_impl.engine.LayoutState]

record WorkingLine:
    start: Int
    top: Int
    width: Int
    height: Int
    baseline: Int
    atoms: List[arcana_text.provider_impl.engine.LayoutAtom]

record LayoutBuildState:
    width: Int
    wrap_width: Int
    line: arcana_text.provider_impl.engine.WorkingLine
    next_top: Int
    longest_line: Int
    logical_index: Int
    line_metrics: List[arcana_text.types.LineMetrics]
    atoms: List[arcana_text.provider_impl.engine.LayoutAtom]
    placeholder_boxes: List[arcana_text.types.TextBox]
    unresolved: List[Int]
    fonts_used: List[Str]
    flattened: List[Str]

record TextAppendRequest:
    run: arcana_text.provider_impl.engine.TextRunState
    text: Str
    logical_index: Int

record TextToken:
    text: Str
    whitespace: Bool
    newline: Bool

record PlaceholderAppendRequest:
    placeholder: arcana_text.types.PlaceholderStyle
    logical_index: Int

record BitmapRenderRequest:
    text: Str
    glyph_index: Int
    font_size: Int
    line_height_milli: Int
    traits: arcana_text.provider_impl.font.FaceTraits

record EllipsisAppendRequest:
    source: arcana_text.provider_impl.engine.LayoutAtom
    x: Int
    line_top: Int
    baseline: Int
    index: Int
    text: Str

record DecorationSpanSpec:
    x0: Int
    x1: Int
    y: Int
    thickness: Int
    color: Int
    style: arcana_text.types.TextDecorationStyle

record EllipsisPartitionState:
    kept: List[arcana_text.provider_impl.engine.LayoutAtom]
    line: List[arcana_text.provider_impl.engine.LayoutAtom]
    remaining_width: Int
    line_top: Int
    line_left: Int

record PixelWriteSpec:
    width: Int
    pos: (Int, Int)
    color: Int

record PixelRectSpec:
    dims: (Int, Int)
    rect: ((Int, Int), (Int, Int))
    color: Int

record FaceRenderResult:
    faces: List[arcana_text.provider_impl.engine.RegisteredFace]
    family: Str
    bitmap: arcana_text.provider_impl.font.GlyphBitmap

record LineMetricSpec:
    start: Int
    end: Int
    baseline: Int
    ascent: Int
    descent: Int
    height: Int
    width: Int
    left: Int
    top: Int

fn empty_strings() -> List[Str]:
    return std.collections.list.new[Str] :: :: call

fn empty_faces() -> List[arcana_text.provider_impl.engine.RegisteredFace]:
    return std.collections.list.new[arcana_text.provider_impl.engine.RegisteredFace] :: :: call

fn empty_items() -> List[arcana_text.provider_impl.engine.BuilderItemState]:
    return std.collections.list.new[arcana_text.provider_impl.engine.BuilderItemState] :: :: call

fn empty_styles() -> List[arcana_text.types.TextStyle]:
    return std.collections.list.new[arcana_text.types.TextStyle] :: :: call

fn empty_atoms() -> List[arcana_text.provider_impl.engine.LayoutAtom]:
    return std.collections.list.new[arcana_text.provider_impl.engine.LayoutAtom] :: :: call

fn empty_boxes() -> List[arcana_text.types.TextBox]:
    return std.collections.list.new[arcana_text.types.TextBox] :: :: call

fn empty_lines() -> List[arcana_text.types.LineMetrics]:
    return std.collections.list.new[arcana_text.types.LineMetrics] :: :: call

fn empty_ints() -> List[Int]:
    return std.collections.list.new[Int] :: :: call

fn empty_decorations() -> List[arcana_text.types.TextDecoration]:
    return std.collections.list.new[arcana_text.types.TextDecoration] :: :: call

fn empty_shadows() -> List[arcana_text.types.Shadow]:
    return std.collections.list.new[arcana_text.types.Shadow] :: :: call

fn max_int(a: Int, b: Int) -> Int:
    if a >= b:
        return a
    return b

fn min_int(a: Int, b: Int) -> Int:
    if a <= b:
        return a
    return b

fn clamp_int(value: Int, low: Int, high: Int) -> Int:
    let mut out = value
    if out < low:
        out = low
    if out > high:
        out = high
    return out

fn abs_int(value: Int) -> Int:
    if value < 0:
        return 0 - value
    return value

fn positive_mod(value: Int, base: Int) -> Int:
    let mut out = value % base
    if out < 0:
        out += base
    return out

fn default_provider_text_style() -> arcana_text.types.TextStyle:
    return arcana_text.types.default_text_style :: 16777215 :: call

fn trace(text: Str):
    let _ = std.fs.write_text :: "engine_trace.txt", text :: call

export fn new_collection_state() -> arcana_text.provider_impl.engine.FontCollectionState:
    let mut out = arcana_text.provider_impl.engine.FontCollectionState :: registered_families = (empty_strings :: :: call), registered_sources = (empty_strings :: :: call), faces = (empty_faces :: :: call) :: call
    out.host_fallback_enabled = false
    return out

export fn new_builder_state(read collection: arcana_text.provider_impl.engine.FontCollectionState, read paragraph_style: arcana_text.types.ParagraphStyle) -> arcana_text.provider_impl.engine.ParagraphBuilderState:
    let mut styles = empty_styles :: :: call
    styles :: (default_provider_text_style :: :: call) :: push
    let mut out = arcana_text.provider_impl.engine.ParagraphBuilderState :: collection = collection, paragraph_style = paragraph_style, style_stack = styles :: call
    out.items = empty_items :: :: call
    return out

export fn build_paragraph_state(read builder: arcana_text.provider_impl.engine.ParagraphBuilderState) -> arcana_text.provider_impl.engine.ParagraphState:
    let mut out = arcana_text.provider_impl.engine.ParagraphState :: collection = builder.collection, paragraph_style = builder.paragraph_style, items = builder.items :: call
    out.layout = Option.None[arcana_text.provider_impl.engine.LayoutState] :: :: call
    return out

fn working_line(start: Int, top: Int) -> arcana_text.provider_impl.engine.WorkingLine:
    let mut out = arcana_text.provider_impl.engine.WorkingLine :: start = start, top = top, width = 0 :: call
    out.height = 0
    out.baseline = 0
    out.atoms = empty_atoms :: :: call
    return out

fn layout_build_state(width: Int, wrap_width: Int) -> arcana_text.provider_impl.engine.LayoutBuildState:
    let mut out = arcana_text.provider_impl.engine.LayoutBuildState :: width = width, wrap_width = wrap_width, line = (working_line :: 0, 0 :: call) :: call
    out.next_top = 0
    out.longest_line = 0
    out.logical_index = 0
    out.line_metrics = empty_lines :: :: call
    out.atoms = empty_atoms :: :: call
    out.placeholder_boxes = empty_boxes :: :: call
    out.unresolved = empty_ints :: :: call
    out.fonts_used = empty_strings :: :: call
    out.flattened = empty_strings :: :: call
    return out

fn text_box_value(pos: (Int, Int), size: (Int, Int), read detail: (arcana_text.types.TextRange, arcana_text.types.TextDirection)) -> arcana_text.types.TextBox:
    let mut out = arcana_text.types.TextBox :: position = pos, size = size, range = detail.0 :: call
    out.direction = detail.1
    return out

fn line_metric_spec(start: Int, end: Int, baseline: Int) -> arcana_text.provider_impl.engine.LineMetricSpec:
    let mut out = arcana_text.provider_impl.engine.LineMetricSpec :: start = start, end = end, baseline = baseline :: call
    out.ascent = 0
    out.descent = 0
    out.height = 0
    out.width = 0
    out.left = 0
    out.top = 0
    return out

fn bitmap_render_request(text: Str, font_size: Int, line_height_milli: Int) -> arcana_text.provider_impl.engine.BitmapRenderRequest:
    let mut out = arcana_text.provider_impl.engine.BitmapRenderRequest :: text = text, glyph_index = -1, font_size = font_size :: call
    out.line_height_milli = line_height_milli
    out.traits = arcana_text.provider_impl.font.default_traits :: :: call
    return out

fn ellipsis_append_request(read source: arcana_text.provider_impl.engine.LayoutAtom, x: Int, line_top: Int) -> arcana_text.provider_impl.engine.EllipsisAppendRequest:
    let mut out = arcana_text.provider_impl.engine.EllipsisAppendRequest :: source = source, x = x, line_top = line_top :: call
    out.baseline = 0
    out.index = 0
    out.text = ""
    return out

fn decoration_span_spec(x0: Int, x1: Int, y: Int) -> arcana_text.provider_impl.engine.DecorationSpanSpec:
    let mut out = arcana_text.provider_impl.engine.DecorationSpanSpec :: x0 = x0, x1 = x1, y = y :: call
    out.thickness = 1
    out.color = 0
    out.style = arcana_text.types.TextDecorationStyle.Solid :: :: call
    return out

fn ellipsis_partition_state(line_top: Int, line_left: Int) -> arcana_text.provider_impl.engine.EllipsisPartitionState:
    let mut out = arcana_text.provider_impl.engine.EllipsisPartitionState :: kept = (empty_atoms :: :: call), line = (empty_atoms :: :: call), remaining_width = 0 :: call
    out.line_top = line_top
    out.line_left = line_left
    return out

fn empty_line_metrics_value() -> arcana_text.types.LineMetrics:
    let mut out = arcana_text.types.LineMetrics :: start = 0, end = 0, baseline = 0 :: call
    out.ascent = 0
    out.descent = 0
    out.height = 0
    out.width = 0
    out.left = 0
    out.top = 0
    return out

fn line_metric_value(read spec: arcana_text.provider_impl.engine.LineMetricSpec) -> arcana_text.types.LineMetrics:
    let mut out = arcana_text.types.LineMetrics :: start = spec.start, end = spec.end, baseline = spec.baseline :: call
    out.ascent = spec.ascent
    out.descent = spec.descent
    out.height = spec.height
    out.width = spec.width
    out.left = spec.left
    out.top = spec.top
    return out

fn text_range_value(start: Int, end: Int) -> arcana_text.types.TextRange:
    return arcana_text.types.TextRange :: start = start, end = end :: call

fn position_with_affinity(index: Int, read affinity: arcana_text.types.Affinity) -> arcana_text.types.PositionWithAffinity:
    return arcana_text.types.PositionWithAffinity :: index = index, affinity = affinity :: call

fn text_item(read style: arcana_text.types.TextStyle, read text: Str) -> arcana_text.provider_impl.engine.BuilderItemState:
    let run = arcana_text.provider_impl.engine.TextRunState :: style = style, text = text :: call
    return arcana_text.provider_impl.engine.BuilderItemState.Text :: run :: call

fn placeholder_item(read value: arcana_text.types.PlaceholderStyle) -> arcana_text.provider_impl.engine.BuilderItemState:
    return arcana_text.provider_impl.engine.BuilderItemState.Placeholder :: value :: call

fn atom_left(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> Int:
    return atom.position.0

fn atom_top(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> Int:
    return atom.position.1

fn atom_width(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> Int:
    return atom.size.0

fn atom_range_start(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> Int:
    return atom.range.start

fn clone_layout_atom(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> arcana_text.provider_impl.engine.LayoutAtom:
    let mut out = layout_atom_value :: atom.position, atom.size, atom.range :: call
    out.direction = atom.direction
    out.foreground = atom.foreground
    out.background_enabled = atom.background_enabled
    out.background = atom.background
    out.decorations = atom.decorations
    out.decoration_style = atom.decoration_style
    out.decoration_color = atom.decoration_color
    out.shadows = atom.shadows
    out.family_name = atom.family_name
    out.unresolved = atom.unresolved
    out.text = atom.text
    out.glyph_index = atom.glyph_index
    out.font_size = atom.font_size
    out.line_height_milli = atom.line_height_milli
    out.traits = atom.traits
    out.placeholder = atom.placeholder
    out.placeholder_alignment = atom.placeholder_alignment
    out.placeholder_baseline = atom.placeholder_baseline
    out.placeholder_baseline_offset = atom.placeholder_baseline_offset
    out.bitmap = atom.bitmap
    return out

fn partition_last_line_atom(edit partition: arcana_text.provider_impl.engine.EllipsisPartitionState, read atom: arcana_text.provider_impl.engine.LayoutAtom):
    let top = atom_top :: atom :: call
    if top == partition.line_top:
        let x = atom_left :: atom :: call
        let width = atom_width :: atom :: call
        partition.remaining_width = x + width - partition.line_left
        partition.line :: (clone_layout_atom :: atom :: call) :: push
        return
    partition.kept :: (clone_layout_atom :: atom :: call) :: push

fn append_kept_atom_if_before_end(edit kept: List[arcana_text.provider_impl.engine.LayoutAtom], read atom: arcana_text.provider_impl.engine.LayoutAtom, keep_end: Int):
    if (atom_range_start :: atom :: call) < keep_end:
        kept :: (clone_layout_atom :: atom :: call) :: push

fn color_red(color: Int) -> Int:
    return (color / 65536) % 256

fn color_green(color: Int) -> Int:
    return (color / 256) % 256

fn color_blue(color: Int) -> Int:
    return color % 256

fn utf8_char_len(first: Int) -> Int:
    if first < 128:
        return 1
    if first < 224:
        return 2
    if first < 240:
        return 3
    if first < 248:
        return 4
    return 1

export fn utf8_chars(read text: Str) -> List[Str]:
    let bytes = std.bytes.from_str_utf8 :: text :: call
    let total = std.bytes.len :: bytes :: call
    let mut out = empty_strings :: :: call
    let mut index = 0
    while index < total:
        let first = std.bytes.at :: bytes, index :: call
        let mut count = utf8_char_len :: first :: call
        if index + count > total:
            count = 1
        let slice = std.bytes.slice :: bytes, index, index + count :: call
        out :: (std.bytes.to_str_utf8 :: slice :: call) :: push
        index += count
    return out

fn first_string_or(read values: List[Str], read fallback: Str) -> Str:
    for value in values:
        return value
    return fallback

fn first_byte(read text: Str) -> Int:
    let bytes = std.bytes.from_str_utf8 :: text :: call
    if (std.bytes.len :: bytes :: call) == 0:
        return 0
    return std.bytes.at :: bytes, 0 :: call

fn is_tab(read text: Str) -> Bool:
    return text == "\t"

fn is_newline(read text: Str) -> Bool:
    return text == "\n"

fn is_space(read text: Str) -> Bool:
    return text == " " or text == "\t" or text == "\n" or text == "\r"

fn glyph_advance(read style: arcana_text.types.TextStyle, read text: Str) -> Int:
    let mut advance = max_int :: style.font_size, 1 :: call
    if text == " ":
        advance += style.word_spacing_milli / 1000
    else:
        advance += style.letter_spacing_milli / 1000
    return advance

fn is_ascii_alpha_num(read text: Str) -> Bool:
    if (std.text.len_bytes :: text :: call) != 1:
        return false
    let b = std.text.byte_at :: text, 0 :: call
    return (std.text.is_alpha_byte :: b :: call) or (std.text.is_digit_byte :: b :: call)

fn preferred_collection_name(read collection: arcana_text.provider_impl.engine.FontCollectionState) -> Str:
    return first_string_or :: collection.registered_families, "Monaspace Neon" :: call

fn preferred_collection_name_for_paragraph(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Str:
    return preferred_collection_name :: paragraph.collection :: call

fn preferred_family_name_for_paragraph(read style: arcana_text.types.TextStyle, read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Str:
    return first_string_or :: style.families, (preferred_collection_name_for_paragraph :: paragraph :: call) :: call

fn push_unique_string(edit out: List[Str], read value: Str):
    for existing in out:
        if existing == value:
            return
    out :: value :: push

fn face_traits_score(read actual: arcana_text.provider_impl.font.FaceTraits, read target: arcana_text.provider_impl.font.FaceTraits) -> Int:
    return (abs_int :: (actual.weight - target.weight) :: call) + ((abs_int :: (actual.width_milli - target.width_milli) :: call) / 1000) + ((abs_int :: (actual.slant_milli - target.slant_milli) :: call) / 1000)

fn axis_tag_matches(read axis: arcana_text.types.FontAxis, read tag: Str) -> Bool:
    return axis.tag == tag

fn resolved_style_traits(read style: arcana_text.types.TextStyle) -> arcana_text.provider_impl.font.FaceTraits:
    let mut traits = arcana_text.provider_impl.font.default_traits :: :: call
    traits.weight = style.weight
    traits.width_milli = style.width * 1000
    traits.slant_milli = style.slant * 1000
    for axis in style.axes:
        if axis_tag_matches :: axis, "wght" :: call:
            traits.weight = axis.value_milli / 1000
            continue
        if axis_tag_matches :: axis, "wdth" :: call:
            traits.width_milli = axis.value_milli
            continue
        if axis_tag_matches :: axis, "slnt" :: call:
            traits.slant_milli = axis.value_milli
    return traits

fn face_score(read traits: arcana_text.provider_impl.font.FaceTraits, read style: arcana_text.types.TextStyle) -> Int:
    return face_traits_score :: traits, (resolved_style_traits :: style :: call) :: call

fn face_request_from_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace) -> arcana_text.provider_impl.font.FaceLoadRequest:
    let mut request = arcana_text.provider_impl.font.face_load_request :: entry.family_alias, entry.source_label, entry.source_path :: call
    request.font_bytes = entry.source_bytes
    request.traits = entry.traits
    return request

fn same_face_entry(read left: arcana_text.provider_impl.engine.RegisteredFace, read right: arcana_text.provider_impl.engine.RegisteredFace) -> Bool:
    return left.family_alias == right.family_alias and left.source_label == right.source_label and left.source_path == right.source_path and left.traits.weight == right.traits.weight and left.traits.width_milli == right.traits.width_milli and left.traits.slant_milli == right.traits.slant_milli

fn copied_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace) -> arcana_text.provider_impl.engine.RegisteredFace:
    return entry

fn empty_registered_face() -> arcana_text.provider_impl.engine.RegisteredFace:
    let mut entry = arcana_text.provider_impl.engine.RegisteredFace :: family_alias = "", source_label = "", source_path = "" :: call
    entry.source_bytes = std.collections.array.from_list[Int] :: (empty_ints :: :: call) :: call
    entry.traits = arcana_text.provider_impl.font.default_traits :: :: call
    entry.face = Option.None[arcana_text.provider_impl.font.FontFaceState] :: :: call
    entry.load_error = ""
    return entry

fn load_face_from_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace) -> Result[arcana_text.provider_impl.font.FontFaceState, Str]:
    let request = face_request_from_entry :: entry :: call
    if (std.bytes.len :: request.font_bytes :: call) > 0:
        return arcana_text.provider_impl.font.load_face_from_bytes :: request :: call
    return arcana_text.provider_impl.font.load_face_from_path :: request.family_name, request.source_path, request.traits :: call

fn default_bitmap(font_size: Int) -> arcana_text.provider_impl.font.GlyphBitmap:
    let mut bitmap = glyph_bitmap_value :: (0, 0), (0, 0), (max_int :: font_size, 1 :: call) :: call
    bitmap.baseline = max_int :: font_size, 1 :: call
    bitmap.line_height = max_int :: font_size, 1 :: call
    return bitmap

fn loaded_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace, take face: arcana_text.provider_impl.font.FontFaceState) -> arcana_text.provider_impl.engine.RegisteredFace:
    let mut next = entry
    next.face = Option.Some[arcana_text.provider_impl.font.FontFaceState] :: face :: call
    next.load_error = ""
    return next

fn failed_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace, err: Str) -> arcana_text.provider_impl.engine.RegisteredFace:
    let mut next = entry
    next.load_error = err
    return next

fn ensure_loaded_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace) -> arcana_text.provider_impl.engine.RegisteredFace:
    if (entry.face :: :: is_some) or entry.load_error != "":
        return entry
    return match (load_face_from_entry :: entry :: call):
        Result.Ok(face) => loaded_face_entry :: entry, face :: call
        Result.Err(err) => failed_face_entry :: entry, err :: call

fn face_entry_supports_text(read entry: arcana_text.provider_impl.engine.RegisteredFace, read text: Str) -> Bool:
    return match entry.face:
        Option.Some(face) => arcana_text.provider_impl.font.supports_text :: face, text :: call
        Option.None => false

fn rendered_loaded_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace, take face: arcana_text.provider_impl.font.FontFaceState, read spec: arcana_text.provider_impl.engine.BitmapRenderRequest) -> (arcana_text.provider_impl.engine.RegisteredFace, arcana_text.provider_impl.font.GlyphBitmap):
    let mut next_face = face
    let mut render_spec = arcana_text.provider_impl.font.glyph_render_spec :: spec.text, spec.font_size, spec.line_height_milli :: call
    render_spec.glyph_index = spec.glyph_index
    render_spec.traits = spec.traits
    let bitmap = arcana_text.provider_impl.font.render_glyph :: next_face, render_spec :: call
    return ((loaded_face_entry :: entry, next_face :: call), bitmap)

fn rendered_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace, read spec: arcana_text.provider_impl.engine.BitmapRenderRequest) -> (arcana_text.provider_impl.engine.RegisteredFace, arcana_text.provider_impl.font.GlyphBitmap):
    return match entry.face:
        Option.Some(face) => rendered_loaded_face_entry :: entry, face, spec :: call
        Option.None => (entry, (default_bitmap :: spec.font_size :: call))

fn measured_loaded_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace, read face: arcana_text.provider_impl.font.FontFaceState, read spec: arcana_text.provider_impl.engine.BitmapRenderRequest) -> (arcana_text.provider_impl.engine.RegisteredFace, arcana_text.provider_impl.font.GlyphBitmap):
    let mut render_spec = arcana_text.provider_impl.font.glyph_render_spec :: spec.text, spec.font_size, spec.line_height_milli :: call
    render_spec.glyph_index = spec.glyph_index
    render_spec.traits = spec.traits
    let bitmap = arcana_text.provider_impl.font.measure_glyph :: face, render_spec :: call
    return (entry, bitmap)

fn measured_face_entry(read entry: arcana_text.provider_impl.engine.RegisteredFace, read spec: arcana_text.provider_impl.engine.BitmapRenderRequest) -> (arcana_text.provider_impl.engine.RegisteredFace, arcana_text.provider_impl.font.GlyphBitmap):
    return match entry.face:
        Option.Some(face) => measured_loaded_face_entry :: entry, face, spec :: call
        Option.None => (entry, (default_bitmap :: spec.font_size :: call))

fn face_selection_pass(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read text_spec: (arcana_text.types.TextStyle, Str), read family_spec: (Str, Bool)) -> (List[arcana_text.provider_impl.engine.RegisteredFace], Option[arcana_text.provider_impl.engine.RegisteredFace]):
    let mut next_faces = empty_faces :: :: call
    let mut best_score = 2147483647
    let mut selected = Option.None[arcana_text.provider_impl.engine.RegisteredFace] :: :: call
    let style = text_spec.0
    let text = text_spec.1
    let target_family = family_spec.0
    let restrict_family = family_spec.1
    for entry in faces:
        let loaded = ensure_loaded_face_entry :: entry :: call
        next_faces :: (copied_face_entry :: loaded :: call) :: push
        if restrict_family and loaded.family_alias != target_family:
            continue
        if not (face_entry_supports_text :: loaded, text :: call):
            continue
        let score = face_score :: loaded.traits, style :: call
        if (selected :: :: is_none) or score < best_score:
            best_score = score
            selected = Option.Some[arcana_text.provider_impl.engine.RegisteredFace] :: (copied_face_entry :: loaded :: call) :: call
    return (next_faces, selected)

fn selected_face_entry(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read text_spec: (arcana_text.types.TextStyle, Str), read target_family: Str) -> (List[arcana_text.provider_impl.engine.RegisteredFace], Option[arcana_text.provider_impl.engine.RegisteredFace]):
    let primary = face_selection_pass :: faces, text_spec, (target_family, true) :: call
    if primary.1 :: :: is_some:
        return primary
    return face_selection_pass :: primary.0, text_spec, (target_family, false) :: call

fn read_face_selection_pass(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read target: (arcana_text.provider_impl.font.FaceTraits, Str), read family_spec: (Str, Bool)) -> Option[arcana_text.provider_impl.engine.RegisteredFace]:
    let mut best_score = 2147483647
    let mut selected = Option.None[arcana_text.provider_impl.engine.RegisteredFace] :: :: call
    let target_traits = target.0
    let text = target.1
    let target_family = family_spec.0
    let restrict_family = family_spec.1
    for entry in faces:
        if restrict_family and entry.family_alias != target_family:
            continue
        if not (face_entry_supports_text :: entry, text :: call):
            continue
        let score = face_traits_score :: entry.traits, target_traits :: call
        if (selected :: :: is_none) or score < best_score:
            best_score = score
            selected = Option.Some[arcana_text.provider_impl.engine.RegisteredFace] :: entry :: call
    return selected

fn read_selected_face_entry(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read target: (arcana_text.provider_impl.font.FaceTraits, Str), read target_family: Str) -> Option[arcana_text.provider_impl.engine.RegisteredFace]:
    let primary = read_face_selection_pass :: faces, target, (target_family, true) :: call
    if primary :: :: is_some:
        return primary
    return read_face_selection_pass :: faces, target, (target_family, false) :: call

fn replace_face_entry(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read target: arcana_text.provider_impl.engine.RegisteredFace, read replacement: arcana_text.provider_impl.engine.RegisteredFace) -> List[arcana_text.provider_impl.engine.RegisteredFace]:
    let mut next_faces = empty_faces :: :: call
    let mut replaced = false
    for entry in faces:
        if not replaced and (same_face_entry :: entry, target :: call):
            next_faces :: replacement :: push
            replaced = true
            continue
        next_faces :: entry :: push
    return next_faces

fn unresolved_selection_value(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read fallback: (Str, Int)) -> arcana_text.provider_impl.engine.FaceRenderResult:
    let default_advance = fallback.1
    let mut bitmap = glyph_bitmap_value :: (0, 0), (0, 0), default_advance :: call
    bitmap.baseline = default_advance
    bitmap.line_height = default_advance
    return arcana_text.provider_impl.engine.FaceRenderResult :: faces = faces, family = fallback.0, bitmap = bitmap :: call

fn measured_selection_value(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read entry: arcana_text.provider_impl.engine.RegisteredFace, read spec: arcana_text.provider_impl.engine.BitmapRenderRequest) -> arcana_text.provider_impl.engine.FaceRenderResult:
    let measured = measured_face_entry :: entry, spec :: call
    let next_entry = measured.0
    return arcana_text.provider_impl.engine.FaceRenderResult :: faces = (replace_face_entry :: faces, entry, next_entry :: call), family = next_entry.family_alias, bitmap = measured.1 :: call

fn rendered_selection_value(read faces: List[arcana_text.provider_impl.engine.RegisteredFace], read entry: arcana_text.provider_impl.engine.RegisteredFace, read spec: arcana_text.provider_impl.engine.BitmapRenderRequest) -> arcana_text.provider_impl.engine.FaceRenderResult:
    let rendered = rendered_face_entry :: entry, spec :: call
    let next_entry = rendered.0
    return arcana_text.provider_impl.engine.FaceRenderResult :: faces = (replace_face_entry :: faces, entry, next_entry :: call), family = next_entry.family_alias, bitmap = rendered.1 :: call

fn glyph_bitmap_value(size: (Int, Int), offset: (Int, Int), advance: Int) -> arcana_text.provider_impl.font.GlyphBitmap:
    let mut out = arcana_text.provider_impl.font.GlyphBitmap :: size = size, offset = offset, advance = advance :: call
    out.baseline = 0
    out.line_height = 0
    out.empty = true
    out.alpha = std.collections.array.from_list[Int] :: (empty_ints :: :: call) :: call
    return out

fn layout_atom_value(position: (Int, Int), size: (Int, Int), read range: arcana_text.types.TextRange) -> arcana_text.provider_impl.engine.LayoutAtom:
    let mut out = arcana_text.provider_impl.engine.LayoutAtom :: position = position, size = size, range = range :: call
    out.direction = arcana_text.types.TextDirection.LeftToRight :: :: call
    out.foreground = 16777215
    out.background_enabled = false
    out.background = 0
    out.decorations = empty_decorations :: :: call
    out.decoration_style = arcana_text.types.TextDecorationStyle.Solid :: :: call
    out.decoration_color = 16777215
    out.shadows = empty_shadows :: :: call
    out.family_name = ""
    out.unresolved = false
    out.text = ""
    out.glyph_index = 0
    out.font_size = 16
    out.line_height_milli = 1000
    out.traits = arcana_text.provider_impl.font.default_traits :: :: call
    out.placeholder = false
    out.placeholder_alignment = arcana_text.types.PlaceholderAlignment.Baseline :: :: call
    out.placeholder_baseline = arcana_text.types.TextBaseline.Alphabetic :: :: call
    out.placeholder_baseline_offset = 0
    out.bitmap = glyph_bitmap_value :: (0, 0), (0, 0), 0 :: call
    return out

fn strong_direction(read ch: Str) -> Option[arcana_text.types.TextDirection]:
    if is_ascii_alpha_num :: ch :: call:
        return Option.Some[arcana_text.types.TextDirection] :: (arcana_text.types.TextDirection.LeftToRight :: :: call) :: call
    let b = first_byte :: ch :: call
    if b >= 216:
        return Option.Some[arcana_text.types.TextDirection] :: (arcana_text.types.TextDirection.RightToLeft :: :: call) :: call
    return Option.None[arcana_text.types.TextDirection] :: :: call

fn resolved_direction(read paragraph_style: arcana_text.types.ParagraphStyle, read atoms: List[arcana_text.provider_impl.engine.LayoutAtom]) -> arcana_text.types.TextDirection:
    if paragraph_style.direction != (arcana_text.types.TextDirection.Auto :: :: call):
        return paragraph_style.direction
    for atom in atoms:
        if atom.placeholder:
            continue
        let direction = match (strong_direction :: atom.text :: call):
            Option.Some(found) => found
            Option.None => arcana_text.types.TextDirection.Auto :: :: call
        if direction != (arcana_text.types.TextDirection.Auto :: :: call):
            return direction
    return arcana_text.types.TextDirection.LeftToRight :: :: call

fn alignment_offset(read align: arcana_text.types.TextAlign, read direction: arcana_text.types.TextDirection, dims: (Int, Int)) -> Int:
    if dims.0 <= 0:
        return 0
    let slack = max_int :: (dims.0 - dims.1), 0 :: call
    return match align:
        arcana_text.types.TextAlign.Center => slack / 2
        arcana_text.types.TextAlign.Right => slack
        arcana_text.types.TextAlign.Start => match direction:
            arcana_text.types.TextDirection.RightToLeft => slack
            _ => 0
        arcana_text.types.TextAlign.End => match direction:
            arcana_text.types.TextDirection.RightToLeft => 0
            _ => slack
        _ => 0

fn line_family_name(read line: arcana_text.provider_impl.engine.WorkingLine) -> Str:
    let mut family = ""
    for atom in line.atoms:
        family = atom.family_name
    return family

fn append_text_char(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, edit line: arcana_text.provider_impl.engine.WorkingLine, read spec: arcana_text.provider_impl.engine.TextAppendRequest) -> Bool:
    trace :: ("append:" + spec.text) :: call
    let run = spec.run
    let text = spec.text
    let logical_index = spec.logical_index
    let style = run.style
    let traits = resolved_style_traits :: style :: call
    let target_family = first_string_or :: style.families, "Monaspace Neon" :: call
    let selection = selected_face_entry :: paragraph.collection.faces, (style, text), target_family :: call
    trace :: ("append:selected:" + spec.text) :: call
    paragraph.collection.faces = selection.0
    let unresolved = selection.1 :: :: is_none
    let default_advance = max_int :: style.font_size, 1 :: call
    let mut render_spec = bitmap_render_request :: text, style.font_size, style.line_height_milli :: call
    render_spec.traits = traits
    let rendered = match selection.1:
        Option.Some(entry) => measured_selection_value :: paragraph.collection.faces, entry, render_spec :: call
        Option.None => unresolved_selection_value :: paragraph.collection.faces, (target_family, default_advance) :: call
    trace :: ("append:measured:" + spec.text) :: call
    paragraph.collection.faces = rendered.faces
    let family = rendered.family
    let bitmap = rendered.bitmap
    let mut width = bitmap.advance + (style.letter_spacing_milli / 1000)
    if text == " ":
        width = bitmap.advance + (style.word_spacing_milli / 1000)
    let height = bitmap.line_height
    let baseline = bitmap.baseline
    if height > line.height:
        line.height = height
    if baseline > line.baseline:
        line.baseline = baseline
    let atom_range = text_range_value :: logical_index, logical_index + 1 :: call
    let mut atom = layout_atom_value :: (line.width, line.top), (width, height), atom_range :: call
    atom.direction = paragraph.paragraph_style.direction
    atom.foreground = style.foreground.color
    atom.background_enabled = style.background_enabled
    atom.background = style.background.color
    atom.decorations = style.decorations
    atom.decoration_style = style.decoration_style
    atom.decoration_color = style.decoration_paint.color
    atom.shadows = style.shadows
    atom.family_name = family
    atom.unresolved = unresolved
    atom.text = text
    atom.glyph_index = render_spec.glyph_index
    atom.font_size = style.font_size
    atom.line_height_milli = style.line_height_milli
    atom.traits = traits
    atom.placeholder = false
    atom.bitmap = bitmap
    line.width += width
    line.atoms :: atom :: push
    return unresolved

fn append_placeholder_atom(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, edit line: arcana_text.provider_impl.engine.WorkingLine, read spec: arcana_text.provider_impl.engine.PlaceholderAppendRequest):
    let placeholder = spec.placeholder
    let logical_index = spec.logical_index
    let width = max_int :: placeholder.size.0, 1 :: call
    let height = max_int :: placeholder.size.1, 16 :: call
    let baseline = (height * 800) / 1000
    if height > line.height:
        line.height = height
    if baseline > line.baseline:
        line.baseline = baseline
    let atom_range = text_range_value :: logical_index, logical_index + 1 :: call
    let mut atom = layout_atom_value :: (line.width, line.top), (width, height), atom_range :: call
    atom.direction = paragraph.paragraph_style.direction
    atom.foreground = 16777215
    atom.background_enabled = true
    atom.background = 4214880
    atom.decorations = empty_decorations :: :: call
    atom.decoration_style = arcana_text.types.TextDecorationStyle.Solid :: :: call
    atom.decoration_color = atom.foreground
    atom.shadows = empty_shadows :: :: call
    atom.family_name = "Monaspace Neon"
    atom.unresolved = false
    atom.text = "\u{FFFC}"
    atom.glyph_index = -1
    atom.font_size = 16
    atom.line_height_milli = 1000
    atom.traits = arcana_text.provider_impl.font.default_traits :: :: call
    atom.placeholder = true
    atom.placeholder_alignment = placeholder.alignment
    atom.placeholder_baseline = placeholder.baseline
    atom.placeholder_baseline_offset = placeholder.baseline_offset
    let mut placeholder_bitmap = glyph_bitmap_value :: (width, height), (0, 0), width :: call
    placeholder_bitmap.baseline = baseline
    placeholder_bitmap.line_height = height
    atom.bitmap = placeholder_bitmap
    line.width += width
    line.atoms :: atom :: push

fn layout_needs_wrap(read state: arcana_text.provider_impl.engine.LayoutBuildState, advance: Int) -> Bool:
    return state.width > 0 and not (state.line.atoms :: :: is_empty) and state.line.width + advance > state.wrap_width

fn strut_height(read paragraph_style: arcana_text.types.ParagraphStyle) -> Int:
    if not paragraph_style.strut.enabled:
        return 0
    let base = max_int :: paragraph_style.strut.font_size, 1 :: call
    let scaled = (base * (max_int :: paragraph_style.strut.line_height_milli, 1000 :: call)) / 1000
    return max_int :: scaled, base :: call

fn strut_baseline(read paragraph_style: arcana_text.types.ParagraphStyle, height: Int) -> Int:
    if not paragraph_style.strut.enabled:
        return 0
    return clamp_int :: ((height * 800) / 1000), 0, height :: call

fn placeholder_top(read atom: arcana_text.provider_impl.engine.LayoutAtom, read placement: ((Int, Int), Int)) -> Int:
    let line_top = placement.0.0
    let line_height = placement.0.1
    let baseline = placement.1
    return match atom.placeholder_alignment:
        arcana_text.types.PlaceholderAlignment.Top => line_top
        arcana_text.types.PlaceholderAlignment.Bottom => line_top + line_height - atom.size.1
        arcana_text.types.PlaceholderAlignment.Middle => line_top + ((line_height - atom.size.1) / 2)
        arcana_text.types.PlaceholderAlignment.BelowBaseline => line_top + baseline + atom.placeholder_baseline_offset
        arcana_text.types.PlaceholderAlignment.AboveBaseline => line_top + baseline - atom.size.1 + atom.placeholder_baseline_offset
        _ => placeholder_top_from_baseline :: atom, line_top, baseline :: call

fn placeholder_top_from_baseline(read atom: arcana_text.provider_impl.engine.LayoutAtom, line_top: Int, baseline: Int) -> Int:
    if atom.placeholder_baseline_offset > 0:
        return line_top + baseline - atom.placeholder_baseline_offset
    return line_top + baseline - atom.size.1

fn aligned_atom_position(read atom: arcana_text.provider_impl.engine.LayoutAtom, x: Int, read placement: ((Int, Int), Int)) -> (Int, Int):
    let line_top = placement.0.0
    let line_height = placement.0.1
    let baseline = placement.1
    if atom.placeholder:
        return (x, (placeholder_top :: atom, placement :: call))
    return (x, line_top + baseline - atom.bitmap.baseline)

fn finalize_line(read paragraph_style: arcana_text.types.ParagraphStyle, edit state: arcana_text.provider_impl.engine.LayoutBuildState):
    if state.line.atoms :: :: is_empty:
        state.line.top = state.next_top
        state.line.start = state.logical_index
        return
    let direction = resolved_direction :: paragraph_style, state.line.atoms :: call
    let offset = alignment_offset :: paragraph_style.align, direction, (state.width, state.line.width) :: call
    let natural_height = max_int :: state.line.height, 16 :: call
    let strut_min = strut_height :: paragraph_style :: call
    let line_height = max_int :: natural_height, strut_min :: call
    let natural_baseline = max_int :: state.line.baseline, ((natural_height * 800) / 1000) :: call
    let baseline = max_int :: natural_baseline, (strut_baseline :: paragraph_style, line_height :: call) :: call
    for item in state.line.atoms:
        let mut atom = item
        atom.direction = direction
        if direction == (arcana_text.types.TextDirection.RightToLeft :: :: call):
            atom.position = aligned_atom_position :: atom, (offset + (state.line.width - item.position.0 - item.size.0)), ((state.line.top, line_height), baseline) :: call
        else:
            atom.position = aligned_atom_position :: atom, (item.position.0 + offset), ((state.line.top, line_height), baseline) :: call
        state.atoms :: atom :: push
    let mut metric_spec = line_metric_spec :: state.line.start, state.logical_index, baseline :: call
    metric_spec.ascent = baseline
    metric_spec.descent = line_height - baseline
    metric_spec.height = line_height
    metric_spec.width = state.line.width
    metric_spec.left = offset
    metric_spec.top = state.line.top
    let metric = line_metric_value :: metric_spec :: call
    state.line_metrics :: metric :: push
    if state.line.width > state.longest_line:
        state.longest_line = state.line.width
    state.next_top = state.line.top + line_height
    state.line = working_line :: state.logical_index, state.next_top :: call

fn last_line_metric(read metrics: List[arcana_text.types.LineMetrics]) -> Option[arcana_text.types.LineMetrics]:
    let mut found = Option.None[arcana_text.types.LineMetrics] :: :: call
    for metric in metrics:
        found = Option.Some[arcana_text.types.LineMetrics] :: metric :: call
    return found

fn text_style_from_atom(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> arcana_text.types.TextStyle:
    let mut style = arcana_text.types.default_text_style :: atom.foreground :: call
    style.font_size = atom.font_size
    style.weight = atom.traits.weight
    style.width = atom.traits.width_milli / 1000
    style.slant = atom.traits.slant_milli / 1000
    style.line_height_milli = atom.line_height_milli
    style.families = std.collections.list.new[Str] :: :: call
    if atom.family_name != "":
        style.families :: atom.family_name :: push
    return style

fn ellipsis_source_atom(read atoms: List[arcana_text.provider_impl.engine.LayoutAtom], read metric: arcana_text.types.LineMetrics) -> Option[arcana_text.provider_impl.engine.LayoutAtom]:
    let mut found = Option.None[arcana_text.provider_impl.engine.LayoutAtom] :: :: call
    for atom in atoms:
        if (atom_top :: atom :: call) == metric.top and (atom_range_start :: atom :: call) < metric.end:
            found = Option.Some[arcana_text.provider_impl.engine.LayoutAtom] :: atom :: call
    return found

fn ellipsis_char_atom(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read request: arcana_text.provider_impl.engine.EllipsisAppendRequest) -> arcana_text.provider_impl.engine.LayoutAtom:
    let source = request.source
    let x = request.x
    let line_top = request.line_top
    let baseline = request.baseline
    let index = request.index
    let text = request.text
    let style = text_style_from_atom :: source :: call
    let selection = selected_face_entry :: paragraph.collection.faces, (style, text), source.family_name :: call
    paragraph.collection.faces = selection.0
    let mut render_spec = bitmap_render_request :: text, source.font_size, source.line_height_milli :: call
    render_spec.traits = source.traits
    let rendered = match selection.1:
        Option.Some(entry) => measured_selection_value :: paragraph.collection.faces, entry, render_spec :: call
        Option.None => unresolved_selection_value :: paragraph.collection.faces, (source.family_name, (max_int :: source.font_size, 1 :: call)) :: call
    paragraph.collection.faces = rendered.faces
    let bitmap = rendered.bitmap
    let width = bitmap.advance
    let mut atom = layout_atom_value :: (0, 0), (width, bitmap.line_height), (text_range_value :: index, index :: call) :: call
    atom.direction = source.direction
    atom.foreground = source.foreground
    atom.background_enabled = source.background_enabled
    atom.background = source.background
    atom.decorations = source.decorations
    atom.decoration_style = source.decoration_style
    atom.decoration_color = source.decoration_color
    atom.shadows = source.shadows
    atom.family_name = rendered.family
    atom.unresolved = false
    atom.text = text
    atom.glyph_index = render_spec.glyph_index
    atom.font_size = source.font_size
    atom.line_height_milli = source.line_height_milli
    atom.traits = source.traits
    atom.placeholder = false
    atom.bitmap = bitmap
    atom.size = (width, bitmap.line_height)
    atom.position = aligned_atom_position :: atom, x, ((line_top, bitmap.line_height), baseline) :: call
    return atom

fn append_ellipsis(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, edit state: arcana_text.provider_impl.engine.LayoutBuildState):
    if paragraph.paragraph_style.ellipsis == "":
        return
    let last_metric_option = last_line_metric :: state.line_metrics :: call
    if last_metric_option :: :: is_none:
        return
    let last_metric = last_metric_option :: (empty_line_metrics_value :: :: call) :: unwrap_or
    let source_option = ellipsis_source_atom :: state.atoms, last_metric :: call
    if source_option :: :: is_none:
        return
    let source = source_option :: (layout_atom_value :: (0, 0), (0, 0), (text_range_value :: 0, 0 :: call) :: call) :: unwrap_or
    let limit = match state.width:
        0 => state.longest_line
        _ => state.width
    let ellipsis_chars = utf8_chars :: paragraph.paragraph_style.ellipsis :: call
    let mut ellipsis_width = 0
    for ch in ellipsis_chars:
        let style = text_style_from_atom :: source :: call
        let selection = selected_face_entry :: paragraph.collection.faces, (style, ch), source.family_name :: call
        paragraph.collection.faces = selection.0
        let mut render_spec = bitmap_render_request :: ch, source.font_size, source.line_height_milli :: call
        render_spec.traits = source.traits
        let rendered = match selection.1:
            Option.Some(entry) => measured_selection_value :: paragraph.collection.faces, entry, render_spec :: call
            Option.None => unresolved_selection_value :: paragraph.collection.faces, (source.family_name, (max_int :: source.font_size, 1 :: call)) :: call
        paragraph.collection.faces = rendered.faces
        ellipsis_width += rendered.bitmap.advance
    let mut partition = ellipsis_partition_state :: last_metric.top, last_metric.left :: call
    for atom in state.atoms:
        partition_last_line_atom :: partition, atom :: call
    while partition.remaining_width + ellipsis_width > limit and not (partition.line :: :: is_empty):
        let removed = partition.line :: :: pop
        partition.remaining_width = max_int :: (removed.position.0 - last_metric.left), 0 :: call
    let mut kept = partition.kept
    let mut line_atoms = partition.line
    let mut x = last_metric.left + partition.remaining_width
    let chars = utf8_chars :: paragraph.paragraph_style.ellipsis :: call
    let baseline = last_metric.baseline
    for ch in chars:
        let mut request = ellipsis_append_request :: source, x, last_metric.top :: call
        request.baseline = baseline
        request.index = last_metric.end
        request.text = ch
        let atom = ellipsis_char_atom :: paragraph, request :: call
        x += atom.size.0
        line_atoms :: atom :: push
    while not (line_atoms :: :: is_empty):
        kept :: (line_atoms :: :: pop) :: push
    state.atoms = kept
    let mut metrics = empty_lines :: :: call
    for metric in state.line_metrics:
        if metric.top == last_metric.top:
            let mut updated = metric
            updated.width = x - last_metric.left
            metrics :: updated :: push
        else:
            metrics :: metric :: push
    state.line_metrics = metrics

fn truncate_to_max_lines(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, edit state: arcana_text.provider_impl.engine.LayoutBuildState) -> Bool:
    let paragraph_style = paragraph.paragraph_style
    if paragraph_style.max_lines <= 0:
        return false
    if (state.line_metrics :: :: len) <= paragraph_style.max_lines:
        return false
    let keep = paragraph_style.max_lines
    let mut kept_lines = empty_lines :: :: call
    let mut index = 0
    let mut keep_end = 0
    for metric in state.line_metrics:
        if index >= keep:
            break
        keep_end = metric.end
        kept_lines :: metric :: push
        index += 1
    state.line_metrics = kept_lines
    let mut kept_atoms = empty_atoms :: :: call
    for atom in state.atoms:
        append_kept_atom_if_before_end :: kept_atoms, atom, keep_end :: call
    state.atoms = kept_atoms
    let mut kept_text = empty_strings :: :: call
    let mut kept_count = 0
    for value in state.flattened:
        if kept_count >= keep_end:
            break
        kept_text :: value :: push
        kept_count += 1
    state.flattened = kept_text
    append_ellipsis :: paragraph, state :: call
    return true

fn process_text_run(edit state: arcana_text.provider_impl.engine.LayoutBuildState, edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read run: arcana_text.provider_impl.engine.TextRunState):
    let tokens = tokenize_text_run :: run.text :: call
    for token in tokens:
        if token.newline:
            finalize_line :: paragraph.paragraph_style, state :: call
            continue
        if token.whitespace:
            if state.line.atoms :: :: is_empty:
                continue
            let chars = utf8_chars :: token.text :: call
            for raw in chars:
                let advance = glyph_advance :: run.style, raw :: call
                if layout_needs_wrap :: state, advance :: call:
                    finalize_line :: paragraph.paragraph_style, state :: call
                    break
                process_text_char :: state, paragraph, (arcana_text.provider_impl.engine.TextAppendRequest :: run = run, text = raw, logical_index = state.logical_index :: call) :: call
            continue
        let estimate = token_width :: run.style, token.text :: call
        if layout_needs_wrap :: state, estimate :: call:
            finalize_line :: paragraph.paragraph_style, state :: call
        let chars = utf8_chars :: token.text :: call
        for raw in chars:
            let advance = glyph_advance :: run.style, raw :: call
            if layout_needs_wrap :: state, advance :: call:
                finalize_line :: paragraph.paragraph_style, state :: call
            process_text_char :: state, paragraph, (arcana_text.provider_impl.engine.TextAppendRequest :: run = run, text = raw, logical_index = state.logical_index :: call) :: call

fn process_placeholder(edit state: arcana_text.provider_impl.engine.LayoutBuildState, edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read placeholder: arcana_text.types.PlaceholderStyle):
    let atom_width = max_int :: placeholder.size.0, 1 :: call
    if layout_needs_wrap :: state, atom_width :: call:
        finalize_line :: paragraph.paragraph_style, state :: call
    let placeholder_spec = arcana_text.provider_impl.engine.PlaceholderAppendRequest :: placeholder = placeholder, logical_index = state.logical_index :: call
    append_placeholder_atom :: paragraph, state.line, placeholder_spec :: call
    state.flattened :: "\u{FFFC}" :: push
    let used_family = line_family_name :: state.line :: call
    push_unique_string :: state.fonts_used, used_family :: call
    state.logical_index += 1

fn first_line_baseline_or(read metrics: List[arcana_text.types.LineMetrics], fallback: Int) -> Int:
    for metric in metrics:
        return metric.baseline
    return fallback

fn first_line_height_or(read metrics: List[arcana_text.types.LineMetrics], fallback: Int) -> Int:
    for metric in metrics:
        return metric.height
    return fallback

fn string_at_or_empty(read values: List[Str], target: Int) -> Str:
    let mut index = 0
    for value in values:
        if index == target:
            return value
        index += 1
    return ""

fn text_token(text: Str, whitespace: Bool, newline: Bool) -> arcana_text.provider_impl.engine.TextToken:
    return arcana_text.provider_impl.engine.TextToken :: text = text, whitespace = whitespace, newline = newline :: call

fn tokenize_text_run(read text: Str) -> List[arcana_text.provider_impl.engine.TextToken]:
    let chars = utf8_chars :: text :: call
    let mut out = std.collections.list.new[arcana_text.provider_impl.engine.TextToken] :: :: call
    let mut current = ""
    let mut current_whitespace = false
    let mut active = false
    for ch in chars:
        if is_newline :: ch :: call:
            if active:
                out :: (text_token :: current, current_whitespace, false :: call) :: push
                current = ""
                current_whitespace = false
                active = false
            out :: (text_token :: ch, false, true :: call) :: push
            continue
        let whitespace = is_space :: ch :: call
        if not active:
            current = ch
            current_whitespace = whitespace
            active = true
            continue
        if whitespace == current_whitespace:
            current = current + ch
            continue
        out :: (text_token :: current, current_whitespace, false :: call) :: push
        current = ch
        current_whitespace = whitespace
        active = true
    if active:
        out :: (text_token :: current, current_whitespace, false :: call) :: push
    return out

fn token_width(read style: arcana_text.types.TextStyle, read text: Str) -> Int:
    let chars = utf8_chars :: text :: call
    let mut width = 0
    for ch in chars:
        width += glyph_advance :: style, ch :: call
    return width

fn process_text_char(edit state: arcana_text.provider_impl.engine.LayoutBuildState, edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read request: arcana_text.provider_impl.engine.TextAppendRequest):
    let run = request.run
    let raw = request.text
    if paragraph.paragraph_style.replace_tab_characters and (is_tab :: raw :: call):
        let mut repeats = 0
        while repeats < 4:
            let ch = " "
            let text_spec = arcana_text.provider_impl.engine.TextAppendRequest :: run = run, text = ch, logical_index = state.logical_index :: call
            append_text_char :: paragraph, state.line, text_spec :: call
            state.flattened :: ch :: push
            let used_family = line_family_name :: state.line :: call
            push_unique_string :: state.fonts_used, used_family :: call
            state.logical_index += 1
            repeats += 1
        return
    let text_spec = arcana_text.provider_impl.engine.TextAppendRequest :: run = run, text = raw, logical_index = state.logical_index :: call
    let unresolved = append_text_char :: paragraph, state.line, text_spec :: call
    state.flattened :: raw :: push
    let used_family = line_family_name :: state.line :: call
    push_unique_string :: state.fonts_used, used_family :: call
    if unresolved:
        let unresolved_index = state.logical_index
        state.unresolved :: unresolved_index :: push
    state.logical_index += 1

fn placeholder_boxes_from_atoms(read atoms: List[arcana_text.provider_impl.engine.LayoutAtom]) -> List[arcana_text.types.TextBox]:
    let mut out = empty_boxes :: :: call
    for atom in atoms:
        if atom.placeholder:
            let box = text_box_value :: atom.position, atom.size, (atom.range, atom.direction) :: call
            out :: box :: push
    return out

export fn layout_paragraph(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, requested_width: Int):
    let mut width = requested_width
    if width < 0:
        width = 0
    let wrap_width = match width:
        0 => 1073741824
        _ => width
    let mut state = layout_build_state :: width, wrap_width :: call

    for item in paragraph.items:
        match item:
            arcana_text.provider_impl.engine.BuilderItemState.Text(run) => process_text_run :: state, paragraph, run :: call
            arcana_text.provider_impl.engine.BuilderItemState.Placeholder(placeholder) => process_placeholder :: state, paragraph, placeholder :: call

    finalize_line :: paragraph.paragraph_style, state :: call

    if (state.atoms :: :: is_empty) and (state.line_metrics :: :: is_empty):
        let mut metric_spec = line_metric_spec :: 0, 0, 12 :: call
        metric_spec.ascent = 12
        metric_spec.descent = 4
        metric_spec.height = 16
        metric_spec.width = 0
        metric_spec.left = 0
        metric_spec.top = 0
        let metric = line_metric_value :: metric_spec :: call
        state.line_metrics :: metric :: push
        state.next_top = 16

    let exceeded = truncate_to_max_lines :: paragraph, state :: call
    let placeholder_boxes = placeholder_boxes_from_atoms :: state.atoms :: call

    let baseline = first_line_baseline_or :: state.line_metrics, 12 :: call
    let ideographic = first_line_height_or :: state.line_metrics, 16 :: call
    let measured_width = match width:
        0 => state.longest_line
        _ => width
    let mut layout = arcana_text.provider_impl.engine.LayoutState :: requested_width = width, width = measured_width, height = (max_int :: state.next_top, 16 :: call) :: call
    layout.longest_line = state.longest_line
    layout.exceeded_max_lines = exceeded
    layout.alphabetic_baseline = baseline
    layout.ideographic_baseline = ideographic
    layout.line_metrics = state.line_metrics
    layout.atoms = state.atoms
    layout.placeholder_boxes = placeholder_boxes
    layout.unresolved_glyphs = state.unresolved
    layout.fonts_used = state.fonts_used
    layout.flattened_text = state.flattened
    paragraph.layout = Option.Some[arcana_text.provider_impl.engine.LayoutState] :: layout :: call

export fn longest_line(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return match paragraph.layout:
        Option.Some(layout) => layout.longest_line
        Option.None => 0

export fn height(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return match paragraph.layout:
        Option.Some(layout) => layout.height
        Option.None => 0

export fn max_intrinsic_width(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return longest_line :: paragraph :: call

export fn min_intrinsic_width(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return longest_line :: paragraph :: call

export fn alphabetic_baseline(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return match paragraph.layout:
        Option.Some(layout) => layout.alphabetic_baseline
        Option.None => 12

export fn ideographic_baseline(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return match paragraph.layout:
        Option.Some(layout) => layout.ideographic_baseline
        Option.None => 16

export fn exceeded_max_lines(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Bool:
    return match paragraph.layout:
        Option.Some(layout) => layout.exceeded_max_lines
        Option.None => false

export fn line_metrics_list(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[arcana_text.types.LineMetrics]:
    return match paragraph.layout:
        Option.Some(layout) => layout.line_metrics
        Option.None => (empty_lines :: :: call)

fn range_boxes_from_layout(read layout: arcana_text.provider_impl.engine.LayoutState, start: Int, end: Int) -> List[arcana_text.types.TextBox]:
    let mut out = empty_boxes :: :: call
    for atom in layout.atoms:
        if atom.range.end > start and atom.range.start < end:
            let box = text_box_value :: atom.position, atom.size, (atom.range, atom.direction) :: call
            out :: box :: push
    return out

export fn range_boxes_list(read paragraph: arcana_text.provider_impl.engine.ParagraphState, start: Int, end: Int) -> List[arcana_text.types.TextBox]:
    return match paragraph.layout:
        Option.None => (empty_boxes :: :: call)
        Option.Some(layout) => range_boxes_from_layout :: layout, start, end :: call

export fn placeholder_boxes_list(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[arcana_text.types.TextBox]:
    return match paragraph.layout:
        Option.Some(layout) => layout.placeholder_boxes
        Option.None => (empty_boxes :: :: call)

fn position_at_in_layout(read layout: arcana_text.provider_impl.engine.LayoutState, pos: (Int, Int), read affinities: (arcana_text.types.Affinity, arcana_text.types.Affinity)) -> arcana_text.types.PositionWithAffinity:
    let mut last = 0
    for atom in layout.atoms:
        last = atom.range.end
        let within_y = pos.1 >= atom.position.1 and pos.1 < atom.position.1 + atom.size.1
        if not within_y:
            continue
        let mid = atom.position.0 + atom.size.0 / 2
        if pos.0 < mid:
            return match atom.direction:
                arcana_text.types.TextDirection.RightToLeft => position_with_affinity :: atom.range.end, affinities.0 :: call
                _ => position_with_affinity :: atom.range.start, affinities.1 :: call
        if pos.0 < atom.position.0 + atom.size.0:
            return match atom.direction:
                arcana_text.types.TextDirection.RightToLeft => position_with_affinity :: atom.range.start, affinities.1 :: call
                _ => position_with_affinity :: atom.range.end, affinities.0 :: call
    return position_with_affinity :: last, affinities.0 :: call

export fn position_at_value(read paragraph: arcana_text.provider_impl.engine.ParagraphState, pos: (Int, Int)) -> arcana_text.types.PositionWithAffinity:
    let downstream = arcana_text.types.Affinity.Downstream :: :: call
    let upstream = arcana_text.types.Affinity.Upstream :: :: call
    return match paragraph.layout:
        Option.None => position_with_affinity :: 0, downstream :: call
        Option.Some(layout) => position_at_in_layout :: layout, pos, (downstream, upstream) :: call

fn word_boundary_in_layout(read layout: arcana_text.provider_impl.engine.LayoutState, index: Int) -> arcana_text.types.TextRange:
    let len = layout.flattened_text :: :: len
    if len == 0:
        return text_range_value :: 0, 0 :: call
    let mut start = clamp_int :: index, 0, len - 1 :: call
    let mut end = start
    while start > 0 and not (is_space :: (string_at_or_empty :: layout.flattened_text, start - 1 :: call) :: call):
        start -= 1
    while end < len and not (is_space :: (string_at_or_empty :: layout.flattened_text, end :: call) :: call):
        end += 1
    return text_range_value :: start, end :: call

export fn word_boundary_value(read paragraph: arcana_text.provider_impl.engine.ParagraphState, index: Int) -> arcana_text.types.TextRange:
    return match paragraph.layout:
        Option.None => text_range_value :: 0, 0 :: call
        Option.Some(layout) => word_boundary_in_layout :: layout, index :: call

export fn unresolved_glyphs_list(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[Int]:
    return match paragraph.layout:
        Option.Some(layout) => layout.unresolved_glyphs
        Option.None => (empty_ints :: :: call)

export fn fonts_used_list(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[Str]:
    return match paragraph.layout:
        Option.Some(layout) => layout.fonts_used
        Option.None => (empty_strings :: :: call)

fn relayout(edit paragraph: arcana_text.provider_impl.engine.ParagraphState):
    let requested = match paragraph.layout:
        Option.Some(layout) => layout.requested_width
        Option.None => 0
    paragraph.layout = Option.None[arcana_text.provider_impl.engine.LayoutState] :: :: call
    layout_paragraph :: paragraph, requested :: call

fn text_style_from_item(read item: arcana_text.provider_impl.engine.BuilderItemState) -> Option[arcana_text.types.TextStyle]:
    return match item:
        arcana_text.provider_impl.engine.BuilderItemState.Text(run) => Option.Some[arcana_text.types.TextStyle] :: run.style :: call
        _ => Option.None[arcana_text.types.TextStyle] :: :: call

fn item_has_text_style(read item: arcana_text.provider_impl.engine.BuilderItemState) -> Bool:
    return match item:
        arcana_text.provider_impl.engine.BuilderItemState.Text(_) => true
        _ => false

fn first_text_style(read items: List[arcana_text.provider_impl.engine.BuilderItemState]) -> arcana_text.types.TextStyle:
    let fallback = default_provider_text_style :: :: call
    for item in items:
        if not (item_has_text_style :: item :: call):
            continue
        let style = match item:
            arcana_text.provider_impl.engine.BuilderItemState.Text(run) => run.style
            _ => fallback
        return style
    return fallback

fn text_item_with_font_size(read run: arcana_text.provider_impl.engine.TextRunState, font_size: Int) -> arcana_text.provider_impl.engine.BuilderItemState:
    let mut style = run.style
    style.font_size = font_size
    return text_item :: style, run.text :: call

fn item_with_font_size(read item: arcana_text.provider_impl.engine.BuilderItemState, font_size: Int) -> arcana_text.provider_impl.engine.BuilderItemState:
    return match item:
        arcana_text.provider_impl.engine.BuilderItemState.Text(run) => text_item_with_font_size :: run, font_size :: call
        arcana_text.provider_impl.engine.BuilderItemState.Placeholder(value) => placeholder_item :: value :: call

fn text_item_with_foreground(read run: arcana_text.provider_impl.engine.TextRunState, read paint: arcana_graphics.types.Paint) -> arcana_text.provider_impl.engine.BuilderItemState:
    let mut style = run.style
    style.foreground = paint
    return text_item :: style, run.text :: call

fn item_with_foreground(read item: arcana_text.provider_impl.engine.BuilderItemState, read paint: arcana_graphics.types.Paint) -> arcana_text.provider_impl.engine.BuilderItemState:
    return match item:
        arcana_text.provider_impl.engine.BuilderItemState.Text(run) => text_item_with_foreground :: run, paint :: call
        arcana_text.provider_impl.engine.BuilderItemState.Placeholder(value) => placeholder_item :: value :: call

fn text_item_with_background(read run: arcana_text.provider_impl.engine.TextRunState, enabled: Bool, read paint: arcana_graphics.types.Paint) -> arcana_text.provider_impl.engine.BuilderItemState:
    let mut style = run.style
    style.background_enabled = enabled
    style.background = paint
    return text_item :: style, run.text :: call

fn item_with_background(read item: arcana_text.provider_impl.engine.BuilderItemState, enabled: Bool, read paint: arcana_graphics.types.Paint) -> arcana_text.provider_impl.engine.BuilderItemState:
    return match item:
        arcana_text.provider_impl.engine.BuilderItemState.Text(run) => text_item_with_background :: run, enabled, paint :: call
        arcana_text.provider_impl.engine.BuilderItemState.Placeholder(value) => placeholder_item :: value :: call

export fn update_text_value(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, text: Str):
    let style = first_text_style :: paragraph.items :: call
    let mut items = empty_items :: :: call
    let item = text_item :: style, text :: call
    items :: item :: push
    paragraph.items = items
    relayout :: paragraph :: call

export fn update_align_value(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read align: arcana_text.types.TextAlign):
    paragraph.paragraph_style.align = align
    relayout :: paragraph :: call

export fn update_font_size_value(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, font_size: Int):
    let mut items = empty_items :: :: call
    let size = max_int :: font_size, 1 :: call
    for item in paragraph.items:
        let next = item_with_font_size :: item, size :: call
        items :: next :: push
    paragraph.items = items
    relayout :: paragraph :: call

export fn update_foreground_value(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read paint: arcana_graphics.types.Paint):
    let mut items = empty_items :: :: call
    for item in paragraph.items:
        let next = item_with_foreground :: item, paint :: call
        items :: next :: push
    paragraph.items = items
    relayout :: paragraph :: call

export fn update_background_value(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, enabled: Bool, read paint: arcana_graphics.types.Paint):
    let mut items = empty_items :: :: call
    for item in paragraph.items:
        let next = item_with_background :: item, enabled, paint :: call
        items :: next :: push
    paragraph.items = items
    relayout :: paragraph :: call

fn render_spec_for_atom(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> arcana_text.provider_impl.engine.BitmapRenderRequest:
    let mut spec = bitmap_render_request :: atom.text, atom.font_size, atom.line_height_milli :: call
    spec.glyph_index = atom.glyph_index
    spec.traits = atom.traits
    return spec

fn rendered_layout_atoms(read paragraph: arcana_text.provider_impl.engine.ParagraphState, read layout: arcana_text.provider_impl.engine.LayoutState) -> List[arcana_text.provider_impl.engine.LayoutAtom]:
    let mut faces = paragraph.collection.faces
    let mut out = empty_atoms :: :: call
    for item in layout.atoms:
        let mut atom = item
        if not atom.placeholder and not atom.unresolved:
            let selection = read_selected_face_entry :: faces, (atom.traits, atom.text), atom.family_name :: call
            if selection :: :: is_some:
                let entry = selection :: (empty_registered_face :: :: call) :: unwrap_or
                let rendered = rendered_selection_value :: faces, entry, (render_spec_for_atom :: atom :: call) :: call
                faces = rendered.faces
                atom.bitmap = rendered.bitmap
        out :: atom :: push
    return out

fn empty_pixel_map() -> Map[Int, Int]:
    return std.collections.map.new[Int, Int] :: :: call

fn pixel_index(width: Int, x: Int, y: Int) -> Int:
    return (y * width) + x

fn pixel_write_spec(width: Int, pos: (Int, Int), color: Int) -> arcana_text.provider_impl.engine.PixelWriteSpec:
    return arcana_text.provider_impl.engine.PixelWriteSpec :: width = width, pos = pos, color = color :: call

fn pixel_rect_spec(dims: (Int, Int), rect: ((Int, Int), (Int, Int)), color: Int) -> arcana_text.provider_impl.engine.PixelRectSpec:
    return arcana_text.provider_impl.engine.PixelRectSpec :: dims = dims, rect = rect, color = color :: call

fn set_pixel(edit pixels: Map[Int, Int], read spec: arcana_text.provider_impl.engine.PixelWriteSpec):
    if spec.pos.0 < 0 or spec.pos.1 < 0 or spec.pos.0 >= spec.width:
        return
    let key = pixel_index :: spec.width, spec.pos.0, spec.pos.1 :: call
    pixels :: key, spec.color :: set

fn fill_rect_pixels(edit pixels: Map[Int, Int], read spec: arcana_text.provider_impl.engine.PixelRectSpec):
    let width = spec.dims.0
    let height = spec.dims.1
    let mut y = spec.rect.0.1
    let max_y = spec.rect.0.1 + spec.rect.1.1
    while y < max_y:
        if y >= 0 and y < height:
            let mut x = spec.rect.0.0
            let max_x = spec.rect.0.0 + spec.rect.1.0
            while x < max_x:
                if x >= 0 and x < width:
                    set_pixel :: pixels, (pixel_write_spec :: width, (x, y), spec.color :: call) :: call
                x += 1
        y += 1

fn paint_bitmap_pixels(edit pixels: Map[Int, Int], dims: (Int, Int), read atom: arcana_text.provider_impl.engine.LayoutAtom):
    if atom.bitmap.empty:
        return
    let width = dims.0
    let height = dims.1
    let mut y = 0
    while y < atom.bitmap.size.1:
        let py = atom.position.1 + atom.bitmap.offset.1 + y
        if py >= 0 and py < height:
            let mut x = 0
            while x < atom.bitmap.size.0:
                let px = atom.position.0 + atom.bitmap.offset.0 + x
                if px >= 0 and px < width:
                    let alpha = std.bytes.at :: atom.bitmap.alpha, (y * atom.bitmap.size.0) + x :: call
                    if alpha > 0:
                        set_pixel :: pixels, (pixel_write_spec :: width, (px, py), atom.foreground :: call) :: call
                x += 1
        y += 1

fn paint_bitmap_pixels_with_offset(edit pixels: Map[Int, Int], dims: (Int, Int), read spec: (arcana_text.provider_impl.engine.LayoutAtom, ((Int, Int), Int))):
    let atom = spec.0
    let delta = spec.1.0
    let color = spec.1.1
    if atom.bitmap.empty:
        return
    let width = dims.0
    let height = dims.1
    let mut y = 0
    while y < atom.bitmap.size.1:
        let py = atom.position.1 + atom.bitmap.offset.1 + y + delta.1
        if py >= 0 and py < height:
            let mut x = 0
            while x < atom.bitmap.size.0:
                let px = atom.position.0 + atom.bitmap.offset.0 + x + delta.0
                if px >= 0 and px < width:
                    let alpha = std.bytes.at :: atom.bitmap.alpha, (y * atom.bitmap.size.0) + x :: call
                    if alpha > 0:
                        set_pixel :: pixels, (pixel_write_spec :: width, (px, py), color :: call) :: call
                x += 1
        y += 1

fn decoration_thickness(read atom: arcana_text.provider_impl.engine.LayoutAtom) -> Int:
    return max_int :: (atom.font_size / 12), 1 :: call

fn decoration_y(read atom: arcana_text.provider_impl.engine.LayoutAtom, read decoration: arcana_text.types.TextDecoration) -> Int:
    let top = atom.position.1
    let baseline = atom.position.1 + atom.bitmap.baseline
    return match decoration:
        arcana_text.types.TextDecoration.Overline => top + 1
        arcana_text.types.TextDecoration.LineThrough => top + (atom.size.1 / 2)
        _ => baseline + 1

fn paint_decoration_span(edit pixels: Map[Int, Int], dims: (Int, Int), read spec: arcana_text.provider_impl.engine.DecorationSpanSpec):
    let width = dims.0
    let height = dims.1
    let mut row = 0
    while row < spec.thickness:
        let py = spec.y + row
        if py >= 0 and py < height:
            let mut x = spec.x0
            while x < spec.x1:
                if x >= 0 and x < width:
                    let draw = match spec.style:
                        arcana_text.types.TextDecorationStyle.Dotted => ((x - spec.x0) % 4) == 0
                        arcana_text.types.TextDecorationStyle.Dashed => ((x - spec.x0) % 8) < 5
                        arcana_text.types.TextDecorationStyle.Wavy => (positive_mod :: ((x - spec.x0) + row), 6 :: call) < 3
                        _ => true
                    if draw:
                        set_pixel :: pixels, (pixel_write_spec :: width, (x, py), spec.color :: call) :: call
                x += 1
        row += 1

fn paint_atom_decorations(edit pixels: Map[Int, Int], dims: (Int, Int), read atom: arcana_text.provider_impl.engine.LayoutAtom):
    if atom.placeholder:
        return
    let x0 = atom.position.0
    let x1 = atom.position.0 + atom.size.0
    let thickness = decoration_thickness :: atom :: call
    for decoration in atom.decorations:
        let y = decoration_y :: atom, decoration :: call
        let mut spec = decoration_span_spec :: x0, x1, y :: call
        spec.thickness = thickness
        spec.color = atom.decoration_color
        spec.style = atom.decoration_style
        paint_decoration_span :: pixels, dims, spec :: call
        if atom.decoration_style == (arcana_text.types.TextDecorationStyle.Double :: :: call):
            let mut second = decoration_span_spec :: x0, x1, (y + (thickness + 1)) :: call
            second.thickness = thickness
            second.color = atom.decoration_color
            second.style = arcana_text.types.TextDecorationStyle.Solid :: :: call
            paint_decoration_span :: pixels, dims, second :: call

fn painted_pixels(read atoms: List[arcana_text.provider_impl.engine.LayoutAtom], dims: (Int, Int)) -> Map[Int, Int]:
    let mut pixels = empty_pixel_map :: :: call
    for atom in atoms:
        if atom.background_enabled:
            fill_rect_pixels :: pixels, (pixel_rect_spec :: dims, (atom.position, atom.size), atom.background :: call) :: call
        for shadow in atom.shadows:
            if atom.placeholder:
                let shadow_pos = (atom.position.0 + shadow.offset.0, atom.position.1 + shadow.offset.1)
                fill_rect_pixels :: pixels, (pixel_rect_spec :: dims, (shadow_pos, atom.size), shadow.paint.color :: call) :: call
                continue
            paint_bitmap_pixels_with_offset :: pixels, dims, (atom, (shadow.offset, shadow.paint.color)) :: call
        if atom.placeholder:
            fill_rect_pixels :: pixels, (pixel_rect_spec :: dims, (atom.position, atom.size), atom.foreground :: call) :: call
            continue
        if atom.text == " " or atom.text == "\t" or atom.text == "\n" or atom.text == "\r":
            paint_atom_decorations :: pixels, dims, atom :: call
            continue
        paint_bitmap_pixels :: pixels, dims, atom :: call
        paint_atom_decorations :: pixels, dims, atom :: call
    return pixels

fn empty_layout_state() -> arcana_text.provider_impl.engine.LayoutState:
    let mut out = arcana_text.provider_impl.engine.LayoutState :: requested_width = 0, width = 0, height = 0 :: call
    out.longest_line = 0
    out.exceeded_max_lines = false
    out.alphabetic_baseline = 12
    out.ideographic_baseline = 16
    out.line_metrics = empty_lines :: :: call
    out.atoms = empty_atoms :: :: call
    out.placeholder_boxes = empty_boxes :: :: call
    out.unresolved_glyphs = empty_ints :: :: call
    out.fonts_used = empty_strings :: :: call
    out.flattened_text = empty_strings :: :: call
    return out

fn has_layout(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Bool:
    return match paragraph.layout:
        Option.Some(_) => true
        Option.None => false

fn layout_or_default(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> arcana_text.provider_impl.engine.LayoutState:
    return match paragraph.layout:
        Option.Some(value) => value
        Option.None => empty_layout_state :: :: call

export fn paint_paragraph(edit win: std.window.Window, read paragraph: arcana_text.provider_impl.engine.ParagraphState, origin: (Int, Int)):
    if not (has_layout :: paragraph :: call):
        return
    let layout = layout_or_default :: paragraph :: call
    let atoms = rendered_layout_atoms :: paragraph, layout :: call
    let width = max_int :: layout.width, 1 :: call
    let height = max_int :: layout.height, 1 :: call
    let pixels = painted_pixels :: atoms, (width, height) :: call
    let mut rgba = std.collections.list.new[Int] :: :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let key = pixel_index :: width, x, y :: call
            let color = pixels :: key, 0 :: get_or
            let visible = pixels :: key :: has
            rgba :: (color_red :: color :: call) :: push
            rgba :: (color_green :: color :: call) :: push
            rgba :: (color_blue :: color :: call) :: push
            if visible:
                rgba :: 255 :: push
            else:
                rgba :: 0 :: push
            x += 1
        y += 1
    let mut image = std.canvas.image_create :: width, height :: call
    std.canvas.image_replace_rgba :: image, (std.collections.array.from_list[Int] :: rgba :: call) :: call
    std.canvas.blit :: win, image, origin.0 :: call
        y = origin.1
