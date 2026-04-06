import arcana_text.font_leaf
import arcana_text.font_leaf.cmap
import arcana_text.font_leaf.load
import arcana_text.font_leaf.raster
import arcana_text.monaspace
import arcana_text.shape.cache
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

export obj FontSystem:
    source_count: Int
    face_count: Int
    sources: Map[Int, arcana_text.types.FontSource]
    faces: Map[Str, arcana_text.fonts.RegisteredFace]
    path_index: Map[Str, Int]
    source_face_counts: Map[Int, Int]
    source_blob_ids: Map[Int, std.memory.SessionId[Array[Int]]]
    source_blobs: std.memory.SessionArena[Array[Int]]
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

fn empty_sources() -> Map[Int, arcana_text.types.FontSource]:
    return std.collections.map.new[Int, arcana_text.types.FontSource] :: :: call

fn empty_faces() -> Map[Str, arcana_text.fonts.RegisteredFace]:
    return std.collections.map.new[Str, arcana_text.fonts.RegisteredFace] :: :: call

fn empty_path_index() -> Map[Str, Int]:
    return std.collections.map.new[Str, Int] :: :: call

fn empty_source_face_counts() -> Map[Int, Int]:
    return std.collections.map.new[Int, Int] :: :: call

fn empty_source_blob_ids() -> Map[Int, std.memory.SessionId[Array[Int]]]:
    return std.collections.map.new[Int, std.memory.SessionId[Array[Int]]] :: :: call

fn empty_source_blobs() -> std.memory.SessionArena[Array[Int]]:
    return std.memory.session_new[Array[Int]] :: 8 :: call

fn empty_live_faces() -> std.memory.Slab[arcana_text.font_leaf.FontFaceState]:
    return std.memory.slab_new[arcana_text.font_leaf.FontFaceState] :: 8 :: call

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
    return out

fn empty_match() -> arcana_text.types.FontMatch:
    return arcana_text.types.FontMatch :: id = (arcana_text.fonts.invalid_face_id :: :: call), source = (arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), "", "" :: call) :: call

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
    source.bytes = arcana_text.fonts.empty_bytes :: :: call
    return source

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
    let mut stored = value
    if (std.bytes.len :: value.bytes :: call) > 0:
        let blob_id = std.kernel.memory.session_alloc[Array[Int]] :: self.source_blobs, value.bytes :: call
        self.source_blob_ids :: index, blob_id :: set
        stored.bytes = arcana_text.fonts.empty_bytes :: :: call
    let path = stored.path
    if path != "":
        self.path_index :: path, index :: set
    self.sources :: index, stored :: set
    self.source_count = index + 1
    return arcana_text.fonts.register_source_faces :: self, index, face_total :: call

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

fn source_from_bytes(label: Str, read bytes: Array[Int]) -> arcana_text.types.FontSource:
    let mut source = arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), label, "" :: call
    source.family = label
    source.face = "Regular"
    source.full_name = label
    source.postscript_name = label
    source.bytes = bytes
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

fn source_matches_family(read source: arcana_text.types.FontSource, family: Str) -> Bool:
    return source.family == family or source.full_name == family or source.label == family or source.postscript_name == family

fn source_at_or_empty(read self: arcana_text.fonts.FontSystem, index: Int) -> arcana_text.types.FontSource:
    if self.sources :: index :: has:
        return self.sources :: index :: get
    return arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), "", "" :: call

fn source_face_total(read self: arcana_text.fonts.FontSystem, source_index: Int) -> Int:
    if self.source_face_counts :: source_index :: has:
        return self.source_face_counts :: source_index :: get
    return 0

fn first_face_id_for_source(read self: arcana_text.fonts.FontSystem, source_index: Int) -> arcana_text.types.FontFaceId:
    if (arcana_text.fonts.source_face_total :: self, source_index :: call) > 0:
        return arcana_text.fonts.face_id :: source_index, 0 :: call
    return arcana_text.fonts.invalid_face_id :: :: call

fn face_at_or_empty(read self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId) -> arcana_text.fonts.RegisteredFace:
    let key = arcana_text.fonts.face_key :: id :: call
    if self.faces :: key :: has:
        return self.faces :: key :: get
    return arcana_text.fonts.entry_from_source :: id, (arcana_text.fonts.source_base :: (arcana_text.types.FontSourceKind.Bytes :: :: call), "", "" :: call) :: call

fn replace_face_at(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read replacement: arcana_text.fonts.RegisteredFace):
    let key = arcana_text.fonts.face_key :: id :: call
    self.faces :: key, replacement :: set

