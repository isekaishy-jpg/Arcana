use std.window.Window

intrinsic fn composition_area_active(read win: Window) -> Bool = TextInputCompositionAreaActive
intrinsic fn composition_area_position(read win: Window) -> (Int, Int) = TextInputCompositionAreaPosition
intrinsic fn composition_area_size(read win: Window) -> (Int, Int) = TextInputCompositionAreaSize
intrinsic fn set_composition_area(edit win: Window, position: (Int, Int), size: (Int, Int)) = TextInputSetCompositionArea
intrinsic fn clear_composition_area(edit win: Window) = TextInputClearCompositionArea
