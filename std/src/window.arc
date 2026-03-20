import std.events
import std.kernel.gfx
import std.result
use std.result.Result

export opaque type Window as move, boundary_unsafe

export enum WindowTheme:
    Unknown
    Light
    Dark

export enum WindowThemeOverride:
    System
    Light
    Dark

export enum CursorGrabMode:
    Free
    Confined
    Locked

export enum CursorIcon:
    Default
    Text
    Crosshair
    Hand
    Move
    Wait
    Help
    NotAllowed
    ResizeHorizontal
    ResizeVertical
    ResizeNwse
    ResizeNesw

export record MonitorInfo:
    index: Int
    name: Str
    position: (Int, Int)
    size: (Int, Int)
    scale_factor_milli: Int
    primary: Bool

export record WindowBounds:
    size: (Int, Int)
    position: (Int, Int)
    visible: Bool
    min_size: (Int, Int)
    max_size: (Int, Int)

export record WindowStyle:
    resizable: Bool
    decorated: Bool
    transparent: Bool

export record CursorSettings:
    visible: Bool
    grab_mode: std.window.CursorGrabMode
    icon: std.window.CursorIcon
    position: (Int, Int)

export record WindowState:
    topmost: Bool
    maximized: Bool
    fullscreen: Bool
    theme_override: std.window.WindowThemeOverride

export record WindowOptions:
    style: std.window.WindowStyle
    state: std.window.WindowState
    cursor: std.window.CursorSettings
    text_input_enabled: Bool

export record WindowConfig:
    title: Str
    bounds: std.window.WindowBounds
    options: std.window.WindowOptions

export record WindowSettings:
    title: Str
    bounds: std.window.WindowBounds
    options: std.window.WindowOptions

fn window_bounds_base(size: (Int, Int), position: (Int, Int), visible: Bool) -> std.window.WindowBounds:
    let mut bounds = std.window.WindowBounds :: size = size, position = position, visible = visible :: call
    bounds.min_size = (0, 0)
    bounds.max_size = (0, 0)
    return bounds

fn cursor_settings_base(visible: Bool, read grab_mode: std.window.CursorGrabMode, read icon: std.window.CursorIcon) -> std.window.CursorSettings:
    let mut cursor = std.window.CursorSettings :: visible = visible, grab_mode = grab_mode, icon = icon :: call
    cursor.position = (-1, -1)
    return cursor

fn window_state_base(topmost: Bool, maximized: Bool, fullscreen: Bool) -> std.window.WindowState:
    let mut state = std.window.WindowState :: topmost = topmost, maximized = maximized, fullscreen = fullscreen :: call
    state.theme_override = std.window.WindowThemeOverride.System :: :: call
    return state

fn window_options_base(read style: std.window.WindowStyle, read state: std.window.WindowState, read cursor: std.window.CursorSettings) -> std.window.WindowOptions:
    let mut options = std.window.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = true
    return options

fn theme_override_code(read value: std.window.WindowThemeOverride) -> Int:
    return match value:
        std.window.WindowThemeOverride.Light => 1
        std.window.WindowThemeOverride.Dark => 2
        _ => 0

fn lift_theme_override(code: Int) -> std.window.WindowThemeOverride:
    if code == 1:
        return std.window.WindowThemeOverride.Light :: :: call
    if code == 2:
        return std.window.WindowThemeOverride.Dark :: :: call
    return std.window.WindowThemeOverride.System :: :: call

fn cursor_grab_mode_code(read mode: std.window.CursorGrabMode) -> Int:
    return match mode:
        std.window.CursorGrabMode.Confined => 1
        std.window.CursorGrabMode.Locked => 2
        _ => 0

fn lift_cursor_grab_mode(code: Int) -> std.window.CursorGrabMode:
    if code == 1:
        return std.window.CursorGrabMode.Confined :: :: call
    if code == 2:
        return std.window.CursorGrabMode.Locked :: :: call
    return std.window.CursorGrabMode.Free :: :: call

