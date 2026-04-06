import actions
import arcana_desktop.monitor
import arcana_desktop.window
import arcana_text.buffer
import arcana_text.cache
import arcana_text.fonts
import arcana_text.monaspace
import arcana_text.raster
import arcana_text.types
import demo_types
import layout
import pages
import std.canvas
import std.bytes
import std.collections.array
import std.collections.list
import std.fs
import std.option
import std.text
use std.option.Option

record Palette:
    surfaces: (Int, (Int, Int))
    tones: (Int, (Int, Int))
    accent: Int

record TextBlock:
    pos: (Int, Int)
    layout: (Int, Int)
    text_and_color: (Str, Int)

record TextLabel:
    pos: (Int, Int)
    text: Str
    color: Int

record WrappedLinesBlock:
    pos: (Int, Int)
    max_lines: Int
    color: Int

record MetricLine:
    pos: (Int, Int)
    text: (Str, Str)
    colors: (Int, Int)

record CardBlock:
    pos: (Int, Int)
    size: (Int, Int)
    title: Str

record ControlDeckState:
    text_input_enabled: Bool
    cursor_visible: Bool
    topmost: Bool
    decorated: Bool
    resizable: Bool
    fullscreen: Bool
    maximized: Bool
    transparent: Bool
    cursor_grabbed: Bool
    composition_area_active: Bool

fn probe_line(read self: demo_types.Demo, head: Str, tail: Str):
    if not self.probe_mode:
        return
    let _ = std.fs.mkdir_all :: "scratch" :: call
    let opened = std.fs.stream_open_write :: "scratch/desktop_probe.log", true :: call
    return match opened:
        std.result.Result.Ok(value) => probe_line_ready :: value, head, tail :: call
        std.result.Result.Err(_) => 0

fn probe_line_ready(take value: std.fs.FileStream, head: Str, tail: Str):
    let mut stream = value
    let bytes = std.bytes.from_str_utf8 :: ("[desktop-proof/render] " + head + " :: " + tail + "\n") :: call
    let _ = std.fs.stream_write :: stream, bytes :: call
    let _ = std.fs.stream_close :: stream :: call

fn default_style(color: Int) -> arcana_text.types.TextStyle:
    let mut style = arcana_text.types.default_text_style :: color :: call
    style.families :: (arcana_text.monaspace.family_name :: (arcana_text.monaspace.default_family :: :: call) :: call) :: push
    return style

fn paint_stream(edit win: arcana_desktop.types.Window, read payload: (arcana_text.raster.GlyphDrawStream, (Int, Int))):
    let stream = payload.0
    let origin = payload.1
    for glyph in stream.glyphs:
        if glyph.background_enabled:
            fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (origin.0 + glyph.position.0, origin.1 + glyph.position.1), size = glyph.size, color = glyph.background_color :: call) :: call
    for image in stream.images:
        let value = surface_image :: image :: call
        if value :: :: is_some:
            let image_value = value :: (std.canvas.image_create :: 1, 1 :: call) :: unwrap_or
            arcana_desktop.canvas.blit :: win, image_value, (origin.0 + image.position.0, origin.1 + image.position.1) :: call

fn rgba_from_surface(read payload: (arcana_text.types.GlyphSurface, Int)) -> Array[Int]:
    let surface = payload.0
    let color = payload.1
    if surface.format == (arcana_text.types.GlyphSurfaceFormat.Rgba8 :: :: call):
        return surface.pixels
    let mut rgba = std.collections.list.empty[Int] :: :: call
    let red = (color / 65536) % 256
    let green = (color / 256) % 256
    let blue = color % 256
    for alpha in surface.pixels:
        rgba :: red :: push
        rgba :: green :: push
        rgba :: blue :: push
        rgba :: alpha :: push
    return std.collections.array.from_list[Int] :: rgba :: call

