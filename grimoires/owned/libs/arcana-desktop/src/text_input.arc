import arcana_desktop.types
import std.text_input
use std.window.Window

fn lift_area(read area: std.text_input.CompositionArea) -> arcana_desktop.types.CompositionArea:
    let mut lifted = arcana_desktop.types.CompositionArea :: active = area.active, position = area.position, size = area.size :: call
    return lifted

fn lower_area(read area: arcana_desktop.types.CompositionArea) -> std.text_input.CompositionArea:
    return std.text_input.composition_area_value :: area.active, area.position, area.size :: call

fn lift_settings(read settings: std.text_input.TextInputSettings) -> arcana_desktop.types.TextInputSettings:
    return arcana_desktop.types.TextInputSettings :: enabled = settings.enabled, composition_area = (arcana_desktop.text_input.lift_area :: settings.composition_area :: call) :: call

fn lower_settings(read settings: arcana_desktop.types.TextInputSettings) -> std.text_input.TextInputSettings:
    return std.text_input.TextInputSettings :: enabled = settings.enabled, composition_area = (arcana_desktop.text_input.lower_area :: settings.composition_area :: call) :: call

export fn default_settings() -> arcana_desktop.types.TextInputSettings:
    let area = arcana_desktop.text_input.lift_area :: (std.text_input.composition_area_value :: false, (0, 0), (0, 0) :: call) :: call
    return arcana_desktop.types.TextInputSettings :: enabled = true, composition_area = area :: call

export fn enabled(read win: Window) -> Bool:
    return std.text_input.enabled :: win :: call

export fn set_enabled(edit win: Window, enabled: Bool):
    std.text_input.set_enabled :: win, enabled :: call

export fn composition_area(read win: Window) -> arcana_desktop.types.CompositionArea:
    return arcana_desktop.text_input.lift_area :: (std.text_input.composition_area :: win :: call) :: call

export fn settings(read win: Window) -> arcana_desktop.types.TextInputSettings:
    return arcana_desktop.text_input.lift_settings :: (std.text_input.settings :: win :: call) :: call

export fn apply_settings(edit win: Window, read settings: arcana_desktop.types.TextInputSettings):
    std.text_input.apply_settings :: win, (arcana_desktop.text_input.lower_settings :: settings :: call) :: call

export fn set_composition_area(edit win: Window, read area: arcana_desktop.types.CompositionArea):
    std.text_input.set_composition_area :: win, (arcana_desktop.text_input.lower_area :: area :: call) :: call

export fn clear_composition_area(edit win: Window):
    std.text_input.clear_composition_area :: win :: call
