export record Vec2i:
    x: Int
    y: Int

export record Size2i:
    w: Int
    h: Int

export record Recti:
    pos: std.types.core.Vec2i
    size: std.types.core.Size2i

export record ColorRgb:
    r: Int
    g: Int
    b: Int

export record Tick:
    value: Int

export record FrameIndex:
    value: Int

export record DurationMs:
    value: Int

export record MonotonicTimeMs:
    value: Int

export fn vec2(x: Int, y: Int) -> std.types.core.Vec2i:
    return std.types.core.Vec2i :: x = x, y = y :: call

export fn size2(w: Int, h: Int) -> std.types.core.Size2i:
    return std.types.core.Size2i :: w = w, h = h :: call

export fn rect(pos: std.types.core.Vec2i, size: std.types.core.Size2i) -> std.types.core.Recti:
    return std.types.core.Recti :: pos = pos, size = size :: call

export fn rgb(r: Int, g: Int, b: Int) -> std.types.core.ColorRgb:
    return std.types.core.ColorRgb :: r = r, g = g, b = b :: call

export fn duration_ms(value: Int) -> std.types.core.DurationMs:
    return std.types.core.DurationMs :: value = value :: call

export fn monotonic_time_ms(value: Int) -> std.types.core.MonotonicTimeMs:
    return std.types.core.MonotonicTimeMs :: value = value :: call
