import winspell.input

export record FrameInput:
    mouse_x: Int
    mouse_y: Int
    wheel_y: Int

export fn snapshot(read win: Window) -> FrameInput:
    let pos = winspell.input.mouse_pos :: win :: call
    return spell_events.frame_input.FrameInput :: mouse_x = pos.0, mouse_y = pos.1, wheel_y = (winspell.input.mouse_wheel_y :: win :: call) :: call
