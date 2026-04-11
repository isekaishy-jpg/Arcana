import arcana_text.font_leaf
import arcana_text.font_leaf.cmap
import arcana_text.font_leaf.load
import arcana_text.font_leaf.raster
import arcana_text.monaspace
import arcana_text.shape.cache
import arcana_text.shape.types
import arcana_text.text_units
import arcana_text.types
import arcana_winapi.fonts
import std.bytes
import std.collections.array
import std.collections.list
import std.collections.map
import std.fs
import std.memory
import std.option
import std.path
import std.result
import std.text
use std.option.Option
use std.result.Result

record RegisteredFace:
    id: arcana_text.types.FontFaceId
    source: arcana_text.types.FontSource
    traits: arcana_text.font_leaf.FaceTraits
    face_id: Option[std.memory.SlabId[arcana_text.font_leaf.FontFaceState]]
    load_error: Str
    units_per_em: Int
    ascender: Int
    descender: Int
    line_gap: Int

record FontSelectionRequest:
    style: arcana_text.types.TextStyle
    text: Str
    family: Str
    restrict_family: Bool

record GsubUnitsRequest:
    matched: arcana_text.types.FontMatch
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    glyphs: List[Int]

record PairAdjustRequest:
    id: arcana_text.types.FontFaceId
    left_glyph: Int
    right_glyph: Int
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    font_size: Int
    width_milli: Int

record SingleAdjustRequest:
    id: arcana_text.types.FontFaceId
    glyph: Int
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]
    font_size: Int
    width_milli: Int

record PositionLookupsRequest:
    id: arcana_text.types.FontFaceId
    script_tag: Str
    language_tag: Str
    features: List[arcana_text.types.FontFeature]

record LookupTypeRequest:
    id: arcana_text.types.FontFaceId
    lookup: arcana_text.font_leaf.GsubLookupRef

record LookupPlacementRequest:
    id: arcana_text.types.FontFaceId
    lookup: arcana_text.font_leaf.GsubLookupRef
    left_glyph: Int
    right_glyph: Int
    font_size: Int
    width_milli: Int

record GlyphClassRequest:
    id: arcana_text.types.FontFaceId
    glyph_index: Int

record BlobSourceRegistration:
    source: arcana_text.types.FontSource
    source_bytes: Array[Int]
    face_total: Int

record FaceSearchRequest:
    target: arcana_text.font_leaf.FaceTraits
    text: Str
    family: Str
    families: List[Str]
    restrict_family: Bool
    require_text_support: Bool

record ScoredMatch:
    score: Int
    tie: Int
    matched: arcana_text.types.FontMatch

fn empty_pair_placement() -> arcana_text.font_leaf.PairPlacement:
    let mut out = arcana_text.font_leaf.PairPlacement :: x_offset = 0, y_offset = 0, x_advance = 0 :: call
    out.y_advance = 0
    out.zero_advance = false
    out.attach_to_left_origin = false
    return out

fn empty_scored_matches() -> List[arcana_text.fonts.ScoredMatch]:
    return std.collections.list.empty[arcana_text.fonts.ScoredMatch] :: :: call

fn stable_match_tie(read matched: arcana_text.types.FontMatch) -> Int:
    let source = matched.source
    let mut tie = 17
    tie = arcana_text.shape.types.mix_signature_text :: tie, source.family :: call
    tie = arcana_text.shape.types.mix_signature_text :: tie, source.face :: call
    tie = arcana_text.shape.types.mix_signature_text :: tie, source.full_name :: call
    tie = arcana_text.shape.types.mix_signature_text :: tie, source.postscript_name :: call
    tie = arcana_text.shape.types.mix_signature_text :: tie, source.label :: call
    tie = arcana_text.shape.types.mix_signature_text :: tie, source.path :: call
    tie = arcana_text.shape.types.mix_signature :: tie, matched.id.face_index :: call
    return tie

fn same_match(read left: arcana_text.types.FontMatch, read right: arcana_text.types.FontMatch) -> Bool:
    return left.id.source_index == right.id.source_index and left.id.face_index == right.id.face_index

fn fonts_probe_flag_path() -> Str:
    return std.path.join :: (std.path.join :: (std.path.cwd :: :: call), "scratch" :: call), "enable_text_fonts_probe" :: call

fn fonts_probe_log_path() -> Str:
    return std.path.join :: (std.path.join :: (std.path.cwd :: :: call), "scratch" :: call), "text_fonts_probe.log" :: call

fn fonts_probe_enabled() -> Bool:
    return std.fs.is_file :: (fonts_probe_flag_path :: :: call) :: call

fn fonts_probe_append(line: Str):
    if not (fonts_probe_enabled :: :: call):
        return
    let _ = std.fs.mkdir_all :: (std.path.parent :: (fonts_probe_log_path :: :: call) :: call) :: call
    let opened = std.fs.stream_open_write :: (fonts_probe_log_path :: :: call), true :: call
    return match opened:
        Result.Ok(value) => fonts_probe_append_ready :: value, line :: call
        Result.Err(_) => 0

fn fonts_probe_append_ready(take value: std.fs.FileStream, line: Str):
    let mut stream = value
    let bytes = std.bytes.from_str_utf8 :: (line + "\n") :: call
    let _ = std.fs.stream_write :: stream, bytes :: call
    let _ = std.fs.stream_close :: stream :: call

export obj FontSystem:
    source_count: Int
    face_count: Int
    locale: Str
    sources: Map[Int, arcana_text.types.FontSource]
    faces: Map[Str, arcana_text.fonts.RegisteredFace]
    selection_cache: Map[Str, arcana_text.types.FontMatch]
    path_index: Map[Str, Int]
    source_face_counts: Map[Int, Int]
    source_blob_ids: Map[Int, std.memory.SlabId[Array[Int]]]
    source_blobs: std.memory.Slab[Array[Int]]
    live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState]
    shape_cache: arcana_text.shape.cache.ShapeCache
    discovered: Bool

impl arcana_text.types.FontSource:
    fn family_or_label(read self: arcana_text.types.FontSource) -> Str:
        if (std.text.len_bytes :: self.family :: call) > 0:
            return self.family
        return self.label

export fn family_or_label(read source: arcana_text.types.FontSource) -> Str:
    if (std.text.len_bytes :: source.family :: call) > 0:
        return source.family
    return source.label

fn empty_bytes() -> Array[Int]:
    return std.collections.array.new[Int] :: 0, 0 :: call

fn empty_strs() -> List[Str]:
    return std.collections.list.empty[Str] :: :: call

fn default_locale() -> Str:
    return "en-US"

fn empty_sources() -> Map[Int, arcana_text.types.FontSource]:
    return std.collections.map.new[Int, arcana_text.types.FontSource] :: :: call

fn empty_faces() -> Map[Str, arcana_text.fonts.RegisteredFace]:
    return std.collections.map.new[Str, arcana_text.fonts.RegisteredFace] :: :: call

fn empty_selection_cache() -> Map[Str, arcana_text.types.FontMatch]:
    return std.collections.map.new[Str, arcana_text.types.FontMatch] :: :: call

fn empty_path_index() -> Map[Str, Int]:
    return std.collections.map.new[Str, Int] :: :: call

fn empty_source_face_counts() -> Map[Int, Int]:
    return std.collections.map.new[Int, Int] :: :: call

fn empty_source_blob_ids() -> Map[Int, std.memory.SlabId[Array[Int]]]:
    return std.collections.map.new[Int, std.memory.SlabId[Array[Int]]] :: :: call

fn empty_source_blobs() -> std.memory.Slab[Array[Int]]:
    return std.memory.slab_new[Array[Int]] :: 64 :: call

fn empty_live_faces() -> std.memory.Slab[arcana_text.font_leaf.FontFaceState]:
    return std.memory.slab_new[arcana_text.font_leaf.FontFaceState] :: 64 :: call

fn invalid_face_id() -> arcana_text.types.FontFaceId:
    return arcana_text.types.FontFaceId :: source_index = -1, face_index = -1 :: call

fn face_id(source_index: Int, face_index: Int) -> arcana_text.types.FontFaceId:
    return arcana_text.types.FontFaceId :: source_index = source_index, face_index = face_index :: call

fn face_key_parts(source_index: Int, face_index: Int) -> Str:
    return (std.text.from_int :: source_index :: call) + ":" + (std.text.from_int :: face_index :: call)

fn face_key(read id: arcana_text.types.FontFaceId) -> Str:
    return arcana_text.fonts.face_key_parts :: id.source_index, id.face_index :: call

fn empty_raster(font_size: Int) -> arcana_text.font_leaf.GlyphBitmap:
    let mut out = arcana_text.font_leaf.GlyphBitmap :: size = (0, 0), offset = (0, 0), advance = (max_int :: font_size, 1 :: call) :: call
    out.baseline = max_int :: font_size, 1 :: call
    out.line_height = max_int :: (font_size + 4), font_size :: call
    out.empty = true
    out.alpha = arcana_text.fonts.empty_bytes :: :: call
    out.rgba = arcana_text.fonts.empty_bytes :: :: call
    return out

export fn invalid_match() -> arcana_text.types.FontMatch:
    return arcana_text.fonts.match_from_source :: (arcana_text.fonts.invalid_face_id :: :: call), (arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), "", "" :: call) :: call

