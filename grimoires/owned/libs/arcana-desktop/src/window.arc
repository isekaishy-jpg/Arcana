import arcana_desktop.text_input
import arcana_desktop.types
import std.events
import std.result
import std.window
use std.result.Result
use std.window.Window

fn lower_theme_override(read value: arcana_desktop.types.WindowThemeOverride) -> std.window.WindowThemeOverride:
    return match value:
        arcana_desktop.types.WindowThemeOverride.Light => std.window.WindowThemeOverride.Light :: :: call
        arcana_desktop.types.WindowThemeOverride.Dark => std.window.WindowThemeOverride.Dark :: :: call
        _ => std.window.WindowThemeOverride.System :: :: call

fn lift_theme_override(read value: std.window.WindowThemeOverride) -> arcana_desktop.types.WindowThemeOverride:
    return match value:
        std.window.WindowThemeOverride.Light => arcana_desktop.types.WindowThemeOverride.Light :: :: call
        std.window.WindowThemeOverride.Dark => arcana_desktop.types.WindowThemeOverride.Dark :: :: call
        _ => arcana_desktop.types.WindowThemeOverride.System :: :: call

fn lower_cursor_grab_mode(read mode: arcana_desktop.types.CursorGrabMode) -> std.window.CursorGrabMode:
    return match mode:
        arcana_desktop.types.CursorGrabMode.Confined => std.window.CursorGrabMode.Confined :: :: call
        arcana_desktop.types.CursorGrabMode.Locked => std.window.CursorGrabMode.Locked :: :: call
        _ => std.window.CursorGrabMode.Free :: :: call

fn lift_cursor_grab_mode(read mode: std.window.CursorGrabMode) -> arcana_desktop.types.CursorGrabMode:
    return match mode:
        std.window.CursorGrabMode.Confined => arcana_desktop.types.CursorGrabMode.Confined :: :: call
        std.window.CursorGrabMode.Locked => arcana_desktop.types.CursorGrabMode.Locked :: :: call
        _ => arcana_desktop.types.CursorGrabMode.Free :: :: call

fn lower_cursor_icon(read icon: arcana_desktop.types.CursorIcon) -> std.window.CursorIcon:
    return match icon:
        arcana_desktop.types.CursorIcon.Text => std.window.CursorIcon.Text :: :: call
        arcana_desktop.types.CursorIcon.Crosshair => std.window.CursorIcon.Crosshair :: :: call
        arcana_desktop.types.CursorIcon.Hand => std.window.CursorIcon.Hand :: :: call
        arcana_desktop.types.CursorIcon.Move => std.window.CursorIcon.Move :: :: call
        arcana_desktop.types.CursorIcon.Wait => std.window.CursorIcon.Wait :: :: call
        arcana_desktop.types.CursorIcon.Help => std.window.CursorIcon.Help :: :: call
        arcana_desktop.types.CursorIcon.NotAllowed => std.window.CursorIcon.NotAllowed :: :: call
        arcana_desktop.types.CursorIcon.ResizeHorizontal => std.window.CursorIcon.ResizeHorizontal :: :: call
        arcana_desktop.types.CursorIcon.ResizeVertical => std.window.CursorIcon.ResizeVertical :: :: call
        arcana_desktop.types.CursorIcon.ResizeNwse => std.window.CursorIcon.ResizeNwse :: :: call
        arcana_desktop.types.CursorIcon.ResizeNesw => std.window.CursorIcon.ResizeNesw :: :: call
        _ => std.window.CursorIcon.Default :: :: call

