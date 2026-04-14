import arcana_desktop.types
import arcana_winapi.helpers.events
import std.collections.list
import std.option
use std.option.Option

fn window_id_value(id: Int) -> arcana_desktop.types.WindowId:
    return arcana_desktop.types.WindowId :: value = id :: call

fn some_device_id(id: Int) -> Option[arcana_desktop.types.DeviceId]:
    if id == 0:
        return Option.None[arcana_desktop.types.DeviceId] :: :: call
    return Option.Some[arcana_desktop.types.DeviceId] :: (arcana_desktop.types.DeviceId :: value = id :: call) :: call

fn key_meta(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.KeyMeta:
    let mut meta = arcana_desktop.types.KeyMeta :: modifiers = raw.flags, repeated = raw.repeated, physical_key = raw.physical_key :: call
    meta.logical_key = raw.logical_key
    meta.location = raw.key_location
    meta.text = raw.text
    return meta

fn key_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.KeyEvent:
    return arcana_desktop.types.KeyEvent :: window_id = raw.window_id, key = raw.key_code, meta = (arcana_desktop.events.key_meta :: raw :: call) :: call

fn mouse_button_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.MouseButtonEvent:
    let mut event = arcana_desktop.types.MouseButtonEvent :: window_id = raw.window_id, button = raw.a, position = (raw.pointer_x, raw.pointer_y) :: call
    event.modifiers = raw.flags
    return event

fn mouse_move_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.MouseMoveEvent:
    return arcana_desktop.types.MouseMoveEvent :: window_id = raw.window_id, position = (raw.a, raw.b), modifiers = raw.flags :: call

fn mouse_wheel_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.MouseWheelEvent:
    return arcana_desktop.types.MouseWheelEvent :: window_id = raw.window_id, delta = (0, raw.a), modifiers = raw.flags :: call

fn text_input_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.TextInputEvent:
    return arcana_desktop.types.TextInputEvent :: window_id = raw.window_id, text = raw.text :: call

fn text_composition_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.TextCompositionEvent:
    return arcana_desktop.types.TextCompositionEvent :: window_id = raw.window_id, text = raw.text, caret = raw.a :: call

fn file_drop_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.FileDropEvent:
    return arcana_desktop.types.FileDropEvent :: window_id = raw.window_id, path = raw.text :: call

fn raw_mouse_motion_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.RawMouseMotionEvent:
    return arcana_desktop.types.RawMouseMotionEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.window_id :: call), delta = (raw.a, raw.b) :: call

fn raw_mouse_button_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.RawMouseButtonEvent:
    return arcana_desktop.types.RawMouseButtonEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.window_id :: call), button = raw.a, pressed = (raw.b != 0) :: call

fn raw_mouse_wheel_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.RawMouseWheelEvent:
    return arcana_desktop.types.RawMouseWheelEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.window_id :: call), delta = (raw.a, raw.b) :: call

fn raw_key_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.RawKeyEvent:
    let mut event = arcana_desktop.types.RawKeyEvent :: device_id = (arcana_desktop.events.some_device_id :: raw.window_id :: call), key = raw.key_code, meta = (arcana_desktop.events.key_meta :: raw :: call) :: call
    event.pressed = raw.b != 0
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

