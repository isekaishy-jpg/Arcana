import std.kernel.time
import std.concurrent
import std.types.core

export fn duration_ms(value: Int) -> std.types.core.DurationMs:
    return std.types.core.DurationMs :: value = value :: call

export fn monotonic_now_ms() -> std.types.core.MonotonicTimeMs:
    let raw = std.kernel.time.monotonic_now_ms :: :: call
    return std.types.core.MonotonicTimeMs :: value = raw :: call

export fn monotonic_now_ns() -> Int:
    return std.kernel.time.monotonic_now_ns :: :: call

export fn elapsed_ms(start: std.types.core.MonotonicTimeMs, end: std.types.core.MonotonicTimeMs) -> std.types.core.DurationMs:
    return std.time.duration_ms :: end.value - start.value :: call

export fn sleep(read duration: std.types.core.DurationMs):
    std.concurrent.sleep :: duration.value :: call

export fn sleep_ms(ms: Int):
    std.concurrent.sleep :: ms :: call
