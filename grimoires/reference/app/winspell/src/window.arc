import std.canvas
import std.window
import std.result
use std.result.Result
use std.window.Window

export fn open(title: Str, width: Int, height: Int) -> Result[Window, Str]:
    return std.canvas.open :: title, width, height :: call

export fn alive(read win: Window) -> Bool:
    return std.canvas.alive :: win :: call

export fn close(edit win: Window):
    std.window.close :: win :: call

export fn size(read win: Window) -> (Int, Int):
    return std.window.size :: win :: call

export fn focused(read win: Window) -> Bool:
    return std.window.focused :: win :: call

export fn resized(read win: Window) -> Bool:
    return std.window.resized :: win :: call

export fn set_title(edit win: Window, title: Str):
    std.window.set_title :: win, title :: call

export fn set_cursor_visible(edit win: Window, enabled: Bool):
    std.window.set_cursor_visible :: win, enabled :: call

export fn set_topmost(edit win: Window, enabled: Bool):
    std.window.set_topmost :: win, enabled :: call
