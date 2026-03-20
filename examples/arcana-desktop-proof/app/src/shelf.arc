import actions
import arcana_desktop.app
import arcana_desktop.ecs
import arcana_desktop.input
import arcana_desktop.loop
import arcana_desktop.monitor
import arcana_desktop.types
import arcana_desktop.window
import demo_types
import layout
import pages
import render
import std.args
import std.events
import std.io
import std.option
import std.result
import std.text
use std.option.Option
use std.window.Window

fn has_flag(flag: Str) -> Bool:
    let total = std.args.count :: :: call
    let mut index = 0
    while index < total:
        if (std.args.get :: index :: call) == flag:
            return true
        index += 1
    return false

fn default_demo(smoke_mode: Bool) -> demo_types.Demo:
    let fixed = arcana_desktop.types.FixedStepConfig :: tick_hz = 30, max_steps = 3 :: call
    let adapter = arcana_desktop.ecs.adapter :: fixed :: call
    let start = arcana_desktop.loop.frame_start :: :: call
    let mut demo = demo_types.Demo :: smoke_mode = smoke_mode, ui_smoke_mode = false, smoke_done = false :: call
    demo.smoke_printed = false
    demo.exercise_second_window = false
    demo.checksum = 0
    demo.line_count = 0
    demo.page_index = 0
    demo.page_scroll = 0
    demo.dirty = true
    demo.redraw_count = 0
    demo.wake_count = 0
    demo.close_requests = 0
    demo.key_events = 0
    demo.mouse_events = 0
    demo.text_events = 0
    demo.raw_motion_total = 0
    demo.second_window_id = -1
    demo.second_window_seen = false
    demo.second_window_dirty = false
    demo.attention_on = false
    demo.adapter_total = 0
    demo.status_head = "starting"
    demo.status_tail = "creating desktop proof state"
    demo.last_event = "-"
    demo.last_key = "-"
    demo.last_mouse = "-"
    demo.last_text = "-"
    demo.last_comp = "-"
    demo.last_drop = "-"
    demo.last_clipboard = "-"
    demo.last_monitor = "-"
    demo.last_window = "-"
    demo.pending_wake_note = ""
    demo.pending_wake = false
    demo.mouse_pos = (0, 0)
    demo.mouse_inside = false
    demo.hover_button_id = -1
    demo.mouse_wheel_y = 0
    demo.last_frame_start = start
    demo.adapter = adapter
    return demo

fn modifier_text(flags: Int) -> Str:
    let mut out = ""
    if arcana_desktop.input.modifier_shift :: flags :: call:
        out = out + "S"
    if arcana_desktop.input.modifier_ctrl :: flags :: call:
        out = out + "C"
    if arcana_desktop.input.modifier_alt :: flags :: call:
        out = out + "A"
    if arcana_desktop.input.modifier_meta :: flags :: call:
        out = out + "M"
    if out == "":
        return "-"
    return out

fn location_text(location: Int) -> Str:
    if location == (arcana_desktop.input.key_location_left :: :: call):
        return "left"
    if location == (arcana_desktop.input.key_location_right :: :: call):
        return "right"
    if location == (arcana_desktop.input.key_location_numpad :: :: call):
        return "numpad"
    return "standard"

fn repeated_text(read ev: std.events.KeyEvent) -> Str:
    if arcana_desktop.input.key_repeated :: ev :: call:
        return "repeat"
    return "single"

fn key_summary(read ev: std.events.KeyEvent) -> Str:
    let mut text = "key=" + (std.text.from_int :: ev.key :: call)
    text = text + " phys=" + (std.text.from_int :: (arcana_desktop.input.key_physical :: ev :: call) :: call)
    text = text + " logical=" + (std.text.from_int :: (arcana_desktop.input.key_logical :: ev :: call) :: call)
    text = text + " loc=" + (location_text :: (arcana_desktop.input.key_location :: ev :: call) :: call)
    text = text + " text=" + (arcana_desktop.input.key_text :: ev :: call)
    text = text + " mods=" + (modifier_text :: ev.meta.modifiers :: call)
    text = text + " " + (repeated_text :: ev :: call)
    return text

