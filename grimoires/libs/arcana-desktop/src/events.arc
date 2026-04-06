import arcana_desktop.types
import std.collections.list
import std.events
import std.option
use std.option.Option

fn window_id_value(id: Int) -> arcana_desktop.types.WindowId:
    return arcana_desktop.types.WindowId :: value = id :: call

fn some_device_id(id: Int) -> Option[arcana_desktop.types.DeviceId]:
    if id == 0:
        return Option.None[arcana_desktop.types.DeviceId] :: :: call
    return Option.Some[arcana_desktop.types.DeviceId] :: (arcana_desktop.types.DeviceId :: value = id :: call) :: call

fn key_meta(read raw: std.events.KeyMeta) -> arcana_desktop.types.KeyMeta:
    let mut meta = arcana_desktop.types.KeyMeta :: modifiers = raw.modifiers, repeated = raw.repeated, physical_key = raw.physical_key :: call
    meta.logical_key = raw.logical_key
    meta.location = raw.location
    meta.text = raw.text
    return meta

fn key_event(read raw: std.events.KeyEvent) -> arcana_desktop.types.KeyEvent:
    return arcana_desktop.types.KeyEvent :: window_id = raw.window_id, key = raw.key, meta = (arcana_desktop.events.key_meta :: raw.meta :: call) :: call

fn mouse_button_event(read raw: std.events.MouseButtonEvent) -> arcana_desktop.types.MouseButtonEvent:
    let mut event = arcana_desktop.types.MouseButtonEvent :: window_id = raw.window_id, button = raw.button, position = raw.position :: call
    event.modifiers = raw.modifiers
    return event

fn mouse_move_event(read raw: std.events.MouseMoveEvent) -> arcana_desktop.types.MouseMoveEvent:
    return arcana_desktop.types.MouseMoveEvent :: window_id = raw.window_id, position = raw.position, modifiers = raw.modifiers :: call

fn mouse_wheel_event(read raw: std.events.MouseWheelEvent) -> arcana_desktop.types.MouseWheelEvent:
    return arcana_desktop.types.MouseWheelEvent :: window_id = raw.window_id, delta = raw.delta, modifiers = raw.modifiers :: call

fn text_input_event(read raw: std.events.TextInputEvent) -> arcana_desktop.types.TextInputEvent:
    return arcana_desktop.types.TextInputEvent :: window_id = raw.window_id, text = raw.text :: call

fn text_composition_event(read raw: std.events.TextCompositionEvent) -> arcana_desktop.types.TextCompositionEvent:
    return arcana_desktop.types.TextCompositionEvent :: window_id = raw.window_id, text = raw.text, caret = raw.caret :: call

fn file_drop_event(read raw: std.events.FileDropEvent) -> arcana_desktop.types.FileDropEvent:
    return arcana_desktop.types.FileDropEvent :: window_id = raw.window_id, path = raw.path :: call

fn raw_mouse_motion_event(read raw: std.events.RawMouseMotionEvent) -> arcana_desktop.types.RawMouseMotionEvent:
    return arcana_desktop.types.RawMouseMotionEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.device_id :: call), delta = raw.delta :: call

fn raw_mouse_button_event(read raw: std.events.RawMouseButtonEvent) -> arcana_desktop.types.RawMouseButtonEvent:
    return arcana_desktop.types.RawMouseButtonEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.device_id :: call), button = raw.button, pressed = raw.pressed :: call

fn raw_mouse_wheel_event(read raw: std.events.RawMouseWheelEvent) -> arcana_desktop.types.RawMouseWheelEvent:
    return arcana_desktop.types.RawMouseWheelEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.device_id :: call), delta = raw.delta :: call

fn raw_key_event(read raw: std.events.RawKeyEvent) -> arcana_desktop.types.RawKeyEvent:
    let mut event = arcana_desktop.types.RawKeyEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.device_id :: call), key = raw.key, meta = (arcana_desktop.events.key_meta :: raw.meta :: call) :: call
    event.pressed = raw.pressed
    return event

fn device_events_code(read value: arcana_desktop.types.DeviceEvents) -> Int:
    return match value:
        arcana_desktop.types.DeviceEvents.Never => 0
        arcana_desktop.types.DeviceEvents.Always => 2
        _ => 1

