import std.kernel.gfx

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
