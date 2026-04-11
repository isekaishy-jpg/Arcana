import arcana_desktop.app
import arcana_desktop.canvas
import arcana_desktop.input
import arcana_desktop.types
import arcana_desktop.window
import arcana_text.buffer
import arcana_text.editor
import arcana_text.font_leaf
import arcana_text.fonts
import arcana_text.monaspace
import arcana_text.queries
import arcana_text.raster
import std.args
import std.bytes
import std.collections.array
import std.collections.list
import std.fs
import std.io
import std.kernel.gfx
import std.option
import std.path
import std.result
import std.text
import std.time
import std.types.core
use std.option.Option
use std.result.Result

record ProofApp:
    renderer: arcana_text.raster.TextRenderer
    discover_system_fonts: Bool
    system_fonts_ready: Bool
    ui_smoke_mode: Bool
    ui_demo_mode: Bool
    redraw_count: Int
    demo_stress_mode: Int
    demo_cache: DemoCache
    fps_window_start_ms: Int
    fps_frame_count: Int
    fps_tenths: Int
    fps_last_update_ms: Int
    last_paint_ms: Int

record ProofMetrics:
    font_sources: Int
    feature_off_glyphs: Int
    feature_on_glyphs: Int
    width_narrow: Int
    width_wide: Int
    weight_light_alpha: Int
    weight_bold_alpha: Int
    slant_upright_span: Int
    slant_italic_span: Int
    wrap_lines: Int
    nowrap_lines: Int
    bidi_unresolved: Int
    bidi_fonts: Int
    edit_hit_index: Int
    edit_range_boxes: Int
    primary_font: Str

record RenderSpec:
    text: Str
    width: Int
    style: arcana_text.types.TextStyle
    paragraph: arcana_text.types.ParagraphStyle

record PanelSpec:
    origin: (Int, Int)
    size: (Int, Int)
    title: Str
    subtitle: Str

record RangePaintSpec:
    boxes: List[arcana_text.types.RangeBox]
    origin: (Int, Int)
    color: Int

record CaretPaintSpec:
    caret: arcana_text.types.CaretBox
    origin: (Int, Int)
    color: Int

record RenderTiming:
    snapshot_ms: Int
    draw_stream_ms: Int
    total_ms: Int

record TimedRender:
    snapshot: arcana_text.layout.LayoutSnapshot
    stream: arcana_text.raster.GlyphDrawStream
    timing: RenderTiming

record BitmapTiming:
    total_ms: Int

record TimedBitmap:
    bitmap: arcana_text.font_leaf.GlyphBitmap
    timing: BitmapTiming

record SmokePerf:
    snapshot_ms: Int
    draw_stream_ms: Int
    feature_render_ms: Int
    axis_render_ms: Int
    wrap_render_ms: Int
    bidi_render_ms: Int
    query_render_ms: Int
    render_total_ms: Int
    total_ms: Int

record DemoCache:
    ready: Bool
    window_size: (Int, Int)
    stress_mode: Int
    build_ms: Int
    feature_off: arcana_text.raster.GlyphDrawStream
    feature_on: arcana_text.raster.GlyphDrawStream
    axis_light: arcana_text.raster.GlyphDrawStream
    axis_bold: arcana_text.raster.GlyphDrawStream
    wrap_block: arcana_text.raster.GlyphDrawStream
    nowrap_block: arcana_text.raster.GlyphDrawStream
    bidi_mixed: arcana_text.raster.GlyphDrawStream
    bidi_arabic: arcana_text.raster.GlyphDrawStream
    query_stream: arcana_text.raster.GlyphDrawStream
    query_boxes: List[arcana_text.types.RangeBox]
    query_caret: arcana_text.types.CaretBox
    query_hit_index: Int
    bidi_fonts: Int
    bidi_unresolved: Int
    stress_block: arcana_text.raster.GlyphDrawStream

fn has_flag(flag: Str) -> Bool:
    let total = std.args.count :: :: call
    let mut index = 0
    while index < total:
        if (std.args.get :: index :: call) == flag:
            return true
        index += 1
    return false

fn probe_enabled() -> Bool:
    return has_flag :: "--probe" :: call

fn probe_log_path() -> Str:
    return std.path.join :: (std.path.join :: (std.path.cwd :: :: call), "scratch" :: call), "arcana_text_proof_probe.log" :: call

fn probe_log_append(line: Str):
    if not (probe_enabled :: :: call):
        return
    let _ = std.fs.mkdir_all :: (std.path.parent :: (probe_log_path :: :: call) :: call) :: call
    let opened = std.fs.stream_open_write :: (probe_log_path :: :: call), true :: call
    return match opened:
        Result.Ok(value) => probe_log_append_ready :: value, line :: call
        Result.Err(_) => 0

fn probe_log_append_ready(take value: std.fs.FileStream, line: Str):
    let mut stream = value
    let bytes = std.bytes.from_str_utf8 :: (line + "\n") :: call
    let _ = std.fs.stream_write :: stream, bytes :: call
    let _ = std.fs.stream_close :: stream :: call

fn reset_probe_log():
    if not (probe_enabled :: :: call):
        return
    let _ = std.fs.mkdir_all :: (std.path.parent :: (probe_log_path :: :: call) :: call) :: call
    let _ = std.fs.write_text :: (probe_log_path :: :: call), "" :: call

fn rgb(r: Int, g: Int, b: Int) -> Int:
    return arcana_desktop.canvas.rgb :: r, g, b :: call

fn max_int(a: Int, b: Int) -> Int:
    if a >= b:
        return a
    return b

fn bool_code(value: Bool) -> Int:
    if value:
        return 1
    return 0

fn elapsed_ms(read start: std.types.core.MonotonicTimeMs, read end: std.types.core.MonotonicTimeMs) -> Int:
    return end.value - start.value

fn empty_draw_stream() -> arcana_text.raster.GlyphDrawStream:
    let mut stream = arcana_text.raster.GlyphDrawStream :: source_version = 0, size = (0, 0), glyphs = (std.collections.list.new[arcana_text.types.GlyphDraw] :: :: call) :: call
    stream.decorations = std.collections.list.new[arcana_text.types.DecorationDraw] :: :: call
    stream.images = std.collections.list.new[arcana_text.types.GlyphImageDraw] :: :: call
    return stream

fn empty_caret_box() -> arcana_text.types.CaretBox:
    return arcana_text.types.CaretBox :: index = 0, position = (0, 0), size = (0, 0) :: call

fn empty_demo_cache() -> DemoCache:
    let mut cache = DemoCache :: ready = false, window_size = (0, 0), stress_mode = -1 :: call
    cache.build_ms = 0
    cache.feature_off = empty_draw_stream :: :: call
    cache.feature_on = empty_draw_stream :: :: call
    cache.axis_light = empty_draw_stream :: :: call
    cache.axis_bold = empty_draw_stream :: :: call
    cache.wrap_block = empty_draw_stream :: :: call
    cache.nowrap_block = empty_draw_stream :: :: call
    cache.bidi_mixed = empty_draw_stream :: :: call
    cache.bidi_arabic = empty_draw_stream :: :: call
    cache.query_stream = empty_draw_stream :: :: call
    cache.query_boxes = std.collections.list.new[arcana_text.types.RangeBox] :: :: call
    cache.query_caret = empty_caret_box :: :: call
    cache.query_hit_index = 0
    cache.bidi_fonts = 0
    cache.bidi_unresolved = 0
    cache.stress_block = empty_draw_stream :: :: call
    return cache

fn append_metric_line(out: Str, label: Str, value: Int) -> Str:
    return out + label + ": " + (std.text.from_int :: value :: call) + "\n"

fn append_timing_lines(read out_and_perf: (Str, SmokePerf)) -> Str:
    let out = out_and_perf.0
    let perf = out_and_perf.1
    let mut text = out
    text = append_metric_line :: text, "snapshot_ms", perf.snapshot_ms :: call
    text = append_metric_line :: text, "draw_stream_ms", perf.draw_stream_ms :: call
    text = append_metric_line :: text, "feature_render_ms", perf.feature_render_ms :: call
    text = append_metric_line :: text, "axis_render_ms", perf.axis_render_ms :: call
    text = append_metric_line :: text, "wrap_render_ms", perf.wrap_render_ms :: call
    text = append_metric_line :: text, "bidi_render_ms", perf.bidi_render_ms :: call
    text = append_metric_line :: text, "query_render_ms", perf.query_render_ms :: call
    text = append_metric_line :: text, "render_total_ms", perf.render_total_ms :: call
    text = append_metric_line :: text, "total_smoke_ms", perf.total_ms :: call
    return text

fn stress_mode_name(mode: Int) -> Str:
    if mode == 1:
        return "Dense"
    if mode == 2:
        return "Heavy"
    return "Normal"

fn cycle_stress_mode(mode: Int) -> Int:
    if mode >= 2:
        return 0
    return mode + 1

fn fps_text(tenths: Int) -> Str:
    let whole = tenths / 10
    let frac = tenths % 10
    return (std.text.from_int :: whole :: call) + "." + (std.text.from_int :: frac :: call)

fn point_in_rect(point: (Int, Int), read rect: ((Int, Int), (Int, Int))) -> Bool:
    return point.0 >= rect.0.0 and point.1 >= rect.0.1 and point.0 < (rect.0.0 + rect.1.0) and point.1 < (rect.0.1 + rect.1.1)

