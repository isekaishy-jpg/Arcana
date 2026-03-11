import std.canvas
import std.window
import std.input
import std.behaviors
import std.concurrent
import std.io
use std.canvas as canvas
use std.window as window
use std.input as input
use std.behaviors.step
use std.io as io

behavior[phase=startup, affinity=main] fn announce():
    "Arcana Pong Attract" :: :: io.print

async fn main() -> Int:
    let _ = step :: "startup" :: call
    let mut win = canvas.open :: "Arcana Pong Attract", 420, 240 :: call
    let esc = input.key_code :: "escape" :: call
    let mut frames = 0
    let a = canvas.rgb :: 12, 14, 20 :: call
    let b = canvas.rgb :: 20, 16, 30 :: call

    while canvas.alive :: win :: call:
        if input.key_pressed :: win, esc :: call:
            window.close :: win :: call
        if frames % 2 == 0:
            canvas.fill :: win, a :: call
        else:
            canvas.fill :: win, b :: call
        canvas.present :: win :: call
        frames += 1
        16 :: :: std.concurrent.sleep

    return 0
