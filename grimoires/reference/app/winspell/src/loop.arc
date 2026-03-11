import winspell.window
import winspell.draw

export record FrameConfig:
    clear: Int

export record FixedRunner:
    tick_ms: Int
    accumulator_ms: Int

export fn default_frame_config() -> FrameConfig:
    return winspell.loop.FrameConfig :: clear = 0 :: call

export fn fixed_tick_ms(tick_hz: Int) -> Int:
    if tick_hz <= 0:
        return 0
    return 1000 / tick_hz

export fn fixed_runner(tick_hz: Int) -> FixedRunner:
    let tick = winspell.loop.fixed_tick_ms :: tick_hz :: call
    return winspell.loop.FixedRunner :: tick_ms = tick, accumulator_ms = 0 :: call

export fn fixed_runner_reset(edit runner: FixedRunner):
    runner.accumulator_ms = 0

export fn fixed_alpha_milli(read accumulator_ms: Int, tick_ms: Int) -> Int:
    if tick_ms <= 0:
        return 0
    let mut alpha = (accumulator_ms * 1000) / tick_ms
    if alpha < 0:
        alpha = 0
    if alpha > 1000:
        alpha = 1000
    return alpha

export fn fixed_runner_step(edit runner: FixedRunner, frame_ms: Int) -> (Int, Int):
    let tick_ms = runner.tick_ms
    if tick_ms <= 0:
        return (0, 0)
    runner.accumulator_ms += frame_ms
    let mut steps = 0
    while runner.accumulator_ms >= tick_ms:
        runner.accumulator_ms -= tick_ms
        steps += 1
    let alpha = winspell.loop.fixed_alpha_milli :: runner.accumulator_ms, tick_ms :: call
    return (steps, alpha)

export fn begin_frame(edit win: Window, read cfg: FrameConfig):
    winspell.draw.fill :: win, cfg.clear :: call

export fn end_frame(edit win: Window):
    winspell.draw.present :: win :: call

export fn should_run(read win: Window) -> Bool:
    return winspell.window.alive :: win :: call
