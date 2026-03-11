import std.ecs
import std.io
import winspell.window
import winspell.draw
import spell_events.keybinds
import shared_core

use std.io as io

fn tick_system() -> Int:
    let entity = std.ecs.spawn :: :: call
    std.ecs.set_component_at[Int] :: entity, 1 :: call
    let phase_count = std.ecs.step_update :: :: call
    return phase_count

fn main() -> Int:
    let mut win = winspell.window.open :: "Workspace ECS", 200, 120 :: call
    let phase_count = tick_system :: :: call
    io.print[Int] :: (phase_count + (shared_core.shared_seed :: :: call)) :: call
    while winspell.window.alive :: win :: call:
        if spell_events.keybinds.quit_requested :: win :: call:
            winspell.window.close :: win :: call
        winspell.draw.fill :: win, (winspell.draw.rgb :: 18, 20, 26 :: call) :: call
        winspell.draw.present :: win :: call
        winspell.window.close :: win :: call
    return 0