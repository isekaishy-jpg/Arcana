import std.result
use std.canvas.Image
use std.events.AppFrame
use std.result.Result
use std.window.Window

intrinsic fn window_open(title: Str, width: Int, height: Int) -> Result[Window, Str] = WindowOpenTry
intrinsic fn canvas_alive(read win: Window) -> Bool = CanvasAlive
intrinsic fn canvas_fill(edit win: Window, color: Int) = CanvasFill
intrinsic fn canvas_rect(edit win: Window, x: Int, y: Int, w: Int, h: Int, color: Int) = CanvasRect
intrinsic fn canvas_line(edit win: Window, x1: Int, y1: Int, x2: Int, y2: Int, color: Int) = CanvasLine
intrinsic fn canvas_circle_fill(edit win: Window, x: Int, y: Int, radius: Int, color: Int) = CanvasCircleFill
intrinsic fn canvas_label(edit win: Window, x: Int, y: Int, text: Str, color: Int) = CanvasLabel
intrinsic fn canvas_label_size(text: Str) -> (Int, Int) = CanvasLabelSize
intrinsic fn canvas_present(edit win: Window) = CanvasPresent
intrinsic fn canvas_rgb(r: Int, g: Int, b: Int) -> Int = CanvasRgb
intrinsic fn image_load(path: Str) -> Result[Image, Str] = ImageLoadTry
intrinsic fn canvas_image_size(read img: Image) -> (Int, Int) = CanvasImageSize
intrinsic fn canvas_blit(edit win: Window, read img: Image, x: Int, y: Int) = CanvasBlit
intrinsic fn canvas_blit_scaled(edit win: Window, read img: Image, x: Int, y: Int, w: Int, h: Int) = CanvasBlitScaled
intrinsic fn canvas_blit_region(edit win: Window, read img: Image, sx: Int, sy: Int, sw: Int, sh: Int, dx: Int, dy: Int, dw: Int, dh: Int) = CanvasBlitRegion

