import arcana_desktop.loop
import arcana_desktop.types
import std.ecs

export record Adapter:
    started: Bool
    runner: arcana_desktop.types.FixedRunner
    cfg: arcana_desktop.types.FixedStepConfig

export fn adapter(read cfg: arcana_desktop.types.FixedStepConfig) -> arcana_desktop.ecs.Adapter:
    return arcana_desktop.ecs.Adapter :: started = false, runner = (arcana_desktop.loop.fixed_runner :: cfg :: call), cfg = cfg :: call

impl Adapter:
    fn startup(edit self: arcana_desktop.ecs.Adapter) -> Int:
        if self.started:
            return 0
        self.started = true
        return std.ecs.step_startup :: :: call

    fn step_frame(edit self: arcana_desktop.ecs.Adapter, elapsed_ms: Int) -> Int:
        let mut total = 0
        let mut steps = 0
        self.runner.accumulator_ms += elapsed_ms
        while self.runner.accumulator_ms >= self.runner.tick_ms:
            if steps >= self.cfg.max_steps:
                break
            total += std.ecs.step_fixed_update :: :: call
            self.runner.accumulator_ms -= self.runner.tick_ms
            steps += 1
        total += std.ecs.step_update :: :: call
        return total

    fn render(edit self: arcana_desktop.ecs.Adapter) -> Int:
        return std.ecs.step_render :: :: call

    fn step_all(edit self: arcana_desktop.ecs.Adapter, elapsed_ms: Int) -> Int:
        let mut total = self :: :: startup
        total += self :: elapsed_ms :: step_frame
        total += self :: :: render
        return total
