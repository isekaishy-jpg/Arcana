import std.canvas
import std.result
import arcana_graphics.types
use std.result.Result

export fn load(path: Str) -> Result[Image, Str]:
    return std.canvas.image_load :: path :: call

export fn size(read img: Image) -> (Int, Int):
    return std.canvas.image_size :: img :: call

export fn blit(edit win: Window, read img: Image, read spec: arcana_graphics.types.SpriteSpec):
    std.canvas.blit :: win, img, spec.pos.0 :: call
        y = spec.pos.1

export fn blit_scaled(edit win: Window, read img: Image, read spec: arcana_graphics.types.SpriteScaledSpec):
    std.canvas.blit_scaled :: win, img, spec.pos.0 :: call
        y = spec.pos.1
        w = spec.size.0
        h = spec.size.1
