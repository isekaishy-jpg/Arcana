import std.input

export fn key_code(name: Str) -> Int:
    return std.input.key_code :: name :: call

export fn key_down(read win: Window, key: Int) -> Bool:
    return std.input.key_down :: win, key :: call

export fn key_pressed(read win: Window, key: Int) -> Bool:
    return std.input.key_pressed :: win, key :: call

export fn key_released(read win: Window, key: Int) -> Bool:
    return std.input.key_released :: win, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.input.mouse_button_code :: name :: call

export fn mouse_pos(read win: Window) -> (Int, Int):
    return std.input.mouse_pos :: win :: call

export fn mouse_down(read win: Window, button: Int) -> Bool:
    return std.input.mouse_down :: win, button :: call

export fn mouse_pressed(read win: Window, button: Int) -> Bool:
    return std.input.mouse_pressed :: win, button :: call

export fn mouse_wheel_y(read win: Window) -> Int:
    return std.input.mouse_wheel_y :: win :: call