import std.kernel.gfx
import std.events
use std.events.AppFrame
use std.events.KeyEvent
use std.events.KeyMeta

export fn key_code(name: Str) -> Int:
    return std.kernel.gfx.input_key_code :: name :: call

export fn key_down(read frame: AppFrame, key: Int) -> Bool:
    return std.kernel.gfx.input_key_down :: frame, key :: call

export fn key_pressed(read frame: AppFrame, key: Int) -> Bool:
    return std.kernel.gfx.input_key_pressed :: frame, key :: call

export fn key_released(read frame: AppFrame, key: Int) -> Bool:
    return std.kernel.gfx.input_key_released :: frame, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.kernel.gfx.input_mouse_button_code :: name :: call

export fn mouse_pos(read frame: AppFrame) -> (Int, Int):
    return std.kernel.gfx.input_mouse_pos :: frame :: call

export fn mouse_down(read frame: AppFrame, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_down :: frame, button :: call

export fn mouse_pressed(read frame: AppFrame, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_pressed :: frame, button :: call

export fn mouse_released(read frame: AppFrame, button: Int) -> Bool:
    return std.kernel.gfx.input_mouse_released :: frame, button :: call

export fn mouse_wheel_y(read frame: AppFrame) -> Int:
    return std.kernel.gfx.input_mouse_wheel_y :: frame :: call

export fn mouse_in_window(read frame: AppFrame) -> Bool:
    return std.kernel.gfx.input_mouse_in_window :: frame :: call

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

export fn key_logical(read event: KeyEvent) -> Int:
    return event.meta.logical_key

export fn key_physical(read event: KeyEvent) -> Int:
    return event.meta.physical_key

export fn key_location(read event: KeyEvent) -> Int:
    return event.meta.location

export fn key_text(read event: KeyEvent) -> Str:
    return event.meta.text

export fn key_repeated(read event: KeyEvent) -> Bool:
    return event.meta.repeated

export fn meta_logical(read meta: KeyMeta) -> Int:
    return meta.logical_key

export fn meta_physical(read meta: KeyMeta) -> Int:
    return meta.physical_key

export fn meta_location(read meta: KeyMeta) -> Int:
    return meta.location

export fn meta_text(read meta: KeyMeta) -> Str:
    return meta.text

export fn meta_repeated(read meta: KeyMeta) -> Bool:
    return meta.repeated
