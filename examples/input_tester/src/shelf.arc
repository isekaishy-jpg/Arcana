import winspell.window
import winspell.draw
import winspell.input
import std.concurrent

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Input Tester", 480, 300 :: call
    let key_space = winspell.input.key_code :: "space" :: call
    let key_escape = winspell.input.key_code :: "escape" :: call
    let btn_left = winspell.input.mouse_button_code :: "left" :: call

    let idle = winspell.draw.rgb :: 16, 18, 24 :: call
    let hot = winspell.draw.rgb :: 90, 220, 150 :: call
    let warn = winspell.draw.rgb :: 220, 90, 90 :: call
    let mut color = idle

    while winspell.window.alive :: win :: call:
        if winspell.input.key_pressed :: win, key_escape :: call:
            winspell.window.close :: win :: call

        if winspell.input.key_down :: win, key_space :: call:
            color = hot
        else:
            color = idle

        if winspell.input.mouse_down :: win, btn_left :: call:
            color = warn

        winspell.draw.fill :: win, color :: call
        winspell.draw.present :: win :: call
        16 :: :: std.concurrent.sleep

    return 0