fn surface_image(read draw: arcana_text.types.GlyphImageDraw) -> Option[std.canvas.Image]:
    if draw.surface.size.0 <= 0 or draw.surface.size.1 <= 0:
        return Option.None[std.canvas.Image] :: :: call
    let rgba = rgba_from_surface :: (draw.surface, draw.color) :: call
    let mut image = std.canvas.image_create :: draw.surface.size.0, draw.surface.size.1 :: call
    std.canvas.image_replace_rgba :: image, rgba :: call
    return Option.Some[std.canvas.Image] :: image :: call

fn draw_label(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read spec: render.TextLabel):
    probe_line :: self, "draw_label", spec.text :: call
    let _ = self
    arcana_desktop.canvas.label :: win, spec.pos, (spec.text, spec.color) :: call

fn measure_text(edit self: demo_types.Demo, text: Str) -> (Int, Int):
    probe_line :: self, "measure_text", text :: call
    let style = default_style :: (rgb :: 255, 255, 255 :: call) :: call
    let paragraph_style = arcana_text.types.default_paragraph_style :: :: call
    let buffer = arcana_text.buffer.open :: text, style, paragraph_style :: call
    let snapshot = self.text_renderer :: buffer, (arcana_text.types.default_layout_config :: 0, paragraph_style :: call) :: snapshot
    return ((snapshot :: :: longest_line), (snapshot :: :: height))

fn text_stream(edit self: demo_types.Demo, read spec: render.TextBlock) -> arcana_text.raster.GlyphDrawStream:
    probe_line :: self, "text_stream", "style" :: call
    let style = default_style :: spec.text_and_color.1 :: call
    let paragraph_style = arcana_text.types.default_paragraph_style :: :: call
    probe_line :: self, "text_stream", "buffer" :: call
    let buffer = arcana_text.buffer.open :: spec.text_and_color.0, style, paragraph_style :: call
    let config = arcana_text.types.default_layout_config :: spec.layout.0, paragraph_style :: call
    probe_line :: self, "text_stream", "snapshot" :: call
    let snapshot = self.text_renderer :: buffer, config :: snapshot
    let raster_cfg = arcana_text.types.default_raster_config :: :: call
    probe_line :: self, "text_stream", "draw_stream" :: call
    return self.text_renderer :: (snapshot, raster_cfg) :: draw_stream

fn sync_body_stream(edit self: demo_types.Demo, read spec: render.TextBlock):
    if self.body_stream_ready and self.body_stream_page_index == self.page_index and self.body_stream_layout.0 == spec.layout.0 and self.body_stream_layout.1 == spec.layout.1 and self.body_stream_color == spec.text_and_color.1:
        return
    self.body_stream = Option.Some[arcana_text.raster.GlyphDrawStream] :: (render.text_stream :: self, spec :: call) :: call
    self.body_stream_ready = true
    self.body_stream_page_index = self.page_index
    self.body_stream_layout = spec.layout
    self.body_stream_color = spec.text_and_color.1

fn estimated_text_width(text: Str) -> Int:
    let mut width = 0
    let total = std.text.len_bytes :: text :: call
    let mut index = 0
    while index < total:
        let glyph = std.text.slice_bytes :: text, index, index + 1 :: call
        if glyph == " ":
            width += 5
        else:
            width += 9
        index += 1
    return width

fn rgb(r: Int, g: Int, b: Int) -> Int:
    return arcana_graphics.canvas.rgb :: r, g, b :: call

fn fill_rect(edit win: arcana_desktop.types.Window, read spec: arcana_graphics.types.RectSpec):
    arcana_graphics.canvas.rect :: win, spec :: call

fn stroke_line(edit win: arcana_desktop.types.Window, read spec: arcana_graphics.types.LineSpec):
    arcana_graphics.canvas.line :: win, spec :: call

fn fill_circle(edit win: arcana_desktop.types.Window, read spec: arcana_graphics.types.CircleFillSpec):
    arcana_graphics.canvas.circle_fill :: win, spec :: call

fn bool_name(value: Bool) -> Str:
    if value:
        return "yes"
    return "no"

