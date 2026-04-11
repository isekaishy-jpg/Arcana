import arcana_text.types
import arcana_text.text_units
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

fn font_leaf_probe_flag_path() -> Str:
    return std.path.join :: (std.path.join :: (std.path.cwd :: :: call), "scratch" :: call), "enable_text_fonts_probe" :: call

fn font_leaf_probe_log_path() -> Str:
    return std.path.join :: (std.path.join :: (std.path.cwd :: :: call), "scratch" :: call), "text_font_leaf_probe.log" :: call

fn font_leaf_probe_enabled() -> Bool:
    return std.fs.is_file :: (font_leaf_probe_flag_path :: :: call) :: call

fn font_leaf_probe_append(line: Str):
    if not (font_leaf_probe_enabled :: :: call):
        return
    let _ = std.fs.mkdir_all :: (std.path.parent :: (font_leaf_probe_log_path :: :: call) :: call) :: call
    let opened = std.fs.stream_open_write :: (font_leaf_probe_log_path :: :: call), true :: call
    return match opened:
        Result.Ok(value) => font_leaf_probe_append_ready :: value, line :: call
        Result.Err(_) => 0

fn font_leaf_probe_append_ready(take value: std.fs.FileStream, line: Str):
    let mut stream = value
    let bytes = std.bytes.from_str_utf8 :: (line + "\n") :: call
    let _ = std.fs.stream_write :: stream, bytes :: call
    let _ = std.fs.stream_close :: stream :: call

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

record GlyphComponent:
    glyph_index: Int
    matrix: arcana_text.font_leaf.AffineMatrix
    offset: (Int, Int)

record CoordPoint:
    x: Int
    y: Int

record PointDelta:
    x: Int
    y: Int
    explicit: Bool

record VariationAxisDef:
    tag: Str
    min_value: Int
    default_value: Int
    max_value: Int

record GlyphOutline:
    advance_width: Int
    left_side_bearing: Int
    x_min: Int
    y_min: Int
    x_max: Int
    y_max: Int
    contours: List[arcana_text.font_leaf.FontContour]
    components: List[arcana_text.font_leaf.GlyphComponent]
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
    lcd: Array[Int]
    rgba: Array[Int]

record DecodedColorImage:
    size: (Int, Int)
    rgba: Array[Int]

record EmbeddedBitmapMetrics:
    offset: (Int, Int)
    advance: Int

record EmbeddedBitmapImage:
    format_tag: Str
    bytes: Array[Int]
    metrics: arcana_text.font_leaf.EmbeddedBitmapMetrics
    draw_outline: Bool
    bottom_origin: Bool

record DeflateBitReader:
    bytes: Array[Int]
    cursor: Int
    bit_value: Int
    bit_count: Int

record HuffmanTable:
    lengths: Array[Int]
    codes: Array[Int]
    max_bits: Int

export record GlyphRenderSpec:
    text: Str
    glyph_index: Int
    font_size: Int
    line_height_milli: Int
    traits: arcana_text.font_leaf.FaceTraits
    feature_signature: Int
    axis_signature: Int
    vertical: Bool
    mode: arcana_text.types.RasterMode
    color: Int
    hinting: arcana_text.types.Hinting

export record GsubGlyphUnit:
    glyph_index: Int
    consumed: Int

export record SourceFaceMetadata:
    face_index: Int
    family_name: Str
    face_name: Str
    full_name: Str
    postscript_name: Str
    traits: arcana_text.font_leaf.FaceTraits

export record FontFaceState:
    family_name: Str
    face_name: Str
    full_name: Str
    postscript_name: Str
    source_label: Str
    source_path: Str
    weight: Int
    width_milli: Int
    slant_milli: Int
    units_per_em: Int
    ascender: Int
    descender: Int
    line_gap: Int
    underline_position: Int
    underline_thickness: Int
    strike_position: Int
    strike_thickness: Int
    glyph_count: Int
    font_view: std.memory.ByteView
    glyf_offset: Int
    loca_offset: Int
    loca_format: Int
    hmtx_offset: Int
    hmetric_count: Int
    advance_widths: Array[Int]
    left_side_bearings: Array[Int]
    vhea_offset: Int
    vhea_length: Int
    vmtx_offset: Int
    vmtx_length: Int
    vmetric_count: Int
    vertical_advances: Array[Int]
    top_side_bearings: Array[Int]
    loca_offsets: Array[Int]
    glyph_index_cache: Map[Int, Int]
    cmap_table_offset: Int
    cmap_table_length: Int
    cmap4_offset: Int
    cmap12_offset: Int
    cmap4_segments: List[arcana_text.font_leaf.Cmap4Segment]
    cmap4_glyphs: Array[Int]
    cmap12_groups: List[arcana_text.font_leaf.Cmap12Group]
    colr_offset: Int
    colr_length: Int
    cpal_offset: Int
    cpal_length: Int
    cblc_offset: Int
    cblc_length: Int
    cbdt_offset: Int
    cbdt_length: Int
    sbix_flags: Int
    sbix_offset: Int
    sbix_length: Int
    svg_offset: Int
    svg_length: Int
    gdef_offset: Int
    gdef_length: Int
    gsub_offset: Int
    gsub_length: Int
    gpos_offset: Int
    gpos_length: Int
    kern_offset: Int
    kern_length: Int
    gsub_lookup_cache: Map[Str, List[arcana_text.font_leaf.GsubLookupRef]]
    gsub_candidate_cache: Map[Str, Bool]
    gpos_lookup_cache: Map[Str, List[arcana_text.font_leaf.GsubLookupRef]]
    gpos_single_cache: Map[Str, arcana_text.font_leaf.PairPlacement]
    gpos_pair_cache: Map[Str, arcana_text.font_leaf.PairPlacement]
    kern_pair_cache: Map[Str, arcana_text.font_leaf.PairPlacement]
    variation_axes: List[arcana_text.font_leaf.VariationAxisDef]
    fvar_offset: Int
    fvar_length: Int
    gvar_offset: Int
    gvar_length: Int
    gvar_axis_count: Int
    gvar_shared_tuple_count: Int
    gvar_shared_tuples_offset: Int
    gvar_glyph_count: Int
    gvar_flags: Int
    gvar_data_offset: Int
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
    face_index: Int
    source_bytes: Array[Int]
    traits: arcana_text.font_leaf.FaceTraits

record FaceLoadMeta:
    family_name: Str
    source_label: Str
    source_path: Str
    traits: arcana_text.font_leaf.FaceTraits

record CoordinateDecodeSpec:
    bytes: std.memory.ByteView
    cursor: Int
    flags: Array[Int]
    count: Int
    short_mask: Int
    same_mask: Int

record Cmap4Lookup:
    segments: List[arcana_text.font_leaf.Cmap4Segment]
    glyphs: Array[Int]
    codepoint: Int

record NameDecodeRequest:
    start: Int
    length: Int
    platform_id: Int

record FaceTraitsRequest:
    tables: Map[Str, (Int, Int)]
    fallback_traits: arcana_text.font_leaf.FaceTraits
    head: (Int, Int)

record InterpolateSegmentRequest:
    coords: List[arcana_text.font_leaf.CoordPoint]
    left_coord: arcana_text.font_leaf.CoordPoint
    left_delta: arcana_text.font_leaf.PointDelta
    right_coord: arcana_text.font_leaf.CoordPoint
    right_delta: arcana_text.font_leaf.PointDelta

record ApplyGvarRequest:
    glyph_index: Int
    coords: List[arcana_text.font_leaf.CoordPoint]
    end_points: List[Int]
    traits: arcana_text.font_leaf.FaceTraits

record TupleScalarRegion:
    start: List[Int]
    peak: List[Int]
    end: List[Int]

record ResolveCompoundRequest:
    coords: List[arcana_text.font_leaf.CoordPoint]
    traits: arcana_text.font_leaf.FaceTraits
    depth: Int

export record SourceFaceMetadataRequest:
    face_index: Int
    fallback_family: Str
    fallback_label: Str
    fallback_traits: arcana_text.font_leaf.FaceTraits

record ScaleContext:
    font_size: Int
    units_per_em: Int
    width_milli: Int
    slant_milli: Int

record RasterKeyRequest:
    glyph_index: Int
    font_size: Int
    traits: arcana_text.font_leaf.FaceTraits
    feature_signature: Int
    axis_signature: Int
    vertical: Bool
    mode: arcana_text.types.RasterMode
    color: Int
    hinting: arcana_text.types.Hinting

record AppendFeatureLookupRefsRequest:
    bytes: std.memory.ByteView
    feature_list_offset: Int
    feature_index: Int
    feature_value: Int

record LookupRefsForFeaturesRequest:
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]

record SubtableMatchRequest:
    bytes: std.memory.ByteView
    lookup_type: Int
    subtable_offset: Int
    units: List[arcana_text.font_leaf.GsubGlyphUnit]
    index: Int
    feature_value: Int

record AlternateSubstitutionMatchRequest:
    bytes: std.memory.ByteView
    subtable_offset: Int
    glyph_index: Int
    feature_value: Int

record LigatureSubstitutionMatchRequest:
    bytes: std.memory.ByteView
    subtable_offset: Int
    units: List[arcana_text.font_leaf.GsubGlyphUnit]
    index: Int

record LookupMatchRequest:
    bytes: std.memory.ByteView
    lookup_list_offset: Int
    lookup_index: Int
    units: List[arcana_text.font_leaf.GsubGlyphUnit]
    index: Int
    feature_value: Int

record ApplyLookupRequest:
    bytes: std.memory.ByteView
    lookup_list_offset: Int
    lookup: arcana_text.font_leaf.GsubLookupRef
    units: List[arcana_text.font_leaf.GsubGlyphUnit]

record LookupApplyAtRequest:
    bytes: std.memory.ByteView
    lookup_list_offset: Int
    lookup_index: Int
    units: List[arcana_text.font_leaf.GsubGlyphUnit]
    index: Int
    feature_value: Int
    depth: Int

record LookupApplyResult:
    units: List[arcana_text.font_leaf.GsubGlyphUnit]
    matched: Bool
    advance: Int

record PairAdjustmentRequest:
    left_glyph: Int
    right_glyph: Int
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    traits: arcana_text.font_leaf.FaceTraits
    font_size: Int

record SingleAdjustmentRequest:
    glyph: Int
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    traits: arcana_text.font_leaf.FaceTraits
    font_size: Int

export record LookupPlacementRequest:
    lookup: arcana_text.font_leaf.GsubLookupRef
    left_glyph: Int
    right_glyph: Int
    traits: arcana_text.font_leaf.FaceTraits
    font_size: Int

export record PairPlacement:
    x_offset: Int
    y_offset: Int
    x_advance: Int
    y_advance: Int
    zero_advance: Bool
    attach_to_left_origin: Bool

record ColrLayerRecord:
    glyph_index: Int
    palette_index: Int

record ColorLayerBitmap:
    bitmap: arcana_text.font_leaf.GlyphBitmap
    color: (Int, Int, Int, Int)

record ColrColorStop:
    offset: Int
    color: (Int, Int, Int, Int)

record ColrColorLine:
    extend: Int
    stops: List[arcana_text.font_leaf.ColrColorStop]

record SvgNumberRead:
    value: Int
    next: Int
    percent: Bool
    ok: Bool

record SvgViewBox:
    min_x: Int
    min_y: Int
    width: Int
    height: Int

record AnchorPoint:
    x: Int
    y: Int
    valid: Bool

record PairPositionSubtableRequest:
    bytes: std.memory.ByteView
    subtable_offset: Int
    left_glyph: Int
    right_glyph: Int

record SinglePositionSubtableRequest:
    bytes: std.memory.ByteView
    subtable_offset: Int
    glyph: Int

record PositionLookupAdjustRequest:
    bytes: std.memory.ByteView
    lookup_list_offset: Int
    lookup: arcana_text.font_leaf.GsubLookupRef
    left_glyph: Int
    right_glyph: Int

export record GsubSubstituteRequest:
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    glyphs: List[Int]

record GsubLookupRef:
    lookup_index: Int
    feature_value: Int

record GsubMatch:
    glyph_index: Int
    sequence_consumed: Int
    found: Bool

fn empty_points() -> List[arcana_text.font_leaf.FontPoint]:
    return std.collections.list.new[arcana_text.font_leaf.FontPoint] :: :: call

fn empty_contours() -> List[arcana_text.font_leaf.FontContour]:
    return std.collections.list.new[arcana_text.font_leaf.FontContour] :: :: call

fn empty_components() -> List[arcana_text.font_leaf.GlyphComponent]:
    return std.collections.list.new[arcana_text.font_leaf.GlyphComponent] :: :: call

fn empty_coord_points() -> List[arcana_text.font_leaf.CoordPoint]:
    return std.collections.list.new[arcana_text.font_leaf.CoordPoint] :: :: call

fn empty_point_deltas() -> List[arcana_text.font_leaf.PointDelta]:
    return std.collections.list.new[arcana_text.font_leaf.PointDelta] :: :: call

fn empty_segments() -> List[arcana_text.font_leaf.LineSegment]:
    return std.collections.list.new[arcana_text.font_leaf.LineSegment] :: :: call

fn empty_cmap4_segments() -> List[arcana_text.font_leaf.Cmap4Segment]:
    return std.collections.list.new[arcana_text.font_leaf.Cmap4Segment] :: :: call

fn empty_cmap12_groups() -> List[arcana_text.font_leaf.Cmap12Group]:
    return std.collections.list.new[arcana_text.font_leaf.Cmap12Group] :: :: call

fn empty_variation_axes() -> List[arcana_text.font_leaf.VariationAxisDef]:
    return std.collections.list.new[arcana_text.font_leaf.VariationAxisDef] :: :: call

fn empty_gsub_units() -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    return std.collections.list.new[arcana_text.font_leaf.GsubGlyphUnit] :: :: call

fn empty_lookup_refs() -> List[arcana_text.font_leaf.GsubLookupRef]:
    return std.collections.list.new[arcana_text.font_leaf.GsubLookupRef] :: :: call

fn empty_lookup_ref_cache() -> Map[Str, List[arcana_text.font_leaf.GsubLookupRef]]:
    return std.collections.map.new[Str, List[arcana_text.font_leaf.GsubLookupRef]] :: :: call

fn empty_pair_adjust_cache() -> Map[Str, arcana_text.font_leaf.PairPlacement]:
    return std.collections.map.new[Str, arcana_text.font_leaf.PairPlacement] :: :: call

fn empty_bool_cache() -> Map[Str, Bool]:
    return std.collections.map.new[Str, Bool] :: :: call

fn empty_int_list() -> List[Int]:
    return std.collections.list.new[Int] :: :: call

fn empty_alpha() -> Array[Int]:
    return std.collections.array.from_list[Int] :: (empty_int_list :: :: call) :: call

fn empty_lcd() -> Array[Int]:
    return std.collections.array.from_list[Int] :: (empty_int_list :: :: call) :: call

fn empty_rgba() -> Array[Int]:
    return std.collections.array.from_list[Int] :: (empty_int_list :: :: call) :: call

fn empty_pair_placement() -> arcana_text.font_leaf.PairPlacement:
    let mut out = arcana_text.font_leaf.PairPlacement :: x_offset = 0, y_offset = 0, x_advance = 0 :: call
    out.y_advance = 0
    out.zero_advance = false
    out.attach_to_left_origin = false
    return out

fn invalid_anchor_point() -> arcana_text.font_leaf.AnchorPoint:
    let mut out = arcana_text.font_leaf.AnchorPoint :: x = 0, y = 0 :: call
    out.valid = false
    return out

fn placement_has_effect(read value: arcana_text.font_leaf.PairPlacement) -> Bool:
    return value.zero_advance or value.attach_to_left_origin or value.x_offset != 0 or value.y_offset != 0 or value.x_advance != 0 or value.y_advance != 0

fn point_zero() -> arcana_text.font_leaf.FontPoint:
    return arcana_text.font_leaf.FontPoint :: x = 0, y = 0, on_curve = true :: call

fn coord_point(x: Int, y: Int) -> arcana_text.font_leaf.CoordPoint:
    return arcana_text.font_leaf.CoordPoint :: x = x, y = y :: call

fn coord_zero() -> arcana_text.font_leaf.CoordPoint:
    return arcana_text.font_leaf.CoordPoint :: x = 0, y = 0 :: call

fn point_delta(x: Int, y: Int, explicit: Bool) -> arcana_text.font_leaf.PointDelta:
    return arcana_text.font_leaf.PointDelta :: x = x, y = y, explicit = explicit :: call

fn delta_zero() -> arcana_text.font_leaf.PointDelta:
    return arcana_text.font_leaf.PointDelta :: x = 0, y = 0, explicit = false :: call

fn variation_axis(tag: Str, bounds: (Int, Int), max_value: Int) -> arcana_text.font_leaf.VariationAxisDef:
    let mut axis = arcana_text.font_leaf.VariationAxisDef :: tag = tag, min_value = bounds.0, default_value = bounds.1 :: call
    axis.max_value = max_value
    return axis

fn glyph_component(glyph_index: Int, read matrix: arcana_text.font_leaf.AffineMatrix, offset: (Int, Int)) -> arcana_text.font_leaf.GlyphComponent:
    return arcana_text.font_leaf.GlyphComponent :: glyph_index = glyph_index, matrix = matrix, offset = offset :: call

fn empty_tables() -> Map[Str, (Int, Int)]:
    return std.collections.map.new[Str, (Int, Int)] :: :: call

fn empty_metrics_pair() -> (Array[Int], Array[Int]):
    return ((empty_alpha :: :: call), (empty_alpha :: :: call))

fn gsub_match(glyph_index: Int, sequence_consumed: Int, found: Bool) -> arcana_text.font_leaf.GsubMatch:
    return arcana_text.font_leaf.GsubMatch :: glyph_index = glyph_index, sequence_consumed = sequence_consumed, found = found :: call

fn copy_gsub_units(read units: List[arcana_text.font_leaf.GsubGlyphUnit]) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let mut out = empty_gsub_units :: :: call
    out :: units :: extend_list
    return out

fn copy_lookup_refs(read refs: List[arcana_text.font_leaf.GsubLookupRef]) -> List[arcana_text.font_leaf.GsubLookupRef]:
    let mut out = empty_lookup_refs :: :: call
    out :: refs :: extend_list
    return out

fn lookup_apply_result(read units: List[arcana_text.font_leaf.GsubGlyphUnit], matched: Bool, advance: Int) -> arcana_text.font_leaf.LookupApplyResult:
    return arcana_text.font_leaf.LookupApplyResult :: units = units, matched = matched, advance = advance :: call

fn lookup_apply_no_match() -> arcana_text.font_leaf.LookupApplyResult:
    return arcana_text.font_leaf.lookup_apply_result :: (empty_gsub_units :: :: call), false, 1 :: call

fn lookup_apply_depth_limit() -> Int:
    return 32

export fn default_traits() -> arcana_text.font_leaf.FaceTraits:
    return arcana_text.font_leaf.FaceTraits :: weight = 400, width_milli = 100000, slant_milli = 0 :: call

export fn glyph_render_spec(text: Str, font_size: Int, line_height_milli: Int) -> arcana_text.font_leaf.GlyphRenderSpec:
    let mut out = arcana_text.font_leaf.GlyphRenderSpec :: text = text, glyph_index = -1, font_size = font_size :: call
    out.line_height_milli = line_height_milli
    out.traits = default_traits :: :: call
    out.feature_signature = 0
    out.axis_signature = 0
    out.vertical = false
    out.mode = arcana_text.types.RasterMode.Alpha :: :: call
    out.color = 0
    out.hinting = arcana_text.types.Hinting.Disabled :: :: call
    return out

export fn face_load_request(family_name: Str, source_label: Str, source_path: Str) -> arcana_text.font_leaf.FaceLoadRequest:
    let mut out = arcana_text.font_leaf.FaceLoadRequest :: family_name = family_name, source_label = source_label, source_path = source_path :: call
    out.face_index = 0
    out.source_bytes = empty_alpha :: :: call
    out.traits = default_traits :: :: call
    return out

fn face_load_meta(family_name: Str, source_label: Str, source_path: Str) -> arcana_text.font_leaf.FaceLoadMeta:
    let mut out = arcana_text.font_leaf.FaceLoadMeta :: family_name = family_name, source_label = source_label, source_path = source_path :: call
    out.traits = default_traits :: :: call
    return out

fn face_load_meta_from_request(read request: arcana_text.font_leaf.FaceLoadRequest) -> arcana_text.font_leaf.FaceLoadMeta:
    let mut out = face_load_meta :: request.family_name, request.source_label, request.source_path :: call
    out.traits = request.traits
    return out

fn request_source_view(read request: arcana_text.font_leaf.FaceLoadRequest) -> Result[std.memory.ByteView, Str]:
    if (request.source_bytes :: :: len) <= 0:
        return Result.Err[std.memory.ByteView, Str] :: "font source bytes are empty" :: call
    return Result.Ok[std.memory.ByteView, Str] :: (std.memory.bytes_view :: request.source_bytes, 0, (request.source_bytes :: :: len) :: call) :: call

fn request_face_view(read request: arcana_text.font_leaf.FaceLoadRequest) -> Result[std.memory.ByteView, Str]:
    let source_view_result = arcana_text.font_leaf.request_source_view :: request :: call
    if source_view_result :: :: is_err:
        return Result.Err[std.memory.ByteView, Str] :: (result_err_or :: source_view_result, "font source bytes are empty" :: call) :: call
    let source_view = source_view_result :: (std.memory.bytes_view :: request.source_bytes, 0, (request.source_bytes :: :: len) :: call) :: unwrap_or
    if request.face_index == 0:
        return Result.Ok[std.memory.ByteView, Str] :: source_view :: call
    let face_view_result = arcana_text.font_leaf.face_view_from_source :: source_view, request.face_index :: call
    if face_view_result :: :: is_err:
        return Result.Err[std.memory.ByteView, Str] :: (result_err_or :: face_view_result, "failed to resolve face view" :: call) :: call
    return Result.Ok[std.memory.ByteView, Str] :: (face_view_result :: source_view :: unwrap_or) :: call

fn face_source_offset_from_source(read bytes: std.memory.ByteView, face_index: Int) -> Result[Int, Str]:
    let offsets_result = arcana_text.font_leaf.face_offsets_from_view :: bytes :: call
    if offsets_result :: :: is_err:
        return Result.Err[Int, Str] :: (result_err_or :: offsets_result, "failed to resolve source faces" :: call) :: call
    let offsets = offsets_result :: (empty_int_list :: :: call) :: unwrap_or
    let total = offsets :: :: len
    if face_index < 0 or face_index >= total:
        return Result.Err[Int, Str] :: ("font face index `" + (std.text.from_int :: face_index :: call) + "` is out of range") :: call
    return Result.Ok[Int, Str] :: (int_list_at_or_zero :: offsets, face_index :: call) :: call

fn cmap_state(read segments: List[arcana_text.font_leaf.Cmap4Segment], read glyphs: Array[Int], read groups: List[arcana_text.font_leaf.Cmap12Group]) -> arcana_text.font_leaf.CmapState:
    return arcana_text.font_leaf.CmapState :: segments = segments, glyphs = glyphs, groups = groups :: call

fn affine_matrix(xx: Int, xy: Int, axes: (Int, Int)) -> arcana_text.font_leaf.AffineMatrix:
    let mut out = arcana_text.font_leaf.AffineMatrix :: xx = xx, xy = xy, yx = axes.0 :: call
    out.yx = axes.0
    out.yy = axes.1
    return out

fn coordinate_decode_spec(read bytes: std.memory.ByteView, cursor: Int, read flags: Array[Int]) -> arcana_text.font_leaf.CoordinateDecodeSpec:
    let mut out = arcana_text.font_leaf.CoordinateDecodeSpec :: bytes = bytes, cursor = cursor, flags = flags :: call
    out.count = 0
    out.short_mask = 0
    out.same_mask = 0
    return out

fn cmap4_lookup(read segments: List[arcana_text.font_leaf.Cmap4Segment], read glyphs: Array[Int], codepoint: Int) -> arcana_text.font_leaf.Cmap4Lookup:
    return arcana_text.font_leaf.Cmap4Lookup :: segments = segments, glyphs = glyphs, codepoint = codepoint :: call

fn empty_int_map() -> Map[Int, Int]:
    return std.collections.map.new[Int, Int] :: :: call

fn metric_cache_widths(read bytes: Array[Int], read metrics: ((Int, Int), Int)) -> Array[Int]:
    let hmtx_offset = metrics.0.0
    let hmetric_count = metrics.0.1
    let glyph_count = metrics.1
    if hmetric_count <= 0 or glyph_count <= 0:
        return empty_alpha :: :: call
    let safe_hmetric_count = min_int :: hmetric_count, glyph_count :: call
    let mut out = empty_int_list :: :: call
    let mut index = 0
    while index < glyph_count:
        let metric_index = min_int :: index, safe_hmetric_count - 1 :: call
        let advance = u16_be :: bytes, hmtx_offset + (metric_index * 4) :: call
        out :: advance :: push
        index += 1
    return std.collections.array.from_list[Int] :: out :: call

fn metric_cache_left_side_bearings(read bytes: Array[Int], read metrics: ((Int, Int), Int)) -> Array[Int]:
    let hmtx_offset = metrics.0.0
    let hmetric_count = metrics.0.1
    let glyph_count = metrics.1
    if hmetric_count <= 0 or glyph_count <= 0:
        return empty_alpha :: :: call
    let safe_hmetric_count = min_int :: hmetric_count, glyph_count :: call
    let mut out = empty_int_list :: :: call
    let mut index = 0
    while index < glyph_count:
        let bearing = match index < safe_hmetric_count:
            true => i16_be :: bytes, hmtx_offset + (index * 4) + 2 :: call
            false => i16_be :: bytes, hmtx_offset + (safe_hmetric_count * 4) + ((index - safe_hmetric_count) * 2) :: call
        out :: bearing :: push
        index += 1
    return std.collections.array.from_list[Int] :: out :: call

fn metric_cache_vertical_advances(read bytes: Array[Int], read metrics: ((Int, Int), Int)) -> Array[Int]:
    let vmtx_offset = metrics.0.0
    let vmetric_count = metrics.0.1
    let glyph_count = metrics.1
    if vmetric_count <= 0 or glyph_count <= 0:
        return empty_alpha :: :: call
    let safe_vmetric_count = min_int :: vmetric_count, glyph_count :: call
    let mut out = empty_int_list :: :: call
    let mut index = 0
    while index < glyph_count:
        let metric_index = min_int :: index, safe_vmetric_count - 1 :: call
        let advance = u16_be :: bytes, vmtx_offset + (metric_index * 4) :: call
        out :: advance :: push
        index += 1
    return std.collections.array.from_list[Int] :: out :: call

fn metric_cache_top_side_bearings(read bytes: Array[Int], read metrics: ((Int, Int), Int)) -> Array[Int]:
    let vmtx_offset = metrics.0.0
    let vmetric_count = metrics.0.1
    let glyph_count = metrics.1
    if vmetric_count <= 0 or glyph_count <= 0:
        return empty_alpha :: :: call
    let safe_vmetric_count = min_int :: vmetric_count, glyph_count :: call
    let mut out = empty_int_list :: :: call
    let mut index = 0
    while index < glyph_count:
        let bearing = match index < safe_vmetric_count:
            true => i16_be :: bytes, vmtx_offset + (index * 4) + 2 :: call
            false => i16_be :: bytes, vmtx_offset + (safe_vmetric_count * 4) + ((index - safe_vmetric_count) * 2) :: call
        out :: bearing :: push
        index += 1
    return std.collections.array.from_list[Int] :: out :: call

fn metric_cache_loca_offsets(read bytes: Array[Int], read metrics: ((Int, Int), Int)) -> Array[Int]:
    let loca_offset = metrics.0.0
    let loca_format = metrics.0.1
    let glyph_count = metrics.1
    if glyph_count < 0:
        return empty_alpha :: :: call
    let mut out = empty_int_list :: :: call
    let total = glyph_count + 1
    let mut index = 0
    while index < total:
        let offset = match loca_format:
            0 => (u16_be :: bytes, loca_offset + (index * 2) :: call) * 2
            _ => u32_be :: bytes, loca_offset + (index * 4) :: call
        out :: offset :: push
        index += 1
    return std.collections.array.from_list[Int] :: out :: call

fn scale_context(read face: arcana_text.font_leaf.FontFaceState, read traits: arcana_text.font_leaf.FaceTraits, font_size: Int) -> arcana_text.font_leaf.ScaleContext:
    let mut out = arcana_text.font_leaf.ScaleContext :: font_size = font_size, units_per_em = face.units_per_em, width_milli = (effective_width_milli :: face, traits :: call) :: call
    out.slant_milli = effective_slant_milli :: face, traits :: call
    return out

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

fn coord_list_at_or_zero(read values: List[arcana_text.font_leaf.CoordPoint], target: Int) -> arcana_text.font_leaf.CoordPoint:
    let mut index = 0
    for value in values:
        if index == target:
            return value
        index += 1
    return coord_zero :: :: call

fn delta_list_at_or_zero(read values: List[arcana_text.font_leaf.PointDelta], target: Int) -> arcana_text.font_leaf.PointDelta:
    let mut index = 0
    for value in values:
        if index == target:
            return value
        index += 1
    return delta_zero :: :: call

fn i8(value: Int) -> Int:
    let raw = value % 256
    if raw >= 128:
        return raw - 256
    return raw

fn u16_be(read bytes: Array[Int], index: Int) -> Int:
    return (byte_at_or_zero :: bytes, index :: call) * 256 + (byte_at_or_zero :: bytes, index + 1 :: call)

fn u16_be_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    return (byte_at_or_zero_ref :: bytes, index :: call) * 256 + (byte_at_or_zero_ref :: bytes, index + 1 :: call)

fn u24_be_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    return ((byte_at_or_zero_ref :: bytes, index :: call) * 65536) + ((byte_at_or_zero_ref :: bytes, index + 1 :: call) * 256) + (byte_at_or_zero_ref :: bytes, index + 2 :: call)

fn u16_be_window(read bytes: std.memory.ByteView, index: Int) -> Int:
    let window = bytes :: index, index + 2 :: subview
    let raw = window :: :: to_array
    return u16_be :: raw, 0 :: call

fn u16_be_array_window(read bytes: Array[Int], index: Int) -> Int:
    let window = std.memory.bytes_view :: bytes, index, index + 2 :: call
    let raw = window :: :: to_array
    return u16_be :: raw, 0 :: call

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

fn i32_be(read bytes: Array[Int], index: Int) -> Int:
    let raw = u32_be :: bytes, index :: call
    if raw >= 2147483648:
        return raw - 4294967296
    return raw

fn u16_le(read bytes: Array[Int], index: Int) -> Int:
    return (byte_at_or_zero :: bytes, index :: call) + ((byte_at_or_zero :: bytes, index + 1 :: call) * 256)

fn u32_le(read bytes: Array[Int], index: Int) -> Int:
    return (byte_at_or_zero :: bytes, index :: call) + ((byte_at_or_zero :: bytes, index + 1 :: call) * 256) + ((byte_at_or_zero :: bytes, index + 2 :: call) * 65536) + ((byte_at_or_zero :: bytes, index + 3 :: call) * 16777216)

fn i32_le(read bytes: Array[Int], index: Int) -> Int:
    let raw = u32_le :: bytes, index :: call
    if raw >= 2147483648:
        return raw - 4294967296
    return raw

fn u32_be_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    return ((u16_be_ref :: bytes, index :: call) * 65536) + (u16_be_ref :: bytes, index + 2 :: call)

fn i32_be_ref(read bytes: std.memory.ByteView, index: Int) -> Int:
    let raw = u32_be_ref :: bytes, index :: call
    if raw >= 2147483648:
        return raw - 4294967296
    return raw

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

fn normalize_family_name(name: Str) -> Str:
    if std.text.ends_with :: name, " Var" :: call:
        return std.text.slice_bytes :: name, 0, (std.text.len_bytes :: name :: call) - 4 :: call
    return name

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
    font_leaf_probe_append :: "parse_table_directory:start" :: call
    let total = bytes :: :: len
    if total < 12:
        return Result.Err[Map[Str, (Int, Int)], Str] :: "font file is too small" :: call
    font_leaf_probe_append :: "parse_table_directory:scaler_value_start" :: call
    let scaler_value = u32_be :: bytes, 0 :: call
    font_leaf_probe_append :: "parse_table_directory:scaler_value_done" :: call
    font_leaf_probe_append :: "parse_table_directory:scaler_tag_start" :: call
    let scaler = tag_at :: bytes, 0 :: call
    font_leaf_probe_append :: "parse_table_directory:scaler_tag_done" :: call
    let is_true_type = scaler_value == 65536 or scaler == "true" or scaler == "OTTO"
    if not is_true_type:
        return Result.Err[Map[Str, (Int, Int)], Str] :: ("unsupported font scaler `" + scaler + "`") :: call
    font_leaf_probe_append :: "parse_table_directory:num_tables_start" :: call
    let num_tables = u16_be :: bytes, 4 :: call
    font_leaf_probe_append :: "parse_table_directory:num_tables_done" :: call
    font_leaf_probe_append :: ("parse_table_directory:num_tables=" + (std.text.from_int :: num_tables :: call)) :: call
    let mut tables = std.collections.map.new[Str, (Int, Int)] :: :: call
    let mut cursor = 12
    let mut index = 0
    while index < num_tables:
        font_leaf_probe_append :: ("parse_table_directory:record index=" + (std.text.from_int :: index :: call) + " cursor=" + (std.text.from_int :: cursor :: call)) :: call
        if cursor + 16 > total:
            return Result.Err[Map[Str, (Int, Int)], Str] :: "truncated table directory" :: call
        let tag = tag_at :: bytes, cursor :: call
        let offset = u32_be :: bytes, cursor + 8 :: call
        let length = u32_be :: bytes, cursor + 12 :: call
        tables :: tag, (offset, length) :: set
        cursor += 16
        index += 1
    font_leaf_probe_append :: "parse_table_directory:done" :: call
    return Result.Ok[Map[Str, (Int, Int)], Str] :: tables :: call

fn parse_table_directory_ref(read bytes: std.memory.ByteView) -> Result[Map[Str, (Int, Int)], Str]:
    font_leaf_probe_append :: "parse_table_directory_ref:start" :: call
    let total = bytes :: :: len
    if total < 12:
        return Result.Err[Map[Str, (Int, Int)], Str] :: "font file is too small" :: call
    font_leaf_probe_append :: "parse_table_directory_ref:scaler_value_start" :: call
    let scaler_value = u32_be_ref :: bytes, 0 :: call
    font_leaf_probe_append :: "parse_table_directory_ref:scaler_value_done" :: call
    font_leaf_probe_append :: "parse_table_directory_ref:scaler_tag_start" :: call
    let scaler = tag_at_ref :: bytes, 0 :: call
    font_leaf_probe_append :: "parse_table_directory_ref:scaler_tag_done" :: call
    let is_true_type = scaler_value == 65536 or scaler == "true" or scaler == "OTTO"
    if not is_true_type:
        return Result.Err[Map[Str, (Int, Int)], Str] :: ("unsupported font scaler `" + scaler + "`") :: call
    font_leaf_probe_append :: "parse_table_directory_ref:num_tables_start" :: call
    let num_tables = u16_be_ref :: bytes, 4 :: call
    font_leaf_probe_append :: "parse_table_directory_ref:num_tables_done" :: call
    font_leaf_probe_append :: ("parse_table_directory_ref:num_tables=" + (std.text.from_int :: num_tables :: call)) :: call
    let mut tables = std.collections.map.new[Str, (Int, Int)] :: :: call
    let mut cursor = 12
    let mut index = 0
    while index < num_tables:
        font_leaf_probe_append :: ("parse_table_directory_ref:record index=" + (std.text.from_int :: index :: call) + " cursor=" + (std.text.from_int :: cursor :: call)) :: call
        if cursor + 16 > total:
            return Result.Err[Map[Str, (Int, Int)], Str] :: "truncated table directory" :: call
        let tag = tag_at_ref :: bytes, cursor :: call
        let offset = u32_be_ref :: bytes, cursor + 8 :: call
        let length = u32_be_ref :: bytes, cursor + 12 :: call
        tables :: tag, (offset, length) :: set
        cursor += 16
        index += 1
    font_leaf_probe_append :: "parse_table_directory_ref:done" :: call
    return Result.Ok[Map[Str, (Int, Int)], Str] :: tables :: call

fn width_class_milli(width_class: Int) -> Int:
    return match width_class:
        1 => 50000
        2 => 62500
        3 => 75000
        4 => 87500
        6 => 112500
        7 => 125000
        8 => 150000
        9 => 200000
        _ => 100000

fn append_utf8_codepoint(edit buf: List[Int], codepoint: Int):
    let cp = clamp_int :: codepoint, 0, 1114111 :: call
    if cp < 128:
        let _ = std.bytes.buf_push :: buf, cp :: call
        return
    if cp < 2048:
        let _ = std.bytes.buf_push :: buf, (192 + (cp / 64)) :: call
        let _ = std.bytes.buf_push :: buf, (128 + (cp % 64)) :: call
        return
    if cp < 65536:
        let _ = std.bytes.buf_push :: buf, (224 + (cp / 4096)) :: call
        let _ = std.bytes.buf_push :: buf, (128 + ((cp / 64) % 64)) :: call
        let _ = std.bytes.buf_push :: buf, (128 + (cp % 64)) :: call
        return
    let _ = std.bytes.buf_push :: buf, (240 + (cp / 262144)) :: call
    let _ = std.bytes.buf_push :: buf, (128 + ((cp / 4096) % 64)) :: call
    let _ = std.bytes.buf_push :: buf, (128 + ((cp / 64) % 64)) :: call
    let _ = std.bytes.buf_push :: buf, (128 + (cp % 64)) :: call

fn utf16be_name_to_str(read bytes: std.memory.ByteView, start: Int, length: Int) -> Str:
    let total = bytes :: :: len
    let mut end = start + length
    if end > total:
        end = total
    let mut cursor = start
    let mut out = std.bytes.new_buf :: :: call
    while cursor + 1 < end:
        let first = u16_be_ref :: bytes, cursor :: call
        let mut codepoint = first
        cursor += 2
        if first >= 55296 and first <= 56319 and cursor + 1 < end:
            let second = u16_be_ref :: bytes, cursor :: call
            if second >= 56320 and second <= 57343:
                codepoint = 65536 + ((first - 55296) * 1024) + (second - 56320)
                cursor += 2
        arcana_text.font_leaf.append_utf8_codepoint :: out, codepoint :: call
    return std.bytes.to_str_utf8 :: (std.bytes.buf_to_array :: out :: call) :: call

fn latin_name_to_str(read bytes: std.memory.ByteView, start: Int, length: Int) -> Str:
    let total = bytes :: :: len
    let mut end = start + length
    if end > total:
        end = total
    let mut cursor = start
    let mut out = std.bytes.new_buf :: :: call
    while cursor < end:
        let value = byte_at_or_zero_ref :: bytes, cursor :: call
        if value >= 0 and value <= 127:
            let _ = std.bytes.buf_push :: out, value :: call
        else:
            arcana_text.font_leaf.append_utf8_codepoint :: out, 63 :: call
        cursor += 1
    return std.bytes.to_str_utf8 :: (std.bytes.buf_to_array :: out :: call) :: call

fn decode_name_string(read bytes: std.memory.ByteView, read request: arcana_text.font_leaf.NameDecodeRequest) -> Str:
    if request.platform_id == 0 or request.platform_id == 3:
        return utf16be_name_to_str :: bytes, request.start, request.length :: call
    return latin_name_to_str :: bytes, request.start, request.length :: call

fn name_record_score(platform_id: Int, language_id: Int) -> Int:
    if platform_id == 3:
        if language_id == 1033:
            return 500
        return 400
    if platform_id == 0:
        return 300
    if platform_id == 1:
        if language_id == 0:
            return 250
        return 200
    return 100

fn face_offsets_from_view(read bytes: std.memory.ByteView) -> Result[List[Int], Str]:
    let total = bytes :: :: len
    if total < 4:
        return Result.Err[List[Int], Str] :: "font file is too small" :: call
    let tag = tag_at_ref :: bytes, 0 :: call
    let mut offsets = empty_int_list :: :: call
    if tag != "ttcf":
        offsets :: 0 :: push
        return Result.Ok[List[Int], Str] :: offsets :: call
    if total < 12:
        return Result.Err[List[Int], Str] :: "truncated font collection header" :: call
    let count = u32_be_ref :: bytes, 8 :: call
    let mut index = 0
    while index < count:
        let record = 12 + (index * 4)
        if record + 4 > total:
            return Result.Err[List[Int], Str] :: "truncated font collection offsets" :: call
        let offset = u32_be_ref :: bytes, record :: call
        if offset < 0 or offset >= total:
            return Result.Err[List[Int], Str] :: "font collection face offset is out of bounds" :: call
        offsets :: offset :: push
        index += 1
    if offsets :: :: is_empty:
        offsets :: 0 :: push
    return Result.Ok[List[Int], Str] :: offsets :: call

export fn source_face_count_from_view(read bytes: std.memory.ByteView) -> Result[Int, Str]:
    let offsets = arcana_text.font_leaf.face_offsets_from_view :: bytes :: call
    if offsets :: :: is_err:
        return Result.Err[Int, Str] :: (result_err_or :: offsets, "failed to read face count" :: call) :: call
    return Result.Ok[Int, Str] :: ((offsets :: (empty_int_list :: :: call) :: unwrap_or) :: :: len) :: call

fn face_view_from_source(read bytes: std.memory.ByteView, face_index: Int) -> Result[std.memory.ByteView, Str]:
    let offsets_result = arcana_text.font_leaf.face_offsets_from_view :: bytes :: call
    if offsets_result :: :: is_err:
        return Result.Err[std.memory.ByteView, Str] :: (result_err_or :: offsets_result, "failed to resolve source faces" :: call) :: call
    let offsets = offsets_result :: (empty_int_list :: :: call) :: unwrap_or
    let total = offsets :: :: len
    if face_index < 0 or face_index >= total:
        return Result.Err[std.memory.ByteView, Str] :: ("font face index `" + (std.text.from_int :: face_index :: call) + "` is out of range") :: call
    let offset = int_list_at_or_zero :: offsets, face_index :: call
    if total == 1 and offset == 0:
        return Result.Ok[std.memory.ByteView, Str] :: bytes :: call
    let face_view = bytes :: offset, (bytes :: :: len) :: subview
    return Result.Ok[std.memory.ByteView, Str] :: face_view :: call

fn name_string_for_id(read bytes: std.memory.ByteView, name_table: (Int, Int), target_id: Int) -> Str:
    font_leaf_probe_append :: ("name_string_for_id:start id=" + (std.text.from_int :: target_id :: call)) :: call
    let table_end = name_table.0 + name_table.1
    if table_end > (bytes :: :: len):
        font_leaf_probe_append :: "name_string_for_id:table_end_oob" :: call
        return ""
    if name_table.1 < 6:
        font_leaf_probe_append :: "name_string_for_id:table_too_small" :: call
        return ""
    let count = u16_be_ref :: bytes, name_table.0 + 2 :: call
    let string_base = name_table.0 + (u16_be_ref :: bytes, name_table.0 + 4 :: call)
    font_leaf_probe_append :: ("name_string_for_id:count=" + (std.text.from_int :: count :: call) + " base=" + (std.text.from_int :: string_base :: call)) :: call
    let mut best_score = -1
    let mut best_value = ""
    let mut index = 0
    while index < count:
        let record = name_table.0 + 6 + (index * 12)
        if record + 12 > table_end:
            font_leaf_probe_append :: "name_string_for_id:record_oob" :: call
            return best_value
        let platform_id = u16_be_ref :: bytes, record :: call
        let language_id = u16_be_ref :: bytes, record + 4 :: call
        let name_id = u16_be_ref :: bytes, record + 6 :: call
        if name_id == target_id:
            let length = u16_be_ref :: bytes, record + 8 :: call
            let offset = u16_be_ref :: bytes, record + 10 :: call
            let start = string_base + offset
            if start >= name_table.0 and start + length <= table_end:
                let score = arcana_text.font_leaf.name_record_score :: platform_id, language_id :: call
                if score > best_score:
                    best_score = score
                    best_value = arcana_text.font_leaf.decode_name_string :: bytes, (arcana_text.font_leaf.NameDecodeRequest :: start = start, length = length, platform_id = platform_id :: call) :: call
        index += 1
    font_leaf_probe_append :: ("name_string_for_id:done id=" + (std.text.from_int :: target_id :: call)) :: call
    return best_value

fn traits_from_face_tables(read bytes: std.memory.ByteView, read request: arcana_text.font_leaf.FaceTraitsRequest) -> arcana_text.font_leaf.FaceTraits:
    let mut traits = request.fallback_traits
    if request.tables :: "OS/2" :: has:
        let os2 = request.tables :: "OS/2" :: get
        if os2.1 >= 8:
            let weight = u16_be_ref :: bytes, os2.0 + 4 :: call
            if weight > 0:
                traits.weight = weight
            traits.width_milli = arcana_text.font_leaf.width_class_milli :: (u16_be_ref :: bytes, os2.0 + 6 :: call) :: call
        if os2.1 >= 64:
            let fs_selection = u16_be_ref :: bytes, os2.0 + 62 :: call
            if (fs_selection % 2) == 1:
                traits.slant_milli = -12000
    if request.head.1 >= 46:
        let mac_style = u16_be_ref :: bytes, request.head.0 + 44 :: call
        if (mac_style / 2) % 2 == 1:
            traits.slant_milli = -12000
    return traits

export fn source_face_metadata_from_view(read bytes: std.memory.ByteView, read request: arcana_text.font_leaf.SourceFaceMetadataRequest) -> Result[arcana_text.font_leaf.SourceFaceMetadata, Str]:
    let face_view_result = arcana_text.font_leaf.face_view_from_source :: bytes, request.face_index :: call
    if face_view_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.SourceFaceMetadata, Str] :: (result_err_or :: face_view_result, "failed to resolve face view" :: call) :: call
    let face_view = face_view_result :: bytes :: unwrap_or
    let tables_result = parse_table_directory_ref :: face_view :: call
    if tables_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.SourceFaceMetadata, Str] :: (result_err_or :: tables_result, "font parse failed" :: call) :: call
    let tables = tables_result :: (empty_tables :: :: call) :: unwrap_or
    let head_result = read_table_offset :: tables, "head" :: call
    if head_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.SourceFaceMetadata, Str] :: (result_err_or :: head_result, "font is missing `head` table" :: call) :: call
    let head = head_result :: (0, 0) :: unwrap_or
    let mut family_name = request.fallback_family
    let mut face_name = "Regular"
    let mut full_name = request.fallback_label
    let mut postscript_name = request.fallback_label
    if tables :: "name" :: has:
        let name_table = tables :: "name" :: get
        let typographic_family = arcana_text.font_leaf.name_string_for_id :: face_view, name_table, 16 :: call
        let family = arcana_text.font_leaf.name_string_for_id :: face_view, name_table, 1 :: call
        let typographic_subfamily = arcana_text.font_leaf.name_string_for_id :: face_view, name_table, 17 :: call
        let subfamily = arcana_text.font_leaf.name_string_for_id :: face_view, name_table, 2 :: call
        let full = arcana_text.font_leaf.name_string_for_id :: face_view, name_table, 4 :: call
        let postscript = arcana_text.font_leaf.name_string_for_id :: face_view, name_table, 6 :: call
        if typographic_family != "":
            family_name = typographic_family
        else:
            if family != "":
                family_name = family
        if typographic_subfamily != "":
            face_name = typographic_subfamily
        else:
            if subfamily != "":
                face_name = subfamily
        if full != "":
            full_name = full
        if postscript != "":
            postscript_name = postscript
    if family_name == "":
        family_name = request.fallback_label
    if face_name == "":
        face_name = "Regular"
    if full_name == "":
        if face_name == "" or face_name == "Regular":
            full_name = family_name
        else:
            full_name = family_name + " " + face_name
    if postscript_name == "":
        postscript_name = full_name
    let traits = arcana_text.font_leaf.traits_from_face_tables :: face_view, (arcana_text.font_leaf.FaceTraitsRequest :: tables = tables, fallback_traits = request.fallback_traits, head = head :: call) :: call
    let mut metadata = arcana_text.font_leaf.SourceFaceMetadata :: face_index = request.face_index, family_name = family_name, face_name = face_name :: call
    metadata.full_name = full_name
    metadata.postscript_name = postscript_name
    metadata.traits = traits
    return Result.Ok[arcana_text.font_leaf.SourceFaceMetadata, Str] :: metadata :: call

fn fixed_16_16_to_int(raw: Int) -> Int:
    if raw >= 0:
        return (raw + 32768) / 65536
    return 0 - (((0 - raw) + 32768) / 65536)

fn axis_fixed_to_value(tag: Str, raw: Int) -> Int:
    let value = fixed_16_16_to_int :: raw :: call
    if tag == "wdth" or tag == "slnt":
        return value * 1000
    return value

fn parse_variation_axes(read bytes: std.memory.ByteView, fvar: (Int, Int)) -> List[arcana_text.font_leaf.VariationAxisDef]:
    let mut axes = empty_variation_axes :: :: call
    if fvar.0 < 0 or fvar.1 < 16:
        return axes
    let table_end = fvar.0 + fvar.1
    if table_end > (bytes :: :: len):
        return axes
    let data_offset = u16_be_ref :: bytes, fvar.0 + 4 :: call
    let axis_count = u16_be_ref :: bytes, fvar.0 + 8 :: call
    let axis_size = u16_be_ref :: bytes, fvar.0 + 10 :: call
    if data_offset < 0 or axis_size < 20:
        return axes
    let mut index = 0
    while index < axis_count:
        let record = fvar.0 + data_offset + (index * axis_size)
        if record + 20 > table_end:
            return axes
        let tag = tag_at_ref :: bytes, record :: call
        let min_value = axis_fixed_to_value :: tag, (i32_be_ref :: bytes, record + 4 :: call) :: call
        let default_value = axis_fixed_to_value :: tag, (i32_be_ref :: bytes, record + 8 :: call) :: call
        let max_value = axis_fixed_to_value :: tag, (i32_be_ref :: bytes, record + 12 :: call) :: call
        let axis = variation_axis :: tag, (min_value, default_value), max_value :: call
        axes :: axis :: push
        index += 1
    return axes

fn parse_variation_axes_bytes(read bytes: Array[Int], fvar: (Int, Int)) -> List[arcana_text.font_leaf.VariationAxisDef]:
    let mut axes = empty_variation_axes :: :: call
    if fvar.0 < 0 or fvar.1 < 16:
        return axes
    let table_end = fvar.0 + fvar.1
    if table_end > (bytes :: :: len):
        return axes
    let data_offset = u16_be :: bytes, fvar.0 + 4 :: call
    let axis_count = u16_be :: bytes, fvar.0 + 8 :: call
    let axis_size = u16_be :: bytes, fvar.0 + 10 :: call
    if data_offset < 0 or axis_size < 20:
        return axes
    let mut index = 0
    while index < axis_count:
        let record = fvar.0 + data_offset + (index * axis_size)
        if record + 20 > table_end:
            return axes
        let tag = tag_at :: bytes, record :: call
        let min_value = axis_fixed_to_value :: tag, (i32_be :: bytes, record + 4 :: call) :: call
        let default_value = axis_fixed_to_value :: tag, (i32_be :: bytes, record + 8 :: call) :: call
        let max_value = axis_fixed_to_value :: tag, (i32_be :: bytes, record + 12 :: call) :: call
        let axis = variation_axis :: tag, (min_value, default_value), max_value :: call
        axes :: axis :: push
        index += 1
    return axes

fn face_has_variation_axis(read face: arcana_text.font_leaf.FontFaceState, tag: Str) -> Bool:
    for axis in face.variation_axes:
        if axis.tag == tag:
            return true
    return false

fn variation_axis_value(read axis: arcana_text.font_leaf.VariationAxisDef, read traits: arcana_text.font_leaf.FaceTraits) -> Int:
    if axis.tag == "wght":
        if traits.weight > 0:
            return traits.weight
        return axis.default_value
    if axis.tag == "wdth":
        if traits.width_milli > 0:
            return traits.width_milli
        return axis.default_value
    if axis.tag == "slnt":
        return traits.slant_milli
    return axis.default_value

fn normalized_axis_value(read axis: arcana_text.font_leaf.VariationAxisDef, value: Int) -> Int:
    let clamped = clamp_int :: value, axis.min_value, axis.max_value :: call
    if clamped == axis.default_value:
        return 0
    if clamped < axis.default_value:
        let span = axis.default_value - axis.min_value
        if span <= 0:
            return -16384
        return clamp_int :: (((clamped - axis.default_value) * 16384) / span), -16384, 16384 :: call
    let span = axis.max_value - axis.default_value
    if span <= 0:
        return 16384
    return clamp_int :: (((clamped - axis.default_value) * 16384) / span), -16384, 16384 :: call

fn normalized_location(read face: arcana_text.font_leaf.FontFaceState, read traits: arcana_text.font_leaf.FaceTraits) -> List[Int]:
    let mut out = empty_int_list :: :: call
    for axis in face.variation_axes:
        let value = variation_axis_value :: axis, traits :: call
        out :: (normalized_axis_value :: axis, value :: call) :: push
    return out

fn location_is_default(read location: List[Int]) -> Bool:
    for value in location:
        if value != 0:
            return false
    return true

fn all_point_indices(total: Int) -> List[Int]:
    let mut out = empty_int_list :: :: call
    let mut index = 0
    while index < total:
        out :: index :: push
        index += 1
    return out

fn decode_packed_points(point_count: Int, read bytes: std.memory.ByteView, offset: Int) -> (List[Int], Int):
    let mut pos = offset
    let mut count = byte_at_or_zero_ref :: bytes, pos :: call
    pos += 1
    if count >= 128:
        count = ((count % 128) * 256) + (byte_at_or_zero_ref :: bytes, pos :: call)
        pos += 1
    if count == 0:
        return ((all_point_indices :: point_count :: call), pos)
    let mut out = empty_int_list :: :: call
    let mut current = 0
    while (out :: :: len) < count:
        let header = byte_at_or_zero_ref :: bytes, pos :: call
        pos += 1
        let run_count = (header % 128) + 1
        let use_words = header >= 128
        let mut seen = 0
        while seen < run_count and (out :: :: len) < count:
            let delta = match use_words:
                true => u16_be_ref :: bytes, pos :: call
                false => byte_at_or_zero_ref :: bytes, pos :: call
            pos += match use_words:
                true => 2
                false => 1
            current += delta
            out :: current :: push
            seen += 1
    return (out, pos)

fn decode_packed_deltas(count: Int, read bytes: std.memory.ByteView, offset: Int) -> (List[Int], Int):
    let mut pos = offset
    let mut out = empty_int_list :: :: call
    while (out :: :: len) < count:
        let header = byte_at_or_zero_ref :: bytes, pos :: call
        pos += 1
        let run_count = (header % 64) + 1
        let mut seen = 0
        while seen < run_count and (out :: :: len) < count:
            let mut value = 0
            if header >= 192:
                value = i32_be_ref :: bytes, pos :: call
                pos += 4
            else:
                if header >= 128:
                    value = 0
                else:
                    if header >= 64:
                        value = i16_be_ref :: bytes, pos :: call
                        pos += 2
                    else:
                        value = i8 :: (byte_at_or_zero_ref :: bytes, pos :: call) :: call
                        pos += 1
            out :: value :: push
            seen += 1
    return (out, pos)

fn copy_gsub_unit(read unit: arcana_text.font_leaf.GsubGlyphUnit) -> arcana_text.font_leaf.GsubGlyphUnit:
    return unit

export fn default_gsub_units(read glyphs: List[Int]) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let mut units = empty_gsub_units :: :: call
    for glyph_index in glyphs:
        units :: (arcana_text.font_leaf.GsubGlyphUnit :: glyph_index = glyph_index, consumed = 1 :: call) :: push
    return units

fn script_table_offset(read bytes: std.memory.ByteView, gsub_offset: Int, read script_tag: Str) -> Int:
    if gsub_offset < 0:
        return -1
    let script_list_offset = gsub_offset + (u16_be_ref :: bytes, gsub_offset + 4 :: call)
    let script_count = u16_be_ref :: bytes, script_list_offset :: call
    let mut default_offset = -1
    let mut index = 0
    while index < script_count:
        let record = script_list_offset + 2 + (index * 6)
        let tag = tag_at_ref :: bytes, record :: call
        let offset = script_list_offset + (u16_be_ref :: bytes, record + 4 :: call)
        if tag == script_tag:
            return offset
        if tag == "DFLT":
            default_offset = offset
        index += 1
    return default_offset

fn langsys_table_offset(read bytes: std.memory.ByteView, script_offset: Int, read language_tag: Str) -> Int:
    if script_offset < 0:
        return -1
    let default_offset = u16_be_ref :: bytes, script_offset :: call
    let lang_count = u16_be_ref :: bytes, script_offset + 2 :: call
    if language_tag != "":
        let mut index = 0
        while index < lang_count:
            let record = script_offset + 4 + (index * 6)
            let tag = tag_at_ref :: bytes, record :: call
            if tag == language_tag:
                return script_offset + (u16_be_ref :: bytes, record + 4 :: call)
            index += 1
    if default_offset > 0:
        return script_offset + default_offset
    if lang_count > 0:
        return script_offset + (u16_be_ref :: bytes, script_offset + 8 :: call)
    return -1

fn feature_value(read features: List[arcana_text.types.FontFeature], read tag: Str) -> Int:
    for feature in features:
        if feature.tag == tag:
            if feature.enabled:
                return max_int :: feature.value, 1 :: call
            return 0
    return 0

fn gsub_lookup_cache_key(read request: arcana_text.font_leaf.LookupRefsForFeaturesRequest) -> Str:
    let mut key = request.script_tag + ":" + request.language_tag
    for feature in request.features:
        key = key + ":" + feature.tag + "=" + (std.text.from_int :: (arcana_text.font_leaf.feature_value :: request.features, feature.tag :: call) :: call)
    return key

fn push_lookup_ref(edit out: List[arcana_text.font_leaf.GsubLookupRef], lookup_index: Int, feature_value: Int):
    for existing in out:
        if existing.lookup_index == lookup_index and existing.feature_value == feature_value:
            return
    out :: (arcana_text.font_leaf.GsubLookupRef :: lookup_index = lookup_index, feature_value = feature_value :: call) :: push

fn append_feature_lookup_refs(edit out: List[arcana_text.font_leaf.GsubLookupRef], read request: arcana_text.font_leaf.AppendFeatureLookupRefsRequest):
    let feature_count = u16_be_ref :: request.bytes, request.feature_list_offset :: call
    if request.feature_index < 0 or request.feature_index >= feature_count:
        return
    let record = request.feature_list_offset + 2 + (request.feature_index * 6)
    let feature_table = request.feature_list_offset + (u16_be_ref :: request.bytes, record + 4 :: call)
    let lookup_count = u16_be_ref :: request.bytes, feature_table + 2 :: call
    let mut index = 0
    while index < lookup_count:
        let lookup_index = u16_be_ref :: request.bytes, feature_table + 4 + (index * 2) :: call
        arcana_text.font_leaf.push_lookup_ref :: out, lookup_index, request.feature_value :: call
        index += 1

fn lookup_refs_for_features(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.LookupRefsForFeaturesRequest) -> List[arcana_text.font_leaf.GsubLookupRef]:
    let mut refs = empty_lookup_refs :: :: call
    if face.gsub_offset < 0 or face.gsub_length <= 0:
        return refs
    let cache_key = arcana_text.font_leaf.gsub_lookup_cache_key :: request :: call
    if face.gsub_lookup_cache :: cache_key :: has:
        return arcana_text.font_leaf.copy_lookup_refs :: (face.gsub_lookup_cache :: cache_key :: get) :: call
    let script_offset = arcana_text.font_leaf.script_table_offset :: face.font_view, face.gsub_offset, request.script_tag :: call
    if script_offset < 0:
        return refs
    let langsys_offset = arcana_text.font_leaf.langsys_table_offset :: face.font_view, script_offset, request.language_tag :: call
    if langsys_offset < 0:
        return refs
    let feature_list_offset = face.gsub_offset + (u16_be_ref :: face.font_view, face.gsub_offset + 6 :: call)
    let required_feature_index = u16_be_ref :: face.font_view, langsys_offset + 2 :: call
    if required_feature_index != 65535:
        let mut append_request = arcana_text.font_leaf.AppendFeatureLookupRefsRequest :: bytes = face.font_view, feature_list_offset = feature_list_offset, feature_index = required_feature_index :: call
        append_request.feature_value = 1
        arcana_text.font_leaf.append_feature_lookup_refs :: refs, append_request :: call
    let feature_count = u16_be_ref :: face.font_view, feature_list_offset :: call
    let enabled_count = u16_be_ref :: face.font_view, langsys_offset + 4 :: call
    let mut enabled_index = 0
    while enabled_index < enabled_count:
        let feature_index = u16_be_ref :: face.font_view, langsys_offset + 6 + (enabled_index * 2) :: call
        if feature_index >= 0 and feature_index < feature_count:
            let record = feature_list_offset + 2 + (feature_index * 6)
            let tag = tag_at_ref :: face.font_view, record :: call
            let value = arcana_text.font_leaf.feature_value :: request.features, tag :: call
            if value > 0:
                let mut append_request = arcana_text.font_leaf.AppendFeatureLookupRefsRequest :: bytes = face.font_view, feature_list_offset = feature_list_offset, feature_index = feature_index :: call
                append_request.feature_value = value
                arcana_text.font_leaf.append_feature_lookup_refs :: refs, append_request :: call
        enabled_index += 1
    face.gsub_lookup_cache :: cache_key, (arcana_text.font_leaf.copy_lookup_refs :: refs :: call) :: set
    return refs

fn position_lookup_refs_for_features(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.LookupRefsForFeaturesRequest) -> List[arcana_text.font_leaf.GsubLookupRef]:
    let mut refs = empty_lookup_refs :: :: call
    if face.gpos_offset < 0 or face.gpos_length <= 0:
        return refs
    let cache_key = arcana_text.font_leaf.gsub_lookup_cache_key :: request :: call
    if face.gpos_lookup_cache :: cache_key :: has:
        return arcana_text.font_leaf.copy_lookup_refs :: (face.gpos_lookup_cache :: cache_key :: get) :: call
    let script_offset = arcana_text.font_leaf.script_table_offset :: face.font_view, face.gpos_offset, request.script_tag :: call
    if script_offset < 0:
        return refs
    let langsys_offset = arcana_text.font_leaf.langsys_table_offset :: face.font_view, script_offset, request.language_tag :: call
    if langsys_offset < 0:
        return refs
    let feature_list_offset = face.gpos_offset + (u16_be_ref :: face.font_view, face.gpos_offset + 6 :: call)
    let required_feature_index = u16_be_ref :: face.font_view, langsys_offset + 2 :: call
    if required_feature_index != 65535:
        let mut append_request = arcana_text.font_leaf.AppendFeatureLookupRefsRequest :: bytes = face.font_view, feature_list_offset = feature_list_offset, feature_index = required_feature_index :: call
        append_request.feature_value = 1
        arcana_text.font_leaf.append_feature_lookup_refs :: refs, append_request :: call
    let feature_count = u16_be_ref :: face.font_view, feature_list_offset :: call
    let enabled_count = u16_be_ref :: face.font_view, langsys_offset + 4 :: call
    let mut enabled_index = 0
    while enabled_index < enabled_count:
        let feature_index = u16_be_ref :: face.font_view, langsys_offset + 6 + (enabled_index * 2) :: call
        if feature_index >= 0 and feature_index < feature_count:
            let record = feature_list_offset + 2 + (feature_index * 6)
            let tag = tag_at_ref :: face.font_view, record :: call
            let value = arcana_text.font_leaf.feature_value :: request.features, tag :: call
            if value > 0:
                let mut append_request = arcana_text.font_leaf.AppendFeatureLookupRefsRequest :: bytes = face.font_view, feature_list_offset = feature_list_offset, feature_index = feature_index :: call
                append_request.feature_value = value
                arcana_text.font_leaf.append_feature_lookup_refs :: refs, append_request :: call
        enabled_index += 1
    face.gpos_lookup_cache :: cache_key, (arcana_text.font_leaf.copy_lookup_refs :: refs :: call) :: set
    return refs

fn gdef_glyph_class_def_offset(read face: arcana_text.font_leaf.FontFaceState) -> Int:
    if face.gdef_offset < 0 or face.gdef_length < 6:
        return -1
    let delta = u16_be_ref :: face.font_view, face.gdef_offset + 4 :: call
    if delta <= 0:
        return -1
    let offset = face.gdef_offset + delta
    if offset >= face.gdef_offset + face.gdef_length:
        return -1
    return offset

fn value_record_size(value_format: Int) -> Int:
    let mut size = 0
    let mut bit = 1
    while bit <= 128:
        if (value_format % (bit * 2)) >= bit:
            size += 2
        bit = bit * 2
    return size

fn value_record(read bytes: std.memory.ByteView, offset: Int, value_format: Int) -> arcana_text.font_leaf.PairPlacement:
    let mut cursor = offset
    let mut value = arcana_text.font_leaf.empty_pair_placement :: :: call
    if (value_format % 2) == 1:
        value.x_offset = i16_be_ref :: bytes, cursor :: call
        cursor += 2
    if (value_format % 4) >= 2:
        value.y_offset = i16_be_ref :: bytes, cursor :: call
        cursor += 2
    if (value_format % 8) >= 4:
        value.x_advance = i16_be_ref :: bytes, cursor :: call
        cursor += 2
    if (value_format % 16) >= 8:
        value.y_advance = i16_be_ref :: bytes, cursor :: call
        cursor += 2
    if (value_format % 32) >= 16:
        cursor += 2
    if (value_format % 64) >= 32:
        cursor += 2
    if (value_format % 128) >= 64:
        cursor += 2
    if (value_format % 256) >= 128:
        cursor += 2
    return value

fn combined_pair_value(read left: arcana_text.font_leaf.PairPlacement, read right: arcana_text.font_leaf.PairPlacement) -> arcana_text.font_leaf.PairPlacement:
    let mut out = arcana_text.font_leaf.empty_pair_placement :: :: call
    out.x_offset = left.x_advance + right.x_offset
    out.y_offset = left.y_advance + right.y_offset
    out.x_advance = left.x_advance + right.x_advance
    out.y_advance = left.y_advance + right.y_advance
    out.zero_advance = left.zero_advance or right.zero_advance
    out.attach_to_left_origin = left.attach_to_left_origin or right.attach_to_left_origin
    return out

fn class_def_index(read bytes: std.memory.ByteView, class_def_offset: Int, glyph_index: Int) -> Int:
    if class_def_offset <= 0:
        return 0
    let format = u16_be_ref :: bytes, class_def_offset :: call
    if format == 1:
        let start_glyph = u16_be_ref :: bytes, class_def_offset + 2 :: call
        let glyph_count = u16_be_ref :: bytes, class_def_offset + 4 :: call
        if glyph_index < start_glyph or glyph_index >= start_glyph + glyph_count:
            return 0
        return u16_be_ref :: bytes, class_def_offset + 6 + ((glyph_index - start_glyph) * 2) :: call
    if format == 2:
        let range_count = u16_be_ref :: bytes, class_def_offset + 2 :: call
        let mut index = 0
        while index < range_count:
            let record = class_def_offset + 4 + (index * 6)
            let start_glyph = u16_be_ref :: bytes, record :: call
            let end_glyph = u16_be_ref :: bytes, record + 2 :: call
            if glyph_index >= start_glyph and glyph_index <= end_glyph:
                return u16_be_ref :: bytes, record + 4 :: call
            index += 1
    return 0

fn pair_position_format1_adjust(read request: arcana_text.font_leaf.PairPositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let value_format1 = u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call
    let value_format2 = u16_be_ref :: request.bytes, request.subtable_offset + 6 :: call
    let pair_set_count = u16_be_ref :: request.bytes, request.subtable_offset + 8 :: call
    let coverage_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, request.left_glyph :: call
    if coverage_value < 0 or coverage_value >= pair_set_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let pair_set_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 10 + (coverage_value * 2) :: call)
    let pair_value_count = u16_be_ref :: request.bytes, pair_set_offset :: call
    let value1_size = arcana_text.font_leaf.value_record_size :: value_format1 :: call
    let value2_size = arcana_text.font_leaf.value_record_size :: value_format2 :: call
    let record_size = 2 + value1_size + value2_size
    let mut index = 0
    while index < pair_value_count:
        let record = pair_set_offset + 2 + (index * record_size)
        let second_glyph = u16_be_ref :: request.bytes, record :: call
        if second_glyph == request.right_glyph:
            let left = arcana_text.font_leaf.value_record :: request.bytes, record + 2, value_format1 :: call
            let right = arcana_text.font_leaf.value_record :: request.bytes, record + 2 + value1_size, value_format2 :: call
            return arcana_text.font_leaf.combined_pair_value :: left, right :: call
        index += 1
    return arcana_text.font_leaf.empty_pair_placement :: :: call

fn pair_position_format2_adjust(read request: arcana_text.font_leaf.PairPositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let value_format1 = u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call
    let value_format2 = u16_be_ref :: request.bytes, request.subtable_offset + 6 :: call
    let class_def1_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 8 :: call)
    let class_def2_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 10 :: call)
    let class1_count = u16_be_ref :: request.bytes, request.subtable_offset + 12 :: call
    let class2_count = u16_be_ref :: request.bytes, request.subtable_offset + 14 :: call
    let coverage_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, request.left_glyph :: call
    if coverage_value < 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let class1 = arcana_text.font_leaf.class_def_index :: request.bytes, class_def1_offset, request.left_glyph :: call
    let class2 = arcana_text.font_leaf.class_def_index :: request.bytes, class_def2_offset, request.right_glyph :: call
    if class1 < 0 or class1 >= class1_count or class2 < 0 or class2 >= class2_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let value1_size = arcana_text.font_leaf.value_record_size :: value_format1 :: call
    let value2_size = arcana_text.font_leaf.value_record_size :: value_format2 :: call
    let record_size = value1_size + value2_size
    let record = request.subtable_offset + 16 + (((class1 * class2_count) + class2) * record_size)
    let left = arcana_text.font_leaf.value_record :: request.bytes, record, value_format1 :: call
    let right = arcana_text.font_leaf.value_record :: request.bytes, record + value1_size, value_format2 :: call
    return arcana_text.font_leaf.combined_pair_value :: left, right :: call

fn pair_position_subtable_adjust(read request: arcana_text.font_leaf.PairPositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let format = u16_be_ref :: request.bytes, request.subtable_offset :: call
    if format == 1:
        return arcana_text.font_leaf.pair_position_format1_adjust :: request :: call
    if format == 2:
        return arcana_text.font_leaf.pair_position_format2_adjust :: request :: call
    return arcana_text.font_leaf.empty_pair_placement :: :: call

fn single_position_format1_adjust(read request: arcana_text.font_leaf.SinglePositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let value_format = u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call
    let coverage_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, request.glyph :: call
    if coverage_value < 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    return arcana_text.font_leaf.value_record :: request.bytes, request.subtable_offset + 6, value_format :: call

fn single_position_format2_adjust(read request: arcana_text.font_leaf.SinglePositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let value_format = u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call
    let value_count = u16_be_ref :: request.bytes, request.subtable_offset + 6 :: call
    let coverage_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, request.glyph :: call
    if coverage_value < 0 or coverage_value >= value_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let record_size = arcana_text.font_leaf.value_record_size :: value_format :: call
    return arcana_text.font_leaf.value_record :: request.bytes, request.subtable_offset + 8 + (coverage_value * record_size), value_format :: call

fn single_position_subtable_adjust(read request: arcana_text.font_leaf.SinglePositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let format = u16_be_ref :: request.bytes, request.subtable_offset :: call
    if format == 1:
        return arcana_text.font_leaf.single_position_format1_adjust :: request :: call
    if format == 2:
        return arcana_text.font_leaf.single_position_format2_adjust :: request :: call
    return arcana_text.font_leaf.empty_pair_placement :: :: call

fn anchor_point(read bytes: std.memory.ByteView, anchor_offset: Int) -> arcana_text.font_leaf.AnchorPoint:
    if anchor_offset <= 0 or anchor_offset + 6 > (bytes :: :: len):
        return arcana_text.font_leaf.invalid_anchor_point :: :: call
    let format = u16_be_ref :: bytes, anchor_offset :: call
    if format <= 0:
        return arcana_text.font_leaf.invalid_anchor_point :: :: call
    let mut point = arcana_text.font_leaf.AnchorPoint :: x = (i16_be_ref :: bytes, anchor_offset + 2 :: call), y = (i16_be_ref :: bytes, anchor_offset + 4 :: call) :: call
    point.valid = true
    return point

fn mark_to_base_adjust(read request: arcana_text.font_leaf.PairPositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let mark_coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let base_coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call)
    let class_count = u16_be_ref :: request.bytes, request.subtable_offset + 6 :: call
    let mark_array_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 8 :: call)
    let base_array_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 10 :: call)
    let mark_index = arcana_text.font_leaf.coverage_index :: request.bytes, mark_coverage_offset, request.right_glyph :: call
    let base_index = arcana_text.font_leaf.coverage_index :: request.bytes, base_coverage_offset, request.left_glyph :: call
    if mark_index < 0 or base_index < 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark_count = u16_be_ref :: request.bytes, mark_array_offset :: call
    let base_count = u16_be_ref :: request.bytes, base_array_offset :: call
    if mark_index >= mark_count or base_index >= base_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark_record = mark_array_offset + 2 + (mark_index * 4)
    let mark_class = u16_be_ref :: request.bytes, mark_record :: call
    if mark_class < 0 or mark_class >= class_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark_anchor_offset = mark_array_offset + (u16_be_ref :: request.bytes, mark_record + 2 :: call)
    let base_record = base_array_offset + 2 + (base_index * class_count * 2)
    let base_anchor_offset = base_array_offset + (u16_be_ref :: request.bytes, base_record + (mark_class * 2) :: call)
    let mark_anchor = arcana_text.font_leaf.anchor_point :: request.bytes, mark_anchor_offset :: call
    let base_anchor = arcana_text.font_leaf.anchor_point :: request.bytes, base_anchor_offset :: call
    if not mark_anchor.valid or not base_anchor.valid:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mut out = arcana_text.font_leaf.empty_pair_placement :: :: call
    out.x_offset = base_anchor.x - mark_anchor.x
    out.y_offset = base_anchor.y - mark_anchor.y
    out.zero_advance = true
    out.attach_to_left_origin = true
    return out

fn mark_to_mark_adjust(read request: arcana_text.font_leaf.PairPositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let mark1_coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let mark2_coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call)
    let class_count = u16_be_ref :: request.bytes, request.subtable_offset + 6 :: call
    let mark1_array_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 8 :: call)
    let mark2_array_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 10 :: call)
    let mark1_index = arcana_text.font_leaf.coverage_index :: request.bytes, mark1_coverage_offset, request.right_glyph :: call
    let mark2_index = arcana_text.font_leaf.coverage_index :: request.bytes, mark2_coverage_offset, request.left_glyph :: call
    if mark1_index < 0 or mark2_index < 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark1_count = u16_be_ref :: request.bytes, mark1_array_offset :: call
    let mark2_count = u16_be_ref :: request.bytes, mark2_array_offset :: call
    if mark1_index >= mark1_count or mark2_index >= mark2_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark1_record = mark1_array_offset + 2 + (mark1_index * 4)
    let mark_class = u16_be_ref :: request.bytes, mark1_record :: call
    if mark_class < 0 or mark_class >= class_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark1_anchor_offset = mark1_array_offset + (u16_be_ref :: request.bytes, mark1_record + 2 :: call)
    let mark2_record = mark2_array_offset + 2 + (mark2_index * class_count * 2)
    let mark2_anchor_offset = mark2_array_offset + (u16_be_ref :: request.bytes, mark2_record + (mark_class * 2) :: call)
    let mark1_anchor = arcana_text.font_leaf.anchor_point :: request.bytes, mark1_anchor_offset :: call
    let mark2_anchor = arcana_text.font_leaf.anchor_point :: request.bytes, mark2_anchor_offset :: call
    if not mark1_anchor.valid or not mark2_anchor.valid:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mut out = arcana_text.font_leaf.empty_pair_placement :: :: call
    out.x_offset = mark2_anchor.x - mark1_anchor.x
    out.y_offset = mark2_anchor.y - mark1_anchor.y
    out.zero_advance = true
    out.attach_to_left_origin = true
    return out

fn mark_to_ligature_adjust(read request: arcana_text.font_leaf.PairPositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let mark_coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let ligature_coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call)
    let class_count = u16_be_ref :: request.bytes, request.subtable_offset + 6 :: call
    let mark_array_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 8 :: call)
    let ligature_array_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 10 :: call)
    let mark_index = arcana_text.font_leaf.coverage_index :: request.bytes, mark_coverage_offset, request.right_glyph :: call
    let ligature_index = arcana_text.font_leaf.coverage_index :: request.bytes, ligature_coverage_offset, request.left_glyph :: call
    if mark_index < 0 or ligature_index < 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark_count = u16_be_ref :: request.bytes, mark_array_offset :: call
    let ligature_count = u16_be_ref :: request.bytes, ligature_array_offset :: call
    if mark_index >= mark_count or ligature_index >= ligature_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark_record = mark_array_offset + 2 + (mark_index * 4)
    let mark_class = u16_be_ref :: request.bytes, mark_record :: call
    if mark_class < 0 or mark_class >= class_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mark_anchor_offset = mark_array_offset + (u16_be_ref :: request.bytes, mark_record + 2 :: call)
    let ligature_attach_offset = ligature_array_offset + (u16_be_ref :: request.bytes, ligature_array_offset + 2 + (ligature_index * 2) :: call)
    let component_count = u16_be_ref :: request.bytes, ligature_attach_offset :: call
    if component_count <= 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let component_index = component_count - 1
    let component_record = ligature_attach_offset + 2 + (component_index * class_count * 2)
    let ligature_anchor_offset = ligature_attach_offset + (u16_be_ref :: request.bytes, component_record + (mark_class * 2) :: call)
    let mark_anchor = arcana_text.font_leaf.anchor_point :: request.bytes, mark_anchor_offset :: call
    let ligature_anchor = arcana_text.font_leaf.anchor_point :: request.bytes, ligature_anchor_offset :: call
    if not mark_anchor.valid or not ligature_anchor.valid:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mut out = arcana_text.font_leaf.empty_pair_placement :: :: call
    out.x_offset = ligature_anchor.x - mark_anchor.x
    out.y_offset = ligature_anchor.y - mark_anchor.y
    out.zero_advance = true
    out.attach_to_left_origin = true
    return out

fn cursive_adjust(read request: arcana_text.font_leaf.PairPositionSubtableRequest) -> arcana_text.font_leaf.PairPlacement:
    let coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let record_count = u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call
    let left_index = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, request.left_glyph :: call
    let right_index = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, request.right_glyph :: call
    if left_index < 0 or right_index < 0 or left_index >= record_count or right_index >= record_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let left_record = request.subtable_offset + 6 + (left_index * 4)
    let right_record = request.subtable_offset + 6 + (right_index * 4)
    let left_exit_offset = request.subtable_offset + (u16_be_ref :: request.bytes, left_record + 2 :: call)
    let right_entry_offset = request.subtable_offset + (u16_be_ref :: request.bytes, right_record :: call)
    let left_exit = arcana_text.font_leaf.anchor_point :: request.bytes, left_exit_offset :: call
    let right_entry = arcana_text.font_leaf.anchor_point :: request.bytes, right_entry_offset :: call
    if not left_exit.valid or not right_entry.valid:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mut out = arcana_text.font_leaf.empty_pair_placement :: :: call
    out.x_offset = left_exit.x - right_entry.x
    out.y_offset = left_exit.y - right_entry.y
    out.attach_to_left_origin = true
    return out

fn position_lookup_adjust(read request: arcana_text.font_leaf.PositionLookupAdjustRequest) -> arcana_text.font_leaf.PairPlacement:
    let lookup_count = u16_be_ref :: request.bytes, request.lookup_list_offset :: call
    if request.lookup.lookup_index < 0 or request.lookup.lookup_index >= lookup_count:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let lookup_offset = request.lookup_list_offset + (u16_be_ref :: request.bytes, request.lookup_list_offset + 2 + (request.lookup.lookup_index * 2) :: call)
    let lookup_type = u16_be_ref :: request.bytes, lookup_offset :: call
    let subtable_count = u16_be_ref :: request.bytes, lookup_offset + 4 :: call
    let mut subtable_index = 0
    while subtable_index < subtable_count:
        let subtable_offset = lookup_offset + (u16_be_ref :: request.bytes, lookup_offset + 6 + (subtable_index * 2) :: call)
        if lookup_type == 1:
            let single_request = arcana_text.font_leaf.SinglePositionSubtableRequest :: bytes = request.bytes, subtable_offset = subtable_offset, glyph = request.left_glyph :: call
            let adjust = arcana_text.font_leaf.single_position_subtable_adjust :: single_request :: call
            if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                return adjust
        if lookup_type == 2:
            let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = subtable_offset, left_glyph = request.left_glyph :: call
            pair_request.right_glyph = request.right_glyph
            let adjust = arcana_text.font_leaf.pair_position_subtable_adjust :: pair_request :: call
            if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                return adjust
        if lookup_type == 3:
            let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = subtable_offset, left_glyph = request.left_glyph :: call
            pair_request.right_glyph = request.right_glyph
            let adjust = arcana_text.font_leaf.cursive_adjust :: pair_request :: call
            if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                return adjust
        if lookup_type == 4:
            let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = subtable_offset, left_glyph = request.left_glyph :: call
            pair_request.right_glyph = request.right_glyph
            let adjust = arcana_text.font_leaf.mark_to_base_adjust :: pair_request :: call
            if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                return adjust
        if lookup_type == 5:
            let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = subtable_offset, left_glyph = request.left_glyph :: call
            pair_request.right_glyph = request.right_glyph
            let adjust = arcana_text.font_leaf.mark_to_ligature_adjust :: pair_request :: call
            if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                return adjust
        if lookup_type == 6:
            let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = subtable_offset, left_glyph = request.left_glyph :: call
            pair_request.right_glyph = request.right_glyph
            let adjust = arcana_text.font_leaf.mark_to_mark_adjust :: pair_request :: call
            if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                return adjust
        if lookup_type == 9:
            let extension_type = u16_be_ref :: request.bytes, subtable_offset + 2 :: call
            if extension_type == 1:
                let single_request = arcana_text.font_leaf.SinglePositionSubtableRequest :: bytes = request.bytes, subtable_offset = (subtable_offset + (u32_be_ref :: request.bytes, subtable_offset + 4 :: call)), glyph = request.left_glyph :: call
                let adjust = arcana_text.font_leaf.single_position_subtable_adjust :: single_request :: call
                if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                    return adjust
            if extension_type == 2:
                let nested_offset = subtable_offset + (u32_be_ref :: request.bytes, subtable_offset + 4 :: call)
                let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = nested_offset, left_glyph = request.left_glyph :: call
                pair_request.right_glyph = request.right_glyph
                let adjust = arcana_text.font_leaf.pair_position_subtable_adjust :: pair_request :: call
                if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                    return adjust
            if extension_type == 3:
                let nested_offset = subtable_offset + (u32_be_ref :: request.bytes, subtable_offset + 4 :: call)
                let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = nested_offset, left_glyph = request.left_glyph :: call
                pair_request.right_glyph = request.right_glyph
                let adjust = arcana_text.font_leaf.cursive_adjust :: pair_request :: call
                if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                    return adjust
            if extension_type == 4:
                let nested_offset = subtable_offset + (u32_be_ref :: request.bytes, subtable_offset + 4 :: call)
                let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = nested_offset, left_glyph = request.left_glyph :: call
                pair_request.right_glyph = request.right_glyph
                let adjust = arcana_text.font_leaf.mark_to_base_adjust :: pair_request :: call
                if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                    return adjust
            if extension_type == 5:
                let nested_offset = subtable_offset + (u32_be_ref :: request.bytes, subtable_offset + 4 :: call)
                let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = nested_offset, left_glyph = request.left_glyph :: call
                pair_request.right_glyph = request.right_glyph
                let adjust = arcana_text.font_leaf.mark_to_ligature_adjust :: pair_request :: call
                if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                    return adjust
            if extension_type == 6:
                let nested_offset = subtable_offset + (u32_be_ref :: request.bytes, subtable_offset + 4 :: call)
                let mut pair_request = arcana_text.font_leaf.PairPositionSubtableRequest :: bytes = request.bytes, subtable_offset = nested_offset, left_glyph = request.left_glyph :: call
                pair_request.right_glyph = request.right_glyph
                let adjust = arcana_text.font_leaf.mark_to_mark_adjust :: pair_request :: call
                if arcana_text.font_leaf.placement_has_effect :: adjust :: call:
                    return adjust
        subtable_index += 1
    return arcana_text.font_leaf.empty_pair_placement :: :: call

fn coverage_index(read bytes: std.memory.ByteView, coverage_offset: Int, glyph_index: Int) -> Int:
    if coverage_offset <= 0:
        return -1
    let format = u16_be_ref :: bytes, coverage_offset :: call
    if format == 1:
        let glyph_count = u16_be_ref :: bytes, coverage_offset + 2 :: call
        let mut index = 0
        while index < glyph_count:
            if (u16_be_ref :: bytes, coverage_offset + 4 + (index * 2) :: call) == glyph_index:
                return index
            index += 1
        return -1
    if format == 2:
        let range_count = u16_be_ref :: bytes, coverage_offset + 2 :: call
        let mut index = 0
        while index < range_count:
            let record = coverage_offset + 4 + (index * 6)
            let start_glyph = u16_be_ref :: bytes, record :: call
            let end_glyph = u16_be_ref :: bytes, record + 2 :: call
            if glyph_index >= start_glyph and glyph_index <= end_glyph:
                return (u16_be_ref :: bytes, record + 4 :: call) + (glyph_index - start_glyph)
            index += 1
        return -1
    return -1

fn unit_glyph_index(read units: List[arcana_text.font_leaf.GsubGlyphUnit], index: Int) -> Int:
    if index < 0 or index >= (units :: :: len):
        return -1
    return (units)[index].glyph_index

fn single_substitution_match(read bytes: std.memory.ByteView, subtable_offset: Int, glyph_index: Int) -> arcana_text.font_leaf.GsubMatch:
    let format = u16_be_ref :: bytes, subtable_offset :: call
    let coverage_offset = subtable_offset + (u16_be_ref :: bytes, subtable_offset + 2 :: call)
    let coverage_index_value = arcana_text.font_leaf.coverage_index :: bytes, coverage_offset, glyph_index :: call
    if coverage_index_value < 0:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    if format == 1:
        let delta = i16_be_ref :: bytes, subtable_offset + 4 :: call
        return arcana_text.font_leaf.gsub_match :: (positive_mod :: glyph_index + delta, 65536 :: call), 1, true :: call
    if format == 2:
        let glyph_count = u16_be_ref :: bytes, subtable_offset + 4 :: call
        if coverage_index_value >= glyph_count:
            return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
        let substitute = u16_be_ref :: bytes, subtable_offset + 6 + (coverage_index_value * 2) :: call
        return arcana_text.font_leaf.gsub_match :: substitute, 1, true :: call
    return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call

fn alternate_substitution_match(read request: arcana_text.font_leaf.AlternateSubstitutionMatchRequest) -> arcana_text.font_leaf.GsubMatch:
    let coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let coverage_index_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, request.glyph_index :: call
    if coverage_index_value < 0:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let alternate_set_count = u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call
    if coverage_index_value >= alternate_set_count:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let set_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 6 + (coverage_index_value * 2) :: call)
    let alternate_count = u16_be_ref :: request.bytes, set_offset :: call
    if alternate_count <= 0:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let mut pick = request.feature_value - 1
    if pick < 0:
        pick = 0
    if pick >= alternate_count:
        pick = alternate_count - 1
    let substitute = u16_be_ref :: request.bytes, set_offset + 2 + (pick * 2) :: call
    return arcana_text.font_leaf.gsub_match :: substitute, 1, true :: call

fn ligature_substitution_match(read request: arcana_text.font_leaf.LigatureSubstitutionMatchRequest) -> arcana_text.font_leaf.GsubMatch:
    let first = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call
    if first < 0:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let coverage_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call)
    let coverage_index_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, first :: call
    if coverage_index_value < 0:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let ligature_set_count = u16_be_ref :: request.bytes, request.subtable_offset + 4 :: call
    if coverage_index_value >= ligature_set_count:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let set_offset = request.subtable_offset + (u16_be_ref :: request.bytes, request.subtable_offset + 6 + (coverage_index_value * 2) :: call)
    let ligature_count = u16_be_ref :: request.bytes, set_offset :: call
    let total_units = request.units :: :: len
    let mut best = arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let mut ligature_index = 0
    while ligature_index < ligature_count:
        let ligature_offset = set_offset + (u16_be_ref :: request.bytes, set_offset + 2 + (ligature_index * 2) :: call)
        let substitute = u16_be_ref :: request.bytes, ligature_offset :: call
        let component_count = u16_be_ref :: request.bytes, ligature_offset + 2 :: call
        if component_count > 0 and (request.index + component_count) <= total_units:
            let mut matched = true
            let mut component_index = 1
            while component_index < component_count and matched:
                let expected = u16_be_ref :: request.bytes, ligature_offset + 4 + ((component_index - 1) * 2) :: call
                let actual = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index + component_index :: call
                if actual != expected:
                    matched = false
                component_index += 1
            if matched:
                if not best.found or component_count > best.sequence_consumed:
                    best = arcana_text.font_leaf.gsub_match :: substitute, component_count, true :: call
        ligature_index += 1
    return best

fn alternate_subtable_match(read request: arcana_text.font_leaf.SubtableMatchRequest) -> arcana_text.font_leaf.GsubMatch:
    let mut alternate_request = arcana_text.font_leaf.AlternateSubstitutionMatchRequest :: bytes = request.bytes, subtable_offset = request.subtable_offset, glyph_index = (arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call) :: call
    alternate_request.feature_value = request.feature_value
    return arcana_text.font_leaf.alternate_substitution_match :: alternate_request :: call

fn ligature_subtable_match(read request: arcana_text.font_leaf.SubtableMatchRequest) -> arcana_text.font_leaf.GsubMatch:
    let mut ligature_request = arcana_text.font_leaf.LigatureSubstitutionMatchRequest :: bytes = request.bytes, subtable_offset = request.subtable_offset, index = request.index :: call
    ligature_request.units = request.units
    return arcana_text.font_leaf.ligature_substitution_match :: ligature_request :: call

fn extension_subtable_match(read request: arcana_text.font_leaf.SubtableMatchRequest) -> arcana_text.font_leaf.GsubMatch:
    let mut nested = arcana_text.font_leaf.SubtableMatchRequest :: bytes = request.bytes, lookup_type = (u16_be_ref :: request.bytes, request.subtable_offset + 2 :: call), subtable_offset = (request.subtable_offset + (u32_be_ref :: request.bytes, request.subtable_offset + 4 :: call)) :: call
    nested.units = request.units
    nested.index = request.index
    nested.feature_value = request.feature_value
    return arcana_text.font_leaf.subtable_match :: nested :: call

fn subtable_match(read request: arcana_text.font_leaf.SubtableMatchRequest) -> arcana_text.font_leaf.GsubMatch:
    return match request.lookup_type:
        1 => arcana_text.font_leaf.single_substitution_match :: request.bytes, request.subtable_offset, (arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call) :: call
        3 => arcana_text.font_leaf.alternate_subtable_match :: request :: call
        4 => arcana_text.font_leaf.ligature_subtable_match :: request :: call
        7 => arcana_text.font_leaf.extension_subtable_match :: request :: call
        _ => arcana_text.font_leaf.gsub_match :: 0, 0, false :: call

fn lookup_match(read request: arcana_text.font_leaf.LookupMatchRequest) -> arcana_text.font_leaf.GsubMatch:
    let lookup_count = u16_be_ref :: request.bytes, request.lookup_list_offset :: call
    if request.lookup_index < 0 or request.lookup_index >= lookup_count:
        return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call
    let lookup_offset = request.lookup_list_offset + (u16_be_ref :: request.bytes, request.lookup_list_offset + 2 + (request.lookup_index * 2) :: call)
    let lookup_type = u16_be_ref :: request.bytes, lookup_offset :: call
    let subtable_count = u16_be_ref :: request.bytes, lookup_offset + 4 :: call
    let mut subtable_index = 0
    while subtable_index < subtable_count:
        let subtable_offset = lookup_offset + (u16_be_ref :: request.bytes, lookup_offset + 6 + (subtable_index * 2) :: call)
        let mut subtable_request = arcana_text.font_leaf.SubtableMatchRequest :: bytes = request.bytes, lookup_type = lookup_type, subtable_offset = subtable_offset :: call
        subtable_request.units = request.units
        subtable_request.index = request.index
        subtable_request.feature_value = request.feature_value
        let matched = arcana_text.font_leaf.subtable_match :: subtable_request :: call
        if matched.found:
            return matched
        subtable_index += 1
    return arcana_text.font_leaf.gsub_match :: 0, 0, false :: call

fn range_consumed(read units: List[arcana_text.font_leaf.GsubGlyphUnit], index: Int, count: Int) -> Int:
    let total = units :: :: len
    let mut sum = 0
    let mut part = 0
    while part < count and (index + part) < total:
        sum += (units)[index + part].consumed
        part += 1
    return sum

fn replacement_units(read glyphs: List[Int], consumed: Int) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let mut out = empty_gsub_units :: :: call
    let mut index = 0
    for glyph_index in glyphs:
        let mut unit = arcana_text.font_leaf.GsubGlyphUnit :: glyph_index = glyph_index, consumed = 0 :: call
        if index == 0:
            unit.consumed = consumed
        out :: unit :: push
        index += 1
    return out

fn replace_units_range(read units: List[arcana_text.font_leaf.GsubGlyphUnit], span: (Int, Int), read replacement: List[arcana_text.font_leaf.GsubGlyphUnit]) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let index = span.0
    let count = span.1
    let mut out = empty_gsub_units :: :: call
    let total = units :: :: len
    let mut cursor = 0
    while cursor < index and cursor < total:
        out :: (arcana_text.font_leaf.copy_gsub_unit :: (units)[cursor] :: call) :: push
        cursor += 1
    out :: replacement :: extend_list
    cursor = index + count
    while cursor < total:
        out :: (arcana_text.font_leaf.copy_gsub_unit :: (units)[cursor] :: call) :: push
        cursor += 1
    return out

fn replace_range_with_glyphs(read units: List[arcana_text.font_leaf.GsubGlyphUnit], span: (Int, Int), read glyphs: List[Int]) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let consumed = arcana_text.font_leaf.range_consumed :: units, span.0, span.1 :: call
    let replacement = arcana_text.font_leaf.replacement_units :: glyphs, consumed :: call
    return arcana_text.font_leaf.replace_units_range :: units, span, replacement :: call

fn multiple_substitution_glyphs(read bytes: std.memory.ByteView, subtable_offset: Int, glyph_index: Int) -> List[Int]:
    let coverage_offset = subtable_offset + (u16_be_ref :: bytes, subtable_offset + 2 :: call)
    let coverage_index_value = arcana_text.font_leaf.coverage_index :: bytes, coverage_offset, glyph_index :: call
    if coverage_index_value < 0:
        return empty_int_list :: :: call
    let sequence_count = u16_be_ref :: bytes, subtable_offset + 4 :: call
    if coverage_index_value >= sequence_count:
        return empty_int_list :: :: call
    let sequence_offset = subtable_offset + (u16_be_ref :: bytes, subtable_offset + 6 + (coverage_index_value * 2) :: call)
    let glyph_count = u16_be_ref :: bytes, sequence_offset :: call
    let mut out = empty_int_list :: :: call
    let mut part = 0
    while part < glyph_count:
        out :: (u16_be_ref :: bytes, sequence_offset + 2 + (part * 2) :: call) :: push
        part += 1
    return out

fn single_substitution_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let glyph_index = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call
    let matched = arcana_text.font_leaf.single_substitution_match :: request.bytes, subtable_offset, glyph_index :: call
    if not matched.found:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let substitute = matched.glyph_index
    let sequence_consumed = matched.sequence_consumed
    let mut replacement = empty_int_list :: :: call
    replacement :: substitute :: push
    let units = arcana_text.font_leaf.replace_range_with_glyphs :: request.units, (request.index, sequence_consumed), replacement :: call
    return arcana_text.font_leaf.lookup_apply_result :: units, true, 1 :: call

fn multiple_substitution_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let glyph_index = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call
    let replacement = arcana_text.font_leaf.multiple_substitution_glyphs :: request.bytes, subtable_offset, glyph_index :: call
    if replacement :: :: is_empty:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let units = arcana_text.font_leaf.replace_range_with_glyphs :: request.units, (request.index, 1), replacement :: call
    return arcana_text.font_leaf.lookup_apply_result :: units, true, (max_int :: (replacement :: :: len), 1 :: call) :: call

fn alternate_substitution_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let mut alternate_request = arcana_text.font_leaf.AlternateSubstitutionMatchRequest :: bytes = request.bytes, subtable_offset = subtable_offset, glyph_index = (arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call) :: call
    alternate_request.feature_value = request.feature_value
    let matched = arcana_text.font_leaf.alternate_substitution_match :: alternate_request :: call
    if not matched.found:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let substitute = matched.glyph_index
    let sequence_consumed = matched.sequence_consumed
    let mut replacement = empty_int_list :: :: call
    replacement :: substitute :: push
    let units = arcana_text.font_leaf.replace_range_with_glyphs :: request.units, (request.index, sequence_consumed), replacement :: call
    return arcana_text.font_leaf.lookup_apply_result :: units, true, 1 :: call

fn ligature_substitution_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let mut ligature_request = arcana_text.font_leaf.LigatureSubstitutionMatchRequest :: bytes = request.bytes, subtable_offset = subtable_offset, index = request.index :: call
    ligature_request.units = request.units
    let matched = arcana_text.font_leaf.ligature_substitution_match :: ligature_request :: call
    if not matched.found:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let substitute = matched.glyph_index
    let sequence_consumed = matched.sequence_consumed
    let mut replacement = empty_int_list :: :: call
    replacement :: substitute :: push
    let units = arcana_text.font_leaf.replace_range_with_glyphs :: request.units, (request.index, sequence_consumed), replacement :: call
    return arcana_text.font_leaf.lookup_apply_result :: units, true, 1 :: call

fn apply_context_lookup_records(read request: arcana_text.font_leaf.LookupApplyAtRequest, input_count: Int, record_block: (Int, Int)) -> arcana_text.font_leaf.LookupApplyResult:
    let records_offset = record_block.0
    let subst_count = record_block.1
    if subst_count <= 0:
        return arcana_text.font_leaf.lookup_apply_result :: request.units, true, (max_int :: input_count, 1 :: call) :: call
    let mut units = arcana_text.font_leaf.copy_gsub_units :: request.units :: call
    let mut record_index = 0
    while record_index < subst_count:
        let sequence_index = u16_be_ref :: request.bytes, records_offset + (record_index * 4) :: call
        let lookup_index = u16_be_ref :: request.bytes, records_offset + 2 + (record_index * 4) :: call
        let mut nested_request = arcana_text.font_leaf.LookupApplyAtRequest :: bytes = request.bytes, lookup_list_offset = request.lookup_list_offset, lookup_index = lookup_index :: call
        nested_request.units = units
        nested_request.index = request.index + sequence_index
        nested_request.feature_value = request.feature_value
        nested_request.depth = request.depth + 1
        let nested = arcana_text.font_leaf.apply_lookup_at :: nested_request :: call
        if nested.matched:
            units = nested.units
        record_index += 1
    return arcana_text.font_leaf.lookup_apply_result :: units, true, (max_int :: input_count, 1 :: call) :: call

fn context_substitution_format1_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let first = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call
    if first < 0:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let coverage_offset = subtable_offset + (u16_be_ref :: request.bytes, subtable_offset + 2 :: call)
    let coverage_index_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, first :: call
    if coverage_index_value < 0:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let rule_set_count = u16_be_ref :: request.bytes, subtable_offset + 4 :: call
    if coverage_index_value >= rule_set_count:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let rule_set_delta = u16_be_ref :: request.bytes, subtable_offset + 6 + (coverage_index_value * 2) :: call
    if rule_set_delta <= 0:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let rule_set_offset = subtable_offset + rule_set_delta
    let rule_count = u16_be_ref :: request.bytes, rule_set_offset :: call
    let total_units = request.units :: :: len
    let mut rule_index = 0
    while rule_index < rule_count:
        let rule_offset = rule_set_offset + (u16_be_ref :: request.bytes, rule_set_offset + 2 + (rule_index * 2) :: call)
        let glyph_count = u16_be_ref :: request.bytes, rule_offset :: call
        let subst_count = u16_be_ref :: request.bytes, rule_offset + 2 :: call
        if glyph_count > 0 and (request.index + glyph_count) <= total_units:
            let mut matched = true
            let mut component_index = 1
            while component_index < glyph_count and matched:
                let expected = u16_be_ref :: request.bytes, rule_offset + 4 + ((component_index - 1) * 2) :: call
                let actual = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index + component_index :: call
                if actual != expected:
                    matched = false
                component_index += 1
            if matched:
                let records_offset = rule_offset + 4 + ((glyph_count - 1) * 2)
                return arcana_text.font_leaf.apply_context_lookup_records :: request, glyph_count, (records_offset, subst_count) :: call
        rule_index += 1
    return arcana_text.font_leaf.lookup_apply_no_match :: :: call

fn context_substitution_format3_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let glyph_count = u16_be_ref :: request.bytes, subtable_offset + 2 :: call
    let subst_count = u16_be_ref :: request.bytes, subtable_offset + 4 :: call
    if glyph_count <= 0 or (request.index + glyph_count) > (request.units :: :: len):
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let mut matched = true
    let mut component_index = 0
    while component_index < glyph_count and matched:
        let coverage_offset = subtable_offset + (u16_be_ref :: request.bytes, subtable_offset + 6 + (component_index * 2) :: call)
        let glyph_index = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index + component_index :: call
        if (arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, glyph_index :: call) < 0:
            matched = false
        component_index += 1
    if not matched:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let records_offset = subtable_offset + 6 + (glyph_count * 2)
    return arcana_text.font_leaf.apply_context_lookup_records :: request, glyph_count, (records_offset, subst_count) :: call

fn context_substitution_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let format = u16_be_ref :: request.bytes, subtable_offset :: call
    if format == 1:
        return arcana_text.font_leaf.context_substitution_format1_apply :: request, subtable_offset :: call
    if format == 3:
        return arcana_text.font_leaf.context_substitution_format3_apply :: request, subtable_offset :: call
    return arcana_text.font_leaf.lookup_apply_no_match :: :: call

fn chain_context_substitution_format1_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let first = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index :: call
    if first < 0:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let coverage_offset = subtable_offset + (u16_be_ref :: request.bytes, subtable_offset + 2 :: call)
    let coverage_index_value = arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, first :: call
    if coverage_index_value < 0:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let rule_set_count = u16_be_ref :: request.bytes, subtable_offset + 4 :: call
    if coverage_index_value >= rule_set_count:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let rule_set_delta = u16_be_ref :: request.bytes, subtable_offset + 6 + (coverage_index_value * 2) :: call
    if rule_set_delta <= 0:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let rule_set_offset = subtable_offset + rule_set_delta
    let rule_count = u16_be_ref :: request.bytes, rule_set_offset :: call
    let total_units = request.units :: :: len
    let mut rule_index = 0
    while rule_index < rule_count:
        let rule_offset = rule_set_offset + (u16_be_ref :: request.bytes, rule_set_offset + 2 + (rule_index * 2) :: call)
        let mut cursor = rule_offset
        let backtrack_count = u16_be_ref :: request.bytes, cursor :: call
        cursor += 2
        if request.index >= backtrack_count:
            let mut matched = true
            let mut backtrack_index = 0
            while backtrack_index < backtrack_count and matched:
                let expected = u16_be_ref :: request.bytes, cursor + (backtrack_index * 2) :: call
                let actual = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index - 1 - backtrack_index :: call
                if actual != expected:
                    matched = false
                backtrack_index += 1
            cursor += backtrack_count * 2
            let input_count = u16_be_ref :: request.bytes, cursor :: call
            cursor += 2
            if input_count <= 0 or (request.index + input_count) > total_units:
                matched = false
            if matched:
                let mut input_index = 1
                while input_index < input_count and matched:
                    let expected = u16_be_ref :: request.bytes, cursor + ((input_index - 1) * 2) :: call
                    let actual = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index + input_index :: call
                    if actual != expected:
                        matched = false
                    input_index += 1
            cursor += (input_count - 1) * 2
            let lookahead_count = u16_be_ref :: request.bytes, cursor :: call
            cursor += 2
            if matched and (request.index + input_count + lookahead_count) > total_units:
                matched = false
            if matched:
                let mut lookahead_index = 0
                while lookahead_index < lookahead_count and matched:
                    let expected = u16_be_ref :: request.bytes, cursor + (lookahead_index * 2) :: call
                    let actual = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index + input_count + lookahead_index :: call
                    if actual != expected:
                        matched = false
                    lookahead_index += 1
            cursor += lookahead_count * 2
            let subst_count = u16_be_ref :: request.bytes, cursor :: call
            cursor += 2
            if matched:
                return arcana_text.font_leaf.apply_context_lookup_records :: request, input_count, (cursor, subst_count) :: call
        rule_index += 1
    return arcana_text.font_leaf.lookup_apply_no_match :: :: call

fn chain_context_substitution_format3_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let total_units = request.units :: :: len
    let mut cursor = subtable_offset + 2
    let backtrack_count = u16_be_ref :: request.bytes, cursor :: call
    cursor += 2
    if request.index < backtrack_count:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let mut backtrack_index = 0
    while backtrack_index < backtrack_count:
        let coverage_offset = subtable_offset + (u16_be_ref :: request.bytes, cursor + (backtrack_index * 2) :: call)
        let glyph_index = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index - 1 - backtrack_index :: call
        if (arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, glyph_index :: call) < 0:
            return arcana_text.font_leaf.lookup_apply_no_match :: :: call
        backtrack_index += 1
    cursor += backtrack_count * 2
    let input_count = u16_be_ref :: request.bytes, cursor :: call
    cursor += 2
    if input_count <= 0 or (request.index + input_count) > total_units:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let mut input_index = 0
    while input_index < input_count:
        let coverage_offset = subtable_offset + (u16_be_ref :: request.bytes, cursor + (input_index * 2) :: call)
        let glyph_index = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index + input_index :: call
        if (arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, glyph_index :: call) < 0:
            return arcana_text.font_leaf.lookup_apply_no_match :: :: call
        input_index += 1
    cursor += input_count * 2
    let lookahead_count = u16_be_ref :: request.bytes, cursor :: call
    cursor += 2
    if (request.index + input_count + lookahead_count) > total_units:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let mut lookahead_index = 0
    while lookahead_index < lookahead_count:
        let coverage_offset = subtable_offset + (u16_be_ref :: request.bytes, cursor + (lookahead_index * 2) :: call)
        let glyph_index = arcana_text.font_leaf.unit_glyph_index :: request.units, request.index + input_count + lookahead_index :: call
        if (arcana_text.font_leaf.coverage_index :: request.bytes, coverage_offset, glyph_index :: call) < 0:
            return arcana_text.font_leaf.lookup_apply_no_match :: :: call
        lookahead_index += 1
    cursor += lookahead_count * 2
    let subst_count = u16_be_ref :: request.bytes, cursor :: call
    cursor += 2
    return arcana_text.font_leaf.apply_context_lookup_records :: request, input_count, (cursor, subst_count) :: call

fn chain_context_substitution_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    let format = u16_be_ref :: request.bytes, subtable_offset :: call
    if format == 1:
        return arcana_text.font_leaf.chain_context_substitution_format1_apply :: request, subtable_offset :: call
    if format == 3:
        return arcana_text.font_leaf.chain_context_substitution_format3_apply :: request, subtable_offset :: call
    return arcana_text.font_leaf.lookup_apply_no_match :: :: call

fn subtable_apply(read request: arcana_text.font_leaf.LookupApplyAtRequest, lookup_type: Int, subtable_offset: Int) -> arcana_text.font_leaf.LookupApplyResult:
    if lookup_type == 1:
        return arcana_text.font_leaf.single_substitution_apply :: request, subtable_offset :: call
    if lookup_type == 2:
        return arcana_text.font_leaf.multiple_substitution_apply :: request, subtable_offset :: call
    if lookup_type == 3:
        return arcana_text.font_leaf.alternate_substitution_apply :: request, subtable_offset :: call
    if lookup_type == 4:
        return arcana_text.font_leaf.ligature_substitution_apply :: request, subtable_offset :: call
    if lookup_type == 5:
        return arcana_text.font_leaf.context_substitution_apply :: request, subtable_offset :: call
    if lookup_type == 6:
        return arcana_text.font_leaf.chain_context_substitution_apply :: request, subtable_offset :: call
    if lookup_type == 7:
        let extension_type = u16_be_ref :: request.bytes, subtable_offset + 2 :: call
        let nested_offset = subtable_offset + (u32_be_ref :: request.bytes, subtable_offset + 4 :: call)
        return arcana_text.font_leaf.subtable_apply :: request, extension_type, nested_offset :: call
    return arcana_text.font_leaf.lookup_apply_no_match :: :: call

fn apply_lookup_at(read request: arcana_text.font_leaf.LookupApplyAtRequest) -> arcana_text.font_leaf.LookupApplyResult:
    if request.depth > (arcana_text.font_leaf.lookup_apply_depth_limit :: :: call):
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let lookup_count = u16_be_ref :: request.bytes, request.lookup_list_offset :: call
    if request.lookup_index < 0 or request.lookup_index >= lookup_count:
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    if request.index < 0 or request.index >= (request.units :: :: len):
        return arcana_text.font_leaf.lookup_apply_no_match :: :: call
    let lookup_offset = request.lookup_list_offset + (u16_be_ref :: request.bytes, request.lookup_list_offset + 2 + (request.lookup_index * 2) :: call)
    let lookup_type = u16_be_ref :: request.bytes, lookup_offset :: call
    let subtable_count = u16_be_ref :: request.bytes, lookup_offset + 4 :: call
    let mut subtable_index = 0
    while subtable_index < subtable_count:
        let subtable_offset = lookup_offset + (u16_be_ref :: request.bytes, lookup_offset + 6 + (subtable_index * 2) :: call)
        let outcome = arcana_text.font_leaf.subtable_apply :: request, lookup_type, subtable_offset :: call
        if outcome.matched:
            return outcome
        subtable_index += 1
    return arcana_text.font_leaf.lookup_apply_no_match :: :: call

fn apply_lookup(read request: arcana_text.font_leaf.ApplyLookupRequest) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let mut units = arcana_text.font_leaf.copy_gsub_units :: request.units :: call
    let mut index = 0
    while index < (units :: :: len):
        let mut lookup_request = arcana_text.font_leaf.LookupApplyAtRequest :: bytes = request.bytes, lookup_list_offset = request.lookup_list_offset, lookup_index = request.lookup.lookup_index :: call
        lookup_request.units = units
        lookup_request.index = index
        lookup_request.feature_value = request.lookup.feature_value
        lookup_request.depth = 0
        let outcome = arcana_text.font_leaf.apply_lookup_at :: lookup_request :: call
        if outcome.matched:
            units = outcome.units
            index += max_int :: outcome.advance, 1 :: call
        else:
            index += 1
    return units

fn unit_glyphs(read units: List[arcana_text.font_leaf.GsubGlyphUnit]) -> List[Int]:
    let mut glyphs = empty_int_list :: :: call
    for unit in units:
        glyphs :: unit.glyph_index :: push
    return glyphs

fn any_glyph_in_coverage(read bytes: std.memory.ByteView, coverage_offset: Int, read glyphs: List[Int]) -> Bool:
    for glyph_index in glyphs:
        if glyph_index > 0 and (arcana_text.font_leaf.coverage_index :: bytes, coverage_offset, glyph_index :: call) >= 0:
            return true
    return false

fn lookup_candidate_key(lookup_index: Int, glyph_index: Int) -> Str:
    return (std.text.from_int :: lookup_index :: call) + ":" + (std.text.from_int :: glyph_index :: call)

fn lookup_type_for(read bytes: std.memory.ByteView, lookup_list_offset: Int, lookup_index: Int) -> Int:
    let lookup_count = u16_be_ref :: bytes, lookup_list_offset :: call
    if lookup_index < 0 or lookup_index >= lookup_count:
        return 0
    let lookup_offset = lookup_list_offset + (u16_be_ref :: bytes, lookup_list_offset + 2 + (lookup_index * 2) :: call)
    return u16_be_ref :: bytes, lookup_offset :: call

fn subtable_can_match_any(read bytes: std.memory.ByteView, payload: (Int, Int), read glyphs: List[Int]) -> Bool:
    let lookup_type = payload.0
    let subtable_offset = payload.1
    if lookup_type == 7:
        let extension_type = u16_be_ref :: bytes, subtable_offset + 2 :: call
        let nested_offset = subtable_offset + (u32_be_ref :: bytes, subtable_offset + 4 :: call)
        return arcana_text.font_leaf.subtable_can_match_any :: bytes, (extension_type, nested_offset), glyphs :: call
    if lookup_type == 1 or lookup_type == 2 or lookup_type == 3 or lookup_type == 4:
        let coverage_offset = subtable_offset + (u16_be_ref :: bytes, subtable_offset + 2 :: call)
        return arcana_text.font_leaf.any_glyph_in_coverage :: bytes, coverage_offset, glyphs :: call
    if lookup_type == 5:
        let format = u16_be_ref :: bytes, subtable_offset :: call
        if format == 1:
            let coverage_offset = subtable_offset + (u16_be_ref :: bytes, subtable_offset + 2 :: call)
            return arcana_text.font_leaf.any_glyph_in_coverage :: bytes, coverage_offset, glyphs :: call
        if format == 3:
            let glyph_count = u16_be_ref :: bytes, subtable_offset + 2 :: call
            if glyph_count <= 0:
                return false
            let coverage_offset = subtable_offset + (u16_be_ref :: bytes, subtable_offset + 6 :: call)
            return arcana_text.font_leaf.any_glyph_in_coverage :: bytes, coverage_offset, glyphs :: call
        return true
    if lookup_type == 6:
        let format = u16_be_ref :: bytes, subtable_offset :: call
        if format == 1:
            let coverage_offset = subtable_offset + (u16_be_ref :: bytes, subtable_offset + 2 :: call)
            return arcana_text.font_leaf.any_glyph_in_coverage :: bytes, coverage_offset, glyphs :: call
        if format == 3:
            let mut cursor = subtable_offset + 2
            let backtrack_count = u16_be_ref :: bytes, cursor :: call
            cursor += 2 + (backtrack_count * 2)
            let input_count = u16_be_ref :: bytes, cursor :: call
            cursor += 2
            if input_count <= 0:
                return false
            let coverage_offset = subtable_offset + (u16_be_ref :: bytes, cursor :: call)
            return arcana_text.font_leaf.any_glyph_in_coverage :: bytes, coverage_offset, glyphs :: call
        return true
    return true

fn lookup_can_match_any_glyph(read bytes: std.memory.ByteView, payload: (Int, Int), read glyphs: List[Int]) -> Bool:
    let lookup_list_offset = payload.0
    let lookup_index = payload.1
    let lookup_count = u16_be_ref :: bytes, lookup_list_offset :: call
    if lookup_index < 0 or lookup_index >= lookup_count:
        return false
    let lookup_offset = lookup_list_offset + (u16_be_ref :: bytes, lookup_list_offset + 2 + (lookup_index * 2) :: call)
    let lookup_type = u16_be_ref :: bytes, lookup_offset :: call
    let subtable_count = u16_be_ref :: bytes, lookup_offset + 4 :: call
    let mut subtable_index = 0
    while subtable_index < subtable_count:
        let subtable_offset = lookup_offset + (u16_be_ref :: bytes, lookup_offset + 6 + (subtable_index * 2) :: call)
        if arcana_text.font_leaf.subtable_can_match_any :: bytes, (lookup_type, subtable_offset), glyphs :: call:
            return true
        subtable_index += 1
    return false

fn lookup_can_match_glyph(edit face: arcana_text.font_leaf.FontFaceState, payload: (Int, Int), glyph_index: Int) -> Bool:
    if glyph_index <= 0:
        return false
    let lookup_index = payload.1
    let cache_key = arcana_text.font_leaf.lookup_candidate_key :: lookup_index, glyph_index :: call
    if face.gsub_candidate_cache :: cache_key :: has:
        return face.gsub_candidate_cache :: cache_key :: get
    let mut glyphs = empty_int_list :: :: call
    glyphs :: glyph_index :: push
    let matched = arcana_text.font_leaf.lookup_can_match_any_glyph :: face.font_view, payload, glyphs :: call
    face.gsub_candidate_cache :: cache_key, matched :: set
    return matched

fn lookup_can_match_unit_glyphs(edit face: arcana_text.font_leaf.FontFaceState, payload: (Int, Int), read glyphs: List[Int]) -> Bool:
    for glyph_index in glyphs:
        if arcana_text.font_leaf.lookup_can_match_glyph :: face, payload, glyph_index :: call:
            return true
    return false

export fn gsub_substitute(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.GsubSubstituteRequest) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let mut units = arcana_text.font_leaf.default_gsub_units :: request.glyphs :: call
    if face.gsub_offset < 0 or face.gsub_length <= 0 or (request.glyphs :: :: is_empty):
        return units
    let mut lookup_request = arcana_text.font_leaf.LookupRefsForFeaturesRequest :: script_tag = request.script_tag, language_tag = request.language_tag, features = (std.collections.list.new[arcana_text.types.FontFeature] :: :: call) :: call
    lookup_request.features = request.features
    let lookups = arcana_text.font_leaf.lookup_refs_for_features :: face, lookup_request :: call
    if lookups :: :: is_empty:
        return units
    let lookup_list_offset = face.gsub_offset + (u16_be_ref :: face.font_view, face.gsub_offset + 8 :: call)
    font_leaf_probe_append :: ("gsub_substitute:start glyphs=" + (std.text.from_int :: (request.glyphs :: :: len) :: call) + " lookups=" + (std.text.from_int :: (lookups :: :: len) :: call)) :: call
    for lookup in lookups:
        let lookup_type = arcana_text.font_leaf.lookup_type_for :: face.font_view, lookup_list_offset, lookup.lookup_index :: call
        font_leaf_probe_append :: ("gsub_substitute:lookup_start " + (std.text.from_int :: lookup.lookup_index :: call) + " type=" + (std.text.from_int :: lookup_type :: call)) :: call
        if lookup_type != 5 and lookup_type != 6 and lookup_type != 7:
            let current_glyphs = arcana_text.font_leaf.unit_glyphs :: units :: call
            if not (arcana_text.font_leaf.lookup_can_match_unit_glyphs :: face, (lookup_list_offset, lookup.lookup_index), current_glyphs :: call):
                font_leaf_probe_append :: ("gsub_substitute:lookup_skip " + (std.text.from_int :: lookup.lookup_index :: call)) :: call
                continue
        let mut apply_request = arcana_text.font_leaf.ApplyLookupRequest :: bytes = face.font_view, lookup_list_offset = lookup_list_offset, lookup = lookup :: call
        apply_request.units = units
        units = arcana_text.font_leaf.apply_lookup :: apply_request :: call
        font_leaf_probe_append :: ("gsub_substitute:lookup_done " + (std.text.from_int :: lookup.lookup_index :: call) + " units=" + (std.text.from_int :: (units :: :: len) :: call)) :: call
    font_leaf_probe_append :: ("gsub_substitute:done units=" + (std.text.from_int :: (units :: :: len) :: call)) :: call
    return units

fn kern_pair_key(left_glyph: Int, right_glyph: Int) -> Str:
    return (std.text.from_int :: left_glyph :: call) + ":" + (std.text.from_int :: right_glyph :: call)

fn gpos_single_key(read request: arcana_text.font_leaf.SingleAdjustmentRequest) -> Str:
    let mut key = (std.text.from_int :: request.glyph :: call) + ":" + request.script_tag + ":" + request.language_tag
    for feature in request.features:
        key = key + ":" + feature.tag + "=" + (std.text.from_int :: (arcana_text.font_leaf.feature_value :: request.features, feature.tag :: call) :: call)
    return key

fn gpos_pair_key(read request: arcana_text.font_leaf.PairAdjustmentRequest) -> Str:
    let mut key = (arcana_text.font_leaf.kern_pair_key :: request.left_glyph, request.right_glyph :: call) + ":" + request.script_tag + ":" + request.language_tag
    for feature in request.features:
        key = key + ":" + feature.tag + "=" + (std.text.from_int :: (arcana_text.font_leaf.feature_value :: request.features, feature.tag :: call) :: call)
    return key

fn gpos_single_value_uncached(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.SingleAdjustmentRequest) -> arcana_text.font_leaf.PairPlacement:
    if face.gpos_offset < 0 or face.gpos_length <= 0 or request.glyph <= 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mut lookup_request = arcana_text.font_leaf.LookupRefsForFeaturesRequest :: script_tag = request.script_tag, language_tag = request.language_tag, features = (std.collections.list.new[arcana_text.types.FontFeature] :: :: call) :: call
    lookup_request.features = request.features
    let lookups = arcana_text.font_leaf.position_lookup_refs_for_features :: face, lookup_request :: call
    if lookups :: :: is_empty:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let lookup_list_offset = face.gpos_offset + (u16_be_ref :: face.font_view, face.gpos_offset + 8 :: call)
    let mut total = arcana_text.font_leaf.empty_pair_placement :: :: call
    for lookup in lookups:
        let mut adjust_request = arcana_text.font_leaf.PositionLookupAdjustRequest :: bytes = face.font_view, lookup_list_offset = lookup_list_offset, lookup = lookup :: call
        adjust_request.left_glyph = request.glyph
        adjust_request.right_glyph = 0
        let value = arcana_text.font_leaf.position_lookup_adjust :: adjust_request :: call
        total.x_offset += value.x_offset
        total.y_offset += value.y_offset
        total.x_advance += value.x_advance
        total.y_advance += value.y_advance
        if value.zero_advance:
            total.zero_advance = true
        if value.attach_to_left_origin:
            total.attach_to_left_origin = true
    return total

fn gpos_pair_value_uncached(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.PairAdjustmentRequest) -> arcana_text.font_leaf.PairPlacement:
    if face.gpos_offset < 0 or face.gpos_length <= 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mut lookup_request = arcana_text.font_leaf.LookupRefsForFeaturesRequest :: script_tag = request.script_tag, language_tag = request.language_tag, features = (std.collections.list.new[arcana_text.types.FontFeature] :: :: call) :: call
    lookup_request.features = request.features
    let lookups = arcana_text.font_leaf.position_lookup_refs_for_features :: face, lookup_request :: call
    if lookups :: :: is_empty:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let lookup_list_offset = face.gpos_offset + (u16_be_ref :: face.font_view, face.gpos_offset + 8 :: call)
    let mut total = arcana_text.font_leaf.empty_pair_placement :: :: call
    for lookup in lookups:
        let mut adjust_request = arcana_text.font_leaf.PositionLookupAdjustRequest :: bytes = face.font_view, lookup_list_offset = lookup_list_offset, lookup = lookup :: call
        adjust_request.left_glyph = request.left_glyph
        adjust_request.right_glyph = request.right_glyph
        let value = arcana_text.font_leaf.position_lookup_adjust :: adjust_request :: call
        total.x_offset += value.x_offset
        total.y_offset += value.y_offset
        total.x_advance += value.x_advance
        total.y_advance += value.y_advance
        if value.zero_advance:
            total.zero_advance = true
        if value.attach_to_left_origin:
            total.attach_to_left_origin = true
    return total

fn scale_placement(read payload: (arcana_text.font_leaf.PairPlacement, arcana_text.font_leaf.FontFaceState, arcana_text.font_leaf.FaceTraits, Int)) -> arcana_text.font_leaf.PairPlacement:
    let raw = payload.0
    let face = payload.1
    let traits = payload.2
    let font_size = payload.3
    let mut scaled = arcana_text.font_leaf.empty_pair_placement :: :: call
    scaled.x_offset = scale_x :: raw.x_offset, font_size, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call
    scaled.y_offset = scale_y :: raw.y_offset, font_size, face.units_per_em :: call
    scaled.x_advance = scale_x :: raw.x_advance, font_size, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call
    scaled.y_advance = scale_y :: raw.y_advance, font_size, face.units_per_em :: call
    scaled.zero_advance = raw.zero_advance
    scaled.attach_to_left_origin = raw.attach_to_left_origin
    return scaled

fn kern_pair_value_uncached(read face: arcana_text.font_leaf.FontFaceState, left_glyph: Int, right_glyph: Int) -> Int:
    if face.kern_offset < 0 or face.kern_length <= 0:
        return 0
    let table_end = face.kern_offset + face.kern_length
    if table_end > (face.font_view :: :: len) or face.kern_offset + 4 > table_end:
        return 0
    let table_count = u16_be_ref :: face.font_view, face.kern_offset + 2 :: call
    let mut cursor = face.kern_offset + 4
    let mut table_index = 0
    while table_index < table_count and cursor + 6 <= table_end:
        let subtable_length = u16_be_ref :: face.font_view, cursor + 2 :: call
        let coverage = u16_be_ref :: face.font_view, cursor + 4 :: call
        let format = coverage / 256
        let flags = coverage % 256
        let horizontal = (flags % 2) == 1
        if subtable_length > 6 and format == 0 and horizontal and cursor + subtable_length <= table_end:
            let pair_count = u16_be_ref :: face.font_view, cursor + 6 :: call
            let mut pair_index = 0
            while pair_index < pair_count:
                let record = cursor + 14 + (pair_index * 6)
                if record + 6 > cursor + subtable_length:
                    break
                let left = u16_be_ref :: face.font_view, record :: call
                let right = u16_be_ref :: face.font_view, record + 2 :: call
                if left == left_glyph and right == right_glyph:
                    return i16_be_ref :: face.font_view, record + 4 :: call
                pair_index += 1
        if subtable_length <= 0:
            break
        cursor += subtable_length
        table_index += 1
    return 0

fn scaled_pair_placement(read raw: arcana_text.font_leaf.PairPlacement, read face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.PairAdjustmentRequest) -> arcana_text.font_leaf.PairPlacement:
    return arcana_text.font_leaf.scale_placement :: (raw, face, request.traits, request.font_size) :: call

fn scaled_single_placement(read raw: arcana_text.font_leaf.PairPlacement, read face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.SingleAdjustmentRequest) -> arcana_text.font_leaf.PairPlacement:
    return arcana_text.font_leaf.scale_placement :: (raw, face, request.traits, request.font_size) :: call

export fn position_lookup_refs(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.LookupRefsForFeaturesRequest) -> List[arcana_text.font_leaf.GsubLookupRef]:
    return arcana_text.font_leaf.position_lookup_refs_for_features :: face, request :: call

export fn position_lookup_type(read face: arcana_text.font_leaf.FontFaceState, read lookup: arcana_text.font_leaf.GsubLookupRef) -> Int:
    if face.gpos_offset < 0 or face.gpos_length <= 0:
        return 0
    let lookup_list_offset = face.gpos_offset + (u16_be_ref :: face.font_view, face.gpos_offset + 8 :: call)
    let lookup_count = u16_be_ref :: face.font_view, lookup_list_offset :: call
    if lookup.lookup_index < 0 or lookup.lookup_index >= lookup_count:
        return 0
    let lookup_offset = lookup_list_offset + (u16_be_ref :: face.font_view, lookup_list_offset + 2 + (lookup.lookup_index * 2) :: call)
    let lookup_type = u16_be_ref :: face.font_view, lookup_offset :: call
    if lookup_type != 9:
        return lookup_type
    let subtable_count = u16_be_ref :: face.font_view, lookup_offset + 4 :: call
    if subtable_count <= 0:
        return lookup_type
    let subtable_offset = lookup_offset + (u16_be_ref :: face.font_view, lookup_offset + 6 :: call)
    return u16_be_ref :: face.font_view, subtable_offset + 2 :: call

export fn glyph_class(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    if glyph_index <= 0:
        return 0
    let class_def_offset = arcana_text.font_leaf.gdef_glyph_class_def_offset :: face :: call
    if class_def_offset <= 0:
        return 0
    return arcana_text.font_leaf.class_def_index :: face.font_view, class_def_offset, glyph_index :: call

export fn supports_script(read face: arcana_text.font_leaf.FontFaceState, script_tag: Str) -> Bool:
    if script_tag == "" or script_tag == "DFLT":
        return true
    if face.gsub_offset >= 0 and (arcana_text.font_leaf.script_table_offset :: face.font_view, face.gsub_offset, script_tag :: call) >= 0:
        return true
    if face.gpos_offset >= 0 and (arcana_text.font_leaf.script_table_offset :: face.font_view, face.gpos_offset, script_tag :: call) >= 0:
        return true
    if face.gsub_offset >= 0 and (arcana_text.font_leaf.script_table_offset :: face.font_view, face.gsub_offset, "DFLT" :: call) >= 0:
        return true
    if face.gpos_offset >= 0 and (arcana_text.font_leaf.script_table_offset :: face.font_view, face.gpos_offset, "DFLT" :: call) >= 0:
        return true
    return false

export fn lookup_placement(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.LookupPlacementRequest) -> arcana_text.font_leaf.PairPlacement:
    if face.gpos_offset < 0 or face.gpos_length <= 0 or request.left_glyph <= 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let lookup_list_offset = face.gpos_offset + (u16_be_ref :: face.font_view, face.gpos_offset + 8 :: call)
    let mut adjust_request = arcana_text.font_leaf.PositionLookupAdjustRequest :: bytes = face.font_view, lookup_list_offset = lookup_list_offset, lookup = request.lookup :: call
    adjust_request.left_glyph = request.left_glyph
    adjust_request.right_glyph = request.right_glyph
    let raw = arcana_text.font_leaf.position_lookup_adjust :: adjust_request :: call
    if not (arcana_text.font_leaf.placement_has_effect :: raw :: call):
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    return arcana_text.font_leaf.scale_placement :: (raw, face, request.traits, request.font_size) :: call

export fn single_placement(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.SingleAdjustmentRequest) -> arcana_text.font_leaf.PairPlacement:
    if request.glyph <= 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let cache_key = arcana_text.font_leaf.gpos_single_key :: request :: call
    let mut raw = arcana_text.font_leaf.empty_pair_placement :: :: call
    if face.gpos_single_cache :: cache_key :: has:
        raw = face.gpos_single_cache :: cache_key :: get
    else:
        let value = arcana_text.font_leaf.gpos_single_value_uncached :: face, request :: call
        face.gpos_single_cache :: cache_key, value :: set
        raw = face.gpos_single_cache :: cache_key :: get
    if raw.x_offset == 0 and raw.y_offset == 0 and raw.x_advance == 0 and raw.y_advance == 0 and not raw.zero_advance and not raw.attach_to_left_origin:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    return arcana_text.font_leaf.scaled_single_placement :: raw, face, request :: call

export fn pair_placement(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.PairAdjustmentRequest) -> arcana_text.font_leaf.PairPlacement:
    if request.left_glyph <= 0 or request.right_glyph <= 0:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    let mut raw = arcana_text.font_leaf.empty_pair_placement :: :: call
    let gpos_key = arcana_text.font_leaf.gpos_pair_key :: request :: call
    if face.gpos_pair_cache :: gpos_key :: has:
        raw = face.gpos_pair_cache :: gpos_key :: get
    else:
        let value = arcana_text.font_leaf.gpos_pair_value_uncached :: face, request :: call
        face.gpos_pair_cache :: gpos_key, value :: set
        raw = face.gpos_pair_cache :: gpos_key :: get
    if raw.x_offset == 0 and raw.y_offset == 0 and raw.x_advance == 0 and raw.y_advance == 0 and not raw.zero_advance and not raw.attach_to_left_origin:
        let kern_key = arcana_text.font_leaf.kern_pair_key :: request.left_glyph, request.right_glyph :: call
        if face.kern_pair_cache :: kern_key :: has:
            raw = face.kern_pair_cache :: kern_key :: get
        else:
            let kern = arcana_text.font_leaf.kern_pair_value_uncached :: face, request.left_glyph, request.right_glyph :: call
            let mut kern_value = arcana_text.font_leaf.PairPlacement :: x_offset = kern, y_offset = 0, x_advance = kern :: call
            kern_value.y_advance = 0
            kern_value.zero_advance = false
            kern_value.attach_to_left_origin = false
            face.kern_pair_cache :: kern_key, kern_value :: set
            raw = face.kern_pair_cache :: kern_key :: get
    if raw.x_offset == 0 and raw.y_offset == 0 and raw.x_advance == 0 and raw.y_advance == 0 and not raw.zero_advance and not raw.attach_to_left_origin:
        return arcana_text.font_leaf.empty_pair_placement :: :: call
    return arcana_text.font_leaf.scaled_pair_placement :: raw, face, request :: call

export fn pair_adjustment(edit face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.PairAdjustmentRequest) -> Int:
    let placement = arcana_text.font_leaf.pair_placement :: face, request :: call
    return placement.x_advance

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
    return glyph_index_from_cmap12_view :: face, codepoint :: call

fn glyph_index_from_cmap12_view(read face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    return glyph_index_from_cmap12_offset_ref :: face.font_view, face.cmap12_offset, codepoint :: call

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
    return glyph_index_from_cmap4_view :: face, codepoint :: call

fn glyph_index_from_cmap4_view(read face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    return glyph_index_from_cmap4_offset_ref :: face.font_view, face.cmap4_offset, codepoint :: call

fn glyph_index_from_segment_cache(read lookup: arcana_text.font_leaf.Cmap4Lookup) -> Int:
    for segment in lookup.segments:
        if lookup.codepoint < segment.start_code or lookup.codepoint > segment.end_code:
            continue
        if segment.direct_map:
            return positive_mod :: lookup.codepoint + segment.id_delta, 65536 :: call
        let glyph_index = segment.glyph_base_index + (lookup.codepoint - segment.start_code)
        if glyph_index < 0 or glyph_index >= (lookup.glyphs :: :: len):
            return 0
        let raw = byte_at_or_zero :: lookup.glyphs, glyph_index :: call
        if raw == 0:
            return 0
        return positive_mod :: raw + segment.id_delta, 65536 :: call
    return 0

fn glyph_index_from_cmap4(read face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    if (face.cmap4_segments :: :: len) > 0:
        return glyph_index_from_segment_cache :: (cmap4_lookup :: face.cmap4_segments, face.cmap4_glyphs, codepoint :: call) :: call
    if face.cmap4_offset >= 0 or face.cmap12_offset >= 0:
        let format4 = glyph_index_from_cmap4_view :: face, codepoint :: call
        if format4 > 0:
            return format4
        return glyph_index_from_cmap12_view :: face, codepoint :: call
    return 0

fn ensure_cmap_state(edit face: arcana_text.font_leaf.FontFaceState):
    if (face.cmap12_groups :: :: len) > 0 or (face.cmap4_segments :: :: len) > 0:
        return
    if face.cmap_table_offset < 0 or face.cmap_table_length <= 0:
        return
    font_leaf_probe_append :: "ensure_cmap_state:start" :: call
    let parsed = parse_cmap_ref :: face.font_view, (face.cmap_table_offset, face.cmap_table_length) :: call
    face.cmap4_segments = parsed.segments
    face.cmap4_glyphs = parsed.glyphs
    face.cmap12_groups = parsed.groups
    font_leaf_probe_append :: ("ensure_cmap_state:done segs=" + (std.text.from_int :: (face.cmap4_segments :: :: len) :: call) + " groups=" + (std.text.from_int :: (face.cmap12_groups :: :: len) :: call)) :: call

fn glyph_index_for_codepoint_uncached(edit face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    if (face.cmap12_groups :: :: len) > 0:
        for group in face.cmap12_groups:
            if codepoint >= group.start_code and codepoint <= group.end_code:
                return group.start_glyph + (codepoint - group.start_code)
    if (face.cmap4_segments :: :: len) > 0:
        let format4 = glyph_index_from_cmap4 :: face, codepoint :: call
        if format4 > 0:
            return format4
    if face.cmap4_offset >= 0 or face.cmap12_offset >= 0:
        if codepoint >= 0 and codepoint < 65536:
            let format4 = glyph_index_from_cmap4_view :: face, codepoint :: call
            if format4 > 0:
                return format4
        let format12 = glyph_index_from_cmap12_view :: face, codepoint :: call
        if format12 > 0:
            return format12
    return glyph_index_from_cmap4 :: face, codepoint :: call

export fn glyph_index_for_codepoint(edit face: arcana_text.font_leaf.FontFaceState, codepoint: Int) -> Int:
    font_leaf_probe_append :: ("glyph_index_for_codepoint:start cp=" + (std.text.from_int :: codepoint :: call)) :: call
    if face.glyph_index_cache :: codepoint :: has:
        let cached = face.glyph_index_cache :: codepoint :: get
        font_leaf_probe_append :: ("glyph_index_for_codepoint:cache " + (std.text.from_int :: cached :: call)) :: call
        return cached
    let glyph = glyph_index_for_codepoint_uncached :: face, codepoint :: call
    face.glyph_index_cache :: codepoint, glyph :: set
    font_leaf_probe_append :: ("glyph_index_for_codepoint:done " + (std.text.from_int :: glyph :: call)) :: call
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

export fn supports_text(edit face: arcana_text.font_leaf.FontFaceState, read text: Str) -> Bool:
    let total = std.text.len_bytes :: text :: call
    let mut index = 0
    while index < total:
        let scalar_end = arcana_text.text_units.next_scalar_end :: text, index :: call
        let scalar = std.text.slice_bytes :: text, index, scalar_end :: call
        if scalar != " " and scalar != "\t" and scalar != "\n" and scalar != "\r":
            if (glyph_index_for_codepoint :: face, (utf8_codepoint :: scalar :: call) :: call) <= 0:
                return false
        index = scalar_end
    return true

fn outline_value(advance_width: Int, left_side_bearing: Int) -> arcana_text.font_leaf.GlyphOutline:
    let mut out = arcana_text.font_leaf.GlyphOutline :: advance_width = advance_width, left_side_bearing = left_side_bearing, x_min = 0 :: call
    out.y_min = 0
    out.x_max = 0
    out.y_max = 0
    out.contours = empty_contours :: :: call
    out.components = empty_components :: :: call
    out.empty = true
    return out

fn contour_value() -> arcana_text.font_leaf.FontContour:
    return arcana_text.font_leaf.FontContour :: points = (empty_points :: :: call) :: call

fn point_value(x: Int, y: Int, on_curve: Bool) -> arcana_text.font_leaf.FontPoint:
    return arcana_text.font_leaf.FontPoint :: x = x, y = y, on_curve = on_curve :: call

fn line_segment(start: (Int, Int), end: (Int, Int)) -> arcana_text.font_leaf.LineSegment:
    return arcana_text.font_leaf.LineSegment :: start = start, end = end :: call

fn glyph_offset_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> (Int, Int):
    if glyph_index < 0 or glyph_index >= face.glyph_count:
        return (0, 0)
    if (face.loca_offsets :: :: len) > glyph_index:
        let start = byte_at_or_zero :: face.loca_offsets, glyph_index :: call
        let next = byte_at_or_zero :: face.loca_offsets, glyph_index + 1 :: call
        return (start, next)
    return match face.loca_format:
        0 => (((u16_be_ref :: face.font_view, face.loca_offset + (glyph_index * 2) :: call) * 2), ((u16_be_ref :: face.font_view, face.loca_offset + ((glyph_index + 1) * 2) :: call) * 2))
        _ => ((u32_be_ref :: face.font_view, face.loca_offset + (glyph_index * 4) :: call), (u32_be_ref :: face.font_view, face.loca_offset + ((glyph_index + 1) * 4) :: call))

fn ensure_loca_offsets(edit face: arcana_text.font_leaf.FontFaceState):
    if (face.loca_offsets :: :: len) > 0:
        return
    let mut out = empty_int_list :: :: call
    let total = face.glyph_count + 1
    let mut index = 0
    while index < total:
        let offset = match face.loca_format:
            0 => (u16_be_ref :: face.font_view, face.loca_offset + (index * 2) :: call) * 2
            _ => u32_be_ref :: face.font_view, face.loca_offset + (index * 4) :: call
        out :: offset :: push
        index += 1
    face.loca_offsets = std.collections.array.from_list[Int] :: out :: call

fn advance_width_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    font_leaf_probe_append :: ("advance_width_for:start glyph=" + (std.text.from_int :: glyph_index :: call) + " hmetrics=" + (std.text.from_int :: face.hmetric_count :: call) + " cache=" + (std.text.from_int :: (face.advance_widths :: :: len) :: call) + " hmtx=" + (std.text.from_int :: face.hmtx_offset :: call)) :: call
    if glyph_index < 0:
        return 0
    if (face.advance_widths :: :: len) > 0:
        let cached_index = min_int :: glyph_index, (face.advance_widths :: :: len) - 1 :: call
        font_leaf_probe_append :: ("advance_width_for:cache_hit index=" + (std.text.from_int :: cached_index :: call)) :: call
        return byte_at_or_zero :: face.advance_widths, cached_index :: call
    if face.hmetric_count <= 0:
        font_leaf_probe_append :: "advance_width_for:no_hmetrics" :: call
        return 0
    let metric_index = min_int :: glyph_index, face.hmetric_count - 1 :: call
    font_leaf_probe_append :: ("advance_width_for:metric_index " + (std.text.from_int :: metric_index :: call)) :: call
    font_leaf_probe_append :: "advance_width_for:view_read" :: call
    let advance = u16_be_window :: face.font_view, face.hmtx_offset + (metric_index * 4) :: call
    font_leaf_probe_append :: ("advance_width_for:view_done " + (std.text.from_int :: advance :: call)) :: call
    return advance

fn left_side_bearing_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    if glyph_index < 0:
        return 0
    if glyph_index < (face.left_side_bearings :: :: len):
        return byte_at_or_zero :: face.left_side_bearings, glyph_index :: call
    if face.hmetric_count <= 0:
        return 0
    if glyph_index < face.hmetric_count:
        return i16_be_ref :: face.font_view, face.hmtx_offset + (glyph_index * 4) + 2 :: call
    let extra_index = glyph_index - face.hmetric_count
    return i16_be_ref :: face.font_view, face.hmtx_offset + (face.hmetric_count * 4) + (extra_index * 2) :: call

fn vertical_advance_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    if glyph_index < 0:
        return 0
    if (face.vertical_advances :: :: len) > 0:
        let cached_index = min_int :: glyph_index, (face.vertical_advances :: :: len) - 1 :: call
        return byte_at_or_zero :: face.vertical_advances, cached_index :: call
    if face.vmetric_count <= 0 or face.vmtx_offset < 0:
        return advance_width_for :: face, glyph_index :: call
    let metric_index = min_int :: glyph_index, face.vmetric_count - 1 :: call
    return u16_be_ref :: face.font_view, face.vmtx_offset + (metric_index * 4) :: call

fn top_side_bearing_for(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    if glyph_index < 0:
        return 0
    if glyph_index < (face.top_side_bearings :: :: len):
        return byte_at_or_zero :: face.top_side_bearings, glyph_index :: call
    if face.vmetric_count <= 0 or face.vmtx_offset < 0:
        return 0
    if glyph_index < face.vmetric_count:
        return i16_be_ref :: face.font_view, face.vmtx_offset + (glyph_index * 4) + 2 :: call
    let extra_index = glyph_index - face.vmetric_count
    return i16_be_ref :: face.font_view, face.vmtx_offset + (face.vmetric_count * 4) + (extra_index * 2) :: call

fn decode_flags(read bytes: std.memory.ByteView, start: Int, count: Int) -> (Array[Int], Int):
    let mut flags = empty_int_list :: :: call
    let mut cursor = start
    while (flags :: :: len) < count:
        let flag = byte_at_or_zero_ref :: bytes, cursor :: call
        cursor += 1
        flags :: flag :: push
        if (flag % 16) >= 8:
            let repeats = byte_at_or_zero_ref :: bytes, cursor :: call
            cursor += 1
            let mut index = 0
            while index < repeats:
                flags :: flag :: push
                index += 1
    return ((std.collections.array.from_list[Int] :: flags :: call), cursor)

fn decode_coordinates(read spec: arcana_text.font_leaf.CoordinateDecodeSpec) -> (Array[Int], Int):
    font_leaf_probe_append :: ("decode_coordinates:start count=" + (std.text.from_int :: spec.count :: call) + " cursor=" + (std.text.from_int :: spec.cursor :: call)) :: call
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
            delta = byte_at_or_zero_ref :: spec.bytes, at :: call
            at += 1
            if not same:
                delta = 0 - delta
        else:
            if not same:
                delta = i16_be_ref :: spec.bytes, at :: call
                at += 2
        current += delta
        out :: current :: push
        index += 1
    font_leaf_probe_append :: ("decode_coordinates:done at=" + (std.text.from_int :: at :: call)) :: call
    return ((std.collections.array.from_list[Int] :: out :: call), at)

fn parse_simple_outline(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int, glyph_offset: Int) -> arcana_text.font_leaf.GlyphOutline:
    let start = face.glyf_offset + glyph_offset
    let contour_count = i16_be_ref :: face.font_view, start :: call
    let advance_width = advance_width_for :: face, glyph_index :: call
    let left_side_bearing = left_side_bearing_for :: face, glyph_index :: call
    let mut outline = outline_value :: advance_width, left_side_bearing :: call
    outline.x_min = i16_be_ref :: face.font_view, start + 2 :: call
    outline.y_min = i16_be_ref :: face.font_view, start + 4 :: call
    outline.x_max = i16_be_ref :: face.font_view, start + 6 :: call
    outline.y_max = i16_be_ref :: face.font_view, start + 8 :: call
    if contour_count <= 0:
        outline.empty = true
        return outline
    let mut end_points = empty_int_list :: :: call
    let mut cursor = start + 10
    let mut contour_index = 0
    while contour_index < contour_count:
        end_points :: (u16_be_ref :: face.font_view, cursor :: call) :: push
        cursor += 2
        contour_index += 1
    let point_count = (int_list_at_or_zero :: end_points, (end_points :: :: len) - 1 :: call) + 1
    font_leaf_probe_append :: ("parse_simple:points=" + (std.text.from_int :: point_count :: call)) :: call
    let instruction_len = u16_be_ref :: face.font_view, cursor :: call
    cursor += 2 + instruction_len
    let decoded = decode_flags :: face.font_view, cursor, point_count :: call
    let flags = decoded.0
    cursor = decoded.1
    font_leaf_probe_append :: ("parse_simple:flags=" + (std.text.from_int :: (flags :: :: len) :: call)) :: call
    let mut x_spec = coordinate_decode_spec :: face.font_view, cursor, flags :: call
    x_spec.count = point_count
    x_spec.short_mask = 2
    x_spec.same_mask = 16
    let x_decoded = decode_coordinates :: x_spec :: call
    let xs = x_decoded.0
    cursor = x_decoded.1
    font_leaf_probe_append :: ("parse_simple:xs=" + (std.text.from_int :: (xs :: :: len) :: call)) :: call
    let mut y_spec = coordinate_decode_spec :: face.font_view, cursor, flags :: call
    y_spec.count = point_count
    y_spec.short_mask = 4
    y_spec.same_mask = 32
    let y_decoded = decode_coordinates :: y_spec :: call
    let ys = y_decoded.0
    font_leaf_probe_append :: ("parse_simple:ys=" + (std.text.from_int :: (ys :: :: len) :: call)) :: call
    let mut first_point = 0
    for end_point in end_points:
        let mut contour = contour_value :: :: call
        let mut point_index = first_point
        while point_index <= end_point:
            let point_x = byte_at_or_zero :: xs, point_index :: call
            let point_y = byte_at_or_zero :: ys, point_index :: call
            let on_curve = ((byte_at_or_zero :: flags, point_index :: call) % 2) == 1
            let point = point_value :: point_x, point_y, on_curve :: call
            contour.points :: point :: push
            point_index += 1
        outline.contours :: contour :: push
        first_point = end_point + 1
    outline.empty = false
    return outline

fn transform_point(point: (Int, Int), read matrix: arcana_text.font_leaf.AffineMatrix, offset: (Int, Int)) -> (Int, Int):
    let x = ((matrix.xx * point.0) + (matrix.xy * point.1)) / 16384 + offset.0
    let y = ((matrix.yx * point.0) + (matrix.yy * point.1)) / 16384 + offset.1
    return (x, y)

fn append_outline_contours(edit out: arcana_text.font_leaf.GlyphOutline, read payload: (arcana_text.font_leaf.GlyphOutline, arcana_text.font_leaf.AffineMatrix, (Int, Int))):
    let source = payload.0
    let matrix = payload.1
    let offset = payload.2
    for source_contour in source.contours:
        let mut contour = contour_value :: :: call
        for source_point in source_contour.points:
            let transformed = transform_point :: (source_point.x, source_point.y), matrix, offset :: call
            let point = point_value :: transformed.0, transformed.1, source_point.on_curve :: call
            contour.points :: point :: push
        out.contours :: contour :: push
    out.empty = false

fn parse_compound_outline(read face: arcana_text.font_leaf.FontFaceState, glyph: (Int, Int), depth: Int) -> arcana_text.font_leaf.GlyphOutline:
    let glyph_index = glyph.0
    let glyph_offset = glyph.1
    let _ = depth
    let start = face.glyf_offset + glyph_offset
    let advance_width = advance_width_for :: face, glyph_index :: call
    let left_side_bearing = left_side_bearing_for :: face, glyph_index :: call
    let mut outline = outline_value :: advance_width, left_side_bearing :: call
    outline.x_min = i16_be_ref :: face.font_view, start + 2 :: call
    outline.y_min = i16_be_ref :: face.font_view, start + 4 :: call
    outline.x_max = i16_be_ref :: face.font_view, start + 6 :: call
    outline.y_max = i16_be_ref :: face.font_view, start + 8 :: call
    let mut cursor = start + 10
    let mut more = true
    while more:
        let flags = u16_be_ref :: face.font_view, cursor :: call
        let component_index = u16_be_ref :: face.font_view, cursor + 2 :: call
        cursor += 4
        let arg_words = (flags % 2) == 1
        let args_are_xy = ((flags / 2) % 2) == 1
        let mut arg1 = byte_at_or_zero_ref :: face.font_view, cursor :: call
        let mut arg2 = byte_at_or_zero_ref :: face.font_view, cursor + 1 :: call
        let mut arg_len = 2
        if arg_words:
            arg1 = i16_be_ref :: face.font_view, cursor :: call
            arg2 = i16_be_ref :: face.font_view, cursor + 2 :: call
            arg_len = 4
        cursor += arg_len
        let mut matrix = affine_matrix :: 16384, 0, (0, 16384) :: call
        if ((flags / 128) % 2) == 1:
            matrix = affine_matrix :: (i16_be_ref :: face.font_view, cursor :: call), (i16_be_ref :: face.font_view, cursor + 2 :: call), ((i16_be_ref :: face.font_view, cursor + 4 :: call), (i16_be_ref :: face.font_view, cursor + 6 :: call)) :: call
            cursor += 8
        else:
            if ((flags / 8) % 2) == 1:
                let scale = i16_be_ref :: face.font_view, cursor :: call
                matrix = affine_matrix :: scale, 0, (0, scale) :: call
                cursor += 2
            else:
                if ((flags / 64) % 2) == 1:
                    matrix = affine_matrix :: (i16_be_ref :: face.font_view, cursor :: call), 0, (0, (i16_be_ref :: face.font_view, cursor + 2 :: call)) :: call
                    cursor += 4
        let mut offset = (0, 0)
        if args_are_xy:
            offset = (arg1, arg2)
        outline.components :: (glyph_component :: component_index, matrix, offset :: call) :: push
        more = ((flags / 32) % 2) == 1
    if not (outline.components :: :: is_empty):
        outline.empty = false
    return outline

fn round_div(value: Int, denom: Int) -> Int:
    if denom == 0:
        return 0
    if value >= 0:
        return (value + (denom / 2)) / denom
    return 0 - (((0 - value) + (denom / 2)) / denom)

fn coord_from_point(read value: arcana_text.font_leaf.FontPoint) -> arcana_text.font_leaf.CoordPoint:
    return coord_point :: value.x, value.y :: call

fn point_with_coord(read point: arcana_text.font_leaf.FontPoint, read coord: arcana_text.font_leaf.CoordPoint) -> arcana_text.font_leaf.FontPoint:
    return point_value :: coord.x, coord.y, point.on_curve :: call

fn copy_coord_points(read values: List[arcana_text.font_leaf.CoordPoint]) -> List[arcana_text.font_leaf.CoordPoint]:
    let mut out = empty_coord_points :: :: call
    out :: values :: extend_list
    return out

fn copy_ints(read values: List[Int]) -> List[Int]:
    let mut out = empty_int_list :: :: call
    out :: values :: extend_list
    return out

fn copy_coord_range(read values: List[arcana_text.font_leaf.CoordPoint], start: Int, end: Int) -> List[arcana_text.font_leaf.CoordPoint]:
    let mut out = empty_coord_points :: :: call
    let mut index = 0
    for value in values:
        if index >= start and index < end:
            out :: value :: push
        index += 1
    return out

fn copy_delta_range(read values: List[arcana_text.font_leaf.PointDelta], start: Int, end: Int) -> List[arcana_text.font_leaf.PointDelta]:
    let mut out = empty_point_deltas :: :: call
    let mut index = 0
    for value in values:
        if index >= start and index < end:
            out :: value :: push
        index += 1
    return out

fn outline_points(read outline: arcana_text.font_leaf.GlyphOutline) -> List[arcana_text.font_leaf.FontPoint]:
    let mut out = empty_points :: :: call
    for contour in outline.contours:
        out :: contour.points :: extend_list
    return out

fn outline_end_points(read outline: arcana_text.font_leaf.GlyphOutline) -> List[Int]:
    let mut out = empty_int_list :: :: call
    let mut total = 0
    for contour in outline.contours:
        let count = contour.points :: :: len
        if count > 0:
            total += count
            out :: (total - 1) :: push
    return out

fn outline_coords(read outline: arcana_text.font_leaf.GlyphOutline) -> List[arcana_text.font_leaf.CoordPoint]:
    let mut out = empty_coord_points :: :: call
    for point in (outline_points :: outline :: call):
        out :: (coord_from_point :: point :: call) :: push
    return out

fn component_coords(read outline: arcana_text.font_leaf.GlyphOutline) -> List[arcana_text.font_leaf.CoordPoint]:
    let mut out = empty_coord_points :: :: call
    for component in outline.components:
        out :: (coord_point :: component.offset.0, component.offset.1 :: call) :: push
    return out

fn component_end_points(read outline: arcana_text.font_leaf.GlyphOutline) -> List[Int]:
    let mut out = empty_int_list :: :: call
    let mut index = 0
    let total = outline.components :: :: len
    while index < total:
        out :: index :: push
        index += 1
    return out

fn outline_coords_with_phantoms(read outline: arcana_text.font_leaf.GlyphOutline) -> List[arcana_text.font_leaf.CoordPoint]:
    let mut out = outline_coords :: outline :: call
    if not (outline.components :: :: is_empty):
        out = component_coords :: outline :: call
    let left_side_x = outline.x_min - outline.left_side_bearing
    let right_side_x = left_side_x + outline.advance_width
    out :: (coord_point :: left_side_x, 0 :: call) :: push
    out :: (coord_point :: right_side_x, 0 :: call) :: push
    out :: (coord_point :: 0, 0 :: call) :: push
    out :: (coord_point :: 0, 0 :: call) :: push
    return out

fn outline_end_points_for_variation(read outline: arcana_text.font_leaf.GlyphOutline) -> List[Int]:
    if not (outline.components :: :: is_empty):
        return component_end_points :: outline :: call
    return outline_end_points :: outline :: call

fn explicit_delta_indices(read values: List[arcana_text.font_leaf.PointDelta]) -> List[Int]:
    let mut out = empty_int_list :: :: call
    let mut index = 0
    for value in values:
        if value.explicit:
            out :: index :: push
        index += 1
    return out

fn interpolate_axis_delta(value: Int, read left: (Int, Int), read right: (Int, Int)) -> Int:
    let mut x1 = left.0
    let mut x2 = right.0
    let mut d1 = left.1
    let mut d2 = right.1
    if x1 == x2:
        if d1 == d2:
            return d1
        return 0
    if x1 > x2:
        let swap_x = x1
        let swap_d = d1
        x1 = x2
        x2 = swap_x
        d1 = d2
        d2 = swap_d
    if value <= x1:
        return d1
    if value >= x2:
        return d2
    return d1 + (round_div :: ((value - x1) * (d2 - d1)), (x2 - x1) :: call)

fn interpolate_segment(read request: arcana_text.font_leaf.InterpolateSegmentRequest) -> List[arcana_text.font_leaf.PointDelta]:
    let mut out = empty_point_deltas :: :: call
    for coord in request.coords:
        let dx = interpolate_axis_delta :: coord.x, (request.left_coord.x, request.left_delta.x), (request.right_coord.x, request.right_delta.x) :: call
        let dy = interpolate_axis_delta :: coord.y, (request.left_coord.y, request.left_delta.y), (request.right_coord.y, request.right_delta.y) :: call
        out :: (point_delta :: dx, dy, false :: call) :: push
    return out

fn iup_contour(read deltas: List[arcana_text.font_leaf.PointDelta], read coords: List[arcana_text.font_leaf.CoordPoint]) -> List[arcana_text.font_leaf.PointDelta]:
    let total = deltas :: :: len
    let indices = explicit_delta_indices :: deltas :: call
    if (indices :: :: len) == total:
        let mut out = empty_point_deltas :: :: call
        out :: deltas :: extend_list
        return out
    if indices :: :: is_empty:
        let mut zeroes = empty_point_deltas :: :: call
        let mut index = 0
        while index < total:
            zeroes :: (delta_zero :: :: call) :: push
            index += 1
        return zeroes
    let first = int_list_at_or_zero :: indices, 0 :: call
    let last = int_list_at_or_zero :: indices, (indices :: :: len) - 1 :: call
    let mut out = empty_point_deltas :: :: call
    let mut start = first
    if first != 0:
        let mut request = arcana_text.font_leaf.InterpolateSegmentRequest :: coords = (copy_coord_range :: coords, 0, first :: call), left_coord = (coord_list_at_or_zero :: coords, first :: call) :: call
        request.left_delta = delta_list_at_or_zero :: deltas, first :: call
        request.right_coord = coord_list_at_or_zero :: coords, last :: call
        request.right_delta = delta_list_at_or_zero :: deltas, last :: call
        out :: (interpolate_segment :: request :: call) :: extend_list
    out :: (delta_list_at_or_zero :: deltas, first :: call) :: push
    let mut step = 1
    while step < (indices :: :: len):
        let finish = int_list_at_or_zero :: indices, step :: call
        if finish - start > 1:
            let mut request = arcana_text.font_leaf.InterpolateSegmentRequest :: coords = (copy_coord_range :: coords, start + 1, finish :: call), left_coord = (coord_list_at_or_zero :: coords, start :: call) :: call
            request.left_delta = delta_list_at_or_zero :: deltas, start :: call
            request.right_coord = coord_list_at_or_zero :: coords, finish :: call
            request.right_delta = delta_list_at_or_zero :: deltas, finish :: call
            out :: (interpolate_segment :: request :: call) :: extend_list
        out :: (delta_list_at_or_zero :: deltas, finish :: call) :: push
        start = finish
        step += 1
    if start != total - 1:
        let mut request = arcana_text.font_leaf.InterpolateSegmentRequest :: coords = (copy_coord_range :: coords, start + 1, total :: call), left_coord = (coord_list_at_or_zero :: coords, start :: call) :: call
        request.left_delta = delta_list_at_or_zero :: deltas, start :: call
        request.right_coord = coord_list_at_or_zero :: coords, first :: call
        request.right_delta = delta_list_at_or_zero :: deltas, first :: call
        out :: (interpolate_segment :: request :: call) :: extend_list
    return out

fn iup_delta(read deltas: List[arcana_text.font_leaf.PointDelta], read coords: List[arcana_text.font_leaf.CoordPoint], read end_points: List[Int]) -> List[arcana_text.font_leaf.PointDelta]:
    let mut ends = copy_ints :: end_points :: call
    let total = coords :: :: len
    if total >= 4:
        ends :: (total - 4) :: push
        ends :: (total - 3) :: push
        ends :: (total - 2) :: push
        ends :: (total - 1) :: push
    let mut out = empty_point_deltas :: :: call
    let mut start = 0
    for end in ends:
        out :: (iup_contour :: (copy_delta_range :: deltas, start, end + 1 :: call), (copy_coord_range :: coords, start, end + 1 :: call) :: call) :: extend_list
        start = end + 1
    return out

fn build_explicit_deltas(point_count: Int, read points: List[Int], read deltas: (List[Int], List[Int])) -> List[arcana_text.font_leaf.PointDelta]:
    let mut out = empty_point_deltas :: :: call
    let mut point_index = 0
    let mut explicit_index = 0
    let total_explicit = points :: :: len
    while point_index < point_count:
        if explicit_index < total_explicit and (int_list_at_or_zero :: points, explicit_index :: call) == point_index:
            let dx = int_list_at_or_zero :: deltas.0, explicit_index :: call
            let dy = int_list_at_or_zero :: deltas.1, explicit_index :: call
            out :: (point_delta :: dx, dy, true :: call) :: push
            explicit_index += 1
        else:
            out :: (delta_zero :: :: call) :: push
        point_index += 1
    return out

fn apply_scaled_deltas(read coords: List[arcana_text.font_leaf.CoordPoint], read deltas: List[arcana_text.font_leaf.PointDelta], scalar: Int) -> List[arcana_text.font_leaf.CoordPoint]:
    let mut out = empty_coord_points :: :: call
    let total = coords :: :: len
    let mut index = 0
    while index < total:
        let coord = coord_list_at_or_zero :: coords, index :: call
        let delta = delta_list_at_or_zero :: deltas, index :: call
        out :: (coord_point :: (coord.x + (round_div :: (delta.x * scalar), 16384 :: call)), (coord.y + (round_div :: (delta.y * scalar), 16384 :: call)) :: call) :: push
        index += 1
    return out

fn tuple_coord_values(read bytes: std.memory.ByteView, offset: Int, count: Int) -> (List[Int], Int):
    let mut out = empty_int_list :: :: call
    let mut pos = offset
    let mut index = 0
    while index < count:
        out :: (i16_be_ref :: bytes, pos :: call) :: push
        pos += 2
        index += 1
    return (out, pos)

fn default_region_start(read peak: List[Int]) -> List[Int]:
    let mut out = empty_int_list :: :: call
    for value in peak:
        if value < 0:
            out :: value :: push
        else:
            out :: 0 :: push
    return out

fn default_region_end(read peak: List[Int]) -> List[Int]:
    let mut out = empty_int_list :: :: call
    for value in peak:
        if value > 0:
            out :: value :: push
        else:
            out :: 0 :: push
    return out

fn tuple_scalar(read location: List[Int], read region: arcana_text.font_leaf.TupleScalarRegion) -> Int:
    let start = region.start
    let peak = region.peak
    let end = region.end
    let mut scalar = 16384
    let total = peak :: :: len
    let mut index = 0
    while index < total:
        let low = int_list_at_or_zero :: start, index :: call
        let mid = int_list_at_or_zero :: peak, index :: call
        let high = int_list_at_or_zero :: end, index :: call
        if mid == 0:
            index += 1
            continue
        if low > mid or mid > high or (low < 0 and high > 0):
            index += 1
            continue
        let value = int_list_at_or_zero :: location, index :: call
        if value == mid:
            index += 1
            continue
        if value <= low or high <= value:
            return 0
        let mut factor = round_div :: ((value - high) * 16384), (mid - high) :: call
        if value < mid:
            factor = round_div :: ((value - low) * 16384), (mid - low) :: call
        scalar = round_div :: (scalar * factor), 16384 :: call
        if scalar == 0:
            return 0
        index += 1
    return scalar

fn gvar_offset_value(read face: arcana_text.font_leaf.FontFaceState, entry_index: Int) -> Int:
    let offset_array = face.gvar_offset + 20
    if (face.gvar_flags % 2) == 1:
        return u32_be_ref :: face.font_view, offset_array + (entry_index * 4) :: call
    return (u16_be_ref :: face.font_view, offset_array + (entry_index * 2) :: call) * 2

fn glyph_variation_data_range(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> (Int, Int):
    if face.gvar_offset < 0 or face.gvar_length <= 0 or face.gvar_axis_count <= 0:
        return (0, 0)
    if glyph_index < 0 or glyph_index >= face.gvar_glyph_count:
        return (0, 0)
    let table_end = face.gvar_offset + face.gvar_length
    let start = face.gvar_data_offset + (gvar_offset_value :: face, glyph_index :: call)
    let end = face.gvar_data_offset + (gvar_offset_value :: face, glyph_index + 1 :: call)
    if start < face.gvar_data_offset or end > table_end or end <= start:
        return (0, 0)
    return (start, end)

fn apply_gvar_to_coords(read face: arcana_text.font_leaf.FontFaceState, read request: arcana_text.font_leaf.ApplyGvarRequest) -> List[arcana_text.font_leaf.CoordPoint]:
    font_leaf_probe_append :: ("apply_gvar:start glyph=" + (std.text.from_int :: request.glyph_index :: call) + " points=" + (std.text.from_int :: (request.coords :: :: len) :: call)) :: call
    if face.gvar_offset < 0 or face.gvar_length <= 0 or face.gvar_axis_count != (face.variation_axes :: :: len):
        font_leaf_probe_append :: "apply_gvar:no_gvar" :: call
        return copy_coord_points :: request.coords :: call
    let location = normalized_location :: face, request.traits :: call
    if location_is_default :: location :: call:
        font_leaf_probe_append :: "apply_gvar:default_location" :: call
        return copy_coord_points :: request.coords :: call
    let range = glyph_variation_data_range :: face, request.glyph_index :: call
    if range.1 <= range.0:
        font_leaf_probe_append :: "apply_gvar:no_range" :: call
        return copy_coord_points :: request.coords :: call
    let tuple_header = u16_be_ref :: face.font_view, range.0 :: call
    let tuple_count = tuple_header % 4096
    font_leaf_probe_append :: ("apply_gvar:tuples=" + (std.text.from_int :: tuple_count :: call)) :: call
    let data_offset = u16_be_ref :: face.font_view, range.0 + 2 :: call
    let mut tuple_pos = range.0 + 4
    let mut data_pos = range.0 + data_offset
    let point_count = request.coords :: :: len
    let mut shared_points = empty_int_list :: :: call
    if tuple_header >= 32768:
        let decoded = decode_packed_points :: point_count, face.font_view, data_pos :: call
        shared_points = decoded.0
        data_pos = decoded.1
    let mut out = copy_coord_points :: request.coords :: call
    let mut tuple_index = 0
    while tuple_index < tuple_count and tuple_pos + 4 <= range.1:
        let data_size = u16_be_ref :: face.font_view, tuple_pos :: call
        let flags = u16_be_ref :: face.font_view, tuple_pos + 2 :: call
        let mut header_pos = tuple_pos + 4
        let mut peak_coords = empty_int_list :: :: call
        if ((flags / 32768) % 2) == 1:
            let peak_value = tuple_coord_values :: face.font_view, header_pos, face.gvar_axis_count :: call
            peak_coords = peak_value.0
            header_pos = peak_value.1
        else:
            peak_coords = (tuple_coord_values :: face.font_view, face.gvar_shared_tuples_offset + ((flags % 4096) * face.gvar_axis_count * 2), face.gvar_axis_count :: call).0
        let mut start_coords = default_region_start :: peak_coords :: call
        let mut end_coords = default_region_end :: peak_coords :: call
        if ((flags / 16384) % 2) == 1:
            let start_value = tuple_coord_values :: face.font_view, header_pos, face.gvar_axis_count :: call
            start_coords = start_value.0
            let end_value = tuple_coord_values :: face.font_view, start_value.1, face.gvar_axis_count :: call
            end_coords = end_value.0
            header_pos = end_value.1
        let region = arcana_text.font_leaf.TupleScalarRegion :: start = start_coords, peak = peak_coords, end = end_coords :: call
        let scalar = tuple_scalar :: location, region :: call
        let next_data = data_pos + data_size
        if scalar != 0 and next_data <= range.1:
            let mut tuple_data_pos = data_pos
            let mut point_ids = copy_ints :: shared_points :: call
            if ((flags / 8192) % 2) == 1:
                let points = decode_packed_points :: point_count, face.font_view, tuple_data_pos :: call
                point_ids = points.0
                tuple_data_pos = points.1
            let delta_xs = decode_packed_deltas :: (point_ids :: :: len), face.font_view, tuple_data_pos :: call
            tuple_data_pos = delta_xs.1
            let delta_ys = decode_packed_deltas :: (point_ids :: :: len), face.font_view, tuple_data_pos :: call
            let mut deltas = build_explicit_deltas :: point_count, point_ids, (delta_xs.0, delta_ys.0) :: call
            if (point_ids :: :: len) < point_count:
                deltas = iup_delta :: deltas, out, request.end_points :: call
            out = apply_scaled_deltas :: out, deltas, scalar :: call
        tuple_pos = header_pos
        data_pos = next_data
        tuple_index += 1
    font_leaf_probe_append :: "apply_gvar:done" :: call
    return out

fn recalc_outline_bounds(edit outline: arcana_text.font_leaf.GlyphOutline):
    if outline.contours :: :: is_empty:
        outline.x_min = 0
        outline.y_min = 0
        outline.x_max = 0
        outline.y_max = 0
        outline.empty = true
        return
    let first_contour = point_list_at_or_zero :: ((outline.contours)[0].points), 0 :: call
    let mut x_min = first_contour.x
    let mut y_min = first_contour.y
    let mut x_max = first_contour.x
    let mut y_max = first_contour.y
    for contour in outline.contours:
        for point in contour.points:
            if point.x < x_min:
                x_min = point.x
            if point.y < y_min:
                y_min = point.y
            if point.x > x_max:
                x_max = point.x
            if point.y > y_max:
                y_max = point.y
    outline.x_min = x_min
    outline.y_min = y_min
    outline.x_max = x_max
    outline.y_max = y_max
    outline.empty = false

fn apply_phantom_metrics(edit outline: arcana_text.font_leaf.GlyphOutline, read coords: List[arcana_text.font_leaf.CoordPoint]):
    let total = coords :: :: len
    if total < 4:
        return
    let left = coord_list_at_or_zero :: coords, total - 4 :: call
    let right = coord_list_at_or_zero :: coords, total - 3 :: call
    outline.advance_width = right.x - left.x
    outline.left_side_bearing = outline.x_min - left.x

fn rebuild_simple_outline(read base: arcana_text.font_leaf.GlyphOutline, read coords: List[arcana_text.font_leaf.CoordPoint]) -> arcana_text.font_leaf.GlyphOutline:
    let base_points = outline_points :: base :: call
    let total_points = base_points :: :: len
    let mut out = outline_value :: base.advance_width, base.left_side_bearing :: call
    let mut point_index = 0
    for contour in base.contours:
        let mut next = contour_value :: :: call
        for point in contour.points:
            let coord = coord_list_at_or_zero :: coords, point_index :: call
            let next_point = point_with_coord :: point, coord :: call
            next.points :: next_point :: push
            point_index += 1
        out.contours :: next :: push
    if total_points > 0:
        recalc_outline_bounds :: out :: call
    apply_phantom_metrics :: out, coords :: call
    return out

fn resolve_compound_outline(edit face: arcana_text.font_leaf.FontFaceState, read base: arcana_text.font_leaf.GlyphOutline, read payload: arcana_text.font_leaf.ResolveCompoundRequest) -> arcana_text.font_leaf.GlyphOutline:
    let coords = payload.coords
    let traits = payload.traits
    let depth = payload.depth
    let mut out = outline_value :: base.advance_width, base.left_side_bearing :: call
    let mut index = 0
    for component in base.components:
        let coord = coord_list_at_or_zero :: coords, index :: call
        let resolved = load_outline_recursive :: face, (component.glyph_index, depth + 1), traits :: call
        append_outline_contours :: out, (resolved, component.matrix, (coord.x, coord.y)) :: call
        index += 1
    recalc_outline_bounds :: out :: call
    apply_phantom_metrics :: out, coords :: call
    return out

fn load_outline_recursive(edit face: arcana_text.font_leaf.FontFaceState, read payload: (Int, Int), read traits: arcana_text.font_leaf.FaceTraits) -> arcana_text.font_leaf.GlyphOutline:
    let glyph_index = payload.0
    let depth = payload.1
    font_leaf_probe_append :: ("load_outline:start glyph=" + (std.text.from_int :: glyph_index :: call) + " depth=" + (std.text.from_int :: depth :: call)) :: call
    let offsets = glyph_offset_for :: face, glyph_index :: call
    let advance_width = advance_width_for :: face, glyph_index :: call
    let left_bearing = left_side_bearing_for :: face, glyph_index :: call
    let mut base = outline_value :: advance_width, left_bearing :: call
    if offsets.0 == offsets.1:
        let mut request = arcana_text.font_leaf.ApplyGvarRequest :: glyph_index = glyph_index, coords = (outline_coords_with_phantoms :: base :: call), end_points = (outline_end_points_for_variation :: base :: call) :: call
        request.traits = traits
        let coords = apply_gvar_to_coords :: face, request :: call
        apply_phantom_metrics :: base, coords :: call
        font_leaf_probe_append :: "load_outline:empty_done" :: call
        return base
    let start = face.glyf_offset + offsets.0
    let contour_count = i16_be_ref :: face.font_view, start :: call
    font_leaf_probe_append :: ("load_outline:contours=" + (std.text.from_int :: contour_count :: call)) :: call
    if contour_count >= 0:
        font_leaf_probe_append :: "load_outline:parse_simple" :: call
        base = parse_simple_outline :: face, glyph_index, offsets.0 :: call
    else:
        font_leaf_probe_append :: "load_outline:parse_compound" :: call
        base = parse_compound_outline :: face, (glyph_index, offsets.0), depth :: call
    font_leaf_probe_append :: "load_outline:parsed" :: call
    let mut request = arcana_text.font_leaf.ApplyGvarRequest :: glyph_index = glyph_index, coords = (outline_coords_with_phantoms :: base :: call), end_points = (outline_end_points_for_variation :: base :: call) :: call
    request.traits = traits
    font_leaf_probe_append :: "load_outline:apply_gvar" :: call
    let coords = apply_gvar_to_coords :: face, request :: call
    if contour_count >= 0:
        font_leaf_probe_append :: "load_outline:simple_done" :: call
        return rebuild_simple_outline :: base, coords :: call
    if depth >= 8:
        font_leaf_probe_append :: "load_outline:compound_depth_limit" :: call
        return base
    let request = arcana_text.font_leaf.ResolveCompoundRequest :: coords = coords, traits = traits, depth = depth :: call
    font_leaf_probe_append :: "load_outline:compound_resolve" :: call
    return resolve_compound_outline :: face, base, request :: call

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

fn effective_width_milli(read face: arcana_text.font_leaf.FontFaceState, read traits: arcana_text.font_leaf.FaceTraits) -> Int:
    if face_has_variation_axis :: face, "wdth" :: call:
        return 100000
    return normalized_width_milli :: traits.width_milli :: call

fn effective_slant_milli(read face: arcana_text.font_leaf.FontFaceState, read traits: arcana_text.font_leaf.FaceTraits) -> Int:
    if face_has_variation_axis :: face, "slnt" :: call:
        return 0
    return traits.slant_milli

fn actual_glyph_index(edit face: arcana_text.font_leaf.FontFaceState, glyph_index: Int, read text: Str) -> Int:
    if glyph_index > 0:
        return glyph_index
    if text == "" or text == "\n" or text == "\r":
        return glyph_index
    return glyph_index_for_codepoint :: face, (utf8_codepoint :: text :: call) :: call

fn scaled_advance(read face: arcana_text.font_leaf.FontFaceState, read traits: arcana_text.font_leaf.FaceTraits, payload: (Int, Int)) -> Int:
    return max_int :: (scale_x :: payload.1, payload.0, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call), 1 :: call

fn scaled_vertical_advance(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, raw_advance: Int) -> Int:
    return max_int :: (scale_y :: raw_advance, font_size, face.units_per_em :: call), 1 :: call

fn scale_x(value: Int, font_size: Int, dims: (Int, Int)) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: dims.0, 1 :: call
    let safe_width = normalized_width_milli :: dims.1 :: call
    return round_div :: (value * safe_font_size * safe_width), (safe_units_per_em * 100000) :: call

fn scale_y(value: Int, font_size: Int, units_per_em: Int) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: units_per_em, 1 :: call
    return round_div :: (value * safe_font_size), safe_units_per_em :: call

fn snap_near(value: Int, target: Int, tolerance: Int) -> Int:
    if tolerance < 0:
        return value
    if (abs_int :: (value - target) :: call) <= tolerance:
        return target
    return value

fn scaled_outline_width(read payload: (arcana_text.font_leaf.FontFaceState, arcana_text.font_leaf.GlyphOutline, arcana_text.font_leaf.FaceTraits, Int)) -> Int:
    let face = payload.0
    let outline = payload.1
    let traits = payload.2
    let font_size = payload.3
    let scaled_min = scale_x :: outline.x_min, font_size, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call
    let scaled_max = scale_x :: outline.x_max, font_size, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call
    return max_int :: (scaled_max - scaled_min + 1), 1 :: call

fn scaled_outline_height(read face: arcana_text.font_leaf.FontFaceState, read outline: arcana_text.font_leaf.GlyphOutline, font_size: Int) -> Int:
    let scaled_min = scale_y :: outline.y_min, font_size, face.units_per_em :: call
    let scaled_max = scale_y :: outline.y_max, font_size, face.units_per_em :: call
    return max_int :: (scaled_max - scaled_min + 1), 1 :: call

fn hinted_outline_advance(read payload: (arcana_text.font_leaf.FontFaceState, arcana_text.font_leaf.GlyphOutline, arcana_text.font_leaf.FaceTraits, Int, arcana_text.types.Hinting)) -> Int:
    let face = payload.0
    let outline = payload.1
    let traits = payload.2
    let font_size = payload.3
    let hinting = payload.4
    let scaled = max_int :: (scale_x :: outline.advance_width, font_size, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call), 1 :: call
    if hinting != (arcana_text.types.Hinting.Enabled :: :: call):
        return scaled
    let bearing = scale_x :: outline.left_side_bearing, font_size, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call
    let outline_width = arcana_text.font_leaf.scaled_outline_width :: (face, outline, traits, font_size) :: call
    let fitted = max_int :: scaled, outline_width :: call
    return max_int :: fitted, (bearing + outline_width) :: call

fn hinted_horizontal_metrics(read payload: (arcana_text.font_leaf.FontFaceState, arcana_text.font_leaf.GlyphOutline, arcana_text.font_leaf.FaceTraits, Int, Int, Int, Int, arcana_text.types.Hinting)) -> (Int, Int, Int):
    let face = payload.0
    let outline = payload.1
    let traits = payload.2
    let font_size = payload.3
    let left = payload.4
    let width = payload.5
    let advance = payload.6
    let hinting = payload.7
    if hinting != (arcana_text.types.Hinting.Enabled :: :: call):
        return (left, width, advance)
    let bearing = scale_x :: outline.left_side_bearing, font_size, (face.units_per_em, (effective_width_milli :: face, traits :: call)) :: call
    let snapped_left = arcana_text.font_leaf.snap_near :: left, bearing, 2 :: call
    let outline_width = arcana_text.font_leaf.scaled_outline_width :: (face, outline, traits, font_size) :: call
    let snapped_advance = arcana_text.font_leaf.hinted_outline_advance :: (face, outline, traits, font_size, hinting) :: call
    let snapped_right = max_int :: (snapped_left + outline_width), snapped_advance :: call
    let snapped_width = max_int :: width, (max_int :: (snapped_right - snapped_left), 1 :: call) :: call
    let fitted_advance = max_int :: snapped_advance, (snapped_left + snapped_width) :: call
    return (snapped_left, snapped_width, fitted_advance)

fn hinted_vertical_top(read payload: (arcana_text.font_leaf.FontFaceState, Int, Int, Int, arcana_text.types.Hinting)) -> Int:
    let face = payload.0
    let glyph_index = payload.1
    let font_size = payload.2
    let top = payload.3
    let hinting = payload.4
    if hinting != (arcana_text.types.Hinting.Enabled :: :: call):
        return top
    let target = scale_y :: (top_side_bearing_for :: face, glyph_index :: call), font_size, face.units_per_em :: call
    return arcana_text.font_leaf.snap_near :: top, target, 2 :: call

fn hinted_vertical_advance(read payload: (arcana_text.font_leaf.FontFaceState, arcana_text.font_leaf.GlyphOutline, Int, Int, arcana_text.types.Hinting)) -> Int:
    let face = payload.0
    let outline = payload.1
    let glyph_index = payload.2
    let font_size = payload.3
    let hinting = payload.4
    let scaled = scaled_vertical_advance :: face, font_size, (vertical_advance_for :: face, glyph_index :: call) :: call
    if hinting != (arcana_text.types.Hinting.Enabled :: :: call):
        return scaled
    let outline_height = arcana_text.font_leaf.scaled_outline_height :: face, outline, font_size :: call
    return max_int :: scaled, outline_height :: call

fn hinted_vertical_metrics(read payload: (arcana_text.font_leaf.FontFaceState, arcana_text.font_leaf.GlyphOutline, Int, Int, Int, Int, Int, arcana_text.types.Hinting)) -> (Int, Int, Int):
    let face = payload.0
    let outline = payload.1
    let glyph_index = payload.2
    let font_size = payload.3
    let top = payload.4
    let height = payload.5
    let advance = payload.6
    let hinting = payload.7
    if hinting != (arcana_text.types.Hinting.Enabled :: :: call):
        return (top, height, advance)
    let snapped_top = arcana_text.font_leaf.hinted_vertical_top :: (face, glyph_index, font_size, top, hinting) :: call
    let outline_height = arcana_text.font_leaf.scaled_outline_height :: face, outline, font_size :: call
    let snapped_height = max_int :: height, outline_height :: call
    let snapped_advance = arcana_text.font_leaf.hinted_vertical_advance :: (face, outline, glyph_index, font_size, hinting) :: call
    return (snapped_top, snapped_height, (max_int :: snapped_advance, snapped_height :: call))

fn hinted_outline_advance_for(edit face: arcana_text.font_leaf.FontFaceState, read payload: (Int, arcana_text.font_leaf.FaceTraits, Int, Bool, arcana_text.types.Hinting)) -> Int:
    let glyph_index = payload.0
    let traits = payload.1
    let font_size = payload.2
    let vertical = payload.3
    let hinting = payload.4
    if hinting != (arcana_text.types.Hinting.Enabled :: :: call) or glyph_index <= 0:
        return -1
    let outline = load_outline_recursive :: face, (glyph_index, 0), traits :: call
    if outline.empty:
        return -1
    return match vertical:
        true => arcana_text.font_leaf.hinted_vertical_advance :: (face, outline, glyph_index, font_size, hinting) :: call
        false => arcana_text.font_leaf.hinted_outline_advance :: (face, outline, traits, font_size, hinting) :: call

fn scaled_point(read scale: arcana_text.font_leaf.ScaleContext, point: (Int, Int)) -> (Int, Int):
    let mut x = scale_x :: point.0, scale.font_size, (scale.units_per_em, scale.width_milli) :: call
    let y = scale_y :: point.1, scale.font_size, scale.units_per_em :: call
    x += (scale.slant_milli * y) / 1000
    return (x, y)

fn midpoint(a: (Int, Int), b: (Int, Int)) -> (Int, Int):
    return ((a.0 + b.0) / 2, (a.1 + b.1) / 2)

fn append_quad_segments(edit out: List[arcana_text.font_leaf.LineSegment], p0: (Int, Int), read curve: ((Int, Int), (Int, Int))):
    let p1 = curve.0
    let p2 = curve.1
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
    font_leaf_probe_append :: ("contour_segments:start count=" + (std.text.from_int :: count :: call)) :: call
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
    let mut guard = 0
    while index < count:
        guard += 1
        if guard > ((count * 4) + 4):
            font_leaf_probe_append :: ("contour_segments:guard count=" + (std.text.from_int :: count :: call) + " index=" + (std.text.from_int :: index :: call)) :: call
            break
        font_leaf_probe_append :: ("contour_segments:step index=" + (std.text.from_int :: index :: call)) :: call
        let raw = point_list_at_or_zero :: contour.points, index :: call
        font_leaf_probe_append :: ("contour_segments:raw index=" + (std.text.from_int :: index :: call) + " point=" + (std.text.from_int :: raw.x :: call) + "," + (std.text.from_int :: raw.y :: call)) :: call
        let scaled = scaled_point :: scale, (raw.x, raw.y) :: call
        font_leaf_probe_append :: ("contour_segments:scaled index=" + (std.text.from_int :: index :: call) + " point=" + (std.text.from_int :: scaled.0 :: call) + "," + (std.text.from_int :: scaled.1 :: call)) :: call
        if raw.on_curve:
            font_leaf_probe_append :: ("contour_segments:on_curve index=" + (std.text.from_int :: index :: call)) :: call
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
                font_leaf_probe_append :: ("contour_segments:off_curve_next_on index=" + (std.text.from_int :: index :: call) + " next=" + (std.text.from_int :: next_index :: call)) :: call
                append_quad_segments :: out, current, (scaled, next_scaled) :: call
                current = next_scaled
                index += 2
            else:
                let mid = midpoint :: scaled, next_scaled :: call
                font_leaf_probe_append :: ("contour_segments:off_curve_mid index=" + (std.text.from_int :: index :: call) + " next=" + (std.text.from_int :: next_index :: call)) :: call
                append_quad_segments :: out, current, (scaled, mid) :: call
                current = mid
                index += 1
    if current.0 != start.0 or current.1 != start.1:
        out :: (line_segment :: current, start :: call) :: push
    font_leaf_probe_append :: ("contour_segments:count=" + (std.text.from_int :: count :: call) + " segments=" + (std.text.from_int :: (out :: :: len) :: call)) :: call
    return out

fn outline_segments(read outline: arcana_text.font_leaf.GlyphOutline, read scale: arcana_text.font_leaf.ScaleContext) -> List[arcana_text.font_leaf.LineSegment]:
    let mut out = empty_segments :: :: call
    let mut contour_index = 0
    for contour in outline.contours:
        font_leaf_probe_append :: ("outline_segments:contour_start index=" + (std.text.from_int :: contour_index :: call)) :: call
        let contour_out = contour_segments :: contour, scale :: call
        font_leaf_probe_append :: ("outline_segments:contour_ready index=" + (std.text.from_int :: contour_index :: call) + " segments=" + (std.text.from_int :: (contour_out :: :: len) :: call)) :: call
        out :: contour_out :: extend_list
        font_leaf_probe_append :: ("outline_segments:contour_merged index=" + (std.text.from_int :: contour_index :: call) + " total=" + (std.text.from_int :: (out :: :: len) :: call)) :: call
        contour_index += 1
    font_leaf_probe_append :: ("outline_segments:contours=" + (std.text.from_int :: (outline.contours :: :: len) :: call) + " segments=" + (std.text.from_int :: (out :: :: len) :: call)) :: call
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

fn fill_bitmap_binary_from_segments(read segments: List[arcana_text.font_leaf.LineSegment], width: Int, height: Int) -> Array[Int]:
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

fn scaled_segment(read segment: arcana_text.font_leaf.LineSegment, factor: Int) -> arcana_text.font_leaf.LineSegment:
    return line_segment :: (segment.start.0 * factor, segment.start.1 * factor), (segment.end.0 * factor, segment.end.1 * factor) :: call

fn scale_segments_for_raster(read segments: List[arcana_text.font_leaf.LineSegment], factor: Int) -> List[arcana_text.font_leaf.LineSegment]:
    let mut out = empty_segments :: :: call
    if factor <= 1:
        out :: segments :: extend_list
        return out
    for segment in segments:
        out :: (arcana_text.font_leaf.scaled_segment :: segment, factor :: call) :: push
    return out

fn downsample_alpha(read payload: (Array[Int], (Int, Int), (Int, Int), Int)) -> Array[Int]:
    let alpha = payload.0
    let source = payload.1
    let target = payload.2
    let factor = payload.3
    if factor <= 1:
        return alpha
    let mut out = empty_int_list :: :: call
    let mut y = 0
    while y < target.1:
        let mut x = 0
        while x < target.0:
            let mut total = 0
            let mut sample_y = 0
            while sample_y < factor:
                let mut sample_x = 0
                while sample_x < factor:
                    let source_x = (x * factor) + sample_x
                    let source_y = (y * factor) + sample_y
                    if source_x >= 0 and source_x < source.0 and source_y >= 0 and source_y < source.1:
                        total += byte_at_or_zero :: alpha, (source_y * source.0) + source_x :: call
                    sample_x += 1
                sample_y += 1
            out :: (total / (factor * factor)) :: push
            x += 1
        y += 1
    return std.collections.array.from_list[Int] :: out :: call

fn lcd_from_scaled_alpha(read payload: (Array[Int], (Int, Int), (Int, Int), Int)) -> Array[Int]:
    let alpha = payload.0
    let source = payload.1
    let target = payload.2
    let factor = payload.3
    if factor < 3:
        return empty_lcd :: :: call
    let mut out = empty_int_list :: :: call
    let band = max_int :: (factor / 3), 1 :: call
    let mut y = 0
    while y < target.1:
        let mut x = 0
        while x < target.0:
            let mut channel = 0
            while channel < 3:
                let start_x = (x * factor) + (channel * band)
                let end_x = match channel == 2:
                    true => ((x + 1) * factor)
                    false => (x * factor) + ((channel + 1) * band)
                let mut total = 0
                let mut count = 0
                let mut sample_y = 0
                while sample_y < factor:
                    let mut sample_x = start_x
                    while sample_x < end_x:
                        if sample_x >= 0 and sample_x < source.0:
                            total += byte_at_or_zero :: alpha, (((y * factor) + sample_y) * source.0) + sample_x :: call
                            count += 1
                        sample_x += 1
                    sample_y += 1
                if count <= 0:
                    out :: 0 :: push
                else:
                    out :: (total / count) :: push
                channel += 1
            x += 1
        y += 1
    return std.collections.array.from_list[Int] :: out :: call

fn raster_sample_boost(value: Int, font_size: Int, read mode: arcana_text.types.RasterMode) -> Int:
    let clamped = clamp_int :: value, 0, 255 :: call
    let scaled = match mode:
        arcana_text.types.RasterMode.Lcd => match font_size <= 14:
            true => (clamped * 120) / 100
            false => match font_size <= 22:
                true => (clamped * 112) / 100
                false => clamped
        _ => match font_size <= 14:
            true => (clamped * 118) / 100
            false => match font_size <= 22:
                true => (clamped * 108) / 100
                false => clamped
    return clamp_int :: scaled, 0, 255 :: call

fn boosted_alpha(read alpha: Array[Int], font_size: Int, read mode: arcana_text.types.RasterMode) -> Array[Int]:
    if font_size > 22:
        return alpha
    let mut out = empty_int_list :: :: call
    let total = alpha :: :: len
    let mut index = 0
    while index < total:
        out :: (arcana_text.font_leaf.raster_sample_boost :: ((alpha)[index]), font_size, mode :: call) :: push
        index += 1
    return std.collections.array.from_list[Int] :: out :: call

fn boosted_lcd(read pixels: Array[Int], font_size: Int) -> Array[Int]:
    if font_size > 22:
        return pixels
    let mut out = empty_int_list :: :: call
    let total = pixels :: :: len
    let mut index = 0
    while index < total:
        out :: (arcana_text.font_leaf.raster_sample_boost :: ((pixels)[index]), font_size, (arcana_text.types.RasterMode.Lcd :: :: call) :: call) :: push
        index += 1
    return std.collections.array.from_list[Int] :: out :: call

fn alpha_at_or_zero(read payload: (Array[Int], Int, Int, Int, Int)) -> Int:
    let pixels = payload.0
    let width = payload.1
    let height = payload.2
    let x = payload.3
    let y = payload.4
    if x < 0 or y < 0 or x >= width or y >= height:
        return 0
    return (pixels)[(y * width) + x]

fn filtered_alpha(read pixels: Array[Int], dims: (Int, Int)) -> Array[Int]:
    let width = dims.0
    let height = dims.1
    if width <= 0 or height <= 0 or (pixels :: :: len) != (width * height):
        return pixels
    let mut out = empty_int_list :: :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let a00 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x - 1, y - 1) :: call
            let a01 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x, y - 1) :: call
            let a02 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x + 1, y - 1) :: call
            let a10 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x - 1, y) :: call
            let a11 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x, y) :: call
            let a12 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x + 1, y) :: call
            let a20 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x - 1, y + 1) :: call
            let a21 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x, y + 1) :: call
            let a22 = arcana_text.font_leaf.alpha_at_or_zero :: (pixels, width, height, x + 1, y + 1) :: call
            out :: ((a00 + (a01 * 2) + a02 + (a10 * 2) + (a11 * 4) + (a12 * 2) + a20 + (a21 * 2) + a22) / 16) :: push
            x += 1
        y += 1
    return std.collections.array.from_list[Int] :: out :: call

fn lcd_channel_at_or_zero(read payload: (Array[Int], Int, Int, Int, Int, Int)) -> Int:
    let pixels = payload.0
    let width = payload.1
    let height = payload.2
    let x = payload.3
    let y = payload.4
    let channel = payload.5
    if x < 0 or y < 0 or x >= width or y >= height or channel < 0 or channel >= 3:
        return 0
    return (pixels)[((y * width) + x) * 3 + channel]

fn filtered_lcd(read pixels: Array[Int], dims: (Int, Int)) -> Array[Int]:
    let width = dims.0
    let height = dims.1
    if width <= 0 or height <= 0 or (pixels :: :: len) != (width * height * 3):
        return pixels
    let mut out = empty_int_list :: :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let mut channel = 0
            while channel < 3:
                let left2 = arcana_text.font_leaf.lcd_channel_at_or_zero :: (pixels, width, height, x - 2, y, channel) :: call
                let left1 = arcana_text.font_leaf.lcd_channel_at_or_zero :: (pixels, width, height, x - 1, y, channel) :: call
                let center = arcana_text.font_leaf.lcd_channel_at_or_zero :: (pixels, width, height, x, y, channel) :: call
                let right1 = arcana_text.font_leaf.lcd_channel_at_or_zero :: (pixels, width, height, x + 1, y, channel) :: call
                let right2 = arcana_text.font_leaf.lcd_channel_at_or_zero :: (pixels, width, height, x + 2, y, channel) :: call
                out :: ((left2 + (left1 * 2) + (center * 3) + (right1 * 2) + right2) / 9) :: push
                channel += 1
            x += 1
        y += 1
    return std.collections.array.from_list[Int] :: out :: call

fn raster_oversample_scale(font_size: Int, read mode: arcana_text.types.RasterMode) -> Int:
    if font_size <= 0:
        return 1
    if mode == (arcana_text.types.RasterMode.Lcd :: :: call):
        if font_size <= 12:
            return 8
        if font_size <= 14:
            return 6
        if font_size <= 22:
            return 4
        if font_size <= 28:
            return 3
        return 2
    if mode == (arcana_text.types.RasterMode.Color :: :: call):
        if font_size <= 14:
            return 6
        if font_size <= 24:
            return 3
        return 2
    if font_size <= 12:
        return 6
    if font_size <= 18:
        return 4
    if font_size <= 32:
        return 2
    return 1

fn hinted_oversample_scale(font_size: Int, read mode: arcana_text.types.RasterMode, read hinting: arcana_text.types.Hinting) -> Int:
    let scale = arcana_text.font_leaf.raster_oversample_scale :: font_size, mode :: call
    if hinting != (arcana_text.types.Hinting.Enabled :: :: call):
        return scale
    if scale >= 8:
        return scale
    if mode == (arcana_text.types.RasterMode.Lcd :: :: call):
        return arcana_text.font_leaf.max_int :: scale, 8 :: call
    return arcana_text.font_leaf.max_int :: scale, 6 :: call

fn fill_bitmap_from_segments(read payload: (List[arcana_text.font_leaf.LineSegment], Int, Int, Int)) -> Array[Int]:
    let segments = payload.0
    let width = payload.1
    let height = payload.2
    let oversample = payload.3
    if oversample <= 1:
        return arcana_text.font_leaf.fill_bitmap_binary_from_segments :: segments, width, height :: call
    let scaled_width = width * oversample
    let scaled_height = height * oversample
    let scaled_segments = arcana_text.font_leaf.scale_segments_for_raster :: segments, oversample :: call
    let scaled_alpha = arcana_text.font_leaf.fill_bitmap_binary_from_segments :: scaled_segments, scaled_width, scaled_height :: call
    return arcana_text.font_leaf.downsample_alpha :: (scaled_alpha, (scaled_width, scaled_height), (width, height), oversample) :: call

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
    let vertical = match request.vertical:
        true => "1"
        false => "0"
    let hinting = match request.hinting:
        arcana_text.types.Hinting.Enabled => "hint"
        _ => "plain"
    return (std.text.from_int :: request.glyph_index :: call) + ":" + (std.text.from_int :: request.font_size :: call) + ":" + (std.text.from_int :: request.traits.weight :: call) + ":" + (std.text.from_int :: request.traits.width_milli :: call) + ":" + (std.text.from_int :: request.traits.slant_milli :: call) + ":" + (std.text.from_int :: request.feature_signature :: call) + ":" + (std.text.from_int :: request.axis_signature :: call) + ":" + vertical + ":" + (arcana_text.font_leaf.raster_mode_key :: request.mode :: call) + ":" + (std.text.from_int :: request.color :: call) + ":" + hinting

fn raster_mode_key(read mode: arcana_text.types.RasterMode) -> Str:
    return match mode:
        arcana_text.types.RasterMode.Lcd => "lcd"
        arcana_text.types.RasterMode.Color => "color"
        _ => "alpha"

fn empty_bitmap_metrics(advance: Int, baseline: Int, line_height: Int) -> arcana_text.font_leaf.GlyphBitmap:
    let mut bitmap = arcana_text.font_leaf.GlyphBitmap :: size = (0, 0), offset = (0, 0), advance = advance :: call
    bitmap.baseline = baseline
    bitmap.line_height = line_height
    bitmap.empty = true
    bitmap.alpha = empty_alpha :: :: call
    bitmap.lcd = empty_lcd :: :: call
    bitmap.rgba = empty_rgba :: :: call
    return bitmap

fn color_rgba(read color: Int) -> (Int, Int, Int, Int):
    return ((color / 65536) % 256, (color / 256) % 256, color % 256, 255)

fn rgba_alpha(read rgba: Array[Int]) -> Array[Int]:
    let total = (rgba :: :: len) / 4
    let mut alpha = std.kernel.collections.array_new[Int] :: total, 0 :: call
    let mut index = 0
    while index < total:
        alpha[index] = (rgba)[(index * 4) + 3]
        index += 1
    return alpha

fn cpal_color_rgba(read face: arcana_text.font_leaf.FontFaceState, palette_index: Int, foreground_color: Int) -> (Int, Int, Int, Int):
    if palette_index == 65535:
        return arcana_text.font_leaf.color_rgba :: foreground_color :: call
    if face.cpal_offset < 0 or face.cpal_length < 14:
        return (0, 0, 0, 0)
    let total = face.font_view :: :: len
    if face.cpal_offset + 14 > total:
        return (0, 0, 0, 0)
    let num_palette_entries = u16_be_ref :: face.font_view, face.cpal_offset + 2 :: call
    let num_palettes = u16_be_ref :: face.font_view, face.cpal_offset + 4 :: call
    let num_color_records = u16_be_ref :: face.font_view, face.cpal_offset + 6 :: call
    let color_records_offset = face.cpal_offset + (u32_be_ref :: face.font_view, face.cpal_offset + 8 :: call)
    if num_palettes <= 0 or palette_index < 0 or palette_index >= num_palette_entries:
        return (0, 0, 0, 0)
    let palette_base = u16_be_ref :: face.font_view, face.cpal_offset + 12 :: call
    let color_record_index = palette_base + palette_index
    if color_record_index < 0 or color_record_index >= num_color_records:
        return (0, 0, 0, 0)
    let color_offset = color_records_offset + (color_record_index * 4)
    if color_offset + 4 > total:
        return (0, 0, 0, 0)
    let blue = byte_at_or_zero_ref :: face.font_view, color_offset :: call
    let green = byte_at_or_zero_ref :: face.font_view, color_offset + 1 :: call
    let red = byte_at_or_zero_ref :: face.font_view, color_offset + 2 :: call
    let alpha = byte_at_or_zero_ref :: face.font_view, color_offset + 3 :: call
    return (red, green, blue, alpha)

fn alpha_from_f2dot14(raw: Int) -> Int:
    let clamped = arcana_text.font_leaf.clamp_int :: raw, 0, 16384 :: call
    return (clamped * 255 + 8192) / 16384

fn fixed_16_16_identity() -> Int:
    return 65536

fn fixed_16_16_mul(a: Int, b: Int) -> Int:
    return (a * b) / 65536

fn fixed_16_16_div(a: Int, b: Int) -> Int:
    if b == 0:
        return 0
    return (a * 65536) / b

fn fixed_16_16_from_f2dot14(raw: Int) -> Int:
    return (raw * 65536) / 16384

fn fixed_pi() -> Int:
    return 205887

fn fixed_two_pi() -> Int:
    return 411775

fn fixed_half_pi() -> Int:
    return 102943

fn fixed_quarter_pi() -> Int:
    return 51471

fn fixed_radians_from_f2dot14(raw: Int) -> Int:
    return (raw * (arcana_text.font_leaf.fixed_pi :: :: call)) / 16384

fn int_sqrt(value: Int) -> Int:
    if value <= 0:
        return 0
    let mut low = 0
    let mut high = value
    let mut out = 0
    while low <= high:
        let mid = (low + high) / 2
        let square = mid * mid
        if square == value:
            return mid
        if square < value:
            out = mid
            low = mid + 1
        else:
            high = mid - 1
    return out

fn fixed_sin_pi(radians: Int) -> Int:
    let pi = arcana_text.font_leaf.fixed_pi :: :: call
    let two_pi = arcana_text.font_leaf.fixed_two_pi :: :: call
    let half_pi = arcana_text.font_leaf.fixed_half_pi :: :: call
    let mut angle = radians % two_pi
    if angle < 0:
        angle += two_pi
    if angle > pi:
        angle -= two_pi
    let mut sign = 1
    if angle < 0:
        angle = 0 - angle
        sign = -1
    if angle > half_pi:
        angle = pi - angle
    let numerator = 16 * angle * (pi - angle)
    let denominator = (5 * pi * pi) - (4 * angle * (pi - angle))
    if denominator == 0:
        return 0
    let value = (numerator * 65536) / denominator
    return value * sign

fn fixed_cos_pi(radians: Int) -> Int:
    return arcana_text.font_leaf.fixed_sin_pi :: ((arcana_text.font_leaf.fixed_half_pi :: :: call) - radians) :: call

fn fixed_tan_from_f2dot14(raw: Int) -> Int:
    let radians = arcana_text.font_leaf.fixed_radians_from_f2dot14 :: raw :: call
    let sine = arcana_text.font_leaf.fixed_sin_pi :: radians :: call
    let cosine = arcana_text.font_leaf.fixed_cos_pi :: radians :: call
    if cosine == 0:
        return 0
    return arcana_text.font_leaf.fixed_16_16_div :: sine, cosine :: call

fn fixed_atan2(y: Int, x: Int) -> Int:
    if x == 0:
        if y > 0:
            return arcana_text.font_leaf.fixed_half_pi :: :: call
        if y < 0:
            return 0 - (arcana_text.font_leaf.fixed_half_pi :: :: call)
        return 0
    let abs_y = arcana_text.font_leaf.abs_int :: y :: call
    let quarter = arcana_text.font_leaf.fixed_quarter_pi :: :: call
    let mut angle = 0
    if x >= 0:
        let r = arcana_text.font_leaf.fixed_16_16_div :: (x - abs_y), (x + abs_y) :: call
        angle = quarter - (arcana_text.font_leaf.fixed_16_16_mul :: quarter, r :: call)
    else:
        let three_quarter = quarter * 3
        let r = arcana_text.font_leaf.fixed_16_16_div :: (x + abs_y), (abs_y - x) :: call
        angle = three_quarter - (arcana_text.font_leaf.fixed_16_16_mul :: quarter, r :: call)
    if y < 0:
        return 0 - angle
    return angle

fn colr_version(read face: arcana_text.font_leaf.FontFaceState) -> Int:
    if face.colr_offset < 0 or face.colr_length < 2:
        return -1
    if face.colr_offset + 2 > (face.font_view :: :: len):
        return -1
    return u16_be_ref :: face.font_view, face.colr_offset :: call

fn colr_v1_base_glyph_list_offset(read face: arcana_text.font_leaf.FontFaceState) -> Int:
    if (arcana_text.font_leaf.colr_version :: face :: call) != 1:
        return -1
    if face.colr_offset + 18 > (face.font_view :: :: len):
        return -1
    let relative = u32_be_ref :: face.font_view, face.colr_offset + 14 :: call
    if relative <= 0:
        return -1
    return face.colr_offset + relative

fn colr_v1_layer_list_offset(read face: arcana_text.font_leaf.FontFaceState) -> Int:
    if (arcana_text.font_leaf.colr_version :: face :: call) != 1:
        return -1
    if face.colr_offset + 22 > (face.font_view :: :: len):
        return -1
    let relative = u32_be_ref :: face.font_view, face.colr_offset + 18 :: call
    if relative <= 0:
        return -1
    return face.colr_offset + relative

fn colr_v1_base_paint_offset(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Int:
    let base_list = arcana_text.font_leaf.colr_v1_base_glyph_list_offset :: face :: call
    if base_list < 0 or base_list + 4 > (face.font_view :: :: len):
        return -1
    let count = u32_be_ref :: face.font_view, base_list :: call
    let mut index = 0
    while index < count:
        let record = base_list + 4 + (index * 6)
        if record + 6 > (face.font_view :: :: len):
            return -1
        if (u16_be_ref :: face.font_view, record :: call) == glyph_index:
            let relative = u32_be_ref :: face.font_view, record + 2 :: call
            if relative <= 0:
                return -1
            return base_list + relative
        index += 1
    return -1

fn colr_v1_layer_paint_offset(read face: arcana_text.font_leaf.FontFaceState, layer_index: Int) -> Int:
    let layer_list = arcana_text.font_leaf.colr_v1_layer_list_offset :: face :: call
    if layer_list < 0 or layer_list + 4 > (face.font_view :: :: len):
        return -1
    let count = u32_be_ref :: face.font_view, layer_list :: call
    if layer_index < 0 or layer_index >= count:
        return -1
    let record = layer_list + 4 + (layer_index * 4)
    if record + 4 > (face.font_view :: :: len):
        return -1
    let relative = u32_be_ref :: face.font_view, record :: call
    if relative <= 0:
        return -1
    return layer_list + relative

fn paint_solid_color(read face: arcana_text.font_leaf.FontFaceState, paint_offset: Int, foreground_color: Int) -> (Int, Int, Int, Int):
    if paint_offset < 0 or paint_offset + 5 > (face.font_view :: :: len):
        return (0, 0, 0, 0)
    let format = byte_at_or_zero_ref :: face.font_view, paint_offset :: call
    if format != 2 and format != 3:
        return (0, 0, 0, 0)
    let palette = u16_be_ref :: face.font_view, paint_offset + 1 :: call
    let alpha = arcana_text.font_leaf.alpha_from_f2dot14 :: (u16_be_ref :: face.font_view, paint_offset + 3 :: call) :: call
    let base = arcana_text.font_leaf.cpal_color_rgba :: face, palette, foreground_color :: call
    return (base.0, base.1, base.2, (base.3 * alpha) / 255)

fn empty_colr_color_stops() -> List[arcana_text.font_leaf.ColrColorStop]:
    return std.collections.list.empty[arcana_text.font_leaf.ColrColorStop] :: :: call

fn empty_colr_color_line() -> arcana_text.font_leaf.ColrColorLine:
    return arcana_text.font_leaf.ColrColorLine :: extend = 0, stops = (arcana_text.font_leaf.empty_colr_color_stops :: :: call) :: call

fn paint_color_line_offset(read face: arcana_text.font_leaf.FontFaceState, paint_offset: Int) -> Int:
    if paint_offset < 0 or paint_offset + 4 > (face.font_view :: :: len):
        return -1
    let relative = u24_be_ref :: face.font_view, paint_offset + 1 :: call
    if relative <= 0:
        return -1
    return paint_offset + relative

fn colr_color_line(read face: arcana_text.font_leaf.FontFaceState, paint_offset: Int, foreground_color: Int) -> arcana_text.font_leaf.ColrColorLine:
    let mut out = arcana_text.font_leaf.empty_colr_color_line :: :: call
    let color_line_offset = arcana_text.font_leaf.paint_color_line_offset :: face, paint_offset :: call
    if color_line_offset < 0 or color_line_offset + 3 > (face.font_view :: :: len):
        return out
    out.extend = byte_at_or_zero_ref :: face.font_view, color_line_offset :: call
    let stop_count = u16_be_ref :: face.font_view, color_line_offset + 1 :: call
    let mut cursor = color_line_offset + 3
    let mut index = 0
    while index < stop_count and cursor + 6 <= (face.font_view :: :: len):
        let stop_offset = clamp_int :: (i16_be_ref :: face.font_view, cursor :: call), 0, 16384 :: call
        let palette_index = u16_be_ref :: face.font_view, cursor + 2 :: call
        let alpha = arcana_text.font_leaf.alpha_from_f2dot14 :: (i16_be_ref :: face.font_view, cursor + 4 :: call) :: call
        let base = arcana_text.font_leaf.cpal_color_rgba :: face, palette_index, foreground_color :: call
        out.stops :: (arcana_text.font_leaf.ColrColorStop :: offset = stop_offset, color = (base.0, base.1, base.2, (base.3 * alpha) / 255) :: call) :: push
        cursor += 6
        index += 1
    return out

fn color_line_position(read line: arcana_text.font_leaf.ColrColorLine, value: Int) -> Int:
    if line.extend == 1:
        return positive_mod :: value, 16384 :: call
    if line.extend == 2:
        let mirrored = positive_mod :: value, 32768 :: call
        if mirrored > 16384:
            return 32768 - mirrored
        return mirrored
    return clamp_int :: value, 0, 16384 :: call

fn lerp_channel(a: Int, b: Int, t: Int) -> Int:
    return a + (((b - a) * t) / 16384)

fn lerp_rgba(read left: (Int, Int, Int, Int), read right: (Int, Int, Int, Int), t: Int) -> (Int, Int, Int, Int):
    return ((arcana_text.font_leaf.lerp_channel :: left.0, right.0, t :: call), (arcana_text.font_leaf.lerp_channel :: left.1, right.1, t :: call), (arcana_text.font_leaf.lerp_channel :: left.2, right.2, t :: call), (arcana_text.font_leaf.lerp_channel :: left.3, right.3, t :: call))

fn sample_color_line(read line: arcana_text.font_leaf.ColrColorLine, value: Int) -> (Int, Int, Int, Int):
    let total = line.stops :: :: len
    if total <= 0:
        return (0, 0, 0, 0)
    let target = arcana_text.font_leaf.color_line_position :: line, value :: call
    if total == 1:
        return (line.stops)[0].color
    if target <= (line.stops)[0].offset:
        return (line.stops)[0].color
    let mut index = 1
    while index < total:
        let left = (line.stops)[index - 1]
        let right = (line.stops)[index]
        if target <= right.offset:
            let span = max_int :: (right.offset - left.offset), 1 :: call
            let local = ((target - left.offset) * 16384) / span
            return arcana_text.font_leaf.lerp_rgba :: left.color, right.color, local :: call
        index += 1
    return (line.stops)[total - 1].color

fn gradient_paint_bounds(read payload: (Int, Int, Int, Int)) -> ((Int, Int), (Int, Int)):
    let advance = payload.0
    let baseline = payload.1
    let line_height = payload.2
    let font_size = payload.3
    let pad = max_int :: (max_int :: line_height, font_size :: call), 1 :: call
    let width = max_int :: (max_int :: advance, font_size :: call), 1 :: call
    let height = max_int :: (max_int :: line_height, font_size :: call), 1 :: call
    return ((0 - pad, (baseline - height) - pad), (width + (pad * 2), height + (pad * 2)))

fn glyph_bitmap_from_rgba(read payload: (Array[Int], (Int, Int), (Int, Int), Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let rgba = payload.0
    let offset = payload.1
    let size = payload.2
    let advance = payload.3
    let baseline = payload.4
    let line_height = payload.5
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = size, offset = offset, advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = arcana_text.font_leaf.rgba_alpha :: rgba :: call
    out.lcd = empty_lcd :: :: call
    out.rgba = rgba
    return out

fn linear_gradient_position(point: (Int, Int), start: (Int, Int), end: (Int, Int)) -> Int:
    let dx = end.0 - start.0
    let dy = end.1 - start.1
    let denom = (dx * dx) + (dy * dy)
    if denom <= 0:
        return 0
    let numer = ((point.0 - start.0) * dx) + ((point.1 - start.1) * dy)
    return (numer * 16384) / denom

fn radial_gradient_position(read payload: ((Int, Int), (Int, Int), Int, (Int, Int), Int)) -> Int:
    let point = payload.0
    let center0 = payload.1
    let radius0 = payload.2
    let center1 = payload.3
    let radius1 = payload.4
    let dx = center1.0 - center0.0
    let dy = center1.1 - center0.1
    let radius_span = radius1 - radius0
    let denom = (dx * dx) + (dy * dy)
    if denom > 0:
        let numer = ((point.0 - center0.0) * dx) + ((point.1 - center0.1) * dy)
        return (numer * 16384) / denom
    let distance = arcana_text.font_leaf.int_sqrt :: (((point.0 - center0.0) * (point.0 - center0.0)) + ((point.1 - center0.1) * (point.1 - center0.1))) :: call
    if radius_span == 0:
        return 0
    return ((distance - radius0) * 16384) / radius_span

fn sweep_gradient_position(read payload: ((Int, Int), (Int, Int), Int, Int)) -> Int:
    let point = payload.0
    let center = payload.1
    let start_angle = payload.2
    let end_angle = payload.3
    let angle = arcana_text.font_leaf.fixed_atan2 :: (point.1 - center.1), (point.0 - center.0) :: call
    let mut span = end_angle - start_angle
    if span == 0:
        return 0
    let two_pi = arcana_text.font_leaf.fixed_two_pi :: :: call
    while span < 0:
        span += two_pi
    let mut relative = angle - start_angle
    while relative < 0:
        relative += two_pi
    return (relative * 16384) / span

fn render_linear_gradient_bitmap(read face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let paint_offset = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let line = arcana_text.font_leaf.colr_color_line :: face, paint_offset, spec.color :: call
    if (line.stops :: :: is_empty):
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let bounds = arcana_text.font_leaf.gradient_paint_bounds :: (advance, baseline, line_height, spec.font_size) :: call
    let offset = bounds.0
    let size = bounds.1
    let width = size.0
    let height = size.1
    let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
    let start = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 4 :: call), (i16_be_ref :: face.font_view, paint_offset + 6 :: call)) :: call
    let end = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 8 :: call), (i16_be_ref :: face.font_view, paint_offset + 10 :: call)) :: call
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let point = (offset.0 + x, offset.1 + y)
            let t = arcana_text.font_leaf.linear_gradient_position :: point, start, end :: call
            let color = arcana_text.font_leaf.sample_color_line :: line, t :: call
            let dest = ((y * width) + x) * 4
            rgba[dest] = color.0
            rgba[dest + 1] = color.1
            rgba[dest + 2] = color.2
            rgba[dest + 3] = color.3
            x += 1
        y += 1
    return arcana_text.font_leaf.glyph_bitmap_from_rgba :: (rgba, offset, size, advance, baseline, line_height) :: call

fn render_radial_gradient_bitmap(read face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let paint_offset = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let line = arcana_text.font_leaf.colr_color_line :: face, paint_offset, spec.color :: call
    if (line.stops :: :: is_empty):
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let bounds = arcana_text.font_leaf.gradient_paint_bounds :: (advance, baseline, line_height, spec.font_size) :: call
    let offset = bounds.0
    let size = bounds.1
    let width = size.0
    let height = size.1
    let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
    let center0 = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 4 :: call), (i16_be_ref :: face.font_view, paint_offset + 6 :: call)) :: call
    let radius0 = scale_y :: (u16_be_ref :: face.font_view, paint_offset + 8 :: call), spec.font_size, face.units_per_em :: call
    let center1 = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 10 :: call), (i16_be_ref :: face.font_view, paint_offset + 12 :: call)) :: call
    let radius1 = scale_y :: (u16_be_ref :: face.font_view, paint_offset + 14 :: call), spec.font_size, face.units_per_em :: call
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let point = (offset.0 + x, offset.1 + y)
            let t = arcana_text.font_leaf.radial_gradient_position :: (point, center0, radius0, center1, radius1) :: call
            let color = arcana_text.font_leaf.sample_color_line :: line, t :: call
            let dest = ((y * width) + x) * 4
            rgba[dest] = color.0
            rgba[dest + 1] = color.1
            rgba[dest + 2] = color.2
            rgba[dest + 3] = color.3
            x += 1
        y += 1
    return arcana_text.font_leaf.glyph_bitmap_from_rgba :: (rgba, offset, size, advance, baseline, line_height) :: call

fn render_sweep_gradient_bitmap(read face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let paint_offset = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let line = arcana_text.font_leaf.colr_color_line :: face, paint_offset, spec.color :: call
    if (line.stops :: :: is_empty):
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let bounds = arcana_text.font_leaf.gradient_paint_bounds :: (advance, baseline, line_height, spec.font_size) :: call
    let offset = bounds.0
    let size = bounds.1
    let width = size.0
    let height = size.1
    let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
    let center = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 4 :: call), (i16_be_ref :: face.font_view, paint_offset + 6 :: call)) :: call
    let start_angle = arcana_text.font_leaf.fixed_radians_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 8 :: call) :: call
    let end_angle = arcana_text.font_leaf.fixed_radians_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 10 :: call) :: call
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let point = (offset.0 + x, offset.1 + y)
            let t = arcana_text.font_leaf.sweep_gradient_position :: (point, center, start_angle, end_angle) :: call
            let color = arcana_text.font_leaf.sample_color_line :: line, t :: call
            let dest = ((y * width) + x) * 4
            rgba[dest] = color.0
            rgba[dest + 1] = color.1
            rgba[dest + 2] = color.2
            rgba[dest + 3] = color.3
            x += 1
        y += 1
    return arcana_text.font_leaf.glyph_bitmap_from_rgba :: (rgba, offset, size, advance, baseline, line_height) :: call

fn empty_colr_layers() -> List[arcana_text.font_leaf.ColrLayerRecord]:
    return std.collections.list.new[arcana_text.font_leaf.ColrLayerRecord] :: :: call

fn colr_layers_for_glyph(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> List[arcana_text.font_leaf.ColrLayerRecord]:
    let mut out = arcana_text.font_leaf.empty_colr_layers :: :: call
    if face.colr_offset < 0 or face.colr_length < 14 or face.cpal_offset < 0:
        return out
    let total = face.font_view :: :: len
    if face.colr_offset + 14 > total:
        return out
    let version = u16_be_ref :: face.font_view, face.colr_offset :: call
    if version != 0:
        return out
    let base_count = u16_be_ref :: face.font_view, face.colr_offset + 2 :: call
    let base_offset = face.colr_offset + (u32_be_ref :: face.font_view, face.colr_offset + 4 :: call)
    let layer_offset = face.colr_offset + (u32_be_ref :: face.font_view, face.colr_offset + 8 :: call)
    let layer_count = u16_be_ref :: face.font_view, face.colr_offset + 12 :: call
    let mut index = 0
    while index < base_count:
        let record = base_offset + (index * 6)
        if record + 6 > total:
            return arcana_text.font_leaf.empty_colr_layers :: :: call
        if (u16_be_ref :: face.font_view, record :: call) == glyph_index:
            let first_layer = u16_be_ref :: face.font_view, record + 2 :: call
            let layer_total = u16_be_ref :: face.font_view, record + 4 :: call
            let mut layer_index = 0
            while layer_index < layer_total and (first_layer + layer_index) < layer_count:
                let layer_record = layer_offset + ((first_layer + layer_index) * 4)
                if layer_record + 4 > total:
                    return arcana_text.font_leaf.empty_colr_layers :: :: call
                let layer_glyph = u16_be_ref :: face.font_view, layer_record :: call
                let palette_value = u16_be_ref :: face.font_view, layer_record + 2 :: call
                let layer_value = arcana_text.font_leaf.ColrLayerRecord :: glyph_index = layer_glyph, palette_index = palette_value :: call
                out :: layer_value :: push
                layer_index += 1
            return out
        index += 1
    return out

fn empty_decoded_color_image() -> arcana_text.font_leaf.DecodedColorImage:
    return arcana_text.font_leaf.DecodedColorImage :: size = (0, 0), rgba = (empty_rgba :: :: call) :: call

fn empty_embedded_bitmap_metrics() -> arcana_text.font_leaf.EmbeddedBitmapMetrics:
    return arcana_text.font_leaf.EmbeddedBitmapMetrics :: offset = (0, 0), advance = 0 :: call

fn empty_embedded_bitmap_image() -> arcana_text.font_leaf.EmbeddedBitmapImage:
    let mut image = arcana_text.font_leaf.EmbeddedBitmapImage :: format_tag = "", bytes = (empty_alpha :: :: call), metrics = (arcana_text.font_leaf.empty_embedded_bitmap_metrics :: :: call) :: call
    image.draw_outline = false
    image.bottom_origin = false
    return image

fn inflate_pow2(bits: Int) -> Int:
    if bits <= 0:
        return 1
    let mut out = 1
    let mut index = 0
    while index < bits:
        out *= 2
        index += 1
    return out

fn empty_huffman_table() -> arcana_text.font_leaf.HuffmanTable:
    return arcana_text.font_leaf.HuffmanTable :: lengths = (empty_alpha :: :: call), codes = (empty_alpha :: :: call), max_bits = 0 :: call

fn deflate_reader(read bytes: Array[Int], start: Int) -> arcana_text.font_leaf.DeflateBitReader:
    let mut reader = arcana_text.font_leaf.DeflateBitReader :: bytes = bytes, cursor = start, bit_value = 0 :: call
    reader.bit_count = 0
    return reader

fn deflate_read_bits(edit reader: arcana_text.font_leaf.DeflateBitReader, count: Int) -> Result[Int, Str]:
    if count < 0 or count > 24:
        return Result.Err[Int, Str] :: "invalid deflate bit count" :: call
    while reader.bit_count < count:
        if reader.cursor >= (reader.bytes :: :: len):
            return Result.Err[Int, Str] :: "deflate stream ended unexpectedly" :: call
        reader.bit_value += ((reader.bytes)[reader.cursor]) * (arcana_text.font_leaf.inflate_pow2 :: reader.bit_count :: call)
        reader.bit_count += 8
        reader.cursor += 1
    let scale = arcana_text.font_leaf.inflate_pow2 :: count :: call
    let value = positive_mod :: reader.bit_value, scale :: call
    reader.bit_value = reader.bit_value / scale
    reader.bit_count -= count
    return Result.Ok[Int, Str] :: value :: call

fn deflate_align_byte(edit reader: arcana_text.font_leaf.DeflateBitReader):
    reader.bit_value = 0
    reader.bit_count = 0

fn huffman_table_from_lengths(read lengths: Array[Int]) -> Result[arcana_text.font_leaf.HuffmanTable, Str]:
    if lengths :: :: is_empty:
        return Result.Err[arcana_text.font_leaf.HuffmanTable, Str] :: "empty huffman length set" :: call
    let mut counts = std.kernel.collections.array_new[Int] :: 16, 0 :: call
    let mut max_bits = 0
    let mut symbol = 0
    while symbol < (lengths :: :: len):
        let length = (lengths)[symbol]
        if length < 0 or length >= 16:
            return Result.Err[arcana_text.font_leaf.HuffmanTable, Str] :: "unsupported huffman code length" :: call
        if length > 0:
            counts[length] = (counts)[length] + 1
            if length > max_bits:
                max_bits = length
        symbol += 1
    if max_bits <= 0:
        return Result.Err[arcana_text.font_leaf.HuffmanTable, Str] :: "huffman table has no symbols" :: call
    let mut next = std.kernel.collections.array_new[Int] :: 16, 0 :: call
    let mut code = 0
    let mut bits = 1
    while bits <= max_bits:
        code = (code + (counts)[bits - 1]) * 2
        next[bits] = code
        bits += 1
    let mut codes = std.kernel.collections.array_new[Int] :: (lengths :: :: len), 0 :: call
    symbol = 0
    while symbol < (lengths :: :: len):
        let length = (lengths)[symbol]
        if length > 0:
            codes[symbol] = (next)[length]
            next[length] = (next)[length] + 1
        symbol += 1
    return Result.Ok[arcana_text.font_leaf.HuffmanTable, Str] :: (arcana_text.font_leaf.HuffmanTable :: lengths = lengths, codes = codes, max_bits = max_bits :: call) :: call

fn decode_huffman_symbol(edit reader: arcana_text.font_leaf.DeflateBitReader, read table: arcana_text.font_leaf.HuffmanTable) -> Result[Int, Str]:
    let mut code = 0
    let mut place = 1
    let mut bits = 1
    while bits <= table.max_bits:
        let bit = arcana_text.font_leaf.deflate_read_bits :: reader, 1 :: call
        if bit :: :: is_err:
            return Result.Err[Int, Str] :: (result_err_or :: bit, "failed to read huffman symbol bit" :: call) :: call
        code += (bit :: 0 :: unwrap_or) * place
        let mut symbol = 0
        while symbol < (table.lengths :: :: len):
            if (table.lengths)[symbol] == bits and (table.codes)[symbol] == code:
                return Result.Ok[Int, Str] :: symbol :: call
            symbol += 1
        place *= 2
        bits += 1
    return Result.Err[Int, Str] :: "deflate huffman symbol not found" :: call

fn fixed_litlen_lengths() -> Array[Int]:
    let mut lengths = std.kernel.collections.array_new[Int] :: 288, 0 :: call
    let mut index = 0
    while index < 288:
        lengths[index] = match index <= 143:
            true => 8
            false => match index <= 255:
                true => 9
                false => match index <= 279:
                    true => 7
                    false => 8
        index += 1
    return lengths

fn fixed_dist_lengths() -> Array[Int]:
    let mut lengths = std.kernel.collections.array_new[Int] :: 32, 5 :: call
    return lengths

fn deflate_code_length_order(index: Int) -> Int:
    return match index:
        0 => 16
        1 => 17
        2 => 18
        3 => 0
        4 => 8
        5 => 7
        6 => 9
        7 => 6
        8 => 10
        9 => 5
        10 => 11
        11 => 4
        12 => 12
        13 => 3
        14 => 13
        15 => 2
        16 => 14
        17 => 1
        18 => 15
        _ => 0

fn deflate_literal_table() -> Result[arcana_text.font_leaf.HuffmanTable, Str]:
    return arcana_text.font_leaf.huffman_table_from_lengths :: (arcana_text.font_leaf.fixed_litlen_lengths :: :: call) :: call

fn deflate_distance_table() -> Result[arcana_text.font_leaf.HuffmanTable, Str]:
    return arcana_text.font_leaf.huffman_table_from_lengths :: (arcana_text.font_leaf.fixed_dist_lengths :: :: call) :: call

fn deflate_dynamic_tables(edit reader: arcana_text.font_leaf.DeflateBitReader) -> Result[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str]:
    let hlit = arcana_text.font_leaf.deflate_read_bits :: reader, 5 :: call
    if hlit :: :: is_err:
        return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: hlit, "failed to read dynamic HLIT" :: call) :: call
    let hdist = arcana_text.font_leaf.deflate_read_bits :: reader, 5 :: call
    if hdist :: :: is_err:
        return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: hdist, "failed to read dynamic HDIST" :: call) :: call
    let hclen = arcana_text.font_leaf.deflate_read_bits :: reader, 4 :: call
    if hclen :: :: is_err:
        return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: hclen, "failed to read dynamic HCLEN" :: call) :: call
    let lit_count = (hlit :: 0 :: unwrap_or) + 257
    let dist_count = (hdist :: 0 :: unwrap_or) + 1
    let code_len_count = (hclen :: 0 :: unwrap_or) + 4
    let mut code_lengths = std.kernel.collections.array_new[Int] :: 19, 0 :: call
    let mut index = 0
    while index < code_len_count:
        let value = arcana_text.font_leaf.deflate_read_bits :: reader, 3 :: call
        if value :: :: is_err:
            return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: value, "failed to read code length alphabet" :: call) :: call
        code_lengths[arcana_text.font_leaf.deflate_code_length_order :: index :: call] = value :: 0 :: unwrap_or
        index += 1
    let code_table_result = arcana_text.font_leaf.huffman_table_from_lengths :: code_lengths :: call
    if code_table_result :: :: is_err:
        return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: code_table_result, "failed to build code length huffman table" :: call) :: call
    let code_table = code_table_result :: (arcana_text.font_leaf.empty_huffman_table :: :: call) :: unwrap_or
    let total = lit_count + dist_count
    let mut lengths = std.kernel.collections.array_new[Int] :: total, 0 :: call
    index = 0
    while index < total:
        let symbol = arcana_text.font_leaf.decode_huffman_symbol :: reader, code_table :: call
        if symbol :: :: is_err:
            return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: symbol, "failed to decode dynamic code length" :: call) :: call
        let value = symbol :: -1 :: unwrap_or
        if value <= 15:
            lengths[index] = value
            index += 1
        else:
            if value == 16:
                if index <= 0:
                    return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: "dynamic code length repeat has no previous value" :: call
                let extra = arcana_text.font_leaf.deflate_read_bits :: reader, 2 :: call
                if extra :: :: is_err:
                    return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: extra, "failed to read dynamic repeat count" :: call) :: call
                let repeat = (extra :: 0 :: unwrap_or) + 3
                let previous = (lengths)[index - 1]
                let mut count = 0
                while count < repeat and index < total:
                    lengths[index] = previous
                    index += 1
                    count += 1
            else:
                if value == 17 or value == 18:
                    let extra_bits = match value == 17:
                        true => 3
                        false => 7
                    let base = match value == 17:
                        true => 3
                        false => 11
                    let extra = arcana_text.font_leaf.deflate_read_bits :: reader, extra_bits :: call
                    if extra :: :: is_err:
                        return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: extra, "failed to read zero repeat count" :: call) :: call
                    let repeat = (extra :: 0 :: unwrap_or) + base
                    let mut count = 0
                    while count < repeat and index < total:
                        lengths[index] = 0
                        index += 1
                        count += 1
    let mut lit_lengths = std.kernel.collections.array_new[Int] :: lit_count, 0 :: call
    let mut dist_lengths = std.kernel.collections.array_new[Int] :: dist_count, 0 :: call
    index = 0
    while index < lit_count:
        lit_lengths[index] = (lengths)[index]
        index += 1
    index = 0
    while index < dist_count:
        dist_lengths[index] = (lengths)[lit_count + index]
        index += 1
    let lit_table = arcana_text.font_leaf.huffman_table_from_lengths :: lit_lengths :: call
    if lit_table :: :: is_err:
        return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: lit_table, "failed to build literal/length huffman table" :: call) :: call
    let dist_table = arcana_text.font_leaf.huffman_table_from_lengths :: dist_lengths :: call
    if dist_table :: :: is_err:
        return Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: (result_err_or :: dist_table, "failed to build distance huffman table" :: call) :: call
    return Result.Ok[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: ((lit_table :: (arcana_text.font_leaf.empty_huffman_table :: :: call) :: unwrap_or), (dist_table :: (arcana_text.font_leaf.empty_huffman_table :: :: call) :: unwrap_or)) :: call

fn deflate_length_base(symbol: Int) -> (Int, Int):
    return match symbol:
        257 => (3, 0)
        258 => (4, 0)
        259 => (5, 0)
        260 => (6, 0)
        261 => (7, 0)
        262 => (8, 0)
        263 => (9, 0)
        264 => (10, 0)
        265 => (11, 1)
        266 => (13, 1)
        267 => (15, 1)
        268 => (17, 1)
        269 => (19, 2)
        270 => (23, 2)
        271 => (27, 2)
        272 => (31, 2)
        273 => (35, 3)
        274 => (43, 3)
        275 => (51, 3)
        276 => (59, 3)
        277 => (67, 4)
        278 => (83, 4)
        279 => (99, 4)
        280 => (115, 4)
        281 => (131, 5)
        282 => (163, 5)
        283 => (195, 5)
        284 => (227, 5)
        285 => (258, 0)
        _ => (0, 0)

fn deflate_distance_base(symbol: Int) -> (Int, Int):
    return match symbol:
        0 => (1, 0)
        1 => (2, 0)
        2 => (3, 0)
        3 => (4, 0)
        4 => (5, 1)
        5 => (7, 1)
        6 => (9, 2)
        7 => (13, 2)
        8 => (17, 3)
        9 => (25, 3)
        10 => (33, 4)
        11 => (49, 4)
        12 => (65, 5)
        13 => (97, 5)
        14 => (129, 6)
        15 => (193, 6)
        16 => (257, 7)
        17 => (385, 7)
        18 => (513, 8)
        19 => (769, 8)
        20 => (1025, 9)
        21 => (1537, 9)
        22 => (2049, 10)
        23 => (3073, 10)
        24 => (4097, 11)
        25 => (6145, 11)
        26 => (8193, 12)
        27 => (12289, 12)
        28 => (16385, 13)
        29 => (24577, 13)
        _ => (0, 0)

fn deflate_copy_match(edit out: List[Int], distance: Int, length: Int) -> Result[Unit, Str]:
    if distance <= 0 or distance > (out :: :: len):
        return Result.Err[Unit, Str] :: "invalid deflate copy distance" :: call
    let mut remaining = length
    while remaining > 0:
        let source = (out :: :: len) - distance
        out :: (out)[source] :: push
        remaining -= 1
    return Result.Ok[Unit, Str] :: :: call

fn inflate_deflate_payload(read bytes: Array[Int], start: Int, end: Int) -> Result[Array[Int], Str]:
    let mut reader = arcana_text.font_leaf.deflate_reader :: bytes, start :: call
    let mut out = empty_int_list :: :: call
    let mut done = false
    while not done:
        if reader.cursor > end:
            return Result.Err[Array[Int], Str] :: "deflate reader advanced out of range" :: call
        let final_bit = arcana_text.font_leaf.deflate_read_bits :: reader, 1 :: call
        if final_bit :: :: is_err:
            return Result.Err[Array[Int], Str] :: (result_err_or :: final_bit, "failed to read deflate final bit" :: call) :: call
        let block_type = arcana_text.font_leaf.deflate_read_bits :: reader, 2 :: call
        if block_type :: :: is_err:
            return Result.Err[Array[Int], Str] :: (result_err_or :: block_type, "failed to read deflate block type" :: call) :: call
        let kind = block_type :: -1 :: unwrap_or
        if kind == 0:
            arcana_text.font_leaf.deflate_align_byte :: reader :: call
            if reader.cursor + 4 > end:
                return Result.Err[Array[Int], Str] :: "stored deflate block header is truncated" :: call
            let length = u16_le :: reader.bytes, reader.cursor :: call
            let inverse = u16_le :: reader.bytes, reader.cursor + 2 :: call
            reader.cursor += 4
            if positive_mod :: (length + inverse), 65536 :: call != 65535:
                return Result.Err[Array[Int], Str] :: "stored deflate block length check failed" :: call
            if reader.cursor + length > end:
                return Result.Err[Array[Int], Str] :: "stored deflate block payload is truncated" :: call
            let mut index = 0
            while index < length:
                out :: (reader.bytes)[reader.cursor + index] :: push
                index += 1
            reader.cursor += length
        else:
            let tables = match kind:
                1 => {
                    let lit_table = arcana_text.font_leaf.deflate_literal_table :: :: call
                    if lit_table :: :: is_err:
                        return Result.Err[Array[Int], Str] :: (result_err_or :: lit_table, "failed to build fixed literal/length table" :: call) :: call
                    let dist_table = arcana_text.font_leaf.deflate_distance_table :: :: call
                    if dist_table :: :: is_err:
                        return Result.Err[Array[Int], Str] :: (result_err_or :: dist_table, "failed to build fixed distance table" :: call) :: call
                    Result.Ok[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: ((lit_table :: (arcana_text.font_leaf.empty_huffman_table :: :: call) :: unwrap_or), (dist_table :: (arcana_text.font_leaf.empty_huffman_table :: :: call) :: unwrap_or)) :: call
                }
                2 => arcana_text.font_leaf.deflate_dynamic_tables :: reader :: call
                _ => Result.Err[(arcana_text.font_leaf.HuffmanTable, arcana_text.font_leaf.HuffmanTable), Str] :: "reserved deflate block type" :: call
            if tables :: :: is_err:
                return Result.Err[Array[Int], Str] :: (result_err_or :: tables, "failed to load deflate huffman tables" :: call) :: call
            let table_pair = tables :: ((arcana_text.font_leaf.empty_huffman_table :: :: call), (arcana_text.font_leaf.empty_huffman_table :: :: call) :: call) :: unwrap_or
            let lit_table = table_pair.0
            let dist_table = table_pair.1
            let mut block_done = false
            while not block_done:
                let symbol = arcana_text.font_leaf.decode_huffman_symbol :: reader, lit_table :: call
                if symbol :: :: is_err:
                    return Result.Err[Array[Int], Str] :: (result_err_or :: symbol, "failed to decode literal/length symbol" :: call) :: call
                let value = symbol :: -1 :: unwrap_or
                if value < 256:
                    out :: value :: push
                else:
                    if value == 256:
                        block_done = true
                    else:
                        let length_spec = arcana_text.font_leaf.deflate_length_base :: value :: call
                        if length_spec.0 <= 0:
                            return Result.Err[Array[Int], Str] :: "invalid deflate length symbol" :: call
                        let mut length = length_spec.0
                        if length_spec.1 > 0:
                            let extra = arcana_text.font_leaf.deflate_read_bits :: reader, length_spec.1 :: call
                            if extra :: :: is_err:
                                return Result.Err[Array[Int], Str] :: (result_err_or :: extra, "failed to read deflate length extra bits" :: call) :: call
                            length += extra :: 0 :: unwrap_or
                        let dist_symbol = arcana_text.font_leaf.decode_huffman_symbol :: reader, dist_table :: call
                        if dist_symbol :: :: is_err:
                            return Result.Err[Array[Int], Str] :: (result_err_or :: dist_symbol, "failed to decode distance symbol" :: call) :: call
                        let distance_spec = arcana_text.font_leaf.deflate_distance_base :: (dist_symbol :: -1 :: unwrap_or) :: call
                        if distance_spec.0 <= 0:
                            return Result.Err[Array[Int], Str] :: "invalid deflate distance symbol" :: call
                        let mut distance = distance_spec.0
                        if distance_spec.1 > 0:
                            let extra = arcana_text.font_leaf.deflate_read_bits :: reader, distance_spec.1 :: call
                            if extra :: :: is_err:
                                return Result.Err[Array[Int], Str] :: (result_err_or :: extra, "failed to read deflate distance extra bits" :: call) :: call
                            distance += extra :: 0 :: unwrap_or
                        let copy = arcana_text.font_leaf.deflate_copy_match :: out, distance, length :: call
                        if copy :: :: is_err:
                            return Result.Err[Array[Int], Str] :: (result_err_or :: copy, "failed to copy deflate match" :: call) :: call
        if (final_bit :: 0 :: unwrap_or) == 1:
            done = true
    return Result.Ok[Array[Int], Str] :: (std.collections.array.from_list[Int] :: out :: call) :: call

fn inflate_zlib_payload(read bytes: Array[Int]) -> Result[Array[Int], Str]:
    if (bytes :: :: len) < 6:
        return Result.Err[Array[Int], Str] :: "zlib payload is truncated" :: call
    let cmf = (bytes)[0]
    let flg = (bytes)[1]
    if (cmf % 16) != 8:
        return Result.Err[Array[Int], Str] :: "zlib compression method is unsupported" :: call
    if ((cmf * 256) + flg) % 31 != 0:
        return Result.Err[Array[Int], Str] :: "zlib header checksum is invalid" :: call
    if ((flg / 32) % 2) == 1:
        return Result.Err[Array[Int], Str] :: "zlib preset dictionary is unsupported" :: call
    return arcana_text.font_leaf.inflate_deflate_payload :: bytes, 2, ((bytes :: :: len) - 4) :: call

fn gzip_flag_set(flags: Int, bit: Int) -> Bool:
    if bit <= 0:
        return false
    return ((flags / bit) % 2) == 1

fn inflate_gzip_payload(read bytes: Array[Int]) -> Result[Array[Int], Str]:
    if (bytes :: :: len) < 18:
        return Result.Err[Array[Int], Str] :: "gzip payload is truncated" :: call
    if (bytes)[0] != 31 or (bytes)[1] != 139:
        return Result.Err[Array[Int], Str] :: "gzip header is invalid" :: call
    if (bytes)[2] != 8:
        return Result.Err[Array[Int], Str] :: "gzip compression method is unsupported" :: call
    let flags = (bytes)[3]
    let mut cursor = 10
    if arcana_text.font_leaf.gzip_flag_set :: flags, 4 :: call:
        if cursor + 2 > (bytes :: :: len):
            return Result.Err[Array[Int], Str] :: "gzip extra header is truncated" :: call
        let extra_length = u16_le :: bytes, cursor :: call
        cursor += 2 + extra_length
    if arcana_text.font_leaf.gzip_flag_set :: flags, 8 :: call:
        while cursor < (bytes :: :: len) and (bytes)[cursor] != 0:
            cursor += 1
        cursor += 1
    if arcana_text.font_leaf.gzip_flag_set :: flags, 16 :: call:
        while cursor < (bytes :: :: len) and (bytes)[cursor] != 0:
            cursor += 1
        cursor += 1
    if arcana_text.font_leaf.gzip_flag_set :: flags, 2 :: call:
        cursor += 2
    if cursor >= (bytes :: :: len) - 8:
        return Result.Err[Array[Int], Str] :: "gzip deflate payload is truncated" :: call
    return arcana_text.font_leaf.inflate_deflate_payload :: bytes, cursor, ((bytes :: :: len) - 8) :: call

fn append_byte_range(edit out: List[Int], read payload: (Array[Int], Int, Int)):
    let bytes = payload.0
    let start = payload.1
    let end = payload.2
    let mut index = start
    while index < end and index < (bytes :: :: len):
        out :: (bytes)[index] :: push
        index += 1

fn png_paeth(a: Int, b: Int, c: Int) -> Int:
    let prediction = a + b - c
    let distance_a = abs_int :: (prediction - a) :: call
    let distance_b = abs_int :: (prediction - b) :: call
    let distance_c = abs_int :: (prediction - c) :: call
    if distance_a <= distance_b and distance_a <= distance_c:
        return a
    if distance_b <= distance_c:
        return b
    return c

fn png_unfilter_rows(read payload: (Array[Int], Int, Int, Int, Int)) -> Result[Array[Int], Str]:
    let bytes = payload.0
    let width = payload.1
    let height = payload.2
    let bytes_per_pixel = payload.3
    let row_bytes = payload.4
    let required = height * (row_bytes + 1)
    if (bytes :: :: len) < required:
        return Result.Err[Array[Int], Str] :: "PNG filtered payload is truncated" :: call
    let mut out = std.kernel.collections.array_new[Int] :: (width * height * bytes_per_pixel), 0 :: call
    let mut y = 0
    while y < height:
        let row_start = y * (row_bytes + 1)
        let filter = (bytes)[row_start]
        let source_start = row_start + 1
        let dest_start = y * row_bytes
        let mut x = 0
        while x < row_bytes:
            let raw = (bytes)[source_start + x]
            let left = match x >= bytes_per_pixel:
                true => (out)[dest_start + x - bytes_per_pixel]
                false => 0
            let up = match y > 0:
                true => (out)[dest_start + x - row_bytes]
                false => 0
            let up_left = match y > 0 and x >= bytes_per_pixel:
                true => (out)[dest_start + x - row_bytes - bytes_per_pixel]
                false => 0
            let recon = match filter:
                0 => raw
                1 => positive_mod :: (raw + left), 256 :: call
                2 => positive_mod :: (raw + up), 256 :: call
                3 => positive_mod :: (raw + ((left + up) / 2)), 256 :: call
                4 => positive_mod :: (raw + (arcana_text.font_leaf.png_paeth :: left, up, up_left :: call)), 256 :: call
                _ => -1
            if recon < 0:
                return Result.Err[Array[Int], Str] :: "PNG filter is unsupported" :: call
            out[dest_start + x] = recon
            x += 1
        y += 1
    return Result.Ok[Array[Int], Str] :: out :: call

fn decode_png_color_image(read bytes: Array[Int]) -> Result[arcana_text.font_leaf.DecodedColorImage, Str]:
    if (bytes :: :: len) < 33:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG image is truncated" :: call
    if (bytes)[0] != 137 or (bytes)[1] != 80 or (bytes)[2] != 78 or (bytes)[3] != 71 or (bytes)[4] != 13 or (bytes)[5] != 10 or (bytes)[6] != 26 or (bytes)[7] != 10:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG signature is invalid" :: call
    let mut width = 0
    let mut height = 0
    let mut bit_depth = 0
    let mut color_type = 0
    let mut palette = empty_alpha :: :: call
    let mut transparency = empty_alpha :: :: call
    let mut compressed = empty_int_list :: :: call
    let mut cursor = 8
    let total = bytes :: :: len
    while cursor + 8 <= total:
        let chunk_length = u32_be :: bytes, cursor :: call
        let chunk_type = tag_at :: bytes, cursor + 4 :: call
        let data_start = cursor + 8
        let data_end = data_start + chunk_length
        if data_end + 4 > total:
            return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: ("PNG chunk `" + chunk_type + "` is truncated") :: call
        if chunk_type == "IHDR":
            if chunk_length < 13:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG IHDR chunk is truncated" :: call
            width = u32_be :: bytes, data_start :: call
            height = u32_be :: bytes, data_start + 4 :: call
            bit_depth = byte_at_or_zero :: bytes, data_start + 8 :: call
            color_type = byte_at_or_zero :: bytes, data_start + 9 :: call
            let compression = byte_at_or_zero :: bytes, data_start + 10 :: call
            let filter_method = byte_at_or_zero :: bytes, data_start + 11 :: call
            let interlace = byte_at_or_zero :: bytes, data_start + 12 :: call
            if compression != 0 or filter_method != 0:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG compression or filter method is unsupported" :: call
            if interlace != 0:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG interlacing is unsupported" :: call
        else:
            if chunk_type == "PLTE":
                palette = std.bytes.slice :: bytes, data_start, data_end :: call
            else:
                if chunk_type == "tRNS":
                    transparency = std.bytes.slice :: bytes, data_start, data_end :: call
                else:
                    if chunk_type == "IDAT":
                        arcana_text.font_leaf.append_byte_range :: compressed, bytes, data_start, data_end :: call
                    else:
                        if chunk_type == "IEND":
                            cursor = total
                            break
        cursor = data_end + 4
    if width <= 0 or height <= 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG dimensions are invalid" :: call
    if bit_depth != 8:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG bit depth is unsupported" :: call
    let channels = match color_type:
        0 => 1
        2 => 3
        3 => 1
        4 => 2
        6 => 4
        _ => 0
    if channels <= 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG color type is unsupported" :: call
    let row_bytes = width * channels
    let inflated = arcana_text.font_leaf.inflate_zlib_payload :: (std.collections.array.from_list[Int] :: compressed :: call) :: call
    if inflated :: :: is_err:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: (result_err_or :: inflated, "failed to inflate PNG payload" :: call) :: call
    let rows = arcana_text.font_leaf.png_unfilter_rows :: ((inflated :: (empty_alpha :: :: call) :: unwrap_or), width, height, channels, row_bytes) :: call
    if rows :: :: is_err:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: (result_err_or :: rows, "failed to unfilter PNG payload" :: call) :: call
    let decoded_rows = rows :: (empty_alpha :: :: call) :: unwrap_or
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut index = 0
    while index < (width * height):
        let source = index * channels
        let dest = index * 4
        if color_type == 6:
            rgba[dest] = (decoded_rows)[source]
            rgba[dest + 1] = (decoded_rows)[source + 1]
            rgba[dest + 2] = (decoded_rows)[source + 2]
            rgba[dest + 3] = (decoded_rows)[source + 3]
        else:
            if color_type == 2:
                rgba[dest] = (decoded_rows)[source]
                rgba[dest + 1] = (decoded_rows)[source + 1]
                rgba[dest + 2] = (decoded_rows)[source + 2]
                rgba[dest + 3] = 255
            else:
                if color_type == 3:
                    let palette_index = (decoded_rows)[source]
                    let entry = palette_index * 3
                    if entry + 2 >= (palette :: :: len):
                        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "PNG palette index is out of range" :: call
                    rgba[dest] = (palette)[entry]
                    rgba[dest + 1] = (palette)[entry + 1]
                    rgba[dest + 2] = (palette)[entry + 2]
                    rgba[dest + 3] = match palette_index < (transparency :: :: len):
                        true => (transparency)[palette_index]
                        false => 255
                else:
                    if color_type == 0:
                        let gray = (decoded_rows)[source]
                        rgba[dest] = gray
                        rgba[dest + 1] = gray
                        rgba[dest + 2] = gray
                        rgba[dest + 3] = 255
                    else:
                        let gray = (decoded_rows)[source]
                        rgba[dest] = gray
                        rgba[dest + 1] = gray
                        rgba[dest + 2] = gray
                        rgba[dest + 3] = (decoded_rows)[source + 1]
        index += 1
    return Result.Ok[arcana_text.font_leaf.DecodedColorImage, Str] :: (arcana_text.font_leaf.DecodedColorImage :: size = (width, height), rgba = rgba :: call) :: call

fn decode_bmp_color_image(read bytes: Array[Int]) -> Result[arcana_text.font_leaf.DecodedColorImage, Str]:
    if (bytes :: :: len) < 54:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "BMP image is truncated" :: call
    if (byte_at_or_zero :: bytes, 0 :: call) != 66 or (byte_at_or_zero :: bytes, 1 :: call) != 77:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "BMP image header is invalid" :: call
    let data_offset = u32_le :: bytes, 10 :: call
    let dib_size = u32_le :: bytes, 14 :: call
    if dib_size < 40:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "BMP DIB header is unsupported" :: call
    let width_raw = i32_le :: bytes, 18 :: call
    let height_raw = i32_le :: bytes, 22 :: call
    let planes = u16_le :: bytes, 26 :: call
    let bits_per_pixel = u16_le :: bytes, 28 :: call
    let compression = u32_le :: bytes, 30 :: call
    if planes != 1 or compression != 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "BMP compression or plane count is unsupported" :: call
    if bits_per_pixel != 24 and bits_per_pixel != 32:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "BMP bit depth is unsupported" :: call
    let width = abs_int :: width_raw :: call
    let height = abs_int :: height_raw :: call
    if width <= 0 or height <= 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "BMP dimensions are invalid" :: call
    let bytes_per_pixel = bits_per_pixel / 8
    let row_size = (((bits_per_pixel * width) + 31) / 32) * 4
    if data_offset < 0 or data_offset + (row_size * height) > (bytes :: :: len):
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "BMP pixel data is truncated" :: call
    let top_down = height_raw < 0
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut y = 0
    while y < height:
        let src_y = match top_down:
            true => y
            false => height - 1 - y
        let row_start = data_offset + (src_y * row_size)
        let mut x = 0
        while x < width:
            let source = row_start + (x * bytes_per_pixel)
            let dest = ((y * width) + x) * 4
            let blue = byte_at_or_zero :: bytes, source :: call
            let green = byte_at_or_zero :: bytes, source + 1 :: call
            let red = byte_at_or_zero :: bytes, source + 2 :: call
            let alpha = match bits_per_pixel:
                32 => byte_at_or_zero :: bytes, source + 3 :: call
                _ => 255
            rgba[dest] = red
            rgba[dest + 1] = green
            rgba[dest + 2] = blue
            rgba[dest + 3] = alpha
            x += 1
        y += 1
    return Result.Ok[arcana_text.font_leaf.DecodedColorImage, Str] :: (arcana_text.font_leaf.DecodedColorImage :: size = (width, height), rgba = rgba :: call) :: call

fn normalized_embedded_bitmap_tag(read tag: Str) -> Str:
    return match tag:
        "jpeg" => "jpg "
        "tif " => "tiff"
        _ => tag

fn bitmap_signature_kind(read bytes: Array[Int]) -> Str:
    if (bytes :: :: len) >= 8 and (bytes)[0] == 137 and (bytes)[1] == 80 and (bytes)[2] == 78 and (bytes)[3] == 71 and (bytes)[4] == 13 and (bytes)[5] == 10 and (bytes)[6] == 26 and (bytes)[7] == 10:
        return "png "
    if (bytes :: :: len) >= 2 and (bytes)[0] == 66 and (bytes)[1] == 77:
        return "bmp "
    if (bytes :: :: len) >= 4 and (bytes)[0] == 0 and (bytes)[1] == 0 and (bytes)[2] == 1 and (bytes)[3] == 0:
        return "ico "
    if (bytes :: :: len) >= 3 and (bytes)[0] == 255 and (bytes)[1] == 216 and (bytes)[2] == 255:
        return "jpg "
    if (bytes :: :: len) >= 6:
        let g = tag_at :: bytes, 0 :: call
        if g == "GIF8":
            return "gif "
    if (bytes :: :: len) >= 4:
        let little = (bytes)[0] == 73 and (bytes)[1] == 73 and (bytes)[2] == 42 and (bytes)[3] == 0
        let big = (bytes)[0] == 77 and (bytes)[1] == 77 and (bytes)[2] == 0 and (bytes)[3] == 42
        if little or big:
            return "tiff"
    return ""

fn decode_ico_color_image(read bytes: Array[Int]) -> Result[arcana_text.font_leaf.DecodedColorImage, Str]:
    if (bytes :: :: len) < 22:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO image is truncated" :: call
    if u16_le :: bytes, 0 :: call != 0 or u16_le :: bytes, 2 :: call != 1:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO image header is invalid" :: call
    let count = u16_le :: bytes, 4 :: call
    if count <= 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO image has no entries" :: call
    let mut best_offset = -1
    let mut best_length = 0
    let mut best_area = -1
    let mut best_depth = -1
    let mut index = 0
    while index < count:
        let record = 6 + (index * 16)
        if record + 16 > (bytes :: :: len):
            return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO directory entry is truncated" :: call
        let width = match byte_at_or_zero :: bytes, record :: call:
            0 => 256
            value => value
        let height = match byte_at_or_zero :: bytes, record + 1 :: call:
            0 => 256
            value => value
        let bit_count = u16_le :: bytes, record + 6 :: call
        let image_length = u32_le :: bytes, record + 8 :: call
        let image_offset = u32_le :: bytes, record + 12 :: call
        if image_length > 0 and image_offset >= 0 and image_offset + image_length <= (bytes :: :: len):
            let image_bytes = std.bytes.slice :: bytes, image_offset, image_offset + image_length :: call
            if (arcana_text.font_leaf.bitmap_signature_kind :: image_bytes :: call) == "png ":
                let area = width * height
                if best_offset < 0 or area > best_area or (area == best_area and bit_count > best_depth):
                    best_offset = image_offset
                    best_length = image_length
                    best_area = area
                    best_depth = bit_count
        index += 1
    if best_offset < 0 or best_length <= 0:
        let mut dib_offset = -1
        let mut dib_length = 0
        let mut dib_area = -1
        let mut dib_depth = -1
        index = 0
        while index < count:
            let record = 6 + (index * 16)
            if record + 16 > (bytes :: :: len):
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO directory entry is truncated" :: call
            let width = match byte_at_or_zero :: bytes, record :: call:
                0 => 256
                value => value
            let height = match byte_at_or_zero :: bytes, record + 1 :: call:
                0 => 256
                value => value
            let bit_count = u16_le :: bytes, record + 6 :: call
            let image_length = u32_le :: bytes, record + 8 :: call
            let image_offset = u32_le :: bytes, record + 12 :: call
            if image_length > 0 and image_offset >= 0 and image_offset + image_length <= (bytes :: :: len):
                let image_bytes = std.bytes.slice :: bytes, image_offset, image_offset + image_length :: call
                let header_size = u32_le :: image_bytes, 0 :: call
                if header_size >= 40:
                    let area = width * height
                    if dib_offset < 0 or area > dib_area or (area == dib_area and bit_count > dib_depth):
                        dib_offset = image_offset
                        dib_length = image_length
                        dib_area = area
                        dib_depth = bit_count
            index += 1
        if dib_offset < 0 or dib_length <= 0:
            return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO image has no supported bitmap payload" :: call
        return arcana_text.font_leaf.decode_ico_dib_color_image :: (std.bytes.slice :: bytes, dib_offset, dib_offset + dib_length :: call) :: call
    return arcana_text.font_leaf.decode_png_color_image :: (std.bytes.slice :: bytes, best_offset, best_offset + best_length :: call) :: call

fn gif_read_code(read bytes: Array[Int], bit_offset: Int, code_size: Int) -> Result[(Int, Int), Str]:
    if code_size <= 0 or code_size > 12:
        return Result.Err[(Int, Int), Str] :: "GIF LZW code size is invalid" :: call
    let total_bits = (bytes :: :: len) * 8
    if bit_offset < 0 or bit_offset + code_size > total_bits:
        return Result.Err[(Int, Int), Str] :: "GIF LZW stream is truncated" :: call
    let mut value = 0
    let mut shift = 0
    let mut cursor = bit_offset
    while shift < code_size:
        let byte_index = cursor / 8
        let bit_index = cursor % 8
        let available = min_int :: (code_size - shift), (8 - bit_index) :: call
        let scale = arcana_text.font_leaf.inflate_pow2 :: bit_index :: call
        let width = arcana_text.font_leaf.inflate_pow2 :: available :: call
        let chunk = positive_mod :: ((byte_at_or_zero :: bytes, byte_index :: call) / scale), width :: call
        value += chunk * (arcana_text.font_leaf.inflate_pow2 :: shift :: call)
        cursor += available
        shift += available
    return Result.Ok[(Int, Int), Str] :: (value, cursor) :: call

fn gif_collect_subblocks(read bytes: Array[Int], start: Int) -> Result[(Array[Int], Int), Str]:
    let mut out = empty_int_list :: :: call
    let mut cursor = start
    let total = bytes :: :: len
    while cursor < total:
        let block_length = byte_at_or_zero :: bytes, cursor :: call
        cursor += 1
        if block_length <= 0:
            return Result.Ok[(Array[Int], Int), Str] :: ((std.collections.array.from_list[Int] :: out :: call), cursor) :: call
        if cursor + block_length > total:
            return Result.Err[(Array[Int], Int), Str] :: "GIF sub-block is truncated" :: call
        append_byte_range :: out, (bytes, cursor, cursor + block_length) :: call
        cursor += block_length
    return Result.Err[(Array[Int], Int), Str] :: "GIF sub-block stream is truncated" :: call

fn gif_append_code_bytes(edit out: List[Int], read payload: (Array[Int], Array[Int], Int, Int)) -> Result[Int, Str]:
    let prefix = payload.0
    let suffix = payload.1
    let code = payload.2
    let clear_code = payload.3
    let mut stack = empty_int_list :: :: call
    let mut current = code
    while current >= clear_code:
        if current < 0 or current >= (suffix :: :: len):
            return Result.Err[Int, Str] :: "GIF LZW code is out of range" :: call
        stack :: (suffix)[current] :: push
        current = (prefix)[current]
    if current < 0 or current > 255:
        return Result.Err[Int, Str] :: "GIF LZW base symbol is invalid" :: call
    let first = current
    stack :: first :: push
    while not (stack :: :: is_empty):
        out :: (stack :: :: pop) :: push
    return Result.Ok[Int, Str] :: first :: call

fn decode_gif_lzw_indices(read bytes: Array[Int], min_code_size: Int, expected_count: Int) -> Result[Array[Int], Str]:
    if min_code_size < 2 or min_code_size > 8:
        return Result.Err[Array[Int], Str] :: "GIF minimum code size is unsupported" :: call
    let clear_code = arcana_text.font_leaf.inflate_pow2 :: min_code_size :: call
    let end_code = clear_code + 1
    let mut prefix = std.kernel.collections.array_new[Int] :: 4096, -1 :: call
    let mut suffix = std.kernel.collections.array_new[Int] :: 4096, 0 :: call
    let mut index = 0
    while index < clear_code:
        suffix[index] = index
        index += 1
    let mut out = empty_int_list :: :: call
    let mut bit_offset = 0
    let mut code_size = min_code_size + 1
    let mut available = end_code + 1
    let mut old_code = -1
    let mut done = false
    while not done:
        let read_code = arcana_text.font_leaf.gif_read_code :: bytes, bit_offset, code_size :: call
        if read_code :: :: is_err:
            return Result.Err[Array[Int], Str] :: (result_err_or :: read_code, "failed to read GIF LZW code" :: call) :: call
        let payload = read_code :: ((0, 0)) :: unwrap_or
        let code = payload.0
        bit_offset = payload.1
        if code == clear_code:
            code_size = min_code_size + 1
            available = end_code + 1
            old_code = -1
        else:
            if code == end_code:
                done = true
            else:
                if old_code < 0:
                    let first = arcana_text.font_leaf.gif_append_code_bytes :: out, (prefix, suffix, code, clear_code) :: call
                    if first :: :: is_err:
                        return Result.Err[Array[Int], Str] :: (result_err_or :: first, "failed to expand initial GIF LZW code" :: call) :: call
                    old_code = code
                else:
                    let mut first = 0
                    if code < available:
                        let expanded = arcana_text.font_leaf.gif_append_code_bytes :: out, (prefix, suffix, code, clear_code) :: call
                        if expanded :: :: is_err:
                            return Result.Err[Array[Int], Str] :: (result_err_or :: expanded, "failed to expand GIF LZW code" :: call) :: call
                        first = expanded :: 0 :: unwrap_or
                    else:
                        if code != available:
                            return Result.Err[Array[Int], Str] :: "GIF LZW code references an invalid dictionary slot" :: call
                        let expanded = arcana_text.font_leaf.gif_append_code_bytes :: out, (prefix, suffix, old_code, clear_code) :: call
                        if expanded :: :: is_err:
                            return Result.Err[Array[Int], Str] :: (result_err_or :: expanded, "failed to expand repeated GIF LZW code" :: call) :: call
                        first = expanded :: 0 :: unwrap_or
                        out :: first :: push
                    if available < 4096:
                        prefix[available] = old_code
                        suffix[available] = first
                        available += 1
                        if available >= (arcana_text.font_leaf.inflate_pow2 :: code_size :: call) and code_size < 12:
                            code_size += 1
                    old_code = code
        if expected_count > 0 and (out :: :: len) >= expected_count:
            done = true
    if expected_count > 0 and (out :: :: len) < expected_count:
        return Result.Err[Array[Int], Str] :: "GIF pixel stream is truncated" :: call
    return Result.Ok[Array[Int], Str] :: (std.collections.array.from_list[Int] :: out :: call) :: call

fn gif_row_order(height: Int, interlaced: Bool) -> Array[Int]:
    let mut out = std.kernel.collections.array_new[Int] :: height, 0 :: call
    if not interlaced:
        let mut row = 0
        while row < height:
            out[row] = row
            row += 1
        return out
    let mut index = 0
    let mut row = 0
    while row < height:
        out[index] = row
        index += 1
        row += 8
    row = 4
    while row < height:
        out[index] = row
        index += 1
        row += 8
    row = 2
    while row < height:
        out[index] = row
        index += 1
        row += 4
    row = 1
    while row < height:
        out[index] = row
        index += 1
        row += 2
    return out

fn decode_gif_color_image(read bytes: Array[Int]) -> Result[arcana_text.font_leaf.DecodedColorImage, Str]:
    if (bytes :: :: len) < 13:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF image is truncated" :: call
    let header = tag_at :: bytes, 0 :: call
    if header != "GIF8":
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF header is invalid" :: call
    let width = u16_le :: bytes, 6 :: call
    let height = u16_le :: bytes, 8 :: call
    if width <= 0 or height <= 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF dimensions are invalid" :: call
    let packed = byte_at_or_zero :: bytes, 10 :: call
    let mut cursor = 13
    let total = bytes :: :: len
    let global_size = (arcana_text.font_leaf.inflate_pow2 :: ((packed % 8) + 1) :: call) * 3
    let global_palette_flag = ((packed / 128) % 2) == 1
    let mut global_palette = empty_alpha :: :: call
    if global_palette_flag:
        if cursor + global_size > total:
            return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF global palette is truncated" :: call
        global_palette = std.bytes.slice :: bytes, cursor, cursor + global_size :: call
        cursor += global_size
    let mut transparency_index = -1
    while cursor < total:
        let marker = byte_at_or_zero :: bytes, cursor :: call
        if marker == 59:
            return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF image has no frame" :: call
        if marker == 33:
            if cursor + 2 > total:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF extension header is truncated" :: call
            let label = byte_at_or_zero :: bytes, cursor + 1 :: call
            cursor += 2
            if label == 249:
                let block_size = byte_at_or_zero :: bytes, cursor :: call
                if block_size < 4 or cursor + 1 + block_size > total:
                    return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF graphics control extension is truncated" :: call
                let gce_packed = byte_at_or_zero :: bytes, cursor + 1 :: call
                transparency_index = match (gce_packed % 2) == 1:
                    true => byte_at_or_zero :: bytes, cursor + 4 :: call
                    false => -1
                cursor += 1 + block_size
                if cursor < total and (byte_at_or_zero :: bytes, cursor :: call) == 0:
                    cursor += 1
            else:
                let skipped = arcana_text.font_leaf.gif_collect_subblocks :: bytes, cursor :: call
                if skipped :: :: is_err:
                    return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: (result_err_or :: skipped, "failed to skip GIF extension blocks" :: call) :: call
                cursor = (skipped :: ((empty_alpha :: :: call), cursor) :: unwrap_or).1
        else:
            if marker != 44:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF block marker is unsupported" :: call
            if cursor + 10 > total:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF image descriptor is truncated" :: call
            let left = u16_le :: bytes, cursor + 1 :: call
            let top = u16_le :: bytes, cursor + 3 :: call
            let image_width = u16_le :: bytes, cursor + 5 :: call
            let image_height = u16_le :: bytes, cursor + 7 :: call
            let image_packed = byte_at_or_zero :: bytes, cursor + 9 :: call
            cursor += 10
            let local_palette_flag = ((image_packed / 128) % 2) == 1
            let interlaced = ((image_packed / 64) % 2) == 1
            let local_size = (arcana_text.font_leaf.inflate_pow2 :: ((image_packed % 8) + 1) :: call) * 3
            let mut palette = global_palette
            if local_palette_flag:
                if cursor + local_size > total:
                    return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF local palette is truncated" :: call
                palette = std.bytes.slice :: bytes, cursor, cursor + local_size :: call
                cursor += local_size
            if cursor >= total:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF image data is truncated" :: call
            let min_code_size = byte_at_or_zero :: bytes, cursor :: call
            cursor += 1
            let blocks = arcana_text.font_leaf.gif_collect_subblocks :: bytes, cursor :: call
            if blocks :: :: is_err:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: (result_err_or :: blocks, "failed to read GIF image sub-blocks" :: call) :: call
            let block_payload = blocks :: ((empty_alpha :: :: call), cursor) :: unwrap_or
            let compressed = block_payload.0
            cursor = block_payload.1
            let decoded = arcana_text.font_leaf.decode_gif_lzw_indices :: compressed, min_code_size, (image_width * image_height) :: call
            if decoded :: :: is_err:
                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: (result_err_or :: decoded, "failed to decode GIF image data" :: call) :: call
            let indices = decoded :: (empty_alpha :: :: call) :: unwrap_or
            let row_order = arcana_text.font_leaf.gif_row_order :: image_height, interlaced :: call
            let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
            let mut source = 0
            let mut row = 0
            while row < image_height:
                let target_row = (row_order)[row]
                let dest_y = top + target_row
                let mut x = 0
                while x < image_width and source < (indices :: :: len):
                    let dest_x = left + x
                    let palette_index = (indices)[source]
                    if dest_x >= 0 and dest_x < width and dest_y >= 0 and dest_y < height:
                        let dest = ((dest_y * width) + dest_x) * 4
                        if palette_index == transparency_index:
                            rgba[dest + 3] = 0
                        else:
                            let entry = palette_index * 3
                            if entry + 2 >= (palette :: :: len):
                                return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF palette index is out of range" :: call
                            rgba[dest] = (palette)[entry]
                            rgba[dest + 1] = (palette)[entry + 1]
                            rgba[dest + 2] = (palette)[entry + 2]
                            rgba[dest + 3] = 255
                    source += 1
                    x += 1
                row += 1
            return Result.Ok[arcana_text.font_leaf.DecodedColorImage, Str] :: (arcana_text.font_leaf.DecodedColorImage :: size = (width, height), rgba = rgba :: call) :: call
    return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "GIF image has no frame" :: call

fn decode_ico_dib_color_image(read bytes: Array[Int]) -> Result[arcana_text.font_leaf.DecodedColorImage, Str]:
    if (bytes :: :: len) < 40:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO DIB payload is truncated" :: call
    let dib_size = u32_le :: bytes, 0 :: call
    if dib_size < 40 or dib_size > (bytes :: :: len):
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO DIB header is unsupported" :: call
    let width_raw = i32_le :: bytes, 4 :: call
    let height_raw = i32_le :: bytes, 8 :: call
    let planes = u16_le :: bytes, 12 :: call
    let bits_per_pixel = u16_le :: bytes, 14 :: call
    let compression = u32_le :: bytes, 16 :: call
    if planes != 1 or compression != 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO DIB compression or plane count is unsupported" :: call
    if bits_per_pixel != 24 and bits_per_pixel != 32:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO DIB bit depth is unsupported" :: call
    let width = abs_int :: width_raw :: call
    let height_full = abs_int :: height_raw :: call
    let height = match height_full > 1:
        true => max_int :: (height_full / 2), 1 :: call
        false => height_full
    if width <= 0 or height <= 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO DIB dimensions are invalid" :: call
    let bytes_per_pixel = bits_per_pixel / 8
    let row_size = (((bits_per_pixel * width) + 31) / 32) * 4
    let data_offset = dib_size
    let xor_length = row_size * height
    if data_offset < 0 or data_offset + xor_length > (bytes :: :: len):
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "ICO DIB pixel data is truncated" :: call
    let mask_row_size = (((width) + 31) / 32) * 4
    let mask_offset = data_offset + xor_length
    let mask_available = mask_offset + (mask_row_size * height) <= (bytes :: :: len)
    let top_down = height_raw < 0
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut saw_alpha = false
    let mut y = 0
    while y < height:
        let src_y = match top_down:
            true => y
            false => height - 1 - y
        let row_start = data_offset + (src_y * row_size)
        let mut x = 0
        while x < width:
            let source = row_start + (x * bytes_per_pixel)
            let dest = ((y * width) + x) * 4
            let blue = byte_at_or_zero :: bytes, source :: call
            let green = byte_at_or_zero :: bytes, source + 1 :: call
            let red = byte_at_or_zero :: bytes, source + 2 :: call
            let alpha = match bits_per_pixel:
                32 => byte_at_or_zero :: bytes, source + 3 :: call
                _ => 255
            if alpha > 0:
                saw_alpha = true
            rgba[dest] = red
            rgba[dest + 1] = green
            rgba[dest + 2] = blue
            rgba[dest + 3] = alpha
            x += 1
        y += 1
    if mask_available and (bits_per_pixel < 32 or not saw_alpha):
        y = 0
        while y < height:
            let mask_y = match top_down:
                true => y
                false => height - 1 - y
            let row_start = mask_offset + (mask_y * mask_row_size)
            let mut x = 0
            while x < width:
                let value = byte_at_or_zero :: bytes, row_start + (x / 8) :: call
                let bit = positive_mod :: (value / (arcana_text.font_leaf.inflate_pow2 :: (7 - (x % 8)) :: call)), 2 :: call
                let dest = ((y * width) + x) * 4
                if bit == 1:
                    rgba[dest + 3] = 0
                else:
                    if bits_per_pixel < 32 or not saw_alpha:
                        rgba[dest + 3] = 255
                x += 1
            y += 1
    return Result.Ok[arcana_text.font_leaf.DecodedColorImage, Str] :: (arcana_text.font_leaf.DecodedColorImage :: size = (width, height), rgba = rgba :: call) :: call

fn embedded_bitmap_kind(read image: arcana_text.font_leaf.EmbeddedBitmapImage) -> Str:
    let signature = arcana_text.font_leaf.bitmap_signature_kind :: image.bytes :: call
    if signature != "":
        return signature
    return arcana_text.font_leaf.normalized_embedded_bitmap_tag :: image.format_tag :: call

fn decode_external_color_image(read image: arcana_text.font_leaf.EmbeddedBitmapImage) -> Result[arcana_text.font_leaf.DecodedColorImage, Str]:
    let kind = arcana_text.font_leaf.embedded_bitmap_kind :: image :: call
    if kind == "bmp ":
        return arcana_text.font_leaf.decode_bmp_color_image :: image.bytes :: call
    if kind == "png ":
        return arcana_text.font_leaf.decode_png_color_image :: image.bytes :: call
    if kind == "ico ":
        return arcana_text.font_leaf.decode_ico_color_image :: image.bytes :: call
    if kind == "gif ":
        return arcana_text.font_leaf.decode_gif_color_image :: image.bytes :: call
    if (image.bytes :: :: len) <= 0:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "embedded bitmap payload is empty" :: call
    return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: ("unsupported embedded bitmap format `" + kind + "`") :: call

fn embedded_bitmap_to_glyph_bitmap(read payload: (arcana_text.font_leaf.EmbeddedBitmapImage, arcana_text.font_leaf.DecodedColorImage, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let image = payload.0
    let decoded = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let mut offset = image.metrics.offset
    if image.bottom_origin:
        offset = (offset.0, offset.1 - decoded.size.1)
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = decoded.size, offset = offset, advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = arcana_text.font_leaf.rgba_alpha :: decoded.rgba :: call
    out.lcd = empty_lcd :: :: call
    out.rgba = decoded.rgba
    return out

fn svg_document_bytes_for_glyph(read face: arcana_text.font_leaf.FontFaceState, glyph_index: Int) -> Result[Array[Int], Str]:
    if face.svg_offset < 0 or face.svg_length < 10:
        return Result.Err[Array[Int], Str] :: "SVG glyph document not available" :: call
    let total = face.font_view :: :: len
    if face.svg_offset + 10 > total:
        return Result.Err[Array[Int], Str] :: "SVG glyph table is out of range" :: call
    let document_index_offset = face.svg_offset + (u32_be_ref :: face.font_view, face.svg_offset + 2 :: call)
    if document_index_offset + 2 > total:
        return Result.Err[Array[Int], Str] :: "SVG glyph document index is out of range" :: call
    let entry_count = u16_be_ref :: face.font_view, document_index_offset :: call
    let mut index = 0
    while index < entry_count:
        let record = document_index_offset + 2 + (index * 12)
        if record + 12 > total:
            return Result.Err[Array[Int], Str] :: "SVG glyph document record is out of range" :: call
        let first_glyph = u16_be_ref :: face.font_view, record :: call
        let last_glyph = u16_be_ref :: face.font_view, record + 2 :: call
        if glyph_index >= first_glyph and glyph_index <= last_glyph:
            let svg_offset = face.svg_offset + (u32_be_ref :: face.font_view, record + 4 :: call)
            let svg_length = u32_be_ref :: face.font_view, record + 8 :: call
            if svg_offset < 0 or svg_length <= 0 or svg_offset + svg_length > total:
                return Result.Err[Array[Int], Str] :: "SVG glyph document payload is invalid" :: call
            return Result.Ok[Array[Int], Str] :: ((face.font_view :: svg_offset, svg_offset + svg_length :: subview) :: :: to_array) :: call
        index += 1
    return Result.Err[Array[Int], Str] :: "SVG glyph document was not found" :: call

fn empty_svg_number_read() -> arcana_text.font_leaf.SvgNumberRead:
    let mut out = arcana_text.font_leaf.SvgNumberRead :: value = 0, next = 0, percent = false :: call
    out.ok = false
    return out

fn svg_view_box_value(read payload: (Int, Int, Int, Int)) -> arcana_text.font_leaf.SvgViewBox:
    let min_x = payload.0
    let min_y = payload.1
    let width = payload.2
    let height = payload.3
    let mut out = arcana_text.font_leaf.SvgViewBox :: min_x = min_x, min_y = min_y, width = width :: call
    out.height = height
    return out

fn svg_is_separator_byte(value: Int) -> Bool:
    return value == 32 or value == 9 or value == 10 or value == 13 or value == 44

fn svg_skip_number_separators(read text: Str, offset: Int) -> Int:
    let total = std.text.len_bytes :: text :: call
    let mut cursor = offset
    while cursor < total and (arcana_text.font_leaf.svg_is_separator_byte :: (std.text.byte_at :: text, cursor :: call) :: call):
        cursor += 1
    return cursor

fn svg_decimal_fixed(read text: Str) -> arcana_text.font_leaf.SvgNumberRead:
    let total = std.text.len_bytes :: text :: call
    let mut out = arcana_text.font_leaf.empty_svg_number_read :: :: call
    out.next = 0
    if total <= 0:
        return out
    let mut cursor = 0
    let mut sign = 1
    let first = std.text.byte_at :: text, cursor :: call
    if first == 45:
        sign = -1
        cursor += 1
    else:
        if first == 43:
            cursor += 1
    let mut whole = 0
    let mut saw_digit = false
    while cursor < total:
        let value = std.text.byte_at :: text, cursor :: call
        if value < 48 or value > 57:
            break
        whole = whole * 10 + (value - 48)
        saw_digit = true
        cursor += 1
    let mut fraction = 0
    let mut divisor = 1
    if cursor < total and (std.text.byte_at :: text, cursor :: call) == 46:
        cursor += 1
        while cursor < total:
            let value = std.text.byte_at :: text, cursor :: call
            if value < 48 or value > 57:
                break
            fraction = fraction * 10 + (value - 48)
            divisor *= 10
            saw_digit = true
            cursor += 1
    if not saw_digit:
        out.next = cursor
        return out
    let mut fixed = whole * 65536
    if divisor > 1:
        fixed += (fraction * 65536) / divisor
    if sign < 0:
        fixed = 0 - fixed
    let mut percent = false
    if cursor < total and (std.text.byte_at :: text, cursor :: call) == 37:
        percent = true
        cursor += 1
    out.value = fixed
    out.next = cursor
    out.percent = percent
    out.ok = true
    return out

fn svg_number_read(read text: Str, offset: Int) -> arcana_text.font_leaf.SvgNumberRead:
    let cursor = arcana_text.font_leaf.svg_skip_number_separators :: text, offset :: call
    if cursor >= (std.text.len_bytes :: text :: call):
        let mut out = arcana_text.font_leaf.empty_svg_number_read :: :: call
        out.next = cursor
        return out
    let remainder = std.text.slice_bytes :: text, cursor, (std.text.len_bytes :: text :: call) :: call
    let parsed = arcana_text.font_leaf.svg_decimal_fixed :: remainder :: call
    if not parsed.ok:
        let mut out = parsed
        out.next = cursor
        return out
    let mut out = parsed
    out.next = cursor + parsed.next
    return out

fn svg_attr_value(read tag: Str, read key: Str) -> Str:
    let double_prefix = key + "=\""
    let single_prefix = key + "='"
    let double_start = std.text.find :: tag, 0, double_prefix :: call
    if double_start >= 0:
        let start = double_start + (std.text.len_bytes :: double_prefix :: call)
        let end = std.text.find :: tag, start, "\"" :: call
        if end >= start:
            return std.text.slice_bytes :: tag, start, end :: call
    let single_start = std.text.find :: tag, 0, single_prefix :: call
    if single_start >= 0:
        let start = single_start + (std.text.len_bytes :: single_prefix :: call)
        let end = std.text.find :: tag, start, "'" :: call
        if end >= start:
            return std.text.slice_bytes :: tag, start, end :: call
    return ""

fn svg_style_value(read style: Str, read key: Str) -> Str:
    for part in (std.text.split :: style, ";" :: call):
        let trimmed = std.text.trim :: part :: call
        let colon = std.text.find :: trimmed, 0, ":" :: call
        if colon < 0:
            continue
        let name = std.text.trim :: (std.text.slice_bytes :: trimmed, 0, colon :: call) :: call
        if name == key:
            return std.text.trim :: (std.text.slice_bytes :: trimmed, colon + 1, (std.text.len_bytes :: trimmed :: call) :: call) :: call
    return ""

fn svg_tag_name(read tag: Str) -> Str:
    let total = std.text.len_bytes :: tag :: call
    let mut start = 0
    while start < total and (arcana_text.font_leaf.svg_is_separator_byte :: (std.text.byte_at :: tag, start :: call) :: call):
        start += 1
    if start < total and (std.text.byte_at :: tag, start :: call) == 47:
        start += 1
    let mut end = start
    while end < total:
        let value = std.text.byte_at :: tag, end :: call
        if value == 47 or value == 62 or (arcana_text.font_leaf.svg_is_separator_byte :: value :: call):
            break
        end += 1
    return std.text.slice_bytes :: tag, start, end :: call

fn svg_hex_digit(value: Int) -> Int:
    if value >= 48 and value <= 57:
        return value - 48
    if value >= 65 and value <= 70:
        return value - 55
    if value >= 97 and value <= 102:
        return value - 87
    return -1

fn svg_document_text(read svg_bytes: Array[Int]) -> Result[Str, Str]:
    if (svg_bytes :: :: len) >= 2 and (svg_bytes)[0] == 31 and (svg_bytes)[1] == 139:
        let expanded = arcana_text.font_leaf.inflate_gzip_payload :: svg_bytes :: call
        if expanded :: :: is_err:
            return Result.Err[Str, Str] :: (result_err_or :: expanded, "failed to expand gzipped SVG glyph document" :: call) :: call
        return Result.Ok[Str, Str] :: (std.bytes.to_str_utf8 :: (expanded :: (empty_alpha :: :: call) :: unwrap_or) :: call) :: call
    return Result.Ok[Str, Str] :: (std.bytes.to_str_utf8 :: svg_bytes :: call) :: call

fn svg_parse_hex_color(read text: Str) -> (Int, Int, Int, Int):
    let raw = std.text.trim :: text :: call
    if not (std.text.starts_with :: raw, "#" :: call):
        return (0, 0, 0, -1)
    let digits = std.text.slice_bytes :: raw, 1, (std.text.len_bytes :: raw :: call) :: call
    let count = std.text.len_bytes :: digits :: call
    if count == 3:
        let r = arcana_text.font_leaf.svg_hex_digit :: (std.text.byte_at :: digits, 0 :: call) :: call
        let g = arcana_text.font_leaf.svg_hex_digit :: (std.text.byte_at :: digits, 1 :: call) :: call
        let b = arcana_text.font_leaf.svg_hex_digit :: (std.text.byte_at :: digits, 2 :: call) :: call
        if r < 0 or g < 0 or b < 0:
            return (0, 0, 0, -1)
        return (r * 17, g * 17, b * 17, 255)
    if count == 6 or count == 8:
        let mut values = std.kernel.collections.array_new[Int] :: count, 0 :: call
        let mut index = 0
        while index < count:
            let digit = arcana_text.font_leaf.svg_hex_digit :: (std.text.byte_at :: digits, index :: call) :: call
            if digit < 0:
                return (0, 0, 0, -1)
            values[index] = digit
            index += 1
        let red = values[0] * 16 + values[1]
        let green = values[2] * 16 + values[3]
        let blue = values[4] * 16 + values[5]
        let alpha = match count:
            8 => values[6] * 16 + values[7]
            _ => 255
        return (red, green, blue, alpha)
    return (0, 0, 0, -1)

fn svg_parse_rgb_color(read text: Str) -> (Int, Int, Int, Int):
    let raw = std.text.trim :: text :: call
    if not ((std.text.starts_with :: raw, "rgb(" :: call) or (std.text.starts_with :: raw, "rgba(" :: call)):
        return (0, 0, 0, -1)
    let open = std.text.find :: raw, 0, "(" :: call
    let close = std.text.find :: raw, open + 1, ")" :: call
    if open < 0 or close <= open:
        return (0, 0, 0, -1)
    let inner = std.text.slice_bytes :: raw, open + 1, close :: call
    let parts = std.text.split :: inner, "," :: call
    if (parts :: :: len) < 3:
        return (0, 0, 0, -1)
    let red_result = std.text.to_int :: ((parts)[0]) :: call
    let green_result = std.text.to_int :: ((parts)[1]) :: call
    let blue_result = std.text.to_int :: ((parts)[2]) :: call
    if red_result :: :: is_err or green_result :: :: is_err or blue_result :: :: is_err:
        return (0, 0, 0, -1)
    let mut alpha = 255
    if (parts :: :: len) > 3:
        let alpha_text = std.text.trim :: ((parts)[3]) :: call
        let parsed = arcana_text.font_leaf.svg_decimal_fixed :: alpha_text :: call
        if parsed.ok:
            alpha = arcana_text.font_leaf.clamp_int :: ((parsed.value * 255) / 65536), 0, 255 :: call
    return (arcana_text.font_leaf.clamp_int :: (red_result :: 0 :: unwrap_or), 0, 255 :: call, arcana_text.font_leaf.clamp_int :: (green_result :: 0 :: unwrap_or), 0, 255 :: call, arcana_text.font_leaf.clamp_int :: (blue_result :: 0 :: unwrap_or), 0, 255 :: call, alpha)

fn svg_named_color(read text: Str) -> (Int, Int, Int, Int):
    let name = std.text.trim :: text :: call
    return match name:
        "black" => (0, 0, 0, 255)
        "white" => (255, 255, 255, 255)
        "red" => (255, 0, 0, 255)
        "green" => (0, 128, 0, 255)
        "blue" => (0, 0, 255, 255)
        "yellow" => (255, 255, 0, 255)
        "cyan" => (0, 255, 255, 255)
        "magenta" => (255, 0, 255, 255)
        _ => (0, 0, 0, -1)

fn svg_color_from_text(read text: Str, fallback: Int) -> (Int, Int, Int, Int):
    let value = std.text.trim :: text :: call
    if value == "" or value == "currentColor":
        return arcana_text.font_leaf.color_rgba :: fallback :: call
    if value == "none":
        return (0, 0, 0, 0)
    let hex = arcana_text.font_leaf.svg_parse_hex_color :: value :: call
    if hex.3 >= 0:
        return hex
    let rgb = arcana_text.font_leaf.svg_parse_rgb_color :: value :: call
    if rgb.3 >= 0:
        return rgb
    let named = arcana_text.font_leaf.svg_named_color :: value :: call
    if named.3 >= 0:
        return named
    return arcana_text.font_leaf.color_rgba :: fallback :: call

fn svg_apply_alpha(read color: (Int, Int, Int, Int), alpha_fixed: Int) -> (Int, Int, Int, Int):
    return (color.0, color.1, color.2, (color.3 * (arcana_text.font_leaf.clamp_int :: alpha_fixed, 0, 65536 :: call)) / 65536)

fn svg_tag_fill_color(read tag: Str, fallback: Int) -> (Int, Int, Int, Int):
    let mut fill_text = arcana_text.font_leaf.svg_attr_value :: tag, "fill" :: call
    let style = arcana_text.font_leaf.svg_attr_value :: tag, "style" :: call
    if fill_text == "" and style != "":
        fill_text = arcana_text.font_leaf.svg_style_value :: style, "fill" :: call
    let mut color = arcana_text.font_leaf.svg_color_from_text :: fill_text, fallback :: call
    let mut alpha_fixed = 65536
    let opacity_text = match (arcana_text.font_leaf.svg_attr_value :: tag, "opacity" :: call):
        "" => arcana_text.font_leaf.svg_style_value :: style, "opacity" :: call
        value => value
    if opacity_text != "":
        let opacity = arcana_text.font_leaf.svg_decimal_fixed :: opacity_text :: call
        if opacity.ok:
            alpha_fixed = arcana_text.font_leaf.fixed_16_16_mul :: alpha_fixed, opacity.value :: call
    let fill_opacity_text = match (arcana_text.font_leaf.svg_attr_value :: tag, "fill-opacity" :: call):
        "" => arcana_text.font_leaf.svg_style_value :: style, "fill-opacity" :: call
        value => value
    if fill_opacity_text != "":
        let fill_opacity = arcana_text.font_leaf.svg_decimal_fixed :: fill_opacity_text :: call
        if fill_opacity.ok:
            alpha_fixed = arcana_text.font_leaf.fixed_16_16_mul :: alpha_fixed, fill_opacity.value :: call
    color = arcana_text.font_leaf.svg_apply_alpha :: color, alpha_fixed :: call
    return color

fn svg_number_list(read text: Str) -> List[Int]:
    let mut out = std.collections.list.empty[Int] :: :: call
    let total = std.text.len_bytes :: text :: call
    let mut cursor = 0
    while cursor < total:
        let value = arcana_text.font_leaf.svg_number_read :: text, cursor :: call
        if not value.ok:
            let next = arcana_text.font_leaf.svg_skip_number_separators :: text, cursor + 1 :: call
            if next <= cursor:
                cursor += 1
            else:
                cursor = next
            continue
        out :: value.value :: push
        if value.next <= cursor:
            cursor += 1
        else:
            cursor = value.next
    return out

fn svg_view_box(read svg_text: Str, target: (Int, Int)) -> arcana_text.font_leaf.SvgViewBox:
    let svg_start = std.text.find :: svg_text, 0, "<svg" :: call
    if svg_start < 0:
        return arcana_text.font_leaf.svg_view_box_value :: (0, 0, (target.0 * 65536), (target.1 * 65536)) :: call
    let svg_end = std.text.find :: svg_text, svg_start, ">" :: call
    if svg_end < 0:
        return arcana_text.font_leaf.svg_view_box_value :: (0, 0, (target.0 * 65536), (target.1 * 65536)) :: call
    let tag = std.text.slice_bytes :: svg_text, svg_start + 1, svg_end :: call
    let view_box_text = arcana_text.font_leaf.svg_attr_value :: tag, "viewBox" :: call
    if view_box_text != "":
        let values = arcana_text.font_leaf.svg_number_list :: view_box_text :: call
        if (values :: :: len) >= 4:
            let width = arcana_text.font_leaf.max_int :: ((values)[2]), 65536 :: call
            let height = arcana_text.font_leaf.max_int :: ((values)[3]), 65536 :: call
            return arcana_text.font_leaf.svg_view_box_value :: ((values)[0], (values)[1], width, height) :: call
    let width_value = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "width" :: call) :: call
    let height_value = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "height" :: call) :: call
    let width = match width_value.ok:
        true => arcana_text.font_leaf.max_int :: width_value.value, 65536 :: call
        false => target.0 * 65536
    let height = match height_value.ok:
        true => arcana_text.font_leaf.max_int :: height_value.value, 65536 :: call
        false => target.1 * 65536
    return arcana_text.font_leaf.svg_view_box_value :: (0, 0, width, height) :: call

fn svg_map_fixed_to_pixel(read payload: (Int, Int, Int, Int)) -> Int:
    let value = payload.0
    let origin = payload.1
    let extent = payload.2
    let target = payload.3
    let safe_extent = arcana_text.font_leaf.max_int :: extent, 65536 :: call
    return (((value - origin) * target) / safe_extent) / 65536

fn svg_point_to_pixel(read point: (Int, Int), read view: arcana_text.font_leaf.SvgViewBox, target: (Int, Int)) -> (Int, Int):
    return ((arcana_text.font_leaf.svg_map_fixed_to_pixel :: (point.0, view.min_x, view.width, target.0) :: call), (arcana_text.font_leaf.svg_map_fixed_to_pixel :: (point.1, view.min_y, view.height, target.1) :: call))

fn svg_append_segment_pixel(edit out: List[arcana_text.font_leaf.LineSegment], read payload: ((Int, Int), (Int, Int), arcana_text.font_leaf.SvgViewBox, (Int, Int))):
    let from = payload.0
    let to = payload.1
    let view = payload.2
    let target = payload.3
    let start = arcana_text.font_leaf.svg_point_to_pixel :: from, view, target :: call
    let end = arcana_text.font_leaf.svg_point_to_pixel :: to, view, target :: call
    if start.0 == end.0 and start.1 == end.1:
        return
    out :: (arcana_text.font_leaf.line_segment :: start, end :: call) :: push

fn svg_bounds_for_segments(read segments: List[arcana_text.font_leaf.LineSegment]) -> (Int, Int, Int, Int):
    if segments :: :: is_empty:
        return (0, 0, 0, 0)
    let first = (segments)[0]
    let mut min_x = arcana_text.font_leaf.min_int :: first.start.0, first.end.0 :: call
    let mut min_y = arcana_text.font_leaf.min_int :: first.start.1, first.end.1 :: call
    let mut max_x = arcana_text.font_leaf.max_int :: first.start.0, first.end.0 :: call
    let mut max_y = arcana_text.font_leaf.max_int :: first.start.1, first.end.1 :: call
    for segment in segments:
        min_x = arcana_text.font_leaf.min_int :: min_x, (arcana_text.font_leaf.min_int :: segment.start.0, segment.end.0 :: call) :: call
        min_y = arcana_text.font_leaf.min_int :: min_y, (arcana_text.font_leaf.min_int :: segment.start.1, segment.end.1 :: call) :: call
        max_x = arcana_text.font_leaf.max_int :: max_x, (arcana_text.font_leaf.max_int :: segment.start.0, segment.end.0 :: call) :: call
        max_y = arcana_text.font_leaf.max_int :: max_y, (arcana_text.font_leaf.max_int :: segment.start.1, segment.end.1 :: call) :: call
    return (min_x, min_y, max_x, max_y)

fn svg_blend_segments_over(edit rgba: Array[Int], read payload: ((Int, Int), List[arcana_text.font_leaf.LineSegment], (Int, Int, Int, Int))):
    let canvas = payload.0
    let segments = payload.1
    let color = payload.2
    if color.3 <= 0 or (segments :: :: len) <= 0:
        return
    let bounds = arcana_text.font_leaf.svg_bounds_for_segments :: segments :: call
    let width = arcana_text.font_leaf.max_int :: ((bounds.2 - bounds.0) + 1), 1 :: call
    let height = arcana_text.font_leaf.max_int :: ((bounds.3 - bounds.1) + 1), 1 :: call
    let mut local = arcana_text.font_leaf.empty_segments :: :: call
    for segment in segments:
        local :: (arcana_text.font_leaf.line_segment :: ((segment.start.0 - bounds.0), (segment.start.1 - bounds.1)), ((segment.end.0 - bounds.0), (segment.end.1 - bounds.1)) :: call) :: push
    let alpha = arcana_text.font_leaf.fill_bitmap_from_segments :: (local, width, height, 2) :: call
    let mut bitmap = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (bounds.0, bounds.1), advance = 0 :: call
    bitmap.baseline = 0
    bitmap.line_height = 0
    bitmap.empty = false
    bitmap.alpha = alpha
    bitmap.lcd = empty_lcd :: :: call
    bitmap.rgba = empty_rgba :: :: call
    arcana_text.font_leaf.blend_bitmap_rgba_over :: rgba, (canvas.0, canvas.1, (0, 0), bitmap, ((color.0 << 24) + (color.1 << 16) + (color.2 << 8) + color.3)) :: call

fn svg_lerp_fixed(a: Int, b: Int, t: Int) -> Int:
    return a + (arcana_text.font_leaf.fixed_16_16_mul :: (b - a), t :: call)

fn svg_quad_point_fixed(read payload: ((Int, Int), (Int, Int), (Int, Int), Int)) -> (Int, Int):
    let p0 = payload.0
    let p1 = payload.1
    let p2 = payload.2
    let t = payload.3
    let a = ((arcana_text.font_leaf.svg_lerp_fixed :: p0.0, p1.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: p0.1, p1.1, t :: call))
    let b = ((arcana_text.font_leaf.svg_lerp_fixed :: p1.0, p2.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: p1.1, p2.1, t :: call))
    return ((arcana_text.font_leaf.svg_lerp_fixed :: a.0, b.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: a.1, b.1, t :: call))

fn svg_cubic_point_fixed(read payload: ((Int, Int), (Int, Int), (Int, Int), (Int, Int), Int)) -> (Int, Int):
    let p0 = payload.0
    let p1 = payload.1
    let p2 = payload.2
    let p3 = payload.3
    let t = payload.4
    let a = ((arcana_text.font_leaf.svg_lerp_fixed :: p0.0, p1.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: p0.1, p1.1, t :: call))
    let b = ((arcana_text.font_leaf.svg_lerp_fixed :: p1.0, p2.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: p1.1, p2.1, t :: call))
    let c = ((arcana_text.font_leaf.svg_lerp_fixed :: p2.0, p3.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: p2.1, p3.1, t :: call))
    let ab = ((arcana_text.font_leaf.svg_lerp_fixed :: a.0, b.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: a.1, b.1, t :: call))
    let bc = ((arcana_text.font_leaf.svg_lerp_fixed :: b.0, c.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: b.1, c.1, t :: call))
    return ((arcana_text.font_leaf.svg_lerp_fixed :: ab.0, bc.0, t :: call), (arcana_text.font_leaf.svg_lerp_fixed :: ab.1, bc.1, t :: call))

fn svg_rect_segments(read tag: Str, read view: arcana_text.font_leaf.SvgViewBox, target: (Int, Int)) -> List[arcana_text.font_leaf.LineSegment]:
    let mut out = arcana_text.font_leaf.empty_segments :: :: call
    let x = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "x" :: call) :: call
    let y = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "y" :: call) :: call
    let width = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "width" :: call) :: call
    let height = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "height" :: call) :: call
    if not x.ok or not y.ok or not width.ok or not height.ok or width.value <= 0 or height.value <= 0:
        return out
    let p0 = (x.value, y.value)
    let p1 = (x.value + width.value, y.value)
    let p2 = (x.value + width.value, y.value + height.value)
    let p3 = (x.value, y.value + height.value)
    arcana_text.font_leaf.svg_append_segment_pixel :: out, (p0, p1, view, target) :: call
    arcana_text.font_leaf.svg_append_segment_pixel :: out, (p1, p2, view, target) :: call
    arcana_text.font_leaf.svg_append_segment_pixel :: out, (p2, p3, view, target) :: call
    arcana_text.font_leaf.svg_append_segment_pixel :: out, (p3, p0, view, target) :: call
    return out

fn svg_poly_segments(read payload: (Str, Bool, arcana_text.font_leaf.SvgViewBox, (Int, Int))) -> List[arcana_text.font_leaf.LineSegment]:
    let points_text = payload.0
    let closed = payload.1
    let view = payload.2
    let target = payload.3
    let mut out = arcana_text.font_leaf.empty_segments :: :: call
    let values = arcana_text.font_leaf.svg_number_list :: points_text :: call
    if (values :: :: len) < 4:
        return out
    let mut index = 2
    let first = ((values)[0], (values)[1])
    let mut previous = first
    while index + 1 < (values :: :: len):
        let next = ((values)[index], (values)[index + 1])
        arcana_text.font_leaf.svg_append_segment_pixel :: out, (previous, next, view, target) :: call
        previous = next
        index += 2
    if closed:
        arcana_text.font_leaf.svg_append_segment_pixel :: out, (previous, first, view, target) :: call
    return out

fn svg_ellipse_segments(read payload: ((Int, Int), (Int, Int), arcana_text.font_leaf.SvgViewBox, (Int, Int))) -> List[arcana_text.font_leaf.LineSegment]:
    let center = payload.0
    let radius = payload.1
    let view = payload.2
    let target = payload.3
    let mut out = arcana_text.font_leaf.empty_segments :: :: call
    if radius.0 <= 0 or radius.1 <= 0:
        return out
    let steps = 24
    let mut index = 0
    let mut previous = (center.0 + radius.0, center.1)
    while index < steps:
        let next_index = index + 1
        let angle = ((arcana_text.font_leaf.fixed_two_pi :: :: call) * next_index) / steps
        let cosine = arcana_text.font_leaf.fixed_cos_pi :: angle :: call
        let sine = arcana_text.font_leaf.fixed_sin_pi :: angle :: call
        let next = (center.0 + (arcana_text.font_leaf.fixed_16_16_mul :: radius.0, cosine :: call), center.1 + (arcana_text.font_leaf.fixed_16_16_mul :: radius.1, sine :: call))
        arcana_text.font_leaf.svg_append_segment_pixel :: out, (previous, next, view, target) :: call
        previous = next
        index += 1
    return out

fn svg_circle_segments(read tag: Str, read view: arcana_text.font_leaf.SvgViewBox, target: (Int, Int)) -> List[arcana_text.font_leaf.LineSegment]:
    let cx = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "cx" :: call) :: call
    let cy = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "cy" :: call) :: call
    let radius = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "r" :: call) :: call
    if not cx.ok or not cy.ok or not radius.ok:
        return arcana_text.font_leaf.empty_segments :: :: call
    return arcana_text.font_leaf.svg_ellipse_segments :: ((cx.value, cy.value), (radius.value, radius.value), view, target) :: call

fn svg_ellipse_tag_segments(read tag: Str, read view: arcana_text.font_leaf.SvgViewBox, target: (Int, Int)) -> List[arcana_text.font_leaf.LineSegment]:
    let cx = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "cx" :: call) :: call
    let cy = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "cy" :: call) :: call
    let rx = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "rx" :: call) :: call
    let ry = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "ry" :: call) :: call
    if not cx.ok or not cy.ok or not rx.ok or not ry.ok:
        return arcana_text.font_leaf.empty_segments :: :: call
    return arcana_text.font_leaf.svg_ellipse_segments :: ((cx.value, cy.value), (rx.value, ry.value), view, target) :: call

fn svg_line_segments(read tag: Str, read view: arcana_text.font_leaf.SvgViewBox, target: (Int, Int)) -> List[arcana_text.font_leaf.LineSegment]:
    let x1 = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "x1" :: call) :: call
    let y1 = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "y1" :: call) :: call
    let x2 = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "x2" :: call) :: call
    let y2 = arcana_text.font_leaf.svg_decimal_fixed :: (arcana_text.font_leaf.svg_attr_value :: tag, "y2" :: call) :: call
    let mut out = arcana_text.font_leaf.empty_segments :: :: call
    if x1.ok and y1.ok and x2.ok and y2.ok:
        arcana_text.font_leaf.svg_append_segment_pixel :: out, ((x1.value, y1.value), (x2.value, y2.value), view, target) :: call
    return out

fn svg_path_segments(read path_text: Str, read view: arcana_text.font_leaf.SvgViewBox, target: (Int, Int)) -> List[arcana_text.font_leaf.LineSegment]:
    let mut out = arcana_text.font_leaf.empty_segments :: :: call
    let total = std.text.len_bytes :: path_text :: call
    let mut cursor = 0
    let mut command = 0
    let mut current = (0, 0)
    let mut subpath_start = (0, 0)
    let mut have_current = false
    let mut cubic_control = (0, 0)
    let mut quad_control = (0, 0)
    let mut have_cubic_control = false
    let mut have_quad_control = false
    while cursor < total:
        cursor = arcana_text.font_leaf.svg_skip_number_separators :: path_text, cursor :: call
        if cursor >= total:
            break
        let token = std.text.byte_at :: path_text, cursor :: call
        if (token >= 65 and token <= 90) or (token >= 97 and token <= 122):
            command = token
            cursor += 1
            if command == 90 or command == 122:
                if have_current:
                    arcana_text.font_leaf.svg_append_segment_pixel :: out, (current, subpath_start, view, target) :: call
                    current = subpath_start
                have_cubic_control = false
                have_quad_control = false
                continue
        if command == 0:
            cursor += 1
            continue
        if command == 77 or command == 109:
            let x_value = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not x_value.ok:
                break
            let y_value = arcana_text.font_leaf.svg_number_read :: path_text, x_value.next :: call
            if not y_value.ok:
                break
            let mut point = (x_value.value, y_value.value)
            if command == 109 and have_current:
                point = (current.0 + point.0, current.1 + point.1)
            current = point
            subpath_start = point
            have_current = true
            have_cubic_control = false
            have_quad_control = false
            cursor = y_value.next
            command = match command:
                109 => 108
                _ => 76
            continue
        if command == 76 or command == 108:
            let x_value = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not x_value.ok:
                break
            let y_value = arcana_text.font_leaf.svg_number_read :: path_text, x_value.next :: call
            if not y_value.ok:
                break
            let mut point = (x_value.value, y_value.value)
            if command == 108:
                point = (current.0 + point.0, current.1 + point.1)
            if have_current:
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (current, point, view, target) :: call
            current = point
            have_current = true
            have_cubic_control = false
            have_quad_control = false
            cursor = y_value.next
            continue
        if command == 72 or command == 104:
            let value = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not value.ok:
                break
            let mut point = (value.value, current.1)
            if command == 104:
                point = (current.0 + value.value, current.1)
            if have_current:
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (current, point, view, target) :: call
            current = point
            have_current = true
            have_cubic_control = false
            have_quad_control = false
            cursor = value.next
            continue
        if command == 86 or command == 118:
            let value = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not value.ok:
                break
            let mut point = (current.0, value.value)
            if command == 118:
                point = (current.0, current.1 + value.value)
            if have_current:
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (current, point, view, target) :: call
            current = point
            have_current = true
            have_cubic_control = false
            have_quad_control = false
            cursor = value.next
            continue
        if command == 81 or command == 113:
            let x1 = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not x1.ok:
                break
            let y1 = arcana_text.font_leaf.svg_number_read :: path_text, x1.next :: call
            if not y1.ok:
                break
            let x2 = arcana_text.font_leaf.svg_number_read :: path_text, y1.next :: call
            if not x2.ok:
                break
            let y2 = arcana_text.font_leaf.svg_number_read :: path_text, x2.next :: call
            if not y2.ok:
                break
            let mut control = (x1.value, y1.value)
            let mut point = (x2.value, y2.value)
            if command == 113:
                control = (current.0 + control.0, current.1 + control.1)
                point = (current.0 + point.0, current.1 + point.1)
            let steps = 12
            let mut prior = current
            let mut index = 1
            while index <= steps:
                let t = (index * 65536) / steps
                let next = arcana_text.font_leaf.svg_quad_point_fixed :: (current, control, point, t) :: call
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (prior, next, view, target) :: call
                prior = next
                index += 1
            current = point
            have_current = true
            quad_control = control
            have_quad_control = true
            have_cubic_control = false
            cursor = y2.next
            continue
        if command == 84 or command == 116:
            let x_value = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not x_value.ok:
                break
            let y_value = arcana_text.font_leaf.svg_number_read :: path_text, x_value.next :: call
            if not y_value.ok:
                break
            let mut control = current
            if have_quad_control:
                control = ((current.0 * 2) - quad_control.0, (current.1 * 2) - quad_control.1)
            let mut point = (x_value.value, y_value.value)
            if command == 116:
                point = (current.0 + point.0, current.1 + point.1)
            let steps = 12
            let mut prior = current
            let mut index = 1
            while index <= steps:
                let t = (index * 65536) / steps
                let next = arcana_text.font_leaf.svg_quad_point_fixed :: (current, control, point, t) :: call
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (prior, next, view, target) :: call
                prior = next
                index += 1
            current = point
            have_current = true
            quad_control = control
            have_quad_control = true
            have_cubic_control = false
            cursor = y_value.next
            continue
        if command == 67 or command == 99:
            let x1 = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not x1.ok:
                break
            let y1 = arcana_text.font_leaf.svg_number_read :: path_text, x1.next :: call
            if not y1.ok:
                break
            let x2 = arcana_text.font_leaf.svg_number_read :: path_text, y1.next :: call
            if not x2.ok:
                break
            let y2 = arcana_text.font_leaf.svg_number_read :: path_text, x2.next :: call
            if not y2.ok:
                break
            let x3 = arcana_text.font_leaf.svg_number_read :: path_text, y2.next :: call
            if not x3.ok:
                break
            let y3 = arcana_text.font_leaf.svg_number_read :: path_text, x3.next :: call
            if not y3.ok:
                break
            let mut control1 = (x1.value, y1.value)
            let mut control2 = (x2.value, y2.value)
            let mut point = (x3.value, y3.value)
            if command == 99:
                control1 = (current.0 + control1.0, current.1 + control1.1)
                control2 = (current.0 + control2.0, current.1 + control2.1)
                point = (current.0 + point.0, current.1 + point.1)
            let steps = 16
            let mut prior = current
            let mut index = 1
            while index <= steps:
                let t = (index * 65536) / steps
                let next = arcana_text.font_leaf.svg_cubic_point_fixed :: (current, control1, control2, point, t) :: call
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (prior, next, view, target) :: call
                prior = next
                index += 1
            current = point
            have_current = true
            cubic_control = control2
            have_cubic_control = true
            have_quad_control = false
            cursor = y3.next
            continue
        if command == 83 or command == 115:
            let x2 = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not x2.ok:
                break
            let y2 = arcana_text.font_leaf.svg_number_read :: path_text, x2.next :: call
            if not y2.ok:
                break
            let x3 = arcana_text.font_leaf.svg_number_read :: path_text, y2.next :: call
            if not x3.ok:
                break
            let y3 = arcana_text.font_leaf.svg_number_read :: path_text, x3.next :: call
            if not y3.ok:
                break
            let control1 = match have_cubic_control:
                true => ((current.0 * 2) - cubic_control.0, (current.1 * 2) - cubic_control.1)
                false => current
            let mut control2 = (x2.value, y2.value)
            let mut point = (x3.value, y3.value)
            if command == 115:
                control2 = (current.0 + control2.0, current.1 + control2.1)
                point = (current.0 + point.0, current.1 + point.1)
            let steps = 16
            let mut prior = current
            let mut index = 1
            while index <= steps:
                let t = (index * 65536) / steps
                let next = arcana_text.font_leaf.svg_cubic_point_fixed :: (current, control1, control2, point, t) :: call
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (prior, next, view, target) :: call
                prior = next
                index += 1
            current = point
            have_current = true
            cubic_control = control2
            have_cubic_control = true
            have_quad_control = false
            cursor = y3.next
            continue
        if command == 65 or command == 97:
            let rx = arcana_text.font_leaf.svg_number_read :: path_text, cursor :: call
            if not rx.ok:
                break
            let ry = arcana_text.font_leaf.svg_number_read :: path_text, rx.next :: call
            if not ry.ok:
                break
            let rotation = arcana_text.font_leaf.svg_number_read :: path_text, ry.next :: call
            if not rotation.ok:
                break
            let large_arc = arcana_text.font_leaf.svg_number_read :: path_text, rotation.next :: call
            if not large_arc.ok:
                break
            let sweep = arcana_text.font_leaf.svg_number_read :: path_text, large_arc.next :: call
            if not sweep.ok:
                break
            let x_value = arcana_text.font_leaf.svg_number_read :: path_text, sweep.next :: call
            if not x_value.ok:
                break
            let y_value = arcana_text.font_leaf.svg_number_read :: path_text, x_value.next :: call
            if not y_value.ok:
                break
            let mut point = (x_value.value, y_value.value)
            if command == 97:
                point = (current.0 + point.0, current.1 + point.1)
            if have_current:
                arcana_text.font_leaf.svg_append_segment_pixel :: out, (current, point, view, target) :: call
            current = point
            have_current = true
            have_cubic_control = false
            have_quad_control = false
            cursor = y_value.next
            continue
        cursor += 1
    return out

fn svg_render_document_native(read svg_text: Str, target: (Int, Int), fallback_color: Int) -> Result[arcana_text.font_leaf.DecodedColorImage, Str]:
    let width = arcana_text.font_leaf.max_int :: target.0, 1 :: call
    let height = arcana_text.font_leaf.max_int :: target.1, 1 :: call
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let view = arcana_text.font_leaf.svg_view_box :: svg_text, (width, height) :: call
    let total = std.text.len_bytes :: svg_text :: call
    let mut cursor = 0
    let mut drew = false
    while cursor < total:
        let open = std.text.find :: svg_text, cursor, "<" :: call
        if open < 0:
            break
        let close = std.text.find :: svg_text, open + 1, ">" :: call
        if close < 0:
            break
        let tag = std.text.slice_bytes :: svg_text, open + 1, close :: call
        let trimmed = std.text.trim :: tag :: call
        if trimmed != "" and not (std.text.starts_with :: trimmed, "/" :: call) and not (std.text.starts_with :: trimmed, "!" :: call) and not (std.text.starts_with :: trimmed, "?" :: call):
            let name = arcana_text.font_leaf.svg_tag_name :: tag :: call
            let color = arcana_text.font_leaf.svg_tag_fill_color :: tag, fallback_color :: call
            let mut segments = arcana_text.font_leaf.empty_segments :: :: call
            if color.3 > 0:
                segments = match name:
                    "path" => arcana_text.font_leaf.svg_path_segments :: (arcana_text.font_leaf.svg_attr_value :: tag, "d" :: call), view, (width, height) :: call
                    "rect" => arcana_text.font_leaf.svg_rect_segments :: tag, view, (width, height) :: call
                    "polygon" => arcana_text.font_leaf.svg_poly_segments :: ((arcana_text.font_leaf.svg_attr_value :: tag, "points" :: call), true, view, (width, height)) :: call
                    "polyline" => arcana_text.font_leaf.svg_poly_segments :: ((arcana_text.font_leaf.svg_attr_value :: tag, "points" :: call), false, view, (width, height)) :: call
                    "circle" => arcana_text.font_leaf.svg_circle_segments :: tag, view, (width, height) :: call
                    "ellipse" => arcana_text.font_leaf.svg_ellipse_tag_segments :: tag, view, (width, height) :: call
                    "line" => arcana_text.font_leaf.svg_line_segments :: tag, view, (width, height) :: call
                    _ => arcana_text.font_leaf.empty_segments :: :: call
                if not (segments :: :: is_empty):
                    arcana_text.font_leaf.svg_blend_segments_over :: rgba, ((width, height), segments, color) :: call
                    drew = true
        cursor = close + 1
    if not drew:
        return Result.Err[arcana_text.font_leaf.DecodedColorImage, Str] :: "SVG glyph document contains no supported filled geometry" :: call
    return Result.Ok[arcana_text.font_leaf.DecodedColorImage, Str] :: (arcana_text.font_leaf.DecodedColorImage :: size = (width, height), rgba = rgba :: call) :: call

fn svg_bitmap_metrics(read payload: (arcana_text.font_leaf.GlyphRenderSpec, (Int, Int), Int, Int, Int)) -> arcana_text.font_leaf.EmbeddedBitmapMetrics:
    let spec = payload.0
    let size = payload.1
    let advance = payload.2
    let baseline = payload.3
    let mut metrics = arcana_text.font_leaf.EmbeddedBitmapMetrics :: offset = (0, baseline - size.1), advance = advance :: call
    if spec.vertical:
        metrics.offset = (0 - (size.0 / 2), 0)
    return metrics

fn outline_color_bitmap(read face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let advance = payload.1
    let baseline = payload.2
    let line_height = payload.3
    let mut outline_spec = spec
    outline_spec.mode = arcana_text.types.RasterMode.Alpha :: :: call
    outline_spec.color = 0
    let outline = arcana_text.font_leaf.render_glyph :: face, outline_spec :: call
    return arcana_text.font_leaf.merge_color_bitmaps :: ((arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call), outline, spec.color, advance, baseline, line_height) :: call

fn render_svg_bitmap(read face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let glyph_index = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let svg_bytes_result = arcana_text.font_leaf.svg_document_bytes_for_glyph :: face, glyph_index :: call
    if svg_bytes_result :: :: is_err:
        return arcana_text.font_leaf.outline_color_bitmap :: face, (spec, advance, baseline, line_height) :: call
    let svg_bytes = svg_bytes_result :: (empty_alpha :: :: call) :: unwrap_or
    let svg_text_result = arcana_text.font_leaf.svg_document_text :: svg_bytes :: call
    if svg_text_result :: :: is_err:
        return arcana_text.font_leaf.outline_color_bitmap :: face, (spec, advance, baseline, line_height) :: call
    let target = match spec.vertical:
        true => ((arcana_text.font_leaf.max_int :: spec.font_size, 1 :: call), (arcana_text.font_leaf.max_int :: advance, spec.font_size :: call))
        false => ((arcana_text.font_leaf.max_int :: advance, spec.font_size :: call), (arcana_text.font_leaf.max_int :: line_height, spec.font_size :: call))
    let decoded_result = arcana_text.font_leaf.svg_render_document_native :: (svg_text_result :: "" :: unwrap_or), target, spec.color :: call
    if decoded_result :: :: is_err:
        return arcana_text.font_leaf.outline_color_bitmap :: face, (spec, advance, baseline, line_height) :: call
    let decoded = decoded_result :: (arcana_text.font_leaf.empty_decoded_color_image :: :: call) :: unwrap_or
    let metrics = arcana_text.font_leaf.svg_bitmap_metrics :: (spec, decoded.size, advance, baseline, line_height) :: call
    let image = arcana_text.font_leaf.EmbeddedBitmapImage :: format_tag = "svg ", bytes = svg_bytes, metrics = metrics, draw_outline = false, bottom_origin = false :: call
    return arcana_text.font_leaf.embedded_bitmap_to_glyph_bitmap :: (image, decoded, advance, baseline, line_height) :: call

fn big_bitmap_metrics(read bytes: std.memory.ByteView, offset: Int, vertical: Bool) -> arcana_text.font_leaf.EmbeddedBitmapMetrics:
    let width = byte_at_or_zero_ref :: bytes, offset + 1 :: call
    let hori_bearing_x = i8 :: (byte_at_or_zero_ref :: bytes, offset + 2 :: call) :: call
    let hori_bearing_y = i8 :: (byte_at_or_zero_ref :: bytes, offset + 3 :: call) :: call
    let hori_advance = byte_at_or_zero_ref :: bytes, offset + 4 :: call
    let vert_bearing_x = i8 :: (byte_at_or_zero_ref :: bytes, offset + 5 :: call) :: call
    let vert_bearing_y = i8 :: (byte_at_or_zero_ref :: bytes, offset + 6 :: call) :: call
    let vert_advance = byte_at_or_zero_ref :: bytes, offset + 7 :: call
    let mut metrics = arcana_text.font_leaf.EmbeddedBitmapMetrics :: offset = (hori_bearing_x, 0 - hori_bearing_y), advance = hori_advance :: call
    if vertical:
        metrics.offset = (vert_bearing_x, vert_bearing_y)
        metrics.advance = vert_advance
    if metrics.advance <= 0:
        metrics.advance = width
    return metrics

fn small_bitmap_metrics(read bytes: std.memory.ByteView, offset: Int, vertical: Bool) -> arcana_text.font_leaf.EmbeddedBitmapMetrics:
    let width = byte_at_or_zero_ref :: bytes, offset + 1 :: call
    let bearing_x = i8 :: (byte_at_or_zero_ref :: bytes, offset + 2 :: call) :: call
    let bearing_y = i8 :: (byte_at_or_zero_ref :: bytes, offset + 3 :: call) :: call
    let advance = byte_at_or_zero_ref :: bytes, offset + 4 :: call
    let mut metrics = arcana_text.font_leaf.EmbeddedBitmapMetrics :: offset = (bearing_x, 0 - bearing_y), advance = advance :: call
    if vertical:
        metrics.offset = (bearing_y, bearing_x)
    if metrics.advance <= 0:
        metrics.advance = width
    return metrics

fn sbix_best_strike_offset(read face: arcana_text.font_leaf.FontFaceState, target_ppem: Int) -> Int:
    if face.sbix_offset < 0 or face.sbix_length < 8:
        return -1
    let total = face.font_view :: :: len
    if face.sbix_offset + 8 > total:
        return -1
    let num_strikes = u32_be_ref :: face.font_view, face.sbix_offset + 4 :: call
    let wanted = max_int :: target_ppem, 1 :: call
    let mut best_offset = -1
    let mut best_distance = 2147483647
    let mut best_ppem = 0
    let mut index = 0
    while index < num_strikes:
        let strike_offset = face.sbix_offset + (u32_be_ref :: face.font_view, face.sbix_offset + 8 + (index * 4) :: call)
        if strike_offset >= face.sbix_offset and strike_offset + 4 <= total:
            let ppem = u16_be_ref :: face.font_view, strike_offset :: call
            let distance = abs_int :: (ppem - wanted) :: call
            if best_offset < 0 or distance < best_distance or (distance == best_distance and ppem > best_ppem):
                best_offset = strike_offset
                best_distance = distance
                best_ppem = ppem
        index += 1
    return best_offset

fn sbix_bitmap_image(edit face: arcana_text.font_leaf.FontFaceState, read payload: (Int, Int, Int)) -> Result[arcana_text.font_leaf.EmbeddedBitmapImage, Str]:
    let glyph_index = payload.0
    let target_ppem = payload.1
    let depth = payload.2
    if depth > 4:
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "sbix dupe depth exceeded" :: call
    if glyph_index < 0 or glyph_index >= face.glyph_count:
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "sbix glyph index out of range" :: call
    let strike_offset = arcana_text.font_leaf.sbix_best_strike_offset :: face, target_ppem :: call
    if strike_offset < 0:
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "sbix strike not found" :: call
    let total = face.font_view :: :: len
    let offsets_base = strike_offset + 4
    let glyph_offset = u32_be_ref :: face.font_view, offsets_base + (glyph_index * 4) :: call
    let next_offset = u32_be_ref :: face.font_view, offsets_base + ((glyph_index + 1) * 4) :: call
    if next_offset <= glyph_offset or glyph_offset <= 0:
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "sbix glyph has no bitmap" :: call
    let glyph_record = strike_offset + glyph_offset
    let glyph_end = strike_offset + next_offset
    if glyph_record + 8 > glyph_end or glyph_end > total:
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "sbix glyph record out of range" :: call
    let origin_x = i16_be_ref :: face.font_view, glyph_record :: call
    let origin_y = i16_be_ref :: face.font_view, glyph_record + 2 :: call
    let format_tag = tag_at_ref :: face.font_view, glyph_record + 4 :: call
    if format_tag == "dupe":
        if glyph_record + 10 > glyph_end:
            return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "sbix dupe record too short" :: call
        let dupe_glyph = u16_be_ref :: face.font_view, glyph_record + 8 :: call
        return arcana_text.font_leaf.sbix_bitmap_image :: face, (dupe_glyph, target_ppem, depth + 1) :: call
    let bytes = (face.font_view :: glyph_record + 8, glyph_end :: subview) :: :: to_array
    let mut metrics = arcana_text.font_leaf.EmbeddedBitmapMetrics :: offset = (0 - origin_x, 0 - origin_y), advance = 0 :: call
    let draw_outline = (face.sbix_flags % 4) >= 2
    return Result.Ok[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: (arcana_text.font_leaf.EmbeddedBitmapImage :: format_tag = format_tag, bytes = bytes, metrics = metrics, draw_outline = draw_outline, bottom_origin = true :: call) :: call

fn cblc_best_strike_offset(read face: arcana_text.font_leaf.FontFaceState, target_ppem: Int, vertical: Bool) -> Int:
    if face.cblc_offset < 0 or face.cblc_length < 8:
        return -1
    let total = face.font_view :: :: len
    if face.cblc_offset + 8 > total:
        return -1
    let num_sizes = u32_be_ref :: face.font_view, face.cblc_offset + 4 :: call
    let wanted = max_int :: target_ppem, 1 :: call
    let mut best_offset = -1
    let mut best_distance = 2147483647
    let mut best_ppem = 0
    let mut index = 0
    while index < num_sizes:
        let size_offset = face.cblc_offset + 8 + (index * 48)
        if size_offset + 48 > total:
            return best_offset
        let ppem = match vertical:
            true => byte_at_or_zero_ref :: face.font_view, size_offset + 45 :: call
            false => byte_at_or_zero_ref :: face.font_view, size_offset + 44 :: call
        let distance = abs_int :: (ppem - wanted) :: call
        if best_offset < 0 or distance < best_distance or (distance == best_distance and ppem > best_ppem):
            best_offset = size_offset
            best_distance = distance
            best_ppem = ppem
        index += 1
    return best_offset

fn cblc_subtable_location(read face: arcana_text.font_leaf.FontFaceState, size_offset: Int, glyph_index: Int) -> (Int, Int, Int, Int):
    let total = face.font_view :: :: len
    let list_offset = face.cblc_offset + (u32_be_ref :: face.font_view, size_offset :: call)
    let subtable_count = u32_be_ref :: face.font_view, size_offset + 8 :: call
    let mut index = 0
    while index < subtable_count:
        let record = list_offset + (index * 8)
        if record + 8 > total:
            return (-1, 0, 0, 0)
        let first = u16_be_ref :: face.font_view, record :: call
        let last = u16_be_ref :: face.font_view, record + 2 :: call
        if glyph_index >= first and glyph_index <= last:
            let subtable_offset = list_offset + (u32_be_ref :: face.font_view, record + 4 :: call)
            if subtable_offset + 8 > total:
                return (-1, 0, 0, 0)
            let index_format = u16_be_ref :: face.font_view, subtable_offset :: call
            let image_format = u16_be_ref :: face.font_view, subtable_offset + 2 :: call
            let image_data_offset = face.cbdt_offset + (u32_be_ref :: face.font_view, subtable_offset + 4 :: call)
            return (subtable_offset, index_format, image_format, image_data_offset)
        index += 1
    return (-1, 0, 0, 0)

fn cblc_bitmap_record(read face: arcana_text.font_leaf.FontFaceState, size_offset: Int, glyph_index: Int) -> (Int, Int, Int):
    let location = arcana_text.font_leaf.cblc_subtable_location :: face, size_offset, glyph_index :: call
    let subtable_offset = location.0
    if subtable_offset < 0:
        return (-1, 0, 0)
    let index_format = location.1
    let image_data_offset = location.3
    let total = face.font_view :: :: len
    let list_offset = face.cblc_offset + (u32_be_ref :: face.font_view, size_offset :: call)
    let subtable_count = u32_be_ref :: face.font_view, size_offset + 8 :: call
    let mut first_glyph = 0
    let mut last_glyph = 0
    let mut index = 0
    while index < subtable_count:
        let record = list_offset + (index * 8)
        if record + 8 > total:
            return (-1, 0, 0)
        let first = u16_be_ref :: face.font_view, record :: call
        let last = u16_be_ref :: face.font_view, record + 2 :: call
        if glyph_index >= first and glyph_index <= last:
            first_glyph = first
            last_glyph = last
            index = subtable_count
        else:
            index += 1
    if first_glyph > last_glyph:
        return (-1, 0, 0)
    if index_format == 1:
        let entry = glyph_index - first_glyph
        let start = image_data_offset + (u32_be_ref :: face.font_view, subtable_offset + 8 + (entry * 4) :: call)
        let stop = image_data_offset + (u32_be_ref :: face.font_view, subtable_offset + 12 + (entry * 4) :: call)
        return (start, stop - start, 0)
    if index_format == 2:
        let image_size = u32_be_ref :: face.font_view, subtable_offset + 8 :: call
        let entry = glyph_index - first_glyph
        return (image_data_offset + (entry * image_size), image_size, subtable_offset + 12)
    if index_format == 3:
        let entry = glyph_index - first_glyph
        let start = image_data_offset + (u16_be_ref :: face.font_view, subtable_offset + 8 + (entry * 2) :: call)
        let stop = image_data_offset + (u16_be_ref :: face.font_view, subtable_offset + 10 + (entry * 2) :: call)
        return (start, stop - start, 0)
    if index_format == 4:
        let num_glyphs = u32_be_ref :: face.font_view, subtable_offset + 8 :: call
        let mut pair = 0
        while pair < num_glyphs:
            let record = subtable_offset + 12 + (pair * 4)
            let current = u16_be_ref :: face.font_view, record :: call
            if current == glyph_index:
                let start = image_data_offset + (u16_be_ref :: face.font_view, record + 2 :: call)
                let stop = image_data_offset + (u16_be_ref :: face.font_view, record + 6 :: call)
                return (start, stop - start, 0)
            pair += 1
        return (-1, 0, 0)
    if index_format == 5:
        let image_size = u32_be_ref :: face.font_view, subtable_offset + 8 :: call
        let num_glyphs = u32_be_ref :: face.font_view, subtable_offset + 20 :: call
        let mut entry = 0
        while entry < num_glyphs:
            if (u16_be_ref :: face.font_view, subtable_offset + 24 + (entry * 2) :: call) == glyph_index:
                return (image_data_offset + (entry * image_size), image_size, subtable_offset + 12)
            entry += 1
    return (-1, 0, 0)

fn cbdt_bitmap_image(read face: arcana_text.font_leaf.FontFaceState, read payload: (Int, Int, Bool)) -> Result[arcana_text.font_leaf.EmbeddedBitmapImage, Str]:
    let glyph_index = payload.0
    let font_size = payload.1
    let vertical = payload.2
    let size_offset = arcana_text.font_leaf.cblc_best_strike_offset :: face, font_size, vertical :: call
    if size_offset < 0:
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "CBDT strike not found" :: call
    let location = arcana_text.font_leaf.cblc_subtable_location :: face, size_offset, glyph_index :: call
    let image_format = location.2
    let record = arcana_text.font_leaf.cblc_bitmap_record :: face, size_offset, glyph_index :: call
    let data_offset = record.0
    let data_length = record.1
    if data_offset < 0 or data_length <= 0 or data_offset + data_length > (face.font_view :: :: len):
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "CBDT glyph has no bitmap" :: call
    let flags = byte_at_or_zero_ref :: face.font_view, size_offset + 47 :: call
    let vertical_metrics = (flags % 4) >= 2
    let use_vertical_metrics = vertical and vertical_metrics
    let mut metrics = arcana_text.font_leaf.EmbeddedBitmapMetrics :: offset = (0, 0), advance = 0 :: call
    let mut image_start = data_offset
    if image_format == 17:
        metrics = arcana_text.font_leaf.small_bitmap_metrics :: face.font_view, data_offset, use_vertical_metrics :: call
        image_start = data_offset + 5 + 4
    else:
        if image_format == 18:
            metrics = arcana_text.font_leaf.big_bitmap_metrics :: face.font_view, data_offset, use_vertical_metrics :: call
            image_start = data_offset + 8 + 4
        else:
            if image_format == 19:
                if record.2 > 0:
                    metrics = arcana_text.font_leaf.big_bitmap_metrics :: face.font_view, record.2, use_vertical_metrics :: call
                else:
                    metrics.advance = max_int :: font_size, 1 :: call
                image_start = data_offset + 4
            else:
                return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: ("unsupported CBDT image format `" + (std.text.from_int :: image_format :: call) + "`") :: call
    let png_length = u32_be_ref :: face.font_view, image_start - 4 :: call
    if png_length <= 0 or image_start + png_length > data_offset + data_length:
        return Result.Err[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: "CBDT image payload is invalid" :: call
    let bytes = (face.font_view :: image_start, image_start + png_length :: subview) :: :: to_array
    return Result.Ok[arcana_text.font_leaf.EmbeddedBitmapImage, Str] :: (arcana_text.font_leaf.EmbeddedBitmapImage :: format_tag = "png ", bytes = bytes, metrics = metrics, draw_outline = false, bottom_origin = false :: call) :: call

fn render_sbix_bitmap(edit face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let glyph_index = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let image_result = arcana_text.font_leaf.sbix_bitmap_image :: face, (glyph_index, spec.font_size, 0) :: call
    if image_result :: :: is_err:
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let image = image_result :: (arcana_text.font_leaf.empty_embedded_bitmap_image :: :: call) :: unwrap_or
    let decoded_result = arcana_text.font_leaf.decode_external_color_image :: image :: call
    if decoded_result :: :: is_err:
        return arcana_text.font_leaf.outline_color_bitmap :: face, (spec, advance, baseline, line_height) :: call
    let bitmap = arcana_text.font_leaf.embedded_bitmap_to_glyph_bitmap :: (image, (decoded_result :: (arcana_text.font_leaf.empty_decoded_color_image :: :: call) :: unwrap_or), advance, baseline, line_height) :: call
    if not image.draw_outline:
        return bitmap
    let mut outline_spec = spec
    outline_spec.mode = arcana_text.types.RasterMode.Alpha :: :: call
    let outline = arcana_text.font_leaf.render_glyph :: face, outline_spec :: call
    return arcana_text.font_leaf.merge_color_bitmaps :: (bitmap, outline, spec.color, advance, baseline, line_height) :: call

fn render_cbdt_bitmap(read face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let glyph_index = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let image_result = arcana_text.font_leaf.cbdt_bitmap_image :: face, (glyph_index, spec.font_size, spec.vertical) :: call
    if image_result :: :: is_err:
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let image = image_result :: (arcana_text.font_leaf.empty_embedded_bitmap_image :: :: call) :: unwrap_or
    let decoded_result = arcana_text.font_leaf.decode_external_color_image :: image :: call
    if decoded_result :: :: is_err:
        return arcana_text.font_leaf.outline_color_bitmap :: face, (spec, advance, baseline, line_height) :: call
    return arcana_text.font_leaf.embedded_bitmap_to_glyph_bitmap :: (image, (decoded_result :: (arcana_text.font_leaf.empty_decoded_color_image :: :: call) :: unwrap_or), advance, baseline, line_height) :: call

fn blend_rgba_over(edit dest: Array[Int], dest_index: Int, read source: (Int, Int, Int, Int)):
    let src_alpha = clamp_int :: source.3, 0, 255 :: call
    if src_alpha <= 0:
        return
    let dst_alpha = (dest)[dest_index + 3]
    let out_alpha = src_alpha + ((dst_alpha * (255 - src_alpha)) / 255)
    if out_alpha <= 0:
        return
    let src_red = source.0
    let src_green = source.1
    let src_blue = source.2
    let dst_red = (dest)[dest_index]
    let dst_green = (dest)[dest_index + 1]
    let dst_blue = (dest)[dest_index + 2]
    let dst_weight = (dst_alpha * (255 - src_alpha)) / 255
    (dest)[dest_index] = (((src_red * src_alpha) + (dst_red * dst_weight)) / out_alpha)
    (dest)[dest_index + 1] = (((src_green * src_alpha) + (dst_green * dst_weight)) / out_alpha)
    (dest)[dest_index + 2] = (((src_blue * src_alpha) + (dst_blue * dst_weight)) / out_alpha)
    (dest)[dest_index + 3] = out_alpha

fn blend_bitmap_rgba_over(edit dest: Array[Int], read payload: (Int, Int, (Int, Int), arcana_text.font_leaf.GlyphBitmap, Int)):
    let surface_width = payload.0
    let surface_height = payload.1
    let origin = payload.2
    let bitmap = payload.3
    let color = payload.4
    if bitmap.empty or bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
        return
    let tint = arcana_text.font_leaf.color_rgba :: color :: call
    let mut y = 0
    while y < bitmap.size.1:
        let mut x = 0
        while x < bitmap.size.0:
            let dest_x = (bitmap.offset.0 - origin.0) + x
            let dest_y = (bitmap.offset.1 - origin.1) + y
            if dest_x >= 0 and dest_x < surface_width and dest_y >= 0 and dest_y < surface_height:
                let dest_index = ((dest_y * surface_width) + dest_x) * 4
                if (bitmap.rgba :: :: len) > 0:
                    let source_index = ((y * bitmap.size.0) + x) * 4
                    let source = ((bitmap.rgba)[source_index], (bitmap.rgba)[source_index + 1], (bitmap.rgba)[source_index + 2], (bitmap.rgba)[source_index + 3])
                    arcana_text.font_leaf.blend_rgba_over :: dest, dest_index, source :: call
                else:
                    let coverage = (bitmap.alpha)[(y * bitmap.size.0) + x]
                    if coverage > 0:
                        arcana_text.font_leaf.blend_rgba_over :: dest, dest_index, (tint.0, tint.1, tint.2, (coverage * tint.3) / 255) :: call
            x += 1
        y += 1

fn merge_color_bitmaps(read payload: (arcana_text.font_leaf.GlyphBitmap, arcana_text.font_leaf.GlyphBitmap, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let base = payload.0
    let overlay = payload.1
    let overlay_color = payload.2
    let advance = payload.3
    let baseline = payload.4
    let line_height = payload.5
    if overlay.empty:
        return base
    if base.empty:
        let min_x = overlay.offset.0
        let min_y = overlay.offset.1
        let width = max_int :: overlay.size.0, 1 :: call
        let height = max_int :: overlay.size.1, 1 :: call
        let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
        arcana_text.font_leaf.blend_bitmap_rgba_over :: rgba, (width, height, (min_x, min_y), overlay, overlay_color) :: call
        let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (min_x, min_y), advance = advance :: call
        out.baseline = baseline
        out.line_height = line_height
        out.empty = false
        out.alpha = arcana_text.font_leaf.rgba_alpha :: rgba :: call
        out.lcd = empty_lcd :: :: call
        out.rgba = rgba
        return out
    let min_x = min_int :: base.offset.0, overlay.offset.0 :: call
    let min_y = min_int :: base.offset.1, overlay.offset.1 :: call
    let max_x = max_int :: (base.offset.0 + base.size.0), (overlay.offset.0 + overlay.size.0) :: call
    let max_y = max_int :: (base.offset.1 + base.size.1), (overlay.offset.1 + overlay.size.1) :: call
    let width = max_int :: (max_x - min_x), 1 :: call
    let height = max_int :: (max_y - min_y), 1 :: call
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    arcana_text.font_leaf.blend_bitmap_rgba_over :: rgba, (width, height, (min_x, min_y), base, 0) :: call
    arcana_text.font_leaf.blend_bitmap_rgba_over :: rgba, (width, height, (min_x, min_y), overlay, overlay_color) :: call
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (min_x, min_y), advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = arcana_text.font_leaf.rgba_alpha :: rgba :: call
    out.lcd = empty_lcd :: :: call
    out.rgba = rgba
    return out

fn empty_color_layers() -> List[arcana_text.font_leaf.ColorLayerBitmap]:
    return std.collections.list.empty[arcana_text.font_leaf.ColorLayerBitmap] :: :: call

fn translated_bitmap(read bitmap: arcana_text.font_leaf.GlyphBitmap, delta: (Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    if bitmap.empty:
        return bitmap
    let mut next = bitmap
    next.offset = (bitmap.offset.0 + delta.0, bitmap.offset.1 + delta.1)
    return next

fn translated_color_layers(read layers: List[arcana_text.font_leaf.ColorLayerBitmap], delta: (Int, Int)) -> List[arcana_text.font_leaf.ColorLayerBitmap]:
    let mut out = arcana_text.font_leaf.empty_color_layers :: :: call
    for layer in layers:
        let mut next = layer
        next.bitmap = arcana_text.font_leaf.translated_bitmap :: layer.bitmap, delta :: call
        out :: next :: push
    return out

fn compose_color_layers(read payload: (List[arcana_text.font_leaf.ColorLayerBitmap], Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let layers = payload.0
    let advance = payload.1
    let baseline = payload.2
    let line_height = payload.3
    let mut min_x = 0
    let mut min_y = 0
    let mut max_x = 0
    let mut max_y = 0
    let mut seen = false
    for layer in layers:
        let bitmap = layer.bitmap
        if bitmap.empty or bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
            continue
        let right = bitmap.offset.0 + bitmap.size.0
        let bottom = bitmap.offset.1 + bitmap.size.1
        if not seen:
            min_x = bitmap.offset.0
            min_y = bitmap.offset.1
            max_x = right
            max_y = bottom
            seen = true
        else:
            min_x = min_int :: min_x, bitmap.offset.0 :: call
            min_y = min_int :: min_y, bitmap.offset.1 :: call
            max_x = max_int :: max_x, right :: call
            max_y = max_int :: max_y, bottom :: call
    if not seen:
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let width = max_int :: (max_x - min_x), 1 :: call
    let height = max_int :: (max_y - min_y), 1 :: call
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    for layer in layers:
        let bitmap = layer.bitmap
        if bitmap.empty:
            continue
        if (bitmap.rgba :: :: len) > 0:
            arcana_text.font_leaf.blend_bitmap_rgba_over :: rgba, (width, height, (min_x, min_y), bitmap, 0) :: call
            continue
        if layer.color.3 <= 0:
            continue
        let mut y = 0
        while y < bitmap.size.1:
            let mut x = 0
            while x < bitmap.size.0:
                let coverage = (bitmap.alpha)[(y * bitmap.size.0) + x]
                if coverage > 0:
                    let dest_x = (bitmap.offset.0 - min_x) + x
                    let dest_y = (bitmap.offset.1 - min_y) + y
                    let dest_index = ((dest_y * width) + dest_x) * 4
                    let alpha = (coverage * layer.color.3) / 255
                    arcana_text.font_leaf.blend_rgba_over :: rgba, dest_index, (layer.color.0, layer.color.1, layer.color.2, alpha) :: call
                x += 1
            y += 1
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (min_x, min_y), advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = arcana_text.font_leaf.rgba_alpha :: rgba :: call
    out.lcd = empty_lcd :: :: call
    out.rgba = rgba
    return out

fn clip_color_bitmap(read payload: (arcana_text.font_leaf.GlyphBitmap, arcana_text.font_leaf.GlyphBitmap, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let child = payload.0
    let clip = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    if child.empty or clip.empty:
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let min_x = max_int :: child.offset.0, clip.offset.0 :: call
    let min_y = max_int :: child.offset.1, clip.offset.1 :: call
    let max_x = min_int :: (child.offset.0 + child.size.0), (clip.offset.0 + clip.size.0) :: call
    let max_y = min_int :: (child.offset.1 + child.size.1), (clip.offset.1 + clip.size.1) :: call
    if max_x <= min_x or max_y <= min_y:
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let width = max_x - min_x
    let height = max_y - min_y
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let child_x = x + (min_x - child.offset.0)
            let child_y = y + (min_y - child.offset.1)
            let clip_x = x + (min_x - clip.offset.0)
            let clip_y = y + (min_y - clip.offset.1)
            let clip_alpha = (clip.alpha)[(clip_y * clip.size.0) + clip_x]
            if clip_alpha > 0:
                let dest_index = ((y * width) + x) * 4
                let source_index = ((child_y * child.size.0) + child_x) * 4
                if (child.rgba :: :: len) > 0 and source_index + 3 < (child.rgba :: :: len):
                    let src_alpha = ((child.rgba)[source_index + 3] * clip_alpha) / 255
                    arcana_text.font_leaf.blend_rgba_over :: rgba, dest_index, ((child.rgba)[source_index], (child.rgba)[source_index + 1], (child.rgba)[source_index + 2], src_alpha) :: call
                else:
                    let src_alpha = ((child.alpha)[(child_y * child.size.0) + child_x] * clip_alpha) / 255
                    arcana_text.font_leaf.blend_rgba_over :: rgba, dest_index, (255, 255, 255, src_alpha) :: call
            x += 1
        y += 1
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (min_x, min_y), advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = arcana_text.font_leaf.rgba_alpha :: rgba :: call
    out.lcd = empty_lcd :: :: call
    out.rgba = rgba
    return out

fn transform_point_16_16(point: (Int, Int), matrix: (Int, Int, Int, Int), delta: (Int, Int)) -> (Int, Int):
    let x = ((matrix.0 * point.0) + (matrix.1 * point.1)) / 65536 + delta.0
    let y = ((matrix.2 * point.0) + (matrix.3 * point.1)) / 65536 + delta.1
    return (x, y)

fn color_bitmap_bounds_after_transform(read bitmap: arcana_text.font_leaf.GlyphBitmap, matrix: (Int, Int, Int, Int), delta: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let top_left = arcana_text.font_leaf.transform_point_16_16 :: bitmap.offset, matrix, delta :: call
    let top_right = arcana_text.font_leaf.transform_point_16_16 :: ((bitmap.offset.0 + bitmap.size.0), bitmap.offset.1), matrix, delta :: call
    let bottom_left = arcana_text.font_leaf.transform_point_16_16 :: (bitmap.offset.0, (bitmap.offset.1 + bitmap.size.1)), matrix, delta :: call
    let bottom_right = arcana_text.font_leaf.transform_point_16_16 :: ((bitmap.offset.0 + bitmap.size.0), (bitmap.offset.1 + bitmap.size.1)), matrix, delta :: call
    let min_x = min_int :: (min_int :: top_left.0, top_right.0 :: call), (min_int :: bottom_left.0, bottom_right.0 :: call) :: call
    let min_y = min_int :: (min_int :: top_left.1, top_right.1 :: call), (min_int :: bottom_left.1, bottom_right.1 :: call) :: call
    let max_x = max_int :: (max_int :: top_left.0, top_right.0 :: call), (max_int :: bottom_left.0, bottom_right.0 :: call) :: call
    let max_y = max_int :: (max_int :: top_left.1, top_right.1 :: call), (max_int :: bottom_left.1, bottom_right.1 :: call) :: call
    return ((min_x, min_y), (max_x, max_y))

fn transformed_bitmap_16_16(read payload: (arcana_text.font_leaf.GlyphBitmap, (Int, Int, Int, Int), (Int, Int), Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let bitmap = payload.0
    let matrix = payload.1
    let delta = payload.2
    let advance = payload.3
    let baseline = payload.4
    let line_height = payload.5
    if bitmap.empty or bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    let bounds = arcana_text.font_leaf.color_bitmap_bounds_after_transform :: bitmap, matrix, delta :: call
    let min_x = bounds.0.0
    let min_y = bounds.0.1
    let max_x = bounds.1.0
    let max_y = bounds.1.1
    let width = max_int :: (max_x - min_x), 1 :: call
    let height = max_int :: (max_y - min_y), 1 :: call
    let has_rgba = (bitmap.rgba :: :: len) > 0
    let mut alpha = std.kernel.collections.array_new[Int] :: (width * height), 0 :: call
    let mut rgba = empty_rgba :: :: call
    if has_rgba:
        rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut y = 0
    while y < bitmap.size.1:
        let mut x = 0
        while x < bitmap.size.0:
            let dest_point = arcana_text.font_leaf.transform_point_16_16 :: ((bitmap.offset.0 + x), (bitmap.offset.1 + y)), matrix, delta :: call
            let dest_x = dest_point.0 - min_x
            let dest_y = dest_point.1 - min_y
            if dest_x >= 0 and dest_x < width and dest_y >= 0 and dest_y < height:
                let dest_index = (dest_y * width) + dest_x
                if has_rgba:
                    let source_index = ((y * bitmap.size.0) + x) * 4
                    arcana_text.font_leaf.blend_rgba_over :: rgba, dest_index * 4, ((bitmap.rgba)[source_index], (bitmap.rgba)[source_index + 1], (bitmap.rgba)[source_index + 2], (bitmap.rgba)[source_index + 3]) :: call
                else:
                    let source_alpha = (bitmap.alpha)[(y * bitmap.size.0) + x]
                    if source_alpha > alpha[dest_index]:
                        alpha[dest_index] = source_alpha
            x += 1
        y += 1
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (min_x, min_y), advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    if has_rgba:
        out.rgba = rgba
        out.alpha = arcana_text.font_leaf.rgba_alpha :: rgba :: call
    else:
        out.alpha = alpha
        out.rgba = empty_rgba :: :: call
    out.lcd = empty_lcd :: :: call
    return out

fn affine_color_layers(read payload: (List[arcana_text.font_leaf.ColorLayerBitmap], (Int, Int, Int, Int), (Int, Int), Int, Int, Int)) -> List[arcana_text.font_leaf.ColorLayerBitmap]:
    let layers = payload.0
    let matrix = payload.1
    let delta = payload.2
    let advance = payload.3
    let baseline = payload.4
    let line_height = payload.5
    let mut out = arcana_text.font_leaf.empty_color_layers :: :: call
    for layer in layers:
        let mut next = layer
        next.bitmap = arcana_text.font_leaf.transformed_bitmap_16_16 :: (layer.bitmap, matrix, delta, advance, baseline, line_height) :: call
        out :: next :: push
    return out

fn transformed_center_delta(center: (Int, Int), matrix: (Int, Int, Int, Int)) -> (Int, Int):
    let transformed = arcana_text.font_leaf.transform_point_16_16 :: center, matrix, (0, 0) :: call
    return (center.0 - transformed.0, center.1 - transformed.1)

fn rgba_surface_from_bitmap(read bitmap: arcana_text.font_leaf.GlyphBitmap, origin: (Int, Int), size: (Int, Int)) -> Array[Int]:
    let width = size.0
    let height = size.1
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    arcana_text.font_leaf.blend_bitmap_rgba_over :: rgba, (width, height, origin, bitmap, 0) :: call
    return rgba

fn composite_rgba_channel(source: Int, backdrop: Int, mode: Int) -> Int:
    return match mode:
        12 => clamp_int :: (source + backdrop), 0, 255 :: call
        13 => 255 - (((255 - source) * (255 - backdrop)) / 255)
        23 => (source * backdrop) / 255
        _ => source

fn composite_color_bitmaps(read payload: (arcana_text.font_leaf.GlyphBitmap, arcana_text.font_leaf.GlyphBitmap, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let backdrop = payload.0
    let source = payload.1
    let mode = payload.2
    let advance = payload.3
    let baseline = payload.4
    let line_height = payload.5
    if mode == 0:
        return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
    if mode == 1:
        return source
    if mode == 2:
        return backdrop
    let min_x = min_int :: backdrop.offset.0, source.offset.0 :: call
    let min_y = min_int :: backdrop.offset.1, source.offset.1 :: call
    let max_x = max_int :: (backdrop.offset.0 + backdrop.size.0), (source.offset.0 + source.size.0) :: call
    let max_y = max_int :: (backdrop.offset.1 + backdrop.size.1), (source.offset.1 + source.size.1) :: call
    let width = max_int :: (max_x - min_x), 1 :: call
    let height = max_int :: (max_y - min_y), 1 :: call
    let source_rgba = arcana_text.font_leaf.rgba_surface_from_bitmap :: source, (min_x, min_y), (width, height) :: call
    let backdrop_rgba = arcana_text.font_leaf.rgba_surface_from_bitmap :: backdrop, (min_x, min_y), (width, height) :: call
    let mut rgba = std.kernel.collections.array_new[Int] :: (width * height * 4), 0 :: call
    let mut index = 0
    while index < (width * height):
        let source_index = index * 4
        let src_a = (source_rgba)[source_index + 3]
        let dst_a = (backdrop_rgba)[source_index + 3]
        let out_alpha = match mode:
            4 => dst_a + ((src_a * (255 - dst_a)) / 255)
            5 => (src_a * dst_a) / 255
            6 => (dst_a * src_a) / 255
            7 => (src_a * (255 - dst_a)) / 255
            8 => (dst_a * (255 - src_a)) / 255
            9 => dst_a
            10 => src_a
            11 => (src_a * (255 - dst_a) + dst_a * (255 - src_a)) / 255
            _ => src_a + ((dst_a * (255 - src_a)) / 255)
        let src_factor = match mode:
            4 => ((255 - dst_a) * 255) / 255
            5 => dst_a
            6 => dst_a
            7 => 255 - dst_a
            8 => 0
            9 => 255
            10 => 255 - dst_a
            11 => 255 - dst_a
            _ => 255
        let dst_factor = match mode:
            4 => 255
            5 => 0
            6 => src_a
            7 => 0
            8 => 255 - src_a
            9 => 255 - src_a
            10 => 255
            11 => 255 - src_a
            _ => 255 - src_a
        let mut channel = 0
        while channel < 3:
            let src_value = (source_rgba)[source_index + channel]
            let dst_value = (backdrop_rgba)[source_index + channel]
            let composed = match mode:
                12 => clamp_int :: (src_value + dst_value), 0, 255 :: call
                13 => arcana_text.font_leaf.composite_rgba_channel :: src_value, dst_value, 13 :: call
                23 => arcana_text.font_leaf.composite_rgba_channel :: src_value, dst_value, 23 :: call
                _ => (((src_value * src_factor) + (dst_value * dst_factor)) / 255)
            rgba[source_index + channel] = match out_alpha > 0:
                true => clamp_int :: composed, 0, 255 :: call
                false => 0
            channel += 1
        rgba[source_index + 3] = clamp_int :: out_alpha, 0, 255 :: call
        index += 1
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (min_x, min_y), advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = arcana_text.font_leaf.rgba_alpha :: rgba :: call
    out.lcd = empty_lcd :: :: call
    out.rgba = rgba
    return out

fn colr_v1_render_paint(edit face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int, Int)) -> List[arcana_text.font_leaf.ColorLayerBitmap]:
    let spec = payload.0
    let paint_offset = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    let depth = payload.5
    let mut out = arcana_text.font_leaf.empty_color_layers :: :: call
    if depth > 16 or paint_offset < 0 or paint_offset >= (face.font_view :: :: len):
        return out
    let format = byte_at_or_zero_ref :: face.font_view, paint_offset :: call
    if format == 1:
        let layer_count = byte_at_or_zero_ref :: face.font_view, paint_offset + 1 :: call
        let first_layer = u32_be_ref :: face.font_view, paint_offset + 2 :: call
        let mut index = 0
        while index < layer_count:
            let child_offset = arcana_text.font_leaf.colr_v1_layer_paint_offset :: face, first_layer + index :: call
            if child_offset >= 0:
                out :: (arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child_offset, advance, baseline, line_height, depth + 1) :: call) :: extend_list
            index += 1
        return out
    if format == 4 or format == 7:
        let bitmap = arcana_text.font_leaf.render_linear_gradient_bitmap :: face, (spec, paint_offset, advance, baseline, line_height) :: call
        if not bitmap.empty:
            out :: (arcana_text.font_leaf.ColorLayerBitmap :: bitmap = bitmap, color = (0, 0, 0, 0) :: call) :: push
        return out
    if format == 5 or format == 8:
        let bitmap = arcana_text.font_leaf.render_radial_gradient_bitmap :: face, (spec, paint_offset, advance, baseline, line_height) :: call
        if not bitmap.empty:
            out :: (arcana_text.font_leaf.ColorLayerBitmap :: bitmap = bitmap, color = (0, 0, 0, 0) :: call) :: push
        return out
    if format == 6 or format == 9:
        let bitmap = arcana_text.font_leaf.render_sweep_gradient_bitmap :: face, (spec, paint_offset, advance, baseline, line_height) :: call
        if not bitmap.empty:
            out :: (arcana_text.font_leaf.ColorLayerBitmap :: bitmap = bitmap, color = (0, 0, 0, 0) :: call) :: push
        return out
    if format == 10:
        let child_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 1 :: call)
        let glyph_id = u16_be_ref :: face.font_view, paint_offset + 4 :: call
        let solid = arcana_text.font_leaf.paint_solid_color :: face, child_offset, spec.color :: call
        if solid.3 > 0:
            let mut glyph_spec = spec
            glyph_spec.glyph_index = glyph_id
            glyph_spec.mode = arcana_text.types.RasterMode.Alpha :: :: call
            glyph_spec.color = 0
            let bitmap = arcana_text.font_leaf.render_glyph :: face, glyph_spec :: call
            if not bitmap.empty:
                out :: (arcana_text.font_leaf.ColorLayerBitmap :: bitmap = bitmap, color = solid :: call) :: push
                return out
        let child_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child_offset, advance, baseline, line_height, depth + 1) :: call
        let child_bitmap = arcana_text.font_leaf.compose_color_layers :: (child_layers, advance, baseline, line_height) :: call
        if child_bitmap.empty:
            return out
        let mut glyph_spec = spec
        glyph_spec.glyph_index = glyph_id
        glyph_spec.mode = arcana_text.types.RasterMode.Alpha :: :: call
        glyph_spec.color = 0
        let clip_bitmap = arcana_text.font_leaf.render_glyph :: face, glyph_spec :: call
        let clipped = arcana_text.font_leaf.clip_color_bitmap :: (child_bitmap, clip_bitmap, advance, baseline, line_height) :: call
        if not clipped.empty:
            out :: (arcana_text.font_leaf.ColorLayerBitmap :: bitmap = clipped, color = (0, 0, 0, 0) :: call) :: push
        return out
    if format == 11:
        let glyph_id = u16_be_ref :: face.font_view, paint_offset + 1 :: call
        let child = arcana_text.font_leaf.colr_v1_base_paint_offset :: face, glyph_id :: call
        if child >= 0:
            return arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child, advance, baseline, line_height, depth + 1) :: call
        return out
    if format == 12 or format == 13:
        let child_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 1 :: call)
        let transform_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 4 :: call)
        if transform_offset < 0 or transform_offset + 24 > (face.font_view :: :: len):
            return out
        let xx = i32_be_ref :: face.font_view, transform_offset :: call
        let yx = i32_be_ref :: face.font_view, transform_offset + 4 :: call
        let xy = i32_be_ref :: face.font_view, transform_offset + 8 :: call
        let yy = i32_be_ref :: face.font_view, transform_offset + 12 :: call
        let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
        let dx = fixed_16_16_to_int :: (i32_be_ref :: face.font_view, transform_offset + 16 :: call) :: call
        let dy = fixed_16_16_to_int :: (i32_be_ref :: face.font_view, transform_offset + 20 :: call) :: call
        let delta = scaled_point :: ctx, (dx, dy) :: call
        let child_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child_offset, advance, baseline, line_height, depth + 1) :: call
        return arcana_text.font_leaf.affine_color_layers :: (child_layers, (xx, xy, yx, yy), delta, advance, baseline, line_height) :: call
    if format == 14 or format == 15:
        let child_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 1 :: call)
        let dx = i16_be_ref :: face.font_view, paint_offset + 4 :: call
        let dy = i16_be_ref :: face.font_view, paint_offset + 6 :: call
        let scale = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
        let delta = scaled_point :: scale, (dx, dy) :: call
        let child_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child_offset, advance, baseline, line_height, depth + 1) :: call
        return arcana_text.font_leaf.translated_color_layers :: child_layers, delta :: call
    if format == 16 or format == 17 or format == 18 or format == 19 or format == 20 or format == 21 or format == 22 or format == 23:
        let child_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 1 :: call)
        let mut scale_x_fixed = arcana_text.font_leaf.fixed_16_16_identity :: :: call
        let mut scale_y_fixed = arcana_text.font_leaf.fixed_16_16_identity :: :: call
        let mut center = (0, 0)
        let mut has_center = false
        if format == 16 or format == 17:
            scale_x_fixed = arcana_text.font_leaf.fixed_16_16_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 4 :: call) :: call
            scale_y_fixed = arcana_text.font_leaf.fixed_16_16_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 6 :: call) :: call
        else:
            if format == 18 or format == 19:
                scale_x_fixed = arcana_text.font_leaf.fixed_16_16_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 4 :: call) :: call
                scale_y_fixed = arcana_text.font_leaf.fixed_16_16_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 6 :: call) :: call
                let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
                center = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 8 :: call), (i16_be_ref :: face.font_view, paint_offset + 10 :: call)) :: call
                has_center = true
            else:
                if format == 20 or format == 21:
                    let uniform = arcana_text.font_leaf.fixed_16_16_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 4 :: call) :: call
                    scale_x_fixed = uniform
                    scale_y_fixed = uniform
                else:
                    let uniform = arcana_text.font_leaf.fixed_16_16_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 4 :: call) :: call
                    scale_x_fixed = uniform
                    scale_y_fixed = uniform
                    let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
                    center = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 6 :: call), (i16_be_ref :: face.font_view, paint_offset + 8 :: call)) :: call
                    has_center = true
        let matrix = (scale_x_fixed, 0, 0, scale_y_fixed)
        let delta = match has_center:
            true => arcana_text.font_leaf.transformed_center_delta :: center, matrix :: call
            false => (0, 0)
        let child_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child_offset, advance, baseline, line_height, depth + 1) :: call
        return arcana_text.font_leaf.affine_color_layers :: (child_layers, matrix, delta, advance, baseline, line_height) :: call
    if format == 24 or format == 25 or format == 26 or format == 27:
        let child_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 1 :: call)
        let radians = arcana_text.font_leaf.fixed_radians_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 4 :: call) :: call
        let cosine = arcana_text.font_leaf.fixed_cos_pi :: radians :: call
        let sine = arcana_text.font_leaf.fixed_sin_pi :: radians :: call
        let matrix = (cosine, 0 - sine, sine, cosine)
        let mut delta = (0, 0)
        if format == 26 or format == 27:
            let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
            let center = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 6 :: call), (i16_be_ref :: face.font_view, paint_offset + 8 :: call)) :: call
            delta = arcana_text.font_leaf.transformed_center_delta :: center, matrix :: call
        let child_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child_offset, advance, baseline, line_height, depth + 1) :: call
        return arcana_text.font_leaf.affine_color_layers :: (child_layers, matrix, delta, advance, baseline, line_height) :: call
    if format == 28 or format == 29 or format == 30 or format == 31:
        let child_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 1 :: call)
        let x_tan = arcana_text.font_leaf.fixed_tan_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 4 :: call) :: call
        let y_tan = arcana_text.font_leaf.fixed_tan_from_f2dot14 :: (i16_be_ref :: face.font_view, paint_offset + 6 :: call) :: call
        let matrix = ((arcana_text.font_leaf.fixed_16_16_identity :: :: call), (0 - x_tan), y_tan, (arcana_text.font_leaf.fixed_16_16_identity :: :: call))
        let mut delta = (0, 0)
        if format == 30 or format == 31:
            let ctx = arcana_text.font_leaf.scale_context :: face, spec.traits, spec.font_size :: call
            let center = scaled_point :: ctx, ((i16_be_ref :: face.font_view, paint_offset + 8 :: call), (i16_be_ref :: face.font_view, paint_offset + 10 :: call)) :: call
            delta = arcana_text.font_leaf.transformed_center_delta :: center, matrix :: call
        let child_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, child_offset, advance, baseline, line_height, depth + 1) :: call
        return arcana_text.font_leaf.affine_color_layers :: (child_layers, matrix, delta, advance, baseline, line_height) :: call
    if format == 32:
        let source_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 1 :: call)
        let composite_mode = byte_at_or_zero_ref :: face.font_view, paint_offset + 4 :: call
        let backdrop_offset = paint_offset + (u24_be_ref :: face.font_view, paint_offset + 5 :: call)
        let source_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, source_offset, advance, baseline, line_height, depth + 1) :: call
        let backdrop_layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, backdrop_offset, advance, baseline, line_height, depth + 1) :: call
        let source_bitmap = arcana_text.font_leaf.compose_color_layers :: (source_layers, advance, baseline, line_height) :: call
        let backdrop_bitmap = arcana_text.font_leaf.compose_color_layers :: (backdrop_layers, advance, baseline, line_height) :: call
        let composed = arcana_text.font_leaf.composite_color_bitmaps :: (backdrop_bitmap, source_bitmap, composite_mode, advance, baseline, line_height) :: call
        if not composed.empty:
            out :: (arcana_text.font_leaf.ColorLayerBitmap :: bitmap = composed, color = (0, 0, 0, 0) :: call) :: push
        return out
    return out

fn render_color_bitmap(edit face: arcana_text.font_leaf.FontFaceState, read payload: (arcana_text.font_leaf.GlyphRenderSpec, Int, Int, Int, Int)) -> arcana_text.font_leaf.GlyphBitmap:
    let spec = payload.0
    let glyph_index = payload.1
    let advance = payload.2
    let baseline = payload.3
    let line_height = payload.4
    if (arcana_text.font_leaf.colr_version :: face :: call) == 1:
        let paint_offset = arcana_text.font_leaf.colr_v1_base_paint_offset :: face, glyph_index :: call
        if paint_offset >= 0:
            let layers = arcana_text.font_leaf.colr_v1_render_paint :: face, (spec, paint_offset, advance, baseline, line_height, 0) :: call
            let composed = arcana_text.font_leaf.compose_color_layers :: (layers, advance, baseline, line_height) :: call
            if not composed.empty:
                return composed
    let layers = arcana_text.font_leaf.colr_layers_for_glyph :: face, glyph_index :: call
    if (layers :: :: len) > 0:
        let mut rendered_layers = std.collections.list.new[arcana_text.font_leaf.ColorLayerBitmap] :: :: call
        let mut min_x = 0
        let mut min_y = 0
        let mut max_x = 0
        let mut max_y = 0
        let mut seen = false
        for layer in layers:
            let mut layer_spec = spec
            layer_spec.glyph_index = layer.glyph_index
            layer_spec.mode = arcana_text.types.RasterMode.Alpha :: :: call
            layer_spec.color = 0
            let layer_bitmap = arcana_text.font_leaf.render_glyph :: face, layer_spec :: call
            if layer_bitmap.empty or layer_bitmap.size.0 <= 0 or layer_bitmap.size.1 <= 0:
                continue
            let color = arcana_text.font_leaf.cpal_color_rgba :: face, layer.palette_index, spec.color :: call
            if color.3 <= 0:
                continue
            let right = layer_bitmap.offset.0 + layer_bitmap.size.0
            let bottom = layer_bitmap.offset.1 + layer_bitmap.size.1
            if not seen:
                min_x = layer_bitmap.offset.0
                min_y = layer_bitmap.offset.1
                max_x = right
                max_y = bottom
                seen = true
            else:
                min_x = min_int :: min_x, layer_bitmap.offset.0 :: call
                min_y = min_int :: min_y, layer_bitmap.offset.1 :: call
                max_x = max_int :: max_x, right :: call
                max_y = max_int :: max_y, bottom :: call
            let layer_value = arcana_text.font_leaf.ColorLayerBitmap :: bitmap = layer_bitmap, color = color :: call
            rendered_layers :: layer_value :: push
        if seen:
            return arcana_text.font_leaf.compose_color_layers :: (rendered_layers, advance, baseline, line_height) :: call
    let sbix = arcana_text.font_leaf.render_sbix_bitmap :: face, (spec, glyph_index, advance, baseline, line_height) :: call
    if not sbix.empty:
        return sbix
    let cbdt = arcana_text.font_leaf.render_cbdt_bitmap :: face, (spec, glyph_index, advance, baseline, line_height) :: call
    if not cbdt.empty:
        return cbdt
    let svg = arcana_text.font_leaf.render_svg_bitmap :: face, (spec, glyph_index, advance, baseline, line_height) :: call
    if not svg.empty:
        return svg
    return arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call

export fn render_glyph(edit face: arcana_text.font_leaf.FontFaceState, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    font_leaf_probe_append :: ("render_glyph:start glyph=" + (std.text.from_int :: spec.glyph_index :: call) + " text=" + spec.text) :: call
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
    let mut glyph_index = spec.glyph_index
    if glyph_index <= 0:
        glyph_index = actual_glyph_index :: face, glyph_index, ch :: call
    let mut key_request = arcana_text.font_leaf.RasterKeyRequest :: glyph_index = glyph_index, font_size = font_size, traits = traits :: call
    key_request.feature_signature = spec.feature_signature
    key_request.axis_signature = spec.axis_signature
    key_request.vertical = spec.vertical
    key_request.mode = spec.mode
    key_request.color = spec.color
    key_request.hinting = spec.hinting
    let key = raster_key :: key_request :: call
    if face.bitmap_cache :: key :: has:
        font_leaf_probe_append :: "render_glyph:cache_hit" :: call
        return face.bitmap_cache :: key :: get
    if glyph_index <= 0 or ch == "\n" or ch == "\r":
        let advance = scaled_advance :: face, traits, (font_size, 0) :: call
        let empty = arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
        face.bitmap_cache :: key, empty :: set
        font_leaf_probe_append :: "render_glyph:empty_input" :: call
        return face.bitmap_cache :: key :: get
    let raw_advance = match spec.vertical:
        true => vertical_advance_for :: face, glyph_index :: call
        false => advance_width_for :: face, glyph_index :: call
    let mut advance = match spec.vertical:
        true => scaled_vertical_advance :: face, font_size, raw_advance :: call
        false => scaled_advance :: face, traits, (font_size, raw_advance) :: call
    if spec.mode == (arcana_text.types.RasterMode.Color :: :: call):
        let hinted_advance = arcana_text.font_leaf.hinted_outline_advance_for :: face, (glyph_index, traits, font_size, spec.vertical, spec.hinting) :: call
        if hinted_advance > 0:
            advance = hinted_advance
        let colored = arcana_text.font_leaf.render_color_bitmap :: face, (spec, glyph_index, advance, baseline, line_height) :: call
        if not colored.empty:
            face.bitmap_cache :: key, colored :: set
            font_leaf_probe_append :: "render_glyph:color_ready" :: call
            return face.bitmap_cache :: key :: get
    let scale = scale_context :: face, traits, font_size :: call
    let outline = load_outline_recursive :: face, (glyph_index, 0), traits :: call
    font_leaf_probe_append :: ("render_glyph:outline bounds=" + (std.text.from_int :: outline.x_min :: call) + "," + (std.text.from_int :: outline.y_min :: call) + " -> " + (std.text.from_int :: outline.x_max :: call) + "," + (std.text.from_int :: outline.y_max :: call) + " advance=" + (std.text.from_int :: outline.advance_width :: call)) :: call
    if outline.empty:
        let empty = arcana_text.font_leaf.empty_bitmap_metrics :: advance, baseline, line_height :: call
        face.bitmap_cache :: key, empty :: set
        font_leaf_probe_append :: "render_glyph:outline_empty" :: call
        return face.bitmap_cache :: key :: get
    let scaled_min = scaled_point :: scale, (outline.x_min, outline.y_min) :: call
    let scaled_max = scaled_point :: scale, (outline.x_max, outline.y_max) :: call
    let min_x = min_int :: scaled_min.0, scaled_max.0 :: call
    let min_y = min_int :: scaled_min.1, scaled_max.1 :: call
    let max_x = max_int :: scaled_min.0, scaled_max.0 :: call
    let max_y = max_int :: scaled_min.1, scaled_max.1 :: call
    let mut left = min_x
    let mut top = baseline - max_y
    let mut width = max_int :: (max_x - min_x + 1), 1 :: call
    let mut height = max_int :: (max_y - min_y + 1), 1 :: call
    if spec.vertical:
        left = 0 - (width / 2)
        top = scale_y :: (top_side_bearing_for :: face, glyph_index :: call), font_size, face.units_per_em :: call
        if top == 0:
            top = min_y
        let hinted = arcana_text.font_leaf.hinted_vertical_metrics :: (face, outline, glyph_index, font_size, top, height, advance, spec.hinting) :: call
        top = hinted.0
        height = hinted.1
        advance = hinted.2
    else:
        let hinted = arcana_text.font_leaf.hinted_horizontal_metrics :: (face, outline, traits, font_size, left, width, advance, spec.hinting) :: call
        left = hinted.0
        width = hinted.1
        advance = hinted.2
    font_leaf_probe_append :: ("render_glyph:bitmap dims=" + (std.text.from_int :: width :: call) + "x" + (std.text.from_int :: height :: call) + " left=" + (std.text.from_int :: left :: call) + " top=" + (std.text.from_int :: top :: call)) :: call
    let mut translated = empty_segments :: :: call
    let raster_segments = outline_segments :: outline, scale :: call
    font_leaf_probe_append :: ("render_glyph:raster_segments=" + (std.text.from_int :: (raster_segments :: :: len) :: call)) :: call
    for segment in raster_segments:
        translated :: (line_segment :: (segment.start.0 - left, max_y - segment.start.1), (segment.end.0 - left, max_y - segment.end.1) :: call) :: push
    font_leaf_probe_append :: ("render_glyph:translated_segments=" + (std.text.from_int :: (translated :: :: len) :: call)) :: call
    let oversample = arcana_text.font_leaf.hinted_oversample_scale :: font_size, spec.mode, spec.hinting :: call
    let mut lcd = empty_lcd :: :: call
    let mut base_alpha = empty_alpha :: :: call
    if oversample <= 1:
        base_alpha = arcana_text.font_leaf.fill_bitmap_binary_from_segments :: translated, width, height :: call
    else:
        let scaled_width = width * oversample
        let scaled_height = height * oversample
        let scaled_segments = arcana_text.font_leaf.scale_segments_for_raster :: translated, oversample :: call
        let scaled_alpha = arcana_text.font_leaf.fill_bitmap_binary_from_segments :: scaled_segments, scaled_width, scaled_height :: call
        base_alpha = arcana_text.font_leaf.downsample_alpha :: (scaled_alpha, (scaled_width, scaled_height), (width, height), oversample) :: call
        if spec.mode == (arcana_text.types.RasterMode.Lcd :: :: call):
            lcd = arcana_text.font_leaf.lcd_from_scaled_alpha :: (scaled_alpha, (scaled_width, scaled_height), (width, height), oversample) :: call
    if spec.hinting == (arcana_text.types.Hinting.Enabled :: :: call):
        base_alpha = arcana_text.font_leaf.filtered_alpha :: base_alpha, (width, height) :: call
    base_alpha = arcana_text.font_leaf.boosted_alpha :: base_alpha, font_size, spec.mode :: call
    if (lcd :: :: len) > 0:
        lcd = arcana_text.font_leaf.boosted_lcd :: lcd, font_size :: call
        lcd = arcana_text.font_leaf.filtered_lcd :: lcd, (width, height) :: call
    font_leaf_probe_append :: ("render_glyph:base_alpha=" + (std.text.from_int :: (base_alpha :: :: len) :: call)) :: call
    let mut embolden_px = 0
    if not (face_has_variation_axis :: face, "wght" :: call):
        embolden_px = max_int :: ((traits.weight - 400) / 250), 0 :: call
    let alpha = embolden_alpha :: base_alpha, (width, height), embolden_px :: call
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (width, height), offset = (left, top), advance = advance :: call
    out.baseline = baseline
    out.line_height = line_height
    out.empty = false
    out.alpha = alpha
    out.lcd = lcd
    out.rgba = empty_rgba :: :: call
    face.bitmap_cache :: key, out :: set
    font_leaf_probe_append :: ("render_glyph:done size=" + (std.text.from_int :: width :: call) + "x" + (std.text.from_int :: height :: call)) :: call
    return face.bitmap_cache :: key :: get

export fn measure_glyph(edit face: arcana_text.font_leaf.FontFaceState, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    font_leaf_probe_append :: ("measure_glyph:start glyph=" + (std.text.from_int :: spec.glyph_index :: call) + " text=" + spec.text) :: call
    let mut traits = spec.traits
    if traits.weight <= 0:
        traits.weight = face.weight
    if traits.width_milli <= 0:
        traits.width_milli = face.width_milli
    if traits.slant_milli == 0:
        traits.slant_milli = face.slant_milli
    let line_height = line_height_for :: face, spec.font_size, spec.line_height_milli :: call
    let baseline = baseline_for :: face, spec.font_size, spec.line_height_milli :: call
    font_leaf_probe_append :: "measure_glyph:line_metrics" :: call
    let mut glyph_index = spec.glyph_index
    if glyph_index <= 0:
        glyph_index = actual_glyph_index :: face, glyph_index, spec.text :: call
    font_leaf_probe_append :: ("measure_glyph:resolved " + (std.text.from_int :: glyph_index :: call)) :: call
    let mut raw_advance = 0
    if glyph_index > 0:
        font_leaf_probe_append :: "measure_glyph:advance_start" :: call
        raw_advance = match spec.vertical:
            true => vertical_advance_for :: face, glyph_index :: call
            false => advance_width_for :: face, glyph_index :: call
        font_leaf_probe_append :: "measure_glyph:advance_done" :: call
    font_leaf_probe_append :: ("measure_glyph:raw_advance " + (std.text.from_int :: raw_advance :: call)) :: call
    let mut advance = match spec.vertical:
        true => scaled_vertical_advance :: face, spec.font_size, raw_advance :: call
        false => scaled_advance :: face, traits, (spec.font_size, raw_advance) :: call
    let hinted_advance = arcana_text.font_leaf.hinted_outline_advance_for :: face, (glyph_index, traits, spec.font_size, spec.vertical, spec.hinting) :: call
    if hinted_advance > 0:
        advance = hinted_advance
    let mut offset = (0, 0)
    let mut size = (0, 0)
    let mut empty = true
    if glyph_index > 0:
        let outline = load_outline_recursive :: face, (glyph_index, 0), traits :: call
        if not outline.empty:
            let scale = scale_context :: face, traits, spec.font_size :: call
            let scaled_min = scaled_point :: scale, (outline.x_min, outline.y_min) :: call
            let scaled_max = scaled_point :: scale, (outline.x_max, outline.y_max) :: call
            let min_x = min_int :: scaled_min.0, scaled_max.0 :: call
            let min_y = min_int :: scaled_min.1, scaled_max.1 :: call
            let max_x = max_int :: scaled_min.0, scaled_max.0 :: call
            let max_y = max_int :: scaled_min.1, scaled_max.1 :: call
            let mut left = min_x
            let mut top = baseline - max_y
            let mut width = max_int :: (max_x - min_x + 1), 1 :: call
            let mut height = max_int :: (max_y - min_y + 1), 1 :: call
            if spec.vertical:
                left = 0 - (width / 2)
                top = scale_y :: (top_side_bearing_for :: face, glyph_index :: call), spec.font_size, face.units_per_em :: call
                if top == 0:
                    top = min_y
                let hinted = arcana_text.font_leaf.hinted_vertical_metrics :: (face, outline, glyph_index, spec.font_size, top, height, advance, spec.hinting) :: call
                top = hinted.0
                height = hinted.1
                advance = hinted.2
            else:
                let hinted = arcana_text.font_leaf.hinted_horizontal_metrics :: (face, outline, traits, spec.font_size, left, width, advance, spec.hinting) :: call
                left = hinted.0
                width = hinted.1
                advance = hinted.2
            offset = (left, top)
            size = (width, height)
            empty = false
    let mut bitmap = arcana_text.font_leaf.GlyphBitmap :: size = size, offset = offset, advance = advance :: call
    bitmap.baseline = baseline
    bitmap.line_height = line_height
    bitmap.empty = empty
    bitmap.alpha = empty_alpha :: :: call
    bitmap.lcd = empty_lcd :: :: call
    bitmap.rgba = empty_rgba :: :: call
    font_leaf_probe_append :: "measure_glyph:bitmap_ready" :: call
    font_leaf_probe_append :: ("measure_glyph:done advance=" + (std.text.from_int :: bitmap.advance :: call)) :: call
    return bitmap

export fn advance_for_glyph(edit face: arcana_text.font_leaf.FontFaceState, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> Int:
    font_leaf_probe_append :: ("advance_for_glyph:start glyph=" + (std.text.from_int :: spec.glyph_index :: call) + " text=" + spec.text) :: call
    let mut traits = spec.traits
    if traits.weight <= 0:
        traits.weight = face.weight
    if traits.width_milli <= 0:
        traits.width_milli = face.width_milli
    if traits.slant_milli == 0:
        traits.slant_milli = face.slant_milli
    let mut glyph_index = spec.glyph_index
    if glyph_index <= 0:
        glyph_index = actual_glyph_index :: face, glyph_index, spec.text :: call
    font_leaf_probe_append :: ("advance_for_glyph:resolved " + (std.text.from_int :: glyph_index :: call)) :: call
    let mut raw_advance = 0
    if glyph_index > 0:
        font_leaf_probe_append :: "advance_for_glyph:raw_start" :: call
        raw_advance = match spec.vertical:
            true => vertical_advance_for :: face, glyph_index :: call
            false => advance_width_for :: face, glyph_index :: call
        font_leaf_probe_append :: "advance_for_glyph:raw_done" :: call
    let mut advance = match spec.vertical:
        true => scaled_vertical_advance :: face, spec.font_size, raw_advance :: call
        false => scaled_advance :: face, traits, (spec.font_size, raw_advance) :: call
    let hinted_advance = arcana_text.font_leaf.hinted_outline_advance_for :: face, (glyph_index, traits, spec.font_size, spec.vertical, spec.hinting) :: call
    if hinted_advance > 0:
        advance = hinted_advance
    font_leaf_probe_append :: ("advance_for_glyph:done " + (std.text.from_int :: advance :: call)) :: call
    return advance

export fn line_height_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    return line_height_for :: face, font_size, line_height_milli :: call

export fn baseline_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int, line_height_milli: Int) -> Int:
    return baseline_for :: face, font_size, line_height_milli :: call

export fn underline_metrics_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int) -> (Int, Int):
    let mut offset = scale_y :: face.underline_position, font_size, face.units_per_em :: call
    let thickness = max_int :: (abs_int :: (scale_y :: face.underline_thickness, font_size, face.units_per_em :: call) :: call), 1 :: call
    if offset == 0:
        offset = 1 - thickness
    return (offset, thickness)

export fn strikethrough_metrics_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int) -> (Int, Int):
    let mut offset = scale_y :: face.strike_position, font_size, face.units_per_em :: call
    let thickness = max_int :: (abs_int :: (scale_y :: face.strike_thickness, font_size, face.units_per_em :: call) :: call), 1 :: call
    if offset == 0:
        offset = scale_y :: (face.ascender / 3), font_size, face.units_per_em :: call
    return (offset, thickness)

export fn overline_metrics_for_face(read face: arcana_text.font_leaf.FontFaceState, font_size: Int) -> (Int, Int):
    let thickness = max_int :: (abs_int :: (scale_y :: face.underline_thickness, font_size, face.units_per_em :: call) :: call), 1 :: call
    let offset = scale_y :: face.ascender, font_size, face.units_per_em :: call
    return (offset, thickness)

fn load_face_from_parts(family_name: Str, read meta: arcana_text.font_leaf.FaceLoadMeta, read bytes_view: std.memory.ByteView) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    font_leaf_probe_append :: "load_face_from_parts:start" :: call
    let source_label = meta.source_label
    let source_path = meta.source_path
    let traits = meta.traits
    let tables_result = parse_table_directory_ref :: bytes_view :: call
    font_leaf_probe_append :: "load_face_from_parts:table_directory" :: call
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
    font_leaf_probe_append :: "load_face_from_parts:required_tables" :: call
    let mut gdef = (-1, 0)
    if tables :: "GDEF" :: has:
        gdef = tables :: "GDEF" :: get
    let mut colr = (-1, 0)
    if tables :: "COLR" :: has:
        colr = tables :: "COLR" :: get
    let mut cpal = (-1, 0)
    if tables :: "CPAL" :: has:
        cpal = tables :: "CPAL" :: get
    let mut cblc = (-1, 0)
    if tables :: "CBLC" :: has:
        cblc = tables :: "CBLC" :: get
    let mut cbdt = (-1, 0)
    if tables :: "CBDT" :: has:
        cbdt = tables :: "CBDT" :: get
    let mut sbix = (-1, 0)
    if tables :: "sbix" :: has:
        sbix = tables :: "sbix" :: get
    let mut svg = (-1, 0)
    if tables :: "SVG " :: has:
        svg = tables :: "SVG " :: get
    let mut gsub = (-1, 0)
    if tables :: "GSUB" :: has:
        gsub = tables :: "GSUB" :: get
    let mut gpos = (-1, 0)
    if tables :: "GPOS" :: has:
        gpos = tables :: "GPOS" :: get
    let mut kern = (-1, 0)
    if tables :: "kern" :: has:
        kern = tables :: "kern" :: get
    let mut vhea = (-1, 0)
    if tables :: "vhea" :: has:
        vhea = tables :: "vhea" :: get
    let mut vmtx = (-1, 0)
    if tables :: "vmtx" :: has:
        vmtx = tables :: "vmtx" :: get
    let mut os2 = (-1, 0)
    if tables :: "OS/2" :: has:
        os2 = tables :: "OS/2" :: get
    let mut post = (-1, 0)
    if tables :: "post" :: has:
        post = tables :: "post" :: get
    let mut fvar = (-1, 0)
    if tables :: "fvar" :: has:
        fvar = tables :: "fvar" :: get
    let mut gvar = (-1, 0)
    if tables :: "gvar" :: has:
        gvar = tables :: "gvar" :: get
    let glyph_count = u16_be_ref :: bytes_view, maxp.0 + 4 :: call
    let hmetric_count = u16_be_ref :: bytes_view, hhea.0 + 34 :: call
    let mut vmetric_count = 0
    if vhea.0 >= 0 and vhea.1 >= 36 and vhea.0 + 36 <= (bytes_view :: :: len):
        vmetric_count = u16_be_ref :: bytes_view, vhea.0 + 34 :: call
    let cmap_offsets = detect_cmap_offsets_ref :: bytes_view, cmap :: call
    font_leaf_probe_append :: "load_face_from_parts:cmap_offsets" :: call
    let variation_axes = parse_variation_axes :: bytes_view, fvar :: call
    font_leaf_probe_append :: "load_face_from_parts:variation_axes" :: call
    let mut gvar_axis_count = 0
    let mut gvar_shared_tuple_count = 0
    let mut gvar_shared_tuples_offset = 0
    let mut gvar_glyph_count = 0
    let mut gvar_flags = 0
    let mut gvar_data_offset = 0
    if gvar.0 >= 0 and gvar.1 >= 20 and gvar.0 + gvar.1 <= (bytes_view :: :: len):
        gvar_axis_count = u16_be_ref :: bytes_view, gvar.0 + 4 :: call
        gvar_shared_tuple_count = u16_be_ref :: bytes_view, gvar.0 + 6 :: call
        gvar_shared_tuples_offset = gvar.0 + (u32_be_ref :: bytes_view, gvar.0 + 8 :: call)
        gvar_glyph_count = u16_be_ref :: bytes_view, gvar.0 + 12 :: call
        gvar_flags = u16_be_ref :: bytes_view, gvar.0 + 14 :: call
        gvar_data_offset = gvar.0 + (u32_be_ref :: bytes_view, gvar.0 + 16 :: call)
        let gvar_end = gvar.0 + gvar.1
        if gvar_shared_tuples_offset < gvar.0 or gvar_shared_tuples_offset > gvar_end:
            gvar_shared_tuples_offset = 0
            gvar_data_offset = 0
            gvar_axis_count = 0
        if gvar_data_offset < gvar.0 or gvar_data_offset > gvar_end:
            gvar_shared_tuples_offset = 0
            gvar_data_offset = 0
            gvar_axis_count = 0
    let mut loaded_family_name = arcana_text.font_leaf.normalize_family_name :: family_name :: call
    let mut face_name = "Regular"
    let mut full_name = source_label
    let mut postscript_name = source_label
    let need_name_table = loaded_family_name == "" or source_label == ""
    if need_name_table:
        if tables :: "name" :: has:
            let name_table = tables :: "name" :: get
            font_leaf_probe_append :: "load_face_from_parts:name_typographic_family" :: call
            let typographic_family = arcana_text.font_leaf.name_string_for_id :: bytes_view, name_table, 16 :: call
            font_leaf_probe_append :: "load_face_from_parts:name_family" :: call
            let family = arcana_text.font_leaf.name_string_for_id :: bytes_view, name_table, 1 :: call
            font_leaf_probe_append :: "load_face_from_parts:name_typographic_subfamily" :: call
            let typographic_subfamily = arcana_text.font_leaf.name_string_for_id :: bytes_view, name_table, 17 :: call
            font_leaf_probe_append :: "load_face_from_parts:name_subfamily" :: call
            let subfamily = arcana_text.font_leaf.name_string_for_id :: bytes_view, name_table, 2 :: call
            font_leaf_probe_append :: "load_face_from_parts:name_full" :: call
            let full = arcana_text.font_leaf.name_string_for_id :: bytes_view, name_table, 4 :: call
            font_leaf_probe_append :: "load_face_from_parts:name_postscript" :: call
            let postscript = arcana_text.font_leaf.name_string_for_id :: bytes_view, name_table, 6 :: call
            if typographic_family != "":
                loaded_family_name = typographic_family
            else:
                if family != "":
                    loaded_family_name = family
            loaded_family_name = arcana_text.font_leaf.normalize_family_name :: loaded_family_name :: call
            if typographic_subfamily != "":
                face_name = typographic_subfamily
            else:
                if subfamily != "":
                    face_name = subfamily
            if full != "":
                full_name = full
            if postscript != "":
                postscript_name = postscript
    font_leaf_probe_append :: "load_face_from_parts:name_table" :: call
    if loaded_family_name == "":
        loaded_family_name = source_label
    loaded_family_name = arcana_text.font_leaf.normalize_family_name :: loaded_family_name :: call
    if full_name == "":
        if face_name == "" or face_name == "Regular":
            full_name = loaded_family_name
        else:
            full_name = loaded_family_name + " " + face_name
    if postscript_name == "":
        postscript_name = full_name
    let mut face = arcana_text.font_leaf.FontFaceState :: family_name = family_name, source_label = source_label, source_path = source_path :: call
    face.family_name = loaded_family_name
    face.face_name = face_name
    face.full_name = full_name
    face.postscript_name = postscript_name
    face.weight = traits.weight
    face.width_milli = traits.width_milli
    face.slant_milli = traits.slant_milli
    face.units_per_em = u16_be_ref :: bytes_view, head.0 + 18 :: call
    face.ascender = i16_be_ref :: bytes_view, hhea.0 + 4 :: call
    face.descender = i16_be_ref :: bytes_view, hhea.0 + 6 :: call
    face.line_gap = i16_be_ref :: bytes_view, hhea.0 + 8 :: call
    face.underline_position = 0 - max_int :: (face.units_per_em / 12), 1 :: call
    face.underline_thickness = max_int :: (face.units_per_em / 18), 1 :: call
    face.strike_position = max_int :: (face.ascender / 3), 1 :: call
    face.strike_thickness = face.underline_thickness
    if post.0 >= 0 and post.1 >= 12 and post.0 + 12 <= (bytes_view :: :: len):
        face.underline_position = i16_be_ref :: bytes_view, post.0 + 8 :: call
        face.underline_thickness = max_int :: (abs_int :: (i16_be_ref :: bytes_view, post.0 + 10 :: call) :: call), 1 :: call
    if os2.0 >= 0 and os2.1 >= 30 and os2.0 + 30 <= (bytes_view :: :: len):
        face.strike_position = i16_be_ref :: bytes_view, os2.0 + 26 :: call
        face.strike_thickness = max_int :: (abs_int :: (i16_be_ref :: bytes_view, os2.0 + 28 :: call) :: call), 1 :: call
    face.glyph_count = glyph_count
    face.font_view = bytes_view
    face.glyf_offset = glyf.0
    face.loca_offset = loca.0
    face.loca_format = i16_be_ref :: bytes_view, head.0 + 50 :: call
    face.hmtx_offset = hmtx.0
    face.hmetric_count = hmetric_count
    face.advance_widths = empty_alpha :: :: call
    face.left_side_bearings = empty_alpha :: :: call
    face.vhea_offset = vhea.0
    face.vhea_length = vhea.1
    face.vmtx_offset = vmtx.0
    face.vmtx_length = vmtx.1
    face.vmetric_count = vmetric_count
    face.vertical_advances = empty_alpha :: :: call
    face.top_side_bearings = empty_alpha :: :: call
    face.loca_offsets = empty_alpha :: :: call
    face.glyph_index_cache = empty_int_map :: :: call
    face.cmap_table_offset = cmap.0
    face.cmap_table_length = cmap.1
    face.cmap4_offset = cmap_offsets.0
    face.cmap12_offset = cmap_offsets.1
    face.cmap4_segments = empty_cmap4_segments :: :: call
    face.cmap4_glyphs = empty_alpha :: :: call
    face.cmap12_groups = empty_cmap12_groups :: :: call
    face.colr_offset = colr.0
    face.colr_length = colr.1
    face.cpal_offset = cpal.0
    face.cpal_length = cpal.1
    face.cblc_offset = cblc.0
    face.cblc_length = cblc.1
    face.cbdt_offset = cbdt.0
    face.cbdt_length = cbdt.1
    if sbix.0 >= 0 and sbix.1 >= 4 and sbix.0 + 4 <= (bytes_view :: :: len):
        face.sbix_flags = u16_be_ref :: bytes_view, sbix.0 + 2 :: call
    else:
        face.sbix_flags = 0
    face.sbix_offset = sbix.0
    face.sbix_length = sbix.1
    face.svg_offset = svg.0
    face.svg_length = svg.1
    face.gdef_offset = gdef.0
    face.gdef_length = gdef.1
    face.gsub_offset = gsub.0
    face.gsub_length = gsub.1
    face.gpos_offset = gpos.0
    face.gpos_length = gpos.1
    face.kern_offset = kern.0
    face.kern_length = kern.1
    face.gsub_lookup_cache = empty_lookup_ref_cache :: :: call
    face.gsub_candidate_cache = empty_bool_cache :: :: call
    face.gpos_lookup_cache = empty_lookup_ref_cache :: :: call
    face.gpos_single_cache = empty_pair_adjust_cache :: :: call
    face.gpos_pair_cache = empty_pair_adjust_cache :: :: call
    face.kern_pair_cache = empty_pair_adjust_cache :: :: call
    face.variation_axes = variation_axes
    face.fvar_offset = fvar.0
    face.fvar_length = fvar.1
    face.gvar_offset = gvar.0
    face.gvar_length = gvar.1
    face.gvar_axis_count = gvar_axis_count
    face.gvar_shared_tuple_count = gvar_shared_tuple_count
    face.gvar_shared_tuples_offset = gvar_shared_tuples_offset
    face.gvar_glyph_count = gvar_glyph_count
    face.gvar_flags = gvar_flags
    face.gvar_data_offset = gvar_data_offset
    face.bitmap_cache = std.collections.map.new[Str, arcana_text.font_leaf.GlyphBitmap] :: :: call
    font_leaf_probe_append :: "load_face_from_parts:done" :: call
    return Result.Ok[arcana_text.font_leaf.FontFaceState, Str] :: face :: call

export fn load_face_from_view(read request: arcana_text.font_leaf.FaceLoadRequest, read bytes_view: std.memory.ByteView) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    font_leaf_probe_append :: ("load_face_from_view:start face=" + (std.text.from_int :: request.face_index :: call)) :: call
    let source_view = bytes_view
    let face_view_result = arcana_text.font_leaf.face_view_from_source :: source_view, request.face_index :: call
    if face_view_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: face_view_result, "failed to resolve face view" :: call) :: call
    let face_view = face_view_result :: source_view :: unwrap_or
    let meta = face_load_meta_from_request :: request :: call
    font_leaf_probe_append :: "load_face_from_view:resolved_view" :: call
    let loaded = load_face_from_parts :: request.family_name, meta, face_view :: call
    if loaded :: :: is_ok:
        font_leaf_probe_append :: "load_face_from_view:done" :: call
    else:
        font_leaf_probe_append :: ("load_face_from_view:error " + (result_err_or :: loaded, "load face error" :: call)) :: call
    return loaded

export fn load_face_state_from_request_view(read request: arcana_text.font_leaf.FaceLoadRequest, read bytes_view: std.memory.ByteView) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    return arcana_text.font_leaf.load_face_from_view :: request, bytes_view :: call

export fn load_face_state_from_request_bytes(take request: arcana_text.font_leaf.FaceLoadRequest) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    let face_index = request.face_index
    let family_name = request.family_name
    let source_label = request.source_label
    let source_path = request.source_path
    let traits = request.traits
    let source_bytes = request.source_bytes
    font_leaf_probe_append :: ("load_face_state_from_request_bytes:start face=" + (std.text.from_int :: face_index :: call) + " bytes=" + (std.text.from_int :: (source_bytes :: :: len) :: call)) :: call
    if (source_bytes :: :: len) <= 0:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: "font source bytes are empty" :: call
    let source_view = std.memory.bytes_view :: source_bytes, 0, (source_bytes :: :: len) :: call
    let mut face_view = source_view
    if face_index != 0:
        let face_view_result = arcana_text.font_leaf.face_view_from_source :: source_view, face_index :: call
        if face_view_result :: :: is_err:
            return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: face_view_result, "failed to resolve face view" :: call) :: call
        face_view = face_view_result :: source_view :: unwrap_or
    let mut meta = face_load_meta :: family_name, source_label, source_path :: call
    meta.traits = traits
    font_leaf_probe_append :: "load_face_state_from_request_bytes:resolved_view" :: call
    let loaded = load_face_from_parts :: family_name, meta, face_view :: call
    return match loaded:
        Result.Ok(value) => load_face_state_from_request_bytes_ok :: value :: call
        Result.Err(err) => load_face_state_from_request_bytes_err :: err :: call

fn load_face_state_from_request_bytes_ok(take value: arcana_text.font_leaf.FontFaceState) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    font_leaf_probe_append :: "load_face_state_from_request_bytes:done" :: call
    return Result.Ok[arcana_text.font_leaf.FontFaceState, Str] :: value :: call

fn load_face_state_from_request_bytes_err(err: Str) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    font_leaf_probe_append :: ("load_face_state_from_request_bytes:error " + err) :: call
    return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: err :: call

fn load_face_state_into_slab_ok(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], take value: arcana_text.font_leaf.FontFaceState) -> Result[std.memory.SlabId[arcana_text.font_leaf.FontFaceState], Str]:
    font_leaf_probe_append :: "load_face_state_into_slab:alloc_start" :: call
    let face_id = std.kernel.memory.slab_alloc[arcana_text.font_leaf.FontFaceState] :: live_faces, value :: call
    font_leaf_probe_append :: "load_face_state_into_slab:alloc_done" :: call
    return Result.Ok[std.memory.SlabId[arcana_text.font_leaf.FontFaceState], Str] :: face_id :: call

export fn load_face_state_into_slab(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], take request: arcana_text.font_leaf.FaceLoadRequest) -> Result[std.memory.SlabId[arcana_text.font_leaf.FontFaceState], Str]:
    let loaded = arcana_text.font_leaf.load_face_state_from_request_bytes :: request :: call
    return match loaded:
        Result.Ok(value) => load_face_state_into_slab_ok :: live_faces, value :: call
        Result.Err(err) => Result.Err[std.memory.SlabId[arcana_text.font_leaf.FontFaceState], Str] :: err :: call

export fn load_face_from_bytes(read request: arcana_text.font_leaf.FaceLoadRequest) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    return arcana_text.font_leaf.load_face_state_from_request_bytes :: request :: call

export fn load_face_from_path(family_name: Str, path: Str, read traits: arcana_text.font_leaf.FaceTraits) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    font_leaf_probe_append :: ("load_face_from_path:start " + path) :: call
    let bytes_result = std.fs.read_bytes :: path :: call
    if bytes_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: bytes_result, "failed to read font file" :: call) :: call
    let bytes = bytes_result :: (empty_alpha :: :: call) :: unwrap_or
    font_leaf_probe_append :: ("load_face_from_path:bytes " + (std.text.from_int :: (bytes :: :: len) :: call)) :: call
    let mut request = arcana_text.font_leaf.face_load_request :: family_name, (safe_file_stem :: path :: call), path :: call
    request.source_bytes = bytes
    request.traits = traits
    let loaded = arcana_text.font_leaf.load_face_state_from_request_bytes :: request :: call
    if loaded :: :: is_ok:
        font_leaf_probe_append :: "load_face_from_path:done" :: call
    else:
        font_leaf_probe_append :: ("load_face_from_path:error " + (result_err_or :: loaded, "path load failure" :: call)) :: call
    return loaded
