import arcana_text.buffer
import arcana_text.cache
import arcana_text.font_leaf
import arcana_text.fonts
import arcana_text.layout
import arcana_text.types
import std.collections.array
import std.collections.list
import std.option
import std.text
use std.option.Option

record RasterScratch:
    value: Int

record RasterPassContext:
    snapshot_version: Int
    config: arcana_text.types.RasterConfig

export obj GlyphDrawStream:
    source_version: Int
    size: (Int, Int)
    glyphs: List[arcana_text.types.GlyphDraw]
    images: List[arcana_text.types.GlyphImageDraw]

export obj TextRenderer:
    fonts: arcana_text.fonts.FontSystem
    cache: arcana_text.cache.TextCache

obj RasterPassState:
    done: Bool
    snapshot_version: Int
    config: arcana_text.types.RasterConfig
    fn init(edit self: Self, read ctx: RasterPassContext):
        self.done = false
        self.snapshot_version = ctx.snapshot_version
        self.config = ctx.config
    fn resume(edit self: Self, read ctx: RasterPassContext):
        self.done = false
        self.snapshot_version = ctx.snapshot_version
        self.config = ctx.config

create RasterPass [RasterPassState] context: RasterPassContext scope-exit:
    done: when RasterPassState.done

Memory temp:raster_scratch -alloc
    capacity = 64
    reset_on = owner_exit

fn in_clip(read clip: arcana_text.types.TextRange, read glyph: arcana_text.types.LayoutGlyph) -> Bool:
    if clip.end <= clip.start:
        return true
    return glyph.range.start < clip.end and clip.start < glyph.range.end

fn raster_mode_key(read mode: arcana_text.types.RasterMode) -> Str:
    return match mode:
        arcana_text.types.RasterMode.Lcd => "lcd"
        arcana_text.types.RasterMode.Color => "color"
        _ => "alpha"

fn glyph_surface_key(read payload: (arcana_text.types.LayoutGlyph, arcana_text.types.RasterMode)) -> Str:
    let glyph = payload.0
    let mode = payload.1
    return (std.text.from_int :: glyph.face_id.source_index :: call) + ":" + (std.text.from_int :: glyph.face_id.face_index :: call) + ":" + (std.text.from_int :: glyph.glyph_index :: call) + ":" + (std.text.from_int :: glyph.font_size :: call) + ":" + (std.text.from_int :: glyph.line_height_milli :: call) + ":" + (std.text.from_int :: glyph.weight :: call) + ":" + (std.text.from_int :: glyph.width_milli :: call) + ":" + (std.text.from_int :: glyph.slant_milli :: call) + ":" + (arcana_text.raster.raster_mode_key :: mode :: call)

fn rgba_from_bitmap(read bitmap: arcana_text.font_leaf.GlyphBitmap, color: Int) -> Array[Int]:
    let mut rgba = std.collections.list.empty[Int] :: :: call
    let red = (color / 65536) % 256
    let green = (color / 256) % 256
    let blue = color % 256
    let mut index = 0
    let total = bitmap.alpha :: :: len
    while index < total:
        let alpha = (bitmap.alpha)[index]
        rgba :: red :: push
        rgba :: green :: push
        rgba :: blue :: push
        rgba :: alpha :: push
        index += 1
    return std.collections.array.from_list[Int] :: rgba :: call

fn empty_surface() -> arcana_text.types.GlyphSurface:
    let mut surface = arcana_text.types.GlyphSurface :: size = (0, 0), stride = 0, format = (arcana_text.types.GlyphSurfaceFormat.Alpha8 :: :: call) :: call
    surface.pixels = std.collections.array.empty[Int] :: :: call
    return surface

fn surface_from_bitmap(read bitmap: arcana_text.font_leaf.GlyphBitmap, mode: arcana_text.types.RasterMode) -> arcana_text.types.GlyphSurface:
    let mut surface = arcana_text.types.GlyphSurface :: size = bitmap.size, stride = bitmap.size.0, format = (arcana_text.types.GlyphSurfaceFormat.Alpha8 :: :: call) :: call
    surface.pixels = bitmap.alpha
    if mode == (arcana_text.types.RasterMode.Lcd :: :: call):
        surface.format = arcana_text.types.GlyphSurfaceFormat.LcdSubpixel :: :: call
    return surface

