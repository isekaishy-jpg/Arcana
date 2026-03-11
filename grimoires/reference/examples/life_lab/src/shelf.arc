import std.canvas
import std.window
import std.input
import std.events
import std.collections.array
import std.concurrent
use std.canvas as canvas
use std.window as window
use std.input as input
use std.collections.array as array

fn idx(x: Int, y: Int, w: Int) -> Int:
    return (y * w) + x

fn main() -> Int:
    let w = 32
    let h = 20
    let mut grid = array.new[Int] :: w * h, 0 :: call
    let mut win = canvas.open :: "Arcana Life Lab", 512, 360 :: call
    let key_space = input.key_code :: "space" :: call
    let key_escape = input.key_code :: "escape" :: call
    let dark = canvas.rgb :: 12, 14, 18 :: call
    let light = canvas.rgb :: 96, 220, 150 :: call
    let mut tick = 0

    while canvas.alive :: win :: call:
        let frame = std.events.pump :: win :: call
        if input.key_pressed :: frame, key_escape :: call:
            window.close :: win :: call

        if input.key_pressed :: frame, key_space :: call:
            let mut y = 0
            while y < h:
                let mut x = 0
                while x < w:
                    let i = idx :: x, y, w :: call
                    if (x + y + tick) % 3 == 0:
                        grid[i] = 1
                    else:
                        grid[i] = 0
                    x += 1
                y += 1

        let mut alive = 0
        for v in grid:
            alive += v

        if alive > (w * h) / 3:
            canvas.fill :: win, light :: call
        else:
            canvas.fill :: win, dark :: call
        canvas.present :: win :: call
        tick += 1
        16 :: :: std.concurrent.sleep

    return 0