fn cursor_icon_code(read icon: std.window.CursorIcon) -> Int:
    return match icon:
        std.window.CursorIcon.Text => 1
        std.window.CursorIcon.Crosshair => 2
        std.window.CursorIcon.Hand => 3
        std.window.CursorIcon.Move => 4
        std.window.CursorIcon.Wait => 5
        std.window.CursorIcon.Help => 6
        std.window.CursorIcon.NotAllowed => 7
        std.window.CursorIcon.ResizeHorizontal => 8
        std.window.CursorIcon.ResizeVertical => 9
        std.window.CursorIcon.ResizeNwse => 10
        std.window.CursorIcon.ResizeNesw => 11
        _ => 0

fn lift_cursor_icon(code: Int) -> std.window.CursorIcon:
    if code == 1:
        return std.window.CursorIcon.Text :: :: call
    if code == 2:
        return std.window.CursorIcon.Crosshair :: :: call
    if code == 3:
        return std.window.CursorIcon.Hand :: :: call
    if code == 4:
        return std.window.CursorIcon.Move :: :: call
    if code == 5:
        return std.window.CursorIcon.Wait :: :: call
    if code == 6:
        return std.window.CursorIcon.Help :: :: call
    if code == 7:
        return std.window.CursorIcon.NotAllowed :: :: call
    if code == 8:
        return std.window.CursorIcon.ResizeHorizontal :: :: call
    if code == 9:
        return std.window.CursorIcon.ResizeVertical :: :: call
    if code == 10:
        return std.window.CursorIcon.ResizeNwse :: :: call
    if code == 11:
        return std.window.CursorIcon.ResizeNesw :: :: call
    return std.window.CursorIcon.Default :: :: call

export fn default_config() -> std.window.WindowConfig:
    let mut bounds = std.window.window_bounds_base :: (640, 480), (0, 0), true :: call
    bounds.min_size = (0, 0)
    bounds.max_size = (0, 0)
    let style = std.window.WindowStyle :: resizable = true, decorated = true, transparent = false :: call
    let mut cursor = std.window.cursor_settings_base :: true, (std.window.CursorGrabMode.Free :: :: call), (std.window.CursorIcon.Default :: :: call) :: call
    cursor.position = (-1, -1)
    let mut state = std.window.window_state_base :: false, false, false :: call
    state.theme_override = std.window.WindowThemeOverride.System :: :: call
    let mut options = std.window.window_options_base :: style, state, cursor :: call
    options.text_input_enabled = true
    return std.window.WindowConfig :: title = "Arcana", bounds = bounds, options = options :: call

