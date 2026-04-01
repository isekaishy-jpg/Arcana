import arcana_desktop.canvas
import arcana_graphics.types

export fn fill(edit win: arcana_desktop.types.Window, color: Int):
    arcana_desktop.canvas.fill :: win, color :: call

export fn rgb(r: Int, g: Int, b: Int) -> Int:
    return arcana_desktop.canvas.rgb :: r, g, b :: call

export fn rect(edit win: arcana_desktop.types.Window, read spec: arcana_graphics.types.RectSpec):
    arcana_desktop.canvas.rect :: win, (spec.pos, spec.size), spec.color :: call

export fn line(edit win: arcana_desktop.types.Window, read spec: arcana_graphics.types.LineSpec):
    arcana_desktop.canvas.line :: win, (spec.start, spec.end), spec.color :: call

export fn circle_fill(edit win: arcana_desktop.types.Window, read spec: arcana_graphics.types.CircleFillSpec):
    arcana_desktop.canvas.circle_fill :: win, (spec.center, spec.radius), spec.color :: call

export fn present(edit win: arcana_desktop.types.Window):
    arcana_desktop.canvas.present :: win :: call
