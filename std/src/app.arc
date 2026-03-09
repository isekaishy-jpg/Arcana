record FixedRunner:
    tick_ms: Int
    accumulator_ms: Int

export fn fixed_tick_ms(tick_hz: Int) -> Int:
    if tick_hz <= 0:
        return 0
    return 1000 / tick_hz

export fn fixed_runner(tick_hz: Int) -> FixedRunner:
    let tick = std.app.fixed_tick_ms :: tick_hz :: call
    return std.app.FixedRunner :: tick_ms = tick, accumulator_ms = 0 :: call

export fn fixed_runner_reset(edit runner: FixedRunner):
    runner.accumulator_ms = 0

export fn fixed_consume_steps(edit accumulator_ms: Int, frame_ms: Int, tick_ms: Int) -> Int:
    if tick_ms <= 0:
        return 0
    accumulator_ms += frame_ms
    let mut steps = 0
    while accumulator_ms >= tick_ms:
        accumulator_ms -= tick_ms
        steps += 1
    return steps

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
    let alpha = std.app.fixed_alpha_milli :: runner.accumulator_ms, tick_ms :: call
    return (steps, alpha)