fn ensure_source_blob(edit self: arcana_text.fonts.FontSystem, index: Int) -> Result[std.memory.SessionId[Array[Int]], Str]:
    if self.source_blob_ids :: index :: has:
        return Result.Ok[std.memory.SessionId[Array[Int]], Str] :: (self.source_blob_ids :: index :: get) :: call
    let source = arcana_text.fonts.source_at_or_empty :: self, index :: call
    let mut bytes_result = Result.Err[Array[Int], Str] :: "font source has no bytes or path" :: call
    if (std.bytes.len :: source.bytes :: call) > 0:
        bytes_result = Result.Ok[Array[Int], Str] :: source.bytes :: call
    else:
        if source.path != "":
            bytes_result = std.fs.read_bytes :: source.path :: call
    if bytes_result :: :: is_err:
        return Result.Err[std.memory.SessionId[Array[Int]], Str] :: (result_err_or :: bytes_result, "failed to read font bytes" :: call) :: call
    let bytes = bytes_result :: (arcana_text.fonts.empty_bytes :: :: call) :: unwrap_or
    let blob_id = std.kernel.memory.session_alloc[Array[Int]] :: self.source_blobs, bytes :: call
    self.source_blob_ids :: index, blob_id :: set
    if (std.bytes.len :: source.bytes :: call) > 0:
        let mut stored = source
        stored.bytes = arcana_text.fonts.empty_bytes :: :: call
        self.sources :: index, stored :: set
    return Result.Ok[std.memory.SessionId[Array[Int]], Str] :: blob_id :: call

fn load_face_from_entry(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read entry: arcana_text.fonts.RegisteredFace) -> Result[arcana_text.font_leaf.FontFaceState, Str]:
    let blob_result = arcana_text.fonts.ensure_source_blob :: self, id.source_index :: call
    if blob_result :: :: is_err:
        return Result.Err[arcana_text.font_leaf.FontFaceState, Str] :: (result_err_or :: blob_result, "failed to stage source blob" :: call) :: call
    let blob_id = blob_result :: ((self.source_blob_ids :: id.source_index :: get)) :: unwrap_or
    let source_bytes = std.kernel.memory.session_borrow_read[Array[Int]] :: self.source_blobs, blob_id :: call
    let bytes_view = std.memory.bytes_view :: source_bytes, 0, (source_bytes :: :: len) :: call
    let mut request = arcana_text.font_leaf.face_load_request :: (arcana_text.fonts.family_or_label :: entry.source :: call), entry.source.label, entry.source.path :: call
    request.traits = entry.traits
    return arcana_text.font_leaf.load.load_face_from_view :: request, bytes_view :: call

fn failed_face_entry(read entry: arcana_text.fonts.RegisteredFace, err: Str) -> arcana_text.fonts.RegisteredFace:
    let mut next = entry
    record place arcana_text.fonts.RegisteredFace from entry -> next -return next
        load_error = err
    return next

fn ensure_loaded_face(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId) -> arcana_text.fonts.RegisteredFace:
    let entry = arcana_text.fonts.face_at_or_empty :: self, id :: call
    if (entry.face_id :: :: is_some) or entry.load_error != "":
        return entry
    let loaded_result = arcana_text.fonts.load_face_from_entry :: self, id, entry :: call
    let next = match loaded_result:
        std.result.Result.Ok(face) => ensure_loaded_face_ok :: self, entry, face :: call
        std.result.Result.Err(err) => arcana_text.fonts.failed_face_entry :: entry, err :: call
    arcana_text.fonts.replace_face_at :: self, id, next :: call
    return next

fn ensure_loaded_face_ok(edit self: arcana_text.fonts.FontSystem, read entry: arcana_text.fonts.RegisteredFace, read face: arcana_text.font_leaf.FontFaceState) -> arcana_text.fonts.RegisteredFace:
    let traits = arcana_text.fonts.face_traits_from_face :: face :: call
    let units_per_em = face.units_per_em
    let ascender = face.ascender
    let descender = face.descender
    let line_gap = face.line_gap
    let face_id = std.kernel.memory.slab_alloc[arcana_text.font_leaf.FontFaceState] :: self.live_faces, face :: call
    let mut next = entry
    record place arcana_text.fonts.RegisteredFace from entry -> next -return next
        traits = traits
        face_id = Option.Some[std.memory.SlabId[arcana_text.font_leaf.FontFaceState]] :: face_id :: call
        load_error = ""
        units_per_em = units_per_em
        ascender = ascender
        descender = descender
        line_gap = line_gap
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

fn line_height_from_face_id(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read style: arcana_text.types.TextStyle) -> Int:
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.line_height_for_face :: face, style.size, (arcana_text.fonts.line_height_milli :: style :: call) :: call

fn baseline_from_face_id(read live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read style: arcana_text.types.TextStyle) -> Int:
    let face = std.kernel.memory.slab_borrow_read :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.baseline_for_face :: face, style.size, (arcana_text.fonts.line_height_milli :: style :: call) :: call

