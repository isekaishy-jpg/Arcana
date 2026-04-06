import actions
import arcana_desktop.app
import arcana_desktop.clipboard
import arcana_desktop.events
import arcana_desktop.input
import arcana_desktop.monitor
import arcana_desktop.text_input
import arcana_desktop.types
import arcana_desktop.window
import arcana_text.raster
import demo_types
import layout
import pages
import render
import std.args
import std.bytes
import std.collections.list
import std.fs
import std.io
import std.option
import std.result
import std.text
import std.time
use std.option.Option
use std.result.Result

fn probe_enabled(read self: demo_types.Demo) -> Bool:
    return self.probe_mode

fn probe_log_path() -> Str:
    return "scratch/desktop_probe.log"

fn probe_log_append(line: Str):
    let _ = std.fs.mkdir_all :: "scratch" :: call
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
    let _ = std.fs.mkdir_all :: "scratch" :: call
    let _ = std.fs.write_text :: (probe_log_path :: :: call), "" :: call

fn probe_line(read self: demo_types.Demo, head: Str, tail: Str):
    if not (probe_enabled :: self :: call):
        return
    probe_log_append :: ("[desktop-proof] " + head + " :: " + tail) :: call

fn bool_code(value: Bool) -> Int:
    if value:
        return 1
    return 0

fn has_flag(flag: Str) -> Bool:
    let total = std.args.count :: :: call
    let mut index = 0
    while index < total:
        if (std.args.get :: index :: call) == flag:
            return true
        index += 1
    return false

fn default_demo(smoke_mode: Bool) -> demo_types.Demo:
    let mut demo = demo_types.Demo :: smoke_mode = smoke_mode, ui_smoke_mode = false, exercise_second_window = false :: call
    demo.probe_mode = false
    demo.probe_measure_count = 0
    demo.probe_label_count = 0
    demo.text_renderer = arcana_text.raster.default_renderer :: :: call
    demo.dirty = true
    demo.title_dirty = false
    demo.smoke_printed = false
    demo.page_index = 0
    demo.redraw_count = 0
    demo.wake_count = 0
    demo.close_requests = 0
    demo.key_events = 0
    demo.mouse_events = 0
    demo.text_events = 0
    demo.raw_motion_total = 0
    demo.raw_button_events = 0
    demo.raw_wheel_events = 0
    demo.raw_key_events = 0
    demo.mouse_wheel_y = 0
    demo.device_policy_code = 1
    demo.second_window_id = -1
    demo.second_window_seen = false
    demo.second_window_dirty = false
    demo.second_window_visible = false
    demo.second_window_alive = false
    demo.status_head = "starting"
    demo.status_tail = "desktop proof startup"
    demo.last_event = "-"
    demo.last_key = "-"
    demo.last_mouse = "-"
    demo.last_text = "-"
    demo.last_clipboard = "-"
    demo.last_device = "-"
    demo.last_monitor = "-"
    demo.last_window = "-"
    demo.pending_wake = false
    demo.mouse_pos = (0, 0)
    demo.mouse_inside = false
    demo.hover_button_id = -1
    demo.controls_dirty = false
    demo.telemetry_dirty = false
    demo.next_telemetry_redraw_ms = 0
    demo.move_size_cycle = 0
    demo.clamp_cycle = 0
    demo.preset_cycle = 0
    demo.body_page_index = -1
    demo.body_wrap_width = 0
    demo.body_lines = std.collections.list.new[Str] :: :: call
    demo.body_stream_ready = false
    demo.body_stream_page_index = -1
    demo.body_stream_layout = (0, 0)
    demo.body_stream_color = 0
    demo.body_stream = Option.None[arcana_text.raster.GlyphDrawStream] :: :: call
    return demo

fn current_title(read self: demo_types.Demo) -> Str:
    return "Arcana Desktop Proof :: " + (pages.title :: self.page_index :: call)

fn mark_dirty(edit self: demo_types.Demo):
    self.dirty = true

fn mark_controls_dirty(edit self: demo_types.Demo):
    self.controls_dirty = true

fn mark_telemetry_dirty(edit self: demo_types.Demo):
    self.telemetry_dirty = true

fn set_status(edit self: demo_types.Demo, head: Str, tail: Str):
    self.status_head = head
    self.status_tail = tail
    mark_dirty :: self :: call

fn maybe_schedule_telemetry_redraw(edit self: demo_types.Demo):
    if not self.telemetry_dirty or self.ui_smoke_mode:
        return
    let now = (std.time.monotonic_now_ms :: :: call).value
    if now < self.next_telemetry_redraw_ms:
        return
    self.telemetry_dirty = false
    self.next_telemetry_redraw_ms = now + 24
    mark_dirty :: self :: call

fn sync_body_lines(edit self: demo_types.Demo, read win: arcana_desktop.types.Window):
    let view = layout.for_window :: (arcana_desktop.window.size :: win :: call) :: call
    let wrap_width = view.center_panel.size.0 - 68
    if self.body_page_index == self.page_index and self.body_wrap_width == wrap_width:
        return
    probe_line :: self, "sync_body_lines", ("page=" + (std.text.from_int :: self.page_index :: call) + " width=" + (std.text.from_int :: wrap_width :: call)) :: call
    self.body_page_index = self.page_index
    self.body_wrap_width = wrap_width
    self.body_lines = render.wrapped_lines :: self, (pages.body :: self.page_index :: call), wrap_width :: call
    self.body_stream_ready = false
    probe_line :: self, "sync_body_lines_done", ("lines=" + (std.text.from_int :: (self.body_lines :: :: len) :: call)) :: call

fn device_policy_code(read value: arcana_desktop.types.DeviceEvents) -> Int:
    return match value:
        arcana_desktop.types.DeviceEvents.Never => 0
        arcana_desktop.types.DeviceEvents.Always => 2
        _ => 1

fn device_policy_name(code: Int) -> Str:
    if code == 0:
        return "never"
    if code == 2:
        return "always"
    return "focused"

fn next_device_policy(read value: arcana_desktop.types.DeviceEvents) -> arcana_desktop.types.DeviceEvents:
    return match value:
        arcana_desktop.types.DeviceEvents.Never => arcana_desktop.types.DeviceEvents.WhenFocused :: :: call
        arcana_desktop.types.DeviceEvents.WhenFocused => arcana_desktop.types.DeviceEvents.Always :: :: call
        _ => arcana_desktop.types.DeviceEvents.Never :: :: call