fn theme_name(read value: arcana_desktop.types.WindowTheme) -> Str:
    return match value:
        arcana_desktop.types.WindowTheme.Light => "light"
        arcana_desktop.types.WindowTheme.Dark => "dark"
        _ => "unknown"

fn theme_override_name(read value: arcana_desktop.types.WindowThemeOverride) -> Str:
    return match value:
        arcana_desktop.types.WindowThemeOverride.Light => "light"
        arcana_desktop.types.WindowThemeOverride.Dark => "dark"
        _ => "system"

fn cursor_icon_name(read value: arcana_desktop.types.CursorIcon) -> Str:
    return match value:
        arcana_desktop.types.CursorIcon.Text => "text"
        arcana_desktop.types.CursorIcon.Crosshair => "crosshair"
        arcana_desktop.types.CursorIcon.Hand => "hand"
        arcana_desktop.types.CursorIcon.Move => "move"
        arcana_desktop.types.CursorIcon.Wait => "wait"
        arcana_desktop.types.CursorIcon.Help => "help"
        arcana_desktop.types.CursorIcon.NotAllowed => "blocked"
        arcana_desktop.types.CursorIcon.ResizeHorizontal => "resize-h"
        arcana_desktop.types.CursorIcon.ResizeVertical => "resize-v"
        arcana_desktop.types.CursorIcon.ResizeNwse => "resize-nwse"
        arcana_desktop.types.CursorIcon.ResizeNesw => "resize-nesw"
        _ => "default"

fn grab_mode_name(read value: arcana_desktop.types.CursorGrabMode) -> Str:
    return match value:
        arcana_desktop.types.CursorGrabMode.Confined => "confined"
        arcana_desktop.types.CursorGrabMode.Locked => "locked"
        _ => "free"

fn device_policy_name(code: Int) -> Str:
    if code == 0:
        return "never"
    if code == 2:
        return "always"
    return "focused"

fn point_text(read value: (Int, Int)) -> Str:
    return (std.text.from_int :: value.0 :: call) + ", " + (std.text.from_int :: value.1 :: call)

fn size_text(read value: (Int, Int)) -> Str:
    return (std.text.from_int :: value.0 :: call) + " x " + (std.text.from_int :: value.1 :: call)

fn bounds_text(read value: (Int, Int)) -> Str:
    if value.0 <= 0 or value.1 <= 0:
        return "none"
    return size_text :: value :: call

fn scale_text(milli: Int) -> Str:
    let whole = milli / 1000
    let frac = (milli % 1000) / 10
    let mut frac_text = std.text.from_int :: frac :: call
    if frac < 10:
        frac_text = "0" + frac_text
    return (std.text.from_int :: whole :: call) + "." + frac_text + "x"

fn composition_summary(read area: arcana_desktop.types.CompositionArea) -> Str:
    if not area.active:
        return "off"
    return (point_text :: area.position :: call) + " / " + (size_text :: area.size :: call)

fn second_window_summary(read self: demo_types.Demo) -> Str:
    if self.second_window_id < 0:
        return "closed"
    let mut text = "hidden"
    if self.second_window_visible:
        text = "visible"
    if self.second_window_alive:
        return "alive " + text
    return "stale " + text

fn push_wrapped_line(read self: demo_types.Demo, edit out: List[Str], read payload: (Str, Int)):
    let source = payload.0
    let max_width = payload.1
    if max_width <= 0:
        return
    if (std.text.len_bytes :: source :: call) == 0:
        out :: "" :: push
        return
    let words = std.text.split :: source, " " :: call
    let mut current = ""
    for word in words:
        if (std.text.len_bytes :: word :: call) == 0:
            continue
        let mut candidate = word
        if current != "":
            candidate = current + " " + word
        let width = estimated_text_width :: candidate :: call
        if current == "" or width <= max_width:
            current = candidate
        else:
            out :: current :: push
            current = word
    if current != "":
        out :: current :: push