fn demo_button_geometry(size: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let width = 148
    let height = 34
    return ((size.0 - width - 28, 18), (width, height))

fn demo_feature_geometry(size: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let gutter = 24
    let column_width = (size.0 - gutter * 3) / 2
    return ((gutter, 88), (column_width, 184))

fn demo_axis_geometry(size: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let gutter = 24
    let column_width = (size.0 - gutter * 3) / 2
    return ((gutter * 2 + column_width, 88), (column_width, 184))

fn demo_wrap_geometry(size: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let gutter = 24
    let column_width = (size.0 - gutter * 3) / 2
    return ((gutter, 288), (column_width, 170))

fn demo_bidi_geometry(size: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let gutter = 24
    let column_width = (size.0 - gutter * 3) / 2
    return ((gutter * 2 + column_width, 288), (column_width, 206))

fn demo_query_geometry(size: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let gutter = 24
    return ((gutter, 474), (size.0 - gutter * 2, 142))

fn demo_stress_geometry(size: (Int, Int)) -> ((Int, Int), (Int, Int)):
    let gutter = 24
    let top = 632
    let available = size.1 - top - gutter
    let height = max_int :: available, 96 :: call
    return ((gutter, top), (size.0 - gutter * 2, height))

fn print_metric(label: Str, value: Int):
    std.io.print[Str] :: (label + ": " + (std.text.from_int :: value :: call) + "\n") :: call

fn print_text_metric(label: Str, value: Str):
    std.io.print[Str] :: (label + ": " + value + "\n") :: call

fn smoke_report_path() -> Str:
    let scratch = std.path.join :: (std.path.cwd :: :: call), "scratch" :: call
    return std.path.join :: scratch, "arcana_text_proof_smoke.txt" :: call

fn smoke_progress_path() -> Str:
    let scratch = std.path.join :: (std.path.cwd :: :: call), "scratch" :: call
    return std.path.join :: scratch, "arcana_text_proof_progress.txt" :: call

fn reset_smoke_report():
    let _ = std.fs.mkdir_all :: (std.path.parent :: (smoke_report_path :: :: call) :: call) :: call
    let _ = std.fs.write_text :: (smoke_report_path :: :: call), "" :: call

fn reset_smoke_progress():
    let _ = std.fs.mkdir_all :: (std.path.parent :: (smoke_progress_path :: :: call) :: call) :: call
    let _ = std.fs.write_text :: (smoke_progress_path :: :: call), "" :: call

fn write_smoke_progress(line: Str):
    let _ = std.fs.mkdir_all :: (std.path.parent :: (smoke_progress_path :: :: call) :: call) :: call
    let _ = std.fs.write_text :: (smoke_progress_path :: :: call), line :: call

fn smoke_report_text(read metrics: ProofMetrics) -> Str:
    let mut out = ""
    out = out + "font_sources: " + (std.text.from_int :: metrics.font_sources :: call) + "\n"
    out = out + "feature_off_glyphs: " + (std.text.from_int :: metrics.feature_off_glyphs :: call) + "\n"
    out = out + "feature_on_glyphs: " + (std.text.from_int :: metrics.feature_on_glyphs :: call) + "\n"
    out = out + "width_narrow: " + (std.text.from_int :: metrics.width_narrow :: call) + "\n"
    out = out + "width_wide: " + (std.text.from_int :: metrics.width_wide :: call) + "\n"
    out = out + "weight_light_alpha: " + (std.text.from_int :: metrics.weight_light_alpha :: call) + "\n"
    out = out + "weight_bold_alpha: " + (std.text.from_int :: metrics.weight_bold_alpha :: call) + "\n"
    out = out + "slant_upright_span: " + (std.text.from_int :: metrics.slant_upright_span :: call) + "\n"
    out = out + "slant_italic_span: " + (std.text.from_int :: metrics.slant_italic_span :: call) + "\n"
    out = out + "wrap_lines: " + (std.text.from_int :: metrics.wrap_lines :: call) + "\n"
    out = out + "nowrap_lines: " + (std.text.from_int :: metrics.nowrap_lines :: call) + "\n"
    out = out + "bidi_fonts: " + (std.text.from_int :: metrics.bidi_fonts :: call) + "\n"
    out = out + "bidi_unresolved: " + (std.text.from_int :: metrics.bidi_unresolved :: call) + "\n"
    out = out + "edit_hit_index: " + (std.text.from_int :: metrics.edit_hit_index :: call) + "\n"
    out = out + "edit_range_boxes: " + (std.text.from_int :: metrics.edit_range_boxes :: call) + "\n"
    out = out + "primary_font: " + metrics.primary_font + "\n"
    return out

fn feature_smoke_report_text(read counts: (Int, Int), read indexes: (Str, Str), read pixels_and_font: ((Int, Int), Str)) -> Str:
    let mut out = ""
    out = out + "feature_off_glyphs: " + (std.text.from_int :: counts.0 :: call) + "\n"
    out = out + "feature_on_glyphs: " + (std.text.from_int :: counts.1 :: call) + "\n"
    out = out + "feature_off_indexes: " + indexes.0 + "\n"
    out = out + "feature_on_indexes: " + indexes.1 + "\n"
    out = out + "feature_off_pixels: " + (std.text.from_int :: pixels_and_font.0.0 :: call) + "\n"
    out = out + "feature_on_pixels: " + (std.text.from_int :: pixels_and_font.0.1 :: call) + "\n"
    out = out + "primary_font: " + pixels_and_font.1 + "\n"
    return out

fn first_font_name(read snapshot: arcana_text.layout.LayoutSnapshot) -> Str:
    return arcana_text.queries.primary_font_name :: snapshot :: call

fn glyph_indexes_text(read snapshot: arcana_text.layout.LayoutSnapshot) -> Str:
    let mut out = ""
    let mut first = true
    for glyph in snapshot.glyphs:
        if not first:
            out = out + ","
        out = out + (std.text.from_int :: glyph.glyph_index :: call)
        first = false
    return out

fn base_style(color: Int, size: Int) -> arcana_text.types.TextStyle:
    let mut style = arcana_text.types.default_text_style :: color :: call
    style.size = size
    style.families :: (arcana_text.monaspace.family_name :: (arcana_text.monaspace.default_family :: :: call) :: call) :: push
    return style

fn disable_feature(edit style: arcana_text.types.TextStyle, tag: Str):
    let mut feature = arcana_text.types.FontFeature :: tag = tag, value = 0, enabled = false :: call
    style.features :: feature :: push

fn feature_off_style(color: Int, size: Int) -> arcana_text.types.TextStyle:
    let mut style = base_style :: color, size :: call
    disable_feature :: style, "liga" :: call
    disable_feature :: style, "calt" :: call
    disable_feature :: style, "rlig" :: call
    disable_feature :: style, "kern" :: call
    return style

fn code_feature_style(color: Int, size: Int) -> arcana_text.types.TextStyle:
    return base_style :: color, size :: call

fn axis_style(color: Int, size: Int, read axis: arcana_text.types.FontAxis) -> arcana_text.types.TextStyle:
    let mut style = code_feature_style :: color, size :: call
    style.axes :: axis :: push
    return style

fn paragraph_left() -> arcana_text.types.ParagraphStyle:
    return arcana_text.types.default_paragraph_style :: :: call

fn paragraph_no_wrap() -> arcana_text.types.ParagraphStyle:
    let mut paragraph = arcana_text.types.default_paragraph_style :: :: call
    paragraph.wrap = arcana_text.types.TextWrap.NoWrap :: :: call
    paragraph.max_lines = 1
    paragraph.ellipsis = "..."
    return paragraph

fn paragraph_wrap_three() -> arcana_text.types.ParagraphStyle:
    let mut paragraph = arcana_text.types.default_paragraph_style :: :: call
    paragraph.max_lines = 3
    return paragraph

fn render_spec(text: Str, width: Int, read style: arcana_text.types.TextStyle) -> RenderSpec:
    let mut spec = RenderSpec :: text = text, width = width, style = style :: call
    spec.paragraph = paragraph_left :: :: call
    return spec

fn panel_spec(origin: (Int, Int), size: (Int, Int), title: Str) -> PanelSpec:
    let mut spec = PanelSpec :: origin = origin, size = size, title = title :: call
    spec.subtitle = ""
    return spec

fn bidi_mixed_text() -> Str:
    return bidi_mixed_text_clean :: :: call

fn bidi_arabic_text() -> Str:
    return bidi_arabic_text_clean :: :: call

fn stress_line_text() -> Str:
    return stress_line_text_clean :: :: call

fn bidi_mixed_text_clean() -> Str:
    return "Arcana -> אבג -> مرحبا بالعالم -> 123"

fn bidi_arabic_text_clean() -> Str:
    return "Arabic join forms: سلام عليكم"

fn stress_line_text_clean() -> Str:
    return "Arcana stress -> != -> mmmm -> سلام عليكم -> אבג -> 123456789"

fn render_text(edit renderer: arcana_text.raster.TextRenderer, read spec: RenderSpec) -> (arcana_text.layout.LayoutSnapshot, arcana_text.raster.GlyphDrawStream):
    probe_log_append :: ("render_text:start width=" + (std.text.from_int :: spec.width :: call) + " bytes=" + (std.text.from_int :: (std.text.len_bytes :: spec.text :: call) :: call)) :: call
    let snapshot = snapshot_text :: renderer, spec :: call
    write_smoke_progress :: ("snapshot bytes=" + (std.text.from_int :: (std.text.len_bytes :: spec.text :: call) :: call)) :: call
    probe_log_append :: "render_text:snapshot" :: call
    let raster = arcana_text.types.default_raster_config :: :: call
    let stream = renderer :: (snapshot, raster) :: draw_stream
    write_smoke_progress :: ("draw-stream bytes=" + (std.text.from_int :: (std.text.len_bytes :: spec.text :: call) :: call)) :: call
    probe_log_append :: "render_text:draw_stream" :: call
    return (snapshot, stream)

fn timed_render_text(edit renderer: arcana_text.raster.TextRenderer, read spec: RenderSpec) -> TimedRender:
    let total_start = std.time.monotonic_now_ms :: :: call
    let snapshot_start = total_start
    let snapshot = snapshot_text :: renderer, spec :: call
    let snapshot_end = std.time.monotonic_now_ms :: :: call
    let raster = arcana_text.types.default_raster_config :: :: call
    let draw_start = snapshot_end
    let stream = renderer :: (snapshot, raster) :: draw_stream
    let draw_end = std.time.monotonic_now_ms :: :: call
    let mut timed = TimedRender :: snapshot = snapshot, stream = stream :: call
    timed.timing = RenderTiming :: snapshot_ms = (elapsed_ms :: snapshot_start, snapshot_end :: call), draw_stream_ms = (elapsed_ms :: draw_start, draw_end :: call), total_ms = (elapsed_ms :: total_start, draw_end :: call) :: call
    return timed

fn timed_render_text_progress(edit renderer: arcana_text.raster.TextRenderer, read payload: (Str, RenderSpec)) -> TimedRender:
    let label = payload.0
    let spec = payload.1
    let total_start = std.time.monotonic_now_ms :: :: call
    let snapshot_start = total_start
    write_smoke_progress :: (label + ":snapshot-start") :: call
    let snapshot = snapshot_text :: renderer, spec :: call
    let snapshot_end = std.time.monotonic_now_ms :: :: call
    write_smoke_progress :: (label + ":snapshot-done ms=" + (std.text.from_int :: (elapsed_ms :: snapshot_start, snapshot_end :: call) :: call)) :: call
    let raster = arcana_text.types.default_raster_config :: :: call
    let draw_start = snapshot_end
    write_smoke_progress :: (label + ":draw-start") :: call
    let stream = renderer :: (snapshot, raster) :: draw_stream
    let draw_end = std.time.monotonic_now_ms :: :: call
    write_smoke_progress :: (label + ":draw-done ms=" + (std.text.from_int :: (elapsed_ms :: draw_start, draw_end :: call) :: call)) :: call
    let mut timed = TimedRender :: snapshot = snapshot, stream = stream :: call
    timed.timing = RenderTiming :: snapshot_ms = (elapsed_ms :: snapshot_start, snapshot_end :: call), draw_stream_ms = (elapsed_ms :: draw_start, draw_end :: call), total_ms = (elapsed_ms :: total_start, draw_end :: call) :: call
    return timed

fn snapshot_text(edit renderer: arcana_text.raster.TextRenderer, read spec: RenderSpec) -> arcana_text.layout.LayoutSnapshot:
    let safe_width = max_int :: spec.width, 1 :: call
    let buffer = arcana_text.buffer.open :: spec.text, spec.style, spec.paragraph :: call
    let config = arcana_text.types.default_layout_config :: safe_width, spec.paragraph :: call
    return renderer :: buffer, config :: snapshot

fn render_query_block(edit renderer: arcana_text.raster.TextRenderer, width: Int) -> (arcana_text.layout.LayoutSnapshot, arcana_text.raster.GlyphDrawStream):
    let snapshot = snapshot_query_block :: renderer, width :: call
    let raster = arcana_text.types.default_raster_config :: :: call
    let stream = renderer :: (snapshot, raster) :: draw_stream
    return (snapshot, stream)

fn timed_render_query_block(edit renderer: arcana_text.raster.TextRenderer, width: Int) -> TimedRender:
    let total_start = std.time.monotonic_now_ms :: :: call
    let snapshot_start = total_start
    let snapshot = snapshot_query_block :: renderer, width :: call
    let snapshot_end = std.time.monotonic_now_ms :: :: call
    let raster = arcana_text.types.default_raster_config :: :: call
    let draw_start = snapshot_end
    let stream = renderer :: (snapshot, raster) :: draw_stream
    let draw_end = std.time.monotonic_now_ms :: :: call
    let mut timed = TimedRender :: snapshot = snapshot, stream = stream :: call
    timed.timing = RenderTiming :: snapshot_ms = (elapsed_ms :: snapshot_start, snapshot_end :: call), draw_stream_ms = (elapsed_ms :: draw_start, draw_end :: call), total_ms = (elapsed_ms :: total_start, draw_end :: call) :: call
    return timed

fn demo_cache_needs_rebuild(read self: ProofApp, size: (Int, Int)) -> Bool:
    if not self.demo_cache.ready:
        return true
    if self.demo_cache.window_size != size:
        return true
    return self.demo_cache.stress_mode != self.demo_stress_mode

fn build_demo_cache(edit self: ProofApp, size: (Int, Int)) -> DemoCache:
    let started = std.time.monotonic_now_ms :: :: call
    let feature = demo_feature_geometry :: size :: call
    let axis = demo_axis_geometry :: size :: call
    let wrap = demo_wrap_geometry :: size :: call
    let bidi = demo_bidi_geometry :: size :: call
    let query = demo_query_geometry :: size :: call
    let stress = demo_stress_geometry :: size :: call
    let mut cache = empty_demo_cache :: :: call
    cache.window_size = size
    cache.stress_mode = self.demo_stress_mode

    let feature_width = feature.1.0 - 120
    let feature_off_block = render_text :: self.renderer, (render_spec :: "!= ->", feature_width, (feature_off_style :: (rgb :: 214, 227, 240 :: call), 20 :: call) :: call) :: call
    let feature_on_block = render_text :: self.renderer, (render_spec :: "!= ->", feature_width, (code_feature_style :: (rgb :: 155, 236, 215 :: call), 20 :: call) :: call) :: call
    cache.feature_off = feature_off_block.1
    cache.feature_on = feature_on_block.1

    let axis_width = (axis.1.0 - 72) / 2
    let axis_light_block = render_text :: self.renderer, (render_spec :: "mmmm", axis_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.weight_axis :: 250 :: call) :: call) :: call) :: call
    let axis_bold_block = render_text :: self.renderer, (render_spec :: "mmmm", axis_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.weight_axis :: 800 :: call) :: call) :: call) :: call
    cache.axis_light = axis_light_block.1
    cache.axis_bold = axis_bold_block.1

    let mut wrap_spec = render_spec :: "aa aa", 28, (code_feature_style :: (rgb :: 222, 232, 242 :: call), 19 :: call) :: call
    wrap_spec.paragraph = paragraph_wrap_three :: :: call
    let mut nowrap_spec = render_spec :: "aa aa", 28, (code_feature_style :: (rgb :: 222, 232, 242 :: call), 19 :: call) :: call
    nowrap_spec.paragraph = paragraph_no_wrap :: :: call
    cache.wrap_block = (render_text :: self.renderer, wrap_spec :: call).1
    cache.nowrap_block = (render_text :: self.renderer, nowrap_spec :: call).1

    let mixed = render_text :: self.renderer, (render_spec :: (bidi_mixed_text_clean :: :: call), (bidi.1.0 - 32), (code_feature_style :: (rgb :: 229, 239, 248 :: call), 23 :: call) :: call) :: call
    let arabic = render_text :: self.renderer, (render_spec :: (bidi_arabic_text_clean :: :: call), (bidi.1.0 - 32), (code_feature_style :: (rgb :: 163, 231, 201 :: call), 22 :: call) :: call) :: call
    cache.bidi_mixed = mixed.1
    cache.bidi_arabic = arabic.1
    cache.bidi_fonts = mixed.0.fonts_used :: :: len
    cache.bidi_unresolved = mixed.0.unresolved :: :: len

    let query_block = render_query_block :: self.renderer, (query.1.0 - 32) :: call
    cache.query_stream = query_block.1
    cache.query_boxes = arcana_text.queries.range_boxes :: query_block.0, (arcana_text.types.TextRange :: start = 0, end = 6 :: call) :: call
    cache.query_caret = arcana_text.queries.caret_box :: query_block.0, 8 :: call
    cache.query_hit_index = (arcana_text.queries.hit_test :: query_block.0, (24, 12) :: call).index

    let stress_text_value = stress_text :: self.demo_stress_mode :: call
    let stress_draw = render_text :: self.renderer, (render_spec :: stress_text_value, (stress.1.0 - 32), (code_feature_style :: (rgb :: 227, 236, 245 :: call), 15 :: call) :: call) :: call
    cache.stress_block = stress_draw.1

    let finished = std.time.monotonic_now_ms :: :: call
    cache.build_ms = elapsed_ms :: started, finished :: call
    cache.ready = true
    return cache

fn ensure_demo_cache(edit self: ProofApp, size: (Int, Int)):
    if demo_cache_needs_rebuild :: self, size :: call:
        self.demo_cache = build_demo_cache :: self, size :: call

fn draw_demo_overlay(edit self: ProofApp, edit win: arcana_desktop.types.Window, size: (Int, Int)):
    let rect = demo_button_geometry :: size :: call
    let active = rgb :: 34, 57, 82 :: call
    let border = rgb :: 88, 124, 162 :: call
    fill_rect :: win, rect, active :: call
    stroke_line :: win, (rect.0, (rect.0.0 + rect.1.0, rect.0.1)), border :: call
    stroke_line :: win, ((rect.0.0, rect.0.1 + rect.1.1), (rect.0.0 + rect.1.0, rect.0.1 + rect.1.1)), border :: call
    stroke_line :: win, (rect.0, (rect.0.0, rect.0.1 + rect.1.1)), border :: call
    stroke_line :: win, ((rect.0.0 + rect.1.0, rect.0.1), (rect.0.0 + rect.1.0, rect.0.1 + rect.1.1)), border :: call
    draw_label :: win, ((rect.0.0 + 12, rect.0.1 + 11), (("Stress: " + (stress_mode_name :: self.demo_stress_mode :: call)), (rgb :: 236, 243, 250 :: call))) :: call
    draw_label :: win, ((size.0 - 278, 58), (("FPS " + (fps_text :: self.fps_tenths :: call) + "  paint " + (std.text.from_int :: self.last_paint_ms :: call) + "ms"), (rgb :: 148, 169, 193 :: call))) :: call
    draw_label :: win, ((size.0 - 278, 78), (("build " + (std.text.from_int :: self.demo_cache.build_ms :: call) + "ms"), (rgb :: 112, 141, 170 :: call))) :: call

fn draw_demo_feature_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let mut panel = panel_spec :: origin, geometry.1, "GSUB + Monaspace" :: call
    panel.subtitle = "Real Arcana text draw streams cached for manual smoke."
    draw_panel_frame :: win, panel :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 74), ("off", (rgb :: 155, 170, 186 :: call))) :: call
    paint_stream :: win, (self.demo_cache.feature_off, (origin.0 + 76, origin.1 + 70)) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 116), ("on", (rgb :: 94, 208, 176 :: call))) :: call
    paint_stream :: win, (self.demo_cache.feature_on, (origin.0 + 76, origin.1 + 112)) :: call

