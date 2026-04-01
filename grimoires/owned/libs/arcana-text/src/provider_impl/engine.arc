import std.bytes
import std.canvas
import std.collections.array
import std.collections.list
import std.option
import std.text
import std.window
use std.option.Option

export record FontCollectionState:
    registered_families: List[Str]
    registered_sources: List[Str]
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
    family_name: Str
    unresolved: Bool
    text: Str
    font_size: Int
    placeholder: Bool

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

record TextAppendSpec:
    collection: arcana_text.provider_impl.engine.FontCollectionState
    paragraph_style: arcana_text.types.ParagraphStyle
    text: Str
    logical_index: Int

record PlaceholderAppendSpec:
    collection: arcana_text.provider_impl.engine.FontCollectionState
    paragraph_style: arcana_text.types.ParagraphStyle
    placeholder: arcana_text.types.PlaceholderStyle
    logical_index: Int

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

fn default_provider_text_style() -> arcana_text.types.TextStyle:
    return arcana_text.types.default_text_style :: 16777215 :: call

export fn new_collection_state() -> arcana_text.provider_impl.engine.FontCollectionState:
    return arcana_text.provider_impl.engine.FontCollectionState :: registered_families = (empty_strings :: :: call), registered_sources = (empty_strings :: :: call), host_fallback_enabled = false :: call

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

fn text_append_spec(read collection: arcana_text.provider_impl.engine.FontCollectionState, read paragraph_style: arcana_text.types.ParagraphStyle, read text: Str) -> arcana_text.provider_impl.engine.TextAppendSpec:
    let mut out = arcana_text.provider_impl.engine.TextAppendSpec :: collection = collection, paragraph_style = paragraph_style, text = text :: call
    out.logical_index = 0
    return out

fn placeholder_append_spec(read collection: arcana_text.provider_impl.engine.FontCollectionState, read paragraph_style: arcana_text.types.ParagraphStyle, read placeholder: arcana_text.types.PlaceholderStyle) -> arcana_text.provider_impl.engine.PlaceholderAppendSpec:
    let mut out = arcana_text.provider_impl.engine.PlaceholderAppendSpec :: collection = collection, paragraph_style = paragraph_style, placeholder = placeholder :: call
    out.logical_index = 0
    return out

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

fn is_ascii_alpha_num(read text: Str) -> Bool:
    if (std.text.len_bytes :: text :: call) != 1:
        return false
    let b = std.text.byte_at :: text, 0 :: call
    return (std.text.is_alpha_byte :: b :: call) or (std.text.is_digit_byte :: b :: call)

fn is_renderable(read text: Str) -> Bool:
    return (std.text.len_bytes :: text :: call) == 1 and (std.text.byte_at :: text, 0 :: call) < 128

fn preferred_collection_name(read collection: arcana_text.provider_impl.engine.FontCollectionState) -> Str:
    return first_string_or :: collection.registered_families, "Monaspace Neon" :: call

fn preferred_family_name(read style: arcana_text.types.TextStyle, read collection: arcana_text.provider_impl.engine.FontCollectionState) -> Str:
    return first_string_or :: style.families, (preferred_collection_name :: collection :: call) :: call

fn push_unique_string(edit out: List[Str], read value: Str):
    for existing in out:
        if existing == value:
            return
    out :: value :: push

fn glyph_cell_width(font_size: Int) -> Int:
    let size = max_int :: font_size, 8 :: call
    return ((size * 8) + 15) / 16

fn glyph_cell_height(font_size: Int) -> Int:
    return max_int :: font_size, 8 :: call

fn glyph_advance(read style: arcana_text.types.TextStyle, read ch: Str) -> Int:
    let base = glyph_cell_width :: style.font_size :: call
    let mut extra = style.letter_spacing_milli / 1000
    if ch == " ":
        extra = style.word_spacing_milli / 1000
    let out = base + extra
    if out <= 0:
        return 1
    return out

fn line_height_for_style(read style: arcana_text.types.TextStyle) -> Int:
    let base = glyph_cell_height :: style.font_size :: call
    let scale = max_int :: style.line_height_milli, 1000 :: call
    return (base * scale) / 1000

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

