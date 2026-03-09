import winspell.input

export fn quit_requested(read win: Window) -> Bool:
    return winspell.input.key_pressed :: win, (winspell.input.key_code :: "escape" :: call) :: call

export fn toggle_fullscreen_pressed(read win: Window) -> Bool:
    return winspell.input.key_pressed :: win, (winspell.input.key_code :: "f" :: call) :: call

export fn left_click(read win: Window) -> Bool:
    return winspell.input.mouse_pressed :: win, (winspell.input.mouse_button_code :: "left" :: call) :: call