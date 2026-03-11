import std.input
import std.events
use std.window.Window

export fn key_code(name: Str) -> Int:
    return std.input.key_code :: name :: call

export fn key_down(edit win: Window, key: Int) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.key_down :: frame, key :: call

export fn key_pressed(edit win: Window, key: Int) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.key_pressed :: frame, key :: call

export fn key_released(edit win: Window, key: Int) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.key_released :: frame, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.input.mouse_button_code :: name :: call

export fn mouse_pos(edit win: Window) -> (Int, Int):
    let frame = std.events.pump :: win :: call
    return std.input.mouse_pos :: frame :: call

export fn mouse_down(edit win: Window, button: Int) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.mouse_down :: frame, button :: call

export fn mouse_pressed(edit win: Window, button: Int) -> Bool:
    let frame = std.events.pump :: win :: call
    return std.input.mouse_pressed :: frame, button :: call

export fn mouse_wheel_y(edit win: Window) -> Int:
    let frame = std.events.pump :: win :: call
    return std.input.mouse_wheel_y :: frame :: call