export fn wrapped_lines(read self: demo_types.Demo, read text: Str, max_width: Int) -> List[Str]:
    probe_line :: self, "wrapped_lines_start", ("width=" + (std.text.from_int :: max_width :: call)) :: call
    let mut out = std.collections.list.new[Str] :: :: call
    let lines = std.text.split_lines :: text :: call
    for value in lines:
        push_wrapped_line :: self, out, (value, max_width) :: call
    probe_line :: self, "wrapped_lines_done", ("count=" + (std.text.from_int :: (out :: :: len) :: call)) :: call
    return out

fn draw_wrapped_lines(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read payload: (List[Str], render.WrappedLinesBlock)):
    let lines = payload.0
    let block = payload.1
    let mut y = block.pos.1
    let mut shown = 0
    for value in lines:
        if shown >= block.max_lines:
            return
        draw_label :: self, win, (render.TextLabel :: pos = (block.pos.0, y), text = value, color = block.color :: call) :: call
        y += 18
        shown += 1

fn draw_metric_line(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read spec: render.MetricLine):
    draw_label :: self, win, (render.TextLabel :: pos = spec.pos, text = spec.text.0, color = spec.colors.0 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (spec.pos.0 + 112, spec.pos.1), text = spec.text.1, color = spec.colors.1 :: call) :: call

fn draw_card(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read payload: (render.CardBlock, render.Palette)):
    let pos = payload.0
    let palette = payload.1
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = pos.pos, size = pos.size, color = (rgb :: 17, 27, 39 :: call) :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (pos.pos.0 + 12, pos.pos.1 + 14), text = pos.title, color = palette.accent :: call) :: call

fn control_deck_state(read win: arcana_desktop.types.Window) -> render.ControlDeckState:
    let settings = arcana_desktop.window.settings :: win :: call
    let text = arcana_desktop.window.text_input_settings :: win :: call
    let mut deck = render.ControlDeckState :: text_input_enabled = text.enabled, cursor_visible = settings.options.cursor.visible, topmost = settings.options.state.topmost :: call
    deck.decorated = settings.options.style.decorated
    deck.resizable = settings.options.style.resizable
    deck.fullscreen = settings.options.state.fullscreen
    deck.maximized = settings.options.state.maximized
    deck.transparent = settings.options.style.transparent
    deck.cursor_grabbed = settings.options.cursor.grab_mode != (arcana_desktop.types.CursorGrabMode.Free :: :: call)
    deck.composition_area_active = text.composition_area.active
    return deck

fn button_fill(read self: demo_types.Demo, id: Int, read deck: render.ControlDeckState) -> Int:
    let mut color = rgb :: 26, 38, 54 :: call
    let page = actions.button_page :: id :: call
    if page >= 0 and self.page_index == page:
        color = rgb :: 30, 97, 118 :: call
    if id == 10 and deck.text_input_enabled:
        color = rgb :: 52, 92, 75 :: call
    if id == 12 and deck.cursor_visible:
        color = rgb :: 52, 92, 75 :: call
    if id == 14 and deck.topmost:
        color = rgb :: 52, 92, 75 :: call
    if id == 15 and deck.decorated:
        color = rgb :: 52, 92, 75 :: call
    if id == 16 and deck.resizable:
        color = rgb :: 52, 92, 75 :: call
    if id == 21:
        if self.device_policy_code == 0:
            color = rgb :: 83, 47, 36 :: call
        if self.device_policy_code == 2:
            color = rgb :: 52, 92, 75 :: call
    if id == 22 and self.second_window_id >= 0:
        color = rgb :: 87, 84, 36 :: call
    if id == 23:
        color = rgb :: 92, 42, 44 :: call
    if id == 24 and deck.fullscreen:
        color = rgb :: 52, 92, 75 :: call
    if id == 25 and deck.maximized:
        color = rgb :: 52, 92, 75 :: call
    if id == 27 and deck.transparent:
        color = rgb :: 52, 92, 75 :: call
    if id == 28 and deck.cursor_grabbed:
        color = rgb :: 52, 92, 75 :: call
    if id == 29 and self.move_size_cycle > 0:
        color = rgb :: 87, 84, 36 :: call
    if id == 30 and self.clamp_cycle > 0:
        color = rgb :: 87, 84, 36 :: call
    if id == 31 and self.preset_cycle > 0:
        color = rgb :: 87, 84, 36 :: call
    if id == 32 and deck.composition_area_active:
        color = rgb :: 52, 92, 75 :: call
    if id == 33 and deck.text_input_enabled:
        color = rgb :: 52, 92, 75 :: call
    if id == 34 and self.second_window_id >= 0 and self.second_window_visible:
        color = rgb :: 52, 92, 75 :: call
    if id == 35 and self.second_window_id >= 0:
        color = rgb :: 92, 42, 44 :: call
    if self.mouse_inside and self.hover_button_id == id:
        color = rgb :: 43, 84, 100 :: call
    return color

