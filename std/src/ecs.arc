import std.behaviors
import std.iter
import std.kernel.ecs

export fn step_startup() -> Int:
    return std.behaviors.step :: "startup" :: call

export fn step_fixed_update() -> Int:
    return std.behaviors.step :: "fixed_update" :: call

export fn step_update() -> Int:
    return std.behaviors.step :: "update" :: call

export fn step_render() -> Int:
    return std.behaviors.step :: "render" :: call

export fn step_phase(phase: Str) -> Int:
    return std.behaviors.step :: phase :: call

export fn set_component[T](take value: T):
    std.kernel.ecs.ecs_set_singleton[T] :: value :: call

export fn has_component[T]() -> Bool:
    return std.kernel.ecs.ecs_has_singleton[T] :: :: call

export fn get_component[T]() -> T:
    return std.kernel.ecs.ecs_get_singleton[T] :: :: call

export fn spawn() -> Int:
    return std.kernel.ecs.ecs_spawn :: :: call

export fn despawn(entity: Int):
    std.kernel.ecs.ecs_despawn :: entity :: call

export fn set_component_at[T](entity: Int, take value: T):
    std.kernel.ecs.ecs_set_component_at[T] :: entity, value :: call

export fn has_component_at[T](entity: Int) -> Bool:
    return std.kernel.ecs.ecs_has_component_at[T] :: entity :: call

export fn get_component_at[T](entity: Int) -> T:
    return std.kernel.ecs.ecs_get_component_at[T] :: entity :: call

export fn remove_component_at[T](entity: Int):
    std.kernel.ecs.ecs_remove_component_at[T] :: entity :: call

export fn remove_component[T]():
    std.kernel.ecs.ecs_remove_component_at[T] :: 0 :: call

export record SingletonIntCursor:
    consumed: Bool

export fn singleton_int_cursor() -> SingletonIntCursor:
    return std.ecs.SingletonIntCursor :: consumed = false :: call

impl std.iter.Iterator[SingletonIntCursor] for SingletonIntCursor:
    type Item = Int
    fn next(edit self: SingletonIntCursor) -> (Bool, Int):
        if self.consumed:
            return (false, 0)
        self.consumed = true
        if has_component[Int] :: :: call:
            return (true, get_component[Int] :: :: call)
        return (false, 0)
