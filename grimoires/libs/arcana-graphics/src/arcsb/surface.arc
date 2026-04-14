import arcana_desktop.types
import arcana_winapi.helpers.graphics
import std.cleanup
import std.collections.list
import std.kernel.memory
import std.result
use std.result.Result

fn err_text() -> Str:
    return arcana_graphics.arcsb.win32.err_text :: :: call

fn ok_unit() -> Result[Unit, Str]:
    return Result.Ok[Unit, Str] :: :: call

fn alpha_supported(read alpha: arcana_graphics.arcsb.AlphaMode) -> Bool:
    let _alpha = alpha
    return true

fn alpha_default() -> arcana_graphics.arcsb.AlphaMode:
    return arcana_graphics.arcsb.AlphaMode.Opaque :: :: call

fn damage_bounds(read rects: List[arcana_graphics.arcsb.Rect]) -> arcana_graphics.arcsb.Rect:
    let mut first = true
    let mut left = 0
    let mut top = 0
    let mut right = 0
    let mut bottom = 0
    for rect in rects:
        if first:
            left = rect.x
            top = rect.y
            right = rect.x + rect.width
            bottom = rect.y + rect.height
            first = false
        else:
            if rect.x < left:
                left = rect.x
            if rect.y < top:
                top = rect.y
            if rect.x + rect.width > right:
                right = rect.x + rect.width
            if rect.y + rect.height > bottom:
                bottom = rect.y + rect.height
    if first:
        let mut empty = arcana_graphics.arcsb.Rect :: x = 0, y = 0, width = 0 :: call
        empty.height = 0
        return empty
    let mut bounds = arcana_graphics.arcsb.Rect :: x = left, y = top, width = right - left :: call
    bounds.height = bottom - top
    return bounds

fn next_buffer_err(message: Str) -> Result[arcana_graphics.arcsb.Buffer, Str]:
    return Result.Err[arcana_graphics.arcsb.Buffer, Str] :: message :: call

fn configure_surface_ok(edit self: arcana_graphics.arcsb.Surface, read size: (Int, Int), read meta: (Int, arcana_graphics.arcsb.AlphaMode)) -> Result[Unit, Str]:
    self.width = size.0
    self.height = size.1
    self.byte_stride = meta.0
    self.alpha_mode = meta.1
    self.configured = true
    return arcana_graphics.arcsb.surface.ok_unit :: :: call

fn next_buffer_ready(read surface_handle: Int, map: Int) -> Result[arcana_graphics.arcsb.Buffer, Str]:
    let len = arcana_winapi.helpers.graphics.software_surface_map_len :: map :: call
    let width = arcana_winapi.helpers.graphics.software_surface_map_width :: map :: call
    let height = arcana_winapi.helpers.graphics.software_surface_map_height :: map :: call
    let byte_stride = arcana_winapi.helpers.graphics.software_surface_map_stride :: map :: call
    let age = arcana_winapi.helpers.graphics.software_surface_map_age :: map :: call
    let pixels = std.kernel.memory.mapped_view_edit :: "arcana_winapi", map, len :: call
    let mut buffer = arcana_graphics.arcsb.Buffer :: surface_handle = surface_handle, map_handle = map, pixels = pixels :: call
    buffer.width = width
    buffer.height = height
    buffer.byte_stride = byte_stride
    buffer.age = age
    return Result.Ok[arcana_graphics.arcsb.Buffer, Str] :: buffer :: call

export fn new_context() -> arcana_graphics.arcsb.Context:
    return arcana_graphics.arcsb.Context :: backend = 1 :: call

export fn new_surface(read cx: arcana_graphics.arcsb.Context, read win: arcana_winapi.desktop_handles.Window) -> Result[arcana_graphics.arcsb.Surface, Str]:
    let _cx = cx
    return match (arcana_graphics.arcsb.win32.open_surface :: win :: call):
        Result.Ok(handle) => new_surface_ready :: handle :: call
        Result.Err(err) => Result.Err[arcana_graphics.arcsb.Surface, Str] :: err :: call

fn new_surface_ready(handle: Int) -> Result[arcana_graphics.arcsb.Surface, Str]:
    let mut surface = arcana_graphics.arcsb.Surface :: handle = handle, configured = false, alpha_mode = (arcana_graphics.arcsb.surface.alpha_default :: :: call) :: call
    surface.width = 0
    surface.height = 0
    surface.byte_stride = 0
    return Result.Ok[arcana_graphics.arcsb.Surface, Str] :: surface :: call

