import std.collections.list
import std.kernel.events
import std.option
use std.option.Option
use std.window.Window

export opaque type AppFrame as move, boundary_unsafe
export opaque type AppSession as move, boundary_unsafe
export opaque type WakeHandle as copy, boundary_unsafe

export record WindowResizeEvent:
    window_id: Int
    size: (Int, Int)

export record WindowMoveEvent:
    window_id: Int
    position: (Int, Int)

export record WindowFocusEvent:
    window_id: Int
    focused: Bool

export record WindowScaleFactorEvent:
    window_id: Int
    scale_factor_milli: Int

export record WindowThemeEvent:
    window_id: Int
    theme_code: Int

export record KeyEvent:
    window_id: Int
    key: Int
    meta: std.events.KeyMeta

export record KeyMeta:
    modifiers: Int
    repeated: Bool
    physical_key: Int
    logical_key: Int
    location: Int
    text: Str

export record MouseButtonEvent:
    window_id: Int
    button: Int
    position: (Int, Int)
    modifiers: Int

export record MouseMoveEvent:
    window_id: Int
    position: (Int, Int)
    modifiers: Int

export record MouseWheelEvent:
    window_id: Int
    delta: (Int, Int)
    modifiers: Int

export record TextInputEvent:
    window_id: Int
    text: Str

export record TextCompositionEvent:
    window_id: Int
    text: Str
    caret: Int

export record FileDropEvent:
    window_id: Int
    path: Str

export record RawMouseMotionEvent:
    window_id: Int
    delta: (Int, Int)

export enum AppEvent:
    AppResumed
    Wake
    AppSuspended
    AboutToWait
    WindowResized(std.events.WindowResizeEvent)
    WindowMoved(std.events.WindowMoveEvent)
    WindowCloseRequested(Int)
    WindowFocused(std.events.WindowFocusEvent)
    WindowRedrawRequested(Int)
    WindowScaleFactorChanged(std.events.WindowScaleFactorEvent)
    WindowThemeChanged(std.events.WindowThemeEvent)
    KeyDown(std.events.KeyEvent)
    KeyUp(std.events.KeyEvent)
    MouseDown(std.events.MouseButtonEvent)
    MouseUp(std.events.MouseButtonEvent)
    MouseMove(std.events.MouseMoveEvent)
    MouseWheel(std.events.MouseWheelEvent)
    MouseEntered(Int)
    MouseLeft(Int)
    TextInput(std.events.TextInputEvent)
    TextCompositionStarted(Int)
    TextCompositionUpdated(std.events.TextCompositionEvent)
    TextCompositionCommitted(std.events.TextCompositionEvent)
    TextCompositionCancelled(Int)
    FileDropped(std.events.FileDropEvent)
    RawMouseMotion(std.events.RawMouseMotionEvent)

fn key_meta(read raw: std.kernel.events.EventRaw) -> std.events.KeyMeta:
    let mut meta = std.events.KeyMeta :: modifiers = raw.flags, repeated = raw.repeated, physical_key = raw.physical_key :: call
    meta.logical_key = raw.logical_key
    meta.location = raw.key_location
    meta.text = raw.text
    return meta

fn key_event(read raw: std.kernel.events.EventRaw) -> std.events.KeyEvent:
    let meta = std.events.key_meta :: raw :: call
    return std.events.KeyEvent :: window_id = raw.window_id, key = raw.key_code, meta = meta :: call

fn mouse_button_event(read raw: std.kernel.events.EventRaw) -> std.events.MouseButtonEvent:
    let mut event = std.events.MouseButtonEvent :: window_id = raw.window_id, button = raw.a, modifiers = raw.flags :: call
    event.position = (raw.pointer_x, raw.pointer_y)
    return event

fn mouse_move_event(read raw: std.kernel.events.EventRaw) -> std.events.MouseMoveEvent:
    return std.events.MouseMoveEvent :: window_id = raw.window_id, position = (raw.a, raw.b), modifiers = raw.flags :: call

