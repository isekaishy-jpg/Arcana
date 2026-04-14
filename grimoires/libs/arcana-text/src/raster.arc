import arcana_text.buffer
import arcana_text.cache
import arcana_text.font_leaf
import arcana_text.fonts
import arcana_text.layout
import arcana_text.shape.types
import arcana_text.types
import std.text
import std.collections.array
import std.collections.list
import std.collections.map
import arcana_process.fs
import std.option
import arcana_process.path
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
    decorations: List[arcana_text.types.DecorationDraw]
    images: List[arcana_text.types.GlyphImageDraw]

export obj TextRenderer:
    fonts: arcana_text.fonts.FontSystem
    cache: arcana_text.cache.TextCache
    layout_snapshots: Map[Str, arcana_text.layout.LayoutSnapshot]
    draw_streams: Map[Str, arcana_text.raster.GlyphDrawStream]

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

fn raster_probe_flag_path() -> Str:
    return arcana_process.path.join :: (arcana_process.path.join :: (arcana_process.path.cwd :: :: call), "scratch" :: call), "enable_text_fonts_probe" :: call

fn raster_probe_log_path() -> Str:
    return arcana_process.path.join :: (arcana_process.path.join :: (arcana_process.path.cwd :: :: call), "scratch" :: call), "text_raster_probe.log" :: call

fn raster_probe_enabled() -> Bool:
    return arcana_process.fs.is_file :: (raster_probe_flag_path :: :: call) :: call

fn raster_probe_append(line: Str):
    if not (raster_probe_enabled :: :: call):
        return
    let _ = arcana_process.fs.mkdir_all :: (arcana_process.path.parent :: (raster_probe_log_path :: :: call) :: call) :: call
    let opened = arcana_process.fs.stream_open_write :: (raster_probe_log_path :: :: call), true :: call
    return match opened:
        std.result.Result.Ok(value) => raster_probe_append_ready :: value, line :: call
        std.result.Result.Err(_) => 0

fn raster_probe_append_ready(take value: arcana_winapi.process_handles.FileStream, line: Str):
    let mut stream = value
    let bytes = std.text.bytes_from_str_utf8 :: (line + "\n") :: call
    let _ = arcana_process.fs.stream_write :: stream, bytes :: call
    let _ = arcana_process.fs.stream_close :: stream :: call

fn max_int(a: Int, b: Int) -> Int:
    if a > b:
        return a
    return b

fn in_clip(read clip: arcana_text.types.TextRange, read glyph: arcana_text.types.LayoutGlyph) -> Bool:
    if clip.end <= clip.start:
        return true
    return glyph.range.start < clip.end and clip.start < glyph.range.end

fn raster_mode_key(read mode: arcana_text.types.RasterMode) -> Str:
    return match mode:
        arcana_text.types.RasterMode.Lcd => "lcd"
        arcana_text.types.RasterMode.Color => "color"
        _ => "alpha"

