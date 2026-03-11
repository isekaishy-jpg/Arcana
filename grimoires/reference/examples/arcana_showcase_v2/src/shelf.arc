import std.canvas
import std.window
import std.input
import std.events
import std.memory
import std.collections.list
import std.collections.array
import std.collections.map
import std.concurrent
import std.io
import winspell.loop

use std.canvas as canvas
use std.window as window
use std.input as input
use std.events as events
use std.memory as memory
use std.collections.list as list
use std.collections.array as array
use std.collections.map as map
use std.concurrent as concurrent
use std.io as io

record Player:
    x: Int
    y: Int
    speed: Int

record Ball:
    x: Int
    y: Int
    vx: Int

record Brick:
    x: Int
    y: Int
    alive: Bool

record Flash:
    x: Int
    y: Int
    ttl: Int

fn dead_brick() -> Brick:
    return Brick :: x = 0, y = 0, alive = false :: call

fn make_player() -> Player:
    return Player :: x = 410, y = 500, speed = 10 :: call

fn make_ball(x: Int, y: Int) -> Ball:
    return Ball :: x = x, y = y, vx = 5 :: call

fn make_flash(x: Int, y: Int) -> Flash:
    return Flash :: x = x, y = y, ttl = 14 :: call

fn clamp(v: Int, lo: Int, hi: Int) -> Int:
    let mut out = v
    if out < lo:
        out = lo
    if out > hi:
        out = hi
    return out

fn overlap_ball_paddle(read ball: Ball, ball_size: Int, read player: Player) -> Bool:
    if ball.x + ball_size <= player.x:
        return false
    if player.x + 120 <= ball.x:
        return false
    if ball.y + ball_size <= player.y:
        return false
    if player.y + 14 <= ball.y:
        return false
    return true

fn overlap_ball_brick(read ball: Ball, ball_size: Int, read brick: Brick) -> Bool:
    if ball.x + ball_size <= brick.x:
        return false
    if brick.x + 68 <= ball.x:
        return false
    if ball.y + ball_size <= brick.y:
        return false
    if brick.y + 18 <= ball.y:
        return false
    return true

fn init_bricks(edit bricks: Array[Brick]):
    let cols = 10
    let rows = 4
    let mut i = 0
    for y in 0..rows:
        for x in 0..cols:
            let px = 36 + (x * 80)
            let py = 52 + (y * 28)
            bricks[i] = Brick :: x = px, y = py, alive = true :: call
            i += 1

fn count_alive(read bricks: Array[Brick]) -> Int:
    let mut alive = 0
    let n = bricks :: :: len
    for i in 0..n:
        if bricks[i].alive:
            alive += 1
    return alive

fn chain_seed() -> Int:
    return 11

fn chain_add(x: Int) -> Int:
    return x + 5

fn chain_mul2(x: Int) -> Int:
    return x * 2

fn chain_cap(x: Int) -> Int:
    if x > 99:
        return 99
    return x

fn chain_echo(x: Int) -> Int:
    x :: :: io.print
    return x

async fn chain_async_add(x: Int) -> Int:
    return x + 3

async fn run_chain_demo_async() -> Int:
    async :=> chain_seed => chain_async_add => chain_echo
    return 0

fn run_chain_demo():
    forward :=> chain_seed => chain_add => chain_echo
    forward :=< chain_add <= chain_echo <= chain_seed
    lazy :=> chain_seed => chain_add => chain_echo
    collect :=> chain_seed => chain_add => chain_echo
    plan :=> chain_seed => chain_add => chain_echo

    chain_seed :: :: call
        parallel :=> chain_add => chain_mul2 => chain_cap
    chain_seed :: :: call
        broadcast :=> chain_add => chain_mul2 => chain_cap

fn score_tune(seed: Int) -> Int:
    return seed + 2

fn score_log(v: Int) -> Int:
    return v

fn flash_id_echo(id: ArenaId[Flash]) -> ArenaId[Flash]:
    return id

