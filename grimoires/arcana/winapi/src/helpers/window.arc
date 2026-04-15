import std.result
use arcana_winapi.desktop_handles.Window
use std.result.Result

native fn take_last_error() -> Str = helpers.window.take_last_error
native fn window_open_raw(title: Str, width: Int, height: Int) -> Window = helpers.window.window_open
native fn window_close_raw(take win: Window) -> Bool = helpers.window.window_close
native fn window_width(read win: Window) -> Int = helpers.window.window_width
native fn window_height(read win: Window) -> Int = helpers.window.window_height
native fn window_x(read win: Window) -> Int = helpers.window.window_x
native fn window_y(read win: Window) -> Int = helpers.window.window_y
native fn window_min_width(read win: Window) -> Int = helpers.window.window_min_width
native fn window_min_height(read win: Window) -> Int = helpers.window.window_min_height
native fn window_max_width(read win: Window) -> Int = helpers.window.window_max_width
native fn window_max_height(read win: Window) -> Int = helpers.window.window_max_height
native fn window_cursor_x(read win: Window) -> Int = helpers.window.window_cursor_x
native fn window_cursor_y(read win: Window) -> Int = helpers.window.window_cursor_y
native fn window_monitor_x(index: Int) -> Int = helpers.window.window_monitor_x
native fn window_monitor_y(index: Int) -> Int = helpers.window.window_monitor_y
native fn window_monitor_width(index: Int) -> Int = helpers.window.window_monitor_width
native fn window_monitor_height(index: Int) -> Int = helpers.window.window_monitor_height

fn pair(x: Int, y: Int) -> (Int, Int):
    return (x, y)

export fn window_open(title: Str, width: Int, height: Int) -> Result[Window, Str]:
    let value = window_open_raw :: title, width, height :: call
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[Window, Str] :: value :: call
    return Result.Err[Window, Str] :: err :: call

export native fn window_alive(read win: Window) -> Bool = helpers.window.window_alive

export fn window_size(read win: Window) -> (Int, Int):
    return pair :: (window_width :: win :: call), (window_height :: win :: call) :: call

export native fn window_native_handle(read win: Window) -> arcana_winapi.raw.types.HWND = helpers.window.window_native_handle
export native fn window_resized(read win: Window) -> Bool = helpers.window.window_resized
export native fn window_fullscreen(read win: Window) -> Bool = helpers.window.window_fullscreen
export native fn window_minimized(read win: Window) -> Bool = helpers.window.window_minimized
export native fn window_maximized(read win: Window) -> Bool = helpers.window.window_maximized
export native fn window_focused(read win: Window) -> Bool = helpers.window.window_focused
export native fn window_id(read win: Window) -> Int = helpers.window.window_id

export fn window_position(read win: Window) -> (Int, Int):
    return pair :: (window_x :: win :: call), (window_y :: win :: call) :: call

export native fn window_title(read win: Window) -> Str = helpers.window.window_title
export native fn window_visible(read win: Window) -> Bool = helpers.window.window_visible
export native fn window_decorated(read win: Window) -> Bool = helpers.window.window_decorated
export native fn window_resizable(read win: Window) -> Bool = helpers.window.window_resizable
export native fn window_topmost(read win: Window) -> Bool = helpers.window.window_topmost
export native fn window_cursor_visible(read win: Window) -> Bool = helpers.window.window_cursor_visible

export fn window_min_size(read win: Window) -> (Int, Int):
    return pair :: (window_min_width :: win :: call), (window_min_height :: win :: call) :: call

export fn window_max_size(read win: Window) -> (Int, Int):
    return pair :: (window_max_width :: win :: call), (window_max_height :: win :: call) :: call

export native fn window_scale_factor_milli(read win: Window) -> Int = helpers.window.window_scale_factor_milli
export native fn window_theme_code(read win: Window) -> Int = helpers.window.window_theme_code
export native fn window_transparent(read win: Window) -> Bool = helpers.window.window_transparent
export native fn window_theme_override_code(read win: Window) -> Int = helpers.window.window_theme_override_code
export native fn window_cursor_icon_code(read win: Window) -> Int = helpers.window.window_cursor_icon_code
export native fn window_cursor_grab_mode(read win: Window) -> Int = helpers.window.window_cursor_grab_mode

