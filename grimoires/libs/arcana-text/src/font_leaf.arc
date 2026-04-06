import std.bytes
import std.collections.array
import std.collections.list
import std.collections.map
import std.fs
import std.memory
import std.path
import std.result
import std.text
use std.result.Result

export record FaceTraits:
    weight: Int
    width_milli: Int
    slant_milli: Int

record Cmap4Segment:
    start_code: Int
    end_code: Int
    id_delta: Int
    glyph_base_index: Int
    direct_map: Bool

record Cmap12Group:
    start_code: Int
    end_code: Int
    start_glyph: Int

record FontPoint:
    x: Int
    y: Int
    on_curve: Bool

record FontContour:
    points: List[arcana_text.font_leaf.FontPoint]

record GlyphOutline:
    advance_width: Int
    left_side_bearing: Int
    x_min: Int
    y_min: Int
    x_max: Int
    y_max: Int
    contours: List[arcana_text.font_leaf.FontContour]
    empty: Bool

record LineSegment:
    start: (Int, Int)
    end: (Int, Int)

export record GlyphBitmap:
    size: (Int, Int)
    offset: (Int, Int)
    advance: Int
    baseline: Int
    line_height: Int
    empty: Bool
    alpha: Array[Int]

export record GlyphRenderSpec:
    text: Str
    glyph_index: Int
    font_size: Int
    line_height_milli: Int
    traits: arcana_text.font_leaf.FaceTraits
    feature_signature: Int
    axis_signature: Int

export record FontFaceState:
    family_name: Str
    source_label: Str
    source_path: Str
    weight: Int
    width_milli: Int
    slant_milli: Int
    units_per_em: Int
    ascender: Int
    descender: Int
    line_gap: Int
    glyph_count: Int
    font_view: std.memory.ByteView
    font_bytes: Array[Int]
    glyf_offset: Int
    loca_offset: Int
    loca_format: Int
    hmtx_offset: Int
    hmetric_count: Int
    advance_widths: Array[Int]
    left_side_bearings: Array[Int]
    loca_offsets: Array[Int]
    glyph_index_cache: Map[Int, Int]
    cmap4_offset: Int
    cmap12_offset: Int
    cmap4_segments: List[arcana_text.font_leaf.Cmap4Segment]
    cmap4_glyphs: Array[Int]
    cmap12_groups: List[arcana_text.font_leaf.Cmap12Group]
    bitmap_cache: Map[Str, arcana_text.font_leaf.GlyphBitmap]

record CmapState:
    segments: List[arcana_text.font_leaf.Cmap4Segment]
    glyphs: Array[Int]
    groups: List[arcana_text.font_leaf.Cmap12Group]

record AffineMatrix:
    xx: Int
    xy: Int
    yx: Int
    yy: Int

export record FaceLoadRequest:
    family_name: Str
    source_label: Str
    source_path: Str
    font_bytes: Array[Int]
    traits: arcana_text.font_leaf.FaceTraits

record FaceLoadMeta:
    family_name: Str
    source_label: Str
    source_path: Str
    traits: arcana_text.font_leaf.FaceTraits

record CoordinateDecodeSpec:
    bytes: Array[Int]
    cursor: Int
    flags: Array[Int]
    count: Int
    short_mask: Int
    same_mask: Int

record Cmap4Lookup:
    segments: List[arcana_text.font_leaf.Cmap4Segment]
    glyphs: Array[Int]
    codepoint: Int

record ScaleContext:
    face: arcana_text.font_leaf.FontFaceState
    traits: arcana_text.font_leaf.FaceTraits
    font_size: Int

record RasterKeyRequest:
    glyph_index: Int
    font_size: Int
    traits: arcana_text.font_leaf.FaceTraits
    feature_signature: Int
    axis_signature: Int

fn empty_points() -> List[arcana_text.font_leaf.FontPoint]:
    return std.collections.list.new[arcana_text.font_leaf.FontPoint] :: :: call

fn empty_contours() -> List[arcana_text.font_leaf.FontContour]:
    return std.collections.list.new[arcana_text.font_leaf.FontContour] :: :: call

fn empty_segments() -> List[arcana_text.font_leaf.LineSegment]:
    return std.collections.list.new[arcana_text.font_leaf.LineSegment] :: :: call

fn empty_cmap4_segments() -> List[arcana_text.font_leaf.Cmap4Segment]:
    return std.collections.list.new[arcana_text.font_leaf.Cmap4Segment] :: :: call

fn empty_cmap12_groups() -> List[arcana_text.font_leaf.Cmap12Group]:
    return std.collections.list.new[arcana_text.font_leaf.Cmap12Group] :: :: call

fn empty_int_list() -> List[Int]:
    return std.collections.list.new[Int] :: :: call

fn empty_alpha() -> Array[Int]:
    return std.collections.array.from_list[Int] :: (empty_int_list :: :: call) :: call

fn point_zero() -> arcana_text.font_leaf.FontPoint:
    return arcana_text.font_leaf.FontPoint :: x = 0, y = 0, on_curve = true :: call

fn empty_tables() -> Map[Str, (Int, Int)]:
    return std.collections.map.new[Str, (Int, Int)] :: :: call

fn empty_metrics_pair() -> (Array[Int], Array[Int]):
    return ((empty_alpha :: :: call), (empty_alpha :: :: call))

export fn default_traits() -> arcana_text.font_leaf.FaceTraits:
    return arcana_text.font_leaf.FaceTraits :: weight = 400, width_milli = 100000, slant_milli = 0 :: call

export fn glyph_render_spec(text: Str, font_size: Int, line_height_milli: Int) -> arcana_text.font_leaf.GlyphRenderSpec:
    let mut out = arcana_text.font_leaf.GlyphRenderSpec :: text = text, glyph_index = -1, font_size = font_size :: call
    out.line_height_milli = line_height_milli
    out.traits = default_traits :: :: call
    out.feature_signature = 0
    out.axis_signature = 0
    return out

export fn face_load_request(family_name: Str, source_label: Str, source_path: Str) -> arcana_text.font_leaf.FaceLoadRequest:
    let mut out = arcana_text.font_leaf.FaceLoadRequest :: family_name = family_name, source_label = source_label, source_path = source_path :: call
    out.font_bytes = empty_alpha :: :: call
    out.traits = default_traits :: :: call
    return out

fn face_load_meta(family_name: Str, source_label: Str, source_path: Str) -> arcana_text.font_leaf.FaceLoadMeta:
    let mut out = arcana_text.font_leaf.FaceLoadMeta :: family_name = family_name, source_label = source_label, source_path = source_path :: call
    out.traits = default_traits :: :: call
    return out

fn cmap_state(read segments: List[arcana_text.font_leaf.Cmap4Segment], read glyphs: Array[Int], read groups: List[arcana_text.font_leaf.Cmap12Group]) -> arcana_text.font_leaf.CmapState:
    return arcana_text.font_leaf.CmapState :: segments = segments, glyphs = glyphs, groups = groups :: call

fn affine_matrix(xx: Int, xy: Int, axes: (Int, Int)) -> arcana_text.font_leaf.AffineMatrix:
    let mut out = arcana_text.font_leaf.AffineMatrix :: xx = xx, xy = xy, yx = axes.0 :: call
    out.yx = axes.0
    out.yy = axes.1
    return out

fn coordinate_decode_spec(read bytes: Array[Int], cursor: Int, read flags: Array[Int]) -> arcana_text.font_leaf.CoordinateDecodeSpec:
    let mut out = arcana_text.font_leaf.CoordinateDecodeSpec :: bytes = bytes, cursor = cursor, flags = flags :: call
    out.count = 0
    out.short_mask = 0
    out.same_mask = 0
    return out

fn cmap4_lookup(read segments: List[arcana_text.font_leaf.Cmap4Segment], read glyphs: Array[Int], codepoint: Int) -> arcana_text.font_leaf.Cmap4Lookup:
    return arcana_text.font_leaf.Cmap4Lookup :: segments = segments, glyphs = glyphs, codepoint = codepoint :: call

fn empty_int_map() -> Map[Int, Int]:
    return std.collections.map.new[Int, Int] :: :: call

fn ensure_font_bytes(edit face: arcana_text.font_leaf.FontFaceState):
    if (face.font_bytes :: :: len) > 0:
        return
    face.font_bytes = face.font_view :: :: to_array

