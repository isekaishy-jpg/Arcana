import arcana_text.buffer
import arcana_text.cache
import arcana_text.editor
import arcana_text.fonts
import arcana_text.layout
import arcana_text.monaspace
import arcana_text.queries
import arcana_text.raster
import std.args
import std.bytes
import std.collections.list
import std.fs
import std.io
import std.text

record SnapshotSpec:
    text: Str
    width: Int
    color: Int
    size: Int
    align: arcana_text.types.TextAlign

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

fn rgb(r: Int, g: Int, b: Int) -> Int:
    return (r * 65536) + (g * 256) + b

fn monaspace_style(color: Int, size: Int) -> arcana_text.types.TextStyle:
    let mut style = arcana_text.types.default_text_style :: color :: call
    style.size = size
    style.background_enabled = true
    style.background_color = rgb :: 16, 28, 42 :: call
    style.families :: (arcana_text.monaspace.family_name :: (arcana_text.monaspace.default_family :: :: call) :: call) :: push
    return style

fn build_snapshot(edit fonts: arcana_text.fonts.FontSystem, read spec: SnapshotSpec) -> arcana_text.layout.LayoutSnapshot:
    let style = monaspace_style :: spec.color, spec.size :: call
    let mut paragraph = arcana_text.types.default_paragraph_style :: :: call
    paragraph.align = spec.align
    paragraph.max_lines = 2
    let buffer = arcana_text.buffer.open :: spec.text, style, paragraph :: call
    let config = arcana_text.types.default_layout_config :: spec.width, paragraph :: call
    return arcana_text.layout.snapshot :: fonts, buffer, config :: call

fn build_mutated_snapshot(edit fonts: arcana_text.fonts.FontSystem, width: Int, color: Int) -> arcana_text.layout.LayoutSnapshot:
    let mut style = monaspace_style :: color, 22 :: call
    style.background_color = rgb :: 18, 35, 49 :: call
    let mut paragraph = arcana_text.types.default_paragraph_style :: :: call
    paragraph.align = arcana_text.types.TextAlign.Center :: :: call
    paragraph.max_lines = 2
    let mut buffer = arcana_text.buffer.open :: "Mutable update path centered after first layout.", style, paragraph :: call
    let mut editor = arcana_text.editor.open :: buffer :: call
    editor :: buffer, 0, 7 :: select_range
    editor :: buffer, "Updated" :: apply_committed_text
    let config = arcana_text.types.default_layout_config :: width, paragraph :: call
    return arcana_text.layout.snapshot :: fonts, buffer, config :: call

fn print_metric(label: Str, value: Int):
    std.io.print[Str] :: (label + ": " + (std.text.from_int :: value :: call) + "\n") :: call

fn print_text_metric(label: Str, value: Str):
    std.io.print[Str] :: (label + ": " + value + "\n") :: call

fn probe(label: Str):
    if not (probe_enabled :: :: call):
        return
    let _ = std.fs.mkdir_all :: "scratch" :: call
    let opened = std.fs.stream_open_write :: "scratch/text_proof_probe.log", true :: call
    match opened:
        std.result.Result.Ok(value) => probe_ready :: value, label :: call
        std.result.Result.Err(_) => 0

fn probe_ready(take value: std.fs.FileStream, label: Str):
    let mut stream = value
    let bytes = std.bytes.from_str_utf8 :: (label + "\n") :: call
    let _ = std.fs.stream_write :: stream, bytes :: call
    let _ = std.fs.stream_close :: stream :: call

fn first_font_name(read snapshot: arcana_text.layout.LayoutSnapshot) -> Str:
    let used = arcana_text.queries.fonts_used :: snapshot :: call
    if used :: :: is_empty:
        return ""
    return arcana_text.fonts.family_or_label :: used[0].source :: call