fn layout_config_signature(read config: arcana_text.types.LayoutConfig) -> Int:
    let mut signature = 97
    signature = arcana_text.shape.types.mix_signature :: signature, config.max_width :: call
    signature = arcana_text.shape.types.mix_signature :: signature, config.max_lines :: call
    signature = arcana_text.shape.types.mix_signature :: signature, config.tab_width :: call
    signature = arcana_text.shape.types.mix_signature_text :: signature, config.ellipsis :: call
    signature = match config.align:
        arcana_text.types.TextAlign.Center => arcana_text.shape.types.mix_signature :: signature, 2 :: call
        arcana_text.types.TextAlign.Right => arcana_text.shape.types.mix_signature :: signature, 3 :: call
        arcana_text.types.TextAlign.Justified => arcana_text.shape.types.mix_signature :: signature, 4 :: call
        arcana_text.types.TextAlign.End => arcana_text.shape.types.mix_signature :: signature, 5 :: call
        _ => arcana_text.shape.types.mix_signature :: signature, 1 :: call
    signature = match config.wrap:
        arcana_text.types.TextWrap.NoWrap => arcana_text.shape.types.mix_signature :: signature, 5 :: call
        arcana_text.types.TextWrap.Glyph => arcana_text.shape.types.mix_signature :: signature, 6 :: call
        arcana_text.types.TextWrap.WordOrGlyph => arcana_text.shape.types.mix_signature :: signature, 7 :: call
        _ => arcana_text.shape.types.mix_signature :: signature, 4 :: call
    signature = match config.ellipsize_mode:
        arcana_text.types.EllipsizeMode.Start => arcana_text.shape.types.mix_signature :: signature, 8 :: call
        arcana_text.types.EllipsizeMode.Middle => arcana_text.shape.types.mix_signature :: signature, 9 :: call
        arcana_text.types.EllipsizeMode.End => arcana_text.shape.types.mix_signature :: signature, 10 :: call
        _ => arcana_text.shape.types.mix_signature :: signature, 11 :: call
    signature = arcana_text.shape.types.mix_signature :: signature, config.ellipsize_limit.value :: call
    signature = match config.ellipsize_limit.kind:
        arcana_text.types.EllipsizeLimitKind.Lines => arcana_text.shape.types.mix_signature :: signature, 12 :: call
        arcana_text.types.EllipsizeLimitKind.Height => arcana_text.shape.types.mix_signature :: signature, 13 :: call
        _ => arcana_text.shape.types.mix_signature :: signature, 14 :: call
    signature = match config.hinting:
        arcana_text.types.Hinting.Enabled => arcana_text.shape.types.mix_signature :: signature, 15 :: call
        _ => arcana_text.shape.types.mix_signature :: signature, 16 :: call
    return signature

fn layout_snapshot_key(read renderer: arcana_text.raster.TextRenderer, read buffer: arcana_text.buffer.TextBuffer, read config: arcana_text.types.LayoutConfig) -> Str:
    let signature = arcana_text.raster.layout_config_signature :: config :: call
    return (std.text.from_int :: buffer.version :: call) + ":" + (std.text.from_int :: (renderer.fonts :: :: count) :: call) + ":" + (std.text.from_int :: renderer.fonts.shape_cache.generation :: call) + ":" + (renderer.fonts :: :: locale) + ":" + (std.text.from_int :: signature :: call)

fn empty_layout_snapshot_cache() -> Map[Str, arcana_text.layout.LayoutSnapshot]:
    return std.collections.map.empty[Str, arcana_text.layout.LayoutSnapshot] :: :: call

fn empty_draw_stream_cache() -> Map[Str, arcana_text.raster.GlyphDrawStream]:
    return std.collections.map.empty[Str, arcana_text.raster.GlyphDrawStream] :: :: call

fn layout_snapshot_cache_limit() -> Int:
    return 32

fn draw_stream_cache_limit() -> Int:
    return 32

fn raster_config_signature(read config: arcana_text.types.RasterConfig) -> Int:
    let mut signature = 131
    signature = match config.mode:
        arcana_text.types.RasterMode.Lcd => arcana_text.shape.types.mix_signature :: signature, 2 :: call
        arcana_text.types.RasterMode.Color => arcana_text.shape.types.mix_signature :: signature, 3 :: call
        _ => arcana_text.shape.types.mix_signature :: signature, 1 :: call
    signature = arcana_text.shape.types.mix_signature :: signature, config.clip_range.start :: call
    signature = arcana_text.shape.types.mix_signature :: signature, config.clip_range.end :: call
    signature = match config.draw_backgrounds:
        true => arcana_text.shape.types.mix_signature :: signature, 5 :: call
        false => arcana_text.shape.types.mix_signature :: signature, 4 :: call
    signature = match config.hinting:
        arcana_text.types.Hinting.Enabled => arcana_text.shape.types.mix_signature :: signature, 6 :: call
        _ => arcana_text.shape.types.mix_signature :: signature, 7 :: call
    return signature

fn draw_stream_key(read snapshot: arcana_text.layout.LayoutSnapshot, read config: arcana_text.types.RasterConfig) -> Str:
    return (std.text.from_int :: snapshot.source_version :: call) + ":" + (std.text.from_int :: snapshot.signature :: call) + ":" + (std.text.from_int :: (arcana_text.raster.raster_config_signature :: config :: call) :: call)

