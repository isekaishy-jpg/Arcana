import std.input

export fn quit_requested(read win: Window) -> Bool:
    return std.input.key_pressed :: win, (std.input.key_code :: "escape" :: call) :: call

export fn toggle_fullscreen_pressed(read win: Window) -> Bool:
    return std.input.key_pressed :: win, (std.input.key_code :: "f" :: call) :: call

export fn left_click(read win: Window) -> Bool:
    return std.input.mouse_pressed :: win, (std.input.mouse_button_code :: "left" :: call) :: call
