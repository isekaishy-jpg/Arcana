import arcana_text.types
import std.collections.list
import std.text

export obj TextBuffer:
    text: Str
    style: arcana_text.types.TextStyle
    paragraph: arcana_text.types.ParagraphStyle
    spans: List[arcana_text.types.TextSpan]
    placeholders: List[arcana_text.types.PlaceholderSpec]
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

fn span_style_from_text_style(read style: arcana_text.types.TextStyle) -> arcana_text.types.SpanStyle:
    let mut span_style = arcana_text.types.SpanStyle :: color = style.color, background_enabled = style.background_enabled, background_color = style.background_color :: call
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
    buffer.version = 1
    buffer.dirty_start = 0
    buffer.dirty_end = len
    return buffer

impl TextBuffer:
    fn len_bytes(read self: arcana_text.buffer.TextBuffer) -> Int:
        return std.text.len_bytes :: self.text :: call

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
        self :: 0, len :: mark_dirty

    fn insert(edit self: arcana_text.buffer.TextBuffer, index: Int, read chunk: Str):
        let total = self :: :: len_bytes
        let offset = arcana_text.buffer.clamp_offset :: index, total :: call
        let prefix = std.text.slice_bytes :: self.text, 0, offset :: call
        let suffix = std.text.slice_bytes :: self.text, offset, total :: call
        self.text = prefix + chunk + suffix
        self :: offset, (offset + (std.text.len_bytes :: chunk :: call)) :: mark_dirty

    fn replace_range(edit self: arcana_text.buffer.TextBuffer, start: Int, end: Int, read chunk: Str):
        let total = self :: :: len_bytes
        let range = arcana_text.buffer.normalize_range :: start, end, total :: call
        let prefix = std.text.slice_bytes :: self.text, 0, range.start :: call
        let suffix = std.text.slice_bytes :: self.text, range.end, total :: call
        self.text = prefix + chunk + suffix
        self :: range.start, (range.start + (std.text.len_bytes :: chunk :: call)) :: mark_dirty

    fn delete_range(edit self: arcana_text.buffer.TextBuffer, start: Int, end: Int):
        self :: start, end, "" :: replace_range

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

    fn set_spans(edit self: arcana_text.buffer.TextBuffer, read spans: List[arcana_text.types.TextSpan]):
        self.spans = std.collections.list.empty[arcana_text.types.TextSpan] :: :: call
        self.spans :: spans :: extend_list
        let len = std.text.len_bytes :: self.text :: call
        self :: 0, len :: mark_dirty
