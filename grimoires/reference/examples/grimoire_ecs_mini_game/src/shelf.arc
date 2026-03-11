import winspell.window
import winspell.draw
import winspell.input
import winspell.loop
import spell_events.router
import std.ecs
import std.concurrent

use std.ecs as ecs

record Player:
    x: Int
    y: Int

record InputState:
    left: Bool
    right: Bool

system[phase=startup, affinity=main] fn boot():
    let e = ecs.spawn :: :: call
    let p = Player :: x = 10, y = 10 :: call
    ecs.set_component_at[Player] :: e, p :: call
    let i = InputState :: left = false, right = false :: call
    ecs.set_component[InputState] :: i :: call

system[phase=fixed_update, affinity=main] fn move_player(edit p: Player, read i: InputState):
    if i.left:
        p.x -= 1
    if i.right:
        p.x += 1

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana ECS Mini Game", 320, 200 :: call
    let mut runner = winspell.loop.fixed_runner :: 60 :: call
    let left = winspell.input.key_code :: "left" :: call
    let right = winspell.input.key_code :: "right" :: call
    let esc = winspell.input.key_code :: "escape" :: call
    let bg = winspell.draw.rgb :: 14, 18, 26 :: call

    ecs.step_startup :: :: call
    while winspell.loop.should_run :: win :: call:
        let _evs = spell_events.router.drain :: win :: call
        if winspell.input.key_pressed :: win, esc :: call:
            winspell.window.close :: win :: call

        let left_down = winspell.input.key_down :: win, left :: call
        let right_down = winspell.input.key_down :: win, right :: call
        let i = InputState :: left = left_down, right = right_down :: call
        ecs.set_component[InputState] :: i :: call

        let s = winspell.loop.fixed_runner_step :: runner, 16 :: call
        let mut n = s.0
        while n > 0:
            ecs.step_fixed_update :: :: call
            n -= 1

        winspell.draw.fill :: win, bg :: call
        winspell.draw.present :: win :: call
        16 :: :: std.concurrent.sleep

    return 0
