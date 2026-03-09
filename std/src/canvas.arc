import std.kernel.gfx

export record RectSpec:
    pos: (Int, Int)
    size: (Int, Int)
    color: Int

export record LabelSpec:
    pos: (Int, Int)
    text: Str
    color: Int

export fn open(title: Str, width: Int, height: Int) -> Window:
    return std.kernel.gfx.canvas_open :: title, width, height :: call

export fn alive(read win: Window) -> Bool:
    return std.kernel.gfx.canvas_alive :: win :: call

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

export fn present(edit win: Window):
    std.kernel.gfx.canvas_present :: win :: call

export fn rgb(r: Int, g: Int, b: Int) -> Int:
    return std.kernel.gfx.canvas_rgb :: r, g, b :: call

export fn image_load(path: Str) -> Image:
    return std.kernel.gfx.canvas_image_load :: path :: call

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
