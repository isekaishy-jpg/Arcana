import std.events
use std.events.AppFrame
use std.window.Window

export fn poll(edit frame: AppFrame) -> std.option.Option[std.events.AppEvent]:
    return std.events.poll :: frame :: call

export fn drain(take frame: AppFrame) -> List[std.events.AppEvent]:
    return std.events.drain :: frame :: call

export fn pump(edit win: Window) -> AppFrame:
    return std.events.pump :: win :: call
