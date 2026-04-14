use arcana_winapi.desktop_handles.Window

native fn composition_area_x(read win: Window) -> Int = helpers.text_input.composition_area_x
native fn composition_area_y(read win: Window) -> Int = helpers.text_input.composition_area_y
native fn composition_area_width(read win: Window) -> Int = helpers.text_input.composition_area_width
native fn composition_area_height(read win: Window) -> Int = helpers.text_input.composition_area_height
native fn set_composition_area_position_raw(edit win: Window, x: Int, y: Int) = helpers.text_input.set_composition_area_position
native fn set_composition_area_size_raw(edit win: Window, width: Int, height: Int) = helpers.text_input.set_composition_area_size

export native fn window_text_input_enabled(read win: Window) -> Bool = helpers.text_input.window_text_input_enabled
export native fn window_set_text_input_enabled(edit win: Window, enabled: Bool) = helpers.text_input.window_set_text_input_enabled
export native fn composition_area_active(read win: Window) -> Bool = helpers.text_input.composition_area_active

export fn composition_area_position(read win: Window) -> (Int, Int):
    return ((composition_area_x :: win :: call), (composition_area_y :: win :: call))

export fn composition_area_size(read win: Window) -> (Int, Int):
    return ((composition_area_width :: win :: call), (composition_area_height :: win :: call))

export fn set_composition_area(edit win: Window, position: (Int, Int), size: (Int, Int)):
    set_composition_area_position_raw :: win, position.0, position.1 :: call
    set_composition_area_size_raw :: win, size.0, size.1 :: call

export native fn clear_composition_area(edit win: Window) = helpers.text_input.clear_composition_area

