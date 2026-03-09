import winspell.window
import scene
use scene.draw_scene

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Grimoire", 420, 240 :: call

    while winspell.window.alive :: win :: call:
        draw_scene :: win :: call

    return 0