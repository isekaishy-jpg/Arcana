import std.option

export opaque type Window as move, boundary_unsafe
export opaque type FrameInput as move, boundary_unsafe
export opaque type Session as move, boundary_unsafe
export opaque type WakeHandle as copy, boundary_unsafe

lang window_handle = Window
lang app_frame_handle = FrameInput
lang app_session_handle = Session
lang wake_handle = WakeHandle

export record WindowId:
    value: Int

export record DeviceId:
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

export record FixedStepConfig:
    tick_hz: Int
    max_steps: Int

export record FixedRunner:
    tick_ms: Int
    accumulator_ms: Int

export enum ControlFlow:
    Poll
    Wait
    WaitUntil(Int)

export record InputSnapshot:
    mouse_pos: (Int, Int)
    mouse_in_window: Bool
    mouse_wheel_y: Int

export record WindowResizeEvent:
    window_id: Int
    size: (Int, Int)

export record WindowMoveEvent:
    window_id: Int
    position: (Int, Int)

export record WindowFocusEvent:
    window_id: Int
    focused: Bool

export record WindowScaleFactorEvent:
    window_id: Int
    scale_factor_milli: Int

export record WindowThemeEvent:
    window_id: Int
    theme_code: Int

export record KeyMeta:
    modifiers: Int
    repeated: Bool
    physical_key: Int
    logical_key: Int
    location: Int
    text: Str

export record KeyEvent:
    window_id: Int
    key: Int
    meta: arcana_desktop.types.KeyMeta

export record MouseButtonEvent:
    window_id: Int
    button: Int
    position: (Int, Int)
    modifiers: Int

export record MouseMoveEvent:
    window_id: Int
    position: (Int, Int)
    modifiers: Int

export record MouseWheelEvent:
    window_id: Int
    delta: (Int, Int)
    modifiers: Int

export record TextInputEvent:
    window_id: Int
    text: Str

export record TextCompositionEvent:
    window_id: Int
    text: Str
    caret: Int

export record FileDropEvent:
    window_id: Int
    path: Str

export enum DeviceEvents:
    Never
    WhenFocused
    Always

export record RawMouseMotionEvent:
    device_id: std.option.Option[arcana_desktop.types.DeviceId]
    delta: (Int, Int)

export record RawMouseButtonEvent:
    device_id: std.option.Option[arcana_desktop.types.DeviceId]
    button: Int
    pressed: Bool

export record RawMouseWheelEvent:
    device_id: std.option.Option[arcana_desktop.types.DeviceId]
    delta: (Int, Int)

export record RawKeyEvent:
    device_id: std.option.Option[arcana_desktop.types.DeviceId]
    key: Int
    meta: arcana_desktop.types.KeyMeta
    pressed: Bool

export enum WindowEvent:
    WindowResized(arcana_desktop.types.WindowResizeEvent)
    WindowMoved(arcana_desktop.types.WindowMoveEvent)
    WindowCloseRequested(Int)
    WindowFocused(arcana_desktop.types.WindowFocusEvent)
    WindowRedrawRequested(Int)
    WindowScaleFactorChanged(arcana_desktop.types.WindowScaleFactorEvent)
    WindowThemeChanged(arcana_desktop.types.WindowThemeEvent)
    KeyDown(arcana_desktop.types.KeyEvent)
    KeyUp(arcana_desktop.types.KeyEvent)
    MouseDown(arcana_desktop.types.MouseButtonEvent)
    MouseUp(arcana_desktop.types.MouseButtonEvent)
    MouseMove(arcana_desktop.types.MouseMoveEvent)
    MouseWheel(arcana_desktop.types.MouseWheelEvent)
    MouseEntered(Int)
    MouseLeft(Int)
    TextInput(arcana_desktop.types.TextInputEvent)
    TextCompositionStarted(Int)
    TextCompositionUpdated(arcana_desktop.types.TextCompositionEvent)
    TextCompositionCommitted(arcana_desktop.types.TextCompositionEvent)
    TextCompositionCancelled(Int)
    FileDropped(arcana_desktop.types.FileDropEvent)

export enum DeviceEvent:
    RawMouseMotion(arcana_desktop.types.RawMouseMotionEvent)
    RawMouseButton(arcana_desktop.types.RawMouseButtonEvent)
    RawMouseWheel(arcana_desktop.types.RawMouseWheelEvent)
    RawKey(arcana_desktop.types.RawKeyEvent)

export record WindowDispatchEvent:
    window_id: arcana_desktop.types.WindowId
    event: arcana_desktop.types.WindowEvent

export enum AppEvent:
    AppResumed
    Wake
    AppSuspended
    AboutToWait
    Unknown(Int)
    Window(arcana_desktop.types.WindowDispatchEvent)
    Device(arcana_desktop.types.DeviceEvent)

export record TargetedEvent:
    window_id: arcana_desktop.types.WindowId
    is_main_window: Bool
    event: arcana_desktop.types.WindowEvent

export record RuntimeContext:
    session: arcana_desktop.types.Session
    wake: arcana_desktop.types.WakeHandle
    main_window_id: arcana_desktop.types.WindowId
    main_window: arcana_desktop.types.Window

export record RunControl:
    exit_requested: Bool
    exit_code: Int
    control_flow: arcana_desktop.types.ControlFlow

export record AppContext:
    runtime: arcana_desktop.types.RuntimeContext
    control: arcana_desktop.types.RunControl
    current_window_id: std.option.Option[arcana_desktop.types.WindowId]
    current_is_main_window: Bool
