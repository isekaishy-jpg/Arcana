import actions
import arcana_desktop.monitor
import arcana_desktop.text_input
import arcana_desktop.types
import arcana_desktop.window
import arcana_graphics.canvas
import arcana_text.labels
import demo_types
import layout
import pages
import std.text

fn color_bg() -> Int:
    return arcana_graphics.canvas.rgb :: 16, 20, 28 :: call

fn color_panel() -> Int:
    return arcana_graphics.canvas.rgb :: 28, 34, 48 :: call

fn color_panel_alt() -> Int:
    return arcana_graphics.canvas.rgb :: 22, 26, 36 :: call

fn color_accent() -> Int:
    return arcana_graphics.canvas.rgb :: 83, 176, 255 :: call

fn color_warn() -> Int:
    return arcana_graphics.canvas.rgb :: 255, 182, 72 :: call

fn color_good() -> Int:
    return arcana_graphics.canvas.rgb :: 102, 214, 126 :: call

fn color_text() -> Int:
    return arcana_graphics.canvas.rgb :: 240, 244, 252 :: call

fn color_dim() -> Int:
    return arcana_graphics.canvas.rgb :: 160, 168, 184 :: call

fn bool_text(value: Bool) -> Str:
    if value:
        return "true"
    return "false"

fn theme_text(read value: arcana_desktop.types.WindowTheme) -> Str:
    return match value:
        arcana_desktop.types.WindowTheme.Light => "light"
        arcana_desktop.types.WindowTheme.Dark => "dark"
        _ => "unknown"

fn theme_override_text(read value: arcana_desktop.types.WindowThemeOverride) -> Str:
    return match value:
        arcana_desktop.types.WindowThemeOverride.Light => "light"
        arcana_desktop.types.WindowThemeOverride.Dark => "dark"
        _ => "system"

fn cursor_icon_text(read value: arcana_desktop.types.CursorIcon) -> Str:
    return match value:
        arcana_desktop.types.CursorIcon.Text => "text"
        arcana_desktop.types.CursorIcon.Crosshair => "crosshair"
        arcana_desktop.types.CursorIcon.Hand => "hand"
        arcana_desktop.types.CursorIcon.Move => "move"
        arcana_desktop.types.CursorIcon.Wait => "wait"
        arcana_desktop.types.CursorIcon.Help => "help"
        arcana_desktop.types.CursorIcon.NotAllowed => "not-allowed"
        arcana_desktop.types.CursorIcon.ResizeHorizontal => "resize-h"
        arcana_desktop.types.CursorIcon.ResizeVertical => "resize-v"
        arcana_desktop.types.CursorIcon.ResizeNwse => "resize-nwse"
        arcana_desktop.types.CursorIcon.ResizeNesw => "resize-nesw"
        _ => "default"

fn grab_text(read value: arcana_desktop.types.CursorGrabMode) -> Str:
    return match value:
        arcana_desktop.types.CursorGrabMode.Confined => "confined"
        arcana_desktop.types.CursorGrabMode.Locked => "locked"
        _ => "free"

fn point_text(point: (Int, Int)) -> Str:
    return "(" + (std.text.from_int :: point.0 :: call) + ", " + (std.text.from_int :: point.1 :: call) + ")"

fn page_banner_text(read self: demo_types.Demo) -> Str:
    let mut text = "page " + (std.text.from_int :: (self.page_index + 1) :: call)
    text = text + "/" + (std.text.from_int :: (pages.count :: :: call) :: call)
    text = text + " :: " + (pages.title :: self.page_index :: call)
    return text

fn monitor_text(read info: arcana_desktop.types.MonitorInfo) -> Str:
    return "#" + (std.text.from_int :: info.index :: call) + " " + info.name + " " + (render.point_text :: info.size :: call)

fn label_spec(pos: (Int, Int), text: Str, color: Int) -> arcana_text.types.LabelSpec:
    return arcana_text.types.LabelSpec :: pos = pos, text = text, color = color :: call

