import arcana_desktop.events
import arcana_desktop.text_input
import arcana_desktop.types
import arcana_winapi.helpers.window
import std.result
use std.result.Result

fn theme_override_code(read value: arcana_desktop.types.WindowThemeOverride) -> Int:
    return match value:
        arcana_desktop.types.WindowThemeOverride.Light => 1
        arcana_desktop.types.WindowThemeOverride.Dark => 2
        _ => 0

fn lift_theme_override(code: Int) -> arcana_desktop.types.WindowThemeOverride:
    if code == 1:
        return arcana_desktop.types.WindowThemeOverride.Light :: :: call
    if code == 2:
        return arcana_desktop.types.WindowThemeOverride.Dark :: :: call
    return arcana_desktop.types.WindowThemeOverride.System :: :: call

fn cursor_grab_mode_code(read mode: arcana_desktop.types.CursorGrabMode) -> Int:
    return match mode:
        arcana_desktop.types.CursorGrabMode.Confined => 1
        arcana_desktop.types.CursorGrabMode.Locked => 2
        _ => 0

fn lift_cursor_grab_mode(code: Int) -> arcana_desktop.types.CursorGrabMode:
    if code == 1:
        return arcana_desktop.types.CursorGrabMode.Confined :: :: call
    if code == 2:
        return arcana_desktop.types.CursorGrabMode.Locked :: :: call
    return arcana_desktop.types.CursorGrabMode.Free :: :: call

fn cursor_icon_code(read icon: arcana_desktop.types.CursorIcon) -> Int:
    return match icon:
        arcana_desktop.types.CursorIcon.Text => 1
        arcana_desktop.types.CursorIcon.Crosshair => 2
        arcana_desktop.types.CursorIcon.Hand => 3
        arcana_desktop.types.CursorIcon.Move => 4
        arcana_desktop.types.CursorIcon.Wait => 5
        arcana_desktop.types.CursorIcon.Help => 6
        arcana_desktop.types.CursorIcon.NotAllowed => 7
        arcana_desktop.types.CursorIcon.ResizeHorizontal => 8
        arcana_desktop.types.CursorIcon.ResizeVertical => 9
        arcana_desktop.types.CursorIcon.ResizeNwse => 10
        arcana_desktop.types.CursorIcon.ResizeNesw => 11
        _ => 0

fn lift_cursor_icon(code: Int) -> arcana_desktop.types.CursorIcon:
    if code == 1:
        return arcana_desktop.types.CursorIcon.Text :: :: call
    if code == 2:
        return arcana_desktop.types.CursorIcon.Crosshair :: :: call
    if code == 3:
        return arcana_desktop.types.CursorIcon.Hand :: :: call
    if code == 4:
        return arcana_desktop.types.CursorIcon.Move :: :: call
    if code == 5:
        return arcana_desktop.types.CursorIcon.Wait :: :: call
    if code == 6:
        return arcana_desktop.types.CursorIcon.Help :: :: call
    if code == 7:
        return arcana_desktop.types.CursorIcon.NotAllowed :: :: call
    if code == 8:
        return arcana_desktop.types.CursorIcon.ResizeHorizontal :: :: call
    if code == 9:
        return arcana_desktop.types.CursorIcon.ResizeVertical :: :: call
    if code == 10:
        return arcana_desktop.types.CursorIcon.ResizeNwse :: :: call
    if code == 11:
        return arcana_desktop.types.CursorIcon.ResizeNesw :: :: call
    return arcana_desktop.types.CursorIcon.Default :: :: call

fn lift_theme(code: Int) -> arcana_desktop.types.WindowTheme:
    if code == 1:
        return arcana_desktop.types.WindowTheme.Light :: :: call
    if code == 2:
        return arcana_desktop.types.WindowTheme.Dark :: :: call
    return arcana_desktop.types.WindowTheme.Unknown :: :: call

fn settings_for_config(read cfg: arcana_desktop.types.WindowConfig) -> arcana_desktop.types.WindowSettings:
    let mut options = arcana_desktop.types.WindowOptions :: style = cfg.options.style, state = cfg.options.state, cursor = cfg.options.cursor :: call
    options.text_input_enabled = cfg.options.text_input_enabled
    return arcana_desktop.types.WindowSettings :: title = cfg.title, bounds = cfg.bounds, options = options :: call