fn empty_match() -> arcana_text.types.FontMatch:
    return arcana_text.fonts.invalid_match :: :: call

fn match_from_source(read id: arcana_text.types.FontFaceId, read source: arcana_text.types.FontSource) -> arcana_text.types.FontMatch:
    return arcana_text.types.FontMatch :: id = id, source = source :: call

fn match_from_entry(read entry: arcana_text.fonts.RegisteredFace) -> arcana_text.types.FontMatch:
    return arcana_text.fonts.match_from_source :: entry.id, entry.source :: call

fn result_err_or[T](read value: Result[T, Str], read fallback: Str) -> Str:
    return match value:
        Result.Ok(_) => fallback
        Result.Err(err) => err

fn max_int(a: Int, b: Int) -> Int:
    if a >= b:
        return a
    return b

fn abs_int(value: Int) -> Int:
    if value < 0:
        return 0 - value
    return value

fn infer_family_from_path(path: Str) -> Str:
    let stem = (std.path.stem :: path :: call) :: (std.path.file_name :: path :: call) :: fallback
    let mark = std.text.find :: stem, 0, " Var" :: call
    if mark >= 0:
        return std.text.slice_bytes :: stem, 0, mark :: call
    return stem

fn infer_face_from_path(path: Str) -> Str:
    let stem = (std.path.stem :: path :: call) :: (std.path.file_name :: path :: call) :: fallback
    if (std.text.contains :: stem, "Bold" :: call):
        return "Bold"
    if (std.text.contains :: stem, "Italic" :: call):
        return "Italic"
    return "Regular"

fn source_base(kind: arcana_text.types.FontSourceKind, label: Str, path: Str) -> arcana_text.types.FontSource:
    let mut source = arcana_text.types.FontSource :: kind = kind, label = label, path = path :: call
    source.family = ""
    source.face = ""
    source.full_name = ""
    source.postscript_name = ""
    source.installed = false
    return source

fn source_with_face_metadata(read source: arcana_text.types.FontSource, read metadata: arcana_text.font_leaf.SourceFaceMetadata) -> arcana_text.types.FontSource:
    let mut next = source
    if metadata.full_name != "":
        next.label = metadata.full_name
    next.family = metadata.family_name
    next.face = metadata.face_name
    next.full_name = metadata.full_name
    next.postscript_name = metadata.postscript_name
    return next

fn default_traits() -> arcana_text.font_leaf.FaceTraits:
    return arcana_text.font_leaf.default_traits :: :: call

fn axis_value_milli(tag: Str, read axes: List[arcana_text.types.FontAxis], fallback: Int) -> Int:
    for axis in axes:
        if axis.tag == tag:
            if tag == "wdth" and axis.value > 0 and axis.value < 1000:
                return axis.value * 1000
            if tag == "slnt" and axis.value > -100 and axis.value < 100:
                return axis.value * 1000
            return axis.value
    return fallback

fn style_traits(read style: arcana_text.types.TextStyle) -> arcana_text.font_leaf.FaceTraits:
    let mut traits = arcana_text.fonts.default_traits :: :: call
    traits.weight = axis_value_milli :: "wght", style.axes, traits.weight :: call
    traits.width_milli = axis_value_milli :: "wdth", style.axes, traits.width_milli :: call
    traits.slant_milli = axis_value_milli :: "slnt", style.axes, traits.slant_milli :: call
    return traits

fn query_traits(read query: arcana_text.types.FontQuery) -> arcana_text.font_leaf.FaceTraits:
    let mut traits = arcana_text.fonts.default_traits :: :: call
    if query.weight > 0:
        traits.weight = query.weight
    if query.width_milli > 0:
        traits.width_milli = query.width_milli
    traits.slant_milli = query.slant_milli
    traits.weight = axis_value_milli :: "wght", query.axes, traits.weight :: call
    traits.width_milli = axis_value_milli :: "wdth", query.axes, traits.width_milli :: call
    traits.slant_milli = axis_value_milli :: "slnt", query.axes, traits.slant_milli :: call
    return traits

export fn style_traits_for(read style: arcana_text.types.TextStyle) -> arcana_text.font_leaf.FaceTraits:
    return arcana_text.fonts.style_traits :: style :: call

fn line_height_milli(read style: arcana_text.types.TextStyle) -> Int:
    if style.line_height > 0 and style.size > 0:
        return (style.line_height * 1000) / style.size
    return 1000

fn normalized_width_milli(width_milli: Int) -> Int:
    if width_milli <= 0:
        return 100000
    return width_milli

export fn style_line_height_milli(read style: arcana_text.types.TextStyle) -> Int:
    return arcana_text.fonts.line_height_milli :: style :: call

fn fallback_line_height(read style: arcana_text.types.TextStyle) -> Int:
    if style.line_height > 0:
        return style.line_height
    return style.size + 6

fn face_traits_from_face(read face: arcana_text.font_leaf.FontFaceState) -> arcana_text.font_leaf.FaceTraits:
    let mut traits = arcana_text.font_leaf.default_traits :: :: call
    traits.weight = face.weight
    traits.width_milli = face.width_milli
    traits.slant_milli = face.slant_milli
    return traits

fn entry_from_source(read id: arcana_text.types.FontFaceId, read source: arcana_text.types.FontSource) -> arcana_text.fonts.RegisteredFace:
    let mut entry = arcana_text.fonts.RegisteredFace :: id = id, source = source, traits = (arcana_text.fonts.default_traits :: :: call) :: call
    entry.face_id = Option.None[std.memory.SlabId[arcana_text.font_leaf.FontFaceState]] :: :: call
    entry.load_error = ""
    entry.units_per_em = 1
    entry.ascender = 0
    entry.descender = 0
    entry.line_gap = 0
    return entry

fn face_traits_score(read actual: arcana_text.font_leaf.FaceTraits, read target: arcana_text.font_leaf.FaceTraits) -> Int:
    return (arcana_text.fonts.abs_int :: (actual.weight - target.weight) :: call) + ((arcana_text.fonts.abs_int :: (actual.width_milli - target.width_milli) :: call) / 1000) + ((arcana_text.fonts.abs_int :: (actual.slant_milli - target.slant_milli) :: call) / 1000)

fn register_face(edit self: arcana_text.fonts.FontSystem, source_index: Int, face_index: Int) -> arcana_text.types.FontFaceId:
    let id = arcana_text.fonts.face_id :: source_index, face_index :: call
    let key = arcana_text.fonts.face_key :: id :: call
    let source = arcana_text.fonts.source_at_or_empty :: self, source_index :: call
    let entry = arcana_text.fonts.entry_from_source :: id, source :: call
    self.faces :: key, entry :: set
    self.source_face_counts :: source_index, (face_index + 1) :: set
    self.face_count += 1
    return id

fn register_source_faces(edit self: arcana_text.fonts.FontSystem, source_index: Int, face_total: Int) -> arcana_text.types.FontFaceId:
    let mut total = face_total
    if total <= 0:
        total = 1
    let mut first = arcana_text.fonts.invalid_face_id :: :: call
    let mut face_index = 0
    while face_index < total:
        let id = arcana_text.fonts.register_face :: self, source_index, face_index :: call
        if first.source_index < 0:
            first = id
        face_index += 1
    return first

fn add_source_value(edit self: arcana_text.fonts.FontSystem, read value: arcana_text.types.FontSource) -> arcana_text.types.FontFaceId:
    return arcana_text.fonts.add_source_value_with_faces :: self, value, 1 :: call

fn add_source_value_with_faces(edit self: arcana_text.fonts.FontSystem, read value: arcana_text.types.FontSource, face_total: Int) -> arcana_text.types.FontFaceId:
    let index = self.source_count
    self.selection_cache = arcana_text.fonts.empty_selection_cache :: :: call
    self.shape_cache = arcana_text.shape.cache.open :: :: call
    let stored = value
    let path = stored.path
    if path != "":
        self.path_index :: path, index :: set
    self.sources :: index, stored :: set
    self.source_count = index + 1
    let id = arcana_text.fonts.register_source_faces :: self, index, face_total :: call
    fonts_probe_append :: ("add_source:done index=" + (std.text.from_int :: index :: call) + " faces=" + (std.text.from_int :: face_total :: call)) :: call
    return id

fn add_source_value_with_blob(edit self: arcana_text.fonts.FontSystem, read registration: arcana_text.fonts.BlobSourceRegistration) -> arcana_text.types.FontFaceId:
    let index = self.source_count
    self.selection_cache = arcana_text.fonts.empty_selection_cache :: :: call
    self.shape_cache = arcana_text.shape.cache.open :: :: call
    let source = registration.source
    let face_total = registration.face_total
    let blob_id = std.kernel.memory.slab_alloc[Array[Int]] :: self.source_blobs, registration.source_bytes :: call
    self.source_blob_ids :: index, blob_id :: set
    let path = source.path
    if path != "":
        self.path_index :: path, index :: set
    self.sources :: index, source :: set
    self.source_count = index + 1
    let id = arcana_text.fonts.register_source_faces :: self, index, face_total :: call
    arcana_text.fonts.hydrate_registered_faces_from_source :: self, index :: call
    fonts_probe_append :: ("add_source:done index=" + (std.text.from_int :: index :: call) + " faces=" + (std.text.from_int :: face_total :: call)) :: call
    return id