fn draw_demo_axis_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let size = geometry.1
    let mut panel = panel_spec :: origin, size, "Variable Axes" :: call
    panel.subtitle = "Weight contrast rendered once, then repainted for FPS checks."
    draw_panel_frame :: win, panel :: call
    let content_width = (size.0 - 72) / 2
    draw_label :: win, ((origin.0 + 16, origin.1 + 74), ("wght 250", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (self.demo_cache.axis_light, (origin.0 + 16, origin.1 + 92)) :: call
    draw_label :: win, ((origin.0 + 24 + content_width, origin.1 + 74), ("wght 800", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (self.demo_cache.axis_bold, (origin.0 + 24 + content_width, origin.1 + 92)) :: call

fn draw_demo_wrap_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let mut panel = panel_spec :: origin, geometry.1, "Wrap + NoWrap" :: call
    panel.subtitle = "Cached wrap behavior still paints from Arcana text surfaces."
    draw_panel_frame :: win, panel :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 74), ("wrap", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (self.demo_cache.wrap_block, (origin.0 + 16, origin.1 + 94)) :: call
    draw_label :: win, ((origin.0 + 74, origin.1 + 74), ("ellipsis", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (self.demo_cache.nowrap_block, (origin.0 + 74, origin.1 + 94)) :: call

fn draw_demo_bidi_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let mut panel = panel_spec :: origin, geometry.1, "Bidi + Fallback" :: call
    panel.subtitle = fallback_panel_subtitle :: self :: call
    draw_panel_frame :: win, panel :: call
    paint_stream :: win, (self.demo_cache.bidi_mixed, (origin.0 + 16, origin.1 + 78)) :: call
    paint_stream :: win, (self.demo_cache.bidi_arabic, (origin.0 + 16, origin.1 + 126)) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 170), (("fonts used: " + (std.text.from_int :: self.demo_cache.bidi_fonts :: call) + "  unresolved: " + (std.text.from_int :: self.demo_cache.bidi_unresolved :: call)), (rgb :: 132, 153, 176 :: call))) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 190), ((fallback_status_text :: self :: call), (rgb :: 94, 208, 176 :: call))) :: call

fn draw_demo_query_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let mut panel = panel_spec :: origin, geometry.1, "Editor + Queries" :: call
    panel.subtitle = "Selection, caret, and hit testing cached alongside the rendered block."
    draw_panel_frame :: win, panel :: call
    let content_origin = (origin.0 + 16, origin.1 + 82)
    paint_range_boxes :: win, (RangePaintSpec :: boxes = self.demo_cache.query_boxes, origin = content_origin, color = (rgb :: 34, 60, 86 :: call) :: call) :: call
    paint_stream :: win, (self.demo_cache.query_stream, content_origin) :: call
    paint_caret :: win, (CaretPaintSpec :: caret = self.demo_cache.query_caret, origin = content_origin, color = (rgb :: 250, 207, 120 :: call) :: call) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 122), (("hit index " + (std.text.from_int :: self.demo_cache.query_hit_index :: call) + "  range boxes " + (std.text.from_int :: (self.demo_cache.query_boxes :: :: len) :: call)), (rgb :: 132, 153, 176 :: call))) :: call

