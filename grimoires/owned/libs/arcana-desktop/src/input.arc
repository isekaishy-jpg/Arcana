import arcana_desktop.types
import std.kernel.gfx

export fn key_code(name: Str) -> Int:
    return std.kernel.gfx.input_key_code :: name :: call

export fn key_down(read frame: arcana_desktop.types.FrameInput, key: Int) -> Bool:
    return std.kernel.gfx.input_key_down :: frame, key :: call

export fn key_pressed(read frame: arcana_desktop.types.FrameInput, key: Int) -> Bool:
    return std.kernel.gfx.input_key_pressed :: frame, key :: call

export fn key_released(read frame: arcana_desktop.types.FrameInput, key: Int) -> Bool:
    return std.kernel.gfx.input_key_released :: frame, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.kernel.gfx.input_mouse_button_code :: name :: call

export fn mouse_pos(read frame: arcana_desktop.types.FrameInput) -> (Int, Int):
    return std.kernel.gfx.input_mouse_pos :: frame :: call

export fn mouse_down(read frame: arcana_desktop.types.FrameInput, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_down :: frame, button :: call

export fn mouse_pressed(read frame: arcana_desktop.types.FrameInput, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_pressed :: frame, button :: call

export fn mouse_released(read frame: arcana_desktop.types.FrameInput, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_released :: frame, button :: call

export fn mouse_in_window(read frame: arcana_desktop.types.FrameInput) -> Bool:
    return std.kernel.gfx.input_mouse_in_window :: frame :: call

export fn mouse_wheel_y(read frame: arcana_desktop.types.FrameInput) -> Int:
    return std.kernel.gfx.input_mouse_wheel_y :: frame :: call

export fn modifier_shift(flags: Int) -> Bool:
    return (flags & 1) != 0

export fn modifier_ctrl(flags: Int) -> Bool:
    return (flags & 2) != 0

export fn modifier_alt(flags: Int) -> Bool:
    return (flags & 4) != 0

export fn modifier_meta(flags: Int) -> Bool:
    return (flags & 8) != 0

export fn key_location_standard() -> Int:
    return 0

export fn key_location_left() -> Int:
    return 1

export fn key_location_right() -> Int:
    return 2

export fn key_location_numpad() -> Int:
    return 3

export fn key_logical(read event: arcana_desktop.types.KeyEvent) -> Int:
    return event.meta.logical_key

export fn key_physical(read event: arcana_desktop.types.KeyEvent) -> Int:
    return event.meta.physical_key

export fn key_location(read event: arcana_desktop.types.KeyEvent) -> Int:
    return event.meta.location

export fn key_text(read event: arcana_desktop.types.KeyEvent) -> Str:
    return event.meta.text

export fn key_repeated(read event: arcana_desktop.types.KeyEvent) -> Bool:
    return event.meta.repeated

export fn meta_logical(read meta: arcana_desktop.types.KeyMeta) -> Int:
    return meta.logical_key

export fn meta_physical(read meta: arcana_desktop.types.KeyMeta) -> Int:
    return meta.physical_key

export fn meta_location(read meta: arcana_desktop.types.KeyMeta) -> Int:
    return meta.location

export fn meta_text(read meta: arcana_desktop.types.KeyMeta) -> Str:
    return meta.text

export fn meta_repeated(read meta: arcana_desktop.types.KeyMeta) -> Bool:
    return meta.repeated

export fn snapshot(read frame: arcana_desktop.types.FrameInput) -> arcana_desktop.types.InputSnapshot:
    return arcana_desktop.types.InputSnapshot :: mouse_pos = (arcana_desktop.input.mouse_pos :: frame :: call), mouse_in_window = (arcana_desktop.input.mouse_in_window :: frame :: call), mouse_wheel_y = (arcana_desktop.input.mouse_wheel_y :: frame :: call) :: call
