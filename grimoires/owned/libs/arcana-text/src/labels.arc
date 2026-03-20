import std.canvas
import arcana_text.types
use std.window.Window

export fn label(edit win: Window, read spec: arcana_text.types.LabelSpec):
    std.canvas.label :: win :: call
        x = spec.pos.0
        y = spec.pos.1
        text = spec.text
        color = spec.color

export fn measure(text: Str) -> (Int, Int):
    return std.canvas.label_size :: text :: call
