import arcana_desktop.app
import arcana_desktop.types
import arcana_desktop.window
import arcana_graphics.canvas
import arcana_graphics.paint
import arcana_text.builder
import arcana_text.fonts
import arcana_text.paragraphs
import std.args

record Demo:
    drawn: Bool
    smoke_mode: Bool

fn has_flag(flag: Str) -> Bool:
    let total = std.args.count :: :: call
    let mut index = 0
    while index < total:
        if (std.args.get :: index :: call) == flag:
            return true
        index += 1
    return false

fn demo_placeholder() -> arcana_text.types.PlaceholderStyle:
    let mut placeholder = arcana_text.types.PlaceholderStyle :: size = (28, 14), alignment = (arcana_text.types.PlaceholderAlignment.Middle :: :: call), baseline = (arcana_text.types.TextBaseline.Alphabetic :: :: call) :: call
    placeholder.baseline_offset = 0
    return placeholder

fn demo_paragraph(text: Str, width: Int, color: Int) -> arcana_text.types.Paragraph:
    let collection = arcana_text.fonts.default_collection :: :: call
    let mut paragraph_style = arcana_text.types.default_paragraph_style :: :: call
    paragraph_style.max_lines = 2
    paragraph_style.ellipsis = "..."
    let mut style = arcana_text.types.default_text_style :: color :: call
    style.background_enabled = true
    style.background = arcana_graphics.paint.solid :: (arcana_graphics.canvas.rgb :: 16, 28, 42 :: call) :: call
    let mut builder = arcana_text.builder.open :: collection, paragraph_style :: call
    arcana_text.builder.push_style :: builder, style :: call
    arcana_text.builder.add_text :: builder, text :: call
    arcana_text.builder.add_placeholder :: builder, (demo_placeholder :: :: call) :: call
    let mut paragraph = arcana_text.builder.build :: builder :: call
    arcana_text.paragraphs.layout :: paragraph, width :: call
    return paragraph

impl arcana_desktop.app.Application[Demo] for Demo:
    fn resumed(edit self: Demo, edit cx: arcana_desktop.types.AppContext):
        let mut main_window = (arcana_desktop.app.main_window_or_cached :: cx :: call)
        arcana_desktop.window.set_title :: main_window, "Arcana Text Proof" :: call
        arcana_desktop.app.request_window_redraw :: cx, main_window :: call
        arcana_desktop.app.set_control_flow :: cx, (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call

    fn suspended(edit self: Demo, edit cx: arcana_desktop.types.AppContext):
        return

    fn window_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
        return match target.event:
            arcana_desktop.types.WindowEvent.WindowRedrawRequested(id) => on_redraw :: self, cx, id :: call
            _ => cx.control.control_flow

    fn device_event(edit self: Demo, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:
        return cx.control.control_flow

    fn about_to_wait(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        return cx.control.control_flow

    fn wake(edit self: Demo, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
        return cx.control.control_flow

    fn exiting(edit self: Demo, edit cx: arcana_desktop.types.AppContext):
        return

fn on_redraw(edit self: Demo, edit cx: arcana_desktop.types.AppContext, id: Int) -> arcana_desktop.types.ControlFlow:
    let _ = id
    let mut main_window = (arcana_desktop.app.main_window_or_cached :: cx :: call)
    arcana_graphics.canvas.fill :: main_window, (arcana_graphics.canvas.rgb :: 8, 14, 21 :: call) :: call

    let mut paragraph = demo_paragraph :: "Styled paragraph proof with placeholder and ellipsis support.", 260, (arcana_graphics.canvas.rgb :: 226, 232, 240 :: call) :: call
    arcana_text.paragraphs.paint :: main_window, paragraph, (24, 26) :: call

    arcana_text.paragraphs.update_text :: paragraph, "Mutable update path centered after first layout." :: call
    arcana_text.paragraphs.update_align :: paragraph, (arcana_text.types.TextAlign.Center :: :: call) :: call
    arcana_text.paragraphs.update_font_size :: paragraph, 24 :: call
    arcana_text.paragraphs.update_foreground :: paragraph, (arcana_graphics.paint.solid :: (arcana_graphics.canvas.rgb :: 113, 214, 225 :: call) :: call) :: call
    arcana_text.paragraphs.update_background :: paragraph, true, (arcana_graphics.paint.solid :: (arcana_graphics.canvas.rgb :: 18, 35, 49 :: call) :: call) :: call
    arcana_text.paragraphs.paint :: main_window, paragraph, (24, 96) :: call

    let boxes = arcana_text.paragraphs.range_boxes :: paragraph, (arcana_text.types.TextRange :: start = 0, end = 5 :: call) :: call
    if (boxes :: :: len) > 0:
        let first = boxes[0]
        let color = arcana_graphics.canvas.rgb :: 36, 77, 90 :: call
        let spec = arcana_graphics.types.RectSpec :: pos = first.position, size = first.size, color = color :: call
        arcana_graphics.canvas.rect :: main_window, spec :: call

    arcana_graphics.canvas.present :: main_window :: call
    self.drawn = true
    if self.smoke_mode:
        arcana_desktop.app.request_exit :: cx, 0 :: call
    return arcana_desktop.types.ControlFlow.Wait :: :: call

fn main() -> Int:
    let mut app = Demo :: drawn = false, smoke_mode = (has_flag :: "--smoke" :: call) :: call
    return arcana_desktop.app.run :: app, (arcana_desktop.app.default_app_config :: :: call) :: call