fn apply_config(take win: Window, read cfg: std.window.WindowConfig) -> Window:
    let mut win = win
    let theme_override_code = match cfg.options.state.theme_override:
        std.window.WindowThemeOverride.Light => 1
        std.window.WindowThemeOverride.Dark => 2
        _ => 0
    let cursor_icon_code = match cfg.options.cursor.icon:
        std.window.CursorIcon.Text => 1
        std.window.CursorIcon.Crosshair => 2
        std.window.CursorIcon.Hand => 3
        std.window.CursorIcon.Move => 4
        std.window.CursorIcon.Wait => 5
        std.window.CursorIcon.Help => 6
        std.window.CursorIcon.NotAllowed => 7
        std.window.CursorIcon.ResizeHorizontal => 8
        std.window.CursorIcon.ResizeVertical => 9
        std.window.CursorIcon.ResizeNwse => 10
        std.window.CursorIcon.ResizeNesw => 11
        _ => 0
    let cursor_grab_mode = match cfg.options.cursor.grab_mode:
        std.window.CursorGrabMode.Confined => 1
        std.window.CursorGrabMode.Locked => 2
        _ => 0
    std.kernel.gfx.window_set_position :: win, cfg.bounds.position.0, cfg.bounds.position.1 :: call
    std.kernel.gfx.window_set_min_size :: win, cfg.bounds.min_size.0, cfg.bounds.min_size.1 :: call
    std.kernel.gfx.window_set_max_size :: win, cfg.bounds.max_size.0, cfg.bounds.max_size.1 :: call
    std.kernel.gfx.window_set_resizable :: win, cfg.options.style.resizable :: call
    std.kernel.gfx.window_set_decorated :: win, cfg.options.style.decorated :: call
    std.kernel.gfx.window_set_transparent :: win, cfg.options.style.transparent :: call
    std.kernel.gfx.window_set_topmost :: win, cfg.options.state.topmost :: call
    std.kernel.gfx.window_set_maximized :: win, cfg.options.state.maximized :: call
    std.kernel.gfx.window_set_fullscreen :: win, cfg.options.state.fullscreen :: call
    std.kernel.gfx.window_set_theme_override_code :: win, theme_override_code :: call
    std.kernel.gfx.window_set_cursor_visible :: win, cfg.options.cursor.visible :: call
    std.kernel.gfx.window_set_cursor_icon_code :: win, cursor_icon_code :: call
    std.kernel.gfx.window_set_cursor_grab_mode :: win, cursor_grab_mode :: call
    if cfg.options.cursor.position.0 >= 0 and cfg.options.cursor.position.1 >= 0:
        std.kernel.gfx.window_set_cursor_position :: win, cfg.options.cursor.position.0, cfg.options.cursor.position.1 :: call
    std.kernel.gfx.window_set_text_input_enabled :: win, cfg.options.text_input_enabled :: call
    std.kernel.gfx.window_set_visible :: win, cfg.bounds.visible :: call
    return win

export fn open(title: Str, width: Int, height: Int) -> Result[Window, Str]:
    let mut cfg = std.window.default_config :: :: call
    let mut bounds = std.window.window_bounds_base :: (width, height), cfg.bounds.position, cfg.bounds.visible :: call
    bounds.min_size = cfg.bounds.min_size
    bounds.max_size = cfg.bounds.max_size
    cfg.title = title
    cfg.bounds = bounds
    return std.window.open_cfg :: cfg :: call

export fn open_cfg(read cfg: std.window.WindowConfig) -> Result[Window, Str]:
    return match (std.kernel.gfx.window_open :: cfg.title, cfg.bounds.size.0, cfg.bounds.size.1 :: call):
        Result.Ok(value) => Result.Ok[Window, Str] :: (std.window.apply_config :: value, cfg :: call) :: call
        Result.Err(err) => Result.Err[Window, Str] :: err :: call

fn attach_and_return(edit session: std.events.AppSession, take value: Window) -> Result[Window, Str]:
    std.events.attach_window :: session, value :: call
    return Result.Ok[Window, Str] :: value :: call

export fn open_in(edit session: std.events.AppSession, read cfg: std.window.WindowConfig) -> Result[Window, Str]:
    return match (std.window.open_cfg :: cfg :: call):
        Result.Ok(value) => std.window.attach_and_return :: session, value :: call
        Result.Err(err) => Result.Err[Window, Str] :: err :: call

export fn alive(read win: Window) -> Bool:
    return std.kernel.gfx.canvas_alive :: win :: call

export fn id(read win: Window) -> Int:
    return std.kernel.gfx.window_id :: win :: call

export fn size(read win: Window) -> (Int, Int):
    return std.kernel.gfx.window_size :: win :: call

export fn position(read win: Window) -> (Int, Int):
    return std.kernel.gfx.window_position :: win :: call

export fn title(read win: Window) -> Str:
    return std.kernel.gfx.window_title :: win :: call

export fn visible(read win: Window) -> Bool:
    return std.kernel.gfx.window_visible :: win :: call

export fn decorated(read win: Window) -> Bool:
    return std.kernel.gfx.window_decorated :: win :: call

export fn resizable(read win: Window) -> Bool:
    return std.kernel.gfx.window_resizable :: win :: call