fn rendered_glyph_surface(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read payload: (arcana_text.types.LayoutGlyph, arcana_text.types.RasterMode)) -> Option[arcana_text.types.GlyphSurface]:
    let glyph = payload.0
    let mode = payload.1
    if glyph.face_id.source_index < 0 or glyph.glyph_index <= 0 or glyph.ink_size.0 <= 0 or glyph.ink_size.1 <= 0:
        return Option.None[arcana_text.types.GlyphSurface] :: :: call
    let key = arcana_text.raster.glyph_surface_key :: (glyph, mode) :: call
    let cached = cache :: key :: cached_glyph_surface
    if cached :: :: is_some:
        return cached
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: glyph.glyph, glyph.font_size, glyph.line_height_milli :: call
    spec.glyph_index = glyph.glyph_index
    spec.traits = arcana_text.font_leaf.FaceTraits :: weight = glyph.weight, width_milli = glyph.width_milli, slant_milli = glyph.slant_milli :: call
    spec.feature_signature = 0
    spec.axis_signature = 0
    let bitmap = fonts :: glyph.face_id, spec :: render_face_glyph
    if bitmap.empty or bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
        return Option.None[arcana_text.types.GlyphSurface] :: :: call
    let surface = arcana_text.raster.surface_from_bitmap :: bitmap, mode :: call
    cache :: key, surface :: remember_glyph_surface
    return cache :: key :: cached_glyph_surface

fn rendered_glyph_surface_renderer(edit self: arcana_text.raster.TextRenderer, read payload: (arcana_text.types.LayoutGlyph, arcana_text.types.RasterMode)) -> Option[arcana_text.types.GlyphSurface]:
    let glyph = payload.0
    let mode = payload.1
    if glyph.face_id.source_index < 0 or glyph.glyph_index <= 0 or glyph.ink_size.0 <= 0 or glyph.ink_size.1 <= 0:
        return Option.None[arcana_text.types.GlyphSurface] :: :: call
    let key = arcana_text.raster.glyph_surface_key :: (glyph, mode) :: call
    let cached = self.cache :: key :: cached_glyph_surface
    if cached :: :: is_some:
        return cached
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: glyph.glyph, glyph.font_size, glyph.line_height_milli :: call
    spec.glyph_index = glyph.glyph_index
    spec.traits = arcana_text.font_leaf.FaceTraits :: weight = glyph.weight, width_milli = glyph.width_milli, slant_milli = glyph.slant_milli :: call
    spec.feature_signature = 0
    spec.axis_signature = 0
    let bitmap = self.fonts :: glyph.face_id, spec :: render_face_glyph
    if bitmap.empty or bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
        return Option.None[arcana_text.types.GlyphSurface] :: :: call
    let surface = arcana_text.raster.surface_from_bitmap :: bitmap, mode :: call
    self.cache :: key, surface :: remember_glyph_surface
    return self.cache :: key :: cached_glyph_surface

