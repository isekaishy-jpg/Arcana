import std.canvas
import arcana_graphics.types
use std.window.Window

export fn fill(edit win: Window, color: Int):
    std.canvas.fill :: win, color :: call

export fn rgb(r: Int, g: Int, b: Int) -> Int:
    return std.canvas.rgb :: r, g, b :: call

export fn rect(edit win: Window, read spec: arcana_graphics.types.RectSpec):
    let cmd = std.canvas.RectSpec :: pos = spec.pos, size = spec.size, color = spec.color :: call
    std.canvas.rect_draw :: win, cmd :: call

export fn line(edit win: Window, read spec: arcana_graphics.types.LineSpec):
    let cmd = std.canvas.LineSpec :: start = spec.start, end = spec.end, color = spec.color :: call
    std.canvas.line_draw :: win, cmd :: call

export fn circle_fill(edit win: Window, read spec: arcana_graphics.types.CircleFillSpec):
    let cmd = std.canvas.CircleFillSpec :: center = spec.center, radius = spec.radius, color = spec.color :: call
    std.canvas.circle_fill_draw :: win, cmd :: call

export fn present(edit win: Window):
    std.canvas.present :: win :: call
