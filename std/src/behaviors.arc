import std.kernel.concurrency

export fn step(phase: Str) -> Int:
    return std.kernel.concurrency.behavior_step :: phase :: call