fn glyph_surface_key(read payload: (arcana_text.types.LayoutGlyph, arcana_text.types.RasterMode, arcana_text.types.Hinting)) -> Str:
    let glyph = payload.0
    let mode = payload.1
    let hinting = payload.2
    let hint_key = match hinting:
        arcana_text.types.Hinting.Enabled => "hint"
        _ => "plain"
    return (std.text.from_int :: glyph.face_id.source_index :: call) + ":" + (std.text.from_int :: glyph.face_id.face_index :: call) + ":" + (std.text.from_int :: glyph.glyph_index :: call) + ":" + (std.text.from_int :: glyph.font_size :: call) + ":" + (std.text.from_int :: glyph.line_height_milli :: call) + ":" + (std.text.from_int :: glyph.weight :: call) + ":" + (std.text.from_int :: glyph.width_milli :: call) + ":" + (std.text.from_int :: glyph.slant_milli :: call) + ":" + (std.text.from_int :: glyph.feature_signature :: call) + ":" + (std.text.from_int :: glyph.axis_signature :: call) + ":" + (std.text.from_int :: glyph.color :: call) + ":" + (arcana_text.raster.raster_mode_key :: mode :: call) + ":" + hint_key

fn rgba_from_bitmap(read bitmap: arcana_text.font_leaf.GlyphBitmap, color: Int) -> Array[Int]:
    if (bitmap.rgba :: :: len) > 0:
        return bitmap.rgba
    let mut rgba = std.collections.list.new[Int] :: :: call
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

fn empty_cached_glyph_image() -> arcana_text.cache.CachedGlyphImage:
    let mut cached = arcana_text.cache.CachedGlyphImage :: offset = (0, 0), surface = (arcana_text.raster.empty_surface :: :: call) :: call
    return cached

fn bitmap_alpha_at(read bitmap: arcana_text.font_leaf.GlyphBitmap, x: Int, y: Int) -> Int:
    if x < 0 or y < 0 or x >= bitmap.size.0 or y >= bitmap.size.1:
        return 0
    return (bitmap.alpha)[(y * bitmap.size.0) + x]

fn lcd_pixels_from_bitmap(read bitmap: arcana_text.font_leaf.GlyphBitmap) -> Array[Int]:
    if (bitmap.lcd :: :: len) > 0:
        return bitmap.lcd
    let width = bitmap.size.0
    let height = bitmap.size.1
    let mut pixels = std.collections.list.new[Int] :: :: call
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let left = arcana_text.raster.bitmap_alpha_at :: bitmap, x - 1, y :: call
            let center = arcana_text.raster.bitmap_alpha_at :: bitmap, x, y :: call
            let right = arcana_text.raster.bitmap_alpha_at :: bitmap, x + 1, y :: call
            pixels :: (((center * 2) + left) / 3) :: push
            pixels :: center :: push
            pixels :: (((center * 2) + right) / 3) :: push
            x += 1
        y += 1
    return std.collections.array.from_list[Int] :: pixels :: call

fn surface_from_bitmap(read bitmap: arcana_text.font_leaf.GlyphBitmap, mode: arcana_text.types.RasterMode) -> arcana_text.types.GlyphSurface:
    let mut surface = arcana_text.types.GlyphSurface :: size = bitmap.size, stride = bitmap.size.0, format = (arcana_text.types.GlyphSurfaceFormat.Alpha8 :: :: call) :: call
    surface.pixels = bitmap.alpha
    if mode == (arcana_text.types.RasterMode.Color :: :: call) and (bitmap.rgba :: :: len) > 0:
        surface.format = arcana_text.types.GlyphSurfaceFormat.Rgba8 :: :: call
        surface.stride = bitmap.size.0
        surface.pixels = bitmap.rgba
        return surface
    if mode == (arcana_text.types.RasterMode.Lcd :: :: call):
        surface.format = arcana_text.types.GlyphSurfaceFormat.LcdSubpixel :: :: call
        surface.stride = bitmap.size.0
        surface.pixels = arcana_text.raster.lcd_pixels_from_bitmap :: bitmap :: call
    if mode == (arcana_text.types.RasterMode.Color :: :: call):
        surface.format = arcana_text.types.GlyphSurfaceFormat.Alpha8 :: :: call
        surface.stride = bitmap.size.0
        surface.pixels = bitmap.alpha
    return surface

fn empty_image_draw(read mode: arcana_text.types.RasterMode) -> arcana_text.types.GlyphImageDraw:
    let mut draw = arcana_text.types.GlyphImageDraw :: position = (0, 0), size = (0, 0), mode = mode :: call
    draw.color = 0
    draw.surface = arcana_text.raster.empty_surface :: :: call
    return draw

