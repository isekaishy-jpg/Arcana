import std.canvas
import std.types.core

export fn rgb(r: Int, g: Int, b: Int) -> Int:
    return std.canvas.rgb :: r, g, b :: call

export fn from_core(read color: std.types.core.ColorRgb) -> Int:
    return std.canvas.rgb :: color.r, color.g, color.b :: call