fn mouse_wheel_event(read raw: std.kernel.events.EventRaw) -> std.events.MouseWheelEvent:
    return std.events.MouseWheelEvent :: window_id = raw.window_id, delta = (0, raw.a), modifiers = raw.flags :: call

fn composition_event(read raw: std.kernel.events.EventRaw) -> std.events.TextCompositionEvent:
    return std.events.TextCompositionEvent :: window_id = raw.window_id, text = raw.text, caret = raw.a :: call

fn lift_event(read raw: std.kernel.events.EventRaw) -> std.events.AppEvent:
    if raw.kind == 1:
        return std.events.AppEvent.WindowResized :: (std.events.WindowResizeEvent :: window_id = raw.window_id, size = (raw.a, raw.b) :: call) :: call
    if raw.kind == 10:
        return std.events.AppEvent.WindowMoved :: (std.events.WindowMoveEvent :: window_id = raw.window_id, position = (raw.a, raw.b) :: call) :: call
    if raw.kind == 2:
        return std.events.AppEvent.WindowCloseRequested :: raw.window_id :: call
    if raw.kind == 3:
        return std.events.AppEvent.WindowFocused :: (std.events.WindowFocusEvent :: window_id = raw.window_id, focused = raw.a != 0 :: call) :: call
    if raw.kind == 4:
        return std.events.AppEvent.KeyDown :: (std.events.key_event :: raw :: call) :: call
    if raw.kind == 5:
        return std.events.AppEvent.KeyUp :: (std.events.key_event :: raw :: call) :: call
    if raw.kind == 6:
        return std.events.AppEvent.MouseDown :: (std.events.mouse_button_event :: raw :: call) :: call
    if raw.kind == 7:
        return std.events.AppEvent.MouseUp :: (std.events.mouse_button_event :: raw :: call) :: call
    if raw.kind == 8:
        return std.events.AppEvent.MouseMove :: (std.events.mouse_move_event :: raw :: call) :: call
    if raw.kind == 9:
        return std.events.AppEvent.MouseWheel :: (std.events.mouse_wheel_event :: raw :: call) :: call
    if raw.kind == 11:
        return std.events.AppEvent.MouseEntered :: raw.window_id :: call
    if raw.kind == 12:
        return std.events.AppEvent.MouseLeft :: raw.window_id :: call
    if raw.kind == 13:
        return std.events.AppEvent.WindowRedrawRequested :: raw.window_id :: call
    if raw.kind == 14:
        return std.events.AppEvent.TextInput :: (std.events.TextInputEvent :: window_id = raw.window_id, text = raw.text :: call) :: call
    if raw.kind == 15:
        return std.events.AppEvent.FileDropped :: (std.events.FileDropEvent :: window_id = raw.window_id, path = raw.text :: call) :: call
    if raw.kind == 16:
        return std.events.AppEvent.WindowScaleFactorChanged :: (std.events.WindowScaleFactorEvent :: window_id = raw.window_id, scale_factor_milli = raw.a :: call) :: call
    if raw.kind == 17:
        return std.events.AppEvent.WindowThemeChanged :: (std.events.WindowThemeEvent :: window_id = raw.window_id, theme_code = raw.a :: call) :: call
    if raw.kind == 18:
        return std.events.AppEvent.RawMouseMotion :: (std.events.RawMouseMotionEvent :: window_id = raw.window_id, delta = (raw.a, raw.b) :: call) :: call
    if raw.kind == 20:
        return std.events.AppEvent.AppResumed :: :: call
    if raw.kind == 21:
        return std.events.AppEvent.Wake :: :: call
    if raw.kind == 22:
        return std.events.AppEvent.AppSuspended :: :: call
    if raw.kind == 24:
        return std.events.AppEvent.TextCompositionStarted :: raw.window_id :: call
    if raw.kind == 25:
        return std.events.AppEvent.TextCompositionUpdated :: (std.events.composition_event :: raw :: call) :: call
    if raw.kind == 26:
        return std.events.AppEvent.TextCompositionCommitted :: (std.events.composition_event :: raw :: call) :: call
    if raw.kind == 27:
        return std.events.AppEvent.TextCompositionCancelled :: raw.window_id :: call
    return std.events.AppEvent.AboutToWait :: :: call

