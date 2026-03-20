import std.events
import std.option
use std.window.Window

export record WindowId:
    value: Int

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
    grab_mode: arcana_desktop.types.CursorGrabMode
    icon: arcana_desktop.types.CursorIcon
    position: (Int, Int)

export record WindowState:
    topmost: Bool
    maximized: Bool
    fullscreen: Bool
    theme_override: arcana_desktop.types.WindowThemeOverride

export record WindowOptions:
    style: arcana_desktop.types.WindowStyle
    state: arcana_desktop.types.WindowState
    cursor: arcana_desktop.types.CursorSettings
    text_input_enabled: Bool

export record WindowConfig:
    title: Str
    bounds: arcana_desktop.types.WindowBounds
    options: arcana_desktop.types.WindowOptions

export record WindowSettings:
    title: Str
    bounds: arcana_desktop.types.WindowBounds
    options: arcana_desktop.types.WindowOptions

export record CompositionArea:
    active: Bool
    position: (Int, Int)
    size: (Int, Int)

export record TextInputSettings:
    enabled: Bool
    composition_area: arcana_desktop.types.CompositionArea

export record AppLoop:
    wait_poll_ms: Int

export record AppConfig:
    window: arcana_desktop.types.WindowConfig
    loop: arcana_desktop.types.AppLoop

export enum ControlFlow:
    Poll
    Wait
    WaitUntil(Int)

export record WakeHandle:
    raw: std.events.WakeHandle

export record InputSnapshot:
    mouse_pos: (Int, Int)
    mouse_in_window: Bool
    mouse_wheel_y: Int

export record TargetedEvent:
    window_id: arcana_desktop.types.WindowId
    is_main_window: Bool
    event: std.events.AppEvent

export record RuntimeContext:
    session: std.events.AppSession
    wake: arcana_desktop.types.WakeHandle
    main_window: Window

export record RunControl:
    exit_requested: Bool
    exit_code: Int
    control_flow: arcana_desktop.types.ControlFlow

export record AppContext:
    runtime: arcana_desktop.types.RuntimeContext
    control: arcana_desktop.types.RunControl
    current_window_id: std.option.Option[arcana_desktop.types.WindowId]
    current_is_main_window: Bool

export record FixedStepConfig:
    tick_hz: Int
    max_steps: Int

export record FixedRunner:
    tick_ms: Int
    accumulator_ms: Int
