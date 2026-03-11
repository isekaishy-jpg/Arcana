import std.canvas
import std.window
import std.input
import std.events
import std.memory
import std.collections.list
import std.collections.map
import std.collections.array
import std.concurrent
import std.io
import winspell.loop
use std.canvas as canvas
use std.window as window
use std.input as input
use std.events as events
use std.memory as memory
use std.collections.list as list
use std.collections.map as map
use std.collections.array as array
use std.concurrent as concurrent
use std.io as io

record TrailPoint:
    x: Int
    y: Int

fn make_point(x: Int, y: Int) -> TrailPoint:
    return TrailPoint :: x = x, y = y :: call

fn point_energy(p: TrailPoint) -> Int:
    return p.x + p.y

fn id_echo(id: ArenaId[TrailPoint]) -> ArenaId[TrailPoint]:
    return id

fn chain_seed() -> Int:
    100 :: :: io.print
    return 100

fn chain_add(x: Int) -> Int:
    return x + 7

fn chain_ping(x: Int) -> Int:
    x :: :: io.print
    return x

fn run_chain_matrix():
    forward :=> chain_seed => chain_add => chain_ping
    forward :=< chain_add <= chain_ping <= chain_seed
    lazy :=> chain_seed => chain_add => chain_ping
    parallel :=> chain_seed => chain_add => chain_ping
    broadcast :=> chain_seed => chain_add => chain_ping
    collect :=> chain_seed => chain_add => chain_ping
    async :=> chain_seed => chain_add => chain_ping
    plan :=> chain_seed => chain_add => chain_ping

fn main() -> Int:
    let mut win = canvas.open :: "Arcana Showcase v24", 800, 480 :: call
    let sprite = canvas.image_load :: "grimoires/reference/examples/assets/arcana_demo.png" :: call
    let sprite_size = canvas.image_size :: sprite :: call
    let sw = sprite_size.0
    let sh = sprite_size.1

    let mut runner = winspell.loop.fixed_runner :: 60 :: call
    let mut palette = array.new[Int] :: 4, 0 :: call
    palette[0] = canvas.rgb :: 12, 16, 24 :: call
    palette[1] = canvas.rgb :: 64, 190, 255 :: call
    palette[2] = canvas.rgb :: 142, 236, 169 :: call
    palette[3] = canvas.rgb :: 255, 187, 73 :: call

    let mut trail = memory.new[TrailPoint] :: 512 :: call
    let mut trail_ids = list.new[ArenaId[TrailPoint]] :: :: call
    let mut metrics = map.new[Str, Int] :: :: call
    metrics :: "frames", 0 :: set
    metrics :: "events", 0 :: set
    metrics :: "clicks", 0 :: set
    metrics :: "resets", 0 :: set
    metrics :: "resized", 0 :: set

    let key_left = input.key_code :: "left" :: call
    let key_right = input.key_code :: "right" :: call
    let key_up = input.key_code :: "up" :: call
    let key_down = input.key_code :: "down" :: call
    let key_escape = input.key_code :: "escape" :: call
    let key_space = input.key_code :: "space" :: call
    let mouse_left = input.mouse_button_code :: "left" :: call

    let mut px = 120
    let mut py = 80
    let mut speed = 5

    run_chain_matrix :: :: call

    while canvas.alive :: win :: call:
        let step = winspell.loop.fixed_runner_step :: runner, 16 :: call
        let _alpha = step.1
        let mut frame = events.pump :: win :: call
        let mut event_count = 0
        while true:
            let next = events.poll :: frame :: call
            let keep_going = next :: :: is_some
            if keep_going == false:
                break
            event_count += 1
        metrics["events"] += event_count
        metrics["frames"] += 1

        if input.key_pressed :: frame, key_escape :: call:
            window.close :: win :: call
        let frames_now = metrics["frames"]
        if frames_now > 360:
            window.close :: win :: call

        if input.key_down :: frame, key_left :: call:
            px -= speed
        if input.key_down :: frame, key_right :: call:
            px += speed
        if input.key_down :: frame, key_up :: call:
            py -= speed
        if input.key_down :: frame, key_down :: call:
            py += speed

        if input.mouse_pressed :: frame, mouse_left :: call:
            metrics["clicks"] += 1

        let m = input.mouse_pos :: frame :: call
        if input.mouse_down :: frame, mouse_left :: call:
            px = m.0
            py = m.1

        let wheel = input.mouse_wheel_y :: frame :: call
        if wheel != 0:
            speed += wheel
            if speed < 1:
                speed = 1
            if speed > 24:
                speed = 24

        let sz = window.size :: win :: call
        let w = sz.0
        let h = sz.1
        if px < 0:
            px = 0
        if py < 0:
            py = 0
        if w > sw and px > w - sw:
            px = w - sw
        if h > sh and py > h - sh:
            py = h - sh

        let id = arena: trail :> px, py <: make_point
        trail_ids :: id :: push
        let trail_len = trail_ids :: :: len
        if trail_len > 180:
            trail :: :: reset
            trail_ids = list.new[ArenaId[TrailPoint]] :: :: call
            metrics["resets"] += 1

        if input.key_pressed :: frame, key_space :: call:
            make_point :: px, py :: call
                forward :=> point_energy => chain_ping
            arena: trail :> px, py <: make_point
                forward :=> id_echo

        if window.resized :: win :: call:
            metrics["resized"] += 1

        let mut bg = palette[0]
        if event_count > 0:
            bg = palette[3]
        canvas.fill :: win, bg :: call

        for tid in trail_ids:
            if trail :: tid :: has:
                let p = trail :: tid :: get
                win :: sprite, p.x, p.y :: canvas.blit

        win :: sprite, px, py :: canvas.blit

        let mut frame_mod = 0
        if w > 0:
            frame_mod = metrics["frames"] % w
        let mut event_mod = 0
        if w > 0:
            event_mod = metrics["events"] % w
        let mut scan_x = frame_mod
        if scan_x > w - sw:
            scan_x = w - sw
        let mut pulse_x = event_mod
        if pulse_x > w - sw:
            pulse_x = w - sw
        win :: sprite, scan_x, 12 :: canvas.blit
        win :: sprite, pulse_x, 28 :: canvas.blit
        canvas.present :: win :: call
        16 :: :: concurrent.sleep

    "showcase-complete" :: :: io.print
    metrics["frames"] :: :: io.print
    metrics["events"] :: :: io.print
    metrics["clicks"] :: :: io.print
    metrics["resets"] :: :: io.print
    metrics["resized"] :: :: io.print
    return 0