fn lift_window_event(read raw: arcana_winapi.helpers.events.EventRaw) -> Option[arcana_desktop.types.WindowDispatchEvent]:
    if raw.kind == 1:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.WindowResized :: (arcana_desktop.types.WindowResizeEvent :: window_id = raw.window_id, size = (raw.a, raw.b) :: call) :: call) :: call
    if raw.kind == 10:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.WindowMoved :: (arcana_desktop.types.WindowMoveEvent :: window_id = raw.window_id, position = (raw.a, raw.b) :: call) :: call) :: call
    if raw.kind == 2:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.WindowCloseRequested :: raw.window_id :: call) :: call
    if raw.kind == 3:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.WindowFocused :: (arcana_desktop.types.WindowFocusEvent :: window_id = raw.window_id, focused = (raw.a != 0) :: call) :: call) :: call
    if raw.kind == 13:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.WindowRedrawRequested :: raw.window_id :: call) :: call
    if raw.kind == 16:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.WindowScaleFactorChanged :: (arcana_desktop.types.WindowScaleFactorEvent :: window_id = raw.window_id, scale_factor_milli = raw.a :: call) :: call) :: call
    if raw.kind == 17:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.WindowThemeChanged :: (arcana_desktop.types.WindowThemeEvent :: window_id = raw.window_id, theme_code = raw.a :: call) :: call) :: call
    if raw.kind == 4:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.KeyDown :: (arcana_desktop.events.key_event :: raw :: call) :: call) :: call
    if raw.kind == 5:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.KeyUp :: (arcana_desktop.events.key_event :: raw :: call) :: call) :: call
    if raw.kind == 6:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.MouseDown :: (arcana_desktop.events.mouse_button_event :: raw :: call) :: call) :: call
    if raw.kind == 7:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.MouseUp :: (arcana_desktop.events.mouse_button_event :: raw :: call) :: call) :: call
    if raw.kind == 8:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.MouseMove :: (arcana_desktop.events.mouse_move_event :: raw :: call) :: call) :: call
    if raw.kind == 9:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.MouseWheel :: (arcana_desktop.events.mouse_wheel_event :: raw :: call) :: call) :: call
    if raw.kind == 11:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.MouseEntered :: raw.window_id :: call) :: call
    if raw.kind == 12:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.MouseLeft :: raw.window_id :: call) :: call
    if raw.kind == 14:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.TextInput :: (arcana_desktop.events.text_input_event :: raw :: call) :: call) :: call
    if raw.kind == 24:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.TextCompositionStarted :: raw.window_id :: call) :: call
    if raw.kind == 25:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.TextCompositionUpdated :: (arcana_desktop.events.text_composition_event :: raw :: call) :: call) :: call
    if raw.kind == 26:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.TextCompositionCommitted :: (arcana_desktop.events.text_composition_event :: raw :: call) :: call) :: call
    if raw.kind == 27:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.TextCompositionCancelled :: raw.window_id :: call) :: call
    if raw.kind == 15:
        return arcana_desktop.events.some_window_dispatch :: raw.window_id, (arcana_desktop.types.WindowEvent.FileDropped :: (arcana_desktop.events.file_drop_event :: raw :: call) :: call) :: call
    return Option.None[arcana_desktop.types.WindowDispatchEvent] :: :: call

fn lift_device_event(read raw: arcana_winapi.helpers.events.EventRaw) -> Option[arcana_desktop.types.DeviceEvent]:
    if raw.kind == 18:
        return Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawMouseMotion :: (arcana_desktop.events.raw_mouse_motion_event :: raw :: call) :: call) :: call
    if raw.kind == 19:
        return Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawMouseButton :: (arcana_desktop.events.raw_mouse_button_event :: raw :: call) :: call) :: call
    if raw.kind == 28:
        return Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawMouseWheel :: (arcana_desktop.events.raw_mouse_wheel_event :: raw :: call) :: call) :: call
    if raw.kind == 29:
        return Option.Some[arcana_desktop.types.DeviceEvent] :: (arcana_desktop.types.DeviceEvent.RawKey :: (arcana_desktop.events.raw_key_event :: raw :: call) :: call) :: call
    return Option.None[arcana_desktop.types.DeviceEvent] :: :: call

fn lift_dispatch_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.AppEvent:
    return match (arcana_desktop.events.lift_window_event :: raw :: call):
        Option.Some(value) => arcana_desktop.types.AppEvent.Window :: value :: call
        Option.None => match (arcana_desktop.events.lift_device_event :: raw :: call):
            Option.Some(value) => arcana_desktop.types.AppEvent.Device :: value :: call
            Option.None => arcana_desktop.types.AppEvent.Unknown :: raw.kind :: call

fn lift_event(read raw: arcana_winapi.helpers.events.EventRaw) -> arcana_desktop.types.AppEvent:
    if raw.kind == 20:
        return arcana_desktop.types.AppEvent.AppResumed :: :: call
    if raw.kind == 21:
        return arcana_desktop.types.AppEvent.Wake :: :: call
    if raw.kind == 22:
        return arcana_desktop.types.AppEvent.AppSuspended :: :: call
    if raw.kind == 23:
        return arcana_desktop.types.AppEvent.AboutToWait :: :: call
    return arcana_desktop.events.lift_dispatch_event :: raw :: call

fn push_drained_event(edit out: List[arcana_desktop.types.AppEvent], read next: Option[arcana_desktop.types.AppEvent]) -> Bool:
    return match next:
        Option.Some(value) => push_drained_event_ready :: out, value :: call
        Option.None => false

