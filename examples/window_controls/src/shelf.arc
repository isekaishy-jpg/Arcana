import winspell.window
import winspell.draw
import winspell.input
import std.concurrent

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Window Controls", 520, 320 :: call
    let key_c = winspell.input.key_code :: "c" :: call
    let key_t = winspell.input.key_code :: "t" :: call
    let key_q = winspell.input.key_code :: "q" :: call
    let mut topmost = false
    let mut cursor_visible = true
    let bg_a = winspell.draw.rgb :: 15, 17, 23 :: call
    let bg_b = winspell.draw.rgb :: 24, 20, 32 :: call
    let mut use_a = true

    while winspell.window.alive :: win :: call:
        if winspell.input.key_pressed :: win, key_q :: call:
            winspell.window.close :: win :: call
        if winspell.input.key_pressed :: win, key_c :: call:
            cursor_visible = not cursor_visible
            winspell.window.set_cursor_visible :: win, cursor_visible :: call
        if winspell.input.key_pressed :: win, key_t :: call:
            topmost = not topmost
            winspell.window.set_topmost :: win, topmost :: call
            use_a = not use_a

        if use_a:
            winspell.draw.fill :: win, bg_a :: call
        else:
            winspell.draw.fill :: win, bg_b :: call
        winspell.draw.present :: win :: call
        16 :: :: std.concurrent.sleep

    return 0