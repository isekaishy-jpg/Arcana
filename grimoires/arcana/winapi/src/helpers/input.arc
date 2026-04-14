use arcana_winapi.desktop_handles.FrameInput

native fn input_mouse_x(read frame: FrameInput) -> Int = helpers.input.input_mouse_x
native fn input_mouse_y(read frame: FrameInput) -> Int = helpers.input.input_mouse_y

export native fn input_key_code(name: Str) -> Int = helpers.input.input_key_code
export native fn input_key_down(read frame: FrameInput, key: Int) -> Bool = helpers.input.input_key_down
export native fn input_key_pressed(read frame: FrameInput, key: Int) -> Bool = helpers.input.input_key_pressed
export native fn input_key_released(read frame: FrameInput, key: Int) -> Bool = helpers.input.input_key_released
export native fn input_mouse_button_code(name: Str) -> Int = helpers.input.input_mouse_button_code

export fn input_mouse_pos(read frame: FrameInput) -> (Int, Int):
    return ((input_mouse_x :: frame :: call), (input_mouse_y :: frame :: call))

export native fn input_mouse_down(read frame: FrameInput, button: Int) -> Bool = helpers.input.input_mouse_down
export native fn input_mouse_pressed(read frame: FrameInput, button: Int) -> Bool = helpers.input.input_mouse_pressed
export native fn input_mouse_released(read frame: FrameInput, button: Int) -> Bool = helpers.input.input_mouse_released
export native fn input_mouse_wheel_y(read frame: FrameInput) -> Int = helpers.input.input_mouse_wheel_y
export native fn input_mouse_in_window(read frame: FrameInput) -> Bool = helpers.input.input_mouse_in_window

