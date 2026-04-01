import arcana_text.provider_impl.paragraphs
import arcana_text.types
import std.window

export fn layout(edit paragraph: arcana_text.types.Paragraph, width: Int):
    arcana_text.provider_impl.paragraphs.layout :: paragraph, width :: call

export fn paint(edit win: std.window.Window, read paragraph: arcana_text.types.Paragraph, pos: (Int, Int)):
    arcana_text.provider_impl.paragraphs.paint :: win, paragraph, pos :: call

export fn longest_line(read paragraph: arcana_text.types.Paragraph) -> Int:
    return arcana_text.provider_impl.paragraphs.longest_line :: paragraph :: call

export fn height(read paragraph: arcana_text.types.Paragraph) -> Int:
    return arcana_text.provider_impl.paragraphs.height :: paragraph :: call

export fn max_intrinsic_width(read paragraph: arcana_text.types.Paragraph) -> Int:
    return arcana_text.provider_impl.paragraphs.max_intrinsic_width :: paragraph :: call

export fn min_intrinsic_width(read paragraph: arcana_text.types.Paragraph) -> Int:
    return arcana_text.provider_impl.paragraphs.min_intrinsic_width :: paragraph :: call

export fn alphabetic_baseline(read paragraph: arcana_text.types.Paragraph) -> Int:
    return arcana_text.provider_impl.paragraphs.alphabetic_baseline :: paragraph :: call

export fn ideographic_baseline(read paragraph: arcana_text.types.Paragraph) -> Int:
    return arcana_text.provider_impl.paragraphs.ideographic_baseline :: paragraph :: call

export fn exceeded_max_lines(read paragraph: arcana_text.types.Paragraph) -> Bool:
    return arcana_text.provider_impl.paragraphs.exceeded_max_lines :: paragraph :: call

export fn line_metrics(read paragraph: arcana_text.types.Paragraph) -> List[arcana_text.types.LineMetrics]:
    return arcana_text.provider_impl.paragraphs.line_metrics :: paragraph :: call

export fn range_boxes(read paragraph: arcana_text.types.Paragraph, read range: arcana_text.types.TextRange) -> List[arcana_text.types.TextBox]:
    return arcana_text.provider_impl.paragraphs.range_boxes :: paragraph, range :: call

export fn placeholder_boxes(read paragraph: arcana_text.types.Paragraph) -> List[arcana_text.types.TextBox]:
    return arcana_text.provider_impl.paragraphs.placeholder_boxes :: paragraph :: call

export fn position_at(read paragraph: arcana_text.types.Paragraph, pos: (Int, Int)) -> arcana_text.types.PositionWithAffinity:
    return arcana_text.provider_impl.paragraphs.position_at :: paragraph, pos :: call

export fn word_boundary(read paragraph: arcana_text.types.Paragraph, index: Int) -> arcana_text.types.TextRange:
    return arcana_text.provider_impl.paragraphs.word_boundary :: paragraph, index :: call

export fn unresolved_glyphs(read paragraph: arcana_text.types.Paragraph) -> List[Int]:
    return arcana_text.provider_impl.paragraphs.unresolved_glyphs :: paragraph :: call

export fn fonts_used(read paragraph: arcana_text.types.Paragraph) -> List[Str]:
    return arcana_text.provider_impl.paragraphs.fonts_used :: paragraph :: call

export fn update_text(edit paragraph: arcana_text.types.Paragraph, text: Str):
    arcana_text.provider_impl.paragraphs.update_text :: paragraph, text :: call

export fn update_align(edit paragraph: arcana_text.types.Paragraph, read align: arcana_text.types.TextAlign):
    arcana_text.provider_impl.paragraphs.update_align :: paragraph, align :: call

export fn update_font_size(edit paragraph: arcana_text.types.Paragraph, font_size: Int):
    arcana_text.provider_impl.paragraphs.update_font_size :: paragraph, font_size :: call

export fn update_foreground(edit paragraph: arcana_text.types.Paragraph, read paint: arcana_graphics.types.Paint):
    arcana_text.provider_impl.paragraphs.update_foreground :: paragraph, paint :: call

export fn update_background(edit paragraph: arcana_text.types.Paragraph, enabled: Bool, read paint: arcana_graphics.types.Paint):
    arcana_text.provider_impl.paragraphs.update_background :: paragraph, enabled, paint :: call