impl std.cleanup.Cleanup[arcana_graphics.arcsb.Surface] for arcana_graphics.arcsb.Surface:
    fn cleanup(take self: arcana_graphics.arcsb.Surface) -> Result[Unit, Str]:
        arcana_graphics.arcsb.win32.destroy_surface :: self.handle :: call
        return arcana_graphics.arcsb.surface.ok_unit :: :: call

impl std.cleanup.Cleanup[arcana_graphics.arcsb.Buffer] for arcana_graphics.arcsb.Buffer:
    fn cleanup(take self: arcana_graphics.arcsb.Buffer) -> Result[Unit, Str]:
        if self.map_handle <= 0:
            return arcana_graphics.arcsb.surface.ok_unit :: :: call
        if arcana_winapi.helpers.graphics.software_surface_discard_map :: self.map_handle :: call:
            return arcana_graphics.arcsb.surface.ok_unit :: :: call
        return Result.Err[Unit, Str] :: (arcana_graphics.arcsb.surface.err_text :: :: call) :: call

impl arcana_graphics.arcsb.Surface:
    fn supports_alpha_mode(read self: arcana_graphics.arcsb.Surface, read alpha: arcana_graphics.arcsb.AlphaMode) -> Bool:
        let _self = self
        return arcana_graphics.arcsb.surface.alpha_supported :: alpha :: call

    fn configure(edit self: arcana_graphics.arcsb.Surface, width: Int, height: Int, read alpha: arcana_graphics.arcsb.AlphaMode) -> Result[Unit, Str]:
        if not (arcana_graphics.arcsb.surface.alpha_supported :: alpha :: call):
            return Result.Err[Unit, Str] :: "unsupported alpha mode" :: call
        let handle = self.handle
        return match (arcana_graphics.arcsb.win32.configure_surface :: handle, width, height :: call):
            Result.Ok(stride) => configure_surface_ok :: self, (width, height), (stride, alpha) :: call
            Result.Err(err) => Result.Err[Unit, Str] :: err :: call

    fn resize(edit self: arcana_graphics.arcsb.Surface, width: Int, height: Int) -> Result[Unit, Str]:
        let alpha = self.alpha_mode
        let handle = self.handle
        return match (arcana_graphics.arcsb.win32.configure_surface :: handle, width, height :: call):
            Result.Ok(stride) => configure_surface_ok :: self, (width, height), (stride, alpha) :: call
            Result.Err(err) => Result.Err[Unit, Str] :: err :: call

    fn next_buffer(edit self: arcana_graphics.arcsb.Surface) -> Result[arcana_graphics.arcsb.Buffer, Str]:
        if not self.configured:
            return arcana_graphics.arcsb.surface.next_buffer_err :: "surface must be configured before next_buffer" :: call
        let map = arcana_winapi.helpers.graphics.software_surface_next_map :: self.handle :: call
        if map <= 0:
            return arcana_graphics.arcsb.surface.next_buffer_err :: (arcana_graphics.arcsb.surface.err_text :: :: call) :: call
        return arcana_graphics.arcsb.surface.next_buffer_ready :: self.handle, map :: call

impl arcana_graphics.arcsb.Buffer:
    fn present(take self: arcana_graphics.arcsb.Buffer, edit surface: arcana_graphics.arcsb.Surface) -> Result[Unit, Str]:
        if self.surface_handle != surface.handle:
            return Result.Err[Unit, Str] :: "buffer does not belong to the provided surface" :: call
        if arcana_winapi.helpers.graphics.software_surface_present :: surface.handle, self.map_handle :: call:
            return arcana_graphics.arcsb.surface.ok_unit :: :: call
        return Result.Err[Unit, Str] :: (arcana_graphics.arcsb.surface.err_text :: :: call) :: call

    fn present_with_damage(take self: arcana_graphics.arcsb.Buffer, edit surface: arcana_graphics.arcsb.Surface, read damage: List[arcana_graphics.arcsb.Rect]) -> Result[Unit, Str]:
        if self.surface_handle != surface.handle:
            return Result.Err[Unit, Str] :: "buffer does not belong to the provided surface" :: call
        if damage :: :: is_empty:
            return self :: surface :: present
        let bounds = arcana_graphics.arcsb.surface.damage_bounds :: damage :: call
        let mut rect = arcana_winapi.raw.types.RECT :: left = bounds.x, top = bounds.y, right = bounds.x + bounds.width :: call
        rect.bottom = bounds.y + bounds.height
        let presented = arcana_winapi.helpers.graphics.software_surface_present_bounded :: surface.handle, self.map_handle, rect :: call
        if presented:
            return arcana_graphics.arcsb.surface.ok_unit :: :: call
        return Result.Err[Unit, Str] :: (arcana_graphics.arcsb.surface.err_text :: :: call) :: call