fn draw_demo_stress_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let mut panel = panel_spec :: origin, geometry.1, "Manual Stress" :: call
    panel.subtitle = "Same text path, more cached glyph surfaces. Click the header button to cycle load."
    draw_panel_frame :: win, panel :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 42), (("mode " + (stress_mode_name :: self.demo_stress_mode :: call)), (rgb :: 155, 236, 215 :: call))) :: call
    paint_stream :: win, (self.demo_cache.stress_block, (origin.0 + 16, origin.1 + 68)) :: call

fn draw_ui_demo(edit self: ProofApp, edit win: arcana_desktop.types.Window, size: (Int, Int)):
    ensure_demo_cache :: self, size :: call
    draw_demo_overlay :: self, win, size :: call
    draw_demo_feature_panel :: self, win, (demo_feature_geometry :: size :: call) :: call
    draw_demo_axis_panel :: self, win, (demo_axis_geometry :: size :: call) :: call
    draw_demo_wrap_panel :: self, win, (demo_wrap_geometry :: size :: call) :: call
    draw_demo_bidi_panel :: self, win, (demo_bidi_geometry :: size :: call) :: call
    draw_demo_query_panel :: self, win, (demo_query_geometry :: size :: call) :: call
    draw_demo_stress_panel :: self, win, (demo_stress_geometry :: size :: call) :: call

fn snapshot_query_block(edit renderer: arcana_text.raster.TextRenderer, width: Int) -> arcana_text.layout.LayoutSnapshot:
    let mut style = code_feature_style :: (rgb :: 225, 236, 247 :: call), 22 :: call
    style.background_enabled = true
    style.background_color = rgb :: 20, 34, 51 :: call
    let paragraph = paragraph_left :: :: call
    let mut buffer = arcana_text.buffer.open :: "Mutable proof.", style, paragraph :: call
    let mut editor = arcana_text.editor.open :: buffer :: call
    editor :: buffer, (arcana_text.types.TextRange :: start = 0, end = 7 :: call) :: select_range
    editor :: buffer, "Edited" :: apply_committed_text
    editor :: buffer, 8 :: set_cursor
    let mut spec = render_spec :: buffer.text, width, style :: call
    spec.paragraph = paragraph
    let config = arcana_text.types.default_layout_config :: (max_int :: spec.width, 1 :: call), spec.paragraph :: call
    return renderer :: buffer, config :: snapshot

fn fill_rect(edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int)), color: Int):
    arcana_desktop.canvas.rect :: win, geometry, color :: call

fn stroke_line(edit win: arcana_desktop.types.Window, read path: ((Int, Int), (Int, Int)), color: Int):
    arcana_desktop.canvas.line :: win, path, color :: call

fn draw_label(edit win: arcana_desktop.types.Window, read payload: ((Int, Int), (Str, Int))):
    arcana_desktop.canvas.label :: win, payload.0, payload.1 :: call

fn rgba_from_surface(read payload: (arcana_text.types.GlyphSurface, Int)) -> Array[Int]:
    let surface = payload.0
    let color = payload.1
    if surface.format == (arcana_text.types.GlyphSurfaceFormat.Rgba8 :: :: call):
        return surface.pixels
    let total = surface.size.0 * surface.size.1
    let mut rgba = std.kernel.collections.array_new[Int] :: (total * 4), 0 :: call
    let red = (color / 65536) % 256
    let green = (color / 256) % 256
    let blue = color % 256
    let mut index = 0
    while index < total:
        let base = index * 4
        if surface.format == (arcana_text.types.GlyphSurfaceFormat.LcdSubpixel :: :: call):
            let mask_base = index * 3
            let red_mask = (surface.pixels)[mask_base]
            let green_mask = (surface.pixels)[mask_base + 1]
            let blue_mask = (surface.pixels)[mask_base + 2]
            rgba[base] = (red * red_mask) / 255
            rgba[base + 1] = (green * green_mask) / 255
            rgba[base + 2] = (blue * blue_mask) / 255
            let mut alpha = red_mask
            if green_mask > alpha:
                alpha = green_mask
            if blue_mask > alpha:
                alpha = blue_mask
            rgba[base + 3] = alpha
        else:
            rgba[base] = red
            rgba[base + 1] = green
            rgba[base + 2] = blue
            rgba[base + 3] = (surface.pixels)[index]
        index += 1
    return rgba

fn surface_image(read draw: arcana_text.types.GlyphImageDraw) -> Option[std.canvas.Image]:
    if draw.surface.size.0 <= 0 or draw.surface.size.1 <= 0:
        return Option.None[std.canvas.Image] :: :: call
    let rgba = rgba_from_surface :: (draw.surface, draw.color) :: call
    let mut image = std.kernel.gfx.canvas_image_create :: draw.surface.size.0, draw.surface.size.1 :: call
    std.kernel.gfx.canvas_image_replace_rgba :: image, rgba :: call
    return Option.Some[std.canvas.Image] :: image :: call

fn surface_alpha_at(read surface: arcana_text.types.GlyphSurface, x: Int, y: Int) -> Int:
    let base = (y * surface.stride) + x
    return match surface.format:
        arcana_text.types.GlyphSurfaceFormat.LcdSubpixel =>
            let mut alpha = (surface.pixels)[base * 3]
            if (surface.pixels)[(base * 3) + 1] > alpha:
                alpha = (surface.pixels)[(base * 3) + 1]
            if (surface.pixels)[(base * 3) + 2] > alpha:
                alpha = (surface.pixels)[(base * 3) + 2]
            alpha
        arcana_text.types.GlyphSurfaceFormat.Rgba8 => (surface.pixels)[(base * 4) + 3]
        _ => (surface.pixels)[base]

fn paint_surface(edit win: arcana_desktop.types.Window, read payload: (arcana_text.types.GlyphImageDraw, (Int, Int))):
    let image = payload.0
    let origin = payload.1
    let width = image.surface.size.0
    let height = image.surface.size.1
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            while x < width and (surface_alpha_at :: image.surface, x, y :: call) <= 24:
                x += 1
            if x >= width:
                break
            let start = x
            while x < width and (surface_alpha_at :: image.surface, x, y :: call) > 24:
                x += 1
            fill_rect :: win, ((origin.0 + start, origin.1 + y), (x - start, 1)), image.color :: call
        y += 1

fn paint_stream(edit win: arcana_desktop.types.Window, read payload: (arcana_text.raster.GlyphDrawStream, (Int, Int))):
    let stream = payload.0
    let origin = payload.1
    for glyph in stream.glyphs:
        if glyph.background_enabled:
            fill_rect :: win, ((origin.0 + glyph.position.0, origin.1 + glyph.position.1), glyph.size), glyph.background_color :: call
    for decoration in stream.decorations:
        fill_rect :: win, ((origin.0 + decoration.position.0, origin.1 + decoration.position.1), decoration.size), decoration.color :: call
    for image in stream.images:
        paint_surface :: win, (image, (origin.0 + image.position.0, origin.1 + image.position.1)) :: call

fn paint_range_boxes(edit win: arcana_desktop.types.Window, read spec: RangePaintSpec):
    for box in spec.boxes:
        fill_rect :: win, ((spec.origin.0 + box.position.0, spec.origin.1 + box.position.1), box.size), spec.color :: call

fn paint_caret(edit win: arcana_desktop.types.Window, read spec: CaretPaintSpec):
    let width = max_int :: spec.caret.size.0, 2 :: call
    fill_rect :: win, ((spec.origin.0 + spec.caret.position.0, spec.origin.1 + spec.caret.position.1), (width, spec.caret.size.1)), spec.color :: call

fn pixel_sum(read stream: arcana_text.raster.GlyphDrawStream) -> Int:
    let mut total = 0
    for image in stream.images:
        for value in image.surface.pixels:
            total += value
    return total

fn image_span(read stream: arcana_text.raster.GlyphDrawStream) -> Int:
    let mut found = false
    let mut left = 0
    let mut right = 0
    for image in stream.images:
        if image.size.0 <= 0:
            continue
        let start = image.position.0
        let end = image.position.0 + image.size.0
        if not found:
            left = start
            right = end
            found = true
        else:
            if start < left:
                left = start
            if end > right:
                right = end
    if not found:
        return 0
    return right - left

