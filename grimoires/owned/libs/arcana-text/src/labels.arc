import arcana_desktop.canvas
import arcana_text.types

export fn label(read win: arcana_desktop.types.Window, read spec: arcana_text.types.LabelSpec):
    arcana_desktop.canvas.label :: win, spec.pos, (spec.text, spec.color) :: call

export fn measure(text: Str) -> (Int, Int):
    return arcana_desktop.canvas.label_size :: text :: call