fn button_summary(prefix: Str, read ev: std.events.MouseButtonEvent) -> Str:
    let mut text = prefix
    text = text + " b=" + (std.text.from_int :: ev.button :: call)
    text = text + " @" + (std.text.from_int :: ev.position.0 :: call)
    text = text + "," + (std.text.from_int :: ev.position.1 :: call)
    text = text + " mods=" + (modifier_text :: ev.modifiers :: call)
    return text

fn move_summary(read ev: std.events.MouseMoveEvent) -> Str:
    let mut text = "move " + (std.text.from_int :: ev.position.0 :: call)
    text = text + "," + (std.text.from_int :: ev.position.1 :: call)
    text = text + " mods=" + (modifier_text :: ev.modifiers :: call)
    return text

fn wheel_summary(read ev: std.events.MouseWheelEvent) -> Str:
    let mut text = "wheel " + (std.text.from_int :: ev.delta.1 :: call)
    text = text + " mods=" + (modifier_text :: ev.modifiers :: call)
    return text

fn composition_summary(prefix: Str, read ev: std.events.TextCompositionEvent) -> Str:
    return prefix + " text=" + ev.text + " caret=" + (std.text.from_int :: ev.caret :: call)

fn absolute(value: Int) -> Int:
    if value < 0:
        return 0 - value
    return value

fn smoke_ready(read self: demo_types.Demo) -> Bool:
    if not self.smoke_done:
        return false
    if self.wake_count <= 0:
        return false
    return true

fn smoke_score(read self: demo_types.Demo, read win: std.window.Window) -> Int:
    let mut total = 0
    let min_size = arcana_desktop.window.min_size :: win :: call
    let max_size = arcana_desktop.window.max_size :: win :: call
    let theme_override = arcana_desktop.window.theme_override :: win :: call
    let cursor_icon = arcana_desktop.window.cursor_icon :: win :: call
    let cursor_position = arcana_desktop.window.cursor_position :: win :: call
    let text_enabled = arcana_desktop.text_input.enabled :: win :: call
    let composition_area = arcana_desktop.text_input.composition_area :: win :: call
    if min_size.0 == 900:
        if min_size.1 == 620:
            total += 1
    if max_size.0 == 1540:
        if max_size.1 == 1040:
            total += 2
    if theme_override == (arcana_desktop.types.WindowThemeOverride.Dark :: :: call):
        total += 4
    if cursor_icon == (arcana_desktop.types.CursorIcon.Hand :: :: call):
        total += 8
    if cursor_position.0 == 160:
        if cursor_position.1 == 128:
            total += 16
    if text_enabled:
        total += 32
    if composition_area.active:
        total += 64
    if std.text.contains :: self.last_clipboard, "bytes:" :: call:
        total += 128
    if self.wake_count > 0:
        total += 512
    return total

