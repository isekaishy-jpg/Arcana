import arcana_desktop.events
import arcana_desktop.types
import arcana_desktop.window
import std.collections.list
import std.concurrent
import std.option
import std.result
import std.text
import std.time
use std.events.AppEvent
use std.events.AppFrame
use std.option.Option
use std.result.Result
use std.window.Window

export record Mailbox[T]:
    queue: Mutex[List[T]]
    wake: arcana_desktop.types.WakeHandle

record LaunchConfig:
    cfg: arcana_desktop.types.AppConfig
    wake: arcana_desktop.types.WakeHandle

record RunWindow:
    cfg: arcana_desktop.types.AppConfig
    runtime: arcana_desktop.types.RuntimeContext

record TargetDispatch:
    window_id: arcana_desktop.types.WindowId
    event: AppEvent

export trait Application[A]:
    fn resumed(edit self: A, edit cx: arcana_desktop.types.AppContext)
    fn suspended(edit self: A, edit cx: arcana_desktop.types.AppContext)
    fn window_event(edit self: A, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow
    fn device_event(edit self: A, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow
    fn about_to_wait(edit self: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow
    fn wake(edit self: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow
    fn exiting(edit self: A, edit cx: arcana_desktop.types.AppContext)

export fn default_app_config() -> arcana_desktop.types.AppConfig:
    let wait_loop = arcana_desktop.types.AppLoop :: wait_poll_ms = 0 :: call
    return arcana_desktop.types.AppConfig :: window = (arcana_desktop.window.default_config :: :: call), loop = wait_loop :: call

export fn mailbox[T](read wake: arcana_desktop.types.WakeHandle) -> arcana_desktop.app.Mailbox[T]:
    return arcana_desktop.app.Mailbox[T] :: queue = (std.concurrent.mutex[List[T]] :: (std.collections.list.new[T] :: :: call) :: call), wake = wake :: call

impl[T] Mailbox[T]:
    fn post(read self: arcana_desktop.app.Mailbox[T], take value: T):
        let mut queue = self.queue :: :: pull
        queue :: value :: push
        self.queue :: queue :: put
        arcana_desktop.events.wake :: self.wake :: call

    fn take_all(read self: arcana_desktop.app.Mailbox[T]) -> List[T]:
        let mut queue = self.queue :: :: pull
        let mut out = std.collections.list.new[T] :: :: call
        while not (queue :: :: is_empty):
            out :: (queue :: :: pop) :: push
        self.queue :: queue :: put
        return out

export fn request_exit(edit cx: arcana_desktop.types.AppContext, code: Int):
    cx.control.exit_requested = true
    cx.control.exit_code = code

export fn set_control_flow(edit cx: arcana_desktop.types.AppContext, flow: arcana_desktop.types.ControlFlow):
    cx.control.control_flow = flow

fn sleep_until(deadline: Int):
    let now = std.time.monotonic_now_ms :: :: call
    let delta = deadline - now.value
    if delta <= 0:
        return 0
    return delta

fn wait_slice(read app_loop: arcana_desktop.types.AppLoop) -> Int:
    if app_loop.wait_poll_ms <= 0:
        return -1
    return app_loop.wait_poll_ms

fn bounded_wait(deadline: Int, read app_loop: arcana_desktop.types.AppLoop) -> Int:
    let remaining = arcana_desktop.app.sleep_until :: deadline :: call
    if remaining <= 0:
        return 0
    let slice = arcana_desktop.app.wait_slice :: app_loop :: call
    if slice < 0:
        return remaining
    if remaining < slice:
        return remaining
    return slice

fn wait_timeout_for_flow(read flow: arcana_desktop.types.ControlFlow, read app_loop: arcana_desktop.types.AppLoop) -> Int:
    return match flow:
        arcana_desktop.types.ControlFlow.Poll => -2
        arcana_desktop.types.ControlFlow.Wait => arcana_desktop.app.wait_slice :: app_loop :: call
        arcana_desktop.types.ControlFlow.WaitUntil(deadline) => arcana_desktop.app.bounded_wait :: deadline, app_loop :: call

fn dispatch_resumed[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    arcana_desktop.app.run_resumed :: app, cx :: call
    return cx.control.control_flow

fn dispatch_suspended[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    arcana_desktop.app.run_suspended :: app, cx :: call
    return cx.control.control_flow

export fn run_resumed[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext):
    app :: cx :: resumed

export fn run_suspended[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext):
    app :: cx :: suspended

export fn run_window_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    return app :: cx, target :: window_event

export fn run_device_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    return app :: cx, target :: device_event

export fn run_about_to_wait[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    return app :: cx :: about_to_wait

export fn run_wake[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    return app :: cx :: wake

export fn run_exiting[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext):
    app :: cx :: exiting

fn dispatch_targeted_window_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read event: AppEvent) -> arcana_desktop.types.ControlFlow:
    return match (arcana_desktop.app.event_window_id :: event :: call):
        Option.Some(window_id) => dispatch_targeted_window_event_seed :: app, cx, (arcana_desktop.app.TargetDispatch :: window_id = window_id, event = event :: call) :: call
        Option.None => cx.control.control_flow

fn dispatch_targeted_device_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read event: AppEvent) -> arcana_desktop.types.ControlFlow:
    return match (arcana_desktop.app.event_window_id :: event :: call):
        Option.Some(window_id) => dispatch_targeted_device_event_seed :: app, cx, (arcana_desktop.app.TargetDispatch :: window_id = window_id, event = event :: call) :: call
        Option.None => cx.control.control_flow

fn targeted_event(read cx: arcana_desktop.types.AppContext, read event: AppEvent, read window_id: arcana_desktop.types.WindowId) -> arcana_desktop.types.TargetedEvent:
    let main_window_id = (arcana_desktop.window.id :: cx.runtime.main_window :: call).value
    return arcana_desktop.types.TargetedEvent :: window_id = window_id, is_main_window = (window_id.value == main_window_id), event = event :: call

fn dispatch_targeted_window_event_seed[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read dispatch: arcana_desktop.app.TargetDispatch) -> arcana_desktop.types.ControlFlow:
    let target = targeted_event :: cx, dispatch.event, dispatch.window_id :: call
    arcana_desktop.app.set_current_target :: cx, target :: call
    let flow = arcana_desktop.app.run_window_event :: app, cx, target :: call
    arcana_desktop.app.clear_current_target :: cx :: call
    return flow

fn dispatch_targeted_device_event_seed[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read dispatch: arcana_desktop.app.TargetDispatch) -> arcana_desktop.types.ControlFlow:
    let target = targeted_event :: cx, dispatch.event, dispatch.window_id :: call
    arcana_desktop.app.set_current_target :: cx, target :: call
    let flow = arcana_desktop.app.run_device_event :: app, cx, target :: call
    arcana_desktop.app.clear_current_target :: cx :: call
    return flow

fn dispatch_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read event: AppEvent) -> arcana_desktop.types.ControlFlow:
    return match event:
        AppEvent.AppResumed => arcana_desktop.app.dispatch_resumed :: app, cx :: call
        AppEvent.AppSuspended => arcana_desktop.app.dispatch_suspended :: app, cx :: call
        AppEvent.Wake => arcana_desktop.app.run_wake :: app, cx :: call
        AppEvent.AboutToWait => arcana_desktop.app.run_about_to_wait :: app, cx :: call
        AppEvent.WindowResized(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.WindowMoved(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.WindowCloseRequested(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.WindowFocused(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.WindowRedrawRequested(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.WindowScaleFactorChanged(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.WindowThemeChanged(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.KeyDown(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.KeyUp(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.MouseDown(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.MouseUp(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.MouseMove(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.MouseWheel(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.MouseEntered(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.MouseLeft(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.TextInput(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.TextCompositionStarted(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.TextCompositionUpdated(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.TextCompositionCommitted(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.TextCompositionCancelled(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.FileDropped(_) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, event :: call
        AppEvent.RawMouseMotion(_) => arcana_desktop.app.dispatch_targeted_device_event :: app, cx, event :: call

fn attach_window(edit session: std.events.AppSession, read launch: arcana_desktop.app.LaunchConfig, take value: Window) -> arcana_desktop.app.RunWindow:
    let mut value = value
    arcana_desktop.events.attach_window :: session, value :: call
    if launch.cfg.window.bounds.visible:
        arcana_desktop.window.request_redraw :: value :: call
    let runtime = arcana_desktop.types.RuntimeContext :: session = session, wake = launch.wake, main_window = value :: call
    return arcana_desktop.app.RunWindow :: cfg = launch.cfg, runtime = runtime :: call

export fn wake_handle(read cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.WakeHandle:
    return cx.runtime.wake

export fn window_for_id(read cx: arcana_desktop.types.AppContext, read id: arcana_desktop.types.WindowId) -> Option[Window]:
    return arcana_desktop.events.window_for_id :: cx.runtime.session, id.value :: call

export fn event_window_id(read event: AppEvent) -> Option[arcana_desktop.types.WindowId]:
    return match (arcana_desktop.events.window_id :: event :: call):
        Option.Some(value) => Option.Some[arcana_desktop.types.WindowId] :: (arcana_desktop.types.WindowId :: value = value :: call) :: call
        Option.None => Option.None[arcana_desktop.types.WindowId] :: :: call

export fn main_window_id(read cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.WindowId:
    return arcana_desktop.window.id :: cx.runtime.main_window :: call

export fn event_window(read cx: arcana_desktop.types.AppContext, read event: AppEvent) -> Option[Window]:
    return arcana_desktop.events.event_window :: cx.runtime.session, event :: call

export fn event_targets_window(read event: AppEvent, read win: Window) -> Bool:
    return match (arcana_desktop.app.event_window_id :: event :: call):
        Option.Some(id) => id.value == (arcana_desktop.window.id :: win :: call).value
        Option.None => false

export fn event_targets_main_window(read cx: arcana_desktop.types.AppContext, read event: AppEvent) -> Bool:
    return arcana_desktop.app.event_targets_window :: event, cx.runtime.main_window :: call

export fn is_main_window(read cx: arcana_desktop.types.AppContext, read win: Window) -> Bool:
    return (arcana_desktop.window.id :: win :: call).value == (arcana_desktop.window.id :: cx.runtime.main_window :: call).value

fn clear_current_target(edit cx: arcana_desktop.types.AppContext):
    cx.current_window_id = Option.None[arcana_desktop.types.WindowId] :: :: call
    cx.current_is_main_window = false

fn set_current_target(edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent):
    cx.current_window_id = Option.Some[arcana_desktop.types.WindowId] :: target.window_id :: call
    cx.current_is_main_window = target.is_main_window

export fn target_window(read cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> Option[Window]:
    return arcana_desktop.app.window_for_id :: cx, target.window_id :: call

export fn require_target_window(read cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> Result[Window, Str]:
    return match (arcana_desktop.app.target_window :: cx, target :: call):
        Option.Some(win) => Result.Ok[Window, Str] :: win :: call
        Option.None => Result.Err[Window, Str] :: ("missing live window for target id " + (std.text.from_int :: target.window_id.value :: call)) :: call

export fn target_is_main_window(read target: arcana_desktop.types.TargetedEvent) -> Bool:
    return target.is_main_window

export fn current_window_id(read cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.WindowId]:
    return cx.current_window_id

export fn current_window(read cx: arcana_desktop.types.AppContext) -> Option[Window]:
    return match cx.current_window_id:
        Option.Some(id) => arcana_desktop.app.window_for_id :: cx, id :: call
        Option.None => Option.None[Window] :: :: call

export fn require_current_window(read cx: arcana_desktop.types.AppContext) -> Result[Window, Str]:
    return match (arcana_desktop.app.current_window :: cx :: call):
        Option.Some(win) => Result.Ok[Window, Str] :: win :: call
        Option.None => Result.Err[Window, Str] :: "missing current event window" :: call

export fn current_is_main_window(read cx: arcana_desktop.types.AppContext) -> Bool:
    return cx.current_is_main_window

export fn target_window_id(read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.WindowId:
    return target.window_id

export fn close_target_window_or_exit_main(edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> Result[arcana_desktop.types.ControlFlow, Str]:
    if target.is_main_window:
        arcana_desktop.app.request_exit :: cx, 0 :: call
        return Result.Ok[arcana_desktop.types.ControlFlow, Str] :: (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call
    return match (arcana_desktop.app.target_window :: cx, target :: call):
        Option.Some(win) => close_current_window_or_exit_main_window :: win :: call
        Option.None => Result.Err[arcana_desktop.types.ControlFlow, Str] :: ("missing live window for target id " + (std.text.from_int :: target.window_id.value :: call)) :: call

export fn close_current_window_or_exit_main(edit cx: arcana_desktop.types.AppContext) -> Result[arcana_desktop.types.ControlFlow, Str]:
    if cx.current_is_main_window:
        arcana_desktop.app.request_exit :: cx, 0 :: call
        return Result.Ok[arcana_desktop.types.ControlFlow, Str] :: (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call
    return match (arcana_desktop.app.current_window :: cx :: call):
        Option.Some(win) => close_current_window_or_exit_main_window :: win :: call
        Option.None => Result.Err[arcana_desktop.types.ControlFlow, Str] :: "missing current event window" :: call

fn close_current_window_or_exit_main_window(read win: Window) -> Result[arcana_desktop.types.ControlFlow, Str]:
    let mut win = win
    return match (arcana_desktop.window.close :: win :: call):
        Result.Ok(_) => Result.Ok[arcana_desktop.types.ControlFlow, Str] :: (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call
        Result.Err(err) => Result.Err[arcana_desktop.types.ControlFlow, Str] :: err :: call

export fn open_window(edit cx: arcana_desktop.types.AppContext, title: Str, size: (Int, Int)) -> Result[Window, Str]:
    let mut cfg = arcana_desktop.window.default_config :: :: call
    let mut bounds = arcana_desktop.types.WindowBounds :: size = size, position = cfg.bounds.position, visible = cfg.bounds.visible :: call
    bounds.min_size = cfg.bounds.min_size
    bounds.max_size = cfg.bounds.max_size
    cfg.title = title
    cfg.bounds = bounds
    return arcana_desktop.app.open_window_cfg :: cx, cfg :: call

export fn open_window_cfg(edit cx: arcana_desktop.types.AppContext, read cfg: arcana_desktop.types.WindowConfig) -> Result[Window, Str]:
    let opened = arcana_desktop.window.open_cfg :: cfg :: call
    return match opened:
        std.result.Result.Ok(win) => open_window_cfg_attached :: cx.runtime.session, cfg, win :: call
        std.result.Result.Err(err) => std.result.Result.Err[Window, Str] :: err :: call

fn open_window_cfg_attached(edit session: std.events.AppSession, read cfg: arcana_desktop.types.WindowConfig, take win: Window) -> Result[Window, Str]:
    let mut win = win
    arcana_desktop.events.attach_window :: session, win :: call
    return open_window_cfg_ready :: cfg, win :: call

fn open_window_cfg_ready(read cfg: arcana_desktop.types.WindowConfig, take win: Window) -> Result[Window, Str]:
    let mut win = win
    if cfg.bounds.visible:
        arcana_desktop.window.request_redraw :: win :: call
    return std.result.Result.Ok[Window, Str] :: win :: call

fn run_with_window[A, where arcana_desktop.app.Application[A]](edit app: A, take run: arcana_desktop.app.RunWindow) -> Int:
    let control = arcana_desktop.types.RunControl :: exit_requested = false, exit_code = 0, control_flow = (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call
    let mut cx = arcana_desktop.types.AppContext :: runtime = run.runtime, control = control :: call
    cx.current_window_id = Option.None[arcana_desktop.types.WindowId] :: :: call
    cx.current_is_main_window = false
    while not cx.control.exit_requested:
        let timeout = arcana_desktop.app.wait_timeout_for_flow :: cx.control.control_flow, run.cfg.loop :: call
        let mut frame = match timeout:
            -2 => arcana_desktop.events.pump_session :: cx.runtime.session :: call
            _ => arcana_desktop.events.wait_session :: cx.runtime.session, timeout :: call
        while true:
            let next = arcana_desktop.events.poll :: frame :: call
            if next :: :: is_none:
                break
            let flow = arcana_desktop.app.dispatch_event :: app, cx, (next :: (AppEvent.AboutToWait :: :: call) :: unwrap_or) :: call
            cx.control.control_flow = flow
            if cx.control.exit_requested:
                break
        if cx.control.exit_requested:
            break
    arcana_desktop.app.run_exiting :: app, cx :: call
    arcana_desktop.events.close_session :: cx.runtime.session :: call
    return cx.control.exit_code

fn run_open_failed(edit session: std.events.AppSession) -> Int:
    arcana_desktop.events.close_session :: session :: call
    return 1

export fn run[A, where arcana_desktop.app.Application[A]](edit app: A, read cfg: arcana_desktop.types.AppConfig) -> Int:
    let mut session = arcana_desktop.events.open_session :: :: call
    let wake = arcana_desktop.events.create_wake :: session :: call
    let launch = arcana_desktop.app.LaunchConfig :: cfg = cfg, wake = wake :: call
    return match (arcana_desktop.window.open_cfg :: cfg.window :: call):
        std.result.Result.Ok(value) => arcana_desktop.app.run_with_window :: app, (arcana_desktop.app.attach_window :: session, launch, value :: call) :: call
        std.result.Result.Err(_) => arcana_desktop.app.run_open_failed :: session :: call