fn draw_header(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read payload: (layout.ViewLayout, render.Palette)):
    let view = payload.0
    let palette = payload.1
    let page_title = pages.title :: self.page_index :: call
    let status_text = self.status_head + " :: " + self.status_tail
    let monitor_text = "last monitor " + self.last_monitor
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (0, 0), size = (view.window_size.0, view.header_height), color = palette.surfaces.1.0 :: call) :: call
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (0, view.header_height - 4), size = (view.window_size.0, 4), color = palette.tones.0 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (24, 24), text = "Arcana Desktop", color = palette.accent :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (24, 48), text = "authoritative shell showcase", color = palette.tones.1.1 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (view.center_panel.pos.0, 24), text = page_title, color = palette.tones.1.0 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (view.center_panel.pos.0, 48), text = status_text, color = palette.tones.1.1 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (view.right_panel.pos.0, 24), text = "Desktop State", color = palette.tones.1.0 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (view.right_panel.pos.0, 48), text = monitor_text, color = palette.tones.1.1 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (view.left_panel.pos.0, 68), text = "Control Deck", color = palette.tones.1.1 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (view.center_panel.pos.0, 68), text = "Guide Page", color = palette.tones.1.1 :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (view.right_panel.pos.0, 68), text = "Live Readback", color = palette.tones.1.1 :: call) :: call
    let mut badge = rgb :: 73, 92, 109 :: call
    if self.pending_wake:
        badge = rgb :: 222, 160, 76 :: call
    fill_circle :: win, (arcana_graphics.types.CircleFillSpec :: center = (view.left_panel.pos.0 + view.left_panel.size.0 - 26, 42), radius = 8, color = badge :: call) :: call

fn draw_controls(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read payload: (layout.ViewLayout, render.Palette)):
    let view = payload.0
    let palette = payload.1
    let deck = control_deck_state :: win :: call
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = view.left_panel.pos, size = view.left_panel.size, color = palette.surfaces.1.1 :: call) :: call
    stroke_line :: win, (arcana_graphics.types.LineSpec :: start = (view.left_panel.pos.0, view.left_panel.pos.1), end = (view.left_panel.pos.0 + view.left_panel.size.0, view.left_panel.pos.1), color = palette.tones.0 :: call) :: call
    let mut id = 0
    while id < (actions.button_count :: :: call):
        let rect = layout.button_rect :: view, id :: call
        let fill = button_fill :: self, id, deck :: call
        fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = rect.pos, size = rect.size, color = fill :: call) :: call
        draw_label :: self, win, (render.TextLabel :: pos = (rect.pos.0 + 10, rect.pos.1 + 8), text = (actions.button_label :: id :: call), color = palette.tones.1.0 :: call) :: call
        id += 1

export fn draw_controls_only(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window):
    probe_line :: self, "draw_controls_only", "start" :: call
    let view = layout.for_window :: (arcana_desktop.window.size :: win :: call) :: call
    let palette = render.Palette :: surfaces = ((rgb :: 8, 13, 21 :: call), ((rgb :: 12, 18, 28 :: call), (rgb :: 13, 20, 31 :: call))), tones = ((rgb :: 32, 54, 75 :: call), ((rgb :: 233, 237, 242 :: call), (rgb :: 161, 173, 186 :: call))), accent = (rgb :: 113, 214, 225 :: call) :: call
    draw_controls :: self, win, (view, palette) :: call
    probe_line :: self, "draw_controls_only", "present" :: call
    arcana_graphics.canvas.present :: win :: call

