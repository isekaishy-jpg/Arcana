import std.canvas
import std.kernel.gfx
import std.result
use std.canvas.Image
use std.result.Result

export fn fill(edit win: arcana_desktop.types.Window, color: Int):
    std.kernel.gfx.canvas_fill :: win, color :: call

export fn rect(edit win: arcana_desktop.types.Window, read geometry: ((Int, Int), (Int, Int)), color: Int):
    std.kernel.gfx.canvas_rect :: win :: call
        x = geometry.0.0
        y = geometry.0.1
        w = geometry.1.0
        h = geometry.1.1
        color = color

export fn line(edit win: arcana_desktop.types.Window, read path: ((Int, Int), (Int, Int)), color: Int):
    std.kernel.gfx.canvas_line :: win :: call
        x1 = path.0.0
        y1 = path.0.1
        x2 = path.1.0
        y2 = path.1.1
        color = color

export fn circle_fill(edit win: arcana_desktop.types.Window, read circle: ((Int, Int), Int), color: Int):
    std.kernel.gfx.canvas_circle_fill :: win :: call
        x = circle.0.0
        y = circle.0.1
        radius = circle.1
        color = color

export fn label(edit win: arcana_desktop.types.Window, pos: (Int, Int), read text_and_color: (Str, Int)):
    std.kernel.gfx.canvas_label :: win :: call
        x = pos.0
        y = pos.1
        text = text_and_color.0
        color = text_and_color.1

export fn label_size(text: Str) -> (Int, Int):
    return std.kernel.gfx.canvas_label_size :: text :: call

export fn present(edit win: arcana_desktop.types.Window):
    std.kernel.gfx.canvas_present :: win :: call

export fn rgb(r: Int, g: Int, b: Int) -> Int:
    return std.kernel.gfx.canvas_rgb :: r, g, b :: call

export fn load_image(path: Str) -> Result[Image, Str]:
    return std.canvas.image_load :: path :: call

export fn image_size(read img: Image) -> (Int, Int):
    return std.kernel.gfx.canvas_image_size :: img :: call

export fn blit(edit win: arcana_desktop.types.Window, read img: Image, pos: (Int, Int)):
    std.kernel.gfx.canvas_blit :: win, img, pos.0 :: call
        y = pos.1

export fn blit_scaled(edit win: arcana_desktop.types.Window, read img: Image, read bounds: ((Int, Int), (Int, Int))):
    std.kernel.gfx.canvas_blit_scaled :: win, img, bounds.0.0 :: call
        y = bounds.0.1
        w = bounds.1.0
        h = bounds.1.1