fn lift_cursor_icon(read icon: std.window.CursorIcon) -> arcana_desktop.types.CursorIcon:
    return match icon:
        std.window.CursorIcon.Text => arcana_desktop.types.CursorIcon.Text :: :: call
        std.window.CursorIcon.Crosshair => arcana_desktop.types.CursorIcon.Crosshair :: :: call
        std.window.CursorIcon.Hand => arcana_desktop.types.CursorIcon.Hand :: :: call
        std.window.CursorIcon.Move => arcana_desktop.types.CursorIcon.Move :: :: call
        std.window.CursorIcon.Wait => arcana_desktop.types.CursorIcon.Wait :: :: call
        std.window.CursorIcon.Help => arcana_desktop.types.CursorIcon.Help :: :: call
        std.window.CursorIcon.NotAllowed => arcana_desktop.types.CursorIcon.NotAllowed :: :: call
        std.window.CursorIcon.ResizeHorizontal => arcana_desktop.types.CursorIcon.ResizeHorizontal :: :: call
        std.window.CursorIcon.ResizeVertical => arcana_desktop.types.CursorIcon.ResizeVertical :: :: call
        std.window.CursorIcon.ResizeNwse => arcana_desktop.types.CursorIcon.ResizeNwse :: :: call
        std.window.CursorIcon.ResizeNesw => arcana_desktop.types.CursorIcon.ResizeNesw :: :: call
        _ => arcana_desktop.types.CursorIcon.Default :: :: call

fn desktop_window_bounds_base(size: (Int, Int), position: (Int, Int), visible: Bool) -> arcana_desktop.types.WindowBounds:
    let mut bounds = arcana_desktop.types.WindowBounds :: size = size, position = position, visible = visible :: call
    bounds.min_size = (0, 0)
    bounds.max_size = (0, 0)
    return bounds

fn desktop_cursor_settings_base(visible: Bool, read grab_mode: arcana_desktop.types.CursorGrabMode, read icon: arcana_desktop.types.CursorIcon) -> arcana_desktop.types.CursorSettings:
    let mut cursor = arcana_desktop.types.CursorSettings :: visible = visible, grab_mode = grab_mode, icon = icon :: call
    cursor.position = (-1, -1)
    return cursor

fn desktop_window_state_base(topmost: Bool, maximized: Bool, fullscreen: Bool) -> arcana_desktop.types.WindowState:
    let mut state = arcana_desktop.types.WindowState :: topmost = topmost, maximized = maximized, fullscreen = fullscreen :: call
    state.theme_override = arcana_desktop.types.WindowThemeOverride.System :: :: call
    return state

fn desktop_window_options_base(read style: arcana_desktop.types.WindowStyle, read state: arcana_desktop.types.WindowState, read cursor: arcana_desktop.types.CursorSettings) -> arcana_desktop.types.WindowOptions:
    let mut options = arcana_desktop.types.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = true
    return options

fn lower_cursor_settings(read settings: arcana_desktop.types.CursorSettings) -> std.window.CursorSettings:
    let mut cursor = std.window.CursorSettings :: visible = settings.visible, grab_mode = (arcana_desktop.window.lower_cursor_grab_mode :: settings.grab_mode :: call), icon = (arcana_desktop.window.lower_cursor_icon :: settings.icon :: call) :: call
    cursor.position = settings.position
    return cursor

fn lift_cursor_settings(read settings: std.window.CursorSettings) -> arcana_desktop.types.CursorSettings:
    let mut cursor = arcana_desktop.types.CursorSettings :: visible = settings.visible, grab_mode = (arcana_desktop.window.lift_cursor_grab_mode :: settings.grab_mode :: call), icon = (arcana_desktop.window.lift_cursor_icon :: settings.icon :: call) :: call
    cursor.position = settings.position
    return cursor

fn std_config(read cfg: arcana_desktop.types.WindowConfig) -> std.window.WindowConfig:
    let mut bounds = std.window.WindowBounds :: size = cfg.bounds.size, position = cfg.bounds.position, visible = cfg.bounds.visible :: call
    bounds.min_size = cfg.bounds.min_size
    bounds.max_size = cfg.bounds.max_size
    let style = std.window.WindowStyle :: resizable = cfg.options.style.resizable, decorated = cfg.options.style.decorated, transparent = cfg.options.style.transparent :: call
    let cursor = arcana_desktop.window.lower_cursor_settings :: cfg.options.cursor :: call
    let mut state = std.window.WindowState :: topmost = cfg.options.state.topmost, maximized = cfg.options.state.maximized, fullscreen = cfg.options.state.fullscreen :: call
    state.theme_override = arcana_desktop.window.lower_theme_override :: cfg.options.state.theme_override :: call
    let mut options = std.window.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = cfg.options.text_input_enabled
    return std.window.WindowConfig :: title = cfg.title, bounds = bounds, options = options :: call

