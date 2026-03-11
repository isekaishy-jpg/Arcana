import std.events
import std.collections.list

export fn poll(read win: Window) -> std.option.Option[std.events.AppEvent]:
    return std.events.poll :: win :: call

export fn drain(read win: Window) -> List[std.events.AppEvent]:
    return std.events.drain :: win :: call

export fn count(read win: Window) -> Int:
    let all = std.events.drain :: win :: call
    let mut n = 0
    for _ev in all:
        n += 1
    return n
