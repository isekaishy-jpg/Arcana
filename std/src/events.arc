import std.kernel.events
import std.collections.list

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

fn decode_event(kind: Int, a: Int, b: Int) -> (Bool, AppEvent):
    if kind == 1:
        return (true, AppEvent.WindowResized :: (a, b) :: call)
    if kind == 2:
        return (true, AppEvent.WindowCloseRequested :: :: call)
    if kind == 3:
        return (true, AppEvent.WindowFocused :: a != 0 :: call)
    if kind == 4:
        return (true, AppEvent.KeyDown :: a :: call)
    if kind == 5:
        return (true, AppEvent.KeyUp :: a :: call)
    if kind == 6:
        return (true, AppEvent.MouseDown :: a :: call)
    if kind == 7:
        return (true, AppEvent.MouseUp :: a :: call)
    if kind == 8:
        return (true, AppEvent.MouseMove :: (a, b) :: call)
    if kind == 9:
        return (true, AppEvent.MouseWheelY :: a :: call)
    return (false, AppEvent.WindowCloseRequested :: :: call)

export fn poll(read win: Window) -> (Bool, AppEvent):
    let kind = std.kernel.events.events_poll_kind :: win :: call
    let a = std.kernel.events.events_poll_a :: win :: call
    let b = std.kernel.events.events_poll_b :: win :: call
    return std.events.decode_event :: kind, a, b :: call

export fn drain(read win: Window) -> List[AppEvent]:
    let mut out = std.collections.list.new[AppEvent] :: :: call
    while true:
        let kind = std.kernel.events.events_poll_kind :: win :: call
        let a = std.kernel.events.events_poll_a :: win :: call
        let b = std.kernel.events.events_poll_b :: win :: call
        if kind == 0:
            return out
        if kind == 1:
            out :: (AppEvent.WindowResized :: (a, b) :: call) :: push
        if kind == 2:
            out :: (AppEvent.WindowCloseRequested :: :: call) :: push
        if kind == 3:
            out :: (AppEvent.WindowFocused :: a != 0 :: call) :: push
        if kind == 4:
            out :: (AppEvent.KeyDown :: a :: call) :: push
        if kind == 5:
            out :: (AppEvent.KeyUp :: a :: call) :: push
        if kind == 6:
            out :: (AppEvent.MouseDown :: a :: call) :: push
        if kind == 7:
            out :: (AppEvent.MouseUp :: a :: call) :: push
        if kind == 8:
            out :: (AppEvent.MouseMove :: (a, b) :: call) :: push
        if kind == 9:
            out :: (AppEvent.MouseWheelY :: a :: call) :: push
    return out
