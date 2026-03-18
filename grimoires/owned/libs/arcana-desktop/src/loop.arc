import std.time
import std.types.core
import arcana_desktop.types

export fn fixed_tick_ms(tick_hz: Int) -> Int:
    if tick_hz <= 0:
        return 16
    return 1000 / tick_hz

export fn fixed_runner(read cfg: arcana_desktop.types.FixedStepConfig) -> arcana_desktop.types.FixedRunner:
    let tick_ms = arcana_desktop.loop.fixed_tick_ms :: cfg.tick_hz :: call
    return arcana_desktop.types.FixedRunner :: tick_ms = tick_ms, accumulator_ms = 0 :: call

export fn frame_start() -> std.types.core.MonotonicTimeMs:
    return std.time.monotonic_now_ms :: :: call

export fn frame_elapsed_ms(start: std.types.core.MonotonicTimeMs) -> Int:
    let end = std.time.monotonic_now_ms :: :: call
    let delta = std.time.elapsed_ms :: start, end :: call
    return delta.value