fn face_supports_text(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], text: Str) -> Bool:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.cmap.supports_text :: face, text :: call

fn measure_face_glyph_spec(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.measure_glyph :: face, spec :: call

fn render_face_glyph_spec(edit live_faces: std.memory.Slab[arcana_text.font_leaf.FontFaceState], read face_id: std.memory.SlabId[arcana_text.font_leaf.FontFaceState], read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
    let mut face = std.kernel.memory.slab_borrow_edit :: live_faces, face_id :: call
    return arcana_text.font_leaf.raster.render_glyph :: face, spec :: call

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

fn query_matches_family(read query: arcana_text.types.FontQuery, read source: arcana_text.types.FontSource) -> Bool:
    if query.families :: :: is_empty:
        return true
    for family in query.families:
        if arcana_text.fonts.source_matches_family :: source, family :: call:
            return true
    return false

fn select_match_internal(edit self: arcana_text.fonts.FontSystem, read request: arcana_text.fonts.FontSelectionRequest) -> arcana_text.types.FontMatch:
    let target = arcana_text.fonts.style_traits :: request.style :: call
    let mut best_score = 2147483647
    let mut selected = arcana_text.fonts.empty_match :: :: call
    let mut source_index = 0
    while source_index < self.source_count:
        let source = arcana_text.fonts.source_at_or_empty :: self, source_index :: call
        if request.restrict_family and not (arcana_text.fonts.source_matches_family :: source, request.family :: call):
            source_index += 1
            continue
        let total_faces = arcana_text.fonts.source_face_total :: self, source_index :: call
        let mut face_index = 0
        while face_index < total_faces:
            let id = arcana_text.fonts.face_id :: source_index, face_index :: call
            let loaded = arcana_text.fonts.ensure_loaded_face :: self, id :: call
            let supports = arcana_text.fonts.entry_supports_text :: loaded, self.live_faces, request.text :: call
            if supports:
                let score = arcana_text.fonts.face_traits_score :: loaded.traits, target :: call
                if selected.id.source_index < 0 or score < best_score:
                    best_score = score
                    selected = arcana_text.types.FontMatch :: id = id, source = source :: call
            face_index += 1
        source_index += 1
    return selected

export fn new_system() -> arcana_text.fonts.FontSystem:
    let mut system = arcana_text.fonts.FontSystem :: source_count = 0, face_count = 0, sources = (arcana_text.fonts.empty_sources :: :: call) :: call
    system.faces = arcana_text.fonts.empty_faces :: :: call
    system.path_index = arcana_text.fonts.empty_path_index :: :: call
    system.source_face_counts = arcana_text.fonts.empty_source_face_counts :: :: call
    system.source_blob_ids = arcana_text.fonts.empty_source_blob_ids :: :: call
    system.source_blobs = arcana_text.fonts.empty_source_blobs :: :: call
    system.live_faces = arcana_text.fonts.empty_live_faces :: :: call
    system.shape_cache = arcana_text.shape.cache.open :: :: call
    system.discovered = false
    return system

export fn default_system() -> arcana_text.fonts.FontSystem:
    let mut out = arcana_text.fonts.new_system :: :: call
    out :: :: register_bundled_defaults
    return out

impl FontSystem:
    fn count(read self: arcana_text.fonts.FontSystem) -> Int:
        return self.face_count

    fn source_at(read self: arcana_text.fonts.FontSystem, index: Int) -> arcana_text.types.FontSource:
        return arcana_text.fonts.source_at_or_empty :: self, index :: call

    fn register_bundled_defaults(edit self: arcana_text.fonts.FontSystem) -> Int:
        let mut added = 0
        added += (self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Argon :: :: call) :: call) :: add_if_file)
        added += (self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Krypton :: :: call) :: call) :: add_if_file)
        added += (self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Neon :: :: call) :: call) :: add_if_file)
        added += (self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Radon :: :: call) :: call) :: add_if_file)
        added += (self :: (arcana_text.monaspace.variable_font_path :: (arcana_text.monaspace.MonaspaceFamily.Xenon :: :: call) :: call) :: add_if_file)
        return added

    fn add_if_file(edit self: arcana_text.fonts.FontSystem, path: Str) -> Int:
        if not (std.fs.is_file :: path :: call):
            return 0
        let _ = self :: path :: add_file
        return 1

    fn add_file(edit self: arcana_text.fonts.FontSystem, path: Str) -> arcana_text.types.FontFaceId:
        if not (std.fs.is_file :: path :: call):
            return arcana_text.fonts.invalid_face_id :: :: call
        if self.path_index :: path :: has:
            let existing = self.path_index :: path :: get
            return arcana_text.fonts.first_face_id_for_source :: self, existing :: call
        return arcana_text.fonts.add_source_value :: self, (arcana_text.fonts.source_from_file_path :: path :: call) :: call

    fn add_dir(edit self: arcana_text.fonts.FontSystem, path: Str) -> Int:
        if not (std.fs.is_dir :: path :: call):
            return 0
        let mut added = 0
        let entries = match (std.fs.list_dir :: path :: call):
            std.result.Result.Ok(value) => value
            std.result.Result.Err(_) => (std.collections.list.new[Str] :: :: call)
        for entry in entries:
            let ext = std.text.trim :: (std.path.ext :: entry :: call) :: call
            if ext == ".ttf" or ext == ".otf":
                let _ = self :: entry :: add_file
                added += 1
        return added

    fn add_bytes(edit self: arcana_text.fonts.FontSystem, label: Str, read bytes: Array[Int]) -> arcana_text.types.FontFaceId:
        return arcana_text.fonts.add_source_value :: self, (arcana_text.fonts.source_from_bytes :: label, bytes :: call) :: call

    fn discover_installed(edit self: arcana_text.fonts.FontSystem) -> Int:
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
        let target = arcana_text.fonts.query_traits :: query :: call
        let mut best_score = 2147483647
        let mut selected = arcana_text.fonts.empty_match :: :: call
        let mut source_index = 0
        while source_index < self.source_count:
            let source = arcana_text.fonts.source_at_or_empty :: self, source_index :: call
            if not (arcana_text.fonts.query_matches_family :: query, source :: call):
                source_index += 1
                continue
            let total_faces = arcana_text.fonts.source_face_total :: self, source_index :: call
            let mut face_index = 0
            while face_index < total_faces:
                let id = arcana_text.fonts.face_id :: source_index, face_index :: call
                let loaded = arcana_text.fonts.ensure_loaded_face :: self, id :: call
                let score = arcana_text.fonts.face_traits_score :: loaded.traits, target :: call
                if selected.id.source_index < 0 or score < best_score:
                    best_score = score
                    selected = arcana_text.types.FontMatch :: id = id, source = source :: call
                face_index += 1
            source_index += 1
        if selected.id.source_index >= 0:
            return selected
        if self.source_count > 0:
            let id = arcana_text.fonts.first_face_id_for_source :: self, 0 :: call
            return arcana_text.types.FontMatch :: id = id, source = (arcana_text.fonts.source_at_or_empty :: self, 0 :: call) :: call
        return arcana_text.fonts.empty_match :: :: call

    fn resolve_style_char(edit self: arcana_text.fonts.FontSystem, read style: arcana_text.types.TextStyle, ch: Str) -> arcana_text.types.FontMatch:
        for family in style.families:
            let mut request = arcana_text.fonts.selection_request :: style, ch, family :: call
            request.restrict_family = true
            let matched = arcana_text.fonts.select_match_internal :: self, request :: call
            if matched.id.source_index >= 0:
                return matched
        let fallback = arcana_text.fonts.select_match_internal :: self, (arcana_text.fonts.selection_request :: style, ch, "" :: call) :: call
        if fallback.id.source_index >= 0:
            return fallback
        if self.source_count > 0:
            let id = arcana_text.fonts.first_face_id_for_source :: self, 0 :: call
            return arcana_text.types.FontMatch :: id = id, source = (arcana_text.fonts.source_at_or_empty :: self, 0 :: call) :: call
        return arcana_text.fonts.empty_match :: :: call

    fn line_height(edit self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch, read style: arcana_text.types.TextStyle) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        return arcana_text.fonts.line_height_from_entry_metrics :: entry, style :: call

    fn baseline(edit self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch, read style: arcana_text.types.TextStyle) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        return arcana_text.fonts.baseline_from_entry_metrics :: entry, style :: call

    fn glyph_index(edit self: arcana_text.fonts.FontSystem, read matched: arcana_text.types.FontMatch, text: Str) -> Int:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, matched.id :: call
        if text == " " or text == "\t" or text == "\n" or text == "\r":
            return 0
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.glyph_index_for_face_id :: self.live_faces, face_id, text :: call
            Option.None => -1

    fn measure_face_glyph(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.measure_face_glyph_spec :: self.live_faces, face_id, spec :: call
            Option.None => arcana_text.fonts.empty_raster :: spec.font_size :: call

    fn render_face_glyph(edit self: arcana_text.fonts.FontSystem, read id: arcana_text.types.FontFaceId, read spec: arcana_text.font_leaf.GlyphRenderSpec) -> arcana_text.font_leaf.GlyphBitmap:
        let entry = arcana_text.fonts.ensure_loaded_face :: self, id :: call
        return match entry.face_id:
            Option.Some(face_id) => arcana_text.fonts.render_face_glyph_spec :: self.live_faces, face_id, spec :: call
            Option.None => arcana_text.fonts.empty_raster :: spec.font_size :: call

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
