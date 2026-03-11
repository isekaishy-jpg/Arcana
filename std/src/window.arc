import std.kernel.gfx
import std.kernel.error
import std.result
use std.result.Result

export fn open(title: Str, width: Int, height: Int) -> Result[Window, Str]:
    let pair = std.kernel.gfx.window_open_try :: title, width, height :: call
    if pair.0:
        return Result.Ok[Window, Str] :: pair.1 :: call
    return Result.Err[Window, Str] :: (std.kernel.error.last_error_take :: :: call) :: call

export fn alive(read win: Window) -> Bool:
    return std.kernel.gfx.canvas_alive :: win :: call

export fn size(read win: Window) -> (Int, Int):
    return std.kernel.gfx.window_size :: win :: call

export fn resized(read win: Window) -> Bool:
    return std.kernel.gfx.window_resized :: win :: call

export fn fullscreen(read win: Window) -> Bool:
    return std.kernel.gfx.window_fullscreen :: win :: call

export fn minimized(read win: Window) -> Bool:
    return std.kernel.gfx.window_minimized :: win :: call

export fn maximized(read win: Window) -> Bool:
    return std.kernel.gfx.window_maximized :: win :: call

export fn focused(read win: Window) -> Bool:
    return std.kernel.gfx.window_focused :: win :: call

export fn set_title(edit win: Window, title: Str):
    std.kernel.gfx.window_set_title :: win, title :: call

export fn set_resizable(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_resizable :: win, enabled :: call

export fn set_fullscreen(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_fullscreen :: win, enabled :: call

export fn set_minimized(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_minimized :: win, enabled :: call

export fn set_maximized(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_maximized :: win, enabled :: call

export fn set_topmost(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_topmost :: win, enabled :: call

export fn set_cursor_visible(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_cursor_visible :: win, enabled :: call

export fn close(edit win: Window):
    std.kernel.gfx.window_close :: win :: call