fn empty_decoration_draws() -> List[arcana_text.types.DecorationDraw]:
    return std.collections.list.new[arcana_text.types.DecorationDraw] :: :: call

fn decoration_color(base: Int, enabled: Bool, override: Int) -> Int:
    if enabled:
        return override
    return base

fn append_decoration(edit out: List[arcana_text.types.DecorationDraw], read payload: ((Int, Int), (Int, Int), Int)):
    let position = payload.0
    let size = payload.1
    let color = payload.2
    if size.0 <= 0 or size.1 <= 0:
        return
    out :: (arcana_text.types.DecorationDraw :: position = position, size = size, color = color :: call) :: push

fn glyph_render_spec(read glyph: arcana_text.types.LayoutGlyph) -> arcana_text.font_leaf.GlyphRenderSpec:
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: glyph.glyph, glyph.font_size, glyph.line_height_milli :: call
    spec.glyph_index = glyph.glyph_index
    spec.traits = arcana_text.font_leaf.FaceTraits :: weight = glyph.weight, width_milli = glyph.width_milli, slant_milli = glyph.slant_milli :: call
    spec.feature_signature = glyph.feature_signature
    spec.axis_signature = glyph.axis_signature
    spec.color = glyph.color
    return spec

fn cached_image_for_bitmap(edit cache: arcana_text.cache.TextCache, read key: Str, read payload: (arcana_text.font_leaf.GlyphBitmap, arcana_text.types.RasterMode)) -> arcana_text.cache.CachedGlyphImage:
    let cached = cache :: key :: cached_glyph_image
    return match cached:
        Option.Some(image) => image
        Option.None => cached_image_for_bitmap_store :: cache, key, payload :: call

fn cached_image_for_bitmap_store(edit cache: arcana_text.cache.TextCache, read key: Str, read payload: (arcana_text.font_leaf.GlyphBitmap, arcana_text.types.RasterMode)) -> arcana_text.cache.CachedGlyphImage:
    let bitmap = payload.0
    let mode = payload.1
    let mut cached = arcana_text.cache.CachedGlyphImage :: offset = bitmap.offset, surface = (arcana_text.raster.surface_from_bitmap :: bitmap, mode :: call) :: call
    cache :: key, cached :: remember_glyph_image
    let hit = cache :: key :: cached_glyph_image
    return hit :: (arcana_text.raster.empty_cached_glyph_image :: :: call) :: unwrap_or

fn cache_empty_glyph_image(edit cache: arcana_text.cache.TextCache, key: Str) -> arcana_text.cache.CachedGlyphImage:
    let empty = arcana_text.raster.empty_cached_glyph_image :: :: call
    cache :: key, empty :: remember_glyph_image
    let cached = cache :: key :: cached_glyph_image
    return cached :: empty :: unwrap_or

fn image_draw_from_cached(read glyph: arcana_text.types.LayoutGlyph, read mode: arcana_text.types.RasterMode, read cached: arcana_text.cache.CachedGlyphImage) -> Option[arcana_text.types.GlyphImageDraw]:
    if cached.surface.size.0 <= 0 or cached.surface.size.1 <= 0:
        return Option.None[arcana_text.types.GlyphImageDraw] :: :: call
    let mut draw = arcana_text.types.GlyphImageDraw :: position = (glyph.position.0 + glyph.offset.0 + cached.offset.0, glyph.position.1 + glyph.offset.1 + cached.offset.1), size = cached.surface.size, mode = mode :: call
    draw.color = glyph.color
    draw.surface = cached.surface
    return Option.Some[arcana_text.types.GlyphImageDraw] :: draw :: call