fn draw_center_panel(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read payload: (layout.ViewLayout, render.Palette)):
    let view = payload.0
    let palette = payload.1
    let page_title = pages.title :: self.page_index :: call
    let status_tail = self.status_tail
    let body_lines = self.body_lines
    let inset = 18
    let inner_x = view.center_panel.pos.0 + inset
    let inner_y = view.center_panel.pos.1 + 18
    let inner_w = view.center_panel.size.0 - inset * 2
    let body_y = inner_y + 96
    let footer_y = view.center_panel.pos.1 + view.center_panel.size.1 - 58
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = view.center_panel.pos, size = view.center_panel.size, color = palette.surfaces.1.1 :: call) :: call
    stroke_line :: win, (arcana_graphics.types.LineSpec :: start = (view.center_panel.pos.0, view.center_panel.pos.1), end = (view.center_panel.pos.0 + view.center_panel.size.0, view.center_panel.pos.1), color = palette.tones.0 :: call) :: call
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (inner_x, inner_y), size = (inner_w, 72), color = (rgb :: 17, 27, 39 :: call) :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (inner_x + 16, inner_y + 14), text = page_title, color = palette.accent :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (inner_x + 16, inner_y + 38), text = status_tail, color = palette.tones.1.0 :: call) :: call
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (inner_x, body_y), size = (inner_w, view.center_panel.size.1 - 180), color = (rgb :: 18, 23, 33 :: call) :: call) :: call
    let body_text = std.text.join :: body_lines, "\n" :: call
    let text_spec = render.TextBlock :: pos = (inner_x + 16, body_y + 14), layout = (inner_w - 32, view.center_panel.size.1 - 208), text_and_color = (body_text, palette.tones.1.0) :: call
    probe_line :: self, "draw_center_panel", "before_text_stream" :: call
    render.sync_body_stream :: self, text_spec :: call
    probe_line :: self, "draw_center_panel", "after_text_stream" :: call
    let stream = match self.body_stream:
        Option.Some(value) => value
        Option.None => render.text_stream :: self, text_spec :: call
    paint_stream :: win, (stream, text_spec.pos) :: call
    probe_line :: self, "draw_center_panel", "after_paint_stream" :: call
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (inner_x, footer_y), size = (inner_w, 40), color = (rgb :: 17, 27, 39 :: call) :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (inner_x + 12, footer_y + 12), text = "Q / E page  |  W wake  |  N second  |  Esc exit", color = palette.tones.1.1 :: call) :: call