fn lift_device_events(code: Int) -> arcana_desktop.types.DeviceEvents:
    if code == 0:
        return arcana_desktop.types.DeviceEvents.Never :: :: call
    if code == 2:
        return arcana_desktop.types.DeviceEvents.Always :: :: call
    return arcana_desktop.types.DeviceEvents.WhenFocused :: :: call

fn window_dispatch(read window_id: arcana_desktop.types.WindowId, read event: arcana_desktop.types.WindowEvent) -> arcana_desktop.types.WindowDispatchEvent:
    return arcana_desktop.types.WindowDispatchEvent :: window_id = window_id, event = event :: call

fn some_window_dispatch(window_id: Int, read event: arcana_desktop.types.WindowEvent) -> Option[arcana_desktop.types.WindowDispatchEvent]:
    let dispatch = arcana_desktop.events.window_dispatch :: (arcana_desktop.events.window_id_value :: window_id :: call), event :: call
    return Option.Some[arcana_desktop.types.WindowDispatchEvent] :: dispatch :: call

fn lift_window_event(read raw: std.events.AppEvent) -> Option[arcana_desktop.types.WindowDispatchEvent]:
    return match raw:
        std.events.AppEvent.WindowResized(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.WindowResized :: (arcana_desktop.types.WindowResizeEvent :: window_id = ev.window_id, size = ev.size :: call) :: call) :: call
        std.events.AppEvent.WindowMoved(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.WindowMoved :: (arcana_desktop.types.WindowMoveEvent :: window_id = ev.window_id, position = ev.position :: call) :: call) :: call
        std.events.AppEvent.WindowCloseRequested(window_id) => arcana_desktop.events.some_window_dispatch :: window_id, (arcana_desktop.types.WindowEvent.WindowCloseRequested :: window_id :: call) :: call
        std.events.AppEvent.WindowFocused(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.WindowFocused :: (arcana_desktop.types.WindowFocusEvent :: window_id = ev.window_id, focused = ev.focused :: call) :: call) :: call
        std.events.AppEvent.WindowRedrawRequested(window_id) => arcana_desktop.events.some_window_dispatch :: window_id, (arcana_desktop.types.WindowEvent.WindowRedrawRequested :: window_id :: call) :: call
        std.events.AppEvent.WindowScaleFactorChanged(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.WindowScaleFactorChanged :: (arcana_desktop.types.WindowScaleFactorEvent :: window_id = ev.window_id, scale_factor_milli = ev.scale_factor_milli :: call) :: call) :: call
        std.events.AppEvent.WindowThemeChanged(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.WindowThemeChanged :: (arcana_desktop.types.WindowThemeEvent :: window_id = ev.window_id, theme_code = ev.theme_code :: call) :: call) :: call
        std.events.AppEvent.KeyDown(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.KeyDown :: (arcana_desktop.events.key_event :: ev :: call) :: call) :: call
        std.events.AppEvent.KeyUp(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.KeyUp :: (arcana_desktop.events.key_event :: ev :: call) :: call) :: call
        std.events.AppEvent.MouseDown(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.MouseDown :: (arcana_desktop.events.mouse_button_event :: ev :: call) :: call) :: call
        std.events.AppEvent.MouseUp(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.MouseUp :: (arcana_desktop.events.mouse_button_event :: ev :: call) :: call) :: call
        std.events.AppEvent.MouseMove(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.MouseMove :: (arcana_desktop.events.mouse_move_event :: ev :: call) :: call) :: call
        std.events.AppEvent.MouseWheel(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.MouseWheel :: (arcana_desktop.events.mouse_wheel_event :: ev :: call) :: call) :: call
        std.events.AppEvent.MouseEntered(window_id) => arcana_desktop.events.some_window_dispatch :: window_id, (arcana_desktop.types.WindowEvent.MouseEntered :: window_id :: call) :: call
        std.events.AppEvent.MouseLeft(window_id) => arcana_desktop.events.some_window_dispatch :: window_id, (arcana_desktop.types.WindowEvent.MouseLeft :: window_id :: call) :: call
        std.events.AppEvent.TextInput(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.TextInput :: (arcana_desktop.events.text_input_event :: ev :: call) :: call) :: call
        std.events.AppEvent.TextCompositionStarted(window_id) => arcana_desktop.events.some_window_dispatch :: window_id, (arcana_desktop.types.WindowEvent.TextCompositionStarted :: window_id :: call) :: call
        std.events.AppEvent.TextCompositionUpdated(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.TextCompositionUpdated :: (arcana_desktop.events.text_composition_event :: ev :: call) :: call) :: call
        std.events.AppEvent.TextCompositionCommitted(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.TextCompositionCommitted :: (arcana_desktop.events.text_composition_event :: ev :: call) :: call) :: call
        std.events.AppEvent.TextCompositionCancelled(window_id) => arcana_desktop.events.some_window_dispatch :: window_id, (arcana_desktop.types.WindowEvent.TextCompositionCancelled :: window_id :: call) :: call
        std.events.AppEvent.FileDropped(ev) => arcana_desktop.events.some_window_dispatch :: ev.window_id, (arcana_desktop.types.WindowEvent.FileDropped :: (arcana_desktop.events.file_drop_event :: ev :: call) :: call) :: call
        _ => Option.None[arcana_desktop.types.WindowDispatchEvent] :: :: call

