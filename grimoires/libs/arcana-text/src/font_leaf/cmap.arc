import arcana_text.font_leaf

export fn glyph_index_for_codepoint(edit face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    return arcana_text.font_leaf.glyph_index_for_codepoint :: face, codepoint :: call

export fn supports_text(edit face: arcana_text.font_leaf.FontFaceState, read ch: Str) -> Bool:
    return arcana_text.font_leaf.supports_text :: face, ch :: call
