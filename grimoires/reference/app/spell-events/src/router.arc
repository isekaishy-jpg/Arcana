import std.events
import std.collections.list

export fn poll(read win: Window) -> std.option.Option[std.events.AppEvent]:
    let frame = std.events.pump :: win :: call
    return std.events.poll :: frame :: call

export fn drain(read win: Window) -> List[std.events.AppEvent]:
    let frame = std.events.pump :: win :: call
    return std.events.drain :: frame :: call

export fn count(read win: Window) -> Int:
    let frame = std.events.pump :: win :: call
    let all = std.events.drain :: frame :: call
    let mut n = 0
    for _ev in all:
        n += 1
    return n