intrinsic fn window_size(read win: Window) -> (Int, Int) = WindowSize
intrinsic fn window_resized(read win: Window) -> Bool = WindowResized
intrinsic fn window_fullscreen(read win: Window) -> Bool = WindowFullscreen
intrinsic fn window_minimized(read win: Window) -> Bool = WindowMinimized
intrinsic fn window_maximized(read win: Window) -> Bool = WindowMaximized
intrinsic fn window_focused(read win: Window) -> Bool = WindowFocused
intrinsic fn window_id(read win: Window) -> Int = WindowId
intrinsic fn window_position(read win: Window) -> (Int, Int) = WindowPosition
intrinsic fn window_title(read win: Window) -> Str = WindowTitle
intrinsic fn window_visible(read win: Window) -> Bool = WindowVisible
intrinsic fn window_decorated(read win: Window) -> Bool = WindowDecorated
intrinsic fn window_resizable(read win: Window) -> Bool = WindowResizable
intrinsic fn window_topmost(read win: Window) -> Bool = WindowTopmost
intrinsic fn window_cursor_visible(read win: Window) -> Bool = WindowCursorVisible
intrinsic fn window_min_size(read win: Window) -> (Int, Int) = WindowMinSize
intrinsic fn window_max_size(read win: Window) -> (Int, Int) = WindowMaxSize
intrinsic fn window_scale_factor_milli(read win: Window) -> Int = WindowScaleFactorMilli
intrinsic fn window_theme_code(read win: Window) -> Int = WindowThemeCode
intrinsic fn window_transparent(read win: Window) -> Bool = WindowTransparent
intrinsic fn window_theme_override_code(read win: Window) -> Int = WindowThemeOverrideCode
intrinsic fn window_cursor_icon_code(read win: Window) -> Int = WindowCursorIconCode
intrinsic fn window_cursor_grab_mode(read win: Window) -> Int = WindowCursorGrabMode
intrinsic fn window_cursor_position(read win: Window) -> (Int, Int) = WindowCursorPosition
intrinsic fn window_text_input_enabled(read win: Window) -> Bool = WindowTextInputEnabled
intrinsic fn window_current_monitor_index(read win: Window) -> Int = WindowCurrentMonitorIndex
intrinsic fn window_primary_monitor_index() -> Int = WindowPrimaryMonitorIndex
intrinsic fn window_monitor_count() -> Int = WindowMonitorCount
intrinsic fn window_monitor_name(index: Int) -> Str = WindowMonitorName
intrinsic fn window_monitor_position(index: Int) -> (Int, Int) = WindowMonitorPosition
intrinsic fn window_monitor_size(index: Int) -> (Int, Int) = WindowMonitorSize
intrinsic fn window_monitor_scale_factor_milli(index: Int) -> Int = WindowMonitorScaleFactorMilli
intrinsic fn window_monitor_is_primary(index: Int) -> Bool = WindowMonitorIsPrimary
intrinsic fn window_set_title(edit win: Window, title: Str) = WindowSetTitle
intrinsic fn window_set_position(edit win: Window, x: Int, y: Int) = WindowSetPosition
intrinsic fn window_set_size(edit win: Window, width: Int, height: Int) = WindowSetSize
intrinsic fn window_set_visible(edit win: Window, enabled: Bool) = WindowSetVisible
intrinsic fn window_set_decorated(edit win: Window, enabled: Bool) = WindowSetDecorated
intrinsic fn window_set_resizable(edit win: Window, enabled: Bool) = WindowSetResizable
intrinsic fn window_set_min_size(edit win: Window, width: Int, height: Int) = WindowSetMinSize
intrinsic fn window_set_max_size(edit win: Window, width: Int, height: Int) = WindowSetMaxSize
intrinsic fn window_set_fullscreen(edit win: Window, enabled: Bool) = WindowSetFullscreen
intrinsic fn window_set_minimized(edit win: Window, enabled: Bool) = WindowSetMinimized
intrinsic fn window_set_maximized(edit win: Window, enabled: Bool) = WindowSetMaximized
intrinsic fn window_set_topmost(edit win: Window, enabled: Bool) = WindowSetTopmost
intrinsic fn window_set_cursor_visible(edit win: Window, enabled: Bool) = WindowSetCursorVisible
intrinsic fn window_set_transparent(edit win: Window, enabled: Bool) = WindowSetTransparent
intrinsic fn window_set_theme_override_code(edit win: Window, code: Int) = WindowSetThemeOverrideCode
intrinsic fn window_set_cursor_icon_code(edit win: Window, code: Int) = WindowSetCursorIconCode
intrinsic fn window_set_cursor_grab_mode(edit win: Window, mode: Int) = WindowSetCursorGrabMode
intrinsic fn window_set_cursor_position(edit win: Window, x: Int, y: Int) = WindowSetCursorPosition
intrinsic fn window_set_text_input_enabled(edit win: Window, enabled: Bool) = WindowTextInputSetEnabled
intrinsic fn window_request_redraw(edit win: Window) = WindowRequestRedraw
intrinsic fn window_request_attention(edit win: Window, enabled: Bool) = WindowRequestAttention
intrinsic fn window_close(take win: Window) -> Result[Unit, Str] = WindowClose

intrinsic fn input_key_code(name: Str) -> Int = InputKeyCode
intrinsic fn input_key_down(read frame: AppFrame, key: Int) -> Bool = InputKeyDown
intrinsic fn input_key_pressed(read frame: AppFrame, key: Int) -> Bool = InputKeyPressed
intrinsic fn input_key_released(read frame: AppFrame, key: Int) -> Bool = InputKeyReleased
intrinsic fn input_mouse_button_code(name: Str) -> Int = InputMouseButtonCode
intrinsic fn input_mouse_pos(read frame: AppFrame) -> (Int, Int) = InputMousePos
intrinsic fn input_mouse_down(read frame: AppFrame, button: Int) -> Bool = InputMouseDown
intrinsic fn input_mouse_pressed(read frame: AppFrame, button: Int) -> Bool = InputMousePressed
intrinsic fn input_mouse_released(read frame: AppFrame, button: Int) -> Bool = InputMouseReleased
intrinsic fn input_mouse_wheel_y(read frame: AppFrame) -> Int = InputMouseWheelY
intrinsic fn input_mouse_in_window(read frame: AppFrame) -> Bool = InputMouseInWindow