fn scale_context(read face: arcana_text.font_leaf.FontFaceState, read traits: arcana_text.font_leaf.FaceTraits, font_size: Int) -> arcana_text.font_leaf.ScaleContext:
    return arcana_text.font_leaf.ScaleContext :: face = face, traits = traits, font_size = font_size :: call

fn max_int(a: Int, b: Int) -> Int:
    if a >= b:
        return a
    return b

fn min_int(a: Int, b: Int) -> Int:
    if a <= b:
        return a
    return b

fn abs_int(value: Int) -> Int:
    if value < 0:
        return 0 - value
    return value

fn clamp_int(value: Int, low: Int, high: Int) -> Int:
    let mut out = value
    if out < low:
        out = low
    if out > high:
        out = high
    return out

fn floor_div(value: Int, denom: Int) -> Int:
    let mut out = value / denom
    if value < 0 and (out * denom) != value:
        out -= 1
    return out

fn ceil_div(value: Int, denom: Int) -> Int:
    let mut out = value / denom
    if value > 0 and (out * denom) != value:
        out += 1
    return out

fn positive_mod(value: Int, base: Int) -> Int:
    let mut out = value % base
    if out < 0:
        out += base
    return out

fn byte_at_or_zero(read bytes: Array[Int], index: Int) -> Int:
    if index < 0 or index >= (bytes :: :: len):
        return 0
    return (bytes)[index]

fn byte_at_or_zero_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    if index < 0 or index >= (bytes :: :: len):
        return 0
    return bytes :: index :: at

fn int_list_at_or_zero(read values: List[Int], target: Int) -> Int:
    let mut index = 0
    for value in values:
        if index == target:
            return value
        index += 1
    return 0

fn point_list_at_or_zero(read values: List[arcana_text.font_leaf.FontPoint], target: Int) -> arcana_text.font_leaf.FontPoint:
    let mut index = 0
    for value in values:
        if index == target:
            return value
        index += 1
    return point_zero :: :: call

fn u16_be(read bytes: Array[Int], index: Int) -> Int:
    return (byte_at_or_zero :: bytes, index :: call) * 256 + (byte_at_or_zero :: bytes, index + 1 :: call)

fn u16_be_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    return (byte_at_or_zero_ref :: bytes, index :: call) * 256 + (byte_at_or_zero_ref :: bytes, index + 1 :: call)

fn i16_be(read bytes: Array[Int], index: Int) -> Int:
    let raw = u16_be :: bytes, index :: call
    if raw >= 32768:
        return raw - 65536
    return raw

fn i16_be_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    let raw = u16_be_ref :: bytes, index :: call
    if raw >= 32768:
        return raw - 65536
    return raw

fn u32_be(read bytes: Array[Int], index: Int) -> Int:
    return ((u16_be :: bytes, index :: call) * 65536) + (u16_be :: bytes, index + 2 :: call)

fn u32_be_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    return ((u16_be_ref :: bytes, index :: call) * 65536) + (u16_be_ref :: bytes, index + 2 :: call)

fn tag_at(read bytes: Array[Int], index: Int) -> Str:
    let mut out = std.bytes.new_buf :: :: call
    std.bytes.buf_push :: out, (byte_at_or_zero :: bytes, index :: call) :: call
    std.bytes.buf_push :: out, (byte_at_or_zero :: bytes, index + 1 :: call) :: call
    std.bytes.buf_push :: out, (byte_at_or_zero :: bytes, index + 2 :: call) :: call
    std.bytes.buf_push :: out, (byte_at_or_zero :: bytes, index + 3 :: call) :: call
    return std.bytes.to_str_utf8 :: (std.bytes.buf_to_array :: out :: call) :: call

fn tag_at_ref(read bytes: std.memory.ByteView, index: Int) -> Str:
    let mut out = std.bytes.new_buf :: :: call
    std.bytes.buf_push :: out, (byte_at_or_zero_ref :: bytes, index :: call) :: call
    std.bytes.buf_push :: out, (byte_at_or_zero_ref :: bytes, index + 1 :: call) :: call
    std.bytes.buf_push :: out, (byte_at_or_zero_ref :: bytes, index + 2 :: call) :: call
    std.bytes.buf_push :: out, (byte_at_or_zero_ref :: bytes, index + 3 :: call) :: call
    return std.bytes.to_str_utf8 :: (std.bytes.buf_to_array :: out :: call) :: call

fn safe_file_stem(path: Str) -> Str:
    return match (std.path.stem :: path :: call):
        Result.Ok(value) => value
        Result.Err(_) => (std.path.file_name :: path :: call)

fn utf8_codepoint(read text: Str) -> Int:
    let bytes = std.bytes.from_str_utf8 :: text :: call
    let total = std.bytes.len :: bytes :: call
    if total <= 0:
        return 0
    let first = std.bytes.at :: bytes, 0 :: call
    if first < 128:
        return first
    if total >= 2 and first < 224:
        return ((first % 32) * 64) + ((std.bytes.at :: bytes, 1 :: call) % 64)
    if total >= 3 and first < 240:
        return ((first % 16) * 4096) + (((std.bytes.at :: bytes, 1 :: call) % 64) * 64) + ((std.bytes.at :: bytes, 2 :: call) % 64)
    if total >= 4 and first < 248:
        return ((first % 8) * 262144) + (((std.bytes.at :: bytes, 1 :: call) % 64) * 4096) + (((std.bytes.at :: bytes, 2 :: call) % 64) * 64) + ((std.bytes.at :: bytes, 3 :: call) % 64)
    return first

fn read_table_offset(read tables: Map[Str, (Int, Int)], read tag: Str) -> Result[(Int, Int), Str]:
    if not (tables :: tag :: has):
        return Result.Err[(Int, Int), Str] :: ("font is missing required `" + tag + "` table") :: call
    return Result.Ok[(Int, Int), Str] :: (tables :: tag :: get) :: call

fn parse_table_directory(read bytes: Array[Int]) -> Result[Map[Str, (Int, Int)], Str]:
    let total = bytes :: :: len
    if total < 12:
        return Result.Err[Map[Str, (Int, Int)], Str] :: "font file is too small" :: call
    let scaler_value = u32_be :: bytes, 0 :: call
    let scaler = tag_at :: bytes, 0 :: call
    let is_true_type = scaler_value == 65536 or scaler == "true" or scaler == "OTTO"
    if not is_true_type:
        return Result.Err[Map[Str, (Int, Int)], Str] :: ("unsupported font scaler `" + scaler + "`") :: call
    let num_tables = u16_be :: bytes, 4 :: call
    let mut tables = std.collections.map.new[Str, (Int, Int)] :: :: call
    let mut cursor = 12
    let mut index = 0
    while index < num_tables:
        if cursor + 16 > total:
            return Result.Err[Map[Str, (Int, Int)], Str] :: "truncated table directory" :: call
        let tag = tag_at :: bytes, cursor :: call
        let offset = u32_be :: bytes, cursor + 8 :: call
        let length = u32_be :: bytes, cursor + 12 :: call
        tables :: tag, (offset, length) :: set
        cursor += 16
        index += 1
    return Result.Ok[Map[Str, (Int, Int)], Str] :: tables :: call

fn parse_table_directory_ref(read bytes: std.memory.ByteView) -> Result[Map[Str, (Int, Int)], Str]:
    let total = bytes :: :: len
    if total < 12:
        return Result.Err[Map[Str, (Int, Int)], Str] :: "font file is too small" :: call
    let scaler_value = u32_be_ref :: bytes, 0 :: call
    let scaler = tag_at_ref :: bytes, 0 :: call
    let is_true_type = scaler_value == 65536 or scaler == "true" or scaler == "OTTO"
    if not is_true_type:
        return Result.Err[Map[Str, (Int, Int)], Str] :: ("unsupported font scaler `" + scaler + "`") :: call
    let num_tables = u16_be_ref :: bytes, 4 :: call
    let mut tables = std.collections.map.new[Str, (Int, Int)] :: :: call
    let mut cursor = 12
    let mut index = 0
    while index < num_tables:
        if cursor + 16 > total:
            return Result.Err[Map[Str, (Int, Int)], Str] :: "truncated table directory" :: call
        let tag = tag_at_ref :: bytes, cursor :: call
        let offset = u32_be_ref :: bytes, cursor + 8 :: call
        let length = u32_be_ref :: bytes, cursor + 12 :: call
        tables :: tag, (offset, length) :: set
        cursor += 16
        index += 1
    return Result.Ok[Map[Str, (Int, Int)], Str] :: tables :: call

fn result_err_or[T](read value: Result[T, Str], read fallback: Str) -> Str:
    return match value:
        Result.Ok(_) => fallback
        Result.Err(err) => err