fn source_path_exists(read self: arcana_text.fonts.FontSystem, path: Str) -> Bool:
    if path == "":
        return false
    return self.path_index :: path :: has

fn source_from_file_path(path: Str) -> arcana_text.types.FontSource:
    let family = arcana_text.fonts.infer_family_from_path :: path :: call
    let face = arcana_text.fonts.infer_face_from_path :: path :: call
    let mut source = arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.File :: :: call), family, path :: call
    source.family = family
    source.face = face
    source.full_name = family
    source.postscript_name = family
    return source

fn source_from_bytes(label: Str) -> arcana_text.types.FontSource:
    let mut source = arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), label, "" :: call
    source.family = label
    source.face = "Regular"
    source.full_name = label
    source.postscript_name = label
    return source

fn source_from_catalog(read catalog: arcana_winapi.types.SystemFontCatalog, index: Int) -> arcana_text.types.FontSource:
    let family = arcana_winapi.fonts.catalog_family_name :: catalog, index :: call
    let face = arcana_winapi.fonts.catalog_face_name :: catalog, index :: call
    let full_name = arcana_winapi.fonts.catalog_full_name :: catalog, index :: call
    let postscript_name = arcana_winapi.fonts.catalog_postscript_name :: catalog, index :: call
    let path = arcana_winapi.fonts.catalog_path :: catalog, index :: call
    let mut source = arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Installed :: :: call), full_name, path :: call
    source.family = family
    source.face = face
    source.full_name = full_name
    source.postscript_name = postscript_name
    source.installed = true
    return source

fn ensure_bundled_family_named(edit self: arcana_text.fonts.FontSystem, family: Str) -> Int:
    if family == (arcana_text.monaspace.family_name :: (arcana_text.monaspace.MonaspaceFamily.Argon :: :: call) :: call):
        return self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Argon :: :: call) :: call) :: add_if_file
    if family == (arcana_text.monaspace.family_name :: (arcana_text.monaspace.MonaspaceFamily.Krypton :: :: call) :: call):
        return self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Krypton :: :: call) :: call) :: add_if_file
    if family == (arcana_text.monaspace.family_name :: (arcana_text.monaspace.MonaspaceFamily.Neon :: :: call) :: call):
        return self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Neon :: :: call) :: call) :: add_if_file
    if family == (arcana_text.monaspace.family_name :: (arcana_text.monaspace.MonaspaceFamily.Radon :: :: call) :: call):
        return self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Radon :: :: call) :: call) :: add_if_file
    if family == (arcana_text.monaspace.family_name :: (arcana_text.monaspace.MonaspaceFamily.Xenon :: :: call) :: call):
        return self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Xenon :: :: call) :: call) :: add_if_file
    return 0

fn source_matches_family(read source: arcana_text.types.FontSource, family: Str) -> Bool:
    return source.family == family or source.full_name == family or source.label == family or source.postscript_name == family

fn fallback_script_tag_for_codepoint(codepoint: Int) -> Str:
    if codepoint >= 125184 and codepoint <= 125279:
        return "adlm"
    if codepoint >= 1424 and codepoint <= 1535:
        return "hebr"
    if (codepoint >= 1536 and codepoint <= 1791) or (codepoint >= 1872 and codepoint <= 1919) or (codepoint >= 2208 and codepoint <= 2303) or (codepoint >= 64336 and codepoint <= 65023):
        return "arab"
    if codepoint >= 1920 and codepoint <= 1983:
        return "thaa"
    if codepoint >= 2304 and codepoint <= 2431:
        return "deva"
    if codepoint >= 2432 and codepoint <= 2559:
        return "beng"
    if codepoint >= 2560 and codepoint <= 2687:
        return "guru"
    if codepoint >= 2688 and codepoint <= 2815:
        return "gujr"
    if codepoint >= 2816 and codepoint <= 2943:
        return "orya"
    if codepoint >= 2944 and codepoint <= 3071:
        return "taml"
    if codepoint >= 3072 and codepoint <= 3199:
        return "telu"
    if codepoint >= 3200 and codepoint <= 3327:
        return "knda"
    if codepoint >= 3328 and codepoint <= 3455:
        return "mlym"
    if codepoint >= 3456 and codepoint <= 3583:
        return "sinh"
    if codepoint >= 3584 and codepoint <= 3711:
        return "thai"
    if codepoint >= 3712 and codepoint <= 3839:
        return "laoo"
    if codepoint >= 3840 and codepoint <= 4095:
        return "tibt"
    if codepoint >= 4096 and codepoint <= 4255:
        return "mymr"
    if codepoint >= 4608 and codepoint <= 4991:
        return "ethi"
    if codepoint >= 5024 and codepoint <= 5119:
        return "cher"
    if codepoint >= 5120 and codepoint <= 5759:
        return "cans"
    if codepoint >= 6016 and codepoint <= 6143:
        return "khmr"
    if codepoint >= 6144 and codepoint <= 6319:
        return "mong"
    if codepoint >= 11568 and codepoint <= 11647:
        return "tfng"
    if codepoint >= 12352 and codepoint <= 12447:
        return "hira"
    if codepoint >= 12448 and codepoint <= 12543:
        return "kana"
    if codepoint >= 12544 and codepoint <= 12591:
        return "bopo"
    if codepoint >= 12704 and codepoint <= 12735:
        return "bopo"
    if (codepoint >= 13312 and codepoint <= 19903) or (codepoint >= 19968 and codepoint <= 40959) or (codepoint >= 63744 and codepoint <= 64255):
        return "hani"
    if codepoint >= 44032 and codepoint <= 55215:
        return "hang"
    if codepoint >= 42240 and codepoint <= 42559:
        return "vaii"
    if codepoint >= 43392 and codepoint <= 43487:
        return "java"
    if codepoint >= 43888 and codepoint <= 43967:
        return "cher"
    if codepoint >= 40960 and codepoint <= 42127:
        return "yiii"
    if codepoint >= 69888 and codepoint <= 70015:
        return "cakm"
    return ""

fn fallback_script_tag_for_text(text: Str) -> Str:
    let total = std.text.len_bytes :: text :: call
    let mut index = 0
    while index < total:
        let codepoint = arcana_text.text_units.codepoint_at :: text, index :: call
        let tag = arcana_text.fonts.fallback_script_tag_for_codepoint :: codepoint :: call
        if tag != "":
            return tag
        index = arcana_text.text_units.next_scalar_end :: text, index :: call
    return ""

fn locale_matches(read locale: Str, read target: Str) -> Bool:
    if locale == target:
        return true
    return std.text.starts_with :: locale, (target + "-") :: call

fn common_fallback_families(read locale: Str) -> List[Str]:
    let mut out = std.collections.list.empty[Str] :: :: call
    out :: "Segoe UI" :: push
    out :: "Segoe UI Emoji" :: push
    out :: "Noto Color Emoji" :: push
    out :: "Segoe UI Symbol" :: push
    out :: "Segoe UI Historic" :: push
    out :: "Noto Sans Symbols2" :: push
    out :: "Arial Unicode MS" :: push
    out :: "Noto Sans" :: push
    out :: "DejaVu Sans" :: push
    if arcana_text.fonts.locale_matches :: locale, "ja" :: call:
        out :: "Yu Gothic UI" :: push
    else:
        if arcana_text.fonts.locale_matches :: locale, "ko" :: call:
            out :: "Malgun Gothic" :: push
        else:
            if arcana_text.fonts.locale_matches :: locale, "zh-HK" :: call:
                out :: "MingLiU_HKSCS" :: push
            else:
                if arcana_text.fonts.locale_matches :: locale, "zh-TW" :: call:
                    out :: "Microsoft JhengHei UI" :: push
                else:
                    if arcana_text.fonts.locale_matches :: locale, "zh" :: call:
                        out :: "Microsoft YaHei UI" :: push
    return out

fn forbidden_fallback_families(read locale: Str) -> List[Str]:
    let mut out = std.collections.list.empty[Str] :: :: call
    if arcana_text.fonts.locale_matches :: locale, "zh" :: call:
        return out
    return out

fn han_unification_families(read locale: Str) -> List[Str]:
    let mut out = std.collections.list.empty[Str] :: :: call
    if arcana_text.fonts.locale_matches :: locale, "ja" :: call:
        out :: "Yu Gothic UI" :: push
        out :: "Yu Gothic" :: push
    else:
        if arcana_text.fonts.locale_matches :: locale, "ko" :: call:
            out :: "Malgun Gothic" :: push
        else:
            if arcana_text.fonts.locale_matches :: locale, "zh-HK" :: call:
                out :: "MingLiU_HKSCS" :: push
            else:
                if arcana_text.fonts.locale_matches :: locale, "zh-TW" :: call:
                    out :: "Microsoft JhengHei UI" :: push
                else:
                    out :: "Microsoft YaHei UI" :: push
    out :: "Noto Sans CJK SC" :: push
    out :: "Noto Sans CJK TC" :: push
    out :: "Noto Sans CJK JP" :: push
    out :: "Noto Sans CJK KR" :: push
    return out

