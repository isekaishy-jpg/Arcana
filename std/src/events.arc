import std.kernel.events
import std.collections.list
import std.option
use std.option.Option

export enum AppEvent:
    WindowResized((Int, Int))
    WindowCloseRequested
    WindowFocused(Bool)
    KeyDown(Int)
    KeyUp(Int)
    MouseDown(Int)
    MouseUp(Int)
    MouseMove((Int, Int))
    MouseWheelY(Int)

fn lift_event(read ev: std.kernel.events.Event) -> Option[AppEvent]:
    return match ev:
        std.kernel.events.Event.None => Option.None[AppEvent] :: :: call
        std.kernel.events.Event.WindowResized(value) => Option.Some[AppEvent] :: (AppEvent.WindowResized :: value :: call) :: call
        std.kernel.events.Event.WindowCloseRequested => Option.Some[AppEvent] :: (AppEvent.WindowCloseRequested :: :: call) :: call
        std.kernel.events.Event.WindowFocused(value) => Option.Some[AppEvent] :: (AppEvent.WindowFocused :: value :: call) :: call
        std.kernel.events.Event.KeyDown(value) => Option.Some[AppEvent] :: (AppEvent.KeyDown :: value :: call) :: call
        std.kernel.events.Event.KeyUp(value) => Option.Some[AppEvent] :: (AppEvent.KeyUp :: value :: call) :: call
        std.kernel.events.Event.MouseDown(value) => Option.Some[AppEvent] :: (AppEvent.MouseDown :: value :: call) :: call
        std.kernel.events.Event.MouseUp(value) => Option.Some[AppEvent] :: (AppEvent.MouseUp :: value :: call) :: call
        std.kernel.events.Event.MouseMove(value) => Option.Some[AppEvent] :: (AppEvent.MouseMove :: value :: call) :: call
        std.kernel.events.Event.MouseWheelY(value) => Option.Some[AppEvent] :: (AppEvent.MouseWheelY :: value :: call) :: call

export fn poll(read win: Window) -> Option[AppEvent]:
    let ev = std.kernel.events.poll :: win :: call
    return std.events.lift_event :: ev :: call

export fn drain(read win: Window) -> List[AppEvent]:
    let mut out = std.collections.list.new[AppEvent] :: :: call
    while true:
        let ev = std.events.poll :: win :: call
        if ev :: :: is_none:
            return out
        out :: (ev :: (AppEvent.WindowCloseRequested :: :: call) :: unwrap_or) :: push
    return out
