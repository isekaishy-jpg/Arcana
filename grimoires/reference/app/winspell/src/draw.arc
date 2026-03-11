import std.canvas
import std.result
use std.result.Result
use std.canvas.Image
use std.window.Window

export record RectSpec:
    pos: (Int, Int)
    size: (Int, Int)
    color: Int

export record LabelSpec:
    pos: (Int, Int)
    text: Str
    color: Int

export fn rgb(r: Int, g: Int, b: Int) -> Int:
    return std.canvas.rgb :: r, g, b :: call

export fn fill(edit win: Window, color: Int):
    std.canvas.fill :: win, color :: call

export fn rect(edit win: Window, read spec: RectSpec):
    let cmd = std.canvas.RectSpec :: pos = spec.pos, size = spec.size, color = spec.color :: call
    std.canvas.rect_draw :: win, cmd :: call

export fn label(edit win: Window, read spec: LabelSpec):
    let cmd = std.canvas.LabelSpec :: pos = spec.pos, text = spec.text, color = spec.color :: call
    std.canvas.label_draw :: win, cmd :: call

export fn present(edit win: Window):
    std.canvas.present :: win :: call

export fn image_load(path: Str) -> Result[Image, Str]:
    return std.canvas.image_load :: path :: call

export fn image_size(read img: Image) -> (Int, Int):
    return std.canvas.image_size :: img :: call