fn script_fallback_families(script_tag: Str, read locale: Str) -> List[Str]:
    let mut out = std.collections.list.empty[Str] :: :: call
    if script_tag == "arab":
        out :: "Noto Sans Arabic" :: push
        out :: "Segoe UI" :: push
        return out
    if script_tag == "hebr":
        out :: "Noto Sans Hebrew" :: push
        out :: "Arial" :: push
        return out
    if script_tag == "deva":
        out :: "Noto Sans Devanagari" :: push
        out :: "Mangal" :: push
        return out
    if script_tag == "adlm":
        out :: "Ebrima" :: push
        return out
    if script_tag == "beng":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "cans":
        out :: "Gadugi" :: push
        return out
    if script_tag == "cakm":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "cher":
        out :: "Gadugi" :: push
        return out
    if script_tag == "ethi":
        out :: "Ebrima" :: push
        return out
    if script_tag == "gujr":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "guru":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "bopo":
        out :: (arcana_text.fonts.han_unification_families :: locale :: call) :: extend_list
        return out
    if script_tag == "hani":
        out :: (arcana_text.fonts.han_unification_families :: locale :: call) :: extend_list
        return out
    if script_tag == "hang":
        out :: (arcana_text.fonts.han_unification_families :: "ko" :: call) :: extend_list
        return out
    if script_tag == "hira":
        out :: (arcana_text.fonts.han_unification_families :: "ja" :: call) :: extend_list
        return out
    if script_tag == "kana":
        out :: (arcana_text.fonts.han_unification_families :: "ja" :: call) :: extend_list
        return out
    if script_tag == "java":
        out :: "Javanese Text" :: push
        return out
    if script_tag == "knda":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "khmr":
        out :: "Leelawadee UI" :: push
        return out
    if script_tag == "laoo":
        out :: "Leelawadee UI" :: push
        return out
    if script_tag == "mlym":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "mong":
        out :: "Mongolian Baiti" :: push
        return out
    if script_tag == "mymr":
        out :: "Myanmar Text" :: push
        return out
    if script_tag == "orya":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "sinh":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "taml":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "telu":
        out :: "Nirmala UI" :: push
        return out
    if script_tag == "thaa":
        out :: "MV Boli" :: push
        return out
    if script_tag == "thai":
        out :: "Leelawadee UI" :: push
        return out
    if script_tag == "tibt":
        out :: "Microsoft Himalaya" :: push
        return out
    if script_tag == "tfng":
        out :: "Ebrima" :: push
        return out
    if script_tag == "vaii":
        out :: "Ebrima" :: push
        return out
    if script_tag == "yiii":
        out :: "Microsoft Yi Baiti" :: push
    return out

fn source_is_forbidden_fallback(read source: arcana_text.types.FontSource, read locale: Str) -> Bool:
    let forbidden = arcana_text.fonts.forbidden_fallback_families :: locale :: call
    for family in forbidden:
        if arcana_text.fonts.source_matches_family :: source, family :: call:
            return true
    return false

fn fallback_family_rank(read source: arcana_text.types.FontSource, script_tag: Str, read locale: Str) -> Int:
    if arcana_text.fonts.source_is_forbidden_fallback :: source, locale :: call:
        return 4000
    let script_families = arcana_text.fonts.script_fallback_families :: script_tag, locale :: call
    let mut rank = 0
    for family in script_families:
        if arcana_text.fonts.source_matches_family :: source, family :: call:
            return rank
        rank += 1
    let common_families = arcana_text.fonts.common_fallback_families :: locale :: call
    for family in common_families:
        if arcana_text.fonts.source_matches_family :: source, family :: call:
            return 100 + rank
        rank += 1
    return 1000 + rank

fn source_at_or_empty(read self: arcana_text.fonts.FontSystem, index: Int) -> arcana_text.types.FontSource:
    if self.sources :: index :: has:
        return self.sources :: index :: get
    return arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), "", "" :: call

fn copy_source(read source: arcana_text.types.FontSource) -> arcana_text.types.FontSource:
    return source

fn source_face_total(read self: arcana_text.fonts.FontSystem, source_index: Int) -> Int:
    if self.source_face_counts :: source_index :: has:
        return self.source_face_counts :: source_index :: get
    return 0

fn first_face_id_for_source(read self: arcana_text.fonts.FontSystem, source_index: Int) -> arcana_text.types.FontFaceId:
    if (arcana_text.fonts.source_face_total :: self, source_index :: call) > 0:
        return arcana_text.fonts.face_id :: source_index, 0 :: call
    return arcana_text.fonts.invalid_face_id :: :: call

fn first_registered_match(read self: arcana_text.fonts.FontSystem) -> arcana_text.types.FontMatch:
    if self.source_count > 0:
        let id = arcana_text.fonts.first_face_id_for_source :: self, 0 :: call
        return arcana_text.fonts.match_from_source :: id, (arcana_text.fonts.source_at_or_empty :: self, 0 :: call) :: call
    return arcana_text.fonts.empty_match :: :: call

fn source_is_collection_bytes(read bytes: Array[Int]) -> Bool:
    if (bytes :: :: len) < 4:
        return false
    return (bytes)[0] == 116 and (bytes)[1] == 116 and (bytes)[2] == 99 and (bytes)[3] == 102

fn source_face_total_from_bytes(read bytes: Array[Int]) -> Int:
    fonts_probe_append :: ("source_face_total:start bytes=" + (std.text.from_int :: (bytes :: :: len) :: call)) :: call
    if not (arcana_text.fonts.source_is_collection_bytes :: bytes :: call):
        fonts_probe_append :: "source_face_total:single_face" :: call
        return 1
    let view = std.memory.bytes_view :: bytes, 0, (bytes :: :: len) :: call
    let total_result = arcana_text.font_leaf.source_face_count_from_view :: view :: call
    let total = total_result :: 1 :: unwrap_or
    fonts_probe_append :: ("source_face_total:done total=" + (std.text.from_int :: total :: call)) :: call
    if total <= 0:
        return 1
    return total

fn fallback_face_metadata(face_index: Int, fallback_family: Str, fallback_label: Str) -> arcana_text.font_leaf.SourceFaceMetadata:
    let mut metadata = arcana_text.font_leaf.SourceFaceMetadata :: face_index = face_index, family_name = fallback_family, face_name = "Regular" :: call
    metadata.full_name = fallback_label
    metadata.postscript_name = fallback_label
    metadata.traits = arcana_text.fonts.default_traits :: :: call
    return metadata

fn face_at_or_empty(read self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId) -> arcana_text.fonts.RegisteredFace:
    let key = arcana_text.fonts.face_key :: id :: call
    if self.faces :: key :: has:
        return self.faces :: key :: get
    return arcana_text.fonts.entry_from_source :: id, (arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), "", "" :: call) :: call

export fn match_source(read self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch) -> arcana_text.types.FontSource:
    if matched.id.source_index >= 0:
        return (arcana_text.fonts.face_at_or_empty :: self, matched.id :: call).source
    return matched.source

export fn match_family_or_label(read self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch) -> Str:
    return arcana_text.fonts.family_or_label :: (arcana_text.fonts.match_source :: self, matched :: call) :: call

fn replace_face_at(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read replacement: arcana_text.fonts.RegisteredFace):
    let key = arcana_text.fonts.face_key :: id :: call
    self.faces :: key, replacement :: set

fn source_bytes_for_index(edit self: arcana_text.fonts.FontSystem, index: Int) -> Result[Array[Int], Str]:
    if self.source_blob_ids :: index :: has:
        let blob_id = self.source_blob_ids :: index :: get
        return Result.Ok[Array[Int], Str] :: (std.kernel.memory.slab_borrow_read :: self.source_blobs, blob_id :: call) :: call
    let source = arcana_text.fonts.source_at_or_empty :: self, index :: call
    if source.path != "":
        let bytes_result = std.fs.read_bytes :: source.path :: call
        if bytes_result :: :: is_err:
            return Result.Err[Array[Int], Str] :: (result_err_or :: bytes_result, "failed to read source bytes" :: call) :: call
        let bytes = bytes_result :: (arcana_text.fonts.empty_bytes :: :: call) :: unwrap_or
        let blob_id = std.kernel.memory.slab_alloc[Array[Int]] :: self.source_blobs, bytes :: call
        self.source_blob_ids :: index, blob_id :: set
        return Result.Ok[Array[Int], Str] :: (std.kernel.memory.slab_borrow_read :: self.source_blobs, blob_id :: call) :: call
    return Result.Err[Array[Int], Str] :: "font source has no blob or path" :: call

fn load_face_from_entry(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read entry: arcana_text.fonts.RegisteredFace) -> arcana_text.fonts.RegisteredFace:
    fonts_probe_append :: ("load_face:start source=" + (std.text.from_int :: id.source_index :: call) + " face=" + (std.text.from_int :: id.face_index :: call)) :: call
    let bytes_result = arcana_text.fonts.source_bytes_for_index :: self, id.source_index :: call
    if bytes_result :: :: is_err:
        fonts_probe_append :: "load_face:bytes_err" :: call
        return arcana_text.fonts.failed_face_entry :: entry, (result_err_or :: bytes_result, "failed to read source bytes" :: call) :: call
    let source_bytes = bytes_result :: (arcana_text.fonts.empty_bytes :: :: call) :: unwrap_or
    fonts_probe_append :: ("load_face:bytes len=" + (std.text.from_int :: (source_bytes :: :: len) :: call)) :: call
    let mut request = arcana_text.font_leaf.face_load_request :: (arcana_text.fonts.family_or_label :: entry.source :: call), entry.source.label, entry.source.path :: call
    request.face_index = id.face_index
    request.source_bytes = source_bytes
    request.traits = entry.traits
    let loaded_face = arcana_text.font_leaf.load_face_state_into_slab :: self.live_faces, request :: call
    return match loaded_face:
        std.result.Result.Ok(face_id) => load_face_from_entry_ok :: self, entry, face_id :: call
        std.result.Result.Err(err) => load_face_from_entry_err :: entry, err :: call