fn bitmap_alpha_sum(read bitmap: arcana_text.font_leaf.GlyphBitmap) -> Int:
    let mut total = 0
    for value in bitmap.alpha:
        total += value
    return total

fn bitmap_ink_span(read bitmap: arcana_text.font_leaf.GlyphBitmap) -> Int:
    if bitmap.size.0 <= 0 or bitmap.size.1 <= 0:
        return 0
    let width = bitmap.size.0
    let mut found = false
    let mut left = 0
    let mut right = 0
    let mut index = 0
    while index < (bitmap.alpha :: :: len):
        let alpha = (bitmap.alpha)[index]
        if alpha > 0:
            let x = index % width
            if not found:
                left = x
                right = x
                found = true
            else:
                if x < left:
                    left = x
                if x > right:
                    right = x
        index += 1
    if not found:
        return 0
    return right - left + 1

fn axis_bitmap(edit renderer: arcana_text.raster.TextRenderer, read payload: (Str, (Int, arcana_text.types.FontAxis))) -> arcana_text.font_leaf.GlyphBitmap:
    let text = payload.0
    let size = payload.1.0
    let axis = payload.1.1
    let mut style = axis_style :: (rgb :: 225, 236, 247 :: call), size, axis :: call
    let matched = renderer.fonts :: style, text :: resolve_style_char
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: text, size, (arcana_text.fonts.style_line_height_milli :: style :: call) :: call
    spec.glyph_index = renderer.fonts :: matched, text :: glyph_index
    spec.traits = arcana_text.fonts.style_traits_for :: style :: call
    spec.feature_signature = arcana_text.types.feature_signature :: style.features :: call
    spec.axis_signature = arcana_text.types.axis_signature :: style.axes :: call
    return renderer.fonts :: matched.id, spec :: render_face_glyph

fn axis_measure(edit renderer: arcana_text.raster.TextRenderer, read payload: (Str, (Int, arcana_text.types.FontAxis))) -> arcana_text.font_leaf.GlyphBitmap:
    let text = payload.0
    let size = payload.1.0
    let axis = payload.1.1
    let mut style = axis_style :: (rgb :: 225, 236, 247 :: call), size, axis :: call
    let matched = renderer.fonts :: style, text :: resolve_style_char
    let mut spec = arcana_text.font_leaf.glyph_render_spec :: text, size, (arcana_text.fonts.style_line_height_milli :: style :: call) :: call
    spec.glyph_index = renderer.fonts :: matched, text :: glyph_index
    spec.traits = arcana_text.fonts.style_traits_for :: style :: call
    spec.feature_signature = arcana_text.types.feature_signature :: style.features :: call
    spec.axis_signature = arcana_text.types.axis_signature :: style.axes :: call
    return renderer.fonts :: matched.id, spec :: measure_face_glyph

fn timed_axis_bitmap(edit renderer: arcana_text.raster.TextRenderer, read payload: (Str, (Int, arcana_text.types.FontAxis))) -> TimedBitmap:
    let start = std.time.monotonic_now_ms :: :: call
    let bitmap = axis_bitmap :: renderer, payload :: call
    let end = std.time.monotonic_now_ms :: :: call
    let mut timed = TimedBitmap :: bitmap = bitmap :: call
    timed.timing = BitmapTiming :: total_ms = (elapsed_ms :: start, end :: call) :: call
    return timed

fn timed_axis_measure(edit renderer: arcana_text.raster.TextRenderer, read payload: (Str, (Int, arcana_text.types.FontAxis))) -> TimedBitmap:
    let start = std.time.monotonic_now_ms :: :: call
    let bitmap = axis_measure :: renderer, payload :: call
    let end = std.time.monotonic_now_ms :: :: call
    let mut timed = TimedBitmap :: bitmap = bitmap :: call
    timed.timing = BitmapTiming :: total_ms = (elapsed_ms :: start, end :: call) :: call
    return timed

fn stress_repeat_count(mode: Int) -> Int:
    if mode == 1:
        return 8
    if mode == 2:
        return 14
    return 4

fn stress_line() -> Str:
    return "Arcana stress -> != -> mmmm -> سلام -> אבג -> 123456789"

fn stress_text(mode: Int) -> Str:
    let mut text = ""
    let mut index = 0
    let total = stress_repeat_count :: mode :: call
    while index < total:
        if index > 0:
            text = text + "\n"
        text = text + (stress_line_text_clean :: :: call)
        index += 1
    return text

fn draw_panel_frame(edit win: arcana_desktop.types.Window, read spec: PanelSpec):
    let panel = rgb :: 11, 18, 28 :: call
    let header = rgb :: 19, 31, 46 :: call
    let border = rgb :: 36, 58, 82 :: call
    fill_rect :: win, (spec.origin, spec.size), panel :: call
    fill_rect :: win, (spec.origin, (spec.size.0, 30)), header :: call
    draw_label :: win, ((spec.origin.0 + 14, spec.origin.1 + 9), (spec.title, (rgb :: 235, 241, 248 :: call))) :: call
    draw_label :: win, ((spec.origin.0 + 14, spec.origin.1 + 42), (spec.subtitle, (rgb :: 132, 153, 176 :: call))) :: call
    stroke_line :: win, ((spec.origin.0, spec.origin.1 + 30), (spec.origin.0 + spec.size.0, spec.origin.1 + 30)), border :: call

fn fallback_panel_subtitle(read self: ProofApp) -> Str:
    if not self.discover_system_fonts:
        return "Bundled Monaspace is active. Run with `--system-fonts` to exercise installed-font fallback too."
    if self.system_fonts_ready:
        return "Installed-font fallback is active for mixed-script runs beyond bundled Monaspace Latin."
    return "The first frame uses bundled Monaspace; installed-font fallback loads immediately after the initial draw."

fn fallback_status_text(read self: ProofApp) -> Str:
    if self.discover_system_fonts and not self.system_fonts_ready:
        return "installed fallback loading..."
    if not self.discover_system_fonts:
        return "installed fallback disabled"
    return "installed fallback ready"

fn request_main_redraw_flow(edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    arcana_desktop.app.request_main_window_redraw :: cx :: call
    return cx.control.control_flow

fn update_demo_frame_stats(edit self: ProofApp, now_ms: Int):
    if self.fps_window_start_ms <= 0:
        self.fps_window_start_ms = now_ms
        self.fps_last_update_ms = now_ms
        self.fps_frame_count = 0
    self.fps_frame_count += 1
    let delta = now_ms - self.fps_window_start_ms
    if delta < 250:
        return
    let safe_delta = max_int :: delta, 1 :: call
    self.fps_tenths = (self.fps_frame_count * 10000) / safe_delta
    self.fps_frame_count = 0
    self.fps_window_start_ms = now_ms
    self.fps_last_update_ms = now_ms

fn on_demo_mouse_up(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, read ev: arcana_desktop.types.MouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    if not self.ui_demo_mode:
        return cx.control.control_flow
    if ev.button != (arcana_desktop.input.mouse_button_code :: "Left" :: call):
        return cx.control.control_flow
    let mut size = self.demo_cache.window_size
    if size.0 <= 0 or size.1 <= 0:
        let win = arcana_desktop.app.main_window_or_cached :: cx :: call
        size = arcana_desktop.window.size :: win :: call
    let button = demo_button_geometry :: size :: call
    if not (point_in_rect :: ev.position, button :: call):
        return cx.control.control_flow
    self.demo_stress_mode = cycle_stress_mode :: self.demo_stress_mode :: call
    self.demo_cache.ready = false
    arcana_desktop.app.request_main_window_redraw :: cx :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn draw_feature_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let size = geometry.1
    let mut panel = panel_spec :: origin, size, "GSUB + Monaspace" :: call
    panel.subtitle = "Default ligature suppression versus recommended Monaspace code features."
    draw_panel_frame :: win, panel :: call
    let text = "!= ->"
    let paragraph = paragraph_left :: :: call
    let content_width = size.0 - 120
    let mut off_spec = render_spec :: text, content_width, (feature_off_style :: (rgb :: 214, 227, 240 :: call), 20 :: call) :: call
    off_spec.paragraph = paragraph
    let mut on_spec = render_spec :: text, content_width, (code_feature_style :: (rgb :: 155, 236, 215 :: call), 20 :: call) :: call
    on_spec.paragraph = paragraph
    let off = render_text :: self.renderer, off_spec :: call
    let on = render_text :: self.renderer, on_spec :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 74), ("off", (rgb :: 155, 170, 186 :: call))) :: call
    paint_stream :: win, (off.1, (origin.0 + 76, origin.1 + 70)) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 116), ("on", (rgb :: 94, 208, 176 :: call))) :: call
    paint_stream :: win, (on.1, (origin.0 + 76, origin.1 + 112)) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 154), ("ss01-ss10 + calt + liga", (rgb :: 113, 141, 170 :: call))) :: call

