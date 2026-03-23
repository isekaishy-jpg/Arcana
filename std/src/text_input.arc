import std.kernel.text_input
import std.window
use std.window.Window

export record CompositionArea:
    active: Bool
    position: (Int, Int)
    size: (Int, Int)

export record TextInputSettings:
    enabled: Bool
    composition_area: std.text_input.CompositionArea

fn composition_area_value(active: Bool, position: (Int, Int), size: (Int, Int)) -> std.text_input.CompositionArea:
    let mut area = std.text_input.CompositionArea :: active = active, position = position, size = size :: call
    return area

export fn default_settings() -> std.text_input.TextInputSettings:
    let area = std.text_input.composition_area_value :: false, (0, 0), (0, 0) :: call
    return std.text_input.TextInputSettings :: enabled = false, composition_area = area :: call

export fn enabled(read win: Window) -> Bool:
    return std.window.text_input_enabled :: win :: call

export fn set_enabled(edit win: Window, enabled: Bool):
    std.window.set_text_input_enabled :: win, enabled :: call

export fn composition_area(read win: Window) -> std.text_input.CompositionArea:
    return std.text_input.composition_area_value :: (std.kernel.text_input.composition_area_active :: win :: call), (std.kernel.text_input.composition_area_position :: win :: call), (std.kernel.text_input.composition_area_size :: win :: call) :: call

export fn settings(read win: Window) -> std.text_input.TextInputSettings:
    return std.text_input.TextInputSettings :: enabled = (std.text_input.enabled :: win :: call), composition_area = (std.text_input.composition_area :: win :: call) :: call

export fn apply_settings(edit win: Window, read settings: std.text_input.TextInputSettings):
    let current = std.text_input.settings :: win :: call
    if current.enabled != settings.enabled:
        std.text_input.set_enabled :: win, settings.enabled :: call
    if not settings.enabled:
        if current.composition_area.active:
            std.text_input.clear_composition_area :: win :: call
        return
    if settings.composition_area.active:
        if not current.composition_area.active or current.composition_area.position != settings.composition_area.position or current.composition_area.size != settings.composition_area.size:
            std.text_input.set_composition_area :: win, settings.composition_area :: call
    else:
        if current.composition_area.active:
            std.text_input.clear_composition_area :: win :: call

export fn set_composition_area(edit win: Window, read area: std.text_input.CompositionArea):
    std.kernel.text_input.set_composition_area :: win, area.position, area.size :: call

export fn clear_composition_area(edit win: Window):
    std.kernel.text_input.clear_composition_area :: win :: call