export fn topmost(read win: Window) -> Bool:
    return std.kernel.gfx.window_topmost :: win :: call

export fn cursor_visible(read win: Window) -> Bool:
    return std.kernel.gfx.window_cursor_visible :: win :: call

export fn min_size(read win: Window) -> (Int, Int):
    return std.kernel.gfx.window_min_size :: win :: call

export fn max_size(read win: Window) -> (Int, Int):
    return std.kernel.gfx.window_max_size :: win :: call

export fn scale_factor_milli(read win: Window) -> Int:
    return std.kernel.gfx.window_scale_factor_milli :: win :: call

fn lift_theme(code: Int) -> std.window.WindowTheme:
    if code == 1:
        return std.window.WindowTheme.Light :: :: call
    if code == 2:
        return std.window.WindowTheme.Dark :: :: call
    return std.window.WindowTheme.Unknown :: :: call

export fn theme(read win: Window) -> std.window.WindowTheme:
    return std.window.lift_theme :: (std.kernel.gfx.window_theme_code :: win :: call) :: call

export fn transparent(read win: Window) -> Bool:
    return std.kernel.gfx.window_transparent :: win :: call

export fn theme_override(read win: Window) -> std.window.WindowThemeOverride:
    return std.window.lift_theme_override :: (std.kernel.gfx.window_theme_override_code :: win :: call) :: call

export fn cursor_icon(read win: Window) -> std.window.CursorIcon:
    return std.window.lift_cursor_icon :: (std.kernel.gfx.window_cursor_icon_code :: win :: call) :: call

export fn cursor_grab_mode(read win: Window) -> std.window.CursorGrabMode:
    return std.window.lift_cursor_grab_mode :: (std.kernel.gfx.window_cursor_grab_mode :: win :: call) :: call

export fn cursor_position(read win: Window) -> (Int, Int):
    return std.kernel.gfx.window_cursor_position :: win :: call

export fn text_input_enabled(read win: Window) -> Bool:
    return std.kernel.gfx.window_text_input_enabled :: win :: call

fn monitor_info(index: Int) -> std.window.MonitorInfo:
    let mut info = std.window.MonitorInfo :: index = index, name = (std.kernel.gfx.window_monitor_name :: index :: call), position = (std.kernel.gfx.window_monitor_position :: index :: call) :: call
    info.size = std.kernel.gfx.window_monitor_size :: index :: call
    info.scale_factor_milli = std.kernel.gfx.window_monitor_scale_factor_milli :: index :: call
    info.primary = std.kernel.gfx.window_monitor_is_primary :: index :: call
    return info

export fn current_monitor(read win: Window) -> std.window.MonitorInfo:
    return std.window.monitor_info :: (std.kernel.gfx.window_current_monitor_index :: win :: call) :: call

export fn primary_monitor() -> std.window.MonitorInfo:
    return std.window.monitor_info :: (std.kernel.gfx.window_primary_monitor_index :: :: call) :: call

export fn monitor_count() -> Int:
    return std.kernel.gfx.window_monitor_count :: :: call

export fn monitor(index: Int) -> std.window.MonitorInfo:
    return std.window.monitor_info :: index :: call

export fn resized(read win: Window) -> Bool:
    return std.kernel.gfx.window_resized :: win :: call

export fn fullscreen(read win: Window) -> Bool:
    return std.kernel.gfx.window_fullscreen :: win :: call

export fn minimized(read win: Window) -> Bool:
    return std.kernel.gfx.window_minimized :: win :: call

export fn maximized(read win: Window) -> Bool:
    return std.kernel.gfx.window_maximized :: win :: call

export fn focused(read win: Window) -> Bool:
    return std.kernel.gfx.window_focused :: win :: call

