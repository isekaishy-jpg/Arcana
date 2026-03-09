import winspell.window
import winspell.draw
import winspell.input
import std.concurrent

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Image Viewer", 640, 360 :: call
    let _img = winspell.draw.image_load :: "examples/assets/arcana_demo.png" :: call
    let esc = winspell.input.key_code :: "escape" :: call
    let bg = winspell.draw.rgb :: 14, 16, 22 :: call

    while winspell.window.alive :: win :: call:
        if winspell.input.key_pressed :: win, esc :: call:
            winspell.window.close :: win :: call
        winspell.draw.fill :: win, bg :: call
        winspell.draw.present :: win :: call
        16 :: :: std.concurrent.sleep

    return 0