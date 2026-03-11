import winspell.window
import winspell.draw
import winspell.input
import winspell.loop
import spell_events.router

fn clamp(v: Int, lo: Int, hi: Int) -> Int:
    let mut out = v
    if out < lo:
        out = lo
    if out > hi:
        out = hi
    return out

fn main() -> Int:
    let mut win = winspell.window.open :: "Arcana Top-Down Arena (ESC to quit)", 640, 360 :: call
    let mut cfg = winspell.loop.default_frame_config :: :: call

    let key_left = winspell.input.key_code :: "left" :: call
    let key_right = winspell.input.key_code :: "right" :: call
    let key_up = winspell.input.key_code :: "up" :: call
    let key_down = winspell.input.key_code :: "down" :: call
    let key_escape = winspell.input.key_code :: "escape" :: call
    let key_space = winspell.input.key_code :: "space" :: call

    let bg = winspell.draw.rgb :: 10, 14, 22 :: call
    let player_col = winspell.draw.rgb :: 70, 210, 170 :: call
    let enemy_col = winspell.draw.rgb :: 235, 90, 90 :: call
    let bullet_col = winspell.draw.rgb :: 255, 235, 140 :: call
    let hud_col = winspell.draw.rgb :: 220, 220, 220 :: call

    let mut px = 320
    let mut py = 300
    let mut ex = 300
    let mut ey = 40
    let mut enemy_dir = 2

    let mut bullet_live = false
    let mut bx = 0
    let mut by = 0

    let mut score = 0
    let mut ticks = 0

    while winspell.loop.should_run :: win :: call:
        if winspell.input.key_pressed :: win, key_escape :: call:
            winspell.window.close :: win :: call

        let _evs = spell_events.router.drain :: win :: call
        let _ev_count = spell_events.router.count :: win :: call

        let mut dx = 0
        let mut dy = 0
        if winspell.input.key_down :: win, key_left :: call:
            dx -= 4
        if winspell.input.key_down :: win, key_right :: call:
            dx += 4
        if winspell.input.key_down :: win, key_up :: call:
            dy -= 4
        if winspell.input.key_down :: win, key_down :: call:
            dy += 4

        px = clamp :: (px + dx), 8, 616 :: call
        py = clamp :: (py + dy), 24, 336 :: call

        let fire = winspell.input.key_pressed :: win, key_space :: call
        if fire and not bullet_live:
            bullet_live = true
            bx = px + 8
            by = py - 6

        if bullet_live:
            by -= 8
            if by < -8:
                bullet_live = false

        if (ticks % 2) == 0:
            ex += enemy_dir
        if ex < 8:
            ex = 8
            enemy_dir = 2
            ey += 10
        if ex > 608:
            ex = 608
            enemy_dir = -2
            ey += 10
        if ey > 180:
            ey = 24

        if bullet_live:
            let hit_x = bx >= ex and bx <= (ex + 24)
            let hit_y = by >= ey and by <= (ey + 24)
            if hit_x and hit_y:
                bullet_live = false
                score += 1
                ex = 20 + ((ticks * 37 + score * 53) % 580)
                ey = 20 + ((ticks * 19 + score * 17) % 120)

        cfg.clear = bg
        winspell.loop.begin_frame :: win, cfg :: call
        let player_rect = winspell.draw.RectSpec :: pos = (px, py), size = (18, 18), color = player_col :: call
        let enemy_rect = winspell.draw.RectSpec :: pos = (ex, ey), size = (24, 24), color = enemy_col :: call
        winspell.draw.rect :: win, player_rect :: call
        winspell.draw.rect :: win, enemy_rect :: call
        if bullet_live:
            let bullet_rect = winspell.draw.RectSpec :: pos = (bx, by), size = (4, 8), color = bullet_col :: call
            winspell.draw.rect :: win, bullet_rect :: call

        let hud_label = winspell.draw.LabelSpec :: pos = (8, 8), text = "ESC quit, arrows move, space fire", color = hud_col :: call
        winspell.draw.label :: win, hud_label :: call
        winspell.loop.end_frame :: win :: call

        ticks += 1

    return 0