fn parse_h_metrics(read bytes: Array[Int], tables: ((Int, Int), (Int, Int)), glyph_count: Int) -> Result[(Array[Int], Array[Int]), Str]:
    let hhea = tables.0
    let hmtx = tables.1
    let metric_count = u16_be :: bytes, hhea.0 + 34 :: call
    let mut widths = empty_int_list :: :: call
    let mut bearings = empty_int_list :: :: call
    let mut cursor = hmtx.0
    let mut index = 0
    let mut last_width = 0
    while index < glyph_count:
        if index < metric_count:
            if cursor + 4 > (hmtx.0 + hmtx.1):
                return Result.Err[(Array[Int], Array[Int]), Str] :: "truncated hmtx table" :: call
            last_width = u16_be :: bytes, cursor :: call
            widths :: last_width :: push
            bearings :: (i16_be :: bytes, cursor + 2 :: call) :: push
            cursor += 4
        else:
            if cursor + 2 > (hmtx.0 + hmtx.1):
                return Result.Err[(Array[Int], Array[Int]), Str] :: "truncated hmtx bearings" :: call
            widths :: last_width :: push
            bearings :: (i16_be :: bytes, cursor :: call) :: push
            cursor += 2
        index += 1
    return Result.Ok[(Array[Int], Array[Int]), Str] :: ((std.collections.array.from_list[Int] :: widths :: call), (std.collections.array.from_list[Int] :: bearings :: call)) :: call

fn parse_h_metrics_ref(read bytes: std.memory.ByteView, tables: ((Int, Int), (Int, Int)), glyph_count: Int) -> Result[(Array[Int], Array[Int]), Str]:
    let hhea = tables.0
    let hmtx = tables.1
    let metric_count = u16_be_ref :: bytes, hhea.0 + 34 :: call
    let mut widths = empty_int_list :: :: call
    let mut bearings = empty_int_list :: :: call
    let mut cursor = hmtx.0
    let mut index = 0
    let mut last_width = 0
    while index < glyph_count:
        if index < metric_count:
            if cursor + 4 > (hmtx.0 + hmtx.1):
                return Result.Err[(Array[Int], Array[Int]), Str] :: "truncated hmtx table" :: call
            last_width = u16_be_ref :: bytes, cursor :: call
            widths :: last_width :: push
            bearings :: (i16_be_ref :: bytes, cursor + 2 :: call) :: push
            cursor += 4
        else:
            if cursor + 2 > (hmtx.0 + hmtx.1):
                return Result.Err[(Array[Int], Array[Int]), Str] :: "truncated hmtx bearings" :: call
            widths :: last_width :: push
            bearings :: (i16_be_ref :: bytes, cursor :: call) :: push
            cursor += 2
        index += 1
    return Result.Ok[(Array[Int], Array[Int]), Str] :: ((std.collections.array.from_list[Int] :: widths :: call), (std.collections.array.from_list[Int] :: bearings :: call)) :: call

fn parse_loca_offsets(read bytes: Array[Int], tables: ((Int, Int), (Int, Int)), glyph_count: Int) -> Result[Array[Int], Str]:
    let head = tables.0
    let loca = tables.1
    let format = i16_be :: bytes, head.0 + 50 :: call
    let mut out = empty_int_list :: :: call
    let total = glyph_count + 1
    let mut index = 0
    while index < total:
        let offset = match format:
            0 => (u16_be :: bytes, loca.0 + (index * 2) :: call) * 2
            _ => u32_be :: bytes, loca.0 + (index * 4) :: call
        out :: offset :: push
        index += 1
    return Result.Ok[Array[Int], Str] :: (std.collections.array.from_list[Int] :: out :: call) :: call

fn parse_loca_offsets_ref(read bytes: std.memory.ByteView, tables: ((Int, Int), (Int, Int)), glyph_count: Int) -> Result[Array[Int], Str]:
    let head = tables.0
    let loca = tables.1
    let format = i16_be_ref :: bytes, head.0 + 50 :: call
    let mut out = empty_int_list :: :: call
    let total = glyph_count + 1
    let mut index = 0
    while index < total:
        let offset = match format:
            0 => (u16_be_ref :: bytes, loca.0 + (index * 2) :: call) * 2
            _ => u32_be_ref :: bytes, loca.0 + (index * 4) :: call
        out :: offset :: push
        index += 1
    return Result.Ok[Array[Int], Str] :: (std.collections.array.from_list[Int] :: out :: call) :: call

fn parse_cmap_format12(read bytes: Array[Int], table_offset: Int) -> List[arcana_text.font_leaf.Cmap12Group]:
    let groups = u32_be :: bytes, table_offset + 12 :: call
    let mut out = empty_cmap12_groups :: :: call
    let mut cursor = table_offset + 16
    let mut index = 0
    while index < groups:
        let group = arcana_text.font_leaf.Cmap12Group :: start_code = (u32_be :: bytes, cursor :: call), end_code = (u32_be :: bytes, cursor + 4 :: call), start_glyph = (u32_be :: bytes, cursor + 8 :: call) :: call
        out :: group :: push
        cursor += 12
        index += 1
    return out

fn parse_cmap_format12_ref(read bytes: std.memory.ByteView, table_offset: Int) -> List[arcana_text.font_leaf.Cmap12Group]:
    let groups = u32_be_ref :: bytes, table_offset + 12 :: call
    let mut out = empty_cmap12_groups :: :: call
    let mut cursor = table_offset + 16
    let mut index = 0
    while index < groups:
        let group = arcana_text.font_leaf.Cmap12Group :: start_code = (u32_be_ref :: bytes, cursor :: call), end_code = (u32_be_ref :: bytes, cursor + 4 :: call), start_glyph = (u32_be_ref :: bytes, cursor + 8 :: call) :: call
        out :: group :: push
        cursor += 12
        index += 1
    return out

fn parse_cmap_format4(read bytes: Array[Int], table_offset: Int) -> (List[arcana_text.font_leaf.Cmap4Segment], Array[Int]):
    let seg_count = (u16_be :: bytes, table_offset + 6 :: call) / 2
    let end_codes = table_offset + 14
    let start_codes = end_codes + (seg_count * 2) + 2
    let id_deltas = start_codes + (seg_count * 2)
    let id_offsets = id_deltas + (seg_count * 2)
    let mut segments = empty_cmap4_segments :: :: call
    let mut index = 0
    while index < seg_count:
        let start_code = u16_be :: bytes, start_codes + (index * 2) :: call
        let end_code = u16_be :: bytes, end_codes + (index * 2) :: call
        let id_delta = i16_be :: bytes, id_deltas + (index * 2) :: call
        let id_range_offset = u16_be :: bytes, id_offsets + (index * 2) :: call
        let mut segment = arcana_text.font_leaf.Cmap4Segment :: start_code = start_code, end_code = end_code, id_delta = id_delta :: call
        segment.glyph_base_index = -1
        segment.direct_map = id_range_offset == 0
        if id_range_offset != 0:
            segment.glyph_base_index = id_offsets + (index * 2) + id_range_offset
        segments :: segment :: push
        index += 1
    return (segments, (empty_alpha :: :: call))

fn parse_cmap_format4_ref(read bytes: std.memory.ByteView, table_offset: Int) -> (List[arcana_text.font_leaf.Cmap4Segment], Array[Int]):
    let seg_count = (u16_be_ref :: bytes, table_offset + 6 :: call) / 2
    let end_codes = table_offset + 14
    let start_codes = end_codes + (seg_count * 2) + 2
    let id_deltas = start_codes + (seg_count * 2)
    let id_offsets = id_deltas + (seg_count * 2)
    let mut segments = empty_cmap4_segments :: :: call
    let mut index = 0
    while index < seg_count:
        let start_code = u16_be_ref :: bytes, start_codes + (index * 2) :: call
        let end_code = u16_be_ref :: bytes, end_codes + (index * 2) :: call
        let id_delta = i16_be_ref :: bytes, id_deltas + (index * 2) :: call
        let id_range_offset = u16_be_ref :: bytes, id_offsets + (index * 2) :: call
        let mut segment = arcana_text.font_leaf.Cmap4Segment :: start_code = start_code, end_code = end_code, id_delta = id_delta :: call
        segment.glyph_base_index = -1
        segment.direct_map = id_range_offset == 0
        if id_range_offset != 0:
            segment.glyph_base_index = id_offsets + (index * 2) + id_range_offset
        segments :: segment :: push
        index += 1
    return (segments, (empty_alpha :: :: call))

