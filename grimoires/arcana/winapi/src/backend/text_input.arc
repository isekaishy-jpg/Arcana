use arcana_winapi.backend.desktop_handles.Window

export native fn window_text_input_enabled(read win: Window) -> Bool = helpers.text_input.window_text_input_enabled
export native fn window_set_text_input_enabled(edit win: Window, enabled: Bool) = helpers.text_input.window_set_text_input_enabled
export native fn composition_area_active(read win: Window) -> Bool = helpers.text_input.composition_area_active
export native fn composition_area_x(read win: Window) -> Int = helpers.text_input.composition_area_x
export native fn composition_area_y(read win: Window) -> Int = helpers.text_input.composition_area_y
export native fn composition_area_width(read win: Window) -> Int = helpers.text_input.composition_area_width
export native fn composition_area_height(read win: Window) -> Int = helpers.text_input.composition_area_height
export native fn set_composition_area_position_raw(edit win: Window, x: Int, y: Int) = helpers.text_input.set_composition_area_position
export native fn set_composition_area_size_raw(edit win: Window, width: Int, height: Int) = helpers.text_input.set_composition_area_size
export native fn clear_composition_area(edit win: Window) = helpers.text_input.clear_composition_area
