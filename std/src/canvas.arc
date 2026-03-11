import std.kernel.gfx
import std.result
import std.window
use std.result.Result
use std.window.Window

export opaque type Image as move, boundary_unsafe

export record RectSpec:
    pos: (Int, Int)
    size: (Int, Int)
    color: Int

export record LineSpec:
    start: (Int, Int)
    end: (Int, Int)
    color: Int

export record CircleFillSpec:
    center: (Int, Int)
    radius: Int
    color: Int

export record LabelSpec:
    pos: (Int, Int)
    text: Str
    color: Int

export fn open(title: Str, width: Int, height: Int) -> Result[Window, Str]:
    return std.window.open :: title, width, height :: call

export fn alive(read win: Window) -> Bool:
    return std.window.alive :: win :: call

export fn fill(edit win: Window, color: Int):
    std.kernel.gfx.canvas_fill :: win, color :: call

export fn rect(edit win: Window, x: Int, y: Int, w: Int, h: Int, color: Int):
    std.kernel.gfx.canvas_rect :: win :: call
        x = x
        y = y
        w = w
        h = h
        color = color

export fn rect_draw(edit win: Window, read spec: RectSpec):
    std.kernel.gfx.canvas_rect :: win :: call
        x = spec.pos.0
        y = spec.pos.1
        w = spec.size.0
        h = spec.size.1
        color = spec.color

export fn line(edit win: Window, x1: Int, y1: Int, x2: Int, y2: Int, color: Int):
    std.kernel.gfx.canvas_line :: win :: call
        x1 = x1
        y1 = y1
        x2 = x2
        y2 = y2
        color = color

export fn line_draw(edit win: Window, read spec: LineSpec):
    std.kernel.gfx.canvas_line :: win :: call
        x1 = spec.start.0
        y1 = spec.start.1
        x2 = spec.end.0
        y2 = spec.end.1
        color = spec.color

export fn circle_fill(edit win: Window, x: Int, y: Int, radius: Int, color: Int):
    std.kernel.gfx.canvas_circle_fill :: win :: call
        x = x
        y = y
        radius = radius
        color = color

export fn circle_fill_draw(edit win: Window, read spec: CircleFillSpec):
    std.kernel.gfx.canvas_circle_fill :: win :: call
        x = spec.center.0
        y = spec.center.1
        radius = spec.radius
        color = spec.color

export fn label(edit win: Window, x: Int, y: Int, text: Str, color: Int):
    std.kernel.gfx.canvas_label :: win :: call
        x = x
        y = y
        text = text
        color = color

export fn label_draw(edit win: Window, read spec: LabelSpec):
    std.kernel.gfx.canvas_label :: win :: call
        x = spec.pos.0
        y = spec.pos.1
        text = spec.text
        color = spec.color

export fn label_size(text: Str) -> (Int, Int):
    return std.kernel.gfx.canvas_label_size :: text :: call

export fn present(edit win: Window):
    std.kernel.gfx.canvas_present :: win :: call

export fn rgb(r: Int, g: Int, b: Int) -> Int:
    return std.kernel.gfx.canvas_rgb :: r, g, b :: call

export fn image_load(path: Str) -> Result[Image, Str]:
    return std.kernel.gfx.image_load :: path :: call

export fn image_size(read img: Image) -> (Int, Int):
    return std.kernel.gfx.canvas_image_size :: img :: call

export fn blit(edit win: Window, read img: Image, x: Int, y: Int):
    std.kernel.gfx.canvas_blit :: win, img, x :: call
        y = y

export fn blit_scaled(edit win: Window, read img: Image, x: Int, y: Int, w: Int, h: Int):
    std.kernel.gfx.canvas_blit_scaled :: win, img, x :: call
        y = y
        w = w
        h = h

export fn blit_region(edit win: Window, read img: Image, sx: Int, sy: Int, sw: Int, sh: Int, dx: Int, dy: Int, dw: Int, dh: Int):
    std.kernel.gfx.canvas_blit_region :: win, img, sx :: call
        sy = sy
        sw = sw
        sh = sh
        dx = dx
        dy = dy
        dw = dw
        dh = dh
