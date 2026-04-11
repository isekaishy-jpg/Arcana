import arcana_text.buffer
import arcana_text.types

export fn span_style_from_text_style(read style: arcana_text.types.TextStyle) -> arcana_text.types.SpanStyle:
    let mut span = arcana_text.types.SpanStyle :: color = style.color, background_enabled = style.background_enabled, background_color = style.background_color :: call
    span.underline = style.underline
    span.underline_color_enabled = style.underline_color_enabled
    span.underline_color = style.underline_color
    span.strikethrough_enabled = style.strikethrough_enabled
    span.strikethrough_color_enabled = style.strikethrough_color_enabled
    span.strikethrough_color = style.strikethrough_color
    span.overline_enabled = style.overline_enabled
    span.overline_color_enabled = style.overline_color_enabled
    span.overline_color = style.overline_color
    span.size = style.size
    span.letter_spacing = style.letter_spacing
    span.line_height = style.line_height
    span.families = style.families
    span.features = style.features
    span.axes = style.axes
    return span

export fn text_style_from_span(read style: arcana_text.types.SpanStyle) -> arcana_text.types.TextStyle:
    let mut text_style = arcana_text.types.TextStyle :: color = style.color, background_enabled = style.background_enabled, background_color = style.background_color :: call
    text_style.underline = style.underline
    text_style.underline_color_enabled = style.underline_color_enabled
    text_style.underline_color = style.underline_color
    text_style.strikethrough_enabled = style.strikethrough_enabled
    text_style.strikethrough_color_enabled = style.strikethrough_color_enabled
    text_style.strikethrough_color = style.strikethrough_color
    text_style.overline_enabled = style.overline_enabled
    text_style.overline_color_enabled = style.overline_color_enabled
    text_style.overline_color = style.overline_color
    text_style.size = style.size
    text_style.letter_spacing = style.letter_spacing
    text_style.line_height = style.line_height
    text_style.families = style.families
    text_style.features = style.features
    text_style.axes = style.axes
    return text_style

fn overlaps(read left: arcana_text.types.TextRange, read right: arcana_text.types.TextRange) -> Bool:
    return left.start < right.end and right.start < left.end

fn covers(read outer: arcana_text.types.TextRange, read inner: arcana_text.types.TextRange) -> Bool:
    return outer.start <= inner.start and inner.end <= outer.end

export fn style_for_range(read buffer: arcana_text.buffer.TextBuffer, read range: arcana_text.types.TextRange) -> arcana_text.types.SpanStyle:
    let fallback = arcana_text.shape.styles.span_style_from_text_style :: buffer.style :: call
    let mut found_cover = false
    let mut best_cover = fallback
    let mut best_cover_size = 2147483647
    for span in buffer.spans:
        if covers :: span.range, range :: call:
            let span_size = span.range.end - span.range.start
            if not found_cover or span_size < best_cover_size:
                found_cover = true
                best_cover = span.style
                best_cover_size = span_size
    if found_cover:
        return best_cover
    for span in buffer.spans:
        if overlaps :: span.range, range :: call:
            return span.style
    return fallback