export fn window_cursor_position(read win: Window) -> (Int, Int):
    return pair :: (window_cursor_x :: win :: call), (window_cursor_y :: win :: call) :: call

export native fn window_text_input_enabled(read win: Window) -> Bool = helpers.window.window_text_input_enabled
export native fn window_current_monitor_index(read win: Window) -> Int = helpers.window.window_current_monitor_index
export native fn window_primary_monitor_index() -> Int = helpers.window.window_primary_monitor_index
export native fn window_monitor_count() -> Int = helpers.window.window_monitor_count
export native fn window_monitor_name(index: Int) -> Str = helpers.window.window_monitor_name

export fn window_monitor_position(index: Int) -> (Int, Int):
    return pair :: (window_monitor_x :: index :: call), (window_monitor_y :: index :: call) :: call

export fn window_monitor_size(index: Int) -> (Int, Int):
    return pair :: (window_monitor_width :: index :: call), (window_monitor_height :: index :: call) :: call

export native fn window_monitor_scale_factor_milli(index: Int) -> Int = helpers.window.window_monitor_scale_factor_milli
export native fn window_monitor_is_primary(index: Int) -> Bool = helpers.window.window_monitor_is_primary
export native fn window_set_title(edit win: Window, title: Str) = helpers.window.window_set_title
export native fn window_set_position(edit win: Window, x: Int, y: Int) = helpers.window.window_set_position
export native fn window_set_size(edit win: Window, width: Int, height: Int) = helpers.window.window_set_size
export native fn window_set_visible(edit win: Window, enabled: Bool) = helpers.window.window_set_visible
export native fn window_set_decorated(edit win: Window, enabled: Bool) = helpers.window.window_set_decorated
export native fn window_set_resizable(edit win: Window, enabled: Bool) = helpers.window.window_set_resizable
export native fn window_set_min_size(edit win: Window, width: Int, height: Int) = helpers.window.window_set_min_size
export native fn window_set_max_size(edit win: Window, width: Int, height: Int) = helpers.window.window_set_max_size
export native fn window_set_fullscreen(edit win: Window, enabled: Bool) = helpers.window.window_set_fullscreen
export native fn window_set_minimized(edit win: Window, enabled: Bool) = helpers.window.window_set_minimized
export native fn window_set_maximized(edit win: Window, enabled: Bool) = helpers.window.window_set_maximized
export native fn window_set_topmost(edit win: Window, enabled: Bool) = helpers.window.window_set_topmost
export native fn window_set_cursor_visible(edit win: Window, enabled: Bool) = helpers.window.window_set_cursor_visible
export native fn window_set_transparent(edit win: Window, enabled: Bool) = helpers.window.window_set_transparent
export native fn window_set_theme_override_code(edit win: Window, code: Int) = helpers.window.window_set_theme_override_code
export native fn window_set_cursor_icon_code(edit win: Window, code: Int) = helpers.window.window_set_cursor_icon_code
export native fn window_set_cursor_grab_mode(edit win: Window, mode: Int) = helpers.window.window_set_cursor_grab_mode
export native fn window_set_cursor_position(edit win: Window, x: Int, y: Int) = helpers.window.window_set_cursor_position
export native fn window_set_text_input_enabled(edit win: Window, enabled: Bool) = helpers.window.window_set_text_input_enabled
export native fn window_request_redraw(edit win: Window) = helpers.window.window_request_redraw
export native fn window_request_attention(edit win: Window, enabled: Bool) = helpers.window.window_request_attention

export fn window_close(take win: Window) -> Result[Unit, Str]:
    let ok = window_close_raw :: win :: call
    if ok:
        return Result.Ok[Unit, Str] :: :: call
    return Result.Err[Unit, Str] :: (take_last_error :: :: call) :: call