fn mark_dirty(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let _ = cx
    if self.dirty:
        return
    self.dirty = true

fn next_hover_button(read self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> Int:
    if not self.mouse_inside:
        return -1
    let size = arcana_desktop.window.size :: cx.runtime.main_window :: call
    return actions.button_at :: (layout.for_window :: size :: call), self.mouse_pos :: call

fn refresh_main_hover(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> Bool:
    let next = next_hover_button :: self, cx :: call
    if self.hover_button_id == next:
        return false
    self.hover_button_id = next
    return true

fn mark_second_window_dirty(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> Bool:
    let _ = cx
    if self.second_window_id < 0:
        return false
    self.second_window_dirty = true
    return true

fn refresh_secondary_window(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    let _ = mark_second_window_dirty :: self, cx :: call

fn request_second_window_redraw(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    if self.second_window_id < 0:
        request_second_window_redraw_missing :: self :: call
        return
    let id = arcana_desktop.types.WindowId :: value = self.second_window_id :: call
    return match (arcana_desktop.app.window_for_id :: cx, id :: call):
        Option.Some(win) => request_second_window_redraw_window :: win :: call
        Option.None => request_second_window_redraw_missing :: self :: call

fn request_second_window_redraw_window(take win: Window):
    let mut win = win
    arcana_desktop.window.request_redraw :: win :: call

fn request_second_window_redraw_missing(edit self: demo_types.Demo):
    self.second_window_dirty = false

fn aux_window_note(edit self: demo_types.Demo, window_id: Int, summary: Str):
    self.last_window = "window " + (std.text.from_int :: window_id :: call)
    self.status_head = "aux window"
    self.status_tail = summary

fn on_resumed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
    self.checksum = actions.button_count :: :: call
    self.line_count = pages.count :: :: call
    self.last_monitor = "primary " + (arcana_desktop.monitor.primary :: :: call).name
    self.last_window = "main " + (std.text.from_int :: (arcana_desktop.window.id :: cx.runtime.main_window :: call).value :: call)
    self.status_head = "resumed"
    if self.smoke_mode:
        self.status_tail = "running deterministic smoke setup"
        actions.run_smoke_setup :: self, cx :: call
        return
    if self.exercise_second_window:
        self.status_tail = "opening second window through the public desktop facade"
        actions.open_second_window :: self, cx :: call
        return
    self.status_tail = "interactive showcase ready"
    actions.sync_main_title :: self, cx :: call
    mark_dirty :: self, cx :: call

fn on_close_requested(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    let id = target.window_id.value
    self.close_requests += 1
    self.last_event = "WindowCloseRequested"
    self.last_window = "close requested for " + (std.text.from_int :: id :: call)
    self.status_head = "close request"
    self.status_tail = "clean exit requested by window shell"
    let handled = arcana_desktop.app.close_target_window_or_exit_main :: cx, target :: call
    return match handled:
        Result.Ok(flow) => on_close_requested_ok :: self, (demo_types.CloseOutcome :: is_main_window = target.is_main_window, flow = flow :: call) :: call
        Result.Err(err) => on_close_requested_failed :: self, cx, err :: call

fn on_close_requested_ok(edit self: demo_types.Demo, read outcome: demo_types.CloseOutcome) -> arcana_desktop.types.ControlFlow:
    if not outcome.is_main_window:
        self.second_window_id = -1
        self.second_window_seen = false
        self.second_window_dirty = false
        self.status_tail = "secondary window closed through arcana_desktop.app.close_target_window_or_exit_main"
        self.dirty = true
    return outcome.flow

fn on_close_requested_failed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, err: Str) -> arcana_desktop.types.ControlFlow:
    self.status_tail = "close request failed: " + err
    mark_dirty :: self, cx :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_redraw(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowRedrawRequested"
    return on_redraw_resolved :: self, cx, target :: call

fn on_redraw_resolved(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    let id = target.window_id.value
    let target_window = arcana_desktop.app.require_target_window :: cx, target :: call
    return match target_window:
        Result.Ok(win) => on_redraw_resolved_window :: self, cx, (demo_types.TargetRedraw :: target = target, win = win :: call) :: call
        Result.Err(_) => on_redraw_missing_window :: self, cx, id :: call

fn on_redraw_resolved_window(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read redraw: demo_types.TargetRedraw) -> arcana_desktop.types.ControlFlow:
    let mut win = redraw.win
    let id = (arcana_desktop.window.id :: win :: call).value
    if not redraw.target.is_main_window:
        self.second_window_seen = true
        self.last_window = "secondary redraw " + (std.text.from_int :: id :: call)
        let drew_secondary = on_redraw_secondary :: self, id, win :: call
        if drew_secondary:
            self.second_window_dirty = false
            return cx.control.control_flow
        if id == self.second_window_id:
            self.second_window_id = -1
            self.second_window_seen = false
            self.second_window_dirty = false
        return cx.control.control_flow
    self.redraw_count += 1
    render.draw_main :: self, win :: call
    self.dirty = false
    if self.smoke_mode:
        let score = smoke_score :: self, win :: call
        if smoke_ready :: self :: call:
            if not self.smoke_printed:
                std.io.print_line[Str] :: ("controls=" + (std.text.from_int :: self.checksum :: call)) :: call
                std.io.print_line[Str] :: ("pages=" + (std.text.from_int :: self.line_count :: call)) :: call
                std.io.print_line[Str] :: ("smoke_score=" + (std.text.from_int :: score :: call)) :: call
                std.io.flush_stdout :: :: call
                self.smoke_printed = true
            arcana_desktop.app.request_exit :: cx, 0 :: call
    return cx.control.control_flow

fn on_redraw_missing_window(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, id: Int) -> arcana_desktop.types.ControlFlow:
    if id == self.second_window_id:
        self.second_window_id = -1
        self.second_window_seen = false
        self.second_window_dirty = false
    return cx.control.control_flow

fn on_redraw_secondary(edit self: demo_types.Demo, id: Int, read win: Window) -> Bool:
    let mut win = win
    if (arcana_desktop.window.id :: win :: call).value != id:
        return false
    render.draw_secondary :: self, win :: call
    return true

fn on_window_resized(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.WindowResizeEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowResized"
    self.last_window = "size " + (std.text.from_int :: ev.size.0 :: call) + "x" + (std.text.from_int :: ev.size.1 :: call)
    if ev.window_id == self.second_window_id:
        refresh_secondary_window :: self, cx :: call
        return cx.control.control_flow
    let _ = refresh_main_hover :: self, cx :: call
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_window_moved(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.WindowMoveEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowMoved"
    self.last_window = "moved " + (std.text.from_int :: ev.position.0 :: call) + "," + (std.text.from_int :: ev.position.1 :: call)
    if ev.window_id == self.second_window_id:
        refresh_secondary_window :: self, cx :: call
        return cx.control.control_flow
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_window_focused(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.WindowFocusEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowFocused"
    if ev.focused:
        self.last_window = "focus gained"
    else:
        self.last_window = "focus lost"
    if ev.window_id == self.second_window_id:
        refresh_secondary_window :: self, cx :: call
        return cx.control.control_flow
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_window_scale(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.WindowScaleFactorEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowScaleFactorChanged"
    self.last_monitor = "scale " + (std.text.from_int :: ev.scale_factor_milli :: call)
    if ev.window_id == self.second_window_id:
        refresh_secondary_window :: self, cx :: call
        return cx.control.control_flow
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_window_theme(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.WindowThemeEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "WindowThemeChanged"
    self.last_monitor = "theme code " + (std.text.from_int :: ev.theme_code :: call)
    if ev.window_id == self.second_window_id:
        refresh_secondary_window :: self, cx :: call
        return cx.control.control_flow
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_aux_window_event(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    return match target.event:
        std.events.AppEvent.WindowRedrawRequested(_) => on_redraw :: self, cx, target :: call
        std.events.AppEvent.WindowCloseRequested(_) => on_close_requested :: self, cx, target :: call
        std.events.AppEvent.WindowResized(ev) => on_window_resized :: self, cx, ev :: call
        std.events.AppEvent.WindowMoved(ev) => on_window_moved :: self, cx, ev :: call
        std.events.AppEvent.WindowFocused(ev) => on_window_focused :: self, cx, ev :: call
        std.events.AppEvent.WindowScaleFactorChanged(ev) => on_window_scale :: self, cx, ev :: call
        std.events.AppEvent.WindowThemeChanged(ev) => on_window_theme :: self, cx, ev :: call
        std.events.AppEvent.KeyDown(ev) => on_aux_key_down :: self, cx, ev :: call
        std.events.AppEvent.KeyUp(ev) => on_aux_key_up :: self, cx, ev :: call
        std.events.AppEvent.MouseDown(ev) => on_aux_mouse_down :: self, cx, ev :: call
        std.events.AppEvent.MouseUp(ev) => on_aux_mouse_up :: self, cx, ev :: call
        std.events.AppEvent.MouseMove(ev) => on_aux_mouse_move :: self, cx, ev :: call
        std.events.AppEvent.MouseWheel(ev) => on_aux_mouse_wheel :: self, cx, ev :: call
        std.events.AppEvent.MouseEntered(id) => on_aux_mouse_enter :: self, cx, id :: call
        std.events.AppEvent.MouseLeft(id) => on_aux_mouse_left :: self, cx, id :: call
        std.events.AppEvent.TextInput(ev) => on_aux_text_input :: self, cx, ev :: call
        std.events.AppEvent.TextCompositionStarted(id) => on_aux_comp_started :: self, cx, id :: call
        std.events.AppEvent.TextCompositionUpdated(ev) => on_aux_comp_updated :: self, cx, ev :: call
        std.events.AppEvent.TextCompositionCommitted(ev) => on_aux_comp_committed :: self, cx, ev :: call
        std.events.AppEvent.TextCompositionCancelled(id) => on_aux_comp_cancelled :: self, cx, id :: call
        std.events.AppEvent.FileDropped(ev) => on_aux_file_drop :: self, cx, ev :: call
        _ => cx.control.control_flow

fn on_aux_key_down(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.KeyEvent) -> arcana_desktop.types.ControlFlow:
    self.key_events += 1
    self.last_event = "KeyDown"
    self.last_key = key_summary :: ev :: call
    aux_window_note :: self, ev.window_id, "key down on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_key_up(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.KeyEvent) -> arcana_desktop.types.ControlFlow:
    self.key_events += 1
    self.last_event = "KeyUp"
    self.last_key = key_summary :: ev :: call
    aux_window_note :: self, ev.window_id, "key up on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_mouse_down(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.last_event = "MouseDown"
    self.last_mouse = button_summary :: "down", ev :: call
    aux_window_note :: self, ev.window_id, "mouse down on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_mouse_up(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.last_event = "MouseUp"
    self.last_mouse = button_summary :: "up", ev :: call
    aux_window_note :: self, ev.window_id, "mouse up on auxiliary window" :: call
    if self.exercise_second_window:
        if ev.window_id == self.second_window_id:
            std.io.print_line[Str] :: ("second_window=click:" + (std.text.from_int :: ev.window_id :: call)) :: call
            std.io.flush_stdout :: :: call
            arcana_desktop.app.request_exit :: cx, 0 :: call
            return arcana_desktop.types.ControlFlow.Wait :: :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_mouse_move(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseMoveEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.last_event = "MouseMove"
    self.last_mouse = move_summary :: ev :: call
    aux_window_note :: self, ev.window_id, "mouse move on auxiliary window" :: call
    let _ = cx
    return cx.control.control_flow

fn on_aux_mouse_wheel(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseWheelEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.last_event = "MouseWheel"
    self.last_mouse = wheel_summary :: ev :: call
    aux_window_note :: self, ev.window_id, "mouse wheel on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_mouse_enter(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, window_id: Int) -> arcana_desktop.types.ControlFlow:
    self.last_event = "MouseEntered"
    self.last_mouse = "entered"
    aux_window_note :: self, window_id, "mouse entered auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_mouse_left(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, window_id: Int) -> arcana_desktop.types.ControlFlow:
    self.last_event = "MouseLeft"
    self.last_mouse = "left"
    aux_window_note :: self, window_id, "mouse left auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_text_input(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.TextInputEvent) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextInput"
    self.last_text = ev.text
    aux_window_note :: self, ev.window_id, "text input on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_comp_started(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, window_id: Int) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionStarted"
    self.last_comp = "composition started"
    aux_window_note :: self, window_id, "composition started on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_comp_updated(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.TextCompositionEvent) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionUpdated"
    self.last_comp = composition_summary :: "update", ev :: call
    aux_window_note :: self, ev.window_id, "composition updated on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_comp_committed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.TextCompositionEvent) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionCommitted"
    self.last_comp = composition_summary :: "commit", ev :: call
    aux_window_note :: self, ev.window_id, "composition committed on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_comp_cancelled(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, window_id: Int) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionCancelled"
    self.last_comp = "composition cancelled"
    aux_window_note :: self, window_id, "composition cancelled on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_file_drop(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.FileDropEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "FileDropped"
    self.last_drop = ev.path
    aux_window_note :: self, ev.window_id, "file dropped on auxiliary window" :: call
    refresh_secondary_window :: self, cx :: call
    return cx.control.control_flow

fn on_aux_device_event(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    return match target.event:
        std.events.AppEvent.RawMouseMotion(ev) => on_aux_raw_mouse :: self, cx, ev :: call
        _ => cx.control.control_flow

fn on_aux_raw_mouse(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.RawMouseMotionEvent) -> arcana_desktop.types.ControlFlow:
    self.raw_motion_total += (absolute :: ev.delta.0 :: call) + (absolute :: ev.delta.1 :: call)
    self.last_event = "RawMouseMotion"
    self.last_mouse = "raw " + (std.text.from_int :: ev.delta.0 :: call) + "," + (std.text.from_int :: ev.delta.1 :: call)
    aux_window_note :: self, ev.window_id, "raw mouse motion on auxiliary window" :: call
    let _ = cx
    return cx.control.control_flow

fn handle_shortcut(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.KeyEvent) -> Bool:
    let key = ev.key
    if key == (arcana_desktop.input.key_code :: "Escape" :: call):
        actions.perform :: self, cx, 23 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "Q" :: call):
        actions.perform :: self, cx, 0 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "E" :: call):
        actions.perform :: self, cx, 1 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "O" :: call):
        actions.perform :: self, cx, 2 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "F" :: call):
        actions.perform :: self, cx, 3 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "X" :: call):
        actions.perform :: self, cx, 4 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "M" :: call):
        actions.perform :: self, cx, 5 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "R" :: call):
        actions.perform :: self, cx, 6 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "D" :: call):
        actions.perform :: self, cx, 7 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "T" :: call):
        actions.perform :: self, cx, 8 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "P" :: call):
        actions.perform :: self, cx, 9 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "Y" :: call):
        actions.perform :: self, cx, 10 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "V" :: call):
        actions.perform :: self, cx, 11 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "I" :: call):
        actions.perform :: self, cx, 12 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "G" :: call):
        actions.perform :: self, cx, 13 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "U" :: call):
        actions.perform :: self, cx, 14 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "K" :: call):
        actions.perform :: self, cx, 15 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "J" :: call):
        actions.perform :: self, cx, 16 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "L" :: call):
        actions.perform :: self, cx, 17 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "C" :: call):
        actions.perform :: self, cx, 18 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "B" :: call):
        actions.perform :: self, cx, 19 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "W" :: call):
        actions.perform :: self, cx, 20 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "A" :: call):
        actions.perform :: self, cx, 21 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "N" :: call):
        actions.perform :: self, cx, 22 :: call
        return true
    if key == (arcana_desktop.input.key_code :: "F1" :: call):
        actions.set_poll_flow :: self, cx :: call
        return true
    if key == (arcana_desktop.input.key_code :: "F2" :: call):
        actions.set_wait_flow :: self, cx :: call
        return true
    if key == (arcana_desktop.input.key_code :: "F3" :: call):
        actions.set_wait_until_flow :: self, cx :: call
        return true
    return false