fn append_text_char(edit line: arcana_text.provider_impl.engine.WorkingLine, read style: arcana_text.types.TextStyle, read spec: arcana_text.provider_impl.engine.TextAppendSpec):
    let width = glyph_advance :: style, spec.text :: call
    let height = line_height_for_style :: style :: call
    let baseline = (height * 800) / 1000
    if height > line.height:
        line.height = height
    if baseline > line.baseline:
        line.baseline = baseline
    let family = preferred_family_name :: style, spec.collection :: call
    let mut atom = arcana_text.provider_impl.engine.LayoutAtom :: position = (line.width, line.top), size = (width, height), range = (text_range_value :: spec.logical_index, spec.logical_index + 1 :: call) :: call
    atom.direction = spec.paragraph_style.direction
    atom.foreground = style.foreground.color
    atom.background_enabled = style.background_enabled
    atom.background = style.background.color
    atom.family_name = family
    atom.unresolved = not (is_renderable :: spec.text :: call)
    atom.text = spec.text
    atom.font_size = style.font_size
    atom.placeholder = false
    line.width += width
    line.atoms :: atom :: push

fn append_placeholder_atom(edit line: arcana_text.provider_impl.engine.WorkingLine, read spec: arcana_text.provider_impl.engine.PlaceholderAppendSpec):
    let width = max_int :: spec.placeholder.size.0, 1 :: call
    let height = max_int :: spec.placeholder.size.1, 16 :: call
    let baseline = (height * 800) / 1000
    if height > line.height:
        line.height = height
    if baseline > line.baseline:
        line.baseline = baseline
    let mut atom = arcana_text.provider_impl.engine.LayoutAtom :: position = (line.width, line.top), size = (width, height), range = (text_range_value :: spec.logical_index, spec.logical_index + 1 :: call) :: call
    atom.direction = spec.paragraph_style.direction
    atom.foreground = 16777215
    atom.background_enabled = true
    atom.background = 4214880
    atom.family_name = preferred_collection_name :: spec.collection :: call
    atom.unresolved = false
    atom.text = "\u{FFFC}"
    atom.font_size = 16
    atom.placeholder = true
    line.width += width
    line.atoms :: atom :: push

fn layout_needs_wrap(read state: arcana_text.provider_impl.engine.LayoutBuildState, advance: Int) -> Bool:
    return state.width > 0 and not (state.line.atoms :: :: is_empty) and state.line.width + advance > state.wrap_width

fn finalize_line(read paragraph_style: arcana_text.types.ParagraphStyle, edit state: arcana_text.provider_impl.engine.LayoutBuildState):
    if state.line.atoms :: :: is_empty:
        state.line.top = state.next_top
        state.line.start = state.logical_index
        return
    let direction = resolved_direction :: paragraph_style, state.line.atoms :: call
    let offset = alignment_offset :: paragraph_style.align, direction, (state.width, state.line.width) :: call
    let line_height = max_int :: state.line.height, 16 :: call
    let baseline = max_int :: state.line.baseline, ((line_height * 800) / 1000) :: call
    for item in state.line.atoms:
        let mut atom = item
        atom.direction = direction
        if direction == (arcana_text.types.TextDirection.RightToLeft :: :: call):
            atom.position = (offset + (state.line.width - item.position.0 - item.size.0), state.line.top)
        else:
            atom.position = (item.position.0 + offset, state.line.top)
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

fn truncate_to_max_lines(read paragraph_style: arcana_text.types.ParagraphStyle, edit state: arcana_text.provider_impl.engine.LayoutBuildState) -> Bool:
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
        if atom.range.start < keep_end:
            kept_atoms :: atom :: push
    state.atoms = kept_atoms
    let mut kept_text = empty_strings :: :: call
    let mut kept_count = 0
    for value in state.flattened:
        if kept_count >= keep_end:
            break
        kept_text :: value :: push
        kept_count += 1
    state.flattened = kept_text
    return true

