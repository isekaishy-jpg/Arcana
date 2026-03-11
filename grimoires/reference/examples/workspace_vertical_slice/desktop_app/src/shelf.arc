import winspell.window
import winspell.draw
import spell_events.router
import std.io
import shared_core

use std.io as io

fn desktop_probe() -> Int:
    let color = winspell.draw.rgb :: 30, 60, 90 :: call
    return color

fn main() -> Int:
    let mut win = winspell.window.open :: "Workspace Desktop", 240, 160 :: call
    let c = desktop_probe :: :: call
    let seed = shared_core.shared_seed :: :: call
    io.print[Int] :: (c + seed) :: call
    while winspell.window.alive :: win :: call:
        let _cnt = spell_events.router.count :: win :: call
        winspell.draw.fill :: win, c :: call
        winspell.draw.present :: win :: call
        winspell.window.close :: win :: call
    return 0