fn lift_device_event(read raw: std.events.AppEvent) -> Option[arcana_desktop.types.DeviceEvent]:
    return match raw:
        std.events.AppEvent.RawMouseMotion(ev) => Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawMouseMotion :: (arcana_desktop.events.raw_mouse_motion_event :: ev :: call) :: call) :: call
        std.events.AppEvent.RawMouseButton(ev) => Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawMouseButton :: (arcana_desktop.events.raw_mouse_button_event :: ev :: call) :: call) :: call
        std.events.AppEvent.RawMouseWheel(ev) => Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawMouseWheel :: (arcana_desktop.events.raw_mouse_wheel_event :: ev :: call) :: call) :: call
        std.events.AppEvent.RawKey(ev) => Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawKey :: (arcana_desktop.events.raw_key_event :: ev :: call) :: call) :: call
        _ => Option.None[arcana_desktop.types.DeviceEvent] :: :: call

fn lift_dispatch_event(read raw: std.events.AppEvent) -> arcana_desktop.types.AppEvent:
    return match (arcana_desktop.events.lift_window_event :: raw :: call):
        Option.Some(value) => arcana_desktop.types.AppEvent.Window :: value :: call
        Option.None => match (arcana_desktop.events.lift_device_event :: raw :: call):
            Option.Some(value) => arcana_desktop.types.AppEvent.Device :: value :: call
            Option.None => arcana_desktop.types.AppEvent.Unknown :: (std.events.kind :: raw :: call) :: call

fn lift_event(read raw: std.events.AppEvent) -> arcana_desktop.types.AppEvent:
    return match raw:
        std.events.AppEvent.AppResumed => arcana_desktop.types.AppEvent.AppResumed :: :: call
        std.events.AppEvent.Wake => arcana_desktop.types.AppEvent.Wake :: :: call
        std.events.AppEvent.AppSuspended => arcana_desktop.types.AppEvent.AppSuspended :: :: call
        std.events.AppEvent.AboutToWait => arcana_desktop.types.AppEvent.AboutToWait :: :: call
        std.events.AppEvent.Unknown(kind) => arcana_desktop.types.AppEvent.Unknown :: kind :: call
        _ => arcana_desktop.events.lift_dispatch_event :: raw :: call

fn push_drained_event(edit out: List[arcana_desktop.types.AppEvent], read next: Option[arcana_desktop.types.AppEvent]) -> Bool:
    return match next:
        Option.Some(value) => push_drained_event_ready :: out, value :: call
        Option.None => false

fn push_drained_event_ready(edit out: List[arcana_desktop.types.AppEvent], read value: arcana_desktop.types.AppEvent) -> Bool:
    out :: value :: push
    return true

export fn poll(edit frame: arcana_desktop.types.FrameInput) -> Option[arcana_desktop.types.AppEvent]:
    let raw = std.events.poll :: frame :: call
    return match raw:
        Option.Some(value) => Option.Some[arcana_desktop.types.AppEvent] :: (arcana_desktop.events.lift_event :: value :: call) :: call
        Option.None => Option.None[arcana_desktop.types.AppEvent] :: :: call

