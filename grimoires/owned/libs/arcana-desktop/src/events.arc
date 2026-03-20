import arcana_desktop.types
import std.events
use std.events.AppFrame
use std.window.Window

export fn poll(edit frame: AppFrame) -> std.option.Option[std.events.AppEvent]:
    return std.events.poll :: frame :: call

export fn drain(take frame: AppFrame) -> List[std.events.AppEvent]:
    return std.events.drain :: frame :: call

export fn pump(edit win: Window) -> AppFrame:
    return std.events.pump :: win :: call

export fn open_session() -> std.events.AppSession:
    return std.events.open_session :: :: call

export fn close_session(edit session: std.events.AppSession):
    std.events.close_session :: session :: call

export fn attach_window(edit session: std.events.AppSession, read win: Window):
    std.events.attach_window :: session, win :: call

export fn detach_window(edit session: std.events.AppSession, read win: Window):
    std.events.detach_window :: session, win :: call

export fn window_for_id(read session: std.events.AppSession, window_id: Int) -> std.option.Option[Window]:
    return std.events.window_for_id :: session, window_id :: call

export fn window_ids(read session: std.events.AppSession) -> List[Int]:
    return std.events.window_ids :: session :: call

export fn window_id(read event: std.events.AppEvent) -> std.option.Option[Int]:
    return std.events.window_id :: event :: call

export fn event_window(read session: std.events.AppSession, read event: std.events.AppEvent) -> std.option.Option[Window]:
    return std.events.event_window :: session, event :: call

export fn pump_session(edit session: std.events.AppSession) -> AppFrame:
    return std.events.pump_session :: session :: call

export fn wait_session(edit session: std.events.AppSession, timeout_ms: Int) -> AppFrame:
    return std.events.wait_session :: session, timeout_ms :: call

export fn create_wake(edit session: std.events.AppSession) -> arcana_desktop.types.WakeHandle:
    return arcana_desktop.types.WakeHandle :: raw = (std.events.create_wake :: session :: call) :: call

export fn wake(read handle: arcana_desktop.types.WakeHandle):
    std.events.wake :: handle.raw :: call
