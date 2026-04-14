import arcana_desktop.app
import arcana_desktop.types
import arcana_desktop.window
import arcana_graphics.arcsb
import arcana_winapi.desktop_handles
import std.collections.list
import arcana_process.io
import std.result
import std.text
use std.result.Result

record ProofApp:
    redraw_count: Int
    printed: Bool

fn damage_axis(value: Int) -> Int:
    let half = value / 2
    if half <= 0:
        return 1
    return half

fn resize_axis(value: Int) -> Int:
    if value > 80:
        return value - 16
    return value

fn put_pixel(edit pixels: View[U8, Mapped], base: Int, read color: (Int, (Int, Int))):
    let rest = color.1
    pixels :: base, color.0 :: set
    pixels :: base + 1, rest.0 :: set
    pixels :: base + 2, rest.1 :: set
    pixels :: base + 3, 255 :: set

fn paint_frame(edit pixels: View[U8, Mapped], read layout: (Int, (Int, Int)), phase: Int):
    let dims = layout.1
    let width = layout.0
    let height = dims.0
    let byte_stride = dims.1
    let mut y = 0
    while y < height:
        let mut x = 0
        while x < width:
            let base = y * byte_stride + x * 4
            let blue = (x * 3 + phase * 17) % 256
            let green = (y * 5 + phase * 11) % 256
            let red = ((x + y) * 2 + phase * 7) % 256
            put_pixel :: pixels, base, (blue, (green, red)) :: call
            x += 1
        y += 1

fn damage_list(width: Int, height: Int) -> List[arcana_graphics.arcsb.Rect]:
    let mut rects = std.collections.list.new[arcana_graphics.arcsb.Rect] :: :: call
    let mut rect = arcana_graphics.arcsb.Rect :: x = width / 4, y = height / 4, width = 0 :: call
    rect.width = damage_axis :: width :: call
    rect.height = damage_axis :: height :: call
    rects :: rect :: push
    return rects

fn configure_surface(edit surface: arcana_graphics.arcsb.Surface, width: Int, height: Int) -> Result[Unit, Str]:
    return surface :: width, height, (arcana_graphics.arcsb.AlphaMode.Opaque :: :: call) :: configure

fn surface_open(read cx: arcana_graphics.arcsb.Context, read win: arcana_winapi.desktop_handles.Window) -> Result[Int, Str]:
    let opened = arcana_graphics.arcsb.new_surface :: cx, win :: call
    return match opened:
        Result.Ok(value) => draw_surface :: value, win :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_surface(take value: arcana_graphics.arcsb.Surface, read win: arcana_winapi.desktop_handles.Window) -> Result[Int, Str]:
    let mut surface = value
    let size = arcana_desktop.window.size :: win :: call
    let configured = configure_surface :: surface, size.0, size.1 :: call
    return match configured:
        Result.Ok(_) => draw_buffer1_open :: surface, size :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_buffer1_open(take surface_value: arcana_graphics.arcsb.Surface, read size: (Int, Int)) -> Result[Int, Str]:
    let mut surface = surface_value
    let next = surface :: :: next_buffer
    return match next:
        Result.Ok(value) => draw_buffer1 :: surface, value, size :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_buffer1(take surface_value: arcana_graphics.arcsb.Surface, take buffer_value: arcana_graphics.arcsb.Buffer, read size: (Int, Int)) -> Result[Int, Str]:
    let mut surface = surface_value
    let mut buffer = buffer_value
    paint_frame :: buffer.pixels, (buffer.width, (buffer.height, buffer.byte_stride)), 3 :: call
    let presented = buffer :: surface :: present
    return match presented:
        Result.Ok(_) => draw_buffer2_open :: surface, size :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_buffer2_open(take surface_value: arcana_graphics.arcsb.Surface, read size: (Int, Int)) -> Result[Int, Str]:
    let mut surface = surface_value
    let next = surface :: :: next_buffer
    return match next:
        Result.Ok(value) => draw_buffer2 :: surface, value, size :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_buffer2(take surface_value: arcana_graphics.arcsb.Surface, take buffer_value: arcana_graphics.arcsb.Buffer, read size: (Int, Int)) -> Result[Int, Str]:
    let mut surface = surface_value
    let mut buffer = buffer_value
    if buffer.age != 1:
        return Result.Err[Int, Str] :: ("expected arcsb buffer age 1, got " + (std.text.from_int :: buffer.age :: call)) :: call
    paint_frame :: buffer.pixels, (buffer.width, (buffer.height, buffer.byte_stride)), 19 :: call
    let damage = damage_list :: size.0, size.1 :: call
    let presented = buffer :: surface, damage :: present_with_damage
    return match presented:
        Result.Ok(_) => draw_resized_surface :: surface, size :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_resized_surface(take surface_value: arcana_graphics.arcsb.Surface, read size: (Int, Int)) -> Result[Int, Str]:
    let mut surface = surface_value
    let resized_width = resize_axis :: size.0 :: call
    let resized_height = resize_axis :: size.1 :: call
    let resized = surface :: resized_width, resized_height :: resize
    return match resized:
        Result.Ok(_) => draw_buffer3_open :: surface, (resized_width, resized_height) :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_buffer3_open(take surface_value: arcana_graphics.arcsb.Surface, read size: (Int, Int)) -> Result[Int, Str]:
    let mut surface = surface_value
    let next = surface :: :: next_buffer
    return match next:
        Result.Ok(value) => draw_buffer3 :: surface, value, size :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_buffer3(take surface_value: arcana_graphics.arcsb.Surface, take buffer_value: arcana_graphics.arcsb.Buffer, read size: (Int, Int)) -> Result[Int, Str]:
    let mut surface = surface_value
    let mut buffer = buffer_value
    if buffer.age != 0:
        return Result.Err[Int, Str] :: ("expected arcsb resized buffer age 0, got " + (std.text.from_int :: buffer.age :: call)) :: call
    paint_frame :: buffer.pixels, (buffer.width, (buffer.height, buffer.byte_stride)), 41 :: call
    let presented = buffer :: surface :: present
    return match presented:
        Result.Ok(_) => Result.Ok[Int, Str] :: (size.0 * 31 + size.1 * 17) :: call
        Result.Err(err) => Result.Err[Int, Str] :: err :: call