fn main() -> Int:
    let mut win = canvas.open :: "Arcana Showcase v2 - Breakout Slice", 900, 540 :: call
    let sprite = canvas.image_load :: "grimoires/reference/examples/assets/arcana_demo.png" :: call

    let color_bg = canvas.rgb :: 10, 12, 18 :: call
    let color_event = canvas.rgb :: 22, 26, 36 :: call

    let key_left = input.key_code :: "left" :: call
    let key_right = input.key_code :: "right" :: call
    let key_a = input.key_code :: "a" :: call
    let key_d = input.key_code :: "d" :: call
    let key_escape = input.key_code :: "escape" :: call
    let key_space = input.key_code :: "space" :: call

    let player_w = 120
    let player_h = 14
    let ball_size = 22
    let mut ball_vy = -5
    let mut player = make_player :: :: call
    let mut ball = make_ball :: 446, 470 :: call
    let mut bricks = array.new[Brick] :: 40, dead_brick :: :: call :: call
    init_bricks :: bricks :: call

    let mut flashes = memory.new[Flash] :: 512 :: call
    let mut flash_ids = list.new[ArenaId[Flash]] :: :: call

    let mut stats = map.new[Str, Int] :: :: call
    stats :: "frames", 0 :: set
    stats :: "events", 0 :: set
    stats :: "score", 0 :: set
    stats :: "hits", 0 :: set
    stats :: "lives", 3 :: set
    stats :: "resizes", 0 :: set

    let mut runner = winspell.loop.fixed_runner :: 60 :: call
    run_chain_demo :: :: call
    let t = weave run_chain_demo_async :: :: call
    t :: :: join

    while canvas.alive :: win :: call:
        let mut frame = events.pump :: win :: call
        let mut event_count = 0
        while true:
            let next = events.poll :: frame :: call
            let k = match next:
                Option.Some(ev) => match ev:
                    events.AppEvent.WindowCloseRequested => 1
                    events.AppEvent.WindowResized(_) => 2
                    events.AppEvent.KeyDown(_) => 3
                    events.AppEvent.KeyUp(_) => 4
                    events.AppEvent.MouseDown(_) => 5
                    events.AppEvent.MouseUp(_) => 6
                    events.AppEvent.MouseMove(_) => 7
                    events.AppEvent.MouseWheelY(_) => 8
                    events.AppEvent.WindowFocused(_) => 9
                Option.None => 0
            if k == 0:
                break
            event_count += 1
            if k == 1:
                window.close :: win :: call
            if k == 2:
                stats["resizes"] += 1
        stats["events"] += event_count

        if input.key_pressed :: frame, key_escape :: call:
            window.close :: win :: call

        let dims = window.size :: win :: call
        let ww = dims.0
        let wh = dims.1

        let sim = winspell.loop.fixed_runner_step :: runner, 16 :: call
        let mut steps = sim.0
        while steps > 0:
            steps -= 1
            stats["frames"] += 1

            let mut move_x = 0
            if input.key_down :: frame, key_left :: call:
                move_x -= player.speed
            if input.key_down :: frame, key_right :: call:
                move_x += player.speed
            if input.key_down :: frame, key_a :: call:
                move_x -= player.speed
            if input.key_down :: frame, key_d :: call:
                move_x += player.speed

            player.x += move_x
            player.x = clamp :: player.x, 0, ww - player_w :: call

            ball.x += ball.vx
            ball.y += ball_vy

            if ball.x <= 0:
                ball.x = 0
                ball.vx = -ball.vx
            if ball.x + ball_size >= ww:
                ball.x = ww - ball_size
                ball.vx = -ball.vx
            if ball.y <= 20:
                ball.y = 20
                ball_vy = -ball_vy

            if overlap_ball_paddle :: ball, ball_size, player :: call:
                ball.y = player.y - ball_size
                if ball_vy > 0:
                    ball_vy = -ball_vy
                let pc = player.x + (player_w / 2)
                if ball.x < pc:
                    ball.vx -= 1
                if ball.x > pc:
                    ball.vx += 1
                ball.vx = clamp :: ball.vx, -9, 9 :: call

            let mut hit = false
            let bn = bricks :: :: len
            for i in 0..bn:
                if hit:
                    break
                let mut b = bricks[i]
                if b.alive:
                    if overlap_ball_brick :: ball, ball_size, b :: call:
                        b.alive = false
                        bricks[i] = b
                        ball_vy = -ball_vy
                        stats["hits"] += 1
                        stats["score"] += 10
                        let fid = arena: flashes :> b.x, b.y <: make_flash
                        flash_ids :: fid :: push
                        score_tune :: stats["hits"] :: call
                            forward :=> chain_add => score_log
                        hit = true

            let mut next_ids = list.new[ArenaId[Flash]] :: :: call
            for fid in flash_ids:
                if flashes :: fid :: has:
                    let mut f = flashes :: fid :: get
                    f.ttl -= 1
                    if f.ttl > 0:
                        flashes :: fid, f :: set
                        next_ids :: fid :: push
                    else:
                        flashes :: fid :: remove
            flash_ids = next_ids

            if ball.y > wh:
                stats["lives"] -= 1
                ball = make_ball :: player.x + (player_w / 2) - 11, player.y - 30 :: call
                ball_vy = -5
                if stats["lives"] <= 0:
                    window.close :: win :: call

            let alive = count_alive :: bricks :: call
            if alive == 0:
                window.close :: win :: call

            if input.key_pressed :: frame, key_space :: call:
                arena: flashes :> player.x, player.y <: make_flash
                    forward :=> flash_id_echo

            if stats["frames"] > 720:
                window.close :: win :: call

        let mut bg = color_bg
        if event_count > 0:
            bg = color_event
        canvas.fill :: win, bg :: call

        let bn2 = bricks :: :: len
        for i in 0..bn2:
            let b = bricks[i]
            if b.alive:
                win :: sprite, b.x, b.y :: canvas.blit

        for fid in flash_ids:
            if flashes :: fid :: has:
                let f = flashes :: fid :: get
                win :: sprite, f.x, f.y :: canvas.blit

        win :: sprite, player.x, player.y :: canvas.blit
        win :: sprite, player.x + 24, player.y :: canvas.blit
        win :: sprite, player.x + 48, player.y :: canvas.blit
        win :: sprite, ball.x, ball.y :: canvas.blit

        let mut score_icons = stats["score"] / 100
        if score_icons > 10:
            score_icons = 10
        for i in 0..score_icons:
            win :: sprite, 8 + (i * 20), 8 :: canvas.blit

        let lives = stats["lives"]
        for i in 0..lives:
            win :: sprite, ww - 28 - (i * 20), 8 :: canvas.blit

        canvas.present :: win :: call
        16 :: :: concurrent.sleep

    "showcase-v2-complete" :: :: io.print
    stats["score"] :: :: io.print
    stats["hits"] :: :: io.print
    stats["events"] :: :: io.print
    stats["resizes"] :: :: io.print
    return 0