fn lift_theme(read theme: std.window.WindowTheme) -> arcana_desktop.types.WindowTheme:
    return match theme:
        std.window.WindowTheme.Light => arcana_desktop.types.WindowTheme.Light :: :: call
        std.window.WindowTheme.Dark => arcana_desktop.types.WindowTheme.Dark :: :: call
        _ => arcana_desktop.types.WindowTheme.Unknown :: :: call

fn lift_monitor(read info: std.window.MonitorInfo) -> arcana_desktop.types.MonitorInfo:
    let mut monitor = arcana_desktop.types.MonitorInfo :: index = info.index, name = info.name, position = info.position :: call
    monitor.size = info.size
    monitor.scale_factor_milli = info.scale_factor_milli
    monitor.primary = info.primary
    return monitor

fn lift_settings(read settings: std.window.WindowSettings) -> arcana_desktop.types.WindowSettings:
    let mut bounds = arcana_desktop.types.WindowBounds :: size = settings.bounds.size, position = settings.bounds.position, visible = settings.bounds.visible :: call
    bounds.min_size = settings.bounds.min_size
    bounds.max_size = settings.bounds.max_size
    let style = arcana_desktop.types.WindowStyle :: resizable = settings.options.style.resizable, decorated = settings.options.style.decorated, transparent = settings.options.style.transparent :: call
    let cursor = arcana_desktop.window.lift_cursor_settings :: settings.options.cursor :: call
    let mut state = arcana_desktop.types.WindowState :: topmost = settings.options.state.topmost, maximized = settings.options.state.maximized, fullscreen = settings.options.state.fullscreen :: call
    state.theme_override = arcana_desktop.window.lift_theme_override :: settings.options.state.theme_override :: call
    let mut options = arcana_desktop.types.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = settings.options.text_input_enabled
    return arcana_desktop.types.WindowSettings :: title = settings.title, bounds = bounds, options = options :: call

fn lower_settings(read settings: arcana_desktop.types.WindowSettings) -> std.window.WindowSettings:
    let mut bounds = std.window.WindowBounds :: size = settings.bounds.size, position = settings.bounds.position, visible = settings.bounds.visible :: call
    bounds.min_size = settings.bounds.min_size
    bounds.max_size = settings.bounds.max_size
    let style = std.window.WindowStyle :: resizable = settings.options.style.resizable, decorated = settings.options.style.decorated, transparent = settings.options.style.transparent :: call
    let cursor = arcana_desktop.window.lower_cursor_settings :: settings.options.cursor :: call
    let mut state = std.window.WindowState :: topmost = settings.options.state.topmost, maximized = settings.options.state.maximized, fullscreen = settings.options.state.fullscreen :: call
    state.theme_override = arcana_desktop.window.lower_theme_override :: settings.options.state.theme_override :: call
    let mut options = std.window.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = settings.options.text_input_enabled
    return std.window.WindowSettings :: title = settings.title, bounds = bounds, options = options :: call

export fn default_config() -> arcana_desktop.types.WindowConfig:
    let mut bounds = arcana_desktop.types.WindowBounds :: size = (640, 480), position = (0, 0), visible = true :: call
    bounds.min_size = (0, 0)
    bounds.max_size = (0, 0)
    let style = arcana_desktop.types.WindowStyle :: resizable = true, decorated = true, transparent = false :: call
    let mut cursor = arcana_desktop.types.CursorSettings :: visible = true, grab_mode = (arcana_desktop.types.CursorGrabMode.Free :: :: call), icon = (arcana_desktop.types.CursorIcon.Default :: :: call) :: call
    cursor.position = (-1, -1)
    let mut state = arcana_desktop.types.WindowState :: topmost = false, maximized = false, fullscreen = false :: call
    state.theme_override = arcana_desktop.types.WindowThemeOverride.System :: :: call
    let mut options = arcana_desktop.types.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = true
    return arcana_desktop.types.WindowConfig :: title = "Arcana", bounds = bounds, options = options :: call