fn rect_spec(pos: (Int, Int), size: (Int, Int), color: Int) -> arcana_graphics.types.RectSpec:
    return arcana_graphics.types.RectSpec :: pos = pos, size = size, color = color :: call

fn line_spec(start: (Int, Int), end: (Int, Int), color: Int) -> arcana_graphics.types.LineSpec:
    return arcana_graphics.types.LineSpec :: start = start, end = end, color = color :: call

fn circle_spec(center: (Int, Int), radius: Int, color: Int) -> arcana_graphics.types.CircleFillSpec:
    return arcana_graphics.types.CircleFillSpec :: center = center, radius = radius, color = color :: call

fn draw_label(edit win: std.window.Window, read spec: arcana_text.types.LabelSpec):
    arcana_text.labels.label :: win, spec :: call

fn draw_rect(edit win: std.window.Window, read spec: arcana_graphics.types.RectSpec):
    arcana_graphics.canvas.rect :: win, spec :: call

fn draw_line(edit win: std.window.Window, read spec: arcana_graphics.types.LineSpec):
    arcana_graphics.canvas.line :: win, spec :: call

fn draw_circle(edit win: std.window.Window, read spec: arcana_graphics.types.CircleFillSpec):
    arcana_graphics.canvas.circle_fill :: win, spec :: call

fn draw_text_line(edit win: std.window.Window, x: Int, y: Int, text: Str, color: Int):
    std.canvas.label :: win, x, y :: call
        text = text
        color = color

fn button_hovered(read self: demo_types.Demo, read view: layout.ViewLayout, id: Int) -> Bool:
    let _ = view
    return self.mouse_inside and self.hover_button_id == id

fn button_color(read self: demo_types.Demo, read view: layout.ViewLayout, id: Int) -> Int:
    if render.button_hovered :: self, view, id :: call:
        return render.color_accent :: :: call
    return render.color_panel_alt :: :: call

fn wake_badge_color(read self: demo_types.Demo) -> Int:
    if self.pending_wake:
        return render.color_warn :: :: call
    return render.color_good :: :: call

fn draw_button_in_view(read self: demo_types.Demo, edit win: std.window.Window, read view: layout.ViewLayout, id: Int):
    let rect = actions.button_rect :: view, id :: call
    let label = actions.button_label :: id :: call
    let bg = render.button_color :: self, view, id :: call
    let text_color = render.color_text :: :: call
    render.draw_rect :: win, (render.rect_spec :: rect.pos, rect.size, bg :: call) :: call
    let stroke = render.color_dim :: :: call
    render.draw_line :: win, (render.line_spec :: rect.pos, (rect.pos.0 + rect.size.0, rect.pos.1), stroke :: call) :: call
    render.draw_line :: win, (render.line_spec :: (rect.pos.0, rect.pos.1 + rect.size.1), (rect.pos.0 + rect.size.0, rect.pos.1 + rect.size.1), stroke :: call) :: call
    let measure = arcana_text.labels.measure :: label :: call
    let text_pos = (rect.pos.0 + (rect.size.0 - measure.0) / 2, rect.pos.1 + 7)
    render.draw_label :: win, (render.label_spec :: text_pos, label, text_color :: call) :: call

fn draw_page_text_in_view(edit win: std.window.Window, read view: layout.ViewLayout, body: Str, start_line: Int):
    let lines = std.text.split_lines :: body :: call
    let mut current_index = 0
    let mut y = view.center_panel.pos.1 + 34
    let bottom = view.center_panel.pos.1 + view.center_panel.size.1 - 56
    let x = view.center_panel.pos.0 + 24
    for line in lines:
        if current_index >= start_line:
            if y > bottom:
                return
            render.draw_label :: win, (render.label_spec :: (x, y), line, (render.color_text :: :: call) :: call) :: call
            y += 22
        current_index += 1

