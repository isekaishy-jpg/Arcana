import arcana_text.fonts
import arcana_text.layout
import arcana_text.text_units
import arcana_text.types
import std.collections.list
import std.option
use std.option.Option

fn overlaps(read left: arcana_text.types.TextRange, read right: arcana_text.types.TextRange) -> Bool:
    return left.start < right.end and right.start < left.end

fn abs_int(value: Int) -> Int:
    if value < 0:
        return 0 - value
    return value

fn empty_line_metrics() -> arcana_text.types.LineMetrics:
    let mut metrics = arcana_text.types.LineMetrics :: index = 0, range = (arcana_text.types.TextRange :: start = 0, end = 0 :: call), position = (0, 0) :: call
    metrics.size = (0, 0)
    metrics.baseline = 0
    return metrics

fn line_text_value(read snapshot: arcana_text.layout.LayoutSnapshot, line_index: Int) -> Str:
    for line in snapshot.lines:
        if line.metrics.index == line_index:
            return line.text
    return ""

fn glyph_is_vertical(read glyph: arcana_text.types.LayoutGlyph) -> Bool:
    return glyph.y_advance > 0 and glyph.x_advance == 0

fn line_is_vertical(read snapshot: arcana_text.layout.LayoutSnapshot, line_index: Int) -> Bool:
    for glyph in snapshot.glyphs:
        if glyph.line_index == line_index and (arcana_text.queries.glyph_is_vertical :: glyph :: call):
            return true
    return false

fn caret_distance(read glyph: arcana_text.types.LayoutGlyph, point: (Int, Int), read caret: arcana_text.types.CaretBox) -> Int:
    if arcana_text.queries.glyph_is_vertical :: glyph :: call:
        let axis = arcana_text.queries.abs_int :: (point.1 - caret.position.1) :: call
        let mut cross = 0
        if point.0 < glyph.position.0:
            cross = glyph.position.0 - point.0
        else:
            let right = glyph.position.0 + glyph.size.0
            if point.0 > right:
                cross = point.0 - right
        return (axis * 2) + cross
    let axis = arcana_text.queries.abs_int :: (point.0 - caret.position.0) :: call
    let mut cross = 0
    if point.1 < glyph.position.1:
        cross = glyph.position.1 - point.1
    else:
        let bottom = glyph.position.1 + glyph.size.1
        if point.1 > bottom:
            cross = point.1 - bottom
    return (axis * 2) + cross

fn caret_before(read glyph: arcana_text.types.LayoutGlyph) -> arcana_text.types.CaretBox:
    if arcana_text.queries.glyph_is_vertical :: glyph :: call:
        return arcana_text.types.CaretBox :: index = glyph.cluster_range.start, position = (glyph.position.0, glyph.position.1), size = (glyph.size.0, 1) :: call
    let caret_x = match glyph.direction:
        arcana_text.types.TextDirection.RightToLeft => glyph.position.0 + glyph.size.0
        _ => glyph.position.0
    return arcana_text.types.CaretBox :: index = glyph.cluster_range.start, position = (caret_x, glyph.position.1), size = (1, glyph.size.1) :: call

fn caret_after(read glyph: arcana_text.types.LayoutGlyph) -> arcana_text.types.CaretBox:
    if arcana_text.queries.glyph_is_vertical :: glyph :: call:
        return arcana_text.types.CaretBox :: index = glyph.cluster_range.end, position = (glyph.position.0, glyph.position.1 + glyph.size.1), size = (glyph.size.0, 1) :: call
    let caret_x = match glyph.direction:
        arcana_text.types.TextDirection.RightToLeft => glyph.position.0
        _ => glyph.position.0 + glyph.size.0
    return arcana_text.types.CaretBox :: index = glyph.cluster_range.end, position = (caret_x, glyph.position.1), size = (1, glyph.size.1) :: call

export fn line_count(read snapshot: arcana_text.layout.LayoutSnapshot) -> Int:
    return snapshot.lines :: :: len

export fn line_metrics_at(read snapshot: arcana_text.layout.LayoutSnapshot, line_index: Int) -> arcana_text.types.LineMetrics:
    for line in snapshot.lines:
        if line.metrics.index == line_index:
            return line.metrics
    if not (snapshot.lines :: :: is_empty):
        if line_index <= 0:
            return snapshot.lines[0].metrics
        return snapshot.lines[(snapshot.lines :: :: len) - 1].metrics
    return arcana_text.queries.empty_line_metrics :: :: call