export fn poll(edit frame: AppFrame) -> Option[AppEvent]:
    let raw = std.kernel.events.poll :: frame :: call
    return match raw:
        Option.Some(value) => Option.Some[AppEvent] :: (std.events.lift_event :: value :: call) :: call
        Option.None => Option.None[AppEvent] :: :: call

export fn drain(take frame: AppFrame) -> List[AppEvent]:
    let mut current = frame
    let mut out = std.collections.list.new[AppEvent] :: :: call
    while true:
        let next = std.events.poll :: current :: call
        if next :: :: is_none:
            return out
        out :: (next :: (std.events.AppEvent.AboutToWait :: :: call) :: unwrap_or) :: push
    return out

export fn pump(edit win: Window) -> AppFrame:
    return std.kernel.events.pump :: win :: call

export fn open_session() -> AppSession:
    return std.kernel.events.session_open :: :: call

export fn close_session(edit session: AppSession):
    std.kernel.events.session_close :: session :: call

export fn attach_window(edit session: AppSession, read win: Window):
    std.kernel.events.session_attach_window :: session, win :: call

export fn detach_window(edit session: AppSession, read win: Window):
    std.kernel.events.session_detach_window :: session, win :: call

export fn window_for_id(read session: AppSession, window_id: Int) -> Option[Window]:
    return std.kernel.events.session_window_for_id :: session, window_id :: call

export fn window_ids(read session: AppSession) -> List[Int]:
    return std.kernel.events.session_window_ids :: session :: call

export fn window_id(read event: AppEvent) -> Option[Int]:
    return match event:
        AppEvent.WindowResized(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.WindowMoved(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.WindowCloseRequested(id) => Option.Some[Int] :: id :: call
        AppEvent.WindowFocused(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.WindowRedrawRequested(id) => Option.Some[Int] :: id :: call
        AppEvent.WindowScaleFactorChanged(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.WindowThemeChanged(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.KeyDown(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.KeyUp(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.MouseDown(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.MouseUp(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.MouseMove(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.MouseWheel(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.MouseEntered(id) => Option.Some[Int] :: id :: call
        AppEvent.MouseLeft(id) => Option.Some[Int] :: id :: call
        AppEvent.TextInput(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.TextCompositionStarted(id) => Option.Some[Int] :: id :: call
        AppEvent.TextCompositionUpdated(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.TextCompositionCommitted(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.TextCompositionCancelled(id) => Option.Some[Int] :: id :: call
        AppEvent.FileDropped(ev) => Option.Some[Int] :: ev.window_id :: call
        AppEvent.RawMouseMotion(ev) => Option.Some[Int] :: ev.window_id :: call
        _ => Option.None[Int] :: :: call

export fn event_window(read session: AppSession, read event: AppEvent) -> Option[Window]:
    return match (std.events.window_id :: event :: call):
        Option.Some(id) => std.events.window_for_id :: session, id :: call
        Option.None => Option.None[Window] :: :: call

export fn pump_session(edit session: AppSession) -> AppFrame:
    return std.kernel.events.session_pump :: session :: call

export fn wait_session(edit session: AppSession, timeout_ms: Int) -> AppFrame:
    return std.kernel.events.session_wait :: session, timeout_ms :: call

export fn create_wake(edit session: AppSession) -> WakeHandle:
    return std.kernel.events.session_create_wake :: session :: call

export fn wake(read handle: WakeHandle):
    std.kernel.events.wake_signal :: handle :: call
