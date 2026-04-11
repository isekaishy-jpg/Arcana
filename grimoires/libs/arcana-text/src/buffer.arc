import arcana_text.types
import std.collections.list
import std.text

export obj TextBuffer:
    text: Str
    style: arcana_text.types.TextStyle
    paragraph: arcana_text.types.ParagraphStyle
    spans: List[arcana_text.types.TextSpan]
    placeholders: List[arcana_text.types.PlaceholderSpec]
    tab_width: Int
    version: Int
    dirty_start: Int
    dirty_end: Int

export fn clamp_offset(value: Int, upper: Int) -> Int:
    if value < 0:
        return 0
    if value > upper:
        return upper
    return value

export fn normalize_range(start: Int, end: Int, upper: Int) -> arcana_text.types.TextRange:
    let mut a = arcana_text.buffer.clamp_offset :: start, upper :: call
    let mut b = arcana_text.buffer.clamp_offset :: end, upper :: call
    if a > b:
        let swap = a
        a = b
        b = swap
    return arcana_text.types.TextRange :: start = a, end = b :: call

fn replace_delta(read range: arcana_text.types.TextRange, inserted_len: Int) -> Int:
    return inserted_len - (range.end - range.start)

fn remap_span_start(offset: Int, read range: arcana_text.types.TextRange, inserted_len: Int) -> Int:
    if offset < range.start:
        return offset
    if offset >= range.end:
        return offset + (arcana_text.buffer.replace_delta :: range, inserted_len :: call)
    return range.start

fn remap_span_end(offset: Int, read range: arcana_text.types.TextRange, inserted_len: Int) -> Int:
    if offset <= range.start:
        return offset
    if offset > range.end:
        return offset + (arcana_text.buffer.replace_delta :: range, inserted_len :: call)
    return range.start + inserted_len

fn copy_placeholder(read spec: arcana_text.types.PlaceholderSpec) -> arcana_text.types.PlaceholderSpec:
    return spec

fn shifted_placeholder(read spec: arcana_text.types.PlaceholderSpec, delta: Int) -> arcana_text.types.PlaceholderSpec:
    let mut next = spec
    next.range = arcana_text.types.TextRange :: start = spec.range.start + delta, end = spec.range.end + delta :: call
    return next

fn placeholder_overlaps(read left: arcana_text.types.TextRange, read right: arcana_text.types.TextRange) -> Bool:
    return left.start < right.end and right.start < left.end

fn is_indent_byte(b: Int) -> Bool:
    return b == 32 or b == 9

