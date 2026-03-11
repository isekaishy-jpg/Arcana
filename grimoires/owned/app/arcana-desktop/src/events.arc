import std.events

export fn poll(read win: Window) -> std.option.Option[std.events.AppEvent]:
    return std.events.poll :: win :: call

export fn drain(read win: Window) -> List[std.events.AppEvent]:
    return std.events.drain :: win :: call