fn on_key_down(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.KeyEvent) -> arcana_desktop.types.ControlFlow:
    self.key_events += 1
    self.last_event = "KeyDown"
    self.last_key = key_summary :: ev :: call
    if handle_shortcut :: self, cx, ev :: call:
        return cx.control.control_flow
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_key_up(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.KeyEvent) -> arcana_desktop.types.ControlFlow:
    self.key_events += 1
    self.last_event = "KeyUp"
    self.last_key = key_summary :: ev :: call
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_mouse_down(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.mouse_pos = ev.position
    let _ = refresh_main_hover :: self, cx :: call
    self.last_event = "MouseDown"
    self.last_mouse = button_summary :: "down", ev :: call
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_mouse_up(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseButtonEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.mouse_pos = ev.position
    let _ = refresh_main_hover :: self, cx :: call
    self.last_event = "MouseUp"
    self.last_mouse = button_summary :: "up", ev :: call
    if ev.button == (arcana_desktop.input.mouse_button_code :: "Left" :: call):
        let size = arcana_desktop.window.size :: cx.runtime.main_window :: call
        let id = actions.button_at :: (layout.for_window :: size :: call), ev.position :: call
        if id >= 0:
            actions.perform :: self, cx, id :: call
            return cx.control.control_flow
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_mouse_move(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseMoveEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.mouse_pos = ev.position
    self.last_event = "MouseMove"
    self.last_mouse = move_summary :: ev :: call
    if refresh_main_hover :: self, cx :: call:
        mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_mouse_wheel(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.MouseWheelEvent) -> arcana_desktop.types.ControlFlow:
    self.mouse_events += 1
    self.mouse_wheel_y += ev.delta.1
    self.last_event = "MouseWheel"
    self.last_mouse = wheel_summary :: ev :: call
    self.page_scroll -= ev.delta.1
    if self.page_scroll < 0:
        self.page_scroll = 0
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_mouse_enter(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    self.mouse_inside = true
    let _ = refresh_main_hover :: self, cx :: call
    self.last_event = "MouseEntered"
    self.last_mouse = "entered"
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_mouse_left(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    self.mouse_inside = false
    self.hover_button_id = -1
    self.last_event = "MouseLeft"
    self.last_mouse = "left"
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_text_input(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.TextInputEvent) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextInput"
    self.last_text = ev.text
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_comp_started(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionStarted"
    self.last_comp = "composition started"
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_comp_updated(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.TextCompositionEvent) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionUpdated"
    self.last_comp = composition_summary :: "update", ev :: call
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_comp_committed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.TextCompositionEvent) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionCommitted"
    self.last_comp = composition_summary :: "commit", ev :: call
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_comp_cancelled(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    self.text_events += 1
    self.last_event = "TextCompositionCancelled"
    self.last_comp = "composition cancelled"
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_file_drop(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.FileDropEvent) -> arcana_desktop.types.ControlFlow:
    self.last_event = "FileDropped"
    self.last_drop = ev.path
    self.status_head = "file drop"
    self.status_tail = ev.path
    mark_dirty :: self, cx :: call
    return cx.control.control_flow

fn on_raw_mouse(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read ev: std.events.RawMouseMotionEvent) -> arcana_desktop.types.ControlFlow:
    self.raw_motion_total += (absolute :: ev.delta.0 :: call) + (absolute :: ev.delta.1 :: call)
    self.last_event = "RawMouseMotion"
    self.last_mouse = "raw " + (std.text.from_int :: ev.delta.0 :: call) + "," + (std.text.from_int :: ev.delta.1 :: call)
    return cx.control.control_flow

fn step_adapter(edit self: demo_types.Demo):
    let elapsed = arcana_desktop.loop.frame_elapsed_ms :: self.last_frame_start :: call
    self.last_frame_start = arcana_desktop.loop.frame_start :: :: call
    let mut adapter = self.adapter
    let stepped = adapter :: elapsed :: step_all
    self.adapter = adapter
    self.adapter_total += stepped

impl arcana_desktop.app.Application[demo_types.Demo] for demo_types.Demo:
    fn resumed(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
        on_resumed :: self, cx :: call

    fn suspended(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
        self.status_head = "suspended"
        self.status_tail = "application suspended"

    fn window_event(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
        if not (arcana_desktop.app.target_is_main_window :: target :: call):
            return on_aux_window_event :: self, cx, target :: call
        return match target.event:
            std.events.AppEvent.WindowRedrawRequested(_) => on_redraw :: self, cx, target :: call
            std.events.AppEvent.WindowCloseRequested(_) => on_close_requested :: self, cx, target :: call
            std.events.AppEvent.WindowResized(ev) => on_window_resized :: self, cx, ev :: call
            std.events.AppEvent.WindowMoved(ev) => on_window_moved :: self, cx, ev :: call
            std.events.AppEvent.WindowFocused(ev) => on_window_focused :: self, cx, ev :: call
            std.events.AppEvent.WindowScaleFactorChanged(ev) => on_window_scale :: self, cx, ev :: call
            std.events.AppEvent.WindowThemeChanged(ev) => on_window_theme :: self, cx, ev :: call
            std.events.AppEvent.KeyDown(ev) => on_key_down :: self, cx, ev :: call
            std.events.AppEvent.KeyUp(ev) => on_key_up :: self, cx, ev :: call
            std.events.AppEvent.MouseDown(ev) => on_mouse_down :: self, cx, ev :: call
            std.events.AppEvent.MouseUp(ev) => on_mouse_up :: self, cx, ev :: call
            std.events.AppEvent.MouseMove(ev) => on_mouse_move :: self, cx, ev :: call
            std.events.AppEvent.MouseWheel(ev) => on_mouse_wheel :: self, cx, ev :: call
            std.events.AppEvent.MouseEntered(_) => on_mouse_enter :: self, cx :: call
            std.events.AppEvent.MouseLeft(_) => on_mouse_left :: self, cx :: call
            std.events.AppEvent.TextInput(ev) => on_text_input :: self, cx, ev :: call
            std.events.AppEvent.TextCompositionStarted(_) => on_comp_started :: self, cx :: call
            std.events.AppEvent.TextCompositionUpdated(ev) => on_comp_updated :: self, cx, ev :: call
            std.events.AppEvent.TextCompositionCommitted(ev) => on_comp_committed :: self, cx, ev :: call
            std.events.AppEvent.TextCompositionCancelled(_) => on_comp_cancelled :: self, cx :: call
            std.events.AppEvent.FileDropped(ev) => on_file_drop :: self, cx, ev :: call
            _ => cx.control.control_flow

    fn device_event(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
        if not (arcana_desktop.app.target_is_main_window :: target :: call):
            return on_aux_device_event :: self, cx, target :: call
        return match target.event:
            std.events.AppEvent.RawMouseMotion(ev) => on_raw_mouse :: self, cx, ev :: call
            _ => cx.control.control_flow

    fn about_to_wait(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        step_adapter :: self :: call
        if self.dirty:
            arcana_desktop.window.request_redraw :: cx.runtime.main_window :: call
        if self.second_window_dirty:
            request_second_window_redraw :: self, cx :: call
        return cx.control.control_flow

    fn wake(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        self.wake_count += 1
        self.pending_wake = false
        self.last_event = "Wake"
        self.status_head = "wake"
        self.status_tail = self.pending_wake_note
        mark_dirty :: self, cx :: call
        return cx.control.control_flow

    fn exiting(edit self: demo_types.Demo, edit cx: arcana_desktop.types.AppContext):
        self.status_head = "exiting"
        self.status_tail = "desktop showcase exiting"

fn main() -> Int:
    let smoke_mode = has_flag :: "--smoke" :: call
    let ui_smoke_mode = has_flag :: "--ui-smoke" :: call
    let second_window_mode = has_flag :: "--exercise-second-window" :: call
    let mut cfg = arcana_desktop.app.default_app_config :: :: call
    cfg.window.title = "Arcana Desktop Proof"
    cfg.window.bounds.size = (1280, 760)
    cfg.window.bounds.position = (64, 52)
    cfg.window.bounds.min_size = (960, 640)
    let mut app = default_demo :: smoke_mode :: call
    app.ui_smoke_mode = ui_smoke_mode
    app.exercise_second_window = second_window_mode
    return arcana_desktop.app.run :: app, cfg :: call