fn glyph_image_draws(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> List[arcana_text.types.GlyphImageDraw]:
    let snapshot = payload.0
    let config = payload.1
    let mut out = std.collections.list.empty[arcana_text.types.GlyphImageDraw] :: :: call
    for glyph in snapshot.glyphs:
        if not (arcana_text.raster.in_clip :: config.clip_range, glyph :: call):
            continue
        if glyph.glyph == " " or glyph.glyph == "\t" or glyph.glyph == "\n" or glyph.glyph == "\r":
            continue
        let surface = arcana_text.raster.rendered_glyph_surface :: fonts, cache, (glyph, config.mode) :: call
        if surface :: :: is_some:
            let value = surface :: (arcana_text.raster.empty_surface :: :: call) :: unwrap_or
            let mut draw = arcana_text.types.GlyphImageDraw :: position = (glyph.position.0 + glyph.ink_offset.0, glyph.position.1 + glyph.ink_offset.1), size = glyph.ink_size, mode = config.mode :: call
            draw.color = glyph.color
            draw.surface = value
            out :: draw :: push
    return out

fn glyph_image_draws_renderer(edit self: arcana_text.raster.TextRenderer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> List[arcana_text.types.GlyphImageDraw]:
    let snapshot = payload.0
    let config = payload.1
    let mut out = std.collections.list.empty[arcana_text.types.GlyphImageDraw] :: :: call
    for glyph in snapshot.glyphs:
        if not (arcana_text.raster.in_clip :: config.clip_range, glyph :: call):
            continue
        if glyph.glyph == " " or glyph.glyph == "\t" or glyph.glyph == "\n" or glyph.glyph == "\r":
            continue
        let surface = arcana_text.raster.rendered_glyph_surface_renderer :: self, (glyph, config.mode) :: call
        if surface :: :: is_some:
            let value = surface :: (arcana_text.raster.empty_surface :: :: call) :: unwrap_or
            let mut draw = arcana_text.types.GlyphImageDraw :: position = (glyph.position.0 + glyph.ink_offset.0, glyph.position.1 + glyph.ink_offset.1), size = glyph.ink_size, mode = config.mode :: call
            draw.color = glyph.color
            draw.surface = value
            out :: draw :: push
    return out

fn glyph_draws(read snapshot: arcana_text.layout.LayoutSnapshot, read config: arcana_text.types.RasterConfig) -> List[arcana_text.types.GlyphDraw]:
    let mut glyphs = std.collections.list.empty[arcana_text.types.GlyphDraw] :: :: call
    for glyph in snapshot.glyphs:
        if not (arcana_text.raster.in_clip :: config.clip_range, glyph :: call):
            continue
        let mut draw = arcana_text.types.GlyphDraw :: text = glyph.glyph, position = glyph.position, size = glyph.size :: call
        draw.color = glyph.color
        draw.background_enabled = glyph.background_enabled
        draw.background_color = glyph.background_color
        draw.family = glyph.family
        glyphs :: draw :: push
    return glyphs

RasterPass
RasterPassState
fn draw_stream_active(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
    let snapshot = payload.0
    let config = payload.1
    let _scratch = temp: arcana_text.raster.raster_scratch :> value = snapshot.source_version <: arcana_text.raster.RasterScratch
    let glyphs = arcana_text.raster.glyph_draws :: snapshot, config :: call
    let images = arcana_text.raster.glyph_image_draws :: fonts, cache, (snapshot, config) :: call
    cache :: snapshot.source_version :: touch_snapshot
    RasterPassState.done = true
    let mut out = arcana_text.raster.GlyphDrawStream :: source_version = snapshot.source_version, size = snapshot.size, glyphs = glyphs :: call
    out.images = images
    return out

fn draw_stream_active_renderer(edit self: arcana_text.raster.TextRenderer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
    let snapshot = payload.0
    let config = payload.1
    let _scratch = temp: arcana_text.raster.raster_scratch :> value = snapshot.source_version <: arcana_text.raster.RasterScratch
    let glyphs = arcana_text.raster.glyph_draws :: snapshot, config :: call
    let images = arcana_text.raster.glyph_image_draws_renderer :: self, (snapshot, config) :: call
    self.cache :: snapshot.source_version :: touch_snapshot
    RasterPassState.done = true
    let mut out = arcana_text.raster.GlyphDrawStream :: source_version = snapshot.source_version, size = snapshot.size, glyphs = glyphs :: call
    out.images = images
    return out

export fn default_renderer() -> arcana_text.raster.TextRenderer:
    return arcana_text.raster.TextRenderer :: fonts = (arcana_text.fonts.default_system :: :: call), cache = (arcana_text.cache.open :: :: call) :: call

export fn draw_stream(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read request: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
    let snapshot = request.0
    let config = request.1
    let ctx = arcana_text.raster.RasterPassContext :: snapshot_version = snapshot.source_version, config = config :: call
    let active = RasterPass :: ctx :: call
    let _ = active
    return arcana_text.raster.draw_stream_active :: fonts, cache, (snapshot, config) :: call

impl TextRenderer:
    fn snapshot(edit self: arcana_text.raster.TextRenderer, read buffer: arcana_text.buffer.TextBuffer, read config: arcana_text.types.LayoutConfig) -> arcana_text.layout.LayoutSnapshot:
        return arcana_text.layout.snapshot :: self.fonts, buffer, config :: call

    fn draw_stream(edit self: arcana_text.raster.TextRenderer, read request: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
        let snapshot = request.0
        let config = request.1
        let ctx = arcana_text.raster.RasterPassContext :: snapshot_version = snapshot.source_version, config = config :: call
        let active = RasterPass :: ctx :: call
        let _ = active
        return arcana_text.raster.draw_stream_active_renderer :: self, (snapshot, config) :: call