fn draw_session_panel(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window, read payload: (layout.ViewLayout, render.Palette)):
    let view = payload.0
    let palette = payload.1
    let x = view.right_panel.pos.0 + 18
    let w = view.right_panel.size.0 - 36
    let settings = arcana_desktop.window.settings :: win :: call
    let text = arcana_desktop.window.text_input_settings :: win :: call
    let cursor = settings.options.cursor
    let window_id_text = std.text.from_int :: (arcana_desktop.window.id :: win :: call).value :: call
    let current_monitor = arcana_desktop.monitor.current :: win :: call
    let primary_monitor = arcana_desktop.monitor.primary :: :: call
    let monitor_count = arcana_desktop.monitor.count :: :: call
    let scale_milli = arcana_desktop.window.scale_factor_milli :: win :: call
    let focused = arcana_desktop.window.focused :: win :: call
    let resized = arcana_desktop.window.resized :: win :: call
    let maximized = arcana_desktop.window.maximized :: win :: call
    let minimized = arcana_desktop.window.minimized :: win :: call
    let theme = arcana_desktop.window.theme :: win :: call
    let current_monitor_name = current_monitor.name
    let primary_monitor_name = primary_monitor.name
    let pointer_text = (point_text :: cursor.position :: call) + " / mouse " + self.last_mouse
    let text_input_text = (bool_name :: text.enabled :: call) + " / raw " + (std.text.from_int :: self.raw_key_events :: call)
    let redraw_wake_text = (std.text.from_int :: self.redraw_count :: call) + " / " + (std.text.from_int :: self.wake_count :: call) + "  close " + (std.text.from_int :: self.close_requests :: call)
    let wheel_move_text = (std.text.from_int :: self.mouse_wheel_y :: call) + " / " + (std.text.from_int :: self.mouse_events :: call)
    let raw_motion_text = std.text.from_int :: self.raw_motion_total :: call
    let raw_counts_text = ((std.text.from_int :: self.raw_button_events :: call) + " / " + (std.text.from_int :: self.raw_wheel_events :: call)) + " / " + (std.text.from_int :: self.raw_key_events :: call)
    let policy_text = (device_policy_name :: self.device_policy_code :: call) + " / " + self.last_device
    let second_text = second_window_summary :: self :: call
    let mut first_monitor = "-"
    if monitor_count > 0:
        first_monitor = (arcana_desktop.monitor.get :: 0 :: call).name
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = view.right_panel.pos, size = view.right_panel.size, color = palette.surfaces.1.1 :: call) :: call
    stroke_line :: win, (arcana_graphics.types.LineSpec :: start = (view.right_panel.pos.0, view.right_panel.pos.1), end = (view.right_panel.pos.0 + view.right_panel.size.0, view.right_panel.pos.1), color = palette.tones.0 :: call) :: call

    let card1_pos = (x, view.right_panel.pos.1 + 18)
    let card1_size = (w, 166)
    draw_card :: self, win, ((render.CardBlock :: pos = card1_pos, size = card1_size, title = "Window Bounds" :: call), palette) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card1_pos.1 + 40), text = ("title", settings.title), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card1_pos.1 + 58), text = ("id / size", (window_id_text + " / " + (size_text :: settings.bounds.size :: call))), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card1_pos.1 + 76), text = ("pos", (point_text :: settings.bounds.position :: call)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card1_pos.1 + 94), text = ("min", (bounds_text :: settings.bounds.min_size :: call)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card1_pos.1 + 112), text = ("max", (bounds_text :: settings.bounds.max_size :: call)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card1_pos.1 + 130), text = ("scale", (scale_text :: scale_milli :: call)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card1_pos.1 + 148), text = ("monitors", ((std.text.from_int :: monitor_count :: call) + "  first " + first_monitor)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call

    let card2_pos = (x, card1_pos.1 + card1_size.1 + 18)
    let card2_size = (w, 130)
    draw_card :: self, win, ((render.CardBlock :: pos = card2_pos, size = card2_size, title = "Window State" :: call), palette) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card2_pos.1 + 40), text = ("shown", ((bool_name :: settings.bounds.visible :: call) + " / focus " + (bool_name :: focused :: call) + " / resize " + (bool_name :: resized :: call))), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card2_pos.1 + 58), text = ("chrome", ((bool_name :: settings.options.style.decorated :: call) + " / resize " + (bool_name :: settings.options.style.resizable :: call))), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card2_pos.1 + 76), text = ("layer", ((bool_name :: settings.options.style.transparent :: call) + " / top " + (bool_name :: settings.options.state.topmost :: call))), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card2_pos.1 + 94), text = ("state", ((bool_name :: settings.options.state.fullscreen :: call) + " / " + (bool_name :: maximized :: call) + " / " + (bool_name :: minimized :: call))), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card2_pos.1 + 112), text = ("theme", ((theme_name :: theme :: call) + " / " + (theme_override_name :: settings.options.state.theme_override :: call))), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call

    let card3_pos = (x, card2_pos.1 + card2_size.1 + 18)
    let card3_size = (w, 148)
    draw_card :: self, win, ((render.CardBlock :: pos = card3_pos, size = card3_size, title = "Input And Text" :: call), palette) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card3_pos.1 + 40), text = ("cursor", (cursor_icon_name :: cursor.icon :: call)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card3_pos.1 + 58), text = ("cursor 2", ((bool_name :: cursor.visible :: call) + " / " + (grab_mode_name :: cursor.grab_mode :: call))), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card3_pos.1 + 76), text = ("pointer", pointer_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card3_pos.1 + 94), text = ("text", text_input_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card3_pos.1 + 112), text = ("comp", (composition_summary :: text.composition_area :: call)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card3_pos.1 + 130), text = ("key / text", (self.last_key + " / " + self.last_text)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call

    let card4_pos = (x, card3_pos.1 + card3_size.1 + 18)
    let card4_size = (w, view.right_panel.pos.1 + view.right_panel.size.1 - card4_pos.1 - 18)
    draw_card :: self, win, ((render.CardBlock :: pos = card4_pos, size = card4_size, title = "Session" :: call), palette) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 40), text = ("redraw/wake", redraw_wake_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 58), text = ("wheel / move", wheel_move_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 76), text = ("raw motion", raw_motion_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 94), text = ("raw b / w / k", raw_counts_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 112), text = ("policy / dev", policy_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 130), text = ("current / primary", (current_monitor_name + " / " + primary_monitor_name)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 148), text = ("second", second_text), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call
    draw_metric_line :: self, win, (render.MetricLine :: pos = (x + 12, card4_pos.1 + 166), text = ("event / clip", (self.last_event + " / " + self.last_clipboard)), colors = (palette.tones.1.1, palette.tones.1.0) :: call) :: call

