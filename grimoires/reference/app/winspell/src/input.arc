import std.input

export fn key_code(name: Str) -> Int:
    return std.input.key_code :: name :: call

export fn key_down(read win: Window, key: Int) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.key_down :: frame, key :: call

export fn key_pressed(read win: Window, key: Int) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.key_pressed :: frame, key :: call

export fn key_released(read win: Window, key: Int) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.key_released :: frame, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.input.mouse_button_code :: name :: call

export fn mouse_pos(read win: Window) -> (Int, Int):
    let frame = std.input.begin_frame :: win :: call
    return std.input.mouse_pos :: frame :: call

export fn mouse_down(read win: Window, button: Int) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.mouse_down :: frame, button :: call

export fn mouse_pressed(read win: Window, button: Int) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.mouse_pressed :: frame, button :: call

export fn mouse_wheel_y(read win: Window) -> Int:
    let frame = std.input.begin_frame :: win :: call
    return std.input.mouse_wheel_y :: frame :: call