export fn cursor_settings(read win: Window) -> std.window.CursorSettings:
    let grab_mode_code = std.kernel.gfx.window_cursor_grab_mode :: win :: call
    let icon_code = std.kernel.gfx.window_cursor_icon_code :: win :: call
    let mut cursor = std.window.CursorSettings :: visible = (std.kernel.gfx.window_cursor_visible :: win :: call), grab_mode = (std.window.CursorGrabMode.Free :: :: call), icon = (std.window.CursorIcon.Default :: :: call) :: call
    if grab_mode_code == 1:
        cursor.grab_mode = std.window.CursorGrabMode.Confined :: :: call
    if grab_mode_code == 2:
        cursor.grab_mode = std.window.CursorGrabMode.Locked :: :: call
    if icon_code == 1:
        cursor.icon = std.window.CursorIcon.Text :: :: call
    if icon_code == 2:
        cursor.icon = std.window.CursorIcon.Crosshair :: :: call
    if icon_code == 3:
        cursor.icon = std.window.CursorIcon.Hand :: :: call
    if icon_code == 4:
        cursor.icon = std.window.CursorIcon.Move :: :: call
    if icon_code == 5:
        cursor.icon = std.window.CursorIcon.Wait :: :: call
    if icon_code == 6:
        cursor.icon = std.window.CursorIcon.Help :: :: call
    if icon_code == 7:
        cursor.icon = std.window.CursorIcon.NotAllowed :: :: call
    if icon_code == 8:
        cursor.icon = std.window.CursorIcon.ResizeHorizontal :: :: call
    if icon_code == 9:
        cursor.icon = std.window.CursorIcon.ResizeVertical :: :: call
    if icon_code == 10:
        cursor.icon = std.window.CursorIcon.ResizeNwse :: :: call
    if icon_code == 11:
        cursor.icon = std.window.CursorIcon.ResizeNesw :: :: call
    cursor.position = std.kernel.gfx.window_cursor_position :: win :: call
    return cursor

export fn settings(read win: Window) -> std.window.WindowSettings:
    let theme_override_code = std.kernel.gfx.window_theme_override_code :: win :: call
    let cursor_grab_mode_code = std.kernel.gfx.window_cursor_grab_mode :: win :: call
    let cursor_icon_code = std.kernel.gfx.window_cursor_icon_code :: win :: call
    let mut bounds = std.window.WindowBounds :: size = (std.kernel.gfx.window_size :: win :: call), position = (std.kernel.gfx.window_position :: win :: call), visible = (std.kernel.gfx.window_visible :: win :: call) :: call
    bounds.min_size = std.kernel.gfx.window_min_size :: win :: call
    bounds.max_size = std.kernel.gfx.window_max_size :: win :: call
    let style = std.window.WindowStyle :: resizable = (std.kernel.gfx.window_resizable :: win :: call), decorated = (std.kernel.gfx.window_decorated :: win :: call), transparent = (std.kernel.gfx.window_transparent :: win :: call) :: call
    let mut cursor = std.window.CursorSettings :: visible = (std.kernel.gfx.window_cursor_visible :: win :: call), grab_mode = (std.window.CursorGrabMode.Free :: :: call), icon = (std.window.CursorIcon.Default :: :: call) :: call
    if cursor_grab_mode_code == 1:
        cursor.grab_mode = std.window.CursorGrabMode.Confined :: :: call
    if cursor_grab_mode_code == 2:
        cursor.grab_mode = std.window.CursorGrabMode.Locked :: :: call
    if cursor_icon_code == 1:
        cursor.icon = std.window.CursorIcon.Text :: :: call
    if cursor_icon_code == 2:
        cursor.icon = std.window.CursorIcon.Crosshair :: :: call
    if cursor_icon_code == 3:
        cursor.icon = std.window.CursorIcon.Hand :: :: call
    if cursor_icon_code == 4:
        cursor.icon = std.window.CursorIcon.Move :: :: call
    if cursor_icon_code == 5:
        cursor.icon = std.window.CursorIcon.Wait :: :: call
    if cursor_icon_code == 6:
        cursor.icon = std.window.CursorIcon.Help :: :: call
    if cursor_icon_code == 7:
        cursor.icon = std.window.CursorIcon.NotAllowed :: :: call
    if cursor_icon_code == 8:
        cursor.icon = std.window.CursorIcon.ResizeHorizontal :: :: call
    if cursor_icon_code == 9:
        cursor.icon = std.window.CursorIcon.ResizeVertical :: :: call
    if cursor_icon_code == 10:
        cursor.icon = std.window.CursorIcon.ResizeNwse :: :: call
    if cursor_icon_code == 11:
        cursor.icon = std.window.CursorIcon.ResizeNesw :: :: call
    cursor.position = std.kernel.gfx.window_cursor_position :: win :: call
    let mut state = std.window.WindowState :: topmost = (std.kernel.gfx.window_topmost :: win :: call), maximized = (std.kernel.gfx.window_maximized :: win :: call), fullscreen = (std.kernel.gfx.window_fullscreen :: win :: call) :: call
    if theme_override_code == 1:
        state.theme_override = std.window.WindowThemeOverride.Light :: :: call
    if theme_override_code == 2:
        state.theme_override = std.window.WindowThemeOverride.Dark :: :: call
    if theme_override_code != 1 and theme_override_code != 2:
        state.theme_override = std.window.WindowThemeOverride.System :: :: call
    let mut options = std.window.WindowOptions :: style = style, state = state, cursor = cursor :: call
    options.text_input_enabled = std.kernel.gfx.window_text_input_enabled :: win :: call
    return std.window.WindowSettings :: title = (std.kernel.gfx.window_title :: win :: call), bounds = bounds, options = options :: call

