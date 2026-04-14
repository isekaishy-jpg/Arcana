reexport arcana_graphics.arcsb.surface

export record Context:
    backend: Int

export enum AlphaMode:
    Opaque
    Ignored

export record Rect:
    x: Int
    y: Int
    width: Int
    height: Int

export record Surface:
    handle: Int
    width: Int
    height: Int
    byte_stride: Int
    configured: Bool
    alpha_mode: arcana_graphics.arcsb.AlphaMode

export record Buffer:
    surface_handle: Int
    map_handle: Int
    width: Int
    height: Int
    byte_stride: Int
    age: Int
    pixels: View[U8, Mapped]

export fn new_context() -> arcana_graphics.arcsb.Context:
    return arcana_graphics.arcsb.surface.new_context :: :: call

export fn new_surface(read cx: arcana_graphics.arcsb.Context, read win: arcana_winapi.desktop_handles.Window) -> std.result.Result[arcana_graphics.arcsb.Surface, Str]:
    return arcana_graphics.arcsb.surface.new_surface :: cx, win :: call

