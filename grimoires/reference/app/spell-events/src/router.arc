import std.events
import std.collections.list
use std.window.Window

export fn poll(edit win: Window) -> std.option.Option[std.events.AppEvent]:
    let mut frame = std.events.pump :: win :: call
    return std.events.poll :: frame :: call

export fn drain(edit win: Window) -> List[std.events.AppEvent]:
    let frame = std.events.pump :: win :: call
    return std.events.drain :: frame :: call

export fn count(edit win: Window) -> Int:
    let frame = std.events.pump :: win :: call
    let all = std.events.drain :: frame :: call
    let mut n = 0
    for _ev in all:
        n += 1
    return n