fn parse_cmap(read bytes: Array[Int], cmap: (Int, Int)) -> arcana_text.font_leaf.CmapState:
    let record_count = u16_be :: bytes, cmap.0 + 2 :: call
    let mut format4_offset = -1
    let mut format12_offset = -1
    let mut cursor = cmap.0 + 4
    let mut index = 0
    while index < record_count:
        let platform = u16_be :: bytes, cursor :: call
        let encoding = u16_be :: bytes, cursor + 2 :: call
        let offset = cmap.0 + (u32_be :: bytes, cursor + 4 :: call)
        let format = u16_be :: bytes, offset :: call
        if format == 12 and (platform == 0 or (platform == 3 and encoding == 10)):
            format12_offset = offset
        if format == 4 and (platform == 0 or (platform == 3 and encoding == 1)):
            format4_offset = offset
        cursor += 8
        index += 1
    let mut segments = empty_cmap4_segments :: :: call
    let mut glyphs = std.collections.array.from_list[Int] :: (empty_int_list :: :: call) :: call
    let mut groups = empty_cmap12_groups :: :: call
    if format4_offset >= 0:
        let parsed = parse_cmap_format4 :: bytes, format4_offset :: call
        segments = parsed.0
        glyphs = parsed.1
    if format12_offset >= 0:
        groups = parse_cmap_format12 :: bytes, format12_offset :: call
    return cmap_state :: segments, glyphs, groups :: call

fn parse_cmap_ref(read bytes: std.memory.ByteView, cmap: (Int, Int)) -> arcana_text.font_leaf.CmapState:
    let record_count = u16_be_ref :: bytes, cmap.0 + 2 :: call
    let mut format4_offset = -1
    let mut format12_offset = -1
    let mut cursor = cmap.0 + 4
    let mut index = 0
    while index < record_count:
        let platform = u16_be_ref :: bytes, cursor :: call
        let encoding = u16_be_ref :: bytes, cursor + 2 :: call
        let offset = cmap.0 + (u32_be_ref :: bytes, cursor + 4 :: call)
        let format = u16_be_ref :: bytes, offset :: call
        if format == 12 and (platform == 0 or (platform == 3 and encoding == 10)):
            format12_offset = offset
        if format == 4 and (platform == 0 or (platform == 3 and encoding == 1)):
            format4_offset = offset
        cursor += 8
        index += 1
    let mut segments = empty_cmap4_segments :: :: call
    let mut glyphs = std.collections.array.from_list[Int] :: (empty_int_list :: :: call) :: call
    let mut groups = empty_cmap12_groups :: :: call
    if format4_offset >= 0:
        let parsed = parse_cmap_format4_ref :: bytes, format4_offset :: call
        segments = parsed.0
        glyphs = parsed.1
    if format12_offset >= 0:
        groups = parse_cmap_format12_ref :: bytes, format12_offset :: call
    return cmap_state :: segments, glyphs, groups :: call

fn detect_cmap_offsets(read bytes: Array[Int], cmap: (Int, Int)) -> (Int, Int):
    let record_count = u16_be :: bytes, cmap.0 + 2 :: call
    let mut format4_offset = -1
    let mut format12_offset = -1
    let mut cursor = cmap.0 + 4
    let mut index = 0
    while index < record_count:
        let platform = u16_be :: bytes, cursor :: call
        let encoding = u16_be :: bytes, cursor + 2 :: call
        let offset = cmap.0 + (u32_be :: bytes, cursor + 4 :: call)
        let format = u16_be :: bytes, offset :: call
        if format == 12 and (platform == 0 or (platform == 3 and encoding == 10)):
            format12_offset = offset
        if format == 4 and (platform == 0 or (platform == 3 and encoding == 1)):
            format4_offset = offset
        cursor += 8
        index += 1
    return (format4_offset, format12_offset)

fn detect_cmap_offsets_ref(read bytes: std.memory.ByteView, cmap: (Int, Int)) -> (Int, Int):
    let record_count = u16_be_ref :: bytes, cmap.0 + 2 :: call
    let mut format4_offset = -1
    let mut format12_offset = -1
    let mut cursor = cmap.0 + 4
    let mut index = 0
    while index < record_count:
        let platform = u16_be_ref :: bytes, cursor :: call
        let encoding = u16_be_ref :: bytes, cursor + 2 :: call
        let offset = cmap.0 + (u32_be_ref :: bytes, cursor + 4 :: call)
        let format = u16_be_ref :: bytes, offset :: call
        if format == 12 and (platform == 0 or (platform == 3 and encoding == 10)):
            format12_offset = offset
        if format == 4 and (platform == 0 or (platform == 3 and encoding == 1)):
            format4_offset = offset
        cursor += 8
        index += 1
    return (format4_offset, format12_offset)

fn glyph_index_from_cmap12_offset(read bytes: Array[Int], cmap12_offset: Int, codepoint: Int) -> Int:
    if cmap12_offset < 0:
        return 0
    let groups = u32_be :: bytes, cmap12_offset + 12 :: call
    let mut cursor = cmap12_offset + 16
    let mut index = 0
    while index < groups:
        let start_code = u32_be :: bytes, cursor :: call
        let end_code = u32_be :: bytes, cursor + 4 :: call
        if codepoint >= start_code and codepoint <= end_code:
            return (u32_be :: bytes, cursor + 8 :: call) + (codepoint - start_code)
        cursor += 12
        index += 1
    return 0

fn glyph_index_from_cmap12_offset_ref(read bytes: std.memory.ByteView, cmap12_offset: Int, codepoint: Int) -> Int:
    if cmap12_offset < 0:
        return 0
    let groups = u32_be_ref :: bytes, cmap12_offset + 12 :: call
    let mut cursor = cmap12_offset + 16
    let mut index = 0
    while index < groups:
        let start_code = u32_be_ref :: bytes, cursor :: call
        let end_code = u32_be_ref :: bytes, cursor + 4 :: call
        if codepoint >= start_code and codepoint <= end_code:
            return (u32_be_ref :: bytes, cursor + 8 :: call) + (codepoint - start_code)
        cursor += 12
        index += 1
    return 0