fn rendered_glyph_image(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read payload: (arcana_text.types.LayoutGlyph, arcana_text.types.RasterConfig)) -> Option[arcana_text.types.GlyphImageDraw]:
    let glyph = payload.0
    let config = payload.1
    let mode = config.mode
    if glyph.face_id.source_index < 0 or glyph.glyph_index <= 0:
        return Option.None[arcana_text.types.GlyphImageDraw] :: :: call
    let key = arcana_text.raster.glyph_surface_key :: (glyph, mode, config.hinting) :: call
    let cached = cache :: key :: cached_glyph_image
    if cached :: :: is_some:
        raster_probe_append :: ("rendered_glyph_image:cache_hit glyph=" + (std.text.from_int :: glyph.glyph_index :: call)) :: call
        return arcana_text.raster.image_draw_from_cached :: glyph, mode, (cached :: (arcana_text.raster.empty_cached_glyph_image :: :: call) :: unwrap_or) :: call
    raster_probe_append :: ("rendered_glyph_image:start glyph=" + (std.text.from_int :: glyph.glyph_index :: call)) :: call
    let mut spec = arcana_text.raster.glyph_render_spec :: glyph :: call
    spec.mode = mode
    spec.hinting = config.hinting
    raster_probe_append :: "rendered_glyph_image:render_face_glyph" :: call
    let bitmap = fonts :: glyph.face_id, spec :: render_face_glyph
    if bitmap.empty or bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
        let _ = arcana_text.raster.cache_empty_glyph_image :: cache, key :: call
        raster_probe_append :: "rendered_glyph_image:empty_bitmap" :: call
        return Option.None[arcana_text.types.GlyphImageDraw] :: :: call
    let cached_image = arcana_text.raster.cached_image_for_bitmap :: cache, key, (bitmap, mode) :: call
    raster_probe_append :: "rendered_glyph_image:ready" :: call
    return arcana_text.raster.image_draw_from_cached :: glyph, mode, cached_image :: call

fn rendered_glyph_image_renderer(edit self: arcana_text.raster.TextRenderer, read payload: (arcana_text.types.LayoutGlyph, arcana_text.types.RasterConfig)) -> Option[arcana_text.types.GlyphImageDraw]:
    let glyph = payload.0
    let config = payload.1
    let mode = config.mode
    if glyph.face_id.source_index < 0 or glyph.glyph_index <= 0:
        return Option.None[arcana_text.types.GlyphImageDraw] :: :: call
    let key = arcana_text.raster.glyph_surface_key :: (glyph, mode, config.hinting) :: call
    let cached = self.cache :: key :: cached_glyph_image
    if cached :: :: is_some:
        return arcana_text.raster.image_draw_from_cached :: glyph, mode, (cached :: (arcana_text.raster.empty_cached_glyph_image :: :: call) :: unwrap_or) :: call
    let mut spec = arcana_text.raster.glyph_render_spec :: glyph :: call
    spec.mode = mode
    spec.hinting = config.hinting
    let bitmap = self.fonts :: glyph.face_id, spec :: render_face_glyph
    if bitmap.empty or bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
        let _ = arcana_text.raster.cache_empty_glyph_image :: self.cache, key :: call
        return Option.None[arcana_text.types.GlyphImageDraw] :: :: call
    let cached_image = arcana_text.raster.cached_image_for_bitmap :: self.cache, key, (bitmap, mode) :: call
    return arcana_text.raster.image_draw_from_cached :: glyph, mode, cached_image :: call

