export trait StartupPhase[T]:
    fn on_startup(edit self: T) -> Int:
        return 0

export trait FixedPhase[T]:
    fn on_fixed(edit self: T) -> Int:
        return 0

export trait UpdatePhase[T]:
    fn on_update(edit self: T) -> Int:
        return 0

export trait RenderPhase[T]:
    fn on_render(edit self: T) -> Int:
        return 0

export fn run_startup[T, where std.behavior_traits.StartupPhase[T]](edit value: T) -> Int:
    return value :: :: on_startup

export fn run_fixed[T, where std.behavior_traits.FixedPhase[T]](edit value: T) -> Int:
    return value :: :: on_fixed

export fn run_update[T, where std.behavior_traits.UpdatePhase[T]](edit value: T) -> Int:
    return value :: :: on_update

export fn run_render[T, where std.behavior_traits.RenderPhase[T]](edit value: T) -> Int:
    return value :: :: on_render
