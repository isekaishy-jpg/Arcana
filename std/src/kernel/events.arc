use std.events.AppFrame
use std.window.Window

enum Event:
    None
    WindowResized((Int, Int))
    WindowMoved((Int, Int))
    WindowCloseRequested
    WindowFocused(Bool)
    KeyDown(Int)
    KeyUp(Int)
    MouseDown(Int)
    MouseUp(Int)
    MouseMove((Int, Int))
    MouseWheelY(Int)
    MouseEntered
    MouseLeft

intrinsic fn pump_frame(edit win: Window) -> AppFrame = EventsPump
intrinsic fn poll_frame(edit frame: AppFrame) -> (Int, (Int, Int)) = EventsPoll

fn pump(edit win: Window) -> AppFrame:
    return std.kernel.events.pump_frame :: win :: call

fn decode(kind: Int, a: Int, b: Int) -> std.kernel.events.Event:
    if kind == 1:
        return std.kernel.events.Event.WindowResized :: (a, b) :: call
    if kind == 10:
        return std.kernel.events.Event.WindowMoved :: (a, b) :: call
    if kind == 2:
        return std.kernel.events.Event.WindowCloseRequested :: :: call
    if kind == 3:
        return std.kernel.events.Event.WindowFocused :: a != 0 :: call
    if kind == 4:
        return std.kernel.events.Event.KeyDown :: a :: call
    if kind == 5:
        return std.kernel.events.Event.KeyUp :: a :: call
    if kind == 6:
        return std.kernel.events.Event.MouseDown :: a :: call
    if kind == 7:
        return std.kernel.events.Event.MouseUp :: a :: call
    if kind == 8:
        return std.kernel.events.Event.MouseMove :: (a, b) :: call
    if kind == 9:
        return std.kernel.events.Event.MouseWheelY :: a :: call
    if kind == 11:
        return std.kernel.events.Event.MouseEntered :: :: call
    if kind == 12:
        return std.kernel.events.Event.MouseLeft :: :: call
    return std.kernel.events.Event.None :: :: call

fn poll(edit frame: AppFrame) -> std.kernel.events.Event:
    let raw = std.kernel.events.poll_frame :: frame :: call
    return std.kernel.events.decode :: raw.0, raw.1.0, raw.1.1 :: call
