import arcana_desktop.canvas
import arcana_graphics.types
import std.result
use std.result.Result
use std.canvas.Image

export fn load(path: Str) -> Result[Image, Str]:
    return arcana_desktop.canvas.load_image :: path :: call

export fn size(read img: Image) -> (Int, Int):
    return arcana_desktop.canvas.image_size :: img :: call

export fn blit(edit win: arcana_desktop.types.Window, read img: Image, read spec: arcana_graphics.types.SpriteSpec):
    arcana_desktop.canvas.blit :: win, img, spec.pos :: call

export fn blit_scaled(edit win: arcana_desktop.types.Window, read img: Image, read spec: arcana_graphics.types.SpriteScaledSpec):
    arcana_desktop.canvas.blit_scaled :: win, img, (spec.pos, spec.size) :: call