fn draw_smoke(edit win: arcana_winapi.desktop_handles.Window) -> Result[Int, Str]:
    let cx = arcana_graphics.arcsb.new_context :: :: call
    return surface_open :: cx, win :: call

fn on_redraw(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    let win = arcana_desktop.app.require_current_window :: cx :: call
    return match win:
        Result.Ok(value) => on_redraw_ready :: self, cx, value :: call
        Result.Err(err) => on_redraw_error :: self, cx, err :: call

fn on_redraw_ready(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, take value: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.ControlFlow:
    let mut win = value
    self.redraw_count += 1
    let result = draw_smoke :: win :: call
    return match result:
        Result.Ok(score) => on_redraw_ok :: self, cx, score :: call
        Result.Err(err) => on_redraw_error :: self, cx, err :: call

fn on_redraw_ok(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, score: Int) -> arcana_desktop.types.ControlFlow:
    if not self.printed:
        arcana_process.io.print_line[Str] :: ("arcsb_score=" + (std.text.from_int :: score :: call)) :: call
        arcana_process.io.flush_stdout :: :: call
        self.printed = true
    arcana_desktop.app.request_exit :: cx, 0 :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn on_redraw_error(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, err: Str) -> arcana_desktop.types.ControlFlow:
    if not self.printed:
        arcana_process.io.print_line[Str] :: ("arcsb_error=" + err) :: call
        arcana_process.io.flush_stdout :: :: call
        self.printed = true
    arcana_desktop.app.request_exit :: cx, 91 :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

impl arcana_desktop.app.Application[ProofApp] for ProofApp:
    fn resumed(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext):
        let _ = self
        arcana_desktop.app.request_main_window_redraw :: cx :: call

    fn suspended(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext):
        let _ = self
        let _ = cx
        return

    fn window_event(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
        return match target.event:
            arcana_desktop.types.WindowEvent.WindowRedrawRequested(_) => on_redraw :: self, cx :: call
            arcana_desktop.types.WindowEvent.WindowCloseRequested(_) => on_close_requested :: cx :: call
            _ => cx.control.control_flow

    fn device_event(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:
        let _ = self
        let _ = event
        return cx.control.control_flow

    fn about_to_wait(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        let _ = self
        return cx.control.control_flow

    fn wake(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        let _ = self
        return cx.control.control_flow

    fn exiting(edit self: ProofApp, edit cx: arcana_desktop.types.AppContext):
        let _ = self
        let _ = cx
        return

fn on_close_requested(edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    arcana_desktop.app.request_exit :: cx, 0 :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn main() -> Int:
    let mut app = ProofApp :: redraw_count = 0, printed = false :: call
    let mut cfg = arcana_desktop.app.default_app_config :: :: call
    cfg.window.title = "Arcana arcsb Proof"
    cfg.window.bounds.size = (640, 400)
    cfg.window.bounds.position = (80, 80)
    cfg.window.bounds.min_size = (320, 240)
    return arcana_desktop.app.run :: app, cfg :: call