fn load_face_from_entry_ok(edit self: arcana_text.fonts.FontSystem, read entry: arcana_text.fonts.RegisteredFace, read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState]) -> arcana_text.fonts.RegisteredFace:
    fonts_probe_append :: "load_face:done" :: call
    return ensure_loaded_face_ok :: self, entry, face_id :: call

fn load_face_from_entry_err(read entry: arcana_text.fonts.RegisteredFace, err: Str) -> arcana_text.fonts.RegisteredFace:
    fonts_probe_append :: ("load_face:error " + err) :: call
    return arcana_text.fonts.failed_face_entry :: entry, err :: call

fn hydrate_registered_faces_from_source(edit self: arcana_text.fonts.FontSystem, source_index: Int):
    let bytes_result = arcana_text.fonts.source_bytes_for_index :: self, source_index :: call
    if bytes_result :: :: is_err:
        return
    let source_bytes = bytes_result :: (arcana_text.fonts.empty_bytes :: :: call) :: unwrap_or
    let bytes_view = std.memory.bytes_view :: source_bytes, 0, (source_bytes :: :: len) :: call
    let source = arcana_text.fonts.source_at_or_empty :: self, source_index :: call
    let fallback_family = arcana_text.fonts.family_or_label :: source :: call
    let fallback_label = source.label
    let mut first_source = source
    let total_faces = arcana_text.fonts.source_face_total :: self, source_index :: call
    let mut face_index = 0
    while face_index < total_faces:
        let mut metadata_request = arcana_text.font_leaf.SourceFaceMetadataRequest :: face_index = face_index, fallback_family = fallback_family, fallback_label = fallback_label :: call
        metadata_request.fallback_traits = arcana_text.fonts.default_traits :: :: call
        let metadata_result = arcana_text.font_leaf.source_face_metadata_from_view :: bytes_view, metadata_request :: call
        if metadata_result :: :: is_ok:
            let metadata = metadata_result :: (arcana_text.fonts.fallback_face_metadata :: face_index, fallback_family, fallback_label :: call) :: unwrap_or
            let id = arcana_text.fonts.face_id :: source_index, face_index :: call
            let entry = arcana_text.fonts.face_at_or_empty :: self, id :: call
            let next_source = arcana_text.fonts.source_with_face_metadata :: entry.source, metadata :: call
            let mut next = entry
            record place arcana_text.fonts.RegisteredFace from entry -> next -return next
                source = next_source
                traits = metadata.traits
            arcana_text.fonts.replace_face_at :: self, id, next :: call
            if face_index == 0:
                first_source = arcana_text.fonts.source_with_face_metadata :: source, metadata :: call
        face_index += 1
    self.sources :: source_index, first_source :: set

fn failed_face_entry(read entry: arcana_text.fonts.RegisteredFace, err: Str) -> arcana_text.fonts.RegisteredFace:
    let mut next = entry
    record place arcana_text.fonts.RegisteredFace from entry -> next -return next
        load_error = err
    return next

fn ensure_loaded_face(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId) -> arcana_text.fonts.RegisteredFace:
    let entry = arcana_text.fonts.face_at_or_empty :: self, id :: call
    if (entry.face_id :: :: is_some) or entry.load_error != "":
        fonts_probe_append :: ("ensure_loaded:cached source=" + (std.text.from_int :: id.source_index :: call) + " face=" + (std.text.from_int :: id.face_index :: call)) :: call
        return entry
    fonts_probe_append :: ("ensure_loaded:start source=" + (std.text.from_int :: id.source_index :: call) + " face=" + (std.text.from_int :: id.face_index :: call)) :: call
    let next = arcana_text.fonts.load_face_from_entry :: self, id, entry :: call
    arcana_text.fonts.replace_face_at :: self, id, next :: call
    fonts_probe_append :: ("ensure_loaded:stored source=" + (std.text.from_int :: id.source_index :: call) + " face=" + (std.text.from_int :: id.face_index :: call)) :: call
    return next

fn ensure_loaded_face_ok(edit self: arcana_text.fonts.FontSystem, read entry: arcana_text.fonts.RegisteredFace, read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState]) -> arcana_text.fonts.RegisteredFace:
    fonts_probe_append :: "ensure_loaded_ok:start" :: call
    let face = std.kernel.memory.slab_borrow_read :: self.live_faces, face_id :: call
    let traits = arcana_text.fonts.face_traits_from_face :: face :: call
    let units_per_em = face.units_per_em
    let ascender = face.ascender
    let descender = face.descender
    let line_gap = face.line_gap
    let mut source = entry.source
    if face.family_name != "":
        source.family = face.family_name
    if face.face_name != "":
        source.face = face.face_name
    if face.full_name != "":
        source.full_name = face.full_name
        source.label = face.full_name
    if face.postscript_name != "":
        source.postscript_name = face.postscript_name
    if entry.id.face_index == 0:
        self.sources :: entry.id.source_index, (arcana_text.fonts.copy_source :: source :: call) :: set
    let mut next = entry
    next.source = arcana_text.fonts.copy_source :: source :: call
    next.traits = traits
    next.face_id = Option.Some[std.memory.SlabId[arcana_text.font_leaf.FontFaceState]] :: face_id :: call
    next.load_error = ""
    next.units_per_em = units_per_em
    next.ascender = ascender
    next.descender = descender
    next.line_gap = line_gap
    fonts_probe_append :: "ensure_loaded_ok:done" :: call
    return next

fn line_height_from_entry_metrics(read entry: arcana_text.fonts.RegisteredFace, read style: arcana_text.types.TextStyle) -> Int:
    return arcana_text.fonts.line_height_from_entry_values :: entry, style.size, (arcana_text.fonts.line_height_milli :: style :: call) :: call

fn baseline_from_entry_metrics(read entry: arcana_text.fonts.RegisteredFace, read style: arcana_text.types.TextStyle) -> Int:
    return arcana_text.fonts.baseline_from_entry_values :: entry, style.size, (arcana_text.fonts.line_height_milli :: style :: call) :: call

fn line_height_from_entry_values(read entry: arcana_text.fonts.RegisteredFace, font_size: Int, line_height_milli: Int) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: entry.units_per_em, 1 :: call
    let safe_line_height = max_int :: line_height_milli, 1000 :: call
    let natural = ((entry.ascender - entry.descender + entry.line_gap) * safe_font_size) / safe_units_per_em
    let scaled = (natural * safe_line_height) / 1000
    return max_int :: scaled, safe_font_size :: call

fn baseline_from_entry_values(read entry: arcana_text.fonts.RegisteredFace, font_size: Int, line_height_milli: Int) -> Int:
    let safe_font_size = max_int :: font_size, 1 :: call
    let safe_units_per_em = max_int :: entry.units_per_em, 1 :: call
    let natural = (entry.ascender * safe_font_size) / safe_units_per_em
    let height = arcana_text.fonts.line_height_from_entry_values :: entry, font_size, line_height_milli :: call
    if natural < 0:
        return 0
    if natural > height:
        return height
    return natural

fn entry_supports_text(read entry: arcana_text.fonts.RegisteredFace, edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], text: Str) -> Bool:
    if text == " " or text == "\t" or text == "\n" or text == "\r":
        return true
    return match entry.face_id:
        Option.Some(face_id) => arcana_text.fonts.face_supports_text :: live_faces, face_id, text :: call
        Option.None => false

fn entry_supports_script(read entry: arcana_text.fonts.RegisteredFace, read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], script_tag: Str) -> Bool:
    if script_tag == "" or script_tag == "DFLT":
        return true
    return match entry.face_id:
        Option.Some(face_id) => arcana_text.fonts.face_supports_script :: live_faces, face_id, script_tag :: call
        Option.None => false

fn line_height_from_face_id(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read style: arcana_text.types.TextStyle) -> Int:
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.line_height_for_face :: face, style.size, (arcana_text.fonts.line_height_milli :: style :: call) :: call

fn baseline_from_face_id(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read style: arcana_text.types.TextStyle) -> Int:
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.baseline_for_face :: face, style.size, (arcana_text.fonts.line_height_milli :: style :: call) :: call

fn underline_metrics_from_face_id(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], font_size: Int) -> (Int, Int):
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.underline_metrics_for_face :: face, font_size :: call

fn strikethrough_metrics_from_face_id(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], font_size: Int) -> (Int, Int):
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.strikethrough_metrics_for_face :: face, font_size :: call

fn overline_metrics_from_face_id(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], font_size: Int) -> (Int, Int):
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.overline_metrics_for_face :: face, font_size :: call

fn face_supports_text(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], text: Str) -> Bool:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.cmap.supports_text :: face, text :: call

fn face_supports_script(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], script_tag: Str) -> Bool:
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.supports_script :: face, script_tag :: call