fn glyph_image_draws(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> List[arcana_text.types.GlyphImageDraw]:
    let snapshot = payload.0
    let config = payload.1
    let mut out = std.collections.list.new[arcana_text.types.GlyphImageDraw] :: :: call
    for glyph in snapshot.glyphs:
        if not (arcana_text.raster.in_clip :: config.clip_range, glyph :: call):
            continue
        if glyph.glyph == " " or glyph.glyph == "\t" or glyph.glyph == "\n" or glyph.glyph == "\r":
            continue
        raster_probe_append :: ("glyph_image_draws:glyph " + (std.text.from_int :: glyph.glyph_index :: call) + " text=" + glyph.glyph) :: call
        let image = arcana_text.raster.rendered_glyph_image :: fonts, cache, (glyph, config) :: call
        if image :: :: is_some:
            out :: (image :: (arcana_text.raster.empty_image_draw :: config.mode :: call) :: unwrap_or) :: push
            raster_probe_append :: "glyph_image_draws:push" :: call
    return out

fn glyph_image_draws_renderer(edit self: arcana_text.raster.TextRenderer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> List[arcana_text.types.GlyphImageDraw]:
    let snapshot = payload.0
    let config = payload.1
    let mut out = std.collections.list.new[arcana_text.types.GlyphImageDraw] :: :: call
    for glyph in snapshot.glyphs:
        if not (arcana_text.raster.in_clip :: config.clip_range, glyph :: call):
            continue
        if glyph.glyph == " " or glyph.glyph == "\t" or glyph.glyph == "\n" or glyph.glyph == "\r":
            continue
        let image = arcana_text.raster.rendered_glyph_image_renderer :: self, (glyph, config) :: call
        if image :: :: is_some:
            out :: (image :: (arcana_text.raster.empty_image_draw :: config.mode :: call) :: unwrap_or) :: push
    return out

fn glyph_draws(read snapshot: arcana_text.layout.LayoutSnapshot, read config: arcana_text.types.RasterConfig) -> List[arcana_text.types.GlyphDraw]:
    let mut glyphs = std.collections.list.new[arcana_text.types.GlyphDraw] :: :: call
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

fn decoration_draws(edit fonts: arcana_text.fonts.FontSystem, read snapshot: arcana_text.layout.LayoutSnapshot, read config: arcana_text.types.RasterConfig) -> List[arcana_text.types.DecorationDraw]:
    let mut out = arcana_text.raster.empty_decoration_draws :: :: call
    for run in snapshot.runs:
        if config.clip_range.end > config.clip_range.start and (run.range.end <= config.clip_range.start or run.range.start >= config.clip_range.end):
            continue
        if run.size.0 <= 0 or run.face_id.source_index < 0:
            continue
        if run.underline != (arcana_text.types.UnderlineStyle.None :: :: call):
            let metrics = fonts :: run.face_id, run.font_size :: underline_metrics
            let thickness = arcana_text.raster.max_int :: metrics.1, 1 :: call
            let y = run.baseline - metrics.0
            let color = arcana_text.raster.decoration_color :: run.color, run.underline_color_enabled, run.underline_color :: call
            arcana_text.raster.append_decoration :: out, ((run.position.0, y), (run.size.0, thickness), color) :: call
            if run.underline == (arcana_text.types.UnderlineStyle.Double :: :: call):
                arcana_text.raster.append_decoration :: out, ((run.position.0, y + thickness + 1), (run.size.0, thickness), color) :: call
        if run.strikethrough_enabled:
            let metrics = fonts :: run.face_id, run.font_size :: strikethrough_metrics
            let thickness = arcana_text.raster.max_int :: metrics.1, 1 :: call
            let y = run.baseline - metrics.0
            let color = arcana_text.raster.decoration_color :: run.color, run.strikethrough_color_enabled, run.strikethrough_color :: call
            arcana_text.raster.append_decoration :: out, ((run.position.0, y), (run.size.0, thickness), color) :: call
        if run.overline_enabled:
            let metrics = fonts :: run.face_id, run.font_size :: overline_metrics
            let thickness = arcana_text.raster.max_int :: metrics.1, 1 :: call
            let y = run.baseline - metrics.0
            let color = arcana_text.raster.decoration_color :: run.color, run.overline_color_enabled, run.overline_color :: call
            arcana_text.raster.append_decoration :: out, ((run.position.0, y), (run.size.0, thickness), color) :: call
    return out

RasterPass
RasterPassState
fn draw_stream_active(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
    let snapshot = payload.0
    let config = payload.1
    let _scratch = temp: arcana_text.raster.raster_scratch :> value = snapshot.source_version <: arcana_text.raster.RasterScratch
    raster_probe_append :: ("draw_stream_active:start glyphs=" + (std.text.from_int :: (snapshot.glyphs :: :: len) :: call)) :: call
    let glyphs = arcana_text.raster.glyph_draws :: snapshot, config :: call
    raster_probe_append :: ("draw_stream_active:glyph_draws " + (std.text.from_int :: (glyphs :: :: len) :: call)) :: call
    let decorations = arcana_text.raster.decoration_draws :: fonts, snapshot, config :: call
    let images = arcana_text.raster.glyph_image_draws :: fonts, cache, (snapshot, config) :: call
    raster_probe_append :: ("draw_stream_active:images " + (std.text.from_int :: (images :: :: len) :: call)) :: call
    cache :: snapshot.source_version :: touch_snapshot
    let mut out = arcana_text.raster.GlyphDrawStream :: source_version = snapshot.source_version, size = snapshot.size, glyphs = glyphs :: call
    out.decorations = decorations
    out.images = images
    return out

fn draw_stream_active_renderer(edit self: arcana_text.raster.TextRenderer, read payload: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
    let snapshot = payload.0
    let config = payload.1
    let _scratch = temp: arcana_text.raster.raster_scratch :> value = snapshot.source_version <: arcana_text.raster.RasterScratch
    let glyphs = arcana_text.raster.glyph_draws :: snapshot, config :: call
    let decorations = arcana_text.raster.decoration_draws :: self.fonts, snapshot, config :: call
    let images = arcana_text.raster.glyph_image_draws_renderer :: self, (snapshot, config) :: call
    self.cache :: snapshot.source_version :: touch_snapshot
    let mut out = arcana_text.raster.GlyphDrawStream :: source_version = snapshot.source_version, size = snapshot.size, glyphs = glyphs :: call
    out.decorations = decorations
    out.images = images
    return out

export fn default_renderer() -> arcana_text.raster.TextRenderer:
    let mut renderer = arcana_text.raster.TextRenderer :: fonts = (arcana_text.fonts.default_system :: :: call), cache = (arcana_text.cache.open :: :: call), layout_snapshots = (arcana_text.raster.empty_layout_snapshot_cache :: :: call) :: call
    renderer.draw_streams = arcana_text.raster.empty_draw_stream_cache :: :: call
    return renderer

export fn default_renderer_with_locale(locale: Str) -> arcana_text.raster.TextRenderer:
    let mut renderer = arcana_text.raster.TextRenderer :: fonts = (arcana_text.fonts.default_system_with_locale :: locale :: call), cache = (arcana_text.cache.open :: :: call), layout_snapshots = (arcana_text.raster.empty_layout_snapshot_cache :: :: call) :: call
    renderer.draw_streams = arcana_text.raster.empty_draw_stream_cache :: :: call
    return renderer

export fn draw_stream(edit fonts: arcana_text.fonts.FontSystem, edit cache: arcana_text.cache.TextCache, read request: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
    let snapshot = request.0
    let config = request.1
    return arcana_text.raster.draw_stream_active :: fonts, cache, (snapshot, config) :: call

impl TextRenderer:
    fn clear_caches(edit self: arcana_text.raster.TextRenderer):
        self.cache = arcana_text.cache.open :: :: call
        self.layout_snapshots = arcana_text.raster.empty_layout_snapshot_cache :: :: call
        self.draw_streams = arcana_text.raster.empty_draw_stream_cache :: :: call

    fn set_locale(edit self: arcana_text.raster.TextRenderer, locale: Str):
        self.fonts :: locale :: set_locale
        self :: :: clear_caches

    fn snapshot(edit self: arcana_text.raster.TextRenderer, read buffer: arcana_text.buffer.TextBuffer, read config: arcana_text.types.LayoutConfig) -> arcana_text.layout.LayoutSnapshot:
        let mut effective = config
        effective.tab_width = buffer :: :: tab_width
        let key = arcana_text.raster.layout_snapshot_key :: self, buffer, effective :: call
        if self.layout_snapshots :: key :: has:
            return self.layout_snapshots :: key :: get
        if (self.layout_snapshots :: :: len) >= (arcana_text.raster.layout_snapshot_cache_limit :: :: call):
            self.layout_snapshots :: :: clear
        let snapshot = arcana_text.layout.snapshot :: self.fonts, buffer, effective :: call
        self.layout_snapshots :: key, snapshot :: set
        return snapshot

    fn draw_stream(edit self: arcana_text.raster.TextRenderer, read request: (arcana_text.layout.LayoutSnapshot, arcana_text.types.RasterConfig)) -> arcana_text.raster.GlyphDrawStream:
        let snapshot = request.0
        let config = request.1
        let key = arcana_text.raster.draw_stream_key :: snapshot, config :: call
        if self.draw_streams :: key :: has:
            return self.draw_streams :: key :: get
        if (self.draw_streams :: :: len) >= (arcana_text.raster.draw_stream_cache_limit :: :: call):
            self.draw_streams :: :: clear
        let stream = arcana_text.raster.draw_stream_active_renderer :: self, (snapshot, config) :: call
        self.draw_streams :: key, stream :: set
        return stream

