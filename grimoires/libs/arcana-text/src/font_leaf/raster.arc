import arcana_text.font_leaf

export fn render_glyph(edit face: arcana_text.font_leaf.FontFaceState, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    return arcana_text.font_leaf.render_glyph :: face, spec :: call

export fn measure_glyph(edit face: arcana_text.font_leaf.FontFaceState, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    return arcana_text.font_leaf.measure_glyph :: face, spec :: call

export fn line_height_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    return arcana_text.font_leaf.line_height_for_face :: face, font_size, line_height_milli :: call

export fn baseline_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    return arcana_text.font_leaf.baseline_for_face :: face, font_size, line_height_milli :: call