fn apply_config(take win: arcana_winapi.desktop_handles.Window, read cfg: arcana_desktop.types.WindowConfig) -> arcana_winapi.desktop_handles.Window:
    let mut win = win
    arcana_desktop.window.apply_settings :: win, (arcana_desktop.window.settings_for_config :: cfg :: call) :: call
    return win

fn monitor_info(index: Int) -> arcana_desktop.types.MonitorInfo:
    let mut info = arcana_desktop.types.MonitorInfo :: index = index, name = (arcana_winapi.helpers.window.window_monitor_name :: index :: call), position = (arcana_winapi.helpers.window.window_monitor_position :: index :: call) :: call
    info.size = arcana_winapi.helpers.window.window_monitor_size :: index :: call
    info.scale_factor_milli = arcana_winapi.helpers.window.window_monitor_scale_factor_milli :: index :: call
    info.primary = arcana_winapi.helpers.window.window_monitor_is_primary :: index :: call
    return info

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
    options.text_input_enabled = false
    return arcana_desktop.types.WindowConfig :: title = "Arcana", bounds = bounds, options = options :: call

export fn open(title: Str, width: Int, height: Int) -> Result[arcana_winapi.desktop_handles.Window, Str]:
    let mut cfg = arcana_desktop.window.default_config :: :: call
    let mut bounds = arcana_desktop.types.WindowBounds :: size = (width, height), position = cfg.bounds.position, visible = cfg.bounds.visible :: call
    bounds.min_size = cfg.bounds.min_size
    bounds.max_size = cfg.bounds.max_size
    cfg.title = title
    cfg.bounds = bounds
    return arcana_desktop.window.open_cfg :: cfg :: call

export fn open_cfg(read cfg: arcana_desktop.types.WindowConfig) -> Result[arcana_winapi.desktop_handles.Window, Str]:
    return match (arcana_winapi.helpers.window.window_open :: cfg.title, cfg.bounds.size.0, cfg.bounds.size.1 :: call):
        Result.Ok(value) => Result.Ok[arcana_winapi.desktop_handles.Window, Str] :: (arcana_desktop.window.apply_config :: value, cfg :: call) :: call
        Result.Err(err) => Result.Err[arcana_winapi.desktop_handles.Window, Str] :: err :: call

export fn open_in(edit session: arcana_winapi.desktop_handles.Session, read cfg: arcana_desktop.types.WindowConfig) -> Result[arcana_winapi.desktop_handles.Window, Str]:
    return match (arcana_desktop.window.open_cfg :: cfg :: call):
        Result.Ok(value) => open_in_ready :: session, value :: call
        Result.Err(err) => Result.Err[arcana_winapi.desktop_handles.Window, Str] :: err :: call

fn open_in_ready(edit session: arcana_winapi.desktop_handles.Session, take value: arcana_winapi.desktop_handles.Window) -> Result[arcana_winapi.desktop_handles.Window, Str]:
    arcana_desktop.events.attach_window :: session, value :: call
    return Result.Ok[arcana_winapi.desktop_handles.Window, Str] :: value :: call

