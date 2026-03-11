import std.input

export fn quit_requested(read win: Window) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.key_pressed :: frame, (std.input.key_code :: "escape" :: call) :: call

export fn toggle_fullscreen_pressed(read win: Window) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.key_pressed :: frame, (std.input.key_code :: "f" :: call) :: call

export fn left_click(read win: Window) -> Bool:
    let frame = std.input.begin_frame :: win :: call
    return std.input.mouse_pressed :: frame, (std.input.mouse_button_code :: "left" :: call) :: call