export fn draw_main(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window):
    probe_line :: self, "draw_main", "start" :: call
    let view = layout.for_window :: (arcana_desktop.window.size :: win :: call) :: call
    let palette = render.Palette :: surfaces = ((rgb :: 8, 13, 21 :: call), ((rgb :: 12, 18, 28 :: call), (rgb :: 13, 20, 31 :: call))), tones = ((rgb :: 32, 54, 75 :: call), ((rgb :: 233, 237, 242 :: call), (rgb :: 161, 173, 186 :: call))), accent = (rgb :: 113, 214, 225 :: call) :: call
    arcana_graphics.canvas.fill :: win, palette.surfaces.0 :: call
    draw_header :: self, win, (view, palette) :: call
    probe_line :: self, "draw_main", "after_header" :: call
    draw_controls :: self, win, (view, palette) :: call
    probe_line :: self, "draw_main", "after_controls" :: call
    draw_center_panel :: self, win, (view, palette) :: call
    probe_line :: self, "draw_main", "after_center" :: call
    draw_session_panel :: self, win, (view, palette) :: call
    probe_line :: self, "draw_main", "after_session" :: call
    probe_line :: self, "draw_main", "present" :: call
    arcana_graphics.canvas.present :: win :: call

export fn draw_secondary(edit self: demo_types.Demo, edit win: arcana_desktop.types.Window):
    probe_line :: self, "draw_secondary", "start" :: call
    let background = rgb :: 16, 24, 36 :: call
    let panel = rgb :: 22, 35, 52 :: call
    let text = rgb :: 233, 237, 242 :: call
    let muted = rgb :: 161, 173, 186 :: call
    let accent = rgb :: 113, 214, 225 :: call
    arcana_graphics.canvas.fill :: win, background :: call
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (20, 20), size = (360, 54), color = panel :: call) :: call
    fill_rect :: win, (arcana_graphics.types.RectSpec :: pos = (20, 92), size = (360, 162), color = panel :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (36, 36), text = "Second Window", color = accent :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (36, 56), text = "live multi-window desktop session", color = muted :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (36, 112), text = "This window stays alive independently.", color = text :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (36, 134), text = "Use 2nd Vis and 2nd End on the main deck.", color = text :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (36, 156), text = "Clicks should not couple input to shutdown.", color = text :: call) :: call
    draw_label :: self, win, (render.TextLabel :: pos = (36, 178), text = "Close the last live window to end the session.", color = muted :: call) :: call
    probe_line :: self, "draw_secondary", "present" :: call
    arcana_graphics.canvas.present :: win :: call