export fn line_index_for_offset(read snapshot: arcana_text.layout.LayoutSnapshot, index: Int) -> Int:
    if snapshot.lines :: :: is_empty:
        return 0
    let mut last = snapshot.lines[0].metrics.index
    for line in snapshot.lines:
        last = line.metrics.index
        if index < line.metrics.range.start:
            return line.metrics.index
        if index >= line.metrics.range.start and index <= line.metrics.range.end:
            return line.metrics.index
    return last

export fn line_boundary(read snapshot: arcana_text.layout.LayoutSnapshot, index: Int) -> arcana_text.types.TextRange:
    let line_index = arcana_text.queries.line_index_for_offset :: snapshot, index :: call
    let metrics = arcana_text.queries.line_metrics_at :: snapshot, line_index :: call
    return metrics.range

export fn line_text(read snapshot: arcana_text.layout.LayoutSnapshot, index: Int) -> Str:
    let line_index = arcana_text.queries.line_index_for_offset :: snapshot, index :: call
    return arcana_text.queries.line_text_value :: snapshot, line_index :: call

export fn line_at_point(read snapshot: arcana_text.layout.LayoutSnapshot, point: (Int, Int)) -> arcana_text.types.LineMetrics:
    if snapshot.lines :: :: is_empty:
        return arcana_text.queries.empty_line_metrics :: :: call
    for line in snapshot.lines:
        let metrics = line.metrics
        if point.0 >= metrics.position.0 and point.0 < (metrics.position.0 + metrics.size.0) and point.1 >= metrics.position.1 and point.1 < (metrics.position.1 + metrics.size.1):
            return metrics
    let mut best = snapshot.lines[0].metrics
    let mut best_distance = 2147483647
    for line in snapshot.lines:
        let metrics = line.metrics
        let mut dx = 0
        if point.0 < metrics.position.0:
            dx = metrics.position.0 - point.0
        else:
            let right = metrics.position.0 + metrics.size.0
            if point.0 > right:
                dx = point.0 - right
        let mut dy = 0
        if point.1 < metrics.position.1:
            dy = metrics.position.1 - point.1
        else:
            let bottom = metrics.position.1 + metrics.size.1
            if point.1 > bottom:
                dy = point.1 - bottom
        let distance = dx + dy
        if distance < best_distance:
            best = metrics
            best_distance = distance
    return best

fn empty_caret(index: Int, read metrics: arcana_text.types.LineMetrics) -> arcana_text.types.CaretBox:
    return arcana_text.types.CaretBox :: index = index, position = metrics.position, size = (1, metrics.size.1) :: call

fn fallback_caret_for_line(read snapshot: arcana_text.layout.LayoutSnapshot, read metrics: arcana_text.types.LineMetrics, index: Int) -> arcana_text.types.CaretBox:
    let mut first = Option.None[arcana_text.types.CaretBox] :: :: call
    let mut last = Option.None[arcana_text.types.CaretBox] :: :: call
    for glyph in snapshot.glyphs:
        if glyph.line_index != metrics.index:
            continue
        if glyph.caret_stop_before:
            let caret = arcana_text.queries.caret_before :: glyph :: call
            if first :: :: is_none:
                first = Option.Some[arcana_text.types.CaretBox] :: caret :: call
        if glyph.caret_stop_after:
            let caret = arcana_text.queries.caret_after :: glyph :: call
            last = Option.Some[arcana_text.types.CaretBox] :: caret :: call
    if first :: :: is_some and index <= metrics.range.start:
        return first :: (arcana_text.queries.empty_caret :: metrics.range.start, metrics :: call) :: unwrap_or
    if last :: :: is_some and index >= metrics.range.end:
        return last :: (arcana_text.queries.empty_caret :: metrics.range.end, metrics :: call) :: unwrap_or
    if arcana_text.queries.line_is_vertical :: snapshot, metrics.index :: call:
        return arcana_text.types.CaretBox :: index = index, position = metrics.position, size = (metrics.size.0, 1) :: call
    return arcana_text.queries.empty_caret :: index, metrics :: call

