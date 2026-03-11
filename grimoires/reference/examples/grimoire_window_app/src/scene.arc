import winspell.draw

import palette
use palette.bg_color
use palette.accent_a
use palette.accent_b
use palette.title_color

export fn draw_scene(edit win: Window):
    let _a = accent_a :: :: call
    let _b = accent_b :: :: call
    let _t = title_color :: :: call
    let bg = bg_color :: :: call
    winspell.draw.fill :: win, bg :: call
    winspell.draw.present :: win :: call