import std.input
import std.events
use std.window.Window

export fn quit_requested(edit win: Window) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.key_pressed :: frame, (std.input.key_code :: "escape" :: call) :: call

export fn toggle_fullscreen_pressed(edit win: Window) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.key_pressed :: frame, (std.input.key_code :: "f" :: call) :: call

export fn left_click(edit win: Window) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.mouse_pressed :: frame, (std.input.mouse_button_code :: "left" :: call) :: call