fn glyph_index_from_cmap12_bytes(read face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    return glyph_index_from_cmap12_offset :: face.font_bytes, face.cmap12_offset, codepoint :: call

fn glyph_index_from_cmap4_offset(read bytes: Array[Int], cmap4_offset: Int, codepoint: Int) -> Int:
    if cmap4_offset < 0:
        return 0
    let seg_count = (u16_be :: bytes, cmap4_offset + 6 :: call) / 2
    let end_codes = cmap4_offset + 14
    let start_codes = end_codes + (seg_count * 2) + 2
    let id_deltas = start_codes + (seg_count * 2)
    let id_offsets = id_deltas + (seg_count * 2)
    let mut index = 0
    while index < seg_count:
        let start_code = u16_be :: bytes, start_codes + (index * 2) :: call
        let end_code = u16_be :: bytes, end_codes + (index * 2) :: call
        if codepoint >= start_code and codepoint <= end_code:
            let id_delta = i16_be :: bytes, id_deltas + (index * 2) :: call
            let id_range_offset = u16_be :: bytes, id_offsets + (index * 2) :: call
            if id_range_offset == 0:
                return positive_mod :: codepoint + id_delta, 65536 :: call
            let glyph_address = id_offsets + (index * 2) + id_range_offset + ((codepoint - start_code) * 2)
            let raw = u16_be :: bytes, glyph_address :: call
            if raw == 0:
                return 0
            return positive_mod :: raw + id_delta, 65536 :: call
        index += 1
    return 0

fn glyph_index_from_cmap4_offset_ref(read bytes: std.memory.ByteView, cmap4_offset: Int, codepoint: Int) -> Int:
    if cmap4_offset < 0:
        return 0
    let seg_count = (u16_be_ref :: bytes, cmap4_offset + 6 :: call) / 2
    let end_codes = cmap4_offset + 14
    let start_codes = end_codes + (seg_count * 2) + 2
    let id_deltas = start_codes + (seg_count * 2)
    let id_offsets = id_deltas + (seg_count * 2)
    let mut index = 0
    while index < seg_count:
        let start_code = u16_be_ref :: bytes, start_codes + (index * 2) :: call
        let end_code = u16_be_ref :: bytes, end_codes + (index * 2) :: call
        if codepoint >= start_code and codepoint <= end_code:
            let id_delta = i16_be_ref :: bytes, id_deltas + (index * 2) :: call
            let id_range_offset = u16_be_ref :: bytes, id_offsets + (index * 2) :: call
            if id_range_offset == 0:
                return positive_mod :: codepoint + id_delta, 65536 :: call
            let glyph_address = id_offsets + (index * 2) + id_range_offset + ((codepoint - start_code) * 2)
            let raw = u16_be_ref :: bytes, glyph_address :: call
            if raw == 0:
                return 0
            return positive_mod :: raw + id_delta, 65536 :: call
        index += 1
    return 0

fn glyph_index_from_cmap4_bytes(read face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    return glyph_index_from_cmap4_offset :: face.font_bytes, face.cmap4_offset, codepoint :: call

fn glyph_index_from_cmap4(read face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    if (face.cmap4_segments :: :: len) > 0:
        return glyph_index_from_segment_tables :: face.font_bytes, (cmap4_lookup :: face.cmap4_segments, face.cmap4_glyphs, codepoint :: call) :: call
    if face.cmap4_offset >= 0 or face.cmap12_offset >= 0:
        let format4 = glyph_index_from_cmap4_bytes :: face, codepoint :: call
        if format4 > 0:
            return format4
        return glyph_index_from_cmap12_bytes :: face, codepoint :: call
    return 0

fn glyph_index_for_codepoint_uncached(edit face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    if face.cmap4_offset >= 0 or face.cmap12_offset >= 0:
        ensure_font_bytes :: face :: call
        if codepoint >= 0 and codepoint < 65536:
            let format4 = glyph_index_from_cmap4_bytes :: face, codepoint :: call
            if format4 > 0:
                return format4
        let format12 = glyph_index_from_cmap12_bytes :: face, codepoint :: call
        if format12 > 0:
            return format12
        return glyph_index_from_cmap4 :: face, codepoint :: call
    for group in face.cmap12_groups:
        if codepoint >= group.start_code and codepoint <= group.end_code:
            return group.start_glyph + (codepoint - group.start_code)
    return glyph_index_from_cmap4 :: face, codepoint :: call

export fn glyph_index_for_codepoint(edit face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    if face.glyph_index_cache :: codepoint :: has:
        return face.glyph_index_cache :: codepoint :: get
    let glyph = glyph_index_for_codepoint_uncached :: face, codepoint :: call
    face.glyph_index_cache :: codepoint, glyph :: set
    return glyph

fn glyph_index_from_segment_tables(read bytes: Array[Int], read lookup: arcana_text.font_leaf.Cmap4Lookup) -> Int:
    for segment in lookup.segments:
        if lookup.codepoint < segment.start_code or lookup.codepoint > segment.end_code:
            continue
        if segment.direct_map:
            return positive_mod :: lookup.codepoint + segment.id_delta, 65536 :: call
        let mut raw = 0
        if (lookup.glyphs :: :: len) > 0:
            let glyph_index = segment.glyph_base_index + (lookup.codepoint - segment.start_code)
            if glyph_index < 0 or glyph_index >= (lookup.glyphs :: :: len):
                return 0
            raw = byte_at_or_zero :: lookup.glyphs, glyph_index :: call
        else:
            let glyph_address = segment.glyph_base_index + ((lookup.codepoint - segment.start_code) * 2)
            raw = u16_be :: bytes, glyph_address :: call
        if raw == 0:
            return 0
        return positive_mod :: raw + segment.id_delta, 65536 :: call
    return 0

export fn supports_text(edit face: arcana_text.font_leaf.FontFaceState, read ch: Str) -> Bool:
    if ch == " " or ch == "\t" or ch == "\n" or ch == "\r":
        return true
    return (glyph_index_for_codepoint :: face, (utf8_codepoint :: ch :: call) :: call) > 0

fn outline_value(advance_width: Int, left_side_bearing: Int) -> arcana_text.font_leaf.GlyphOutline:
    let mut out = arcana_text.font_leaf.GlyphOutline :: advance_width = advance_width, left_side_bearing = left_side_bearing, x_min = 0 :: call
    out.y_min = 0
    out.x_max = 0
    out.y_max = 0
    out.contours = empty_contours :: :: call
    out.empty = true
    return out

fn contour_value() -> arcana_text.font_leaf.FontContour:
    return arcana_text.font_leaf.FontContour :: points = (empty_points :: :: call) :: call

fn point_value(x: Int, y: Int, on_curve: Bool) -> arcana_text.font_leaf.FontPoint:
    return arcana_text.font_leaf.FontPoint :: x = x, y = y, on_curve = on_curve :: call

fn line_segment(start: (Int, Int), end: (Int, Int)) -> arcana_text.font_leaf.LineSegment:
    return arcana_text.font_leaf.LineSegment :: start = start, end = end :: call

fn glyph_offset_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> (Int, Int):
    if glyph_index < 0 or glyph_index + 1 >= (face.loca_offsets :: :: len):
        return (0, 0)
    return ((byte_at_or_zero :: face.loca_offsets, glyph_index :: call), (byte_at_or_zero :: face.loca_offsets, glyph_index + 1 :: call))

fn ensure_loca_offsets(edit face: arcana_text.font_leaf.FontFaceState):
    if (face.loca_offsets :: :: len) > 0:
        return
    ensure_font_bytes :: face :: call
    let mut out = empty_int_list :: :: call
    let total = face.glyph_count + 1
    let mut index = 0
    while index < total:
        let offset = match face.loca_format:
            0 => (u16_be :: face.font_bytes, face.loca_offset + (index * 2) :: call) * 2
            _ => u32_be :: face.font_bytes, face.loca_offset + (index * 4) :: call
        out :: offset :: push
        index += 1
    face.loca_offsets = std.collections.array.from_list[Int] :: out :: call

fn advance_width_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    if glyph_index < 0:
        return 0
    if glyph_index < (face.advance_widths :: :: len):
        return byte_at_or_zero :: face.advance_widths, glyph_index :: call
    if face.hmetric_count <= 0:
        return 0
    let metric_index = min_int :: glyph_index, face.hmetric_count - 1 :: call
    return u16_be :: face.font_bytes, face.hmtx_offset + (metric_index * 4) :: call

fn left_side_bearing_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    if glyph_index < 0:
        return 0
    if glyph_index < (face.left_side_bearings :: :: len):
        return byte_at_or_zero :: face.left_side_bearings, glyph_index :: call
    if face.hmetric_count <= 0:
        return 0
    if glyph_index < face.hmetric_count:
        return i16_be :: face.font_bytes, face.hmtx_offset + (glyph_index * 4) + 2 :: call
    let extra_index = glyph_index - face.hmetric_count
    return i16_be :: face.font_bytes, face.hmtx_offset + (face.hmetric_count * 4) + (extra_index * 2) :: call

fn decode_flags(read bytes: Array[Int], start: Int, count: Int) -> (Array[Int], Int):
    let mut flags = empty_int_list :: :: call
    let mut cursor = start
    while (flags :: :: len) < count:
        let flag = byte_at_or_zero :: bytes, cursor :: call
        cursor += 1
        flags :: flag :: push
        if (flag % 16) >= 8:
            let repeats = byte_at_or_zero :: bytes, cursor :: call
            cursor += 1
            let mut index = 0
            while index < repeats:
                flags :: flag :: push
                index += 1
    return ((std.collections.array.from_list[Int] :: flags :: call), cursor)

fn decode_coordinates(read spec: arcana_text.font_leaf.CoordinateDecodeSpec) -> (Array[Int], Int):
    let mut out = empty_int_list :: :: call
    let mut at = spec.cursor
    let mut current = 0
    let mut index = 0
    while index < spec.count:
        let flag = byte_at_or_zero :: spec.flags, index :: call
        let short = (flag % (spec.short_mask * 2)) >= spec.short_mask
        let same = (flag % (spec.same_mask * 2)) >= spec.same_mask
        let mut delta = 0
        if short:
            delta = byte_at_or_zero :: spec.bytes, at :: call
            at += 1
            if not same:
                delta = 0 - delta
        else:
            if not same:
                delta = i16_be :: spec.bytes, at :: call
                at += 2
        current += delta
        out :: current :: push
        index += 1
    return ((std.collections.array.from_list[Int] :: out :: call), at)

fn parse_simple_outline(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int, glyph_offset: Int) -> arcana_text.font_leaf.GlyphOutline:
    let start = face.glyf_offset + glyph_offset
    let contour_count = i16_be :: face.font_bytes, start :: call
    let advance_width = advance_width_for :: face, glyph_index :: call
    let left_side_bearing = left_side_bearing_for :: face, glyph_index :: call
    let mut outline = outline_value :: advance_width, left_side_bearing :: call
    outline.x_min = i16_be :: face.font_bytes, start + 2 :: call
    outline.y_min = i16_be :: face.font_bytes, start + 4 :: call
    outline.x_max = i16_be :: face.font_bytes, start + 6 :: call
    outline.y_max = i16_be :: face.font_bytes, start + 8 :: call
    if contour_count <= 0:
        outline.empty = true
        return outline
    let mut end_points = empty_int_list :: :: call
    let mut cursor = start + 10
    let mut contour_index = 0
    while contour_index < contour_count:
        end_points :: (u16_be :: face.font_bytes, cursor :: call) :: push
        cursor += 2
        contour_index += 1
    let point_count = (int_list_at_or_zero :: end_points, (end_points :: :: len) - 1 :: call) + 1
    let instruction_len = u16_be :: face.font_bytes, cursor :: call
    cursor += 2 + instruction_len
    let decoded = decode_flags :: face.font_bytes, cursor, point_count :: call
    let flags = decoded.0
    cursor = decoded.1
    let mut x_spec = coordinate_decode_spec :: face.font_bytes, cursor, flags :: call
    x_spec.count = point_count
    x_spec.short_mask = 2
    x_spec.same_mask = 16
    let x_decoded = decode_coordinates :: x_spec :: call
    let xs = x_decoded.0
    cursor = x_decoded.1
    let mut y_spec = coordinate_decode_spec :: face.font_bytes, cursor, flags :: call
    y_spec.count = point_count
    y_spec.short_mask = 4
    y_spec.same_mask = 32
    let y_decoded = decode_coordinates :: y_spec :: call
    let ys = y_decoded.0
    let mut first_point = 0
    for end_point in end_points:
        let mut contour = contour_value :: :: call
        let mut point_index = first_point
        while point_index <= end_point:
            contour.points :: (point_value :: (byte_at_or_zero :: xs, point_index :: call), (byte_at_or_zero :: ys, point_index :: call), (((byte_at_or_zero :: flags, point_index :: call) % 2) == 1) :: call) :: call :: push
            point_index += 1
        outline.contours :: contour :: push
        first_point = end_point + 1
    outline.empty = false
    return outline

fn transform_point(point: (Int, Int), read matrix: arcana_text.font_leaf.AffineMatrix, offset: (Int, Int)) -> (Int, Int):
    let x = ((matrix.xx * point.0) + (matrix.xy * point.1)) / 16384 + offset.0
    let y = ((matrix.yx * point.0) + (matrix.yy * point.1)) / 16384 + offset.1
    return (x, y)

fn append_outline_contours(edit out: arcana_text.font_leaf.GlyphOutline, read source: arcana_text.font_leaf.GlyphOutline, read matrix: arcana_text.font_leaf.AffineMatrix, offset: (Int, Int)):
    for source_contour in source.contours:
        let mut contour = contour_value :: :: call
        for source_point in source_contour.points:
            let transformed = transform_point :: (source_point.x, source_point.y), matrix, offset :: call
            contour.points :: (point_value :: transformed.0, transformed.1, source_point.on_curve :: call) :: call :: push
        out.contours :: contour :: push
    out.empty = false

fn parse_compound_outline(read face: arcana_text.font_leaf.FontFaceState, glyph: (Int, Int), depth: Int) -> arcana_text.font_leaf.GlyphOutline:
    let glyph_index = glyph.0
    let glyph_offset = glyph.1
    let start = face.glyf_offset + glyph_offset
    let advance_width = advance_width_for :: face, glyph_index :: call
    let left_side_bearing = left_side_bearing_for :: face, glyph_index :: call
    let mut outline = outline_value :: advance_width, left_side_bearing :: call
    outline.x_min = i16_be :: face.font_bytes, start + 2 :: call
    outline.y_min = i16_be :: face.font_bytes, start + 4 :: call
    outline.x_max = i16_be :: face.font_bytes, start + 6 :: call
    outline.y_max = i16_be :: face.font_bytes, start + 8 :: call
    let mut cursor = start + 10
    let mut more = true
    while more and depth < 8:
        let flags = u16_be :: face.font_bytes, cursor :: call
        let component_index = u16_be :: face.font_bytes, cursor + 2 :: call
        cursor += 4
        let arg_words = (flags % 2) == 1
        let args_are_xy = ((flags / 2) % 2) == 1
        let mut arg1 = byte_at_or_zero :: face.font_bytes, cursor :: call
        let mut arg2 = byte_at_or_zero :: face.font_bytes, cursor + 1 :: call
        let mut arg_len = 2
        if arg_words:
            arg1 = i16_be :: face.font_bytes, cursor :: call
            arg2 = i16_be :: face.font_bytes, cursor + 2 :: call
            arg_len = 4
        cursor += arg_len
        let mut matrix = affine_matrix :: 16384, 0, (0, 16384) :: call
        if ((flags / 8) % 2) == 1:
            let scale = i16_be :: face.font_bytes, cursor :: call
            matrix = affine_matrix :: scale, 0, (0, scale) :: call
            cursor += 2
        else:
            if ((flags / 64) % 2) == 1:
                matrix = affine_matrix :: (i16_be :: face.font_bytes, cursor :: call), 0, (0, (i16_be :: face.font_bytes, cursor + 2 :: call)) :: call
                cursor += 4
        let mut offset = (0, 0)
        if args_are_xy:
            offset = (arg1, arg2)
        let component = load_outline_recursive :: face, component_index, depth + 1 :: call
        append_outline_contours :: outline, component, matrix :: call
            offset = offset
        more = ((flags / 32) % 2) == 1
    return outline

fn load_outline_recursive(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int, depth: Int) -> arcana_text.font_leaf.GlyphOutline:
    let offsets = glyph_offset_for :: face, glyph_index :: call
    let advance_width = advance_width_for :: face, glyph_index :: call
    let left_bearing = left_side_bearing_for :: face, glyph_index :: call
    if offsets.0 == offsets.1:
        return outline_value :: advance_width, left_bearing :: call
    let start = face.glyf_offset + offsets.0
    let contour_count = i16_be :: face.font_bytes, start :: call
    if contour_count >= 0:
        return parse_simple_outline :: face, glyph_index, offsets.0 :: call
    return parse_compound_outline :: face, (glyph_index, offsets.0), depth :: call

fn line_height_for(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: face.units_per_em, 1 :: call
    let safe_line_height = max_int :: line_height_milli, 1000 :: call
    let natural = ((face.ascender - face.descender + face.line_gap) * safe_font_size) / safe_units_per_em
    let scaled = (natural * safe_line_height) / 1000
    return max_int :: scaled, safe_font_size :: call

fn baseline_for(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: face.units_per_em, 1 :: call
    let natural = (face.ascender * safe_font_size) / safe_units_per_em
    let height = line_height_for :: face, font_size, line_height_milli :: call
    return clamp_int :: natural, 0, height :: call

fn normalized_width_milli(width_milli: Int) -> Int:
    if width_milli <= 0:
        return 100000
    return width_milli

fn scale_x(value: Int, font_size: Int, dims: (Int, Int)) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: dims.0, 1 :: call
    let safe_width = normalized_width_milli :: dims.1 :: call
    return (value * safe_font_size * safe_width) / (safe_units_per_em * 100000)

fn scale_y(value: Int, font_size: Int, units_per_em: Int) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: units_per_em, 1 :: call
    return (value * safe_font_size) / safe_units_per_em

fn scaled_point(read scale: arcana_text.font_leaf.ScaleContext, point: (Int, Int)) -> (Int, Int):
    let mut x = scale_x :: point.0, scale.font_size, (scale.face.units_per_em, scale.traits.width_milli) :: call
    let y = scale_y :: point.1, scale.font_size, scale.face.units_per_em :: call
    x += (scale.traits.slant_milli * y) / 1000
    return (x, y)

fn midpoint(a: (Int, Int), b: (Int, Int)) -> (Int, Int):
    return ((a.0 + b.0) / 2, (a.1 + b.1) / 2)

fn append_quad_segments(edit out: List[arcana_text.font_leaf.LineSegment], p0: (Int, Int), p1: (Int, Int), p2: (Int, Int)):
    let span_x0 = abs_int :: (p0.0 - p1.0) :: call
    let span_y0 = abs_int :: (p0.1 - p1.1) :: call
    let span_x1 = abs_int :: (p1.0 - p2.0) :: call
    let span_y1 = abs_int :: (p1.1 - p2.1) :: call
    let span = max_int :: (max_int :: span_x0, span_y0 :: call), (max_int :: span_x1, span_y1 :: call) :: call
    let steps = clamp_int :: (span / 4), 2, 10 :: call
    let denom = steps * steps
    let mut previous = p0
    let mut step = 1
    while step <= steps:
        let inv = steps - step
        let x = ((inv * inv * p0.0) + (2 * inv * step * p1.0) + (step * step * p2.0)) / denom
        let y = ((inv * inv * p0.1) + (2 * inv * step * p1.1) + (step * step * p2.1)) / denom
        let next = (x, y)
        out :: (line_segment :: previous, next :: call) :: push
        previous = next
        step += 1

fn contour_segments(read contour: arcana_text.font_leaf.FontContour, read scale: arcana_text.font_leaf.ScaleContext) -> List[arcana_text.font_leaf.LineSegment]:
    let count = contour.points :: :: len
    let mut out = empty_segments :: :: call
    if count <= 0:
        return out
    let first_raw = point_list_at_or_zero :: contour.points, 0 :: call
    let last_raw = point_list_at_or_zero :: contour.points, count - 1 :: call
    let first = scaled_point :: scale, (first_raw.x, first_raw.y) :: call
    let last = scaled_point :: scale, (last_raw.x, last_raw.y) :: call
    let mut current = midpoint :: first, last :: call
    if first_raw.on_curve:
        current = first
    else:
        if last_raw.on_curve:
            current = last
    let start = current
    let mut index = 0
    while index < count:
        let raw = point_list_at_or_zero :: contour.points, index :: call
        let scaled = scaled_point :: scale, (raw.x, raw.y) :: call
        if raw.on_curve:
            out :: (line_segment :: current, scaled :: call) :: push
            current = scaled
            index += 1
        else:
            let mut next_index = index + 1
            if next_index >= count:
                next_index = 0
            let next_raw = point_list_at_or_zero :: contour.points, next_index :: call
            let next_scaled = scaled_point :: scale, (next_raw.x, next_raw.y) :: call
            if next_raw.on_curve:
                append_quad_segments :: out, current, scaled :: call
                    p2 = next_scaled
                current = next_scaled
                index += 2
            else:
                let mid = midpoint :: scaled, next_scaled :: call
                append_quad_segments :: out, current, scaled :: call
                    p2 = mid
                current = mid
                index += 1
    if current.0 != start.0 or current.1 != start.1:
        out :: (line_segment :: current, start :: call) :: push
    return out

fn outline_segments(read outline: arcana_text.font_leaf.GlyphOutline, read scale: arcana_text.font_leaf.ScaleContext) -> List[arcana_text.font_leaf.LineSegment]:
    let mut out = empty_segments :: :: call
    for contour in outline.contours:
        let contour_out = contour_segments :: contour, scale :: call
        out :: contour_out :: extend_list
    return out

fn sort_ints(read values: List[Int]) -> List[Int]:
    let mut sorted = empty_int_list :: :: call
    for value in values:
        let mut next = empty_int_list :: :: call
        let mut inserted = false
        for current in sorted:
            if not inserted and value < current:
                next :: value :: push
                inserted = true
            next :: current :: push
        if not inserted:
            next :: value :: push
        sorted = next
    return sorted

fn fill_bitmap_from_segments(read segments: List[arcana_text.font_leaf.LineSegment], width: Int, height: Int) -> Array[Int]:
    let mut alpha = empty_int_list :: :: call
    let mut y = 0
    while y < height:
        let mut intersections = empty_int_list :: :: call
        for segment in segments:
            let y0 = segment.start.1
            let y1 = segment.end.1
            if y0 == y1:
                continue
            let low = min_int :: y0, y1 :: call
            let high = max_int :: y0, y1 :: call
            if y < low or y >= high:
                continue
            let x = segment.start.0 + (((y - y0) * (segment.end.0 - segment.start.0)) / (y1 - y0))
            intersections :: x :: push
        let sorted = sort_ints :: intersections :: call
        let mut pair_open = false
        let mut left = 0
        let mut index = 0
        let mut x = 0
        while x < width:
            while index < (sorted :: :: len) and (int_list_at_or_zero :: sorted, index :: call) <= x:
                if not pair_open:
                    left = int_list_at_or_zero :: sorted, index :: call
                    pair_open = true
                else:
                    pair_open = false
                index += 1
            if pair_open and x >= left:
                alpha :: 255 :: push
            else:
                alpha :: 0 :: push
            x += 1
        y += 1
    return std.collections.array.from_list[Int] :: alpha :: call

fn embolden_alpha(read alpha: Array[Int], dims: (Int, Int), embolden_px: Int) -> Array[Int]:
    if embolden_px <= 0:
        return alpha
    let mut out = empty_int_list :: :: call
    let width = dims.0
    let height = dims.1
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let mut filled = false
            let mut dx = 0 - embolden_px
            while dx <= embolden_px and not filled:
                let probe = x + dx
                if probe >= 0 and probe < width:
                    if (byte_at_or_zero :: alpha, (y * width) + probe :: call) > 0:
                        filled = true
                dx += 1
            if filled:
                out :: 255 :: push
            else:
                out :: (byte_at_or_zero :: alpha, (y * width) + x :: call) :: push
            x += 1
        y += 1
    return std.collections.array.from_list[Int] :: out :: call

fn raster_key(read request: arcana_text.font_leaf.RasterKeyRequest) -> Str:
    return (std.text.from_int :: request.glyph_index :: call) + ":" + (std.text.from_int :: request.font_size :: call) + ":" + (std.text.from_int :: request.traits.weight :: call) + ":" + (std.text.from_int :: request.traits.width_milli :: call) + ":" + (std.text.from_int :: request.traits.slant_milli :: call) + ":" + (std.text.from_int :: request.feature_signature :: call) + ":" + (std.text.from_int :: request.axis_signature :: call)

export fn render_glyph(edit face: arcana_text.font_leaf.FontFaceState, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    let ch = spec.text
    let font_size = spec.font_size
    let line_height_milli = spec.line_height_milli
    let mut traits = spec.traits
    if traits.weight <= 0:
        traits.weight = face.weight
    if traits.width_milli <= 0:
        traits.width_milli = face.width_milli
    if traits.slant_milli == 0:
        traits.slant_milli = face.slant_milli
    let line_height = line_height_for :: face, font_size, line_height_milli :: call
    let baseline = baseline_for :: face, font_size, line_height_milli :: call
    let scale = scale_context :: face, traits, font_size :: call
    let mut glyph_index = spec.glyph_index
    if glyph_index < 0:
        glyph_index = glyph_index_for_codepoint :: face, (utf8_codepoint :: ch :: call) :: call
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_width = normalized_width_milli :: traits.width_milli :: call
    let safe_units_per_em = max_int :: face.units_per_em, 1 :: call
    let raw_advance = advance_width_for :: face, glyph_index :: call
    let advance = max_int :: ((raw_advance * safe_font_size * safe_width) / (safe_units_per_em * 100000)), 1 :: call
    let mut key_request = arcana_text.font_leaf.RasterKeyRequest :: glyph_index = glyph_index, font_size = font_size, traits = traits :: call
    key_request.feature_signature = spec.feature_signature
    key_request.axis_signature = spec.axis_signature
    let key = raster_key :: key_request :: call
    if face.bitmap_cache :: key :: has:
        return face.bitmap_cache :: key :: get
    if glyph_index <= 0 or ch == " ":
        let mut empty = arcana_text.font_leaf.GlyphBitmap :: size = (0, 0), offset = (0, 0), advance = advance :: call
        empty.baseline = baseline
        empty.line_height = line_height
        empty.empty = true
        empty.alpha = empty_alpha :: :: call
        face.bitmap_cache :: key, empty :: set
        return face.bitmap_cache :: key :: get
    ensure_loca_offsets :: face :: call
    let outline = load_outline_recursive :: face, glyph_index, 0 :: call
    if outline.empty:
        let mut empty = arcana_text.font_leaf.GlyphBitmap :: size = (0, 0), offset = (0, 0), advance = advance :: call
        empty.baseline = baseline
        empty.line_height = line_height
        empty.empty = true
        empty.alpha = empty_alpha :: :: call
        face.bitmap_cache :: key, empty :: set
        return face.bitmap_cache :: key :: get
    let scaled_min = scaled_point :: scale, (outline.x_min, outline.y_min) :: call
    let scaled_max = scaled_point :: scale, (outline.x_max, outline.y_max) :: call
    let min_x = min_int :: scaled_min.0, scaled_max.0 :: call
    let min_y = min_int :: scaled_min.1, scaled_max.1 :: call
    let max_x = max_int :: scaled_min.0, scaled_max.0 :: call
    let max_y = max_int :: scaled_min.1, scaled_max.1 :: call
    let left = min_x
    let top = baseline - max_y
    let width = max_int :: (max_x - min_x + 1), 1 :: call
    let height = max_int :: (max_y - min_y + 1), 1 :: call
    let mut translated = empty_segments :: :: call
    let raster_segments = outline_segments :: outline, scale :: call
    for segment in raster_segments:
        translated :: (line_segment :: (segment.start.0 - left, max_y - segment.start.1), (segment.end.0 - left, max_y - segment.end.1) :: call) :: push
    let base_alpha = fill_bitmap_from_segments :: translated, width, height :: call
    let embolden_px = max_int :: ((traits.weight - 400) / 250), 0 :: call
    let alpha = embolden_alpha :: base_alpha, (width, height), embolden_px :: call
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (left, top), advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = alpha
    face.bitmap_cache :: key, out :: set
    return face.bitmap_cache :: key :: get

export fn measure_glyph(edit face: arcana_text.font_leaf.FontFaceState, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    let mut traits = spec.traits
    if traits.weight <= 0:
        traits.weight = face.weight
    if traits.width_milli <= 0:
        traits.width_milli = face.width_milli
    if traits.slant_milli == 0:
        traits.slant_milli = face.slant_milli
    let line_height = line_height_for :: face, spec.font_size, spec.line_height_milli :: call
    let baseline = baseline_for :: face, spec.font_size, spec.line_height_milli :: call
    let mut glyph_index = spec.glyph_index
    if glyph_index < 0:
        glyph_index = glyph_index_for_codepoint :: face, (utf8_codepoint :: spec.text :: call) :: call
    let safe_font_size = max_int :: spec.font_size, 1 :: call
    let safe_width = normalized_width_milli :: traits.width_milli :: call
    let safe_units_per_em = max_int :: face.units_per_em, 1 :: call
    let raw_advance = advance_width_for :: face, glyph_index :: call
    let advance = max_int :: ((raw_advance * safe_font_size * safe_width) / (safe_units_per_em * 100000)), 1 :: call
    let mut bitmap = arcana_text.font_leaf.GlyphBitmap :: size = (0, 0), offset = (0, 0), advance = advance :: call
    bitmap.baseline = baseline
    bitmap.line_height = line_height
    bitmap.empty = true
    bitmap.alpha = empty_alpha :: :: call
    return bitmap

export fn line_height_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    return line_height_for :: face, font_size, line_height_milli :: call

export fn baseline_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    return baseline_for :: face, font_size, line_height_milli :: call

fn load_face_from_parts(read meta: arcana_text.font_leaf.FaceLoadMeta, read bytes_view: std.memory.ByteView) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    let family_name = meta.family_name
    let source_label = meta.source_label
    let source_path = meta.source_path
    let traits = meta.traits
    let tables_result = parse_table_directory_ref :: bytes_view :: call
    if tables_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: tables_result, "font parse failed" :: call) :: call
    let tables = tables_result :: (empty_tables :: :: call) :: unwrap_or
    let head_result = read_table_offset :: tables, "head" :: call
    if head_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: head_result, "font is missing `head` table" :: call) :: call
    let head = head_result :: (0, 0) :: unwrap_or
    let maxp_result = read_table_offset :: tables, "maxp" :: call
    if maxp_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: maxp_result, "font is missing `maxp` table" :: call) :: call
    let maxp = maxp_result :: (0, 0) :: unwrap_or
    let hhea_result = read_table_offset :: tables, "hhea" :: call
    if hhea_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: hhea_result, "font is missing `hhea` table" :: call) :: call
    let hhea = hhea_result :: (0, 0) :: unwrap_or
    let hmtx_result = read_table_offset :: tables, "hmtx" :: call
    if hmtx_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: hmtx_result, "font is missing `hmtx` table" :: call) :: call
    let hmtx = hmtx_result :: (0, 0) :: unwrap_or
    let cmap_result = read_table_offset :: tables, "cmap" :: call
    if cmap_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: cmap_result, "font is missing `cmap` table" :: call) :: call
    let cmap = cmap_result :: (0, 0) :: unwrap_or
    let loca_result = read_table_offset :: tables, "loca" :: call
    if loca_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: loca_result, "font is missing `loca` table" :: call) :: call
    let loca = loca_result :: (0, 0) :: unwrap_or
    let glyf_result = read_table_offset :: tables, "glyf" :: call
    if glyf_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: glyf_result, "font is missing `glyf` table" :: call) :: call
    let glyf = glyf_result :: (0, 0) :: unwrap_or
    let glyph_count = u16_be_ref :: bytes_view, maxp.0 + 4 :: call
    let hmetric_count = u16_be_ref :: bytes_view, hhea.0 + 34 :: call
    let metrics_result = parse_h_metrics_ref :: bytes_view, (hhea, hmtx), glyph_count :: call
    if metrics_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: metrics_result, "font has invalid `hmtx` metrics" :: call) :: call
    let metrics = metrics_result :: (empty_metrics_pair :: :: call) :: unwrap_or
    let cmap_offsets = detect_cmap_offsets_ref :: bytes_view, cmap :: call
    let mut face = arcana_text.font_leaf.FontFaceState :: family_name = family_name, source_label = source_label, source_path = source_path :: call
    face.weight = traits.weight
    face.width_milli = traits.width_milli
    face.slant_milli = traits.slant_milli
    face.units_per_em = u16_be_ref :: bytes_view, head.0 + 18 :: call
    face.ascender = i16_be_ref :: bytes_view, hhea.0 + 4 :: call
    face.descender = i16_be_ref :: bytes_view, hhea.0 + 6 :: call
    face.line_gap = i16_be_ref :: bytes_view, hhea.0 + 8 :: call
    face.glyph_count = glyph_count
    face.font_view = bytes_view
    face.font_bytes = empty_alpha :: :: call
    face.glyf_offset = glyf.0
    face.loca_offset = loca.0
    face.loca_format = i16_be_ref :: bytes_view, head.0 + 50 :: call
    face.hmtx_offset = hmtx.0
    face.hmetric_count = hmetric_count
    face.advance_widths = metrics.0
    face.left_side_bearings = metrics.1
    face.loca_offsets = empty_alpha :: :: call
    face.glyph_index_cache = empty_int_map :: :: call
    face.cmap4_offset = cmap_offsets.0
    face.cmap12_offset = cmap_offsets.1
    face.cmap4_segments = empty_cmap4_segments :: :: call
    face.cmap4_glyphs = empty_alpha :: :: call
    face.cmap12_groups = empty_cmap12_groups :: :: call
    face.bitmap_cache = std.collections.map.new[Str, arcana_text.font_leaf.GlyphBitmap] :: :: call
    return Result.Ok[arcana_text.font_leaf.FontFaceState, Str] :: face :: call

