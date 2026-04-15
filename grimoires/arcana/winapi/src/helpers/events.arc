import std.collections.list
import std.option
use arcana_winapi.desktop_handles.FrameInput
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

export native fn wake_create() -> WakeHandle = helpers.events.wake_create
export native fn wake_close(edit handle: WakeHandle) = helpers.events.wake_close
export native fn wake_signal(read handle: WakeHandle) = helpers.events.wake_signal
export native fn wake_take_pending(edit handle: WakeHandle) -> Int = helpers.events.wake_take_pending
export native fn wait_wake_or_messages(read handle: WakeHandle, timeout_ms: Int) -> Bool = helpers.events.wait_wake_or_messages

