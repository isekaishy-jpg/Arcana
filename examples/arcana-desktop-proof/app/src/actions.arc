import arcana_desktop.app
import arcana_desktop.clipboard
import arcana_desktop.events
import arcana_desktop.text_input
import arcana_desktop.types
import arcana_desktop.window
import demo_types
import layout
import pages
import std.bytes
import std.result
import std.text
import std.time
import std.io
use std.result.Result
use std.window.Window

fn set_status(edit self: demo_types.Demo, head: Str, tail: Str):
    self.status_head = head
    self.status_tail = tail

fn enabled_text(enabled: Bool) -> Str:
    if enabled:
        return "enabled"
    return "disabled"

fn requested_text(enabled: Bool) -> Str:
    if enabled:
        return "requested"
    return "cleared"

fn mark_dirty(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let _ = cx
    if self.dirty:
        return
    self.dirty = true

fn next_page_index(current: Int, delta: Int) -> Int:
    let total = pages.count :: :: call
    let mut next = current + delta
    if next < 0:
        next = total - 1
    if next >= total:
        next = 0
    return next

fn next_theme(read value: arcana_desktop.types.WindowThemeOverride) -> arcana_desktop.types.WindowThemeOverride:
    return match value:
        arcana_desktop.types.WindowThemeOverride.System => arcana_desktop.types.WindowThemeOverride.Dark :: :: call
        arcana_desktop.types.WindowThemeOverride.Dark => arcana_desktop.types.WindowThemeOverride.Light :: :: call
        _ => arcana_desktop.types.WindowThemeOverride.System :: :: call

fn next_cursor_icon(read value: arcana_desktop.types.CursorIcon) -> arcana_desktop.types.CursorIcon:
    return match value:
        arcana_desktop.types.CursorIcon.Default => arcana_desktop.types.CursorIcon.Text :: :: call
        arcana_desktop.types.CursorIcon.Text => arcana_desktop.types.CursorIcon.Crosshair :: :: call
        arcana_desktop.types.CursorIcon.Crosshair => arcana_desktop.types.CursorIcon.Hand :: :: call
        arcana_desktop.types.CursorIcon.Hand => arcana_desktop.types.CursorIcon.Move :: :: call
        arcana_desktop.types.CursorIcon.Move => arcana_desktop.types.CursorIcon.Wait :: :: call
        arcana_desktop.types.CursorIcon.Wait => arcana_desktop.types.CursorIcon.Help :: :: call
        arcana_desktop.types.CursorIcon.Help => arcana_desktop.types.CursorIcon.NotAllowed :: :: call
        arcana_desktop.types.CursorIcon.NotAllowed => arcana_desktop.types.CursorIcon.ResizeHorizontal :: :: call
        arcana_desktop.types.CursorIcon.ResizeHorizontal => arcana_desktop.types.CursorIcon.ResizeVertical :: :: call
        arcana_desktop.types.CursorIcon.ResizeVertical => arcana_desktop.types.CursorIcon.ResizeNwse :: :: call
        arcana_desktop.types.CursorIcon.ResizeNwse => arcana_desktop.types.CursorIcon.ResizeNesw :: :: call
        _ => arcana_desktop.types.CursorIcon.Default :: :: call

fn next_grab_mode(read value: arcana_desktop.types.CursorGrabMode) -> arcana_desktop.types.CursorGrabMode:
    return match value:
        arcana_desktop.types.CursorGrabMode.Free => arcana_desktop.types.CursorGrabMode.Confined :: :: call
        arcana_desktop.types.CursorGrabMode.Confined => arcana_desktop.types.CursorGrabMode.Locked :: :: call
        _ => arcana_desktop.types.CursorGrabMode.Free :: :: call

fn sync_main_title(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let title = "Arcana Desktop Proof :: " + (pages.title :: self.page_index :: call)
    if (arcana_desktop.window.title :: cx.runtime.main_window :: call) == title:
        return
    arcana_desktop.window.set_title :: cx.runtime.main_window, title :: call

fn open_second_window(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    if self.second_window_id >= 0:
        actions.set_status :: self, "second window", "already open" :: call
        actions.mark_dirty :: self, cx :: call
        return
    let opened = arcana_desktop.app.open_window :: cx, "Arcana Desktop Proof :: Second Window", (460, 300) :: call
    return match opened:
        Result.Ok(win) => on_second_window_opened :: self, cx, win :: call
        Result.Err(err) => on_second_window_failed :: self, cx, err :: call

fn on_second_window_opened(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, take win: Window):
    let _ = cx
    let win = win
    self.second_window_id = (arcana_desktop.window.id :: win :: call).value
    self.second_window_seen = false
    self.dirty = false
    if self.ui_smoke_mode or self.exercise_second_window:
        std.io.print_line[Str] :: ("second_window=open:" + (std.text.from_int :: self.second_window_id :: call)) :: call
        std.io.flush_stdout :: :: call
    actions.set_status :: self, "second window", "opened through arcana_desktop.app.open_window" :: call

fn on_second_window_failed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, err: Str):
    self.second_window_id = -1
    self.second_window_seen = false
    if self.ui_smoke_mode or self.exercise_second_window:
        std.io.print_line[Str] :: ("second_window=failed:" + err) :: call
        std.io.flush_stdout :: :: call
    actions.set_status :: self, "second window", ("open failed: " + err) :: call
    actions.mark_dirty :: self, cx :: call

fn apply_window_profile(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let mut win = cx.runtime.main_window
    let mut settings = arcana_desktop.window.settings :: win :: call
    settings.title = "Arcana Desktop Proof :: " + (pages.title :: self.page_index :: call)
    settings.bounds.size = (1220 + self.page_index * 12, 760 + self.page_index * 6)
    settings.bounds.position = (48 + self.page_index * 12, 60 + self.page_index * 8)
    settings.bounds.min_size = (900, 620)
    settings.bounds.max_size = (1540, 1040)
    settings.options.style.resizable = true
    settings.options.style.decorated = true
    settings.options.style.transparent = self.page_index % 2 == 0
    settings.options.state.topmost = false
    settings.options.state.maximized = false
    settings.options.state.fullscreen = false
    settings.options.state.theme_override = (arcana_desktop.types.WindowThemeOverride.Dark :: :: call)
    settings.options.cursor.visible = true
    settings.options.cursor.icon = (arcana_desktop.types.CursorIcon.Hand :: :: call)
    settings.options.cursor.grab_mode = (arcana_desktop.types.CursorGrabMode.Free :: :: call)
    settings.options.cursor.position = (160, 128)
    settings.options.text_input_enabled = true
    arcana_desktop.window.apply_settings :: win, settings :: call
    let mut text = arcana_desktop.text_input.settings :: win :: call
    text.enabled = true
    text.composition_area.active = true
    text.composition_area.position = (120, 540)
    text.composition_area.size = (260, 28)
    arcana_desktop.text_input.apply_settings :: win, text :: call
    actions.set_status :: self, "window profile applied", "whole-record window/text settings roundtrip applied live" :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_fullscreen(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.window.fullscreen :: cx.runtime.main_window :: call)
    arcana_desktop.window.set_fullscreen :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "fullscreen", (actions.enabled_text :: enabled :: call) :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_maximized(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.window.maximized :: cx.runtime.main_window :: call)
    arcana_desktop.window.set_maximized :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "maximize", (actions.enabled_text :: enabled :: call) :: call
    actions.mark_dirty :: self, cx :: call

fn minimize_now(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    arcana_desktop.window.set_minimized :: cx.runtime.main_window, true :: call
    actions.set_status :: self, "minimize", "window requested minimized" :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_resizable(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.window.resizable :: cx.runtime.main_window :: call)
    arcana_desktop.window.set_resizable :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "resizable", (actions.enabled_text :: enabled :: call) :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_decorated(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.window.decorated :: cx.runtime.main_window :: call)
    arcana_desktop.window.set_decorated :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "decorated", (actions.enabled_text :: enabled :: call) :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_transparent(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.window.transparent :: cx.runtime.main_window :: call)
    arcana_desktop.window.set_transparent :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "transparent", (actions.enabled_text :: enabled :: call) :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_topmost(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.window.topmost :: cx.runtime.main_window :: call)
    arcana_desktop.window.set_topmost :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "topmost", (actions.enabled_text :: enabled :: call) :: call
    actions.mark_dirty :: self, cx :: call

fn cycle_theme(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let next = actions.next_theme :: (arcana_desktop.window.theme_override :: cx.runtime.main_window :: call) :: call
    arcana_desktop.window.set_theme_override :: cx.runtime.main_window, next :: call
    actions.set_status :: self, "theme override", "cycled theme override" :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_cursor_visible(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.window.cursor_visible :: cx.runtime.main_window :: call)
    arcana_desktop.window.set_cursor_visible :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "cursor visible", (actions.enabled_text :: enabled :: call) :: call
    actions.mark_dirty :: self, cx :: call

fn cycle_cursor_icon(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let current = arcana_desktop.window.cursor_icon :: cx.runtime.main_window :: call
    let next = actions.next_cursor_icon :: current :: call
    arcana_desktop.window.set_cursor_icon :: cx.runtime.main_window, next :: call
    actions.set_status :: self, "cursor icon", "cycled cursor icon" :: call
    actions.mark_dirty :: self, cx :: call

fn cycle_grab_mode(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let current = arcana_desktop.window.cursor_grab_mode :: cx.runtime.main_window :: call
    let next = actions.next_grab_mode :: current :: call
    arcana_desktop.window.set_cursor_grab_mode :: cx.runtime.main_window, next :: call
    actions.set_status :: self, "cursor grab", "cycled grab mode" :: call
    actions.mark_dirty :: self, cx :: call

fn center_cursor(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let size = arcana_desktop.window.size :: cx.runtime.main_window :: call
    let x = size.0 / 2
    let y = size.1 / 2
    arcana_desktop.window.set_cursor_position :: cx.runtime.main_window, x, y :: call
    actions.set_status :: self, "cursor position", ("centered to " + (std.text.from_int :: x :: call) + "," + (std.text.from_int :: y :: call)) :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_text_input(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let enabled = not (arcana_desktop.text_input.enabled :: cx.runtime.main_window :: call)
    arcana_desktop.text_input.set_enabled :: cx.runtime.main_window, enabled :: call
    actions.set_status :: self, "text input", "toggled text input enabled state" :: call
    actions.mark_dirty :: self, cx :: call

fn set_comp_area(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let mut area = arcana_desktop.text_input.composition_area :: cx.runtime.main_window :: call
    area.active = true
    area.position = (100 + self.page_index * 12, 560)
    area.size = (260, 28)
    arcana_desktop.text_input.set_composition_area :: cx.runtime.main_window, area :: call
    actions.set_status :: self, "composition area", "set active composition area" :: call
    actions.mark_dirty :: self, cx :: call

fn clear_comp_area(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    arcana_desktop.text_input.clear_composition_area :: cx.runtime.main_window :: call
    actions.set_status :: self, "composition area", "cleared composition area" :: call
    actions.mark_dirty :: self, cx :: call

fn copy_text(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let text = "Arcana Desktop Proof :: " + (pages.title :: self.page_index :: call)
    let wrote = arcana_desktop.clipboard.write_text :: text :: call
    return match wrote:
        Result.Ok(_) => on_copy_text_done :: self, cx :: call
        Result.Err(err) => on_copy_failed :: self, cx, err :: call

fn on_copy_text_done(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let text = (arcana_desktop.clipboard.read_text :: :: call) :: "" :: unwrap_or
    self.last_clipboard = "text: " + text
    actions.set_status :: self, "clipboard text", text :: call
    actions.mark_dirty :: self, cx :: call

fn copy_bytes(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let bytes = std.bytes.from_str_utf8 :: ("page:" + (pages.title :: self.page_index :: call)) :: call
    let wrote = arcana_desktop.clipboard.write_bytes :: bytes :: call
    return match wrote:
        Result.Ok(_) => on_copy_bytes_done :: self, cx :: call
        Result.Err(err) => on_copy_failed :: self, cx, err :: call

fn on_copy_bytes_done(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let bytes = (arcana_desktop.clipboard.read_bytes :: :: call) :: (std.bytes.from_str_utf8 :: "" :: call) :: unwrap_or
    let summary = "bytes: " + (std.text.from_int :: (std.bytes.len :: bytes :: call) :: call)
    self.last_clipboard = summary
    actions.set_status :: self, "clipboard bytes", summary :: call
    actions.mark_dirty :: self, cx :: call

fn on_copy_failed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, err: Str):
    self.last_clipboard = "clipboard error: " + err
    actions.set_status :: self, "clipboard", err :: call
    actions.mark_dirty :: self, cx :: call

fn send_wake(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    self.pending_wake = true
    self.pending_wake_note = "wake requested from showcase control"
    arcana_desktop.events.wake :: (arcana_desktop.app.wake_handle :: cx :: call) :: call
    actions.set_status :: self, "wake", "wake signal sent" :: call
    actions.mark_dirty :: self, cx :: call

fn toggle_attention(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    self.attention_on = not self.attention_on
    arcana_desktop.window.request_attention :: cx.runtime.main_window, self.attention_on :: call
    let status = actions.requested_text :: self.attention_on :: call
    actions.set_status :: self, "attention", status :: call
    actions.mark_dirty :: self, cx :: call

fn set_poll(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    arcana_desktop.app.set_control_flow :: cx, (arcana_desktop.types.ControlFlow.Poll :: :: call) :: call
    actions.set_status :: self, "control flow", "poll" :: call
    actions.mark_dirty :: self, cx :: call

fn set_wait(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    arcana_desktop.app.set_control_flow :: cx, (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call
    actions.set_status :: self, "control flow", "wait" :: call
    actions.mark_dirty :: self, cx :: call

fn set_wait_until(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let now = std.time.monotonic_now_ms :: :: call
    let deadline = now.value + 250
    arcana_desktop.app.set_control_flow :: cx, (arcana_desktop.types.ControlFlow.WaitUntil :: deadline :: call) :: call
    actions.set_status :: self, "control flow", "wait until +250ms" :: call
    actions.mark_dirty :: self, cx :: call

fn exit_now(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    actions.set_status :: self, "exit", "requested clean shutdown" :: call
    arcana_desktop.app.request_exit :: cx, 0 :: call

export fn button_count() -> Int:
    return 24

export fn button_label(id: Int) -> Str:
    if id == 0:
        return "Prev Page"
    if id == 1:
        return "Next Page"
    if id == 2:
        return "Profile"
    if id == 3:
        return "Fullscreen"
    if id == 4:
        return "Maximize"
    if id == 5:
        return "Minimize"
    if id == 6:
        return "Resizable"
    if id == 7:
        return "Decorated"
    if id == 8:
        return "Transparent"
    if id == 9:
        return "Topmost"
    if id == 10:
        return "Theme"
    if id == 11:
        return "Cursor Vis"
    if id == 12:
        return "Cursor Icon"
    if id == 13:
        return "Cursor Grab"
    if id == 14:
        return "Center Cursor"
    if id == 15:
        return "Text Input"
    if id == 16:
        return "Set Comp"
    if id == 17:
        return "Clear Comp"
    if id == 18:
        return "Copy Text"
    if id == 19:
        return "Copy Bytes"
    if id == 20:
        return "Wake"
    if id == 21:
        return "Attention"
    if id == 22:
        return "Second Win"
    return "Exit"

export fn button_rect(read view: layout.ViewLayout, id: Int) -> layout.Rect:
    return layout.button_rect :: view, id :: call

export fn button_at(read view: layout.ViewLayout, point: (Int, Int)) -> Int:
    let total = actions.button_count :: :: call
    let mut id = 0
    while id < total:
        if layout.button_hit :: (actions.button_rect :: view, id :: call), point :: call:
            return id
        id += 1
    return -1

export fn perform(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, id: Int):
    if id == 0:
        self.page_index = actions.next_page_index :: self.page_index, -1 :: call
        self.page_scroll = 0
        actions.sync_main_title :: self, cx :: call
        let page = pages.title :: self.page_index :: call
        if self.ui_smoke_mode:
            std.io.print_line[Str] :: ("page=" + page) :: call
            std.io.flush_stdout :: :: call
        actions.set_status :: self, "page", page :: call
        actions.mark_dirty :: self, cx :: call
        return
    if id == 1:
        self.page_index = actions.next_page_index :: self.page_index, 1 :: call
        self.page_scroll = 0
        actions.sync_main_title :: self, cx :: call
        let page = pages.title :: self.page_index :: call
        if self.ui_smoke_mode:
            std.io.print_line[Str] :: ("page=" + page) :: call
            std.io.flush_stdout :: :: call
        actions.set_status :: self, "page", page :: call
        actions.mark_dirty :: self, cx :: call
        return
    if id == 2:
        actions.apply_window_profile :: self, cx :: call
        return
    if id == 3:
        actions.toggle_fullscreen :: self, cx :: call
        return
    if id == 4:
        actions.toggle_maximized :: self, cx :: call
        return
    if id == 5:
        actions.minimize_now :: self, cx :: call
        return
    if id == 6:
        actions.toggle_resizable :: self, cx :: call
        return
    if id == 7:
        actions.toggle_decorated :: self, cx :: call
        return
    if id == 8:
        actions.toggle_transparent :: self, cx :: call
        return
    if id == 9:
        actions.toggle_topmost :: self, cx :: call
        return
    if id == 10:
        actions.cycle_theme :: self, cx :: call
        return
    if id == 11:
        actions.toggle_cursor_visible :: self, cx :: call
        return
    if id == 12:
        actions.cycle_cursor_icon :: self, cx :: call
        return
    if id == 13:
        actions.cycle_grab_mode :: self, cx :: call
        return
    if id == 14:
        actions.center_cursor :: self, cx :: call
        return
    if id == 15:
        actions.toggle_text_input :: self, cx :: call
        return
    if id == 16:
        actions.set_comp_area :: self, cx :: call
        return
    if id == 17:
        actions.clear_comp_area :: self, cx :: call
        return
    if id == 18:
        actions.copy_text :: self, cx :: call
        return
    if id == 19:
        actions.copy_bytes :: self, cx :: call
        return
    if id == 20:
        actions.send_wake :: self, cx :: call
        return
    if id == 21:
        actions.toggle_attention :: self, cx :: call
        return
    if id == 22:
        actions.open_second_window :: self, cx :: call
        return
    actions.exit_now :: self, cx :: call

export fn run_smoke_setup(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    actions.apply_window_profile :: self, cx :: call
    actions.copy_text :: self, cx :: call
    actions.copy_bytes :: self, cx :: call
    actions.send_wake :: self, cx :: call
    self.smoke_done = true
    actions.set_status :: self, "smoke", "waiting for wake and redraw" :: call
    actions.mark_dirty :: self, cx :: call

export fn set_poll_flow(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    actions.set_poll :: self, cx :: call

export fn set_wait_flow(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    actions.set_wait :: self, cx :: call

export fn set_wait_until_flow(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    actions.set_wait_until :: self, cx :: call
