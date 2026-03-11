import std.canvas
import arcana_text.types

export fn label(edit win: Window, read spec: arcana_text.types.LabelSpec):
    let cmd = std.canvas.LabelSpec :: pos = spec.pos, text = spec.text, color = spec.color :: call
    std.canvas.label_draw :: win, cmd :: call

export fn measure(text: Str) -> (Int, Int):
    return std.canvas.label_size :: text :: call