export fn drain(take frame: arcana_desktop.types.FrameInput) -> List[arcana_desktop.types.AppEvent]:
    let mut current = frame
    let mut out = std.collections.list.new[arcana_desktop.types.AppEvent] :: :: call
    while true:
        let next = arcana_desktop.events.poll :: current :: call
        if not (arcana_desktop.events.push_drained_event :: out, next :: call):
            return out
    return out

export fn pump(edit win: arcana_desktop.types.Window) -> arcana_desktop.types.FrameInput:
    return std.kernel.events.pump :: win :: call

export fn open_session() -> arcana_desktop.types.Session:
    return std.kernel.events.session_open :: :: call

export fn close_session(edit session: arcana_desktop.types.Session):
    std.kernel.events.session_close :: session :: call

export fn attach_window(edit session: arcana_desktop.types.Session, read win: arcana_desktop.types.Window):
    std.kernel.events.session_attach_window :: session, win :: call

export fn detach_window(edit session: arcana_desktop.types.Session, read win: arcana_desktop.types.Window):
    std.kernel.events.session_detach_window :: session, win :: call

export fn window_for_id(read session: arcana_desktop.types.Session, read window_id: arcana_desktop.types.WindowId) -> Option[arcana_desktop.types.Window]:
    return std.kernel.events.session_window_for_id :: session, window_id.value :: call

export fn window_ids(read session: arcana_desktop.types.Session) -> List[arcana_desktop.types.WindowId]:
    let raw = std.kernel.events.session_window_ids :: session :: call
    let mut out = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
    for value in raw:
        out :: (arcana_desktop.events.window_id_value :: value :: call) :: push
    return out

export fn device_events(edit session: arcana_desktop.types.Session) -> arcana_desktop.types.DeviceEvents:
    return arcana_desktop.events.lift_device_events :: (std.kernel.events.session_device_events :: session :: call) :: call

export fn set_device_events(edit session: arcana_desktop.types.Session, read value: arcana_desktop.types.DeviceEvents):
    std.kernel.events.session_set_device_events :: session, (arcana_desktop.events.device_events_code :: value :: call) :: call

export fn window_id(read event: arcana_desktop.types.AppEvent) -> Option[arcana_desktop.types.WindowId]:
    return match event:
        arcana_desktop.types.AppEvent.Window(ev) => Option.Some[arcana_desktop.types.WindowId] :: ev.window_id :: call
        _ => Option.None[arcana_desktop.types.WindowId] :: :: call

export fn device_id(read event: arcana_desktop.types.DeviceEvent) -> Option[arcana_desktop.types.DeviceId]:
    return match event:
        arcana_desktop.types.DeviceEvent.RawMouseMotion(ev) => ev.device_id
        arcana_desktop.types.DeviceEvent.RawMouseButton(ev) => ev.device_id
        arcana_desktop.types.DeviceEvent.RawMouseWheel(ev) => ev.device_id
        arcana_desktop.types.DeviceEvent.RawKey(ev) => ev.device_id

export fn event_window(read session: arcana_desktop.types.Session, read event: arcana_desktop.types.AppEvent) -> Option[arcana_desktop.types.Window]:
    return match (arcana_desktop.events.window_id :: event :: call):
        Option.Some(id) => arcana_desktop.events.window_for_id :: session, id :: call
        Option.None => Option.None[arcana_desktop.types.Window] :: :: call

export fn pump_session(edit session: arcana_desktop.types.Session) -> arcana_desktop.types.FrameInput:
    return std.kernel.events.session_pump :: session :: call

export fn wait_session(edit session: arcana_desktop.types.Session, timeout_ms: Int) -> arcana_desktop.types.FrameInput:
    return std.kernel.events.session_wait :: session, timeout_ms :: call

export fn create_wake(edit session: arcana_desktop.types.Session) -> arcana_desktop.types.WakeHandle:
    return std.kernel.events.session_create_wake :: session :: call

export fn wake(read handle: arcana_desktop.types.WakeHandle):
    std.kernel.events.wake_signal :: handle :: call
