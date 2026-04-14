import arcana_desktop.types
import arcana_desktop.window
import arcana_winapi.helpers.graphics
import std.result
use std.result.Result

export fn err_text() -> Str:
    return arcana_winapi.helpers.graphics.software_surface_take_last_error :: :: call

export fn open_surface(read win: arcana_winapi.desktop_handles.Window) -> Result[Int, Str]:
    let hwnd = arcana_desktop.window.native_handle :: win :: call
    let handle = arcana_winapi.helpers.graphics.software_surface_open :: hwnd :: call
    if handle <= 0:
        return Result.Err[Int, Str] :: (arcana_graphics.arcsb.win32.err_text :: :: call) :: call
    return Result.Ok[Int, Str] :: handle :: call

export fn configure_surface(read handle: Int, width: Int, height: Int) -> Result[Int, Str]:
    if arcana_winapi.helpers.graphics.software_surface_configure :: handle, width, height :: call:
        let stride = arcana_winapi.helpers.graphics.gdi_memory_surface_stride :: width, height :: call
        return Result.Ok[Int, Str] :: stride :: call
    return Result.Err[Int, Str] :: (arcana_graphics.arcsb.win32.err_text :: :: call) :: call

export fn destroy_surface(read handle: Int):
    arcana_winapi.helpers.graphics.software_surface_destroy :: handle :: call

