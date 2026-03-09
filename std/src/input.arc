import std.kernel.gfx

export fn key_code(name: Str) -> Int:
    return std.kernel.gfx.input_key_code :: name :: call

export fn key_down(read win: Window, key: Int) -> Bool:
    return std.kernel.gfx.input_key_down :: win, key :: call

export fn key_pressed(read win: Window, key: Int) -> Bool:
    return std.kernel.gfx.input_key_pressed :: win, key :: call

export fn key_released(read win: Window, key: Int) -> Bool:
    return std.kernel.gfx.input_key_released :: win, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.kernel.gfx.input_mouse_button_code :: name :: call

export fn mouse_pos(read win: Window) -> (Int, Int):
    return std.kernel.gfx.input_mouse_pos :: win :: call

export fn mouse_down(read win: Window, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_down :: win, button :: call

export fn mouse_pressed(read win: Window, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_pressed :: win, button :: call

export fn mouse_released(read win: Window, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_released :: win, button :: call

export fn mouse_wheel_y(read win: Window) -> Int:
    return std.kernel.gfx.input_mouse_wheel_y :: win :: call

export fn mouse_in_window(read win: Window) -> Bool:
    return std.kernel.gfx.input_mouse_in_window :: win :: call
