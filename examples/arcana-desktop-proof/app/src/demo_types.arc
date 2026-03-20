import arcana_desktop.ecs
import arcana_desktop.types
import std.types.core
use std.window.Window

record ButtonRect:
    pos: (Int, Int)
    size: (Int, Int)

record CloseOutcome:
    is_main_window: Bool
    flow: arcana_desktop.types.ControlFlow

record TargetRedraw:
    target: arcana_desktop.types.TargetedEvent
    win: Window

record Demo:
    smoke_mode: Bool
    ui_smoke_mode: Bool
    exercise_second_window: Bool
    smoke_done: Bool
    smoke_printed: Bool
    checksum: Int
    line_count: Int
    page_index: Int
    page_scroll: Int
    dirty: Bool
    redraw_count: Int
    wake_count: Int
    close_requests: Int
    key_events: Int
    mouse_events: Int
    text_events: Int
    raw_motion_total: Int
    second_window_id: Int
    second_window_seen: Bool
    second_window_dirty: Bool
    attention_on: Bool
    adapter_total: Int
    status_head: Str
    status_tail: Str
    last_event: Str
    last_key: Str
    last_mouse: Str
    last_text: Str
    last_comp: Str
    last_drop: Str
    last_clipboard: Str
    last_monitor: Str
    last_window: Str
    pending_wake_note: Str
    pending_wake: Bool
    mouse_pos: (Int, Int)
    mouse_inside: Bool
    hover_button_id: Int
    mouse_wheel_y: Int
    last_frame_start: std.types.core.MonotonicTimeMs
    adapter: arcana_desktop.ecs.Adapter