export fn line_metrics(read snapshot: arcana_text.layout.LayoutSnapshot) -> List[arcana_text.types.LineMetrics]:
    let mut out = std.collections.list.new[arcana_text.types.LineMetrics] :: :: call
    for line in snapshot.lines:
        out :: line.metrics :: push
    return out

export fn range_boxes(read snapshot: arcana_text.layout.LayoutSnapshot, read range: arcana_text.types.TextRange) -> List[arcana_text.types.RangeBox]:
    let mut out = std.collections.list.new[arcana_text.types.RangeBox] :: :: call
    for glyph in snapshot.glyphs:
        if arcana_text.queries.overlaps :: glyph.range, range :: call:
            let mut box = arcana_text.types.RangeBox :: position = glyph.position, size = glyph.size, range = glyph.range :: call
            box.direction = glyph.direction
            out :: box :: push
    return out

export fn caret_box(read snapshot: arcana_text.layout.LayoutSnapshot, index: Int) -> arcana_text.types.CaretBox:
    for glyph in snapshot.glyphs:
        if glyph.caret_stop_before and (glyph.cluster_range.start == index or glyph.range.start == index):
            return arcana_text.queries.caret_before :: glyph :: call
        if glyph.caret_stop_after and (glyph.cluster_range.end == index or glyph.range.end == index):
            return arcana_text.queries.caret_after :: glyph :: call
    let line_index = arcana_text.queries.line_index_for_offset :: snapshot, index :: call
    let metrics = arcana_text.queries.line_metrics_at :: snapshot, line_index :: call
    return arcana_text.queries.fallback_caret_for_line :: snapshot, metrics, index :: call

export fn hit_test(read snapshot: arcana_text.layout.LayoutSnapshot, point: (Int, Int)) -> arcana_text.types.HitTest:
    let line = arcana_text.queries.line_at_point :: snapshot, point :: call
    let mut best = Option.None[arcana_text.types.CaretBox] :: :: call
    let mut best_distance = 2147483647
    for glyph in snapshot.glyphs:
        if glyph.line_index != line.index:
            continue
        if glyph.caret_stop_before:
            let caret = arcana_text.queries.caret_before :: glyph :: call
            let distance = arcana_text.queries.caret_distance :: glyph, point, caret :: call
            if best :: :: is_none or distance < best_distance:
                best = Option.Some[arcana_text.types.CaretBox] :: caret :: call
                best_distance = distance
        if glyph.caret_stop_after:
            let caret = arcana_text.queries.caret_after :: glyph :: call
            let distance = arcana_text.queries.caret_distance :: glyph, point, caret :: call
            if best :: :: is_none or distance < best_distance:
                best = Option.Some[arcana_text.types.CaretBox] :: caret :: call
                best_distance = distance
    let caret = match best:
        Option.Some(value) => value
        Option.None => arcana_text.queries.fallback_caret_for_line :: snapshot, line, line.range.start :: call
    let mut hit = arcana_text.types.HitTest :: index = caret.index, line_index = line.index, position = caret.position :: call
    hit.size = caret.size
    return hit

export fn word_boundary(read snapshot: arcana_text.layout.LayoutSnapshot, index: Int) -> arcana_text.types.TextRange:
    let line_index = arcana_text.queries.line_index_for_offset :: snapshot, index :: call
    let metrics = arcana_text.queries.line_metrics_at :: snapshot, line_index :: call
    let text = arcana_text.queries.line_text_value :: snapshot, line_index :: call
    let local = arcana_text.text_units.clamp_offset :: text, (index - metrics.range.start) :: call
    let bounds = arcana_text.text_units.word_boundary :: text, local :: call
    return arcana_text.types.TextRange :: start = metrics.range.start + bounds.0, end = metrics.range.start + bounds.1 :: call

export fn fonts_used(read snapshot: arcana_text.layout.LayoutSnapshot) -> List[arcana_text.types.FontMatch]:
    return snapshot.fonts_used

export fn primary_font_name(read snapshot: arcana_text.layout.LayoutSnapshot) -> Str:
    if snapshot.fonts_used :: :: is_empty:
        return ""
    let source = snapshot.fonts_used[0].source
    let name = arcana_text.fonts.family_or_label :: source :: call
    if name != "":
        return name
    return source.path

export fn unresolved_glyphs(read snapshot: arcana_text.layout.LayoutSnapshot) -> List[arcana_text.types.UnresolvedGlyph]:
    return snapshot.unresolved
