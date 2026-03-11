export record WindowConfig:
    title: Str
    size: (Int, Int)

export record FixedStepConfig:
    tick_hz: Int
    max_steps: Int

export record FixedRunner:
    tick_ms: Int
    accumulator_ms: Int