export fn apply_settings(edit win: Window, read settings: std.window.WindowSettings):
    let current = std.window.settings :: win :: call
    if current.options.state.fullscreen and not settings.options.state.fullscreen:
        std.window.set_fullscreen :: win, false :: call
    if current.options.state.maximized and not settings.options.state.maximized:
        std.window.set_maximized :: win, false :: call
    if current.bounds.min_size != settings.bounds.min_size:
        std.window.set_min_size :: win, settings.bounds.min_size.0, settings.bounds.min_size.1 :: call
    if current.bounds.max_size != settings.bounds.max_size:
        std.window.set_max_size :: win, settings.bounds.max_size.0, settings.bounds.max_size.1 :: call
    if current.bounds.size != settings.bounds.size:
        std.window.set_size :: win, settings.bounds.size.0, settings.bounds.size.1 :: call
    if current.bounds.position != settings.bounds.position:
        std.window.set_position :: win, settings.bounds.position.0, settings.bounds.position.1 :: call
    if current.title != settings.title:
        std.window.set_title :: win, settings.title :: call
    if current.options.style.resizable != settings.options.style.resizable:
        std.window.set_resizable :: win, settings.options.style.resizable :: call
    if current.options.style.decorated != settings.options.style.decorated:
        std.window.set_decorated :: win, settings.options.style.decorated :: call
    if current.options.style.transparent != settings.options.style.transparent:
        std.window.set_transparent :: win, settings.options.style.transparent :: call
    if current.options.state.topmost != settings.options.state.topmost:
        std.window.set_topmost :: win, settings.options.state.topmost :: call
    if current.options.state.theme_override != settings.options.state.theme_override:
        std.window.set_theme_override :: win, settings.options.state.theme_override :: call
    if current.options.cursor.visible != settings.options.cursor.visible:
        std.window.set_cursor_visible :: win, settings.options.cursor.visible :: call
    if current.options.cursor.icon != settings.options.cursor.icon:
        std.window.set_cursor_icon :: win, settings.options.cursor.icon :: call
    if current.options.cursor.grab_mode != settings.options.cursor.grab_mode:
        std.window.set_cursor_grab_mode :: win, settings.options.cursor.grab_mode :: call
    if settings.options.cursor.position.0 >= 0 and settings.options.cursor.position.1 >= 0:
        if current.options.cursor.position != settings.options.cursor.position:
            std.window.set_cursor_position :: win, settings.options.cursor.position.0, settings.options.cursor.position.1 :: call
    if current.options.text_input_enabled != settings.options.text_input_enabled:
        std.window.set_text_input_enabled :: win, settings.options.text_input_enabled :: call
    if current.options.state.maximized != settings.options.state.maximized and settings.options.state.maximized:
        std.window.set_maximized :: win, true :: call
    if current.options.state.fullscreen != settings.options.state.fullscreen and settings.options.state.fullscreen:
        std.window.set_fullscreen :: win, true :: call
    if current.bounds.visible != settings.bounds.visible:
        std.window.set_visible :: win, settings.bounds.visible :: call

