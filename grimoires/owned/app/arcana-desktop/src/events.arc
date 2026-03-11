import std.events

export fn poll(read frame: std.events.AppFrame) -> std.option.Option[std.events.AppEvent]:
    return std.events.poll :: frame :: call

export fn drain(read frame: std.events.AppFrame) -> List[std.events.AppEvent]:
    return std.events.drain :: frame :: call

export fn pump(read win: Window) -> std.events.AppFrame:
    return std.events.pump :: win :: call