fn measure_face_glyph_spec(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.measure_glyph :: face, spec :: call

fn advance_face_glyph_spec(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read spec: arcana_text.font_leaf.GlyphRenderSpec) -> Int:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.advance_for_glyph :: face, spec :: call

fn render_face_glyph_spec(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.render_glyph :: face, spec :: call

fn substitute_gsub_units(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read request: arcana_text.fonts.GsubUnitsRequest) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    let mut gsub_request = arcana_text.font_leaf.GsubSubstituteRequest :: script_tag = request.script_tag, language_tag = request.language_tag, features = (std.collections.list.new[arcana_text.types.FontFeature] :: :: call) :: call
    gsub_request.features = request.features
    gsub_request.glyphs = request.glyphs
    return arcana_text.font_leaf.gsub_substitute :: face, gsub_request :: call

fn pair_adjust_from_face_id(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read request: arcana_text.fonts.PairAdjustRequest) -> arcana_text.font_leaf.PairPlacement:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    let mut traits = arcana_text.font_leaf.default_traits :: :: call
    traits.weight = face.weight
    traits.width_milli = request.width_milli
    traits.slant_milli = face.slant_milli
    let mut pair_request = arcana_text.font_leaf.PairAdjustmentRequest :: left_glyph = request.left_glyph, right_glyph = request.right_glyph, script_tag = request.script_tag :: call
    pair_request.language_tag = request.language_tag
    pair_request.features = request.features
    pair_request.traits = traits
    pair_request.font_size = request.font_size
    return arcana_text.font_leaf.pair_placement :: face, pair_request :: call

fn single_adjust_from_face_id(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read request: arcana_text.fonts.SingleAdjustRequest) -> arcana_text.font_leaf.PairPlacement:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    let mut traits = arcana_text.font_leaf.default_traits :: :: call
    traits.weight = face.weight
    traits.width_milli = request.width_milli
    traits.slant_milli = face.slant_milli
    let mut single_request = arcana_text.font_leaf.SingleAdjustmentRequest :: glyph = request.glyph, script_tag = request.script_tag, language_tag = request.language_tag :: call
    single_request.features = request.features
    single_request.traits = traits
    single_request.font_size = request.font_size
    return arcana_text.font_leaf.single_placement :: face, single_request :: call

fn position_lookups_from_face_id(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read request: arcana_text.fonts.PositionLookupsRequest) -> List[arcana_text.font_leaf.GsubLookupRef]:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    let mut lookup_request = arcana_text.font_leaf.LookupRefsForFeaturesRequest :: script_tag = request.script_tag, language_tag = request.language_tag, features = (std.collections.list.new[arcana_text.types.FontFeature] :: :: call) :: call
    lookup_request.features = request.features
    return arcana_text.font_leaf.position_lookup_refs :: face, lookup_request :: call

fn lookup_type_from_face_id(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read request: arcana_text.fonts.LookupTypeRequest) -> Int:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.position_lookup_type :: face, request.lookup :: call

fn lookup_placement_from_face_id(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read request: arcana_text.fonts.LookupPlacementRequest) -> arcana_text.font_leaf.PairPlacement:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    let mut traits = arcana_text.font_leaf.default_traits :: :: call
    traits.weight = face.weight
    traits.width_milli = request.width_milli
    traits.slant_milli = face.slant_milli
    let mut lookup_request = arcana_text.font_leaf.LookupPlacementRequest :: lookup = request.lookup, left_glyph = request.left_glyph, right_glyph = request.right_glyph :: call
    lookup_request.traits = traits
    lookup_request.font_size = request.font_size
    return arcana_text.font_leaf.lookup_placement :: face, lookup_request :: call

fn glyph_class_from_face_id(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read request: arcana_text.fonts.GlyphClassRequest) -> Int:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.glyph_class :: face, request.glyph_index :: call

fn glyph_index_for_face_id(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], text: Str) -> Int:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.cmap.glyph_index_for_codepoint :: face, (arcana_text.fonts.utf8_codepoint :: text :: call) :: call

fn line_height_from_entry(edit self: arcana_text.fonts.FontSystem, read entry: arcana_text.fonts.RegisteredFace, read style: arcana_text.types.TextStyle) -> Int:
    return match entry.face_id:
        Option.Some(face_id) => arcana_text.fonts.line_height_from_face_id :: self.live_faces, face_id, style :: call
        Option.None => arcana_text.fonts.fallback_line_height :: style :: call

fn baseline_from_entry(edit self: arcana_text.fonts.FontSystem, read entry: arcana_text.fonts.RegisteredFace, read style: arcana_text.types.TextStyle) -> Int:
    return match entry.face_id:
        Option.Some(face_id) => arcana_text.fonts.baseline_from_face_id :: self.live_faces, face_id, style :: call
        Option.None => max_int :: style.size, 1 :: call

fn selection_request(read style: arcana_text.types.TextStyle, text: Str, family: Str) -> arcana_text.fonts.FontSelectionRequest:
    let mut out = arcana_text.fonts.FontSelectionRequest :: style = style, text = text, family = family :: call
    out.restrict_family = false
    return out

fn face_search_request_from_selection(read request: arcana_text.fonts.FontSelectionRequest) -> arcana_text.fonts.FaceSearchRequest:
    let mut search = arcana_text.fonts.FaceSearchRequest :: target = (arcana_text.fonts.style_traits :: request.style :: call), text = request.text, family = request.family :: call
    search.families = arcana_text.fonts.empty_strs :: :: call
    search.restrict_family = request.restrict_family
    search.require_text_support = true
    return search

fn face_search_request_from_query(read query: arcana_text.types.FontQuery) -> arcana_text.fonts.FaceSearchRequest:
    let mut search = arcana_text.fonts.FaceSearchRequest :: target = (arcana_text.fonts.query_traits :: query :: call), text = "", family = "" :: call
    search.families = query.families
    search.restrict_family = false
    search.require_text_support = false
    return search

fn selection_request_key(read request: arcana_text.fonts.FontSelectionRequest) -> Str:
    let traits = arcana_text.fonts.style_traits :: request.style :: call
    let family_flag = match request.restrict_family:
        true => "1"
        false => "0"
    return family_flag + ":" + request.family + ":" + (std.text.from_int :: (std.text.len_bytes :: request.text :: call) :: call) + ":" + request.text + ":" + (std.text.from_int :: traits.weight :: call) + ":" + (std.text.from_int :: traits.width_milli :: call) + ":" + (std.text.from_int :: traits.slant_milli :: call)

fn source_matches_any_family(read source: arcana_text.types.FontSource, read families: List[Str]) -> Bool:
    if families :: :: is_empty:
        return true
    for family in families:
        if arcana_text.fonts.source_matches_family :: source, family :: call:
            return true
    return false

fn entry_allowed_for_search(read entry: arcana_text.fonts.RegisteredFace, read request: arcana_text.fonts.FaceSearchRequest) -> Bool:
    if request.restrict_family:
        return arcana_text.fonts.source_matches_family :: entry.source, request.family :: call
    return arcana_text.fonts.source_matches_any_family :: entry.source, request.families :: call

fn scored_match(score: Int, read matched: arcana_text.types.FontMatch) -> arcana_text.fonts.ScoredMatch:
    let mut out = arcana_text.fonts.ScoredMatch :: score = score, tie = (arcana_text.fonts.stable_match_tie :: matched :: call) :: call
    out.matched = matched
    return out

fn copy_scored_match(read value: arcana_text.fonts.ScoredMatch) -> arcana_text.fonts.ScoredMatch:
    return value

fn scored_matches_has(read values: List[arcana_text.fonts.ScoredMatch], read matched: arcana_text.types.FontMatch) -> Bool:
    for value in values:
        if arcana_text.fonts.same_match :: value.matched, matched :: call:
            return true
    return false

fn insert_scored_match(edit values: List[arcana_text.fonts.ScoredMatch], read value: arcana_text.fonts.ScoredMatch):
    if arcana_text.fonts.scored_matches_has :: values, value.matched :: call:
        return
    let mut copy = arcana_text.fonts.empty_scored_matches :: :: call
    copy :: values :: extend_list
    values :: :: clear
    let total = copy :: :: len
    let mut inserted = false
    let mut index = 0
    while index < total:
        let existing = arcana_text.fonts.copy_scored_match :: (copy)[index] :: call
        let before = value.score < existing.score or (value.score == existing.score and value.tie < existing.tie)
        if not inserted and before:
            values :: (arcana_text.fonts.copy_scored_match :: value :: call) :: push
            inserted = true
        values :: existing :: push
        index += 1
    if not inserted:
        values :: value :: push

fn collect_matches(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.FaceSearchRequest) -> List[arcana_text.fonts.ScoredMatch]:
    let mut matches = arcana_text.fonts.empty_scored_matches :: :: call
    let mut source_index = 0
    while source_index < self.source_count:
        let total_faces = arcana_text.fonts.source_face_total :: self, source_index :: call
        let mut face_index = 0
        while face_index < total_faces:
            let id = arcana_text.fonts.face_id :: source_index, face_index :: call
            let loaded = arcana_text.fonts.ensure_loaded_face :: self, id :: call
            if not (arcana_text.fonts.entry_allowed_for_search :: loaded, request :: call):
                face_index += 1
                continue
            let mut supports = true
            if request.require_text_support:
                supports = arcana_text.fonts.entry_supports_text :: loaded, self.live_faces, request.text :: call
            if supports:
                let score = arcana_text.fonts.face_traits_score :: loaded.traits, request.target :: call
                let matched = arcana_text.fonts.match_from_entry :: loaded :: call
                let scored = arcana_text.fonts.scored_match :: score, matched :: call
                arcana_text.fonts.insert_scored_match :: matches, scored :: call
            face_index += 1
        source_index += 1
    return matches

fn scored_matches_values(read values: List[arcana_text.fonts.ScoredMatch], read exclude: arcana_text.types.FontMatch) -> List[arcana_text.types.FontMatch]:
    let mut out = std.collections.list.empty[arcana_text.types.FontMatch] :: :: call
    for value in values:
        if exclude.id.source_index >= 0 and (arcana_text.fonts.same_match :: value.matched, exclude :: call):
            continue
        out :: value.matched :: push
    return out

fn script_ranked_matches(edit self: arcana_text.fonts.FontSystem, read values: List[arcana_text.types.FontMatch], script_tag: Str) -> List[arcana_text.types.FontMatch]:
    let mut scored = arcana_text.fonts.empty_scored_matches :: :: call
    let mut index = 0
    for matched in values:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        if arcana_text.fonts.source_is_forbidden_fallback :: entry.source, self.locale :: call:
            index += 1
            continue
        let mut score = arcana_text.fonts.fallback_family_rank :: entry.source, script_tag, self.locale :: call
        if script_tag != "" and script_tag != "DFLT" and not (arcana_text.fonts.entry_supports_script :: entry, self.live_faces, script_tag :: call):
            score = 2000 + index
        else:
            score = arcana_text.fonts.fallback_family_rank :: entry.source, script_tag, self.locale :: call
        let value = arcana_text.fonts.scored_match :: score, matched :: call
        arcana_text.fonts.insert_scored_match :: scored, value :: call
        index += 1
    return arcana_text.fonts.scored_matches_values :: scored, (arcana_text.fonts.empty_match :: :: call) :: call

fn select_best_match(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.FaceSearchRequest) -> arcana_text.types.FontMatch:
    let matches = arcana_text.fonts.collect_matches :: self, request :: call
    for value in matches:
        return value.matched
    return arcana_text.fonts.empty_match :: :: call

fn select_match_internal(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.FontSelectionRequest) -> arcana_text.types.FontMatch:
    let cache_key = arcana_text.fonts.selection_request_key :: request :: call
    if self.selection_cache :: cache_key :: has:
        fonts_probe_append :: ("select_match_internal:cache " + cache_key) :: call
        return self.selection_cache :: cache_key :: get
    fonts_probe_append :: ("select_match_internal:start " + cache_key + " sources=" + (std.text.from_int :: self.source_count :: call)) :: call
    let selected = arcana_text.fonts.select_best_match :: self, (arcana_text.fonts.face_search_request_from_selection :: request :: call) :: call
    let selected_source = selected.id.source_index
    let selected_face = selected.id.face_index
    fonts_probe_append :: ("select_match_internal:done source=" + (std.text.from_int :: selected_source :: call) + " face=" + (std.text.from_int :: selected_face :: call)) :: call
    self.selection_cache :: cache_key, selected :: set
    return self.selection_cache :: cache_key :: get

export fn new_system() -> arcana_text.fonts.FontSystem:
    let mut system = arcana_text.fonts.FontSystem :: source_count = 0, face_count = 0, locale = (arcana_text.fonts.default_locale :: :: call) :: call
    system.sources = arcana_text.fonts.empty_sources :: :: call
    system.faces = arcana_text.fonts.empty_faces :: :: call
    system.selection_cache = arcana_text.fonts.empty_selection_cache :: :: call
    system.path_index = arcana_text.fonts.empty_path_index :: :: call
    system.source_face_counts = arcana_text.fonts.empty_source_face_counts :: :: call
    system.source_blob_ids = arcana_text.fonts.empty_source_blob_ids :: :: call
    system.source_blobs = arcana_text.fonts.empty_source_blobs :: :: call
    system.live_faces = arcana_text.fonts.empty_live_faces :: :: call
    system.shape_cache = arcana_text.shape.cache.open :: :: call
    system.discovered = false
    return system

export fn new_system_with_locale(locale: Str) -> arcana_text.fonts.FontSystem:
    let mut system = arcana_text.fonts.new_system :: :: call
    system.locale = locale
    return system

export fn default_system() -> arcana_text.fonts.FontSystem:
    fonts_probe_append :: "default_system:start" :: call
    let mut out = arcana_text.fonts.new_system :: :: call
    let added = out :: :: register_bundled_defaults
    fonts_probe_append :: ("default_system:bundled_done added=" + (std.text.from_int :: added :: call)) :: call
    return out

export fn default_system_with_locale(locale: Str) -> arcana_text.fonts.FontSystem:
    let mut out = arcana_text.fonts.new_system_with_locale :: locale :: call
    let _ = out :: :: register_bundled_defaults
    return out

impl FontSystem:
    fn locale(read self: arcana_text.fonts.FontSystem) -> Str:
        return self.locale

    fn set_locale(edit self: arcana_text.fonts.FontSystem, locale: Str):
        if self.locale == locale:
            return
        self.locale = locale
        self.selection_cache = arcana_text.fonts.empty_selection_cache :: :: call
        self.shape_cache = arcana_text.shape.cache.open :: :: call

    fn count(read self: arcana_text.fonts.FontSystem) -> Int:
        return self.face_count

    fn source_at(read self: arcana_text.fonts.FontSystem, index: Int) -> arcana_text.types.FontSource:
        return arcana_text.fonts.source_at_or_empty :: self, index :: call

    fn register_bundled_defaults(edit self: arcana_text.fonts.FontSystem) -> Int:
        let default_family = arcana_text.monaspace.default_family :: :: call
        fonts_probe_append :: ("bundled:default " + (arcana_text.monaspace.family_name :: default_family :: call)) :: call
        let added = self :: (arcana_text.monaspace.variable_font_path :: default_family :: call) :: add_if_file
        fonts_probe_append :: ("bundled:done added=" + (std.text.from_int :: added :: call)) :: call
        return added

    fn add_if_file(edit self: arcana_text.fonts.FontSystem, path: Str) -> Int:
        if not (std.fs.is_file :: path :: call):
            return 0
        let _ = self :: path :: add_file
        return 1

    fn add_file(edit self: arcana_text.fonts.FontSystem, path: Str) -> arcana_text.types.FontFaceId:
        fonts_probe_append :: ("add_file:start path=" + path) :: call
        if not (std.fs.is_file :: path :: call):
            fonts_probe_append :: "add_file:missing" :: call
            return arcana_text.fonts.invalid_face_id :: :: call
        if self.path_index :: path :: has:
            let existing = self.path_index :: path :: get
            fonts_probe_append :: ("add_file:existing source=" + (std.text.from_int :: existing :: call)) :: call
            return arcana_text.fonts.first_face_id_for_source :: self, existing :: call
        let bytes_result = std.fs.read_bytes :: path :: call
        if bytes_result :: :: is_err:
            fonts_probe_append :: "add_file:read_err" :: call
            return arcana_text.fonts.invalid_face_id :: :: call
        let bytes = bytes_result :: (arcana_text.fonts.empty_bytes :: :: call) :: unwrap_or
        let face_total = arcana_text.fonts.source_face_total_from_bytes :: bytes :: call
        let source = arcana_text.fonts.source_from_file_path :: path :: call
        fonts_probe_append :: ("add_file:register faces=" + (std.text.from_int :: face_total :: call)) :: call
        let registration = arcana_text.fonts.BlobSourceRegistration :: source = source, source_bytes = bytes, face_total = face_total :: call
        return arcana_text.fonts.add_source_value_with_blob :: self, registration :: call

    fn add_dir(edit self: arcana_text.fonts.FontSystem, path: Str) -> Int:
        if not (std.fs.is_dir :: path :: call):
            return 0
        let mut added = 0
        let entries = match (std.fs.list_dir :: path :: call):
            std.result.Result.Ok(value) => value
            std.result.Result.Err(_) => (std.collections.list.new[Str] :: :: call)
        for entry in entries:
            let ext = std.text.trim :: (std.path.ext :: entry :: call) :: call
            if ext == ".ttf" or ext == ".otf" or ext == ".ttc" or ext == ".otc":
                let _ = self :: entry :: add_file
                added += 1
        return added

    fn add_bytes(edit self: arcana_text.fonts.FontSystem, label: Str, read bytes: Array[Int]) -> arcana_text.types.FontFaceId:
        let face_total = arcana_text.fonts.source_face_total_from_bytes :: bytes :: call
        let source = arcana_text.fonts.source_from_bytes :: label :: call
        let registration = arcana_text.fonts.BlobSourceRegistration :: source = source, source_bytes = bytes, face_total = face_total :: call
        return arcana_text.fonts.add_source_value_with_blob :: self, registration :: call

    fn discover_installed(edit self: arcana_text.fonts.FontSystem) -> Int:
        self.selection_cache = arcana_text.fonts.empty_selection_cache :: :: call
        self.shape_cache = arcana_text.shape.cache.open :: :: call
        let catalog = arcana_winapi.fonts.system_font_catalog :: :: call
        let count = arcana_winapi.fonts.catalog_count :: catalog :: call
        let mut added = 0
        let mut index = 0
        while index < count:
            let source = arcana_text.fonts.source_from_catalog :: catalog, index :: call
            if not (arcana_text.fonts.source_path_exists :: self, source.path :: call):
                let _ = arcana_text.fonts.add_source_value :: self, source :: call
                added += 1
            index += 1
        arcana_winapi.fonts.catalog_destroy :: catalog :: call
        self.discovered = true
        return added

    fn resolve(edit self: arcana_text.fonts.FontSystem, read query: arcana_text.types.FontQuery) -> arcana_text.types.FontMatch:
        let selected = arcana_text.fonts.select_best_match :: self, (arcana_text.fonts.face_search_request_from_query :: query :: call) :: call
        if selected.id.source_index >= 0:
            return selected
        return arcana_text.fonts.first_registered_match :: self :: call

    fn resolve_style_text(edit self: arcana_text.fonts.FontSystem, read style: arcana_text.types.TextStyle, text: Str) -> arcana_text.types.FontMatch:
        fonts_probe_append :: ("resolve_style_text:start `" + text + "` families=" + (std.text.from_int :: (style.families :: :: len) :: call)) :: call
        for family in style.families:
            let _ = arcana_text.fonts.ensure_bundled_family_named :: self, family :: call
            let mut request = arcana_text.fonts.selection_request :: style, text, family :: call
            request.restrict_family = true
            let matched = arcana_text.fonts.select_match_internal :: self, request :: call
            if matched.id.source_index >= 0:
                fonts_probe_append :: ("resolve_style_text:family_hit " + family) :: call
                return matched
        let script_tag = arcana_text.fonts.fallback_script_tag_for_text :: text :: call
        if script_tag != "":
            let ranked = self :: (style, text, (arcana_text.fonts.empty_match :: :: call), script_tag) :: resolve_style_text_script_fallbacks
            for matched in ranked:
                if matched.id.source_index >= 0:
                    fonts_probe_append :: ("resolve_style_text:script_fallback " + script_tag) :: call
                    return matched
        let fallback = arcana_text.fonts.select_match_internal :: self, (arcana_text.fonts.selection_request :: style, text, "" :: call) :: call
        if fallback.id.source_index >= 0:
            fonts_probe_append :: "resolve_style_text:fallback_hit" :: call
            return fallback
        fonts_probe_append :: "resolve_style_text:empty" :: call
        return arcana_text.fonts.empty_match :: :: call

    fn resolve_style_text_fallbacks(edit self: arcana_text.fonts.FontSystem, read payload: (arcana_text.types.TextStyle, Str, arcana_text.types.FontMatch)) -> List[arcana_text.types.FontMatch]:
        let style = payload.0
        let text = payload.1
        let primary = payload.2
        let mut scored = arcana_text.fonts.empty_scored_matches :: :: call
        for family in style.families:
            let _ = arcana_text.fonts.ensure_bundled_family_named :: self, family :: call
            let mut request = arcana_text.fonts.selection_request :: style, text, family :: call
            request.restrict_family = true
            let search = arcana_text.fonts.face_search_request_from_selection :: request :: call
            let matches = arcana_text.fonts.collect_matches :: self, search :: call
            for value in matches:
                if arcana_text.fonts.source_is_forbidden_fallback :: value.matched.source, self.locale :: call:
                    continue
                arcana_text.fonts.insert_scored_match :: scored, value :: call
        let fallback_request = arcana_text.fonts.selection_request :: style, text, "" :: call
        let fallback_matches = arcana_text.fonts.collect_matches :: self, (arcana_text.fonts.face_search_request_from_selection :: fallback_request :: call) :: call
        for value in fallback_matches:
            if arcana_text.fonts.source_is_forbidden_fallback :: value.matched.source, self.locale :: call:
                continue
            arcana_text.fonts.insert_scored_match :: scored, value :: call
        return arcana_text.fonts.scored_matches_values :: scored, primary :: call

    fn resolve_style_text_script_fallbacks(edit self: arcana_text.fonts.FontSystem, read payload: (arcana_text.types.TextStyle, Str, arcana_text.types.FontMatch, Str)) -> List[arcana_text.types.FontMatch]:
        let style = payload.0
        let text = payload.1
        let primary = payload.2
        let script_tag = payload.3
        let matches = self :: (style, text, primary) :: resolve_style_text_fallbacks
        return arcana_text.fonts.script_ranked_matches :: self, matches, script_tag :: call

    fn resolve_style_char(edit self: arcana_text.fonts.FontSystem, read style: arcana_text.types.TextStyle, ch: Str) -> arcana_text.types.FontMatch:
        return self :: style, ch :: resolve_style_text

    fn supports_text(edit self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch, text: Str) -> Bool:
        if matched.id.source_index < 0:
            return false
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        return arcana_text.fonts.entry_supports_text :: entry, self.live_faces, text :: call

    fn line_height(edit self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch, read style: arcana_text.types.TextStyle) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        return arcana_text.fonts.line_height_from_entry_metrics :: entry, style :: call

    fn baseline(edit self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch, read style: arcana_text.types.TextStyle) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        return arcana_text.fonts.baseline_from_entry_metrics :: entry, style :: call

    fn glyph_index(edit self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch, text: Str) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.glyph_index_for_face_id :: self.live_faces, face_id, text :: call
            Option.None => -1

    fn gsub_units(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.GsubUnitsRequest) -> List[arcana_text.font_leaf.GsubGlyphUnit]:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, request.matched.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.substitute_gsub_units :: self.live_faces, face_id, request :: call
            Option.None => arcana_text.font_leaf.default_gsub_units :: request.glyphs :: call

    fn pair_adjust(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.PairAdjustRequest) -> arcana_text.font_leaf.PairPlacement:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, request.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.pair_adjust_from_face_id :: self.live_faces, face_id, request :: call
            Option.None => arcana_text.fonts.empty_pair_placement :: :: call

    fn single_adjust(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.SingleAdjustRequest) -> arcana_text.font_leaf.PairPlacement:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, request.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.single_adjust_from_face_id :: self.live_faces, face_id, request :: call
            Option.None => arcana_text.fonts.empty_pair_placement :: :: call

    fn position_lookups(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.PositionLookupsRequest) -> List[arcana_text.font_leaf.GsubLookupRef]:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, request.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.position_lookups_from_face_id :: self.live_faces, face_id, request :: call
            Option.None => std.collections.list.empty[arcana_text.font_leaf.GsubLookupRef] :: :: call

    fn lookup_type(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.LookupTypeRequest) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, request.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.lookup_type_from_face_id :: self.live_faces, face_id, request :: call
            Option.None => 0

    fn lookup_placement(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.LookupPlacementRequest) -> arcana_text.font_leaf.PairPlacement:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, request.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.lookup_placement_from_face_id :: self.live_faces, face_id, request :: call
            Option.None => arcana_text.fonts.empty_pair_placement :: :: call

    fn glyph_class(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.GlyphClassRequest) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, request.id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.glyph_class_from_face_id :: self.live_faces, face_id, request :: call
            Option.None => 0

    fn measure_face_glyph(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.measure_face_glyph_spec :: self.live_faces, face_id, spec :: call
            Option.None => arcana_text.fonts.empty_raster :: spec.font_size :: call

    fn advance_face_glyph(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.advance_face_glyph_spec :: self.live_faces, face_id, spec :: call
            Option.None => arcana_text.fonts.empty_raster :: spec.font_size :: call .advance

    fn render_face_glyph(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.render_face_glyph_spec :: self.live_faces, face_id, spec :: call
            Option.None => arcana_text.fonts.empty_raster :: spec.font_size :: call

    fn underline_metrics(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, font_size: Int) -> (Int, Int):
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.underline_metrics_from_face_id :: self.live_faces, face_id, font_size :: call
            Option.None => (0, 1)

    fn strikethrough_metrics(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, font_size: Int) -> (Int, Int):
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.strikethrough_metrics_from_face_id :: self.live_faces, face_id, font_size :: call
            Option.None => (0, 1)

    fn overline_metrics(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, font_size: Int) -> (Int, Int):
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.overline_metrics_from_face_id :: self.live_faces, face_id, font_size :: call
            Option.None => (0, 1)

fn utf8_codepoint(read text: Str) -> Int:
    let bytes = std.bytes.from_str_utf8 :: text :: call
    let total = std.bytes.len :: bytes :: call
    if total <= 0:
        return 0
    let first = std.bytes.at :: bytes, 0 :: call
    if first < 128:
        return first
    if first < 224 and total >= 2:
        return ((first - 192) * 64) + ((std.bytes.at :: bytes, 1 :: call) - 128)
    if first < 240 and total >= 3:
        return ((first - 224) * 4096) + (((std.bytes.at :: bytes, 1 :: call) - 128) * 64) + ((std.bytes.at :: bytes, 2 :: call) - 128)
    if first < 248 and total >= 4:
        return ((first - 240) * 262144) + (((std.bytes.at :: bytes, 1 :: call) - 128) * 4096) + (((std.bytes.at :: bytes, 2 :: call) - 128) * 64) + ((std.bytes.at :: bytes, 3 :: call) - 128)
    return first
