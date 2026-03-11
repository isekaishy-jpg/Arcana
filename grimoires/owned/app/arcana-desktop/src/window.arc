import std.window
import std.result
import arcana_desktop.types
use std.result.Result
use std.window.Window

export fn open(title: Str, width: Int, height: Int) -> Result[Window, Str]:
    return std.window.open :: title, width, height :: call

export fn open_cfg(read cfg: arcana_desktop.types.WindowConfig) -> Result[Window, Str]:
    return std.window.open :: cfg.title, cfg.size.0, cfg.size.1 :: call

export fn alive(read win: Window) -> Bool:
    return std.window.alive :: win :: call

export fn close(take win: Window) -> Result[Unit, Str]:
    return std.window.close :: win :: call

export fn size(read win: Window) -> (Int, Int):
    return std.window.size :: win :: call

export fn focused(read win: Window) -> Bool:
    return std.window.focused :: win :: call

export fn resized(read win: Window) -> Bool:
    return std.window.resized :: win :: call

export fn fullscreen(read win: Window) -> Bool:
    return std.window.fullscreen :: win :: call

export fn minimized(read win: Window) -> Bool:
    return std.window.minimized :: win :: call

export fn maximized(read win: Window) -> Bool:
    return std.window.maximized :: win :: call

export fn set_title(edit win: Window, title: Str):
    std.window.set_title :: win, title :: call

export fn set_resizable(edit win: Window, enabled: Bool):
    std.window.set_resizable :: win, enabled :: call

export fn set_fullscreen(edit win: Window, enabled: Bool):
    std.window.set_fullscreen :: win, enabled :: call

export fn set_minimized(edit win: Window, enabled: Bool):
    std.window.set_minimized :: win, enabled :: call

export fn set_maximized(edit win: Window, enabled: Bool):
    std.window.set_maximized :: win, enabled :: call

export fn set_topmost(edit win: Window, enabled: Bool):
    std.window.set_topmost :: win, enabled :: call

export fn set_cursor_visible(edit win: Window, enabled: Bool):
    std.window.set_cursor_visible :: win, enabled :: call
