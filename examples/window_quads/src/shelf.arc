import winspell.window
import winspell.draw

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Quads", 400, 240 :: call
    let mut frames = 0
    let mut c = winspell.draw.rgb :: 12, 12, 18 :: call

    while winspell.window.alive :: win :: call:
        winspell.draw.fill :: win, c :: call
        winspell.draw.present :: win :: call
        frames += 1
        if frames % 2 == 0:
            c = winspell.draw.rgb :: 20, 26, 42 :: call
        else:
            c = winspell.draw.rgb :: 12, 12, 18 :: call
        if frames > 12:
            winspell.window.close :: win :: call

    return 0