export fn load_face_from_view(read request: arcana_text.font_leaf.FaceLoadRequest, read bytes_view: std.memory.ByteView) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    let mut meta = face_load_meta :: request.family_name, request.source_label, request.source_path :: call
    meta.traits = request.traits
    return load_face_from_parts :: meta, bytes_view :: call

export fn load_face_from_bytes(read request: arcana_text.font_leaf.FaceLoadRequest) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    let mut meta = face_load_meta :: request.family_name, request.source_label, request.source_path :: call
    meta.traits = request.traits
    let view = std.memory.bytes_view :: request.font_bytes, 0, (request.font_bytes :: :: len) :: call
    return load_face_from_parts :: meta, view :: call

export fn load_face_from_path(family_name: Str, path: Str, read traits: arcana_text.font_leaf.FaceTraits) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    let bytes_result = std.fs.read_bytes :: path :: call
    if bytes_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: bytes_result, "failed to read font file" :: call) :: call
    let bytes = bytes_result :: (empty_alpha :: :: call) :: unwrap_or
    let mut meta = face_load_meta :: family_name, (safe_file_stem :: path :: call), path :: call
    meta.traits = traits
    let view = std.memory.bytes_view :: bytes, 0, (bytes :: :: len) :: call
    return load_face_from_parts :: meta, view :: call