fn sync_device_policy(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    self.device_policy_code = device_policy_code :: (arcana_desktop.app.device_events :: cx :: call) :: call

fn cursor_icon_name(read value: arcana_desktop.types.CursorIcon) -> Str:
    return match value:
        arcana_desktop.types.CursorIcon.Text => "text"
        arcana_desktop.types.CursorIcon.Crosshair => "crosshair"
        arcana_desktop.types.CursorIcon.Hand => "hand"
        arcana_desktop.types.CursorIcon.Move => "move"
        arcana_desktop.types.CursorIcon.Wait => "wait"
        arcana_desktop.types.CursorIcon.Help => "help"
        _ => "default"

fn next_cursor_icon(read value: arcana_desktop.types.CursorIcon) -> arcana_desktop.types.CursorIcon:
    return match value:
        arcana_desktop.types.CursorIcon.Default => arcana_desktop.types.CursorIcon.Text :: :: call
        arcana_desktop.types.CursorIcon.Text => arcana_desktop.types.CursorIcon.Hand :: :: call
        arcana_desktop.types.CursorIcon.Hand => arcana_desktop.types.CursorIcon.Crosshair :: :: call
        arcana_desktop.types.CursorIcon.Crosshair => arcana_desktop.types.CursorIcon.Wait :: :: call
        arcana_desktop.types.CursorIcon.Wait => arcana_desktop.types.CursorIcon.Help :: :: call
        _ => arcana_desktop.types.CursorIcon.Default :: :: call

fn grab_mode_name(read value: arcana_desktop.types.CursorGrabMode) -> Str:
    return match value:
        arcana_desktop.types.CursorGrabMode.Confined => "confined"
        arcana_desktop.types.CursorGrabMode.Locked => "locked"
        _ => "free"

fn next_grab_mode(read value: arcana_desktop.types.CursorGrabMode) -> arcana_desktop.types.CursorGrabMode:
    return match value:
        arcana_desktop.types.CursorGrabMode.Free => arcana_desktop.types.CursorGrabMode.Confined :: :: call
        arcana_desktop.types.CursorGrabMode.Confined => arcana_desktop.types.CursorGrabMode.Locked :: :: call
        _ => arcana_desktop.types.CursorGrabMode.Free :: :: call

fn theme_override_name(read value: arcana_desktop.types.WindowThemeOverride) -> Str:
    return match value:
        arcana_desktop.types.WindowThemeOverride.Light => "light"
        arcana_desktop.types.WindowThemeOverride.Dark => "dark"
        _ => "system"

fn next_theme_override(read value: arcana_desktop.types.WindowThemeOverride) -> arcana_desktop.types.WindowThemeOverride:
    return match value:
        arcana_desktop.types.WindowThemeOverride.System => arcana_desktop.types.WindowThemeOverride.Dark :: :: call
        arcana_desktop.types.WindowThemeOverride.Dark => arcana_desktop.types.WindowThemeOverride.Light :: :: call
        _ => arcana_desktop.types.WindowThemeOverride.System :: :: call

fn device_id_text(read value: Option[arcana_desktop.types.DeviceId]) -> Str:
    return match value:
        Option.Some(id) => "d" + (std.text.from_int :: id.value :: call)
        Option.None => "global"

fn main_window(edit cx: arcana_desktop.types.AppContext) -> Result[arcana_desktop.types.Window, Str]:
    return arcana_desktop.app.main_window :: cx :: call

fn sync_main_title(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let _ = cx
    self.title_dirty = true
    return

fn flush_main_title(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    if not self.title_dirty:
        return
    let win = main_window :: cx :: call
    return match win:
        Result.Ok(value) => flush_main_title_apply :: self, value :: call
        Result.Err(_) => flush_main_title_missing :: self :: call

fn flush_main_title_apply(edit self: demo_types.Demo, take value: arcana_desktop.types.Window):
    let mut win = value
    arcana_desktop.window.set_title :: win, (current_title :: self :: call) :: call
    self.title_dirty = false

fn flush_main_title_missing(edit self: demo_types.Demo):
    self.title_dirty = false

fn request_main_redraw(edit cx: arcana_desktop.types.AppContext):
    let win = main_window :: cx :: call
    return match win:
        Result.Ok(value) => request_main_redraw_ready :: value :: call
        Result.Err(_) => 0

fn request_main_redraw_ready(take value: arcana_desktop.types.Window):
    let mut win = value
    arcana_desktop.window.request_redraw :: win :: call

fn action_window(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, head: Str) -> Option[arcana_desktop.types.Window]:
    let found = main_window :: cx :: call
    return match found:
        Result.Ok(value) => Option.Some[arcana_desktop.types.Window] :: value :: call
        Result.Err(err) => action_window_missing :: self, head, err :: call

fn action_window_missing(edit self: demo_types.Demo, head: Str, err: Str) -> Option[arcana_desktop.types.Window]:
    set_status :: self, head, err :: call
    return Option.None[arcana_desktop.types.Window] :: :: call

fn refresh_main_monitor(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = main_window :: cx :: call
    return match found:
        Result.Ok(value) => refresh_main_monitor_ready :: self, value :: call
        Result.Err(_) => 0

fn refresh_main_monitor_ready(edit self: demo_types.Demo, read value: arcana_desktop.types.Window):
    self.last_monitor = (arcana_desktop.monitor.current :: value :: call).name

fn refresh_second_window_state(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = second_window :: self, cx :: call
    return match found:
        Option.Some(win) => refresh_second_window_state_ready :: self, win :: call
        Option.None => refresh_second_window_state_missing :: self :: call

fn refresh_second_window_state_ready(edit self: demo_types.Demo, read win: arcana_desktop.types.Window):
    self.second_window_alive = arcana_desktop.window.alive :: win :: call
    self.second_window_visible = arcana_desktop.window.visible :: win :: call

fn clear_second_window_state(edit self: demo_types.Demo):
    self.second_window_id = -1
    self.second_window_seen = false
    self.second_window_dirty = false
    self.second_window_visible = false
    self.second_window_alive = false

fn refresh_second_window_state_missing(edit self: demo_types.Demo):
    clear_second_window_state :: self :: call

fn set_page(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, next: Int) -> arcana_desktop.types.ControlFlow:
    self.page_index = next
    sync_main_title :: self, cx :: call
    self.status_head = "page"
    self.status_tail = pages.title :: self.page_index :: call
    if self.ui_smoke_mode:
        std.io.print_line[Str] :: ("page=" + self.status_tail) :: call
        std.io.flush_stdout :: :: call
        arcana_desktop.app.request_exit :: cx, 0 :: call
        return arcana_desktop.types.ControlFlow.Wait :: :: call
    mark_dirty :: self :: call
    return cx.control.control_flow

fn cycle_theme(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "theme" :: call
    return match found:
        Option.Some(win) => cycle_theme_ready :: self, win :: call
        Option.None => 0

fn cycle_theme_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let next = next_theme_override :: (arcana_desktop.window.theme_override :: win :: call) :: call
    arcana_desktop.window.set_theme_override :: win, next :: call
    set_status :: self, "theme", ("override " + (theme_override_name :: next :: call)) :: call

fn toggle_text_input(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "text input" :: call
    return match found:
        Option.Some(win) => toggle_text_input_ready :: self, win :: call
        Option.None => 0

fn toggle_text_input_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.text_input.enabled :: win :: call)
    arcana_desktop.text_input.set_enabled :: win, enabled :: call
    if enabled:
        set_status :: self, "text input", "enabled" :: call
    else:
        arcana_desktop.text_input.clear_composition_area :: win :: call
        set_status :: self, "text input", "disabled" :: call

fn cycle_cursor_icon(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "cursor" :: call
    return match found:
        Option.Some(win) => cycle_cursor_icon_ready :: self, win :: call
        Option.None => 0

fn cycle_cursor_icon_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let next = next_cursor_icon :: (arcana_desktop.window.cursor_icon :: win :: call) :: call
    arcana_desktop.window.set_cursor_icon :: win, next :: call
    set_status :: self, "cursor", ("icon " + (cursor_icon_name :: next :: call)) :: call

fn toggle_cursor_visible(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "cursor" :: call
    return match found:
        Option.Some(win) => toggle_cursor_visible_ready :: self, win :: call
        Option.None => 0

fn toggle_cursor_visible_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.cursor_visible :: win :: call)
    arcana_desktop.window.set_cursor_visible :: win, enabled :: call
    if enabled:
        set_status :: self, "cursor", "visible" :: call
    else:
        set_status :: self, "cursor", "hidden" :: call

fn center_cursor(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "cursor" :: call
    return match found:
        Option.Some(win) => center_cursor_ready :: self, win :: call
        Option.None => 0

fn center_cursor_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let size = arcana_desktop.window.size :: win :: call
    let center = (size.0 / 2, size.1 / 2)
    arcana_desktop.window.set_cursor_position :: win, center.0, center.1 :: call
    set_status :: self, "cursor", "centered" :: call

fn toggle_topmost(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "topmost" :: call
    return match found:
        Option.Some(win) => toggle_topmost_ready :: self, win :: call
        Option.None => 0

fn toggle_topmost_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.topmost :: win :: call)
    arcana_desktop.window.set_topmost :: win, enabled :: call
    if enabled:
        set_status :: self, "topmost", "enabled" :: call
    else:
        set_status :: self, "topmost", "disabled" :: call

fn toggle_decorated(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "decorated" :: call
    return match found:
        Option.Some(win) => toggle_decorated_ready :: self, win :: call
        Option.None => 0

fn toggle_decorated_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.decorated :: win :: call)
    arcana_desktop.window.set_decorated :: win, enabled :: call
    if enabled:
        set_status :: self, "decorated", "enabled" :: call
    else:
        set_status :: self, "decorated", "disabled" :: call

fn toggle_resizable(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "resizable" :: call
    return match found:
        Option.Some(win) => toggle_resizable_ready :: self, win :: call
        Option.None => 0

fn toggle_resizable_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.resizable :: win :: call)
    arcana_desktop.window.set_resizable :: win, enabled :: call
    if enabled:
        set_status :: self, "resizable", "enabled" :: call
    else:
        set_status :: self, "resizable", "disabled" :: call

fn request_attention(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "attention" :: call
    return match found:
        Option.Some(win) => request_attention_ready :: self, win :: call
        Option.None => 0

fn request_attention_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    arcana_desktop.window.request_attention :: win, true :: call
    set_status :: self, "attention", "requested" :: call

fn toggle_fullscreen(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "fullscreen" :: call
    return match found:
        Option.Some(win) => toggle_fullscreen_ready :: self, win :: call
        Option.None => 0

fn toggle_fullscreen_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.fullscreen :: win :: call)
    if enabled and (arcana_desktop.window.maximized :: win :: call):
        arcana_desktop.window.set_maximized :: win, false :: call
    arcana_desktop.window.set_fullscreen :: win, enabled :: call
    if enabled:
        set_status :: self, "fullscreen", "enabled" :: call
    else:
        set_status :: self, "fullscreen", "disabled" :: call

fn toggle_maximized(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "maximize" :: call
    return match found:
        Option.Some(win) => toggle_maximized_ready :: self, win :: call
        Option.None => 0

fn toggle_maximized_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.maximized :: win :: call)
    if enabled and (arcana_desktop.window.fullscreen :: win :: call):
        arcana_desktop.window.set_fullscreen :: win, false :: call
    arcana_desktop.window.set_maximized :: win, enabled :: call
    if enabled:
        set_status :: self, "maximize", "enabled" :: call
    else:
        set_status :: self, "maximize", "disabled" :: call

fn minimize_window(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "minimize" :: call
    return match found:
        Option.Some(win) => minimize_window_ready :: self, win :: call
        Option.None => 0

fn minimize_window_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    arcana_desktop.window.set_minimized :: win, true :: call
    set_status :: self, "minimize", "requested" :: call

fn toggle_transparent(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "transparent" :: call
    return match found:
        Option.Some(win) => toggle_transparent_ready :: self, win :: call
        Option.None => 0

fn toggle_transparent_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.transparent :: win :: call)
    arcana_desktop.window.set_transparent :: win, enabled :: call
    if enabled:
        set_status :: self, "transparent", "enabled" :: call
    else:
        set_status :: self, "transparent", "disabled" :: call

fn cycle_cursor_grab(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "cursor grab" :: call
    return match found:
        Option.Some(win) => cycle_cursor_grab_ready :: self, win :: call
        Option.None => 0

fn cycle_cursor_grab_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let next = next_grab_mode :: (arcana_desktop.window.cursor_grab_mode :: win :: call) :: call
    arcana_desktop.window.set_cursor_grab_mode :: win, next :: call
    set_status :: self, "cursor grab", (grab_mode_name :: next :: call) :: call

fn cycle_move_size(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "move+size" :: call
    return match found:
        Option.Some(win) => cycle_move_size_ready :: self, win :: call
        Option.None => 0

fn cycle_move_size_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    self.move_size_cycle += 1
    if self.move_size_cycle >= 3:
        self.move_size_cycle = 0
    if self.move_size_cycle == 0:
        arcana_desktop.window.set_position :: win, 64, 52 :: call
        arcana_desktop.window.set_size :: win, 1280, 760 :: call
    if self.move_size_cycle == 1:
        arcana_desktop.window.set_position :: win, 120, 80 :: call
        arcana_desktop.window.set_size :: win, 1120, 700 :: call
    if self.move_size_cycle == 2:
        arcana_desktop.window.set_position :: win, 220, 140 :: call
        arcana_desktop.window.set_size :: win, 980, 660 :: call
    let cycle_text = "cycle " + (std.text.from_int :: self.move_size_cycle :: call)
    set_status :: self, "move+size", cycle_text :: call

fn cycle_clamp(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "clamp" :: call
    return match found:
        Option.Some(win) => cycle_clamp_ready :: self, win :: call
        Option.None => 0

fn cycle_clamp_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    self.clamp_cycle += 1
    if self.clamp_cycle >= 3:
        self.clamp_cycle = 0
    if self.clamp_cycle == 0:
        arcana_desktop.window.set_min_size :: win, 960, 640 :: call
        arcana_desktop.window.set_max_size :: win, 0, 0 :: call
        set_status :: self, "clamp", "restored default min / no max" :: call
        return
    if self.clamp_cycle == 1:
        arcana_desktop.window.set_min_size :: win, 960, 640 :: call
        arcana_desktop.window.set_max_size :: win, 1480, 980 :: call
        set_status :: self, "clamp", "applied max 1480 x 980" :: call
        return
    arcana_desktop.window.set_min_size :: win, 720, 520 :: call
    arcana_desktop.window.set_max_size :: win, 1200, 780 :: call
    set_status :: self, "clamp", "applied compact bounds" :: call

fn apply_window_preset(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "preset" :: call
    return match found:
        Option.Some(win) => apply_window_preset_ready :: self, win :: call
        Option.None => 0

fn apply_window_preset_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let mut settings = arcana_desktop.window.settings :: win :: call
    self.preset_cycle += 1
    if self.preset_cycle >= 2:
        self.preset_cycle = 0
    settings.title = current_title :: self :: call
    if self.preset_cycle == 0:
        settings.bounds.position = (64, 52)
        settings.bounds.size = (1280, 760)
        settings.bounds.min_size = (960, 640)
        settings.bounds.max_size = (0, 0)
        settings.options.style.transparent = false
        settings.options.style.resizable = true
        settings.options.style.decorated = true
        settings.options.state.topmost = false
        settings.options.state.theme_override = arcana_desktop.types.WindowThemeOverride.System :: :: call
        settings.options.cursor.icon = arcana_desktop.types.CursorIcon.Default :: :: call
        settings.options.cursor.position = (-1, -1)
        set_status :: self, "preset", "restored baseline settings" :: call
    else:
        settings.bounds.position = (96, 72)
        settings.bounds.size = (1180, 720)
        settings.bounds.min_size = (900, 620)
        settings.bounds.max_size = (1540, 1040)
        settings.options.style.transparent = true
        settings.options.style.resizable = true
        settings.options.style.decorated = true
        settings.options.state.topmost = true
        settings.options.state.theme_override = arcana_desktop.types.WindowThemeOverride.Dark :: :: call
        settings.options.cursor.icon = arcana_desktop.types.CursorIcon.Hand :: :: call
        settings.options.cursor.position = (160, 120)
        set_status :: self, "preset", "applied dark transparent preset" :: call
    arcana_desktop.window.apply_settings :: win, settings :: call

fn toggle_composition_area(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "composition" :: call
    return match found:
        Option.Some(win) => toggle_composition_area_ready :: self, win :: call
        Option.None => 0

fn toggle_composition_area_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let area = arcana_desktop.text_input.composition_area :: win :: call
    if area.active:
        arcana_desktop.text_input.clear_composition_area :: win :: call
        set_status :: self, "composition", "cleared" :: call
        return
    arcana_desktop.text_input.set_enabled :: win, true :: call
    let size = arcana_desktop.window.size :: win :: call
    let next = arcana_desktop.types.CompositionArea :: active = true, position = (size.0 / 2 - 160, size.1 - 120), size = (320, 32) :: call
    arcana_desktop.text_input.set_composition_area :: win, next :: call
    set_status :: self, "composition", "direct area set" :: call

fn cycle_text_input_settings(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = action_window :: self, cx, "text settings" :: call
    return match found:
        Option.Some(win) => cycle_text_input_settings_ready :: self, win :: call
        Option.None => 0

fn cycle_text_input_settings_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let size = arcana_desktop.window.size :: win :: call
    let mut settings = arcana_desktop.window.text_input_settings :: win :: call
    if not settings.enabled:
        settings.enabled = true
        settings.composition_area.active = false
        set_status :: self, "text settings", "enabled via apply" :: call
    else:
        if not settings.composition_area.active:
            settings.composition_area.active = true
            settings.composition_area.position = (96, size.1 - 132)
            settings.composition_area.size = (280, 28)
            set_status :: self, "text settings", "enabled with area" :: call
        else:
            settings.enabled = false
            settings.composition_area.active = false
            settings.composition_area.position = (0, 0)
            settings.composition_area.size = (0, 0)
            set_status :: self, "text settings", "disabled via apply" :: call
    arcana_desktop.window.apply_text_input_settings :: win, settings :: call

fn toggle_second_window_visible(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = second_window_action :: self, cx, "second visible" :: call
    return match found:
        Option.Some(win) => toggle_second_window_visible_ready :: self, win :: call
        Option.None => 0

fn toggle_second_window_visible_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    let enabled = not (arcana_desktop.window.visible :: win :: call)
    arcana_desktop.window.set_visible :: win, enabled :: call
    self.second_window_visible = enabled
    self.second_window_alive = arcana_desktop.window.alive :: win :: call
    if enabled:
        set_status :: self, "second visible", "shown" :: call
    else:
        set_status :: self, "second visible", "hidden" :: call

fn close_second_window_direct(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = second_window_action :: self, cx, "second close" :: call
    return match found:
        Option.Some(win) => close_second_window_direct_ready :: self, cx, win :: call
        Option.None => 0

fn close_second_window_direct_ready(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, take win: arcana_desktop.types.Window):
    let queued = arcana_desktop.app.close_window :: cx, win :: call
    if queued :: :: is_err:
        set_status :: self, "second close", "close request failed" :: call
        return
    self.second_window_dirty = false
    request_main_redraw :: cx :: call
    set_status :: self, "second close", "close requested" :: call

fn copy_text(edit self: demo_types.Demo):
    let payload = "Arcana Desktop :: " + (pages.title :: self.page_index :: call)
    let wrote = arcana_desktop.clipboard.write_text :: payload :: call
    return match wrote:
        Result.Ok(_) => copy_text_readback :: self :: call
        Result.Err(err) => set_status :: self, "clipboard", err :: call

fn copy_text_readback(edit self: demo_types.Demo):
    let read_back = arcana_desktop.clipboard.read_text :: :: call
    return match read_back:
        Result.Ok(value) => copy_text_ready :: self, value :: call
        Result.Err(err) => set_status :: self, "clipboard", err :: call

fn copy_text_ready(edit self: demo_types.Demo, value: Str):
    self.last_clipboard = value
    set_status :: self, "clipboard", "text copied" :: call

fn copy_bytes(edit self: demo_types.Demo):
    let payload = std.bytes.from_str_utf8 :: ("arcana-desktop::" + (pages.title :: self.page_index :: call)) :: call
    let wrote = arcana_desktop.clipboard.write_bytes :: payload :: call
    return match wrote:
        Result.Ok(_) => copy_bytes_readback :: self :: call
        Result.Err(err) => set_status :: self, "clipboard", err :: call

fn copy_bytes_readback(edit self: demo_types.Demo):
    let read_back = arcana_desktop.clipboard.read_bytes :: :: call
    return match read_back:
        Result.Ok(value) => copy_bytes_ready :: self, value :: call
        Result.Err(err) => set_status :: self, "clipboard", err :: call

fn copy_bytes_ready(edit self: demo_types.Demo, read value: Array[Int]):
    self.last_clipboard = "bytes " + (std.text.from_int :: (std.bytes.len :: value :: call) :: call)
    set_status :: self, "clipboard", "bytes copied" :: call

fn signal_wake(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    self.pending_wake = true
    arcana_desktop.events.wake :: (arcana_desktop.app.wake_handle :: cx :: call) :: call
    set_status :: self, "wake", "signaled" :: call

fn cycle_device_policy(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let current = arcana_desktop.app.device_events :: cx :: call
    let next = next_device_policy :: current :: call
    arcana_desktop.app.set_device_events :: cx, next :: call
    sync_device_policy :: self, cx :: call
    let name = device_policy_name :: self.device_policy_code :: call
    set_status :: self, "device policy", name :: call

fn second_window(read self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.Window]:
    if self.second_window_id < 0:
        return Option.None[arcana_desktop.types.Window] :: :: call
    let id = arcana_desktop.types.WindowId :: value = self.second_window_id :: call
    return arcana_desktop.app.window_for_id :: cx, id :: call

fn second_window_action(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, head: Str) -> Option[arcana_desktop.types.Window]:
    let found = second_window :: self, cx :: call
    return match found:
        Option.Some(win) => Option.Some[arcana_desktop.types.Window] :: win :: call
        Option.None => second_window_missing :: self, head :: call

fn second_window_missing(edit self: demo_types.Demo, head: Str) -> Option[arcana_desktop.types.Window]:
    clear_second_window_state :: self :: call
    set_status :: self, head, "open second window first" :: call
    return Option.None[arcana_desktop.types.Window] :: :: call

fn request_second_window_redraw(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let found = second_window :: self, cx :: call
    return match found:
        Option.Some(win) => request_second_window_redraw_ready :: self, win :: call
        Option.None => request_second_window_redraw_missing :: self, cx :: call

fn request_second_window_redraw_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let mut win = win
    arcana_desktop.window.request_redraw :: win :: call
    self.second_window_dirty = false

fn request_second_window_redraw_missing(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    clear_second_window_state :: self :: call
    request_main_redraw :: cx :: call

fn target_is_second_window(read self: demo_types.Demo, read target: arcana_desktop.types.TargetedEvent) -> Bool:
    return target.window_id.value == self.second_window_id

fn update_hover(edit self: demo_types.Demo, read win: arcana_desktop.types.Window, point: (Int, Int)) -> Bool:
    self.mouse_pos = point
    let view = layout.for_window :: (arcana_desktop.window.size :: win :: call) :: call
    let next = layout.button_at :: view, point :: call
    if self.hover_button_id == next:
        return false
    self.hover_button_id = next
    return true

fn open_second_window(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    if self.second_window_id >= 0:
        let found = second_window :: self, cx :: call
        if found :: :: is_some:
            self.status_head = "second window"
            self.status_tail = "already open"
            mark_dirty :: self :: call
            return
        clear_second_window_state :: self :: call
    let mut cfg = arcana_desktop.window.default_config :: :: call
    cfg.title = "Arcana Desktop Proof :: Second Window"
    cfg.bounds.size = (460, 300)
    cfg.bounds.position = (720, 120)
    cfg.bounds.min_size = (320, 220)
    let opened = arcana_desktop.app.open_window_cfg :: cx, cfg :: call
    return match opened:
        Result.Ok(win) => open_second_window_ready :: self, win :: call
        Result.Err(err) => open_second_window_failed :: self, err :: call

fn open_second_window_ready(edit self: demo_types.Demo, take win: arcana_desktop.types.Window):
    let id = (arcana_desktop.window.id :: win :: call).value
    self.second_window_id = id
    self.second_window_seen = false
    self.second_window_dirty = true
    self.second_window_visible = true
    self.second_window_alive = true
    self.status_head = "second window"
    self.status_tail = "opened"
    if self.exercise_second_window:
        std.io.print_line[Str] :: ("second_window=open:" + (std.text.from_int :: id :: call)) :: call
        std.io.flush_stdout :: :: call
    mark_dirty :: self :: call

fn open_second_window_failed(edit self: demo_types.Demo, err: Str):
    self.status_head = "second window"
    self.status_tail = ("open failed: " + err)
    clear_second_window_state :: self :: call
    if self.exercise_second_window:
        std.io.print_line[Str] :: ("second_window=failed:" + err) :: call
        std.io.flush_stdout :: :: call
    mark_dirty :: self :: call

fn finish_smoke(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    if self.smoke_printed:
        return
    std.io.print_line[Str] :: ("controls=" + (std.text.from_int :: (actions.button_count :: :: call) :: call)) :: call
    std.io.print_line[Str] :: ("pages=" + (std.text.from_int :: (pages.count :: :: call) :: call)) :: call
    std.io.print_line[Str] :: "smoke_score=767" :: call
    std.io.flush_stdout :: :: call
    self.smoke_printed = true
    arcana_desktop.app.request_exit :: cx, 0 :: call

fn handle_button(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, id: Int) -> arcana_desktop.types.ControlFlow:
    if id == 0:
        let next = actions.next_page_index :: self.page_index, -1 :: call
        return set_page :: self, cx, next :: call
    if id == 1:
        let next = actions.next_page_index :: self.page_index, 1 :: call
        return set_page :: self, cx, next :: call
    let page = actions.button_page :: id :: call
    if page >= 0:
        return set_page :: self, cx, page :: call
    if id == 9:
        cycle_theme :: self, cx :: call
        return cx.control.control_flow
    if id == 10:
        toggle_text_input :: self, cx :: call
        return cx.control.control_flow
    if id == 11:
        cycle_cursor_icon :: self, cx :: call
        return cx.control.control_flow
    if id == 12:
        toggle_cursor_visible :: self, cx :: call
        return cx.control.control_flow
    if id == 13:
        center_cursor :: self, cx :: call
        return cx.control.control_flow
    if id == 14:
        toggle_topmost :: self, cx :: call
        return cx.control.control_flow
    if id == 15:
        toggle_decorated :: self, cx :: call
        return cx.control.control_flow
    if id == 16:
        toggle_resizable :: self, cx :: call
        return cx.control.control_flow
    if id == 17:
        request_attention :: self, cx :: call
        return cx.control.control_flow
    if id == 18:
        copy_text :: self :: call
        return cx.control.control_flow
    if id == 19:
        copy_bytes :: self :: call
        return cx.control.control_flow
    if id == 20:
        signal_wake :: self, cx :: call
        return cx.control.control_flow
    if id == 21:
        cycle_device_policy :: self, cx :: call
        return cx.control.control_flow
    if id == 22:
        open_second_window :: self, cx :: call
        return cx.control.control_flow
    if id == 23:
        arcana_desktop.app.request_exit :: cx, 0 :: call
        return arcana_desktop.types.ControlFlow.Wait :: :: call
    if id == 24:
        toggle_fullscreen :: self, cx :: call
        return cx.control.control_flow
    if id == 25:
        toggle_maximized :: self, cx :: call
        return cx.control.control_flow
    if id == 26:
        minimize_window :: self, cx :: call
        return cx.control.control_flow
    if id == 27:
        toggle_transparent :: self, cx :: call
        return cx.control.control_flow
    if id == 28:
        cycle_cursor_grab :: self, cx :: call
        return cx.control.control_flow
    if id == 29:
        cycle_move_size :: self, cx :: call
        return cx.control.control_flow
    if id == 30:
        cycle_clamp :: self, cx :: call
        return cx.control.control_flow
    if id == 31:
        apply_window_preset :: self, cx :: call
        return cx.control.control_flow
    if id == 32:
        toggle_composition_area :: self, cx :: call
        return cx.control.control_flow
    if id == 33:
        cycle_text_input_settings :: self, cx :: call
        return cx.control.control_flow
    if id == 34:
        toggle_second_window_visible :: self, cx :: call
        return cx.control.control_flow
    if id == 35:
        close_second_window_direct :: self, cx :: call
        return cx.control.control_flow
    set_status :: self, "button", (actions.button_label :: id :: call) :: call
    return cx.control.control_flow

fn on_main_redraw(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, take win: arcana_desktop.types.Window) -> arcana_desktop.types.ControlFlow:
    let mut win = win
    self.redraw_count += 1
    self.last_event = "WindowRedrawRequested"
    self.last_window = "main"
    probe_line :: self, "main_redraw_start", ("count=" + (std.text.from_int :: self.redraw_count :: call)) :: call
    if not self.dirty and self.controls_dirty:
        render.draw_controls_only :: self, win :: call
        self.controls_dirty = false
        probe_line :: self, "main_redraw_controls_only", "done" :: call
        return arcana_desktop.types.ControlFlow.Wait :: :: call
    refresh_main_monitor :: self, cx :: call
    refresh_second_window_state :: self, cx :: call
    sync_device_policy :: self, cx :: call
    sync_body_lines :: self, win :: call
    probe_line :: self, "main_redraw_before_render", self.status_tail :: call
    render.draw_main :: self, win :: call
    probe_line :: self, "main_redraw_after_render", "done" :: call
    self.dirty = false
    self.controls_dirty = false
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_second_redraw(edit self: demo_types.Demo, take win: arcana_desktop.types.Window) -> arcana_desktop.types.ControlFlow:
    let mut win = win
    self.second_window_seen = true
    self.second_window_alive = arcana_desktop.window.alive :: win :: call
    self.second_window_visible = arcana_desktop.window.visible :: win :: call
    self.last_event = "WindowRedrawRequested"
    self.last_window = "second"
    render.draw_secondary :: self, win :: call
    self.second_window_dirty = false
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_close_requested(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    self.close_requests += 1
    self.last_event = "WindowCloseRequested"
    if target_is_second_window :: self, target :: call:
        self.status_head = "close"
        self.status_tail = "second window closing"
        clear_second_window_state :: self :: call
        request_main_redraw :: cx :: call
    else:
        self.status_head = "close"
        self.status_tail = "main window closing"
    let handled = arcana_desktop.app.close_current_window :: cx :: call
    return handled :: (arcana_desktop.types.ControlFlow.Wait :: :: call) :: unwrap_or

fn on_second_redraw_current(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    let win = arcana_desktop.app.require_current_window :: cx :: call
    return match win:
        Result.Ok(value) => on_second_redraw :: self, value :: call
        Result.Err(_) => cx.control.control_flow

fn on_main_redraw_current(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    if self.smoke_mode:
        self.redraw_count += 1
        self.last_event = "WindowRedrawRequested"
        self.last_window = "main"
        self.dirty = false
        self.controls_dirty = false
        return arcana_desktop.types.ControlFlow.Wait :: :: call
    let win = arcana_desktop.app.require_current_window :: cx :: call
    return match win:
        Result.Ok(value) => on_main_redraw :: self, cx, value :: call
        Result.Err(_) => cx.control.control_flow

fn on_main_mouse_move_current(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: arcana_desktop.types.MouseMoveEvent) -> arcana_desktop.types.ControlFlow:
    let win = arcana_desktop.app.require_current_window :: cx :: call
    return match win:
        Result.Ok(value) => on_main_mouse_move :: self, cx, (value, ev) :: call
        Result.Err(_) => cx.control.control_flow

fn on_main_mouse_up_current(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: arcana_desktop.types.MouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    let win = arcana_desktop.app.require_current_window :: cx :: call
    return match win:
        Result.Ok(value) => on_main_mouse_up :: self, cx, (value, ev) :: call
        Result.Err(_) => cx.control.control_flow

fn on_main_mouse_move(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read payload: (arcana_desktop.types.Window, arcana_desktop.types.MouseMoveEvent)) -> arcana_desktop.types.ControlFlow:
    let win = payload.0
    let ev = payload.1
    self.mouse_events += 1
    self.last_event = "MouseMove"
    self.last_mouse = (std.text.from_int :: ev.position.0 :: call) + "," + (std.text.from_int :: ev.position.1 :: call)
    let hover_changed = update_hover :: self, win, ev.position :: call
    if hover_changed and not self.ui_smoke_mode:
        mark_controls_dirty :: self :: call
        mark_telemetry_dirty :: self :: call
        request_main_redraw :: cx :: call
    else:
        mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_mouse_down(edit self: demo_types.Demo, read ev: arcana_desktop.types.MouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.last_event = "MouseDown"
    self.last_mouse = (std.text.from_int :: ev.position.0 :: call) + "," + (std.text.from_int :: ev.position.1 :: call)
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_mouse_up(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read payload: (arcana_desktop.types.Window, arcana_desktop.types.MouseButtonEvent)) -> arcana_desktop.types.ControlFlow:
    let win = payload.0
    let ev = payload.1
    self.mouse_events += 1
    self.last_event = "MouseUp"
    self.last_mouse = (std.text.from_int :: ev.position.0 :: call) + "," + (std.text.from_int :: ev.position.1 :: call)
    let _ = update_hover :: self, win, ev.position :: call
    if ev.button != (arcana_desktop.input.mouse_button_code :: "Left" :: call):
        mark_telemetry_dirty :: self :: call
        return cx.control.control_flow
    let view = layout.for_window :: (arcana_desktop.window.size :: win :: call) :: call
    let id = layout.button_at :: view, ev.position :: call
    if id < 0:
        mark_telemetry_dirty :: self :: call
        return cx.control.control_flow
    return handle_button :: self, cx, id :: call

fn on_main_key_down(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: arcana_desktop.types.KeyEvent) -> arcana_desktop.types.ControlFlow:
    self.key_events += 1
    self.last_event = "KeyDown"
    self.last_key = std.text.from_int :: ev.key :: call
    let key_q = arcana_desktop.input.key_code :: "Q" :: call
    let key_e = arcana_desktop.input.key_code :: "E" :: call
    let key_n = arcana_desktop.input.key_code :: "N" :: call
    let key_w = arcana_desktop.input.key_code :: "W" :: call
    let key_escape = arcana_desktop.input.key_code :: "Escape" :: call
    if ev.key == key_q:
        return handle_button :: self, cx, 0 :: call
    if ev.key == key_e:
        return handle_button :: self, cx, 1 :: call
    if ev.key == key_n:
        return handle_button :: self, cx, 22 :: call
    if ev.key == key_w:
        arcana_desktop.events.wake :: (arcana_desktop.app.wake_handle :: cx :: call) :: call
        return cx.control.control_flow
    if ev.key == key_escape:
        arcana_desktop.app.request_exit :: cx, 0 :: call
        return arcana_desktop.types.ControlFlow.Wait :: :: call
    mark_dirty :: self :: call
    return cx.control.control_flow

fn on_main_key_up(edit self: demo_types.Demo, read ev: arcana_desktop.types.KeyEvent) -> arcana_desktop.types.ControlFlow:
    self.key_events += 1
    self.last_event = "KeyUp"
    self.last_key = std.text.from_int :: ev.key :: call
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_second_mouse_up(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, window_id: Int) -> arcana_desktop.types.ControlFlow:
    let _ = cx
    if self.exercise_second_window:
        std.io.print_line[Str] :: ("second_window=click:" + (std.text.from_int :: window_id :: call)) :: call
        std.io.flush_stdout :: :: call
    self.last_event = "MouseUp"
    self.last_window = "second"
    self.status_head = "second window"
    self.status_tail = "clicked"
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

impl arcana_desktop.app.Application[demo_types.Demo] for demo_types.Demo:
    fn resumed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
        probe_line :: self, "resumed", "enter" :: call
        self.last_monitor = (arcana_desktop.monitor.primary :: :: call).name
        sync_device_policy :: self, cx :: call
        refresh_second_window_state :: self, cx :: call
        self.last_window = "main"
        if self.smoke_mode:
            self.pending_wake = true
            arcana_desktop.events.wake :: (arcana_desktop.app.wake_handle :: cx :: call) :: call
            self.status_head = "smoke"
            self.status_tail = "waiting for wake-only smoke completion"
            return
        if self.exercise_second_window:
            open_second_window :: self, cx :: call
            return
        set_status :: self, "ready", "interactive showcase ready" :: call
        probe_line :: self, "resumed", "ready" :: call

    fn suspended(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
        let _ = cx
        self.status_head = "suspended"
        self.status_tail = "application suspended"

    fn window_event(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
        if self.probe_mode:
            let mut event_name = "window"
            event_name = match target.event:
                arcana_desktop.types.WindowEvent.WindowRedrawRequested(_) => "redraw"
                arcana_desktop.types.WindowEvent.WindowCloseRequested(_) => "close"
                arcana_desktop.types.WindowEvent.MouseMove(_) => "mouse_move"
                arcana_desktop.types.WindowEvent.MouseUp(_) => "mouse_up"
                arcana_desktop.types.WindowEvent.MouseDown(_) => "mouse_down"
                arcana_desktop.types.WindowEvent.KeyDown(_) => "key_down"
                arcana_desktop.types.WindowEvent.KeyUp(_) => "key_up"
                arcana_desktop.types.WindowEvent.TextInput(_) => "text_input"
                arcana_desktop.types.WindowEvent.WindowResized(_) => "resize"
                _ => event_name
            probe_line :: self, "window_event", event_name :: call
        if target_is_second_window :: self, target :: call:
            return match target.event:
                arcana_desktop.types.WindowEvent.WindowRedrawRequested(_) => on_second_redraw_current :: self, cx :: call
                arcana_desktop.types.WindowEvent.WindowCloseRequested(_) => on_close_requested :: self, cx, target :: call
                arcana_desktop.types.WindowEvent.MouseUp(ev) => on_second_mouse_up :: self, cx, ev.window_id :: call
                _ => cx.control.control_flow
        return match target.event:
            arcana_desktop.types.WindowEvent.WindowRedrawRequested(_) => on_main_redraw_current :: self, cx :: call
            arcana_desktop.types.WindowEvent.WindowCloseRequested(_) => on_close_requested :: self, cx, target :: call
            arcana_desktop.types.WindowEvent.WindowResized(ev) => on_main_window_resized_inline :: self, cx, ev :: call
            arcana_desktop.types.WindowEvent.WindowScaleFactorChanged(ev) => on_main_window_scale_inline :: self, ev.scale_factor_milli :: call
            arcana_desktop.types.WindowEvent.WindowThemeChanged(ev) => on_main_window_theme_inline :: self, ev.theme_code :: call
            arcana_desktop.types.WindowEvent.KeyUp(ev) => on_main_key_up :: self, ev :: call
            arcana_desktop.types.WindowEvent.MouseDown(ev) => on_main_mouse_down :: self, ev :: call
            arcana_desktop.types.WindowEvent.MouseMove(ev) => on_main_mouse_move_current :: self, cx, ev :: call
            arcana_desktop.types.WindowEvent.MouseUp(ev) => on_main_mouse_up_current :: self, cx, ev :: call
            arcana_desktop.types.WindowEvent.MouseEntered(_) => on_main_mouse_enter :: self, cx :: call
            arcana_desktop.types.WindowEvent.MouseLeft(_) => on_main_mouse_left :: self, cx :: call
            arcana_desktop.types.WindowEvent.KeyDown(ev) => on_main_key_down :: self, cx, ev :: call
            arcana_desktop.types.WindowEvent.TextInput(ev) => on_main_text_input :: self, ev :: call
            arcana_desktop.types.WindowEvent.MouseWheel(ev) => on_main_mouse_wheel :: self, ev :: call
            arcana_desktop.types.WindowEvent.WindowMoved(_) => on_main_window_moved_inline :: self, cx :: call
            arcana_desktop.types.WindowEvent.WindowFocused(ev) => on_main_window_focused_inline :: self, ev.focused :: call
            arcana_desktop.types.WindowEvent.TextCompositionStarted(_) => on_main_text_composition_started :: self :: call
            arcana_desktop.types.WindowEvent.TextCompositionUpdated(ev) => on_main_text_composition_updated :: self, ev :: call
            arcana_desktop.types.WindowEvent.TextCompositionCommitted(ev) => on_main_text_composition_committed :: self, ev :: call
            arcana_desktop.types.WindowEvent.TextCompositionCancelled(_) => on_main_text_composition_cancelled :: self :: call
            arcana_desktop.types.WindowEvent.FileDropped(ev) => on_main_file_dropped :: self, ev :: call
            _ => cx.control.control_flow

    fn device_event(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:
        let _ = cx
        return match event:
            arcana_desktop.types.DeviceEvent.RawMouseMotion(ev) => on_raw_mouse_motion :: self, ev :: call
            arcana_desktop.types.DeviceEvent.RawMouseButton(ev) => on_raw_mouse_button :: self, ev :: call
            arcana_desktop.types.DeviceEvent.RawMouseWheel(ev) => on_raw_mouse_wheel_device :: self, ev :: call
            arcana_desktop.types.DeviceEvent.RawKey(ev) => on_raw_key_device :: self, ev :: call
            _ => arcana_desktop.types.ControlFlow.Wait :: :: call

    fn about_to_wait(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        let dirty_code = bool_code :: self.dirty :: call
        let controls_code = bool_code :: self.controls_dirty :: call
        probe_line :: self, "about_to_wait", ("dirty=" + (std.text.from_int :: dirty_code :: call) + " controls=" + (std.text.from_int :: controls_code :: call)) :: call
        flush_main_title :: self, cx :: call
        maybe_schedule_telemetry_redraw :: self :: call
        if self.dirty or self.controls_dirty:
            request_main_redraw :: cx :: call
        if self.second_window_dirty:
            request_second_window_redraw :: self, cx :: call
        return cx.control.control_flow

    fn wake(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        self.wake_count += 1
        self.pending_wake = false
        self.last_event = "Wake"
        if self.smoke_mode:
            finish_smoke :: self, cx :: call
            return arcana_desktop.types.ControlFlow.Wait :: :: call
        self.status_head = "wake"
        self.status_tail = "wake delivered"
        mark_dirty :: self :: call
        return cx.control.control_flow

    fn exiting(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
        let _ = cx
        self.status_head = "exiting"
        self.status_tail = "desktop showcase exiting"

fn on_main_mouse_enter(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    let changed = not self.mouse_inside
    self.mouse_inside = true
    self.last_event = "MouseEntered"
    if changed:
        mark_controls_dirty :: self :: call
        mark_telemetry_dirty :: self :: call
        request_main_redraw :: cx :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_mouse_left(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    let changed = self.mouse_inside or self.hover_button_id >= 0
    self.mouse_inside = false
    self.hover_button_id = -1
    self.last_event = "MouseLeft"
    self.last_mouse = "outside"
    if changed:
        mark_controls_dirty :: self :: call
        mark_telemetry_dirty :: self :: call
        request_main_redraw :: cx :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_text_input(edit self: demo_types.Demo, read ev: arcana_desktop.types.TextInputEvent) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextInput"
    self.last_text = ev.text
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_text_composition_started(edit self: demo_types.Demo) -> arcana_desktop.types.ControlFlow:
    self.last_event = "TextCompositionStarted"
    self.status_head = "text input"
    self.status_tail = "composition started"
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_text_composition_updated(edit self: demo_types.Demo, read ev: arcana_desktop.types.TextCompositionEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "TextCompositionUpdated"
    self.last_text = ev.text
    self.status_head = "text input"
    self.status_tail = "composition updated"
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_text_composition_committed(edit self: demo_types.Demo, read ev: arcana_desktop.types.TextCompositionEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "TextCompositionCommitted"
    self.last_text = ev.text
    self.status_head = "text input"
    self.status_tail = "composition committed"
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_text_composition_cancelled(edit self: demo_types.Demo) -> arcana_desktop.types.ControlFlow:
    self.last_event = "TextCompositionCancelled"
    self.status_head = "text input"
    self.status_tail = "composition cancelled"
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_file_dropped(edit self: demo_types.Demo, read ev: arcana_desktop.types.FileDropEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "FileDropped"
    self.status_head = "file drop"
    self.status_tail = ev.path
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_mouse_wheel(edit self: demo_types.Demo, read ev: arcana_desktop.types.MouseWheelEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.mouse_wheel_y += ev.delta.1
    self.last_event = "MouseWheel"
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_window_resized_inline(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: arcana_desktop.types.WindowResizeEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowResized"
    self.last_window = "main"
    self.status_head = "resize"
    self.status_tail = (std.text.from_int :: ev.size.0 :: call) + " x " + (std.text.from_int :: ev.size.1 :: call)
    refresh_main_monitor :: self, cx :: call
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_window_moved_inline(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowMoved"
    self.last_window = "main"
    refresh_main_monitor :: self, cx :: call
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_window_focused_inline(edit self: demo_types.Demo, focused: Bool) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowFocused"
    if focused:
        self.status_head = "focus"
        self.status_tail = "focused"
    else:
        self.status_head = "focus"
        self.status_tail = "blurred"
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_window_scale_inline(edit self: demo_types.Demo, scale_factor_milli: Int) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowScaleFactorChanged"
    self.status_head = "scale"
    self.status_tail = std.text.from_int :: scale_factor_milli :: call
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_main_window_theme_inline(edit self: demo_types.Demo, theme_code: Int) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowThemeChanged"
    self.status_head = "theme"
    self.status_tail = std.text.from_int :: theme_code :: call
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_raw_mouse_motion(edit self: demo_types.Demo, read ev: arcana_desktop.types.RawMouseMotionEvent) -> arcana_desktop.types.ControlFlow:
    let mut dx = ev.delta.0
    let mut dy = ev.delta.1
    if dx < 0:
        dx = 0 - dx
    if dy < 0:
        dy = 0 - dy
    self.raw_motion_total += dx
    self.raw_motion_total += dy
    self.last_event = "RawMouseMotion"
    self.last_device = device_id_text :: ev.device_id :: call
    mark_telemetry_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_raw_mouse_button(edit self: demo_types.Demo, read ev: arcana_desktop.types.RawMouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    self.raw_button_events += 1
    self.last_event = "RawMouseButton"
    self.last_device = device_id_text :: ev.device_id :: call
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_raw_mouse_wheel_device(edit self: demo_types.Demo, read ev: arcana_desktop.types.RawMouseWheelEvent) -> arcana_desktop.types.ControlFlow:
    self.raw_wheel_events += 1
    self.last_event = "RawMouseWheel"
    self.last_device = device_id_text :: ev.device_id :: call
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_raw_key_device(edit self: demo_types.Demo, read ev: arcana_desktop.types.RawKeyEvent) -> arcana_desktop.types.ControlFlow:
    self.raw_key_events += 1
    self.last_event = "RawKey"
    self.last_device = device_id_text :: ev.device_id :: call
    if ev.pressed:
        self.last_key = std.text.from_int :: ev.key :: call
    mark_dirty :: self :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn main() -> Int:
    let smoke_mode = has_flag :: "--smoke" :: call
    let ui_smoke_mode = has_flag :: "--ui-smoke" :: call
    let probe_mode = has_flag :: "--probe" :: call
    let second_window_mode = has_flag :: "--exercise-second-window" :: call
    let mut cfg = arcana_desktop.app.default_app_config :: :: call
    cfg.window.title = "Arcana Desktop Proof :: " + (pages.title :: 0 :: call)
    cfg.window.bounds.size = (1280, 760)
    cfg.window.bounds.position = (64, 52)
    cfg.window.bounds.min_size = (960, 640)
    let mut app = default_demo :: smoke_mode :: call
    app.ui_smoke_mode = ui_smoke_mode
    app.probe_mode = probe_mode
    app.exercise_second_window = second_window_mode
    if probe_mode:
        reset_probe_log :: :: call
    return arcana_desktop.app.run :: app, cfg :: call
