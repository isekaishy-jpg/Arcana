import arcana_desktop.types
import std.input
use std.events.AppFrame
use std.events.KeyEvent
use std.events.KeyMeta

export fn key_code(name: Str) -> Int:
    return std.input.key_code :: name :: call

export fn key_down(read frame: AppFrame, key: Int) -> Bool:
    return std.input.key_down :: frame, key :: call

export fn key_pressed(read frame: AppFrame, key: Int) -> Bool:
    return std.input.key_pressed :: frame, key :: call

export fn key_released(read frame: AppFrame, key: Int) -> Bool:
    return std.input.key_released :: frame, key :: call

export fn mouse_button_code(name: Str) -> Int:
    return std.input.mouse_button_code :: name :: call

export fn mouse_pos(read frame: AppFrame) -> (Int, Int):
    return std.input.mouse_pos :: frame :: call

export fn mouse_down(read frame: AppFrame, button: Int) -> Bool:
    return std.input.mouse_down :: frame, button :: call

export fn mouse_pressed(read frame: AppFrame, button: Int) -> Bool:
    return std.input.mouse_pressed :: frame, button :: call

export fn mouse_released(read frame: AppFrame, button: Int) -> Bool:
    return std.input.mouse_released :: frame, button :: call

export fn mouse_in_window(read frame: AppFrame) -> Bool:
    return std.input.mouse_in_window :: frame :: call

export fn mouse_wheel_y(read frame: AppFrame) -> Int:
    return std.input.mouse_wheel_y :: frame :: call

export fn modifier_shift(flags: Int) -> Bool:
    return std.input.modifier_shift :: flags :: call

export fn modifier_ctrl(flags: Int) -> Bool:
    return std.input.modifier_ctrl :: flags :: call

export fn modifier_alt(flags: Int) -> Bool:
    return std.input.modifier_alt :: flags :: call

export fn modifier_meta(flags: Int) -> Bool:
    return std.input.modifier_meta :: flags :: call

export fn key_location_standard() -> Int:
    return std.input.key_location_standard :: :: call

export fn key_location_left() -> Int:
    return std.input.key_location_left :: :: call

export fn key_location_right() -> Int:
    return std.input.key_location_right :: :: call

export fn key_location_numpad() -> Int:
    return std.input.key_location_numpad :: :: call

export fn key_logical(read event: KeyEvent) -> Int:
    return std.input.key_logical :: event :: call

export fn key_physical(read event: KeyEvent) -> Int:
    return std.input.key_physical :: event :: call

export fn key_location(read event: KeyEvent) -> Int:
    return std.input.key_location :: event :: call

export fn key_text(read event: KeyEvent) -> Str:
    return std.input.key_text :: event :: call

export fn key_repeated(read event: KeyEvent) -> Bool:
    return std.input.key_repeated :: event :: call

export fn meta_logical(read meta: KeyMeta) -> Int:
    return std.input.meta_logical :: meta :: call

export fn meta_physical(read meta: KeyMeta) -> Int:
    return std.input.meta_physical :: meta :: call

export fn meta_location(read meta: KeyMeta) -> Int:
    return std.input.meta_location :: meta :: call

export fn meta_text(read meta: KeyMeta) -> Str:
    return std.input.meta_text :: meta :: call

export fn meta_repeated(read meta: KeyMeta) -> Bool:
    return std.input.meta_repeated :: meta :: call

export fn snapshot(read frame: AppFrame) -> arcana_desktop.types.InputSnapshot:
    return arcana_desktop.types.InputSnapshot :: mouse_pos = (arcana_desktop.input.mouse_pos :: frame :: call), mouse_in_window = (arcana_desktop.input.mouse_in_window :: frame :: call), mouse_wheel_y = (arcana_desktop.input.mouse_wheel_y :: frame :: call) :: call
