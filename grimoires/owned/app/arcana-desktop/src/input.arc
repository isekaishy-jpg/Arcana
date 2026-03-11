import std.input
import std.events

export fn begin_frame(read win: Window) -> std.events.AppFrame:
    return std.input.begin_frame :: win :: call

export fn key_code(name: Str) -> Int:
    return std.input.key_code :: name :: call

export fn key_down(read frame: std.events.AppFrame, key: Int) -> Bool:
    return std.input.key_down :: frame, key :: call

export fn key_pressed(read frame: std.events.AppFrame, key: Int) -> Bool:
    return std.input.key_pressed :: frame, key :: call

export fn key_released(read frame: std.events.AppFrame, key: Int) -> Bool:
    return std.input.key_released :: frame, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.input.mouse_button_code :: name :: call

export fn mouse_pos(read frame: std.events.AppFrame) -> (Int, Int):
    return std.input.mouse_pos :: frame :: call

export fn mouse_down(read frame: std.events.AppFrame, button: Int) -> Bool:
    return std.input.mouse_down :: frame, button :: call

export fn mouse_pressed(read frame: std.events.AppFrame, button: Int) -> Bool:
    return std.input.mouse_pressed :: frame, button :: call

export fn mouse_released(read frame: std.events.AppFrame, button: Int) -> Bool:
    return std.input.mouse_released :: frame, button :: call

export fn mouse_in_window(read frame: std.events.AppFrame) -> Bool:
    return std.input.mouse_in_window :: frame :: call

export fn mouse_wheel_y(read frame: std.events.AppFrame) -> Int:
    return std.input.mouse_wheel_y :: frame :: call