fn push_drained_event_ready(edit out: List[arcana_desktop.types.AppEvent], read value: arcana_desktop.types.AppEvent) -> Bool:
    out :: value :: push
    return true

export fn poll(edit frame: arcana_winapi.desktop_handles.FrameInput) -> Option[arcana_desktop.types.AppEvent]:
    let raw = arcana_winapi.helpers.events.poll :: frame :: call
    return match raw:
        Option.Some(value) => Option.Some[arcana_desktop.types.AppEvent] :: (arcana_desktop.events.lift_event :: value :: call) :: call
        Option.None => Option.None[arcana_desktop.types.AppEvent] :: :: call

export fn drain(take frame: arcana_winapi.desktop_handles.FrameInput) -> List[arcana_desktop.types.AppEvent]:
    let mut current = frame
    let mut out = std.collections.list.new[arcana_desktop.types.AppEvent] :: :: call
    while true:
        let next = arcana_desktop.events.poll :: current :: call
        if not (arcana_desktop.events.push_drained_event :: out, next :: call):
            return out
    return out

export fn pump(edit win: arcana_winapi.desktop_handles.Window) -> arcana_winapi.desktop_handles.FrameInput:
    return arcana_winapi.helpers.events.pump :: win :: call

export fn open_session() -> arcana_winapi.desktop_handles.Session:
    return arcana_winapi.helpers.events.session_open :: :: call

export fn close_session(edit session: arcana_winapi.desktop_handles.Session):
    arcana_winapi.helpers.events.session_close :: session :: call

export fn attach_window(edit session: arcana_winapi.desktop_handles.Session, read win: arcana_winapi.desktop_handles.Window):
    arcana_winapi.helpers.events.session_attach_window :: session, win :: call

export fn detach_window(edit session: arcana_winapi.desktop_handles.Session, read win: arcana_winapi.desktop_handles.Window):
    arcana_winapi.helpers.events.session_detach_window :: session, win :: call

export fn window_for_id(read session: arcana_winapi.desktop_handles.Session, read window_id: arcana_desktop.types.WindowId) -> Option[arcana_winapi.desktop_handles.Window]:
    return arcana_winapi.helpers.events.session_window_for_id :: session, window_id.value :: call

export fn window_ids(read session: arcana_winapi.desktop_handles.Session) -> List[arcana_desktop.types.WindowId]:
    let raw = arcana_winapi.helpers.events.session_window_ids :: session :: call
    let mut out = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
    for value in raw:
        out :: (arcana_desktop.events.window_id_value :: value :: call) :: push
    return out

export fn device_events(edit session: arcana_winapi.desktop_handles.Session) -> arcana_desktop.types.DeviceEvents:
    return arcana_desktop.events.lift_device_events :: (arcana_winapi.helpers.events.session_device_events :: session :: call) :: call

export fn set_device_events(edit session: arcana_winapi.desktop_handles.Session, read value: arcana_desktop.types.DeviceEvents):
    arcana_winapi.helpers.events.session_set_device_events :: session, (arcana_desktop.events.device_events_code :: value :: call) :: call

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

export fn event_window(read session: arcana_winapi.desktop_handles.Session, read event: arcana_desktop.types.AppEvent) -> Option[arcana_winapi.desktop_handles.Window]:
    return match (arcana_desktop.events.window_id :: event :: call):
        Option.Some(id) => arcana_desktop.events.window_for_id :: session, id :: call
        Option.None => Option.None[arcana_winapi.desktop_handles.Window] :: :: call

export fn pump_session(edit session: arcana_winapi.desktop_handles.Session) -> arcana_winapi.desktop_handles.FrameInput:
    return arcana_winapi.helpers.events.session_pump :: session :: call

export fn wait_session(edit session: arcana_winapi.desktop_handles.Session, timeout_ms: Int) -> arcana_winapi.desktop_handles.FrameInput:
    return arcana_winapi.helpers.events.session_wait :: session, timeout_ms :: call

export fn create_wake(edit session: arcana_winapi.desktop_handles.Session) -> arcana_winapi.desktop_handles.WakeHandle:
    return arcana_winapi.helpers.events.session_create_wake :: session :: call

export fn wake(read handle: arcana_winapi.desktop_handles.WakeHandle):
    arcana_winapi.helpers.events.wake_signal :: handle :: call