export fn apply_cursor_settings(edit win: Window, read settings: std.window.CursorSettings):
    let current = std.window.cursor_settings :: win :: call
    if current.visible != settings.visible:
        std.window.set_cursor_visible :: win, settings.visible :: call
    if current.icon != settings.icon:
        std.window.set_cursor_icon :: win, settings.icon :: call
    if current.grab_mode != settings.grab_mode:
        std.window.set_cursor_grab_mode :: win, settings.grab_mode :: call
    if settings.position.0 >= 0 and settings.position.1 >= 0:
        if current.position != settings.position:
            std.window.set_cursor_position :: win, settings.position.0, settings.position.1 :: call

export fn set_title(edit win: Window, title: Str):
    std.kernel.gfx.window_set_title :: win, title :: call

export fn set_position(edit win: Window, x: Int, y: Int):
    std.kernel.gfx.window_set_position :: win, x, y :: call

export fn set_size(edit win: Window, width: Int, height: Int):
    std.kernel.gfx.window_set_size :: win, width, height :: call

export fn set_visible(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_visible :: win, enabled :: call

export fn set_decorated(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_decorated :: win, enabled :: call

export fn set_resizable(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_resizable :: win, enabled :: call

export fn set_min_size(edit win: Window, width: Int, height: Int):
    std.kernel.gfx.window_set_min_size :: win, width, height :: call

export fn set_max_size(edit win: Window, width: Int, height: Int):
    std.kernel.gfx.window_set_max_size :: win, width, height :: call

export fn set_fullscreen(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_fullscreen :: win, enabled :: call

export fn set_minimized(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_minimized :: win, enabled :: call

export fn set_maximized(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_maximized :: win, enabled :: call

export fn set_topmost(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_topmost :: win, enabled :: call

export fn set_cursor_visible(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_cursor_visible :: win, enabled :: call

export fn set_transparent(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_transparent :: win, enabled :: call

export fn set_theme_override(edit win: Window, read value: std.window.WindowThemeOverride):
    std.kernel.gfx.window_set_theme_override_code :: win, (std.window.theme_override_code :: value :: call) :: call

export fn set_cursor_icon(edit win: Window, read icon: std.window.CursorIcon):
    std.kernel.gfx.window_set_cursor_icon_code :: win, (std.window.cursor_icon_code :: icon :: call) :: call

export fn set_cursor_grab_mode(edit win: Window, read mode: std.window.CursorGrabMode):
    std.kernel.gfx.window_set_cursor_grab_mode :: win, (std.window.cursor_grab_mode_code :: mode :: call) :: call

export fn set_cursor_position(edit win: Window, x: Int, y: Int):
    std.kernel.gfx.window_set_cursor_position :: win, x, y :: call

export fn set_text_input_enabled(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_set_text_input_enabled :: win, enabled :: call

export fn request_redraw(edit win: Window):
    std.kernel.gfx.window_request_redraw :: win :: call

export fn request_attention(edit win: Window, enabled: Bool):
    std.kernel.gfx.window_request_attention :: win, enabled :: call

export fn close(take win: Window) -> Result[Unit, Str]:
    return std.kernel.gfx.window_close :: win :: call
