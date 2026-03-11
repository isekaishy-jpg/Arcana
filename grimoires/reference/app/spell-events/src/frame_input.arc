import std.input

export record FrameInput:
    mouse_x: Int
    mouse_y: Int
    wheel_y: Int

export fn snapshot(read win: Window) -> FrameInput:
    let frame = std.input.begin_frame :: win :: call
    let pos = std.input.mouse_pos :: frame :: call
    return spell_events.frame_input.FrameInput :: mouse_x = pos.0, mouse_y = pos.1, wheel_y = (std.input.mouse_wheel_y :: frame :: call) :: call