fn draw_axis_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let size = geometry.1
    let mut panel = panel_spec :: origin, size, "Variable Axes" :: call
    panel.subtitle = "Real Monaspace `wght`, `wdth`, and `slnt` axes flowing through layout and raster."
    draw_panel_frame :: win, panel :: call
    let content_width = (size.0 - 72) / 2
    let weight_light = render_text :: self.renderer, (render_spec :: "mmmm", content_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.weight_axis :: 250 :: call) :: call) :: call) :: call
    let weight_bold = render_text :: self.renderer, (render_spec :: "mmmm", content_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.weight_axis :: 800 :: call) :: call) :: call) :: call
    let width_narrow = render_text :: self.renderer, (render_spec :: "mmmm", content_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.width_axis :: 100 :: call) :: call) :: call) :: call
    let width_wide = render_text :: self.renderer, (render_spec :: "mmmm", content_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.width_axis :: 125 :: call) :: call) :: call) :: call
    let slant_upright = render_text :: self.renderer, (render_spec :: "mmmm", content_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.slant_axis :: 0 :: call) :: call) :: call) :: call
    let slant_italic = render_text :: self.renderer, (render_spec :: "mmmm", content_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.slant_axis :: -11 :: call) :: call) :: call) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 74), ("wght 250", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (weight_light.1, (origin.0 + 16, origin.1 + 90)) :: call
    draw_label :: win, ((origin.0 + 24 + content_width, origin.1 + 74), ("wght 800", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (weight_bold.1, (origin.0 + 24 + content_width, origin.1 + 90)) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 144), ("wdth 100", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (width_narrow.1, (origin.0 + 16, origin.1 + 160)) :: call
    draw_label :: win, ((origin.0 + 24 + content_width, origin.1 + 144), ("wdth 125", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (width_wide.1, (origin.0 + 24 + content_width, origin.1 + 160)) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 214), ("slnt 0", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (slant_upright.1, (origin.0 + 16, origin.1 + 230)) :: call
    draw_label :: win, ((origin.0 + 24 + content_width, origin.1 + 214), ("slnt -11", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (slant_italic.1, (origin.0 + 24 + content_width, origin.1 + 230)) :: call

fn draw_wrap_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let size = geometry.1
    let mut panel = panel_spec :: origin, size, "Wrap + NoWrap" :: call
    panel.subtitle = "Layout keeps word wrapping distinct from single-line no-wrap with ellipsis."
    draw_panel_frame :: win, panel :: call
    let text = "aa aa"
    let column_width = 28
    let mut wrap_spec = render_spec :: text, column_width, (code_feature_style :: (rgb :: 222, 232, 242 :: call), 19 :: call) :: call
    wrap_spec.paragraph = paragraph_wrap_three :: :: call
    let mut nowrap_spec = render_spec :: text, column_width, (code_feature_style :: (rgb :: 222, 232, 242 :: call), 19 :: call) :: call
    nowrap_spec.paragraph = paragraph_no_wrap :: :: call
    let wrap_block = render_text :: self.renderer, wrap_spec :: call
    let nowrap_block = render_text :: self.renderer, nowrap_spec :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 74), ("word wrap", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (wrap_block.1, (origin.0 + 16, origin.1 + 94)) :: call
    draw_label :: win, ((origin.0 + 24 + column_width, origin.1 + 74), ("no-wrap + ellipsis", (rgb :: 132, 153, 176 :: call))) :: call
    paint_stream :: win, (nowrap_block.1, (origin.0 + 24 + column_width, origin.1 + 94)) :: call

fn draw_bidi_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let size = geometry.1
    let mut panel = panel_spec :: origin, size, "Bidi + Fallback" :: call
    panel.subtitle = fallback_panel_subtitle :: self :: call
    draw_panel_frame :: win, panel :: call
    let mixed = render_text :: self.renderer, (render_spec :: (bidi_mixed_text_clean :: :: call), (size.0 - 32), (code_feature_style :: (rgb :: 229, 239, 248 :: call), 23 :: call) :: call) :: call
    let arabic = render_text :: self.renderer, (render_spec :: (bidi_arabic_text_clean :: :: call), (size.0 - 32), (code_feature_style :: (rgb :: 163, 231, 201 :: call), 22 :: call) :: call) :: call
    paint_stream :: win, (mixed.1, (origin.0 + 16, origin.1 + 78)) :: call
    paint_stream :: win, (arabic.1, (origin.0 + 16, origin.1 + 126)) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 170), (("fonts used: " + (std.text.from_int :: (mixed.0.fonts_used :: :: len) :: call) + "  unresolved: " + (std.text.from_int :: (mixed.0.unresolved :: :: len) :: call)), (rgb :: 132, 153, 176 :: call))) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 190), ((fallback_status_text :: self :: call), (rgb :: 94, 208, 176 :: call))) :: call

fn draw_query_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let size = geometry.1
    let mut panel = panel_spec :: origin, size, "Editor + Queries" :: call
    panel.subtitle = "The proof renders an edited buffer, then overlays `range_boxes`, `caret_box`, and `hit_test` output."
    draw_panel_frame :: win, panel :: call
    let query = render_query_block :: self.renderer, (size.0 - 32) :: call
    let selection = arcana_text.queries.range_boxes :: query.0, (arcana_text.types.TextRange :: start = 0, end = 6 :: call) :: call
    let caret = arcana_text.queries.caret_box :: query.0, 8 :: call
    let hit = arcana_text.queries.hit_test :: query.0, (24, 12) :: call
    let content_origin = (origin.0 + 16, origin.1 + 82)
    paint_range_boxes :: win, (RangePaintSpec :: boxes = selection, origin = content_origin, color = (rgb :: 34, 60, 86 :: call) :: call) :: call
    paint_stream :: win, (query.1, content_origin) :: call
    paint_caret :: win, (CaretPaintSpec :: caret = caret, origin = content_origin, color = (rgb :: 250, 207, 120 :: call) :: call) :: call
    draw_label :: win, ((origin.0 + 16, origin.1 + 142), (("hit index " + (std.text.from_int :: hit.index :: call) + "  range boxes " + (std.text.from_int :: (selection :: :: len) :: call)), (rgb :: 132, 153, 176 :: call))) :: call

fn build_metrics(edit renderer: arcana_text.raster.TextRenderer) -> (ProofMetrics, SmokePerf):
    probe_log_append :: "build_metrics:start" :: call
    let mut perf = SmokePerf :: snapshot_ms = 0, draw_stream_ms = 0, feature_render_ms = 0 :: call
    perf.axis_render_ms = 0
    perf.wrap_render_ms = 0
    perf.bidi_render_ms = 0
    perf.query_render_ms = 0
    perf.render_total_ms = 0
    perf.total_ms = 0
    write_smoke_progress :: "features" :: call
    let feature_text = "!= ->"
    let feature_off = timed_render_text_progress :: renderer, ("feature-off", (render_spec :: feature_text, 420, (feature_off_style :: (rgb :: 214, 227, 240 :: call), 20 :: call) :: call)) :: call
    write_smoke_progress :: "features-off" :: call
    let feature_on = timed_render_text_progress :: renderer, ("feature-on", (render_spec :: feature_text, 420, (code_feature_style :: (rgb :: 155, 236, 215 :: call), 20 :: call) :: call)) :: call
    write_smoke_progress :: "features-on" :: call
    perf.snapshot_ms += feature_off.timing.snapshot_ms + feature_on.timing.snapshot_ms
    perf.draw_stream_ms += feature_off.timing.draw_stream_ms + feature_on.timing.draw_stream_ms
    perf.feature_render_ms = feature_off.timing.total_ms + feature_on.timing.total_ms
    probe_log_append :: "build_metrics:features" :: call
    write_smoke_progress :: "axes" :: call
    write_smoke_progress :: "axes-width-100" :: call
    let width_narrow = timed_axis_measure :: renderer, ("m", (22, (arcana_text.monaspace.width_axis :: 100 :: call))) :: call
    write_smoke_progress :: "axes-width-125" :: call
    let width_wide = timed_axis_measure :: renderer, ("m", (22, (arcana_text.monaspace.width_axis :: 125 :: call))) :: call
    write_smoke_progress :: "axes-weight-250" :: call
    let weight_light = timed_axis_bitmap :: renderer, ("m", (22, (arcana_text.monaspace.weight_axis :: 250 :: call))) :: call
    write_smoke_progress :: "axes-weight-800" :: call
    let weight_bold = timed_axis_bitmap :: renderer, ("m", (22, (arcana_text.monaspace.weight_axis :: 800 :: call))) :: call
    write_smoke_progress :: "axes-slnt-0" :: call
    let slant_upright = timed_axis_bitmap :: renderer, ("m", (22, (arcana_text.monaspace.slant_axis :: 0 :: call))) :: call
    write_smoke_progress :: "axes-slnt-11" :: call
    let slant_italic = timed_axis_bitmap :: renderer, ("m", (22, (arcana_text.monaspace.slant_axis :: -11 :: call))) :: call
    perf.axis_render_ms = width_narrow.timing.total_ms + width_wide.timing.total_ms + weight_light.timing.total_ms + weight_bold.timing.total_ms + slant_upright.timing.total_ms + slant_italic.timing.total_ms
    probe_log_append :: "build_metrics:axes" :: call
    write_smoke_progress :: "wrap" :: call
    let wrap_text = "aa aa"
    let mut wrap_spec = render_spec :: wrap_text, 28, (code_feature_style :: (rgb :: 222, 232, 242 :: call), 19 :: call) :: call
    wrap_spec.paragraph = paragraph_wrap_three :: :: call
    let mut nowrap_spec = render_spec :: wrap_text, 28, (code_feature_style :: (rgb :: 222, 232, 242 :: call), 19 :: call) :: call
    nowrap_spec.paragraph = paragraph_no_wrap :: :: call
    write_smoke_progress :: "wrap-wrap" :: call
    let wrap_block = timed_render_text :: renderer, wrap_spec :: call
    write_smoke_progress :: "wrap-nowrap" :: call
    let nowrap_block = timed_render_text :: renderer, nowrap_spec :: call
    perf.snapshot_ms += wrap_block.timing.snapshot_ms + nowrap_block.timing.snapshot_ms
    perf.draw_stream_ms += wrap_block.timing.draw_stream_ms + nowrap_block.timing.draw_stream_ms
    perf.wrap_render_ms = wrap_block.timing.total_ms + nowrap_block.timing.total_ms
    probe_log_append :: "build_metrics:wrap" :: call
    write_smoke_progress :: "bidi" :: call
    let bidi_start = std.time.monotonic_now_ms :: :: call
    let bidi = render_text :: renderer, (render_spec :: (bidi_mixed_text_clean :: :: call), 420, (code_feature_style :: (rgb :: 229, 239, 248 :: call), 23 :: call) :: call) :: call
    let query_start = std.time.monotonic_now_ms :: :: call
    perf.bidi_render_ms = elapsed_ms :: bidi_start, query_start :: call
    write_smoke_progress :: "query" :: call
    let query = timed_render_query_block :: renderer, 420 :: call
    perf.snapshot_ms += query.timing.snapshot_ms
    perf.draw_stream_ms += query.timing.draw_stream_ms
    perf.query_render_ms = query.timing.total_ms
    perf.render_total_ms = perf.feature_render_ms + perf.axis_render_ms + perf.wrap_render_ms + perf.bidi_render_ms + perf.query_render_ms
    probe_log_append :: "build_metrics:query" :: call
    let selection = arcana_text.queries.range_boxes :: query.snapshot, (arcana_text.types.TextRange :: start = 0, end = 6 :: call) :: call
    let hit = arcana_text.queries.hit_test :: query.snapshot, (24, 12) :: call
    let mut metrics = ProofMetrics :: font_sources = (renderer.fonts :: :: count), feature_off_glyphs = (feature_off.snapshot.glyphs :: :: len), feature_on_glyphs = (feature_on.snapshot.glyphs :: :: len) :: call
    metrics.width_narrow = width_narrow.bitmap.advance
    metrics.width_wide = width_wide.bitmap.advance
    metrics.weight_light_alpha = bitmap_alpha_sum :: weight_light.bitmap :: call
    metrics.weight_bold_alpha = bitmap_alpha_sum :: weight_bold.bitmap :: call
    metrics.slant_upright_span = bitmap_ink_span :: slant_upright.bitmap :: call
    metrics.slant_italic_span = bitmap_ink_span :: slant_italic.bitmap :: call
    metrics.wrap_lines = wrap_block.snapshot.lines :: :: len
    metrics.nowrap_lines = nowrap_block.snapshot.lines :: :: len
    metrics.bidi_unresolved = bidi.0.unresolved :: :: len
    metrics.bidi_fonts = bidi.0.fonts_used :: :: len
    metrics.edit_hit_index = hit.index
    metrics.edit_range_boxes = selection :: :: len
    metrics.primary_font = first_font_name :: feature_on.snapshot :: call
    probe_log_append :: "build_metrics:done" :: call
    return (metrics, perf)

fn feature_width(read snapshot: arcana_text.layout.LayoutSnapshot) -> Int:
    return snapshot :: :: longest_line

fn draw_metrics_panel(edit self: ProofApp, edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int))):
    let origin = geometry.0
    let size = geometry.1
    let mut panel = panel_spec :: origin, size, "Proof Readback" :: call
    panel.subtitle = "Live metrics pulled from the same snapshots that feed the rendered proof."
    draw_panel_frame :: win, panel :: call
    let metrics = (build_metrics :: self.renderer :: call).0
    let label_color = rgb :: 132, 153, 176 :: call
    let value_color = rgb :: 227, 236, 245 :: call
    let mut y = origin.1 + 74
    draw_label :: win, ((origin.0 + 16, y), ("primary font", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (metrics.primary_font, value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("font sources", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), ((std.text.from_int :: metrics.font_sources :: call), value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("glyphs off / on", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (((std.text.from_int :: metrics.feature_off_glyphs :: call) + " / " + (std.text.from_int :: metrics.feature_on_glyphs :: call)), value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("wdth 100 / 125", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (((std.text.from_int :: metrics.width_narrow :: call) + " / " + (std.text.from_int :: metrics.width_wide :: call)), value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("wght alpha 250 / 800", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (((std.text.from_int :: metrics.weight_light_alpha :: call) + " / " + (std.text.from_int :: metrics.weight_bold_alpha :: call)), value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("slnt span 0 / -11", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (((std.text.from_int :: metrics.slant_upright_span :: call) + " / " + (std.text.from_int :: metrics.slant_italic_span :: call)), value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("wrap lines / nowrap", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (((std.text.from_int :: metrics.wrap_lines :: call) + " / " + (std.text.from_int :: metrics.nowrap_lines :: call)), value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("bidi fonts / unresolved", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (((std.text.from_int :: metrics.bidi_fonts :: call) + " / " + (std.text.from_int :: metrics.bidi_unresolved :: call)), value_color)) :: call
    y += 24
    draw_label :: win, ((origin.0 + 16, y), ("query hit / boxes", label_color)) :: call
    draw_label :: win, ((origin.0 + 156, y), (((std.text.from_int :: metrics.edit_hit_index :: call) + " / " + (std.text.from_int :: metrics.edit_range_boxes :: call)), value_color)) :: call

fn draw_header(edit self: ProofApp, edit win: arcana_desktop.types.Window, width: Int):
    let _ = self
    let bg = rgb :: 7, 12, 20 :: call
    fill_rect :: win, ((0, 0), (width, 70)), bg :: call
    draw_label :: win, ((28, 22), ("Arcana Text Proof", (rgb :: 241, 245, 249 :: call))) :: call
    draw_label :: win, ((28, 44), ("Arcana-owned shaping, layout, raster, queries, and Monaspace OpenType variation in one window.", (rgb :: 132, 153, 176 :: call))) :: call

fn draw_proof(edit self: ProofApp, edit win: arcana_desktop.types.Window):
    probe_log_append :: "draw_proof:start" :: call
    let bg = rgb :: 4, 8, 14 :: call
    arcana_desktop.canvas.fill :: win, bg :: call
    let size = arcana_desktop.window.size :: win :: call
    draw_header :: self, win, size.0 :: call
    probe_log_append :: "draw_proof:header" :: call
    let gutter = 24
    if self.ui_demo_mode:
        draw_ui_demo :: self, win, size :: call
        arcana_desktop.canvas.present :: win :: call
        probe_log_append :: "draw_proof:present" :: call
        return
    if self.ui_smoke_mode:
        let proof_width = size.0 - (gutter * 2)
        let feature_width = (proof_width - 16) / 2
        let mut feature_off = render_spec :: "!= ->", feature_width, (feature_off_style :: (rgb :: 214, 227, 240 :: call), 20 :: call) :: call
        let mut feature_on = render_spec :: "!= ->", feature_width, (code_feature_style :: (rgb :: 155, 236, 215 :: call), 20 :: call) :: call
        let feature_off_block = render_text :: self.renderer, feature_off :: call
        let feature_on_block = render_text :: self.renderer, feature_on :: call
        draw_label :: win, ((gutter, 88), ("features off", (rgb :: 132, 153, 176 :: call))) :: call
        paint_stream :: win, (feature_off_block.1, (gutter, 108)) :: call
        draw_label :: win, ((gutter + feature_width + 16, 88), ("features on", (rgb :: 132, 153, 176 :: call))) :: call
        paint_stream :: win, (feature_on_block.1, (gutter + feature_width + 16, 108)) :: call
        probe_log_append :: "draw_proof:ui_smoke_feature" :: call
        let axis_y = 188
        let axis_width = (proof_width - 16) / 2
        let weight_light = render_text :: self.renderer, (render_spec :: "mmmm", axis_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.weight_axis :: 250 :: call) :: call) :: call) :: call
        let weight_bold = render_text :: self.renderer, (render_spec :: "mmmm", axis_width, (axis_style :: (rgb :: 225, 236, 247 :: call), 22, (arcana_text.monaspace.weight_axis :: 800 :: call) :: call) :: call) :: call
        draw_label :: win, ((gutter, axis_y), ("wght 250", (rgb :: 132, 153, 176 :: call))) :: call
        paint_stream :: win, (weight_light.1, (gutter, axis_y + 20)) :: call
        draw_label :: win, ((gutter + axis_width + 16, axis_y), ("wght 800", (rgb :: 132, 153, 176 :: call))) :: call
        paint_stream :: win, (weight_bold.1, (gutter + axis_width + 16, axis_y + 20)) :: call
        probe_log_append :: "draw_proof:ui_smoke_axis" :: call
        arcana_desktop.canvas.present :: win :: call
        probe_log_append :: "draw_proof:present" :: call
        return
    let column_width = (size.0 - gutter * 3) / 2
    let left_x = gutter
    let right_x = gutter * 2 + column_width
    draw_feature_panel :: self, win, ((left_x, 88), (column_width, 184)) :: call
    probe_log_append :: "draw_proof:feature" :: call
    draw_axis_panel :: self, win, ((left_x, 288), (column_width, 304)) :: call
    probe_log_append :: "draw_proof:axis" :: call
    draw_wrap_panel :: self, win, ((left_x, 608), (column_width, 168)) :: call
    probe_log_append :: "draw_proof:wrap" :: call
    draw_bidi_panel :: self, win, ((right_x, 88), (column_width, 206)) :: call
    probe_log_append :: "draw_proof:bidi" :: call
    draw_query_panel :: self, win, ((right_x, 310), (column_width, 188)) :: call
    probe_log_append :: "draw_proof:query" :: call
    draw_metrics_panel :: self, win, ((right_x, 514), (column_width, 262)) :: call
    probe_log_append :: "draw_proof:metrics" :: call
    arcana_desktop.canvas.present :: win :: call
    probe_log_append :: "draw_proof:present" :: call

fn run_smoke(edit renderer: arcana_text.raster.TextRenderer) -> Int:
    probe_log_append :: "run_smoke:enter" :: call
    let smoke_started = std.time.monotonic_now_ms :: :: call
    reset_smoke_report :: :: call
    reset_smoke_progress :: :: call
    write_smoke_progress :: "start" :: call
    let built = build_metrics :: renderer :: call
    write_smoke_progress :: "report" :: call
    let smoke_finished = std.time.monotonic_now_ms :: :: call
    let mut perf = built.1
    perf.total_ms = elapsed_ms :: smoke_started, smoke_finished :: call
    let report_path = smoke_report_path :: :: call
    let _ = std.fs.mkdir_all :: (std.path.parent :: report_path :: call) :: call
    let report_text = append_timing_lines :: ((smoke_report_text :: built.0 :: call), perf) :: call
    let _ = std.fs.write_text :: report_path, report_text :: call
    probe_log_append :: ("run_smoke:done path=" + report_path) :: call
    return 0

fn run_feature_smoke(edit renderer: arcana_text.raster.TextRenderer) -> Int:
    let feature_text = "!= ->"
    let smoke_started = std.time.monotonic_now_ms :: :: call
    reset_smoke_report :: :: call
    reset_smoke_progress :: :: call
    write_smoke_progress :: "feature-smoke-start" :: call
    write_smoke_progress :: "feature-smoke-off-start" :: call
    let feature_off_style_value = feature_off_style :: (rgb :: 214, 227, 240 :: call), 20 :: call
    write_smoke_progress :: "feature-smoke-off-style" :: call
    let feature_off_spec = render_spec :: feature_text, 420, feature_off_style_value :: call
    write_smoke_progress :: "feature-smoke-off-spec" :: call
    let feature_off = timed_render_text_progress :: renderer, ("feature-off", feature_off_spec) :: call
    write_smoke_progress :: ("feature-smoke-off-done snapshot_ms=" + (std.text.from_int :: feature_off.timing.snapshot_ms :: call) + " draw_ms=" + (std.text.from_int :: feature_off.timing.draw_stream_ms :: call)) :: call
    write_smoke_progress :: "feature-smoke-on-start" :: call
    let feature_on_style_value = code_feature_style :: (rgb :: 155, 236, 215 :: call), 20 :: call
    write_smoke_progress :: "feature-smoke-on-style" :: call
    let feature_on_spec = render_spec :: feature_text, 420, feature_on_style_value :: call
    write_smoke_progress :: "feature-smoke-on-spec" :: call
    let feature_on = timed_render_text_progress :: renderer, ("feature-on", feature_on_spec) :: call
    write_smoke_progress :: ("feature-smoke-on-done snapshot_ms=" + (std.text.from_int :: feature_on.timing.snapshot_ms :: call) + " draw_ms=" + (std.text.from_int :: feature_on.timing.draw_stream_ms :: call)) :: call
    let report_path = smoke_report_path :: :: call
    write_smoke_progress :: "feature-smoke-indexes" :: call
    let feature_off_indexes = glyph_indexes_text :: feature_off.snapshot :: call
    let feature_on_indexes = glyph_indexes_text :: feature_on.snapshot :: call
    write_smoke_progress :: "feature-smoke-pixels-off" :: call
    let feature_off_pixels = pixel_sum :: feature_off.stream :: call
    write_smoke_progress :: "feature-smoke-pixels-on" :: call
    let feature_on_pixels = pixel_sum :: feature_on.stream :: call
    write_smoke_progress :: "feature-smoke-font" :: call
    let primary_font = first_font_name :: feature_on.snapshot :: call
    let mut report_text = feature_smoke_report_text :: ((feature_off.snapshot.glyphs :: :: len), (feature_on.snapshot.glyphs :: :: len)), (feature_off_indexes, feature_on_indexes), ((feature_off_pixels, feature_on_pixels), primary_font) :: call
    report_text = append_metric_line :: report_text, "feature_off_snapshot_ms", feature_off.timing.snapshot_ms :: call
    report_text = append_metric_line :: report_text, "feature_off_draw_stream_ms", feature_off.timing.draw_stream_ms :: call
    report_text = append_metric_line :: report_text, "feature_off_total_ms", feature_off.timing.total_ms :: call
    report_text = append_metric_line :: report_text, "feature_on_snapshot_ms", feature_on.timing.snapshot_ms :: call
    report_text = append_metric_line :: report_text, "feature_on_draw_stream_ms", feature_on.timing.draw_stream_ms :: call
    report_text = append_metric_line :: report_text, "feature_on_total_ms", feature_on.timing.total_ms :: call
    let smoke_finished = std.time.monotonic_now_ms :: :: call
    report_text = append_metric_line :: report_text, "total_smoke_ms", (elapsed_ms :: smoke_started, smoke_finished :: call) :: call
    let _ = std.fs.mkdir_all :: (std.path.parent :: report_path :: call) :: call
    let _ = std.fs.write_text :: report_path, report_text :: call
    write_smoke_progress :: "feature-smoke-done" :: call
    return 0

fn run_axis_smoke(edit renderer: arcana_text.raster.TextRenderer) -> Int:
    let smoke_started = std.time.monotonic_now_ms :: :: call
    reset_smoke_report :: :: call
    reset_smoke_progress :: :: call
    write_smoke_progress :: "axis-smoke-start" :: call
    let weight = timed_axis_bitmap :: renderer, ("m", (22, (arcana_text.monaspace.weight_axis :: 250 :: call))) :: call
    let report_path = smoke_report_path :: :: call
    let mut report_text = "axis_smoke_alpha: " + (std.text.from_int :: (bitmap_alpha_sum :: weight.bitmap :: call) :: call) + "\n"
    report_text = append_metric_line :: report_text, "axis_render_ms", weight.timing.total_ms :: call
    let smoke_finished = std.time.monotonic_now_ms :: :: call
    report_text = append_metric_line :: report_text, "total_smoke_ms", (elapsed_ms :: smoke_started, smoke_finished :: call) :: call
    let _ = std.fs.mkdir_all :: (std.path.parent :: report_path :: call) :: call
    let _ = std.fs.write_text :: report_path, report_text :: call
    write_smoke_progress :: ("axis-smoke-alpha=" + (std.text.from_int :: (bitmap_alpha_sum :: weight.bitmap :: call) :: call)) :: call
    return 0

fn on_redraw(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    probe_log_append :: "on_redraw:event" :: call
    let found = arcana_desktop.app.require_target_window :: cx, target :: call
    return match found:
        Result.Ok(value) => on_redraw_ready :: self, cx, value :: call
        Result.Err(_) => arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_redraw_ready(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, take value: arcana_desktop.types.Window) -> arcana_desktop.types.ControlFlow:
    let mut win = value
    self.redraw_count += 1
    probe_log_append :: ("on_redraw:ready count=" + (std.text.from_int :: self.redraw_count :: call)) :: call
    let draw_started = std.time.monotonic_now_ms :: :: call
    draw_proof :: self, win :: call
    let draw_finished = std.time.monotonic_now_ms :: :: call
    self.last_paint_ms = elapsed_ms :: draw_started, draw_finished :: call
    if self.ui_demo_mode:
        update_demo_frame_stats :: self, draw_finished.value :: call
        arcana_desktop.app.request_main_window_redraw :: cx :: call
    if self.ui_smoke_mode and ((not self.discover_system_fonts) or self.system_fonts_ready) and self.redraw_count >= 1:
        arcana_desktop.app.request_exit :: cx, 0 :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_close_requested(edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    arcana_desktop.app.request_exit :: cx, 0 :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_key_down(edit cx: arcana_desktop.types.AppContext, read ev: arcana_desktop.types.KeyEvent) -> arcana_desktop.types.ControlFlow:
    let escape = arcana_desktop.input.key_code :: "Escape" :: call
    if ev.key == escape:
        arcana_desktop.app.request_exit :: cx, 0 :: call
        return arcana_desktop.types.ControlFlow.Wait :: :: call
    return cx.control.control_flow

impl arcana_desktop.app.Application[ProofApp] for ProofApp:
    fn resumed(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext):
        probe_log_append :: "app:resumed" :: call
        arcana_desktop.app.request_main_window_redraw :: cx :: call
        return

    fn suspended(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext):
        let _ = self
        let _ = cx
        return

    fn window_event(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
        return match target.event:
            arcana_desktop.types.WindowEvent.WindowRedrawRequested(_) => on_redraw :: self, cx, target :: call
            arcana_desktop.types.WindowEvent.WindowResized(_) => request_main_redraw_flow :: cx :: call
            arcana_desktop.types.WindowEvent.WindowScaleFactorChanged(_) => request_main_redraw_flow :: cx :: call
            arcana_desktop.types.WindowEvent.WindowCloseRequested(_) => on_close_requested :: cx :: call
            arcana_desktop.types.WindowEvent.MouseUp(ev) => on_demo_mouse_up :: self, cx, ev :: call
            arcana_desktop.types.WindowEvent.KeyDown(ev) => on_key_down :: cx, ev :: call
            _ => cx.control.control_flow

    fn device_event(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:
        let _ = self
        let _ = event
        return cx.control.control_flow

    fn about_to_wait(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        probe_log_append :: ("app:about_to_wait redraws=" + (std.text.from_int :: self.redraw_count :: call) + " fonts_ready=" + (std.text.from_int :: (bool_code :: self.system_fonts_ready :: call) :: call)) :: call
        if self.discover_system_fonts and not self.system_fonts_ready and self.redraw_count > 0:
            probe_log_append :: "app:discover_installed:start" :: call
            let _ = self.renderer.fonts :: :: discover_installed
            self.system_fonts_ready = true
            self.demo_cache.ready = false
            probe_log_append :: "app:discover_installed:done" :: call
            arcana_desktop.app.request_main_window_redraw :: cx :: call
        return cx.control.control_flow

    fn wake(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        let _ = self
        return cx.control.control_flow

    fn exiting(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext):
        let _ = self
        let _ = cx
        return

fn main() -> Int:
    let smoke_mode = has_flag :: "--smoke" :: call
    let feature_smoke_mode = has_flag :: "--smoke-features" :: call
    let axis_smoke_mode = has_flag :: "--smoke-axis" :: call
    let ui_smoke_mode = has_flag :: "--ui-smoke" :: call
    let ui_demo_mode = has_flag :: "--ui-demo" :: call
    let use_system_fonts = has_flag :: "--system-fonts" :: call
    reset_probe_log :: :: call
    probe_log_append :: ("main:flags smoke=" + (std.text.from_int :: (bool_code :: smoke_mode :: call) :: call) + " features=" + (std.text.from_int :: (bool_code :: feature_smoke_mode :: call) :: call) + " axis=" + (std.text.from_int :: (bool_code :: axis_smoke_mode :: call) :: call) + " ui=" + (std.text.from_int :: (bool_code :: ui_smoke_mode :: call) :: call) + " demo=" + (std.text.from_int :: (bool_code :: ui_demo_mode :: call) :: call) + " sys=" + (std.text.from_int :: (bool_code :: use_system_fonts :: call) :: call)) :: call
    let discover_system_fonts = ((not smoke_mode) and (not feature_smoke_mode) and (not axis_smoke_mode) and (not ui_smoke_mode) and (not ui_demo_mode)) or use_system_fonts
    let mut app = ProofApp :: renderer = (arcana_text.raster.default_renderer :: :: call), discover_system_fonts = discover_system_fonts, system_fonts_ready = false :: call
    probe_log_append :: "main:renderer_ready" :: call
    app.ui_smoke_mode = ui_smoke_mode
    app.ui_demo_mode = ui_demo_mode
    app.redraw_count = 0
    app.demo_stress_mode = 0
    app.demo_cache = empty_demo_cache :: :: call
    app.fps_window_start_ms = 0
    app.fps_frame_count = 0
    app.fps_tenths = 0
    app.fps_last_update_ms = 0
    app.last_paint_ms = 0
    if (smoke_mode or feature_smoke_mode) and use_system_fonts:
        let _ = app.renderer.fonts :: :: discover_installed
        app.system_fonts_ready = true
    if axis_smoke_mode:
        return run_axis_smoke :: app.renderer :: call
    if feature_smoke_mode:
        return run_feature_smoke :: app.renderer :: call
    if smoke_mode:
        probe_log_append :: "main:smoke_branch" :: call
        return run_smoke :: app.renderer :: call
    let mut cfg = arcana_desktop.app.default_app_config :: :: call
    cfg.window.title = "Arcana Text Proof"
    cfg.window.bounds.size = (1460, 820)
    cfg.window.bounds.position = (48, 40)
    cfg.window.bounds.min_size = (1200, 780)
    probe_log_append :: "main:before_app_run" :: call
    return arcana_desktop.app.run :: app, cfg :: call
