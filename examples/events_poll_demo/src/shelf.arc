import winspell.window
import winspell.draw
import winspell.loop
import spell_events.router
import std.io
use std.io as io
use std.collections.list as list

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Events", 320, 180 :: call
    let mut runner = winspell.loop.fixed_runner :: 60 :: call
    let mut frames = 0
    while winspell.loop.should_run :: win :: call:
        let step = winspell.loop.fixed_runner_step :: runner, 16 :: call
        let queued = spell_events.router.drain :: win :: call
        let qn = queued :: :: list.len
        if qn > 0:
            qn :: :: io.print
        if winspell.window.resized :: win :: call:
            let size = winspell.window.size :: win :: call
            size.0 :: :: io.print
            size.1 :: :: io.print
        let _alpha = step.1
        let bg = winspell.draw.rgb :: 8, 10, 18 :: call
        winspell.draw.fill :: win, bg :: call
        winspell.draw.present :: win :: call
        frames += 1
        if frames > 4:
            winspell.window.close :: win :: call
    return 0