fn draw_status_in_view(read self: demo_types.Demo, edit win: std.window.Window, read view: layout.ViewLayout):
    let settings = arcana_desktop.window.settings :: win :: call
    let text_settings = arcana_desktop.text_input.settings :: win :: call
    let alive = arcana_desktop.window.alive :: win :: call
    let focused = arcana_desktop.window.focused :: win :: call
    let fullscreen = arcana_desktop.window.fullscreen :: win :: call
    let minimized = arcana_desktop.window.minimized :: win :: call
    let maximized = arcana_desktop.window.maximized :: win :: call
    let resized = arcana_desktop.window.resized :: win :: call
    let theme = arcana_desktop.window.theme :: win :: call
    let scale = arcana_desktop.window.scale_factor_milli :: win :: call
    let current_monitor = arcana_desktop.monitor.current :: win :: call
    let primary_monitor = arcana_desktop.monitor.primary :: :: call
    let count = arcana_desktop.monitor.count :: :: call
    let line_title = "title: " + settings.title
    let line_alive = "alive: " + (render.bool_text :: alive :: call) + "  focused: " + (render.bool_text :: focused :: call)
    let line_pos = "pos: " + (render.point_text :: settings.bounds.position :: call) + "  size: " + (render.point_text :: settings.bounds.size :: call)
    let line_limits = "min/max: " + (render.point_text :: settings.bounds.min_size :: call) + " / " + (render.point_text :: settings.bounds.max_size :: call)
    let line_shell = "fullscreen: " + (render.bool_text :: fullscreen :: call) + "  minimized: " + (render.bool_text :: minimized :: call)
    let line_state = "maximized: " + (render.bool_text :: maximized :: call) + "  resized: " + (render.bool_text :: resized :: call)
    let line_theme = "theme: " + (render.theme_text :: theme :: call) + "  override: " + (render.theme_override_text :: settings.options.state.theme_override :: call)
    let line_scale = "scale: " + (std.text.from_int :: scale :: call) + "  monitors: " + (std.text.from_int :: count :: call)
    let line_cursor = "cursor: " + (render.cursor_icon_text :: settings.options.cursor.icon :: call) + " / " + (render.grab_text :: settings.options.cursor.grab_mode :: call)
    let line_cursor_pos = "cursor pos: " + (render.point_text :: settings.options.cursor.position :: call) + "  visible: " + (render.bool_text :: settings.options.cursor.visible :: call)
    let line_text = "text enabled: " + (render.bool_text :: text_settings.enabled :: call) + "  comp active: " + (render.bool_text :: text_settings.composition_area.active :: call)
    let line_area = "comp area: " + (render.point_text :: text_settings.composition_area.position :: call) + " size " + (render.point_text :: text_settings.composition_area.size :: call)
    let line_current_monitor = "current monitor: " + (render.monitor_text :: current_monitor :: call)
    let line_primary_monitor = "primary monitor: " + (render.monitor_text :: primary_monitor :: call)
    let line_demo = "controls/pages: " + (std.text.from_int :: self.checksum :: call) + " / " + (std.text.from_int :: self.line_count :: call)
    let line_counts = "events: key " + (std.text.from_int :: self.key_events :: call) + "  mouse " + (std.text.from_int :: self.mouse_events :: call) + "  text " + (std.text.from_int :: self.text_events :: call)
    let line_wakes = "wake " + (std.text.from_int :: self.wake_count :: call) + "  close " + (std.text.from_int :: self.close_requests :: call) + "  redraw " + (std.text.from_int :: self.redraw_count :: call)
    let line_motion = "raw motion total: " + (std.text.from_int :: self.raw_motion_total :: call) + "  ecs: " + (std.text.from_int :: self.adapter_total :: call)
    let line_windows = "second window: id " + (std.text.from_int :: self.second_window_id :: call) + "  seen: " + (render.bool_text :: self.second_window_seen :: call)
    let line_status = "status: " + self.status_head
    let line_last_event = "last event: " + self.last_event
    let line_last_key = "last key: " + self.last_key
    let line_last_mouse = "last mouse: " + self.last_mouse
    let line_last_text = "last text: " + self.last_text
    let line_last_comp = "last comp: " + self.last_comp
    let line_last_drop = "last drop: " + self.last_drop
    let line_clipboard = "clipboard: " + self.last_clipboard
    let mut y = view.right_panel.pos.1 + 34
    let right_x = view.right_panel.pos.0 + 16
    let bottom = view.right_panel.pos.1 + view.right_panel.size.1 - 32
    render.draw_text_line :: win, right_x, y :: call
        text = "Live State"
        color = (render.color_accent :: :: call)
    y += 28
    render.draw_text_line :: win, right_x, y :: call
        text = line_title
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_alive
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_pos
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_limits
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_shell
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_state
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_theme
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_scale
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_cursor
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_cursor_pos
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_text
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_area
        color = (render.color_text :: :: call)
    y += 28
    render.draw_text_line :: win, right_x, y :: call
        text = line_current_monitor
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_primary_monitor
        color = (render.color_text :: :: call)
    y += 28
    render.draw_text_line :: win, right_x, y :: call
        text = line_demo
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_counts
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_wakes
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_motion
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_windows
        color = (render.color_text :: :: call)
    y += 28
    render.draw_text_line :: win, right_x, y :: call
        text = line_status
        color = (render.color_warn :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = self.status_tail
        color = (render.color_text :: :: call)
    y += 22
    render.draw_text_line :: win, right_x, y :: call
        text = line_last_event
        color = (render.color_dim :: :: call)
    y += 20
    render.draw_text_line :: win, right_x, y :: call
        text = line_last_key
        color = (render.color_dim :: :: call)
    y += 20
    render.draw_text_line :: win, right_x, y :: call
        text = line_last_mouse
        color = (render.color_dim :: :: call)
    y += 20
    render.draw_text_line :: win, right_x, y :: call
        text = line_last_text
        color = (render.color_dim :: :: call)
    y += 20
    render.draw_text_line :: win, right_x, y :: call
        text = line_last_comp
        color = (render.color_dim :: :: call)
    y += 20
    render.draw_text_line :: win, right_x, y :: call
        text = line_last_drop
        color = (render.color_dim :: :: call)
    y += 20
    render.draw_text_line :: win, right_x, y :: call
        text = line_clipboard
        color = (render.color_dim :: :: call)
    let _ = bottom

export fn draw_main(read self: demo_types.Demo, edit win: std.window.Window):
    let mut win = win
    let size = arcana_desktop.window.size :: win :: call
    let view = layout.for_window :: size :: call
    arcana_graphics.canvas.fill :: win, (render.color_bg :: :: call) :: call
    render.draw_rect :: win, (render.rect_spec :: (0, 0), (view.window_size.0, view.header_height), (render.color_panel :: :: call) :: call) :: call
    render.draw_rect :: win, (render.rect_spec :: view.left_panel.pos, view.left_panel.size, (render.color_panel :: :: call) :: call) :: call
    render.draw_rect :: win, (render.rect_spec :: view.center_panel.pos, view.center_panel.size, (render.color_panel_alt :: :: call) :: call) :: call
    render.draw_rect :: win, (render.rect_spec :: view.right_panel.pos, view.right_panel.size, (render.color_panel :: :: call) :: call) :: call
    render.draw_line :: win, (render.line_spec :: (view.center_panel.pos.0, view.center_panel.pos.1), (view.center_panel.pos.0, view.center_panel.pos.1 + view.center_panel.size.1), (render.color_dim :: :: call) :: call) :: call
    render.draw_line :: win, (render.line_spec :: (view.right_panel.pos.0, view.right_panel.pos.1), (view.right_panel.pos.0, view.right_panel.pos.1 + view.right_panel.size.1), (render.color_dim :: :: call) :: call) :: call
    render.draw_label :: win, (render.label_spec :: (20, 18), "Arcana Desktop Proof", (render.color_text :: :: call) :: call) :: call
    let page_banner = render.page_banner_text :: self :: call
    std.canvas.label :: win :: call
        x = 20
        y = 44
        text = page_banner
        color = (render.color_accent :: :: call)
    render.draw_label :: win, (render.label_spec :: (view.center_panel.pos.0 + 4, 18), "Interactive Guide", (render.color_text :: :: call) :: call) :: call
    render.draw_label :: win, (render.label_spec :: (view.center_panel.pos.0 + 4, 44), "Click buttons, type, drag, drop files, and use the close button.", (render.color_dim :: :: call) :: call) :: call
    render.draw_circle :: win, (render.circle_spec :: (view.window_size.0 - 42, 42), 10, (render.wake_badge_color :: self :: call) :: call) :: call
    let total_buttons = actions.button_count :: :: call
    let mut id = 0
    while id < total_buttons:
        render.draw_button_in_view :: self, win :: call
            view = view
            id = id
        id += 1
    render.draw_label :: win, (render.label_spec :: (view.center_panel.pos.0 + 24, view.center_panel.pos.1 + 4), (pages.title :: self.page_index :: call), (render.color_accent :: :: call) :: call) :: call
    render.draw_page_text_in_view :: win, view :: call
        body = (pages.body :: self.page_index :: call)
        start_line = self.page_scroll
    render.draw_status_in_view :: self, win, view :: call
    let footer_y = view.left_panel.pos.1 + view.left_panel.size.1 - 46
    render.draw_label :: win, (render.label_spec :: (view.left_panel.pos.0 + 18, footer_y), "Shortcuts: Esc exit | Q/E page | F1/F2/F3 loop | O profile | W wake | N second win", (render.color_dim :: :: call) :: call) :: call
    if self.smoke_mode:
        render.draw_label :: win, (render.label_spec :: (view.left_panel.pos.0 + 18, footer_y + 22), "Smoke mode is active: the packaged test runs this path with --smoke.", (render.color_warn :: :: call) :: call) :: call
    else:
        render.draw_label :: win, (render.label_spec :: (view.left_panel.pos.0 + 18, footer_y + 22), "Manual checks: X should close, file drop should update state, IME should surface composition.", (render.color_dim :: :: call) :: call) :: call
    arcana_graphics.canvas.present :: win :: call

export fn draw_secondary(read self: demo_types.Demo, edit win: std.window.Window):
    let size = arcana_desktop.window.size :: win :: call
    arcana_graphics.canvas.fill :: win, (render.color_panel_alt :: :: call) :: call
    render.draw_rect :: win, (render.rect_spec :: (0, 0), size, (render.color_panel :: :: call) :: call) :: call
    std.canvas.label :: win :: call
        x = 20
        y = 20
        text = "Arcana Desktop Proof :: Second Window"
        color = (render.color_accent :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 50
        text = "This window is opened through arcana_desktop.app.open_window."
        color = (render.color_text :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 78
        text = "Resize or close it to verify multi-window shell behavior."
        color = (render.color_text :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 118
        text = ("page: " + (pages.title :: self.page_index :: call))
        color = (render.color_dim :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 142
        text = ("last event: " + self.last_event)
        color = (render.color_dim :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 166
        text = ("last mouse: " + self.last_mouse)
        color = (render.color_dim :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 190
        text = ("last key: " + self.last_key)
        color = (render.color_dim :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 214
        text = ("status: " + self.status_head)
        color = (render.color_warn :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = 238
        text = self.status_tail
        color = (render.color_text :: :: call)
    std.canvas.label :: win :: call
        x = 20
        y = size.1 - 36
        text = "Close this window with the shell close button or keep interacting with the main showcase."
        color = (render.color_dim :: :: call)
    arcana_graphics.canvas.present :: win :: call