fn main() -> Int:
    probe :: "entry" :: call
    std.io.print[Str] :: "stage:entry\n" :: call
    std.io.flush_stdout :: :: call
    let smoke_mode = has_flag :: "--smoke" :: call
    let use_system_fonts = has_flag :: "--system-fonts" :: call
    let mut fonts = arcana_text.fonts.default_system :: :: call
    probe :: "after_default_system" :: call
    std.io.print[Str] :: "stage:fonts\n" :: call
    std.io.flush_stdout :: :: call
    if use_system_fonts:
        let discovered = fonts :: :: discover_installed
        if not smoke_mode:
            print_metric :: "discovered_fonts", discovered :: call
    probe :: "before_primary_snapshot" :: call

    let mut snapshot_spec = SnapshotSpec :: text = "Styled engine rebuild proof using Monaspace defaults and explicit Arcana objects.", width = 260, color = (rgb :: 226, 232, 240 :: call) :: call
    snapshot_spec.size = 18
    snapshot_spec.align = arcana_text.types.TextAlign.Left :: :: call
    let snapshot = build_snapshot :: fonts, snapshot_spec :: call
    probe :: "after_primary_snapshot" :: call
    let mutated = build_mutated_snapshot :: fonts, 280, (rgb :: 129, 230, 217 :: call) :: call
    probe :: "after_mutated_snapshot" :: call
    let metrics = arcana_text.queries.line_metrics :: snapshot :: call
    let unresolved = arcana_text.queries.unresolved_glyphs :: snapshot :: call
    let hit = arcana_text.queries.hit_test :: snapshot, (24, 12) :: call
    let caret = arcana_text.queries.caret_box :: snapshot, 6 :: call
    let range = arcana_text.queries.range_boxes :: snapshot, (arcana_text.types.TextRange :: start = 0, end = 12 :: call) :: call
    let mut cache = arcana_text.cache.open :: :: call
    let raster_cfg = arcana_text.types.default_raster_config :: :: call
    probe :: "before_raster" :: call
    let stream = arcana_text.raster.draw_stream :: fonts, cache, (snapshot, raster_cfg) :: call
    let stream_again = arcana_text.raster.draw_stream :: fonts, cache, (snapshot, raster_cfg) :: call
    let mutated_stream = arcana_text.raster.draw_stream :: fonts, cache, (mutated, raster_cfg) :: call
    probe :: "after_raster" :: call
    if smoke_mode:
        print_metric :: "font_sources", (fonts :: :: count) :: call
        print_metric :: "lines", (metrics :: :: len) :: call
        print_metric :: "glyphs", (snapshot.glyphs :: :: len) :: call
        print_metric :: "unresolved", (unresolved :: :: len) :: call
        print_metric :: "hit_index", hit.index :: call
        print_metric :: "caret_y", caret.position.1 :: call
        print_metric :: "range_boxes", (range :: :: len) :: call
        print_metric :: "stream_width", stream.size.0 :: call
        print_metric :: "stream_height", stream.size.1 :: call
        print_metric :: "cached_width", stream_again.size.0 :: call
        print_metric :: "mutated_width", mutated_stream.size.0 :: call
        print_text_metric :: "primary_font", (first_font_name :: snapshot :: call) :: call
        return 0
    print_metric :: "font_sources", (fonts :: :: count) :: call
    print_metric :: "snapshot_width", (snapshot :: :: longest_line) :: call
    print_metric :: "snapshot_height", (snapshot :: :: height) :: call
    print_metric :: "lines", (metrics :: :: len) :: call
    print_metric :: "glyphs", (snapshot.glyphs :: :: len) :: call
    print_metric :: "unresolved", (unresolved :: :: len) :: call
    print_metric :: "range_boxes", (range :: :: len) :: call
    print_metric :: "mutated_width", (mutated :: :: longest_line) :: call
    print_metric :: "mutated_height", (mutated :: :: height) :: call
    print_metric :: "image_width", stream.size.0 :: call
    print_metric :: "image_height", stream.size.1 :: call
    print_text_metric :: "primary_font", (first_font_name :: snapshot :: call) :: call
    return 0