fn line_start_in_text(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    let mut cursor = arcana_text.buffer.clamp_offset :: offset, total :: call
    while cursor > 0:
        let prior = std.text.byte_at :: text, cursor - 1 :: call
        if prior == 10:
            break
        cursor -= 1
    return cursor

fn line_end_in_text(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    let mut cursor = arcana_text.buffer.clamp_offset :: offset, total :: call
    while cursor < total:
        if (std.text.byte_at :: text, cursor :: call) == 10:
            break
        cursor += 1
    return cursor

fn line_range_in_text(read text: Str, offset: Int) -> arcana_text.types.TextRange:
    let start = arcana_text.buffer.line_start_in_text :: text, offset :: call
    let end = arcana_text.buffer.line_end_in_text :: text, offset :: call
    return arcana_text.types.TextRange :: start = start, end = end :: call

fn line_indentation_in_text(read text: Str, offset: Int) -> Str:
    let range = arcana_text.buffer.line_range_in_text :: text, offset :: call
    let mut cursor = range.start
    while cursor < range.end:
        let value = std.text.byte_at :: text, cursor :: call
        if not (arcana_text.buffer.is_indent_byte :: value :: call):
            break
        cursor += 1
    return std.text.slice_bytes :: text, range.start, cursor :: call

fn line_index_in_text(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    let upper = arcana_text.buffer.clamp_offset :: offset, total :: call
    let mut index = 0
    let mut cursor = 0
    while cursor < upper:
        if (std.text.byte_at :: text, cursor :: call) == 10:
            index += 1
        cursor += 1
    return index

fn adjust_spans_after_replace(edit self: arcana_text.buffer.TextBuffer, read range: arcana_text.types.TextRange, inserted_len: Int):
    let mut adjusted = std.collections.list.empty[arcana_text.types.TextSpan] :: :: call
    for span in self.spans:
        let start = arcana_text.buffer.remap_span_start :: span.range.start, range, inserted_len :: call
        let end = arcana_text.buffer.remap_span_end :: span.range.end, range, inserted_len :: call
        if end > start:
            let mut next = span
            next.range = arcana_text.types.TextRange :: start = start, end = end :: call
            adjusted :: next :: push
    if adjusted :: :: is_empty:
        let len = std.text.len_bytes :: self.text :: call
        let style = self.style
        adjusted :: (arcana_text.types.TextSpan :: range = (arcana_text.types.TextRange :: start = 0, end = len :: call), style = (arcana_text.buffer.span_style_from_text_style :: style :: call) :: call) :: push
    self.spans = adjusted

fn adjust_placeholders_after_replace(edit self: arcana_text.buffer.TextBuffer, read range: arcana_text.types.TextRange, inserted_len: Int):
    let delta = arcana_text.buffer.replace_delta :: range, inserted_len :: call
    let mut adjusted = std.collections.list.empty[arcana_text.types.PlaceholderSpec] :: :: call
    for spec in self.placeholders:
        if spec.range.end <= range.start:
            adjusted :: (arcana_text.buffer.copy_placeholder :: spec :: call) :: push
        else:
            if spec.range.start >= range.end:
                adjusted :: (arcana_text.buffer.shifted_placeholder :: spec, delta :: call) :: push
    self.placeholders = adjusted

fn span_style_from_text_style(read style: arcana_text.types.TextStyle) -> arcana_text.types.SpanStyle:
    let mut span_style = arcana_text.types.SpanStyle :: color = style.color, background_enabled = style.background_enabled, background_color = style.background_color :: call
    span_style.underline = style.underline
    span_style.underline_color_enabled = style.underline_color_enabled
    span_style.underline_color = style.underline_color
    span_style.strikethrough_enabled = style.strikethrough_enabled
    span_style.strikethrough_color_enabled = style.strikethrough_color_enabled
    span_style.strikethrough_color = style.strikethrough_color
    span_style.overline_enabled = style.overline_enabled
    span_style.overline_color_enabled = style.overline_color_enabled
    span_style.overline_color = style.overline_color
    span_style.size = style.size
    span_style.letter_spacing = style.letter_spacing
    span_style.line_height = style.line_height
    span_style.families = style.families
    span_style.features = style.features
    span_style.axes = style.axes
    return span_style

export fn open(text: Str, read style: arcana_text.types.TextStyle, read paragraph: arcana_text.types.ParagraphStyle) -> arcana_text.buffer.TextBuffer:
    let len = std.text.len_bytes :: text :: call
    let mut buffer = arcana_text.buffer.TextBuffer :: text = text, style = style, paragraph = paragraph :: call
    buffer.spans = std.collections.list.new[arcana_text.types.TextSpan] :: :: call
    buffer.spans :: (arcana_text.types.TextSpan :: range = (arcana_text.types.TextRange :: start = 0, end = len :: call), style = (arcana_text.buffer.span_style_from_text_style :: style :: call) :: call) :: push
    buffer.placeholders = std.collections.list.new[arcana_text.types.PlaceholderSpec] :: :: call
    buffer.tab_width = 8
    buffer.version = 1
    buffer.dirty_start = 0
    buffer.dirty_end = len
    return buffer

impl TextBuffer:
    fn len_bytes(read self: arcana_text.buffer.TextBuffer) -> Int:
        return std.text.len_bytes :: self.text :: call

    fn tab_width(read self: arcana_text.buffer.TextBuffer) -> Int:
        return self.tab_width

    fn set_tab_width(edit self: arcana_text.buffer.TextBuffer, tab_width: Int):
        if tab_width <= 0 or tab_width == self.tab_width:
            return
        self.tab_width = tab_width
        let len = std.text.len_bytes :: self.text :: call
        self :: 0, len :: mark_dirty

    fn line_start(read self: arcana_text.buffer.TextBuffer, offset: Int) -> Int:
        return arcana_text.buffer.line_start_in_text :: self.text, offset :: call

    fn line_end(read self: arcana_text.buffer.TextBuffer, offset: Int) -> Int:
        return arcana_text.buffer.line_end_in_text :: self.text, offset :: call

    fn line_range(read self: arcana_text.buffer.TextBuffer, offset: Int) -> arcana_text.types.TextRange:
        return arcana_text.buffer.line_range_in_text :: self.text, offset :: call

    fn line_text(read self: arcana_text.buffer.TextBuffer, offset: Int) -> Str:
        let range = self :: offset :: line_range
        return std.text.slice_bytes :: self.text, range.start, range.end :: call

    fn line_indentation(read self: arcana_text.buffer.TextBuffer, offset: Int) -> Str:
        return arcana_text.buffer.line_indentation_in_text :: self.text, offset :: call

    fn line_index(read self: arcana_text.buffer.TextBuffer, offset: Int) -> Int:
        return arcana_text.buffer.line_index_in_text :: self.text, offset :: call

    fn mark_dirty(edit self: arcana_text.buffer.TextBuffer, start: Int, end: Int):
        let upper = self :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: start, end, upper :: call
        self.version += 1
        self.dirty_start = range.start
        self.dirty_end = range.end

    fn set_text(edit self: arcana_text.buffer.TextBuffer, text: Str):
        self.text = text
        let len = std.text.len_bytes :: self.text :: call
        let style = self.style
        self.spans = std.collections.list.empty[arcana_text.types.TextSpan] :: :: call
        self.spans :: (arcana_text.types.TextSpan :: range = (arcana_text.types.TextRange :: start = 0, end = len :: call), style = (arcana_text.buffer.span_style_from_text_style :: style :: call) :: call) :: push
        self.placeholders = std.collections.list.empty[arcana_text.types.PlaceholderSpec] :: :: call
        self :: 0, len :: mark_dirty

    fn insert(edit self: arcana_text.buffer.TextBuffer, index: Int, read chunk: Str):
        let total = self :: :: len_bytes
        let offset = arcana_text.buffer.clamp_offset :: index, total :: call
        let prefix = std.text.slice_bytes :: self.text, 0, offset :: call
        let suffix = std.text.slice_bytes :: self.text, offset, total :: call
        self.text = prefix + chunk + suffix
        let edit_range = arcana_text.types.TextRange :: start = offset, end = offset :: call
        let inserted_len = std.text.len_bytes :: chunk :: call
        arcana_text.buffer.adjust_spans_after_replace :: self, edit_range, inserted_len :: call
        arcana_text.buffer.adjust_placeholders_after_replace :: self, edit_range, inserted_len :: call
        self :: offset, (offset + (std.text.len_bytes :: chunk :: call)) :: mark_dirty

    fn replace_range(edit self: arcana_text.buffer.TextBuffer, read range_payload: arcana_text.types.TextRange, read chunk: Str):
        let total = self :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: range_payload.start, range_payload.end, total :: call
        let prefix = std.text.slice_bytes :: self.text, 0, range.start :: call
        let suffix = std.text.slice_bytes :: self.text, range.end, total :: call
        self.text = prefix + chunk + suffix
        let inserted_len = std.text.len_bytes :: chunk :: call
        arcana_text.buffer.adjust_spans_after_replace :: self, range, inserted_len :: call
        arcana_text.buffer.adjust_placeholders_after_replace :: self, range, inserted_len :: call
        self :: range.start, (range.start + (std.text.len_bytes :: chunk :: call)) :: mark_dirty

    fn delete_range(edit self: arcana_text.buffer.TextBuffer, start: Int, end: Int):
        let range = arcana_text.types.TextRange :: start = start, end = end :: call
        self :: range, "" :: replace_range

    fn copy_range(read self: arcana_text.buffer.TextBuffer, start: Int, end: Int) -> Str:
        let total = self :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: start, end, total :: call
        return std.text.slice_bytes :: self.text, range.start, range.end :: call

    fn set_style(edit self: arcana_text.buffer.TextBuffer, read style: arcana_text.types.TextStyle):
        self.style = style
        let len = std.text.len_bytes :: self.text :: call
        self.spans = std.collections.list.empty[arcana_text.types.TextSpan] :: :: call
        self.spans :: (arcana_text.types.TextSpan :: range = (arcana_text.types.TextRange :: start = 0, end = len :: call), style = (arcana_text.buffer.span_style_from_text_style :: style :: call) :: call) :: push
        self :: 0, len :: mark_dirty

    fn set_paragraph_style(edit self: arcana_text.buffer.TextBuffer, read paragraph: arcana_text.types.ParagraphStyle):
        self.paragraph = paragraph
        let len = std.text.len_bytes :: self.text :: call
        self :: 0, len :: mark_dirty

    fn add_placeholder(edit self: arcana_text.buffer.TextBuffer, read spec: arcana_text.types.PlaceholderSpec):
        let range = spec.range
        self.placeholders :: spec :: push
        self :: range.start, range.end :: mark_dirty

    fn clear_placeholders(edit self: arcana_text.buffer.TextBuffer):
        if self.placeholders :: :: is_empty:
            return
        self.placeholders = std.collections.list.empty[arcana_text.types.PlaceholderSpec] :: :: call
        let len = self :: :: len_bytes
        self :: 0, len :: mark_dirty

    fn remove_placeholders(edit self: arcana_text.buffer.TextBuffer, start: Int, end: Int):
        let total = self :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: start, end, total :: call
        let mut kept = std.collections.list.empty[arcana_text.types.PlaceholderSpec] :: :: call
        let mut removed = false
        for spec in self.placeholders:
            if arcana_text.buffer.placeholder_overlaps :: spec.range, range :: call:
                removed = true
            else:
                kept :: spec :: push
        if not removed:
            return
        self.placeholders = kept
        self :: range.start, range.end :: mark_dirty

    fn set_spans(edit self: arcana_text.buffer.TextBuffer, read spans: List[arcana_text.types.TextSpan]):
        self.spans = std.collections.list.empty[arcana_text.types.TextSpan] :: :: call
        self.spans :: spans :: extend_list
        let len = std.text.len_bytes :: self.text :: call
        self :: 0, len :: mark_dirty

    fn clear_spans(edit self: arcana_text.buffer.TextBuffer):
        let len = std.text.len_bytes :: self.text :: call
        let style = self.style
        self.spans = std.collections.list.empty[arcana_text.types.TextSpan] :: :: call
        self.spans :: (arcana_text.types.TextSpan :: range = (arcana_text.types.TextRange :: start = 0, end = len :: call), style = (arcana_text.buffer.span_style_from_text_style :: style :: call) :: call) :: push
        self :: 0, len :: mark_dirty

    fn add_span(edit self: arcana_text.buffer.TextBuffer, read span: arcana_text.types.TextSpan):
        self.spans :: span :: push
        self :: span.range.start, span.range.end :: mark_dirty

    fn set_span_style(edit self: arcana_text.buffer.TextBuffer, read payload: (arcana_text.types.TextRange, arcana_text.types.SpanStyle)):
        let total = self :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: payload.0.start, payload.0.end, total :: call
        let style = payload.1
        if range.end <= range.start:
            return
        self.spans :: (arcana_text.types.TextSpan :: range = range, style = style :: call) :: push
        self :: range.start, range.end :: mark_dirty
