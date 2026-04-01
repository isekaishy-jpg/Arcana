import arcana_text.provider_impl.engine
import std.window

fn layout(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, width: Int):
    arcana_text.provider_impl.engine.layout_paragraph :: paragraph, width :: call

fn paint(edit win: std.window.Window, read paragraph: arcana_text.provider_impl.engine.ParagraphState, pos: (Int, Int)):
    arcana_text.provider_impl.engine.paint_paragraph :: win, paragraph, pos :: call

fn longest_line(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return arcana_text.provider_impl.engine.longest_line :: paragraph :: call

fn height(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return arcana_text.provider_impl.engine.height :: paragraph :: call

fn max_intrinsic_width(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return arcana_text.provider_impl.engine.max_intrinsic_width :: paragraph :: call

fn min_intrinsic_width(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return arcana_text.provider_impl.engine.min_intrinsic_width :: paragraph :: call

fn alphabetic_baseline(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return arcana_text.provider_impl.engine.alphabetic_baseline :: paragraph :: call

fn ideographic_baseline(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Int:
    return arcana_text.provider_impl.engine.ideographic_baseline :: paragraph :: call

fn exceeded_max_lines(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> Bool:
    return arcana_text.provider_impl.engine.exceeded_max_lines :: paragraph :: call

fn line_metrics(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[arcana_text.types.LineMetrics]:
    return arcana_text.provider_impl.engine.line_metrics_list :: paragraph :: call

fn range_boxes(read paragraph: arcana_text.provider_impl.engine.ParagraphState, read range: arcana_text.types.TextRange) -> List[arcana_text.types.TextBox]:
    return arcana_text.provider_impl.engine.range_boxes_list :: paragraph, range.start, range.end :: call

fn placeholder_boxes(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[arcana_text.types.TextBox]:
    return arcana_text.provider_impl.engine.placeholder_boxes_list :: paragraph :: call

fn position_at(read paragraph: arcana_text.provider_impl.engine.ParagraphState, pos: (Int, Int)) -> arcana_text.types.PositionWithAffinity:
    return arcana_text.provider_impl.engine.position_at_value :: paragraph, pos :: call

fn word_boundary(read paragraph: arcana_text.provider_impl.engine.ParagraphState, index: Int) -> arcana_text.types.TextRange:
    return arcana_text.provider_impl.engine.word_boundary_value :: paragraph, index :: call

fn unresolved_glyphs(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[Int]:
    return arcana_text.provider_impl.engine.unresolved_glyphs_list :: paragraph :: call

fn fonts_used(read paragraph: arcana_text.provider_impl.engine.ParagraphState) -> List[Str]:
    return arcana_text.provider_impl.engine.fonts_used_list :: paragraph :: call

fn update_text(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, text: Str):
    arcana_text.provider_impl.engine.update_text_value :: paragraph, text :: call

fn update_align(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read align: arcana_text.types.TextAlign):
    arcana_text.provider_impl.engine.update_align_value :: paragraph, align :: call

fn update_font_size(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, font_size: Int):
    arcana_text.provider_impl.engine.update_font_size_value :: paragraph, font_size :: call

fn update_foreground(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, read paint: arcana_graphics.types.Paint):
    arcana_text.provider_impl.engine.update_foreground_value :: paragraph, paint :: call

fn update_background(edit paragraph: arcana_text.provider_impl.engine.ParagraphState, enabled: Bool, read paint: arcana_graphics.types.Paint):
    arcana_text.provider_impl.engine.update_background_value :: paragraph, enabled, paint :: call