export fn open(title: Str, width: Int, height: Int) -> Result[Window, Str]:
    let mut cfg = arcana_desktop.window.default_config :: :: call
    let mut bounds = arcana_desktop.window.desktop_window_bounds_base :: (width, height), cfg.bounds.position, cfg.bounds.visible :: call
    bounds.min_size = cfg.bounds.min_size
    bounds.max_size = cfg.bounds.max_size
    cfg.title = title
    cfg.bounds = bounds
    return arcana_desktop.window.open_cfg :: cfg :: call

export fn open_cfg(read cfg: arcana_desktop.types.WindowConfig) -> Result[Window, Str]:
    return std.window.open_cfg :: (arcana_desktop.window.std_config :: cfg :: call) :: call

export fn open_in(edit session: std.events.AppSession, read cfg: arcana_desktop.types.WindowConfig) -> Result[Window, Str]:
    return std.window.open_in :: session, (arcana_desktop.window.std_config :: cfg :: call) :: call

export fn alive(read win: Window) -> Bool:
    return std.window.alive :: win :: call

export fn id(read win: Window) -> arcana_desktop.types.WindowId:
    return arcana_desktop.types.WindowId :: value = (std.window.id :: win :: call) :: call

export fn size(read win: Window) -> (Int, Int):
    return std.window.size :: win :: call

export fn position(read win: Window) -> (Int, Int):
    return std.window.position :: win :: call

export fn title(read win: Window) -> Str:
    return std.window.title :: win :: call

export fn visible(read win: Window) -> Bool:
    return std.window.visible :: win :: call

export fn decorated(read win: Window) -> Bool:
    return std.window.decorated :: win :: call

export fn resizable(read win: Window) -> Bool:
    return std.window.resizable :: win :: call

export fn topmost(read win: Window) -> Bool:
    return std.window.topmost :: win :: call

export fn cursor_visible(read win: Window) -> Bool:
    return std.window.cursor_visible :: win :: call

export fn min_size(read win: Window) -> (Int, Int):
    return std.window.min_size :: win :: call

export fn max_size(read win: Window) -> (Int, Int):
    return std.window.max_size :: win :: call

export fn scale_factor_milli(read win: Window) -> Int:
    return std.window.scale_factor_milli :: win :: call

export fn theme(read win: Window) -> arcana_desktop.types.WindowTheme:
    return arcana_desktop.window.lift_theme :: (std.window.theme :: win :: call) :: call

export fn transparent(read win: Window) -> Bool:
    return std.window.transparent :: win :: call

export fn theme_override(read win: Window) -> arcana_desktop.types.WindowThemeOverride:
    return arcana_desktop.window.lift_theme_override :: (std.window.theme_override :: win :: call) :: call

export fn cursor_icon(read win: Window) -> arcana_desktop.types.CursorIcon:
    return arcana_desktop.window.lift_cursor_icon :: (std.window.cursor_icon :: win :: call) :: call

export fn cursor_grab_mode(read win: Window) -> arcana_desktop.types.CursorGrabMode:
    return arcana_desktop.window.lift_cursor_grab_mode :: (std.window.cursor_grab_mode :: win :: call) :: call

export fn cursor_position(read win: Window) -> (Int, Int):
    return std.window.cursor_position :: win :: call

export fn text_input_enabled(read win: Window) -> Bool:
    return std.window.text_input_enabled :: win :: call

export fn current_monitor(read win: Window) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.lift_monitor :: (std.window.current_monitor :: win :: call) :: call

export fn primary_monitor() -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.lift_monitor :: (std.window.primary_monitor :: :: call) :: call

export fn monitor_count() -> Int:
    return std.window.monitor_count :: :: call

export fn monitor(index: Int) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.lift_monitor :: (std.window.monitor :: index :: call) :: call

export fn focused(read win: Window) -> Bool:
    return std.window.focused :: win :: call

export fn resized(read win: Window) -> Bool:
    return std.window.resized :: win :: call

export fn fullscreen(read win: Window) -> Bool:
    return std.window.fullscreen :: win :: call

export fn minimized(read win: Window) -> Bool:
    return std.window.minimized :: win :: call

