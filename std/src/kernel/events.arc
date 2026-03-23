import std.option
use std.events.AppFrame
use std.events.AppSession
use std.events.WakeHandle
use std.option.Option
use std.window.Window

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

intrinsic fn pump_frame(edit win: Window) -> AppFrame = EventsPump
intrinsic fn poll_frame(edit frame: AppFrame) -> Option[std.kernel.events.EventRaw] = EventsPoll
intrinsic fn session_open() -> AppSession = EventsSessionOpen
intrinsic fn session_close(edit session: AppSession) = EventsSessionClose
intrinsic fn session_attach_window(edit session: AppSession, read win: Window) = EventsSessionAttachWindow
intrinsic fn session_detach_window(edit session: AppSession, read win: Window) = EventsSessionDetachWindow
intrinsic fn session_window_for_id(read session: AppSession, window_id: Int) -> Option[Window] = EventsSessionWindowById
intrinsic fn session_window_ids(read session: AppSession) -> List[Int] = EventsSessionWindowIds
intrinsic fn session_pump(edit session: AppSession) -> AppFrame = EventsSessionPump
intrinsic fn session_wait(edit session: AppSession, timeout_ms: Int) -> AppFrame = EventsSessionWait
intrinsic fn session_device_events(edit session: AppSession) -> Int = EventsSessionDeviceEvents
intrinsic fn session_set_device_events(edit session: AppSession, policy: Int) = EventsSessionSetDeviceEvents
intrinsic fn session_create_wake(edit session: AppSession) -> WakeHandle = EventsSessionCreateWake
intrinsic fn wake_signal(read wake: WakeHandle) = EventsWakeSignal

fn pump(edit win: Window) -> AppFrame:
    return std.kernel.events.pump_frame :: win :: call

fn poll(edit frame: AppFrame) -> Option[std.kernel.events.EventRaw]:
    return std.kernel.events.poll_frame :: frame :: call
