import arcana_text.layout
import arcana_text.types
import std.collections.list

fn overlaps(read left: arcana_text.types.TextRange, read right: arcana_text.types.TextRange) -> Bool:
    return left.start < right.end and right.start < left.end

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
        if glyph.cluster_range.start == index or glyph.range.start == index:
            return arcana_text.types.CaretBox :: index = index, position = glyph.position, size = (1, glyph.size.1) :: call
    if (snapshot.glyphs :: :: len) > 0:
        let last = snapshot.glyphs[(snapshot.glyphs :: :: len) - 1]
        return arcana_text.types.CaretBox :: index = index, position = (last.position.0 + last.size.0, last.position.1), size = (1, last.size.1) :: call
    return arcana_text.types.CaretBox :: index = index, position = (0, 0), size = (1, 0) :: call

export fn hit_test(read snapshot: arcana_text.layout.LayoutSnapshot, point: (Int, Int)) -> arcana_text.types.HitTest:
    for glyph in snapshot.glyphs:
        if point.0 >= glyph.position.0 and point.0 < (glyph.position.0 + glyph.size.0) and point.1 >= glyph.position.1 and point.1 < (glyph.position.1 + glyph.size.1):
            let mut hit = arcana_text.types.HitTest :: index = glyph.cluster_range.start, line_index = glyph.line_index, position = glyph.position :: call
            hit.size = glyph.size
            return hit
    let mut hit = arcana_text.types.HitTest :: index = 0, line_index = 0, position = (0, 0) :: call
    hit.size = (0, 0)
    return hit

export fn word_boundary(read snapshot: arcana_text.layout.LayoutSnapshot, index: Int) -> arcana_text.types.TextRange:
    let mut start = index
    let mut end = index
    for glyph in snapshot.glyphs:
        if glyph.range.end <= index and glyph.glyph != " ":
            start = glyph.range.start
        if glyph.range.start >= index and glyph.glyph == " ":
            end = glyph.range.start
            return arcana_text.types.TextRange :: start = start, end = end :: call
    if end < start:
        end = start
    return arcana_text.types.TextRange :: start = start, end = end :: call

export fn fonts_used(read snapshot: arcana_text.layout.LayoutSnapshot) -> List[arcana_text.types.FontMatch]:
    return snapshot.fonts_used

export fn unresolved_glyphs(read snapshot: arcana_text.layout.LayoutSnapshot) -> List[arcana_text.types.UnresolvedGlyph]:
    return snapshot.unresolved