export fn maximized(read win: Window) -> Bool:
    return std.window.maximized :: win :: call

export fn cursor_settings(read win: Window) -> arcana_desktop.types.CursorSettings:
    return arcana_desktop.window.lift_cursor_settings :: (std.window.cursor_settings :: win :: call) :: call

export fn settings(read win: Window) -> arcana_desktop.types.WindowSettings:
    return arcana_desktop.window.lift_settings :: (std.window.settings :: win :: call) :: call

export fn apply_settings(edit win: Window, read settings: arcana_desktop.types.WindowSettings):
    std.window.apply_settings :: win, (arcana_desktop.window.lower_settings :: settings :: call) :: call

export fn apply_cursor_settings(edit win: Window, read settings: arcana_desktop.types.CursorSettings):
    std.window.apply_cursor_settings :: win, (arcana_desktop.window.lower_cursor_settings :: settings :: call) :: call

export fn set_title(edit win: Window, title: Str):
    std.window.set_title :: win, title :: call

export fn set_position(edit win: Window, x: Int, y: Int):
    std.window.set_position :: win, x, y :: call

export fn set_size(edit win: Window, width: Int, height: Int):
    std.window.set_size :: win, width, height :: call

export fn set_visible(edit win: Window, enabled: Bool):
    std.window.set_visible :: win, enabled :: call

export fn set_decorated(edit win: Window, enabled: Bool):
    std.window.set_decorated :: win, enabled :: call

export fn set_resizable(edit win: Window, enabled: Bool):
    std.window.set_resizable :: win, enabled :: call

export fn set_min_size(edit win: Window, width: Int, height: Int):
    std.window.set_min_size :: win, width, height :: call

export fn set_max_size(edit win: Window, width: Int, height: Int):
    std.window.set_max_size :: win, width, height :: call

export fn set_fullscreen(edit win: Window, enabled: Bool):
    std.window.set_fullscreen :: win, enabled :: call

export fn set_minimized(edit win: Window, enabled: Bool):
    std.window.set_minimized :: win, enabled :: call

export fn set_maximized(edit win: Window, enabled: Bool):
    std.window.set_maximized :: win, enabled :: call

export fn set_topmost(edit win: Window, enabled: Bool):
    std.window.set_topmost :: win, enabled :: call

export fn set_cursor_visible(edit win: Window, enabled: Bool):
    std.window.set_cursor_visible :: win, enabled :: call

export fn set_transparent(edit win: Window, enabled: Bool):
    std.window.set_transparent :: win, enabled :: call

export fn set_theme_override(edit win: Window, read value: arcana_desktop.types.WindowThemeOverride):
    std.window.set_theme_override :: win, (arcana_desktop.window.lower_theme_override :: value :: call) :: call

export fn set_cursor_icon(edit win: Window, read icon: arcana_desktop.types.CursorIcon):
    std.window.set_cursor_icon :: win, (arcana_desktop.window.lower_cursor_icon :: icon :: call) :: call

export fn set_cursor_grab_mode(edit win: Window, read mode: arcana_desktop.types.CursorGrabMode):
    std.window.set_cursor_grab_mode :: win, (arcana_desktop.window.lower_cursor_grab_mode :: mode :: call) :: call

export fn set_cursor_position(edit win: Window, x: Int, y: Int):
    std.window.set_cursor_position :: win, x, y :: call

export fn set_text_input_enabled(edit win: Window, enabled: Bool):
    std.window.set_text_input_enabled :: win, enabled :: call

export fn text_input_settings(read win: Window) -> arcana_desktop.types.TextInputSettings:
    return arcana_desktop.text_input.settings :: win :: call

export fn apply_text_input_settings(edit win: Window, read settings: arcana_desktop.types.TextInputSettings):
    arcana_desktop.text_input.apply_settings :: win, settings :: call

export fn request_redraw(edit win: Window):
    std.window.request_redraw :: win :: call

export fn request_attention(edit win: Window, enabled: Bool):
    std.window.request_attention :: win, enabled :: call

export fn close(take win: Window) -> Result[Unit, Str]:
    return std.window.close :: win :: call
