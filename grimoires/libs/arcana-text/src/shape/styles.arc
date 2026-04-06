import arcana_text.buffer
import arcana_text.types

export fn span_style_from_text_style(read style: arcana_text.types.TextStyle) -> arcana_text.types.SpanStyle:
    let mut span = arcana_text.types.SpanStyle :: color = style.color, background_enabled = style.background_enabled, background_color = style.background_color :: call
    span.size = style.size
    span.letter_spacing = style.letter_spacing
    span.line_height = style.line_height
    span.families = style.families
    span.features = style.features
    span.axes = style.axes
    return span

export fn text_style_from_span(read style: arcana_text.types.SpanStyle) -> arcana_text.types.TextStyle:
    let mut text_style = arcana_text.types.TextStyle :: color = style.color, background_enabled = style.background_enabled, background_color = style.background_color :: call
    text_style.size = style.size
    text_style.letter_spacing = style.letter_spacing
    text_style.line_height = style.line_height
    text_style.families = style.families
    text_style.features = style.features
    text_style.axes = style.axes
    return text_style

fn overlaps(read left: arcana_text.types.TextRange, read right: arcana_text.types.TextRange) -> Bool:
    return left.start < right.end and right.start < left.end

export fn style_for_range(read buffer: arcana_text.buffer.TextBuffer, read range: arcana_text.types.TextRange) -> arcana_text.types.SpanStyle:
    for span in buffer.spans:
        if overlaps :: span.range, range :: call:
            return span.style
    return arcana_text.shape.styles.span_style_from_text_style :: buffer.style :: call
