import arcana_desktop.types
import std.kernel.gfx
import std.kernel.text_input

fn area_value(active: Bool, position: (Int, Int), size: (Int, Int)) -> arcana_desktop.types.CompositionArea:
    let mut lifted = arcana_desktop.types.CompositionArea :: active = active, position = position, size = size :: call
    return lifted

export fn default_settings() -> arcana_desktop.types.TextInputSettings:
    let area = arcana_desktop.text_input.area_value :: false, (0, 0), (0, 0) :: call
    return arcana_desktop.types.TextInputSettings :: enabled = false, composition_area = area :: call

export fn enabled(read win: arcana_desktop.types.Window) -> Bool:
    return std.kernel.gfx.window_text_input_enabled :: win :: call

export fn set_enabled(edit win: arcana_desktop.types.Window, enabled: Bool):
    std.kernel.gfx.window_set_text_input_enabled :: win, enabled :: call

export fn composition_area(read win: arcana_desktop.types.Window) -> arcana_desktop.types.CompositionArea:
    return arcana_desktop.text_input.area_value :: (std.kernel.text_input.composition_area_active :: win :: call), (std.kernel.text_input.composition_area_position :: win :: call), (std.kernel.text_input.composition_area_size :: win :: call) :: call

export fn settings(read win: arcana_desktop.types.Window) -> arcana_desktop.types.TextInputSettings:
    return arcana_desktop.types.TextInputSettings :: enabled = (arcana_desktop.text_input.enabled :: win :: call), composition_area = (arcana_desktop.text_input.composition_area :: win :: call) :: call

export fn apply_settings(edit win: arcana_desktop.types.Window, read settings: arcana_desktop.types.TextInputSettings):
    let current = arcana_desktop.text_input.settings :: win :: call
    if current.enabled != settings.enabled:
        arcana_desktop.text_input.set_enabled :: win, settings.enabled :: call
    if not settings.enabled:
        if current.composition_area.active:
            arcana_desktop.text_input.clear_composition_area :: win :: call
        return 0
    if settings.composition_area.active:
        if not current.composition_area.active or current.composition_area.position != settings.composition_area.position or current.composition_area.size != settings.composition_area.size:
            arcana_desktop.text_input.set_composition_area :: win, settings.composition_area :: call
    else:
        if current.composition_area.active:
            arcana_desktop.text_input.clear_composition_area :: win :: call

export fn set_composition_area(edit win: arcana_desktop.types.Window, read area: arcana_desktop.types.CompositionArea):
    std.kernel.text_input.set_composition_area :: win, area.position, area.size :: call

export fn clear_composition_area(edit win: arcana_desktop.types.Window):
    std.kernel.text_input.clear_composition_area :: win :: call
