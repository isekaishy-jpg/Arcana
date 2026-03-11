import std.kernel.gfx
import std.events

export fn begin_frame(read win: Window) -> std.events.AppFrame:
    return std.events.pump :: win :: call

export fn key_code(name: Str) -> Int:
    return std.kernel.gfx.input_key_code :: name :: call

export fn key_down(read frame: std.events.AppFrame, key: Int) -> Bool:
    return std.kernel.gfx.input_key_down :: frame.input, key :: call

export fn key_pressed(read frame: std.events.AppFrame, key: Int) -> Bool:
    return std.kernel.gfx.input_key_pressed :: frame.input, key :: call

export fn key_released(read frame: std.events.AppFrame, key: Int) -> Bool:
    return std.kernel.gfx.input_key_released :: frame.input, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.kernel.gfx.input_mouse_button_code :: name :: call

export fn mouse_pos(read frame: std.events.AppFrame) -> (Int, Int):
    return std.kernel.gfx.input_mouse_pos :: frame.input :: call

export fn mouse_down(read frame: std.events.AppFrame, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_down :: frame.input, button :: call

export fn mouse_pressed(read frame: std.events.AppFrame, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_pressed :: frame.input, button :: call

export fn mouse_released(read frame: std.events.AppFrame, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_released :: frame.input, button :: call

export fn mouse_wheel_y(read frame: std.events.AppFrame) -> Int:
    return std.kernel.gfx.input_mouse_wheel_y :: frame.input :: call

export fn mouse_in_window(read frame: std.events.AppFrame) -> Bool:
    return std.kernel.gfx.input_mouse_in_window :: frame.input :: call
