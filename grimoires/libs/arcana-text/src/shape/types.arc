import arcana_text.types
import std.bytes
import std.collections.list

export obj ShapeSnapshot:
    source_version: Int
    default_style: arcana_text.types.SpanStyle
    paragraph: arcana_text.types.ParagraphStyle
    signature: Int
    default_line_height: Int
    default_baseline: Int
    runs: List[arcana_text.types.ShapedRun]
    plan_keys: List[arcana_text.types.ShapePlanKey]
    unresolved: List[arcana_text.types.UnresolvedGlyph]
    fonts_used: List[arcana_text.types.FontMatch]

export fn max_int(a: Int, b: Int) -> Int:
    if a >= b:
        return a
    return b

export fn mix_signature(seed: Int, value: Int) -> Int:
    let modulus = 2147483629
    let mut next = ((seed * 131) + value + 59) % modulus
    if next < 0:
        next += modulus
    return next

export fn mix_signature_text(seed: Int, read text: Str) -> Int:
    let bytes = std.bytes.from_str_utf8 :: text :: call
    let total = std.bytes.len :: bytes :: call
    let mut next = seed
    let mut index = 0
    while index < total:
        next = arcana_text.shape.types.mix_signature :: next, (std.bytes.at :: bytes, index :: call) :: call
        index += 1
    return next

export fn empty_runs() -> List[arcana_text.types.ShapedRun]:
    return std.collections.list.empty[arcana_text.types.ShapedRun] :: :: call

export fn empty_unresolved() -> List[arcana_text.types.UnresolvedGlyph]:
    return std.collections.list.empty[arcana_text.types.UnresolvedGlyph] :: :: call

export fn empty_matches() -> List[arcana_text.types.FontMatch]:
    return std.collections.list.empty[arcana_text.types.FontMatch] :: :: call

export fn empty_plan_keys() -> List[arcana_text.types.ShapePlanKey]:
    return std.collections.list.empty[arcana_text.types.ShapePlanKey] :: :: call

export fn fallback_placeholder(read range: arcana_text.types.TextRange) -> arcana_text.types.PlaceholderSpec:
    let mut spec = arcana_text.types.PlaceholderSpec :: range = range, size = (0, 0), alignment = (arcana_text.types.PlaceholderAlignment.Baseline :: :: call) :: call
    spec.baseline = arcana_text.types.TextBaseline.Alphabetic :: :: call
    spec.baseline_offset = 0
    return spec

export fn push_unique_match(edit out: List[arcana_text.types.FontMatch], read value: arcana_text.types.FontMatch):
    if value.id.source_index < 0:
        return
    for existing in out:
        if existing.id.source_index == value.id.source_index and existing.id.face_index == value.id.face_index:
            return
    out :: value :: push

export fn push_unique_plan_key(edit out: List[arcana_text.types.ShapePlanKey], read value: arcana_text.types.ShapePlanKey):
    if value.face_id.source_index < 0:
        return
    for existing in out:
        if existing.face_id.source_index == value.face_id.source_index and existing.face_id.face_index == value.face_id.face_index and existing.direction == value.direction and existing.script == value.script and existing.language_tag == value.language_tag and existing.font_size == value.font_size and existing.weight == value.weight and existing.width_milli == value.width_milli and existing.slant_milli == value.slant_milli and existing.feature_signature == value.feature_signature and existing.axis_signature == value.axis_signature:
            return
    out :: value :: push
