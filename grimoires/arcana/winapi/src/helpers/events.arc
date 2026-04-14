import std.collections.list
import std.option
use arcana_winapi.desktop_handles.FrameInput
use arcana_winapi.desktop_handles.Session
use arcana_winapi.desktop_handles.WakeHandle
use arcana_winapi.desktop_handles.Window
use std.option.Option

export record EventRaw:
    kind: Int
    window_id: Int
    a: Int
    b: Int
    flags: Int
    text: Str
    key_code: Int
    physical_key: Int
    logical_key: Int
    key_location: Int
    pointer_x: Int
    pointer_y: Int
    repeated: Bool

native fn poll_kind(edit frame: FrameInput) -> Int = helpers.events.poll_kind
native fn poll_window_id(read frame: FrameInput) -> Int = helpers.events.poll_window_id
native fn poll_a(read frame: FrameInput) -> Int = helpers.events.poll_a
native fn poll_b(read frame: FrameInput) -> Int = helpers.events.poll_b
native fn poll_flags(read frame: FrameInput) -> Int = helpers.events.poll_flags
native fn poll_text(read frame: FrameInput) -> Str = helpers.events.poll_text
native fn poll_key_code(read frame: FrameInput) -> Int = helpers.events.poll_key_code
native fn poll_physical_key(read frame: FrameInput) -> Int = helpers.events.poll_physical_key
native fn poll_logical_key(read frame: FrameInput) -> Int = helpers.events.poll_logical_key
native fn poll_key_location(read frame: FrameInput) -> Int = helpers.events.poll_key_location
native fn poll_pointer_x(read frame: FrameInput) -> Int = helpers.events.poll_pointer_x
native fn poll_pointer_y(read frame: FrameInput) -> Int = helpers.events.poll_pointer_y
native fn poll_repeated(read frame: FrameInput) -> Bool = helpers.events.poll_repeated
native fn session_window_for_id_raw(read session: Session, window_id: Int) -> Window = helpers.events.session_window_for_id

fn event_raw_from_frame(read frame: FrameInput, kind: Int) -> EventRaw:
    let mut event = EventRaw :: kind = kind, window_id = (poll_window_id :: frame :: call), a = (poll_a :: frame :: call) :: call
    event.b = poll_b :: frame :: call
    event.flags = poll_flags :: frame :: call
    event.text = poll_text :: frame :: call
    event.key_code = poll_key_code :: frame :: call
    event.physical_key = poll_physical_key :: frame :: call
    event.logical_key = poll_logical_key :: frame :: call
    event.key_location = poll_key_location :: frame :: call
    event.pointer_x = poll_pointer_x :: frame :: call
    event.pointer_y = poll_pointer_y :: frame :: call
    event.repeated = poll_repeated :: frame :: call
    return event

export native fn pump(edit win: Window) -> FrameInput = helpers.events.pump

export fn poll(edit frame: FrameInput) -> Option[EventRaw]:
    let kind = poll_kind :: frame :: call
    if kind == 0:
        return Option.None[EventRaw] :: :: call
    return Option.Some[EventRaw] :: (event_raw_from_frame :: frame, kind :: call) :: call

export native fn session_open() -> Session = helpers.events.session_open
export native fn session_close(edit session: Session) = helpers.events.session_close
export native fn session_attach_window(edit session: Session, read win: Window) = helpers.events.session_attach_window
export native fn session_detach_window(edit session: Session, read win: Window) = helpers.events.session_detach_window

export fn session_window_for_id(read session: Session, window_id: Int) -> Option[Window]:
    let value = session_window_for_id_raw :: session, window_id :: call
    if arcana_winapi.helpers.window.window_alive :: value :: call:
        return Option.Some[Window] :: value :: call
    return Option.None[Window] :: :: call

native fn session_window_count(read session: Session) -> Int = helpers.events.session_window_count
native fn session_window_id_at(read session: Session, index: Int) -> Int = helpers.events.session_window_id_at
export native fn session_device_events(edit session: Session) -> Int = helpers.events.session_device_events
export native fn session_set_device_events(edit session: Session, policy: Int) = helpers.events.session_set_device_events
export native fn session_pump(edit session: Session) -> FrameInput = helpers.events.session_pump
export native fn session_wait(edit session: Session, timeout_ms: Int) -> FrameInput = helpers.events.session_wait
export native fn session_create_wake(edit session: Session) -> WakeHandle = helpers.events.session_create_wake
export native fn wake_signal(read handle: WakeHandle) = helpers.events.wake_signal

export fn session_window_ids(read session: Session) -> List[Int]:
    let count = arcana_winapi.helpers.events.session_window_count :: session :: call
    let mut out = std.collections.list.new[Int] :: :: call
    let mut index = 0
    while index < count:
        out :: (arcana_winapi.helpers.events.session_window_id_at :: session, index :: call) :: push
        index = index + 1
    return out