fn process_text_run(edit state: arcana_text.provider_impl.engine.LayoutBuildState, read paragraph: arcana_text.provider_impl.engine.ParagraphState, read run: arcana_text.provider_impl.engine.TextRunState):
    let chars = utf8_chars :: run.text :: call
    for raw in chars:
        if paragraph.paragraph_style.replace_tab_characters and (is_tab :: raw :: call):
            let mut repeats = 0
            while repeats < 4:
                let ch = " "
                let advance = glyph_advance :: run.style, ch :: call
                if layout_needs_wrap :: state, advance :: call:
                    finalize_line :: paragraph.paragraph_style, state :: call
                let mut spec = text_append_spec :: paragraph.collection, paragraph.paragraph_style, ch :: call
                spec.logical_index = state.logical_index
                let family_name = preferred_family_name :: run.style, paragraph.collection :: call
                append_text_char :: state.line, run.style, spec :: call
                state.flattened :: ch :: push
                push_unique_string :: state.fonts_used, family_name :: call
                state.logical_index += 1
                repeats += 1
            continue
        if is_newline :: raw :: call:
            finalize_line :: paragraph.paragraph_style, state :: call
            continue
        let advance = glyph_advance :: run.style, raw :: call
        if layout_needs_wrap :: state, advance :: call:
            finalize_line :: paragraph.paragraph_style, state :: call
        let mut spec = text_append_spec :: paragraph.collection, paragraph.paragraph_style, raw :: call
        spec.logical_index = state.logical_index
        let family_name = preferred_family_name :: run.style, paragraph.collection :: call
        let unresolved = not (is_renderable :: raw :: call)
        append_text_char :: state.line, run.style, spec :: call
        state.flattened :: raw :: push
        push_unique_string :: state.fonts_used, family_name :: call
        if unresolved:
            let unresolved_index = state.logical_index
            state.unresolved :: unresolved_index :: push
        state.logical_index += 1

fn process_placeholder(edit state: arcana_text.provider_impl.engine.LayoutBuildState, read paragraph: arcana_text.provider_impl.engine.ParagraphState, read placeholder: arcana_text.types.PlaceholderStyle):
    let atom_width = max_int :: placeholder.size.0, 1 :: call
    if layout_needs_wrap :: state, atom_width :: call:
        finalize_line :: paragraph.paragraph_style, state :: call
    let mut spec = placeholder_append_spec :: paragraph.collection, paragraph.paragraph_style, placeholder :: call
    spec.logical_index = state.logical_index
    let family_name = preferred_collection_name :: paragraph.collection :: call
    append_placeholder_atom :: state.line, spec :: call
    state.flattened :: "\u{FFFC}" :: push
    push_unique_string :: state.fonts_used, family_name :: call
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

    let exceeded = truncate_to_max_lines :: paragraph.paragraph_style, state :: call
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

fn atom_contains(read atom: arcana_text.provider_impl.engine.LayoutAtom, x: Int, y: Int) -> Bool:
    return x >= atom.position.0 and x < atom.position.0 + atom.size.0 and y >= atom.position.1 and y < atom.position.1 + atom.size.1

fn glyph_pixel_on(read atom: arcana_text.provider_impl.engine.LayoutAtom, x: Int, y: Int) -> Bool:
    if atom.placeholder:
        return true
    if atom.text == " ":
        return false
    if atom.size.0 <= 2 or atom.size.1 <= 2:
        return true
    let inner_x = x - atom.position.0
    let inner_y = y - atom.position.1
    let w = atom.size.0
    let h = atom.size.1
    if inner_x <= 0 or inner_x >= w - 1 or inner_y <= 0 or inner_y >= h - 1:
        return false
    if atom.unresolved:
        return inner_x == inner_y or inner_x == (w - inner_y - 1)
    return true

fn pixel_color(read layout: arcana_text.provider_impl.engine.LayoutState, x: Int, y: Int) -> (Int, Int):
    let mut color = 0
    let mut alpha = 0
    for atom in layout.atoms:
        if not (atom_contains :: atom, x, y :: call):
            continue
        if atom.background_enabled:
            color = atom.background
            alpha = 255
        if glyph_pixel_on :: atom, x, y :: call:
            color = atom.foreground
            alpha = 255
    return (color, alpha)

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
    let width = max_int :: layout.width, 1 :: call
    let height = max_int :: layout.height, 1 :: call
    let mut rgba = std.collections.list.new[Int] :: :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let pixel = pixel_color :: layout, x, y :: call
            rgba :: (color_red :: pixel.0 :: call) :: push
            rgba :: (color_green :: pixel.0 :: call) :: push
            rgba :: (color_blue :: pixel.0 :: call) :: push
            rgba :: pixel.1 :: push
            x += 1
        y += 1
    let mut image = std.canvas.image_create :: width, height :: call
    std.canvas.image_replace_rgba :: image, (std.collections.array.from_list[Int] :: rgba :: call) :: call
    std.canvas.blit :: win, image, origin.0 :: call
        y = origin.1