export fn alive(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_alive :: win :: call

export fn id(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.WindowId:
    return arcana_desktop.types.WindowId :: value = (arcana_winapi.helpers.window.window_id :: win :: call) :: call

export fn native_handle(read win: arcana_winapi.desktop_handles.Window) -> Int:
    return arcana_winapi.helpers.window.window_native_handle :: win :: call

export fn size(read win: arcana_winapi.desktop_handles.Window) -> (Int, Int):
    return arcana_winapi.helpers.window.window_size :: win :: call

export fn position(read win: arcana_winapi.desktop_handles.Window) -> (Int, Int):
    return arcana_winapi.helpers.window.window_position :: win :: call

export fn title(read win: arcana_winapi.desktop_handles.Window) -> Str:
    return arcana_winapi.helpers.window.window_title :: win :: call

export fn visible(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_visible :: win :: call

export fn decorated(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_decorated :: win :: call

export fn resizable(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_resizable :: win :: call

export fn topmost(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_topmost :: win :: call

export fn cursor_visible(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_cursor_visible :: win :: call

export fn min_size(read win: arcana_winapi.desktop_handles.Window) -> (Int, Int):
    return arcana_winapi.helpers.window.window_min_size :: win :: call

export fn max_size(read win: arcana_winapi.desktop_handles.Window) -> (Int, Int):
    return arcana_winapi.helpers.window.window_max_size :: win :: call

export fn scale_factor_milli(read win: arcana_winapi.desktop_handles.Window) -> Int:
    return arcana_winapi.helpers.window.window_scale_factor_milli :: win :: call

export fn theme(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.WindowTheme:
    return arcana_desktop.window.lift_theme :: (arcana_winapi.helpers.window.window_theme_code :: win :: call) :: call

export fn transparent(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_transparent :: win :: call

export fn theme_override(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.WindowThemeOverride:
    return arcana_desktop.window.lift_theme_override :: (arcana_winapi.helpers.window.window_theme_override_code :: win :: call) :: call

export fn cursor_icon(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.CursorIcon:
    return arcana_desktop.window.lift_cursor_icon :: (arcana_winapi.helpers.window.window_cursor_icon_code :: win :: call) :: call

export fn cursor_grab_mode(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.CursorGrabMode:
    return arcana_desktop.window.lift_cursor_grab_mode :: (arcana_winapi.helpers.window.window_cursor_grab_mode :: win :: call) :: call

export fn cursor_position(read win: arcana_winapi.desktop_handles.Window) -> (Int, Int):
    return arcana_winapi.helpers.window.window_cursor_position :: win :: call

export fn text_input_enabled(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_desktop.text_input.enabled :: win :: call

export fn current_monitor(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.monitor_info :: (arcana_winapi.helpers.window.window_current_monitor_index :: win :: call) :: call

export fn primary_monitor() -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.monitor_info :: (arcana_winapi.helpers.window.window_primary_monitor_index :: :: call) :: call

export fn monitor_count() -> Int:
    return arcana_winapi.helpers.window.window_monitor_count :: :: call

export fn monitor(index: Int) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.monitor_info :: index :: call

export fn focused(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_focused :: win :: call

export fn resized(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_resized :: win :: call

export fn fullscreen(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_fullscreen :: win :: call

export fn minimized(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_minimized :: win :: call

export fn maximized(read win: arcana_winapi.desktop_handles.Window) -> Bool:
    return arcana_winapi.helpers.window.window_maximized :: win :: call

export fn cursor_settings(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.CursorSettings:
    let mut cursor = arcana_desktop.types.CursorSettings :: visible = (arcana_winapi.helpers.window.window_cursor_visible :: win :: call), grab_mode = (arcana_desktop.types.CursorGrabMode.Free :: :: call), icon = (arcana_desktop.types.CursorIcon.Default :: :: call) :: call
    cursor.grab_mode = arcana_desktop.window.lift_cursor_grab_mode :: (arcana_winapi.helpers.window.window_cursor_grab_mode :: win :: call) :: call
    cursor.icon = arcana_desktop.window.lift_cursor_icon :: (arcana_winapi.helpers.window.window_cursor_icon_code :: win :: call) :: call
    cursor.position = arcana_winapi.helpers.window.window_cursor_position :: win :: call
    return cursor

export fn settings(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.WindowSettings:
    let mut bounds = arcana_desktop.types.WindowBounds :: size = (arcana_winapi.helpers.window.window_size :: win :: call), position = (arcana_winapi.helpers.window.window_position :: win :: call), visible = (arcana_winapi.helpers.window.window_visible :: win :: call) :: call
    bounds.min_size = arcana_winapi.helpers.window.window_min_size :: win :: call
    bounds.max_size = arcana_winapi.helpers.window.window_max_size :: win :: call
    let style = arcana_desktop.types.WindowStyle :: resizable = (arcana_winapi.helpers.window.window_resizable :: win :: call), decorated = (arcana_winapi.helpers.window.window_decorated :: win :: call), transparent = (arcana_winapi.helpers.window.window_transparent :: win :: call) :: call
    let cursor = arcana_desktop.window.cursor_settings :: win :: call
    let mut state = arcana_desktop.types.WindowState :: topmost = (arcana_winapi.helpers.window.window_topmost :: win :: call), maximized = (arcana_winapi.helpers.window.window_maximized :: win :: call), fullscreen = (arcana_winapi.helpers.window.window_fullscreen :: win :: call) :: call
    state.theme_override = arcana_desktop.window.lift_theme_override :: (arcana_winapi.helpers.window.window_theme_override_code :: win :: call) :: call
    let mut options = arcana_desktop.types.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = arcana_desktop.text_input.enabled :: win :: call
    return arcana_desktop.types.WindowSettings :: title = (arcana_winapi.helpers.window.window_title :: win :: call), bounds = bounds, options = options :: call

export fn apply_settings(edit win: arcana_winapi.desktop_handles.Window, read settings: arcana_desktop.types.WindowSettings):
    let current = arcana_desktop.window.settings :: win :: call
    if current.options.state.fullscreen and not settings.options.state.fullscreen:
        arcana_desktop.window.set_fullscreen :: win, false :: call
    if current.options.state.maximized and not settings.options.state.maximized:
        arcana_desktop.window.set_maximized :: win, false :: call
    if current.bounds.min_size != settings.bounds.min_size:
        arcana_desktop.window.set_min_size :: win, settings.bounds.min_size.0, settings.bounds.min_size.1 :: call
    if current.bounds.max_size != settings.bounds.max_size:
        arcana_desktop.window.set_max_size :: win, settings.bounds.max_size.0, settings.bounds.max_size.1 :: call
    if current.bounds.size != settings.bounds.size:
        arcana_desktop.window.set_size :: win, settings.bounds.size.0, settings.bounds.size.1 :: call
    if current.bounds.position != settings.bounds.position:
        arcana_desktop.window.set_position :: win, settings.bounds.position.0, settings.bounds.position.1 :: call
    if current.title != settings.title:
        arcana_desktop.window.set_title :: win, settings.title :: call
    if current.options.style.resizable != settings.options.style.resizable:
        arcana_desktop.window.set_resizable :: win, settings.options.style.resizable :: call
    if current.options.style.decorated != settings.options.style.decorated:
        arcana_desktop.window.set_decorated :: win, settings.options.style.decorated :: call
    if current.options.style.transparent != settings.options.style.transparent:
        arcana_desktop.window.set_transparent :: win, settings.options.style.transparent :: call
    if current.options.state.topmost != settings.options.state.topmost:
        arcana_desktop.window.set_topmost :: win, settings.options.state.topmost :: call
    if current.options.state.theme_override != settings.options.state.theme_override:
        arcana_desktop.window.set_theme_override :: win, settings.options.state.theme_override :: call
    if current.options.cursor.visible != settings.options.cursor.visible:
        arcana_desktop.window.set_cursor_visible :: win, settings.options.cursor.visible :: call
    if current.options.cursor.icon != settings.options.cursor.icon:
        arcana_desktop.window.set_cursor_icon :: win, settings.options.cursor.icon :: call
    if current.options.cursor.grab_mode != settings.options.cursor.grab_mode:
        arcana_desktop.window.set_cursor_grab_mode :: win, settings.options.cursor.grab_mode :: call
    if settings.options.cursor.position.0 >= 0 and settings.options.cursor.position.1 >= 0:
        if current.options.cursor.position != settings.options.cursor.position:
            arcana_desktop.window.set_cursor_position :: win, settings.options.cursor.position.0, settings.options.cursor.position.1 :: call
    if current.options.text_input_enabled != settings.options.text_input_enabled:
        arcana_desktop.window.set_text_input_enabled :: win, settings.options.text_input_enabled :: call
    if current.options.state.maximized != settings.options.state.maximized and settings.options.state.maximized:
        arcana_desktop.window.set_maximized :: win, true :: call
    if current.options.state.fullscreen != settings.options.state.fullscreen and settings.options.state.fullscreen:
        arcana_desktop.window.set_fullscreen :: win, true :: call
    if current.bounds.visible != settings.bounds.visible:
        arcana_desktop.window.set_visible :: win, settings.bounds.visible :: call

export fn apply_cursor_settings(edit win: arcana_winapi.desktop_handles.Window, read settings: arcana_desktop.types.CursorSettings):
    let current = arcana_desktop.window.cursor_settings :: win :: call
    if current.visible != settings.visible:
        arcana_desktop.window.set_cursor_visible :: win, settings.visible :: call
    if current.icon != settings.icon:
        arcana_desktop.window.set_cursor_icon :: win, settings.icon :: call
    if current.grab_mode != settings.grab_mode:
        arcana_desktop.window.set_cursor_grab_mode :: win, settings.grab_mode :: call
    if settings.position.0 >= 0 and settings.position.1 >= 0:
        if current.position != settings.position:
            arcana_desktop.window.set_cursor_position :: win, settings.position.0, settings.position.1 :: call

export fn set_title(edit win: arcana_winapi.desktop_handles.Window, title: Str):
    arcana_winapi.helpers.window.window_set_title :: win, title :: call

export fn set_position(edit win: arcana_winapi.desktop_handles.Window, x: Int, y: Int):
    arcana_winapi.helpers.window.window_set_position :: win, x, y :: call

export fn set_size(edit win: arcana_winapi.desktop_handles.Window, width: Int, height: Int):
    arcana_winapi.helpers.window.window_set_size :: win, width, height :: call

export fn set_visible(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_visible :: win, enabled :: call

export fn set_decorated(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_decorated :: win, enabled :: call

export fn set_resizable(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_resizable :: win, enabled :: call

export fn set_min_size(edit win: arcana_winapi.desktop_handles.Window, width: Int, height: Int):
    arcana_winapi.helpers.window.window_set_min_size :: win, width, height :: call

export fn set_max_size(edit win: arcana_winapi.desktop_handles.Window, width: Int, height: Int):
    arcana_winapi.helpers.window.window_set_max_size :: win, width, height :: call

export fn set_fullscreen(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_fullscreen :: win, enabled :: call

export fn set_minimized(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_minimized :: win, enabled :: call

export fn set_maximized(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_maximized :: win, enabled :: call

export fn set_topmost(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_topmost :: win, enabled :: call

export fn set_cursor_visible(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_cursor_visible :: win, enabled :: call

export fn set_transparent(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_set_transparent :: win, enabled :: call

export fn set_theme_override(edit win: arcana_winapi.desktop_handles.Window, read value: arcana_desktop.types.WindowThemeOverride):
    arcana_winapi.helpers.window.window_set_theme_override_code :: win, (arcana_desktop.window.theme_override_code :: value :: call) :: call

export fn set_cursor_icon(edit win: arcana_winapi.desktop_handles.Window, read icon: arcana_desktop.types.CursorIcon):
    arcana_winapi.helpers.window.window_set_cursor_icon_code :: win, (arcana_desktop.window.cursor_icon_code :: icon :: call) :: call

export fn set_cursor_grab_mode(edit win: arcana_winapi.desktop_handles.Window, read mode: arcana_desktop.types.CursorGrabMode):
    arcana_winapi.helpers.window.window_set_cursor_grab_mode :: win, (arcana_desktop.window.cursor_grab_mode_code :: mode :: call) :: call

export fn set_cursor_position(edit win: arcana_winapi.desktop_handles.Window, x: Int, y: Int):
    arcana_winapi.helpers.window.window_set_cursor_position :: win, x, y :: call

export fn set_text_input_enabled(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_desktop.text_input.set_enabled :: win, enabled :: call

export fn text_input_settings(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.TextInputSettings:
    return arcana_desktop.text_input.settings :: win :: call

export fn apply_text_input_settings(edit win: arcana_winapi.desktop_handles.Window, read settings: arcana_desktop.types.TextInputSettings):
    arcana_desktop.text_input.apply_settings :: win, settings :: call

export fn request_redraw(edit win: arcana_winapi.desktop_handles.Window):
    arcana_winapi.helpers.window.window_request_redraw :: win :: call

export fn request_attention(edit win: arcana_winapi.desktop_handles.Window, enabled: Bool):
    arcana_winapi.helpers.window.window_request_attention :: win, enabled :: call

export fn close(take win: arcana_winapi.desktop_handles.Window) -> Result[Unit, Str]:
    return arcana_winapi.helpers.window.window_close :: win :: call



