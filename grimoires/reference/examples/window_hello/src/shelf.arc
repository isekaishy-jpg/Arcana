import winspell.window
import winspell.draw

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Window", 320, 180 :: call
    let mut frames = 0
    let bg = winspell.draw.rgb :: 16, 18, 24 :: call

    while winspell.window.alive :: win :: call:
        winspell.draw.fill :: win, bg :: call
        winspell.draw.present :: win :: call
        frames += 1
        if frames > 10:
            winspell.window.close :: win :: call

    return 0