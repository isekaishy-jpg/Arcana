import arcana_desktop.events
import arcana_desktop.types
import arcana_desktop.window
import std.collections.list
import std.concurrent
import std.option
import std.result
import std.time
use std.option.Option
use std.result.Result

export record Mailbox[T]:
    queue: Mutex[List[T]]
    wake: arcana_desktop.types.WakeHandle

record RunWindow:
    cfg: arcana_desktop.types.AppConfig
    runtime: arcana_desktop.types.RuntimeContext

record LaunchConfig:
    cfg: arcana_desktop.types.AppConfig
    session: arcana_desktop.types.Session
    wake: arcana_desktop.types.WakeHandle

obj DesktopAppState:
    cx: arcana_desktop.types.AppContext

obj DesktopLifecycle:
    finished: Bool
    fn init(edit self: Self):
        self.finished = false
    fn resume(edit self: Self):
        self.finished = false

obj DesktopPendingActions:
    close_ids: List[arcana_desktop.types.WindowId]
    redraw_ids: List[arcana_desktop.types.WindowId]
    sync_main_window: Bool
    fn init(edit self: Self):
        self.close_ids = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
        self.redraw_ids = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
        self.sync_main_window = false
    fn resume(edit self: Self):
        self.close_ids = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
        self.redraw_ids = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
        self.sync_main_window = false

create DesktopOwner [DesktopAppState, DesktopLifecycle, DesktopPendingActions] scope-exit:
    done: when DesktopLifecycle.finished

export trait Application[A]:
    fn resumed(edit self: A, edit cx: arcana_desktop.types.AppContext)
    fn suspended(edit self: A, edit cx: arcana_desktop.types.AppContext)
    fn window_event(edit self: A, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow
    fn device_event(edit self: A, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow
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
        let mut reversed = std.collections.list.new[T] :: :: call
        while not (queue :: :: is_empty):
            reversed :: (queue :: :: pop) :: push
        let mut out = std.collections.list.new[T] :: :: call
        while not (reversed :: :: is_empty):
            out :: (reversed :: :: pop) :: push
        self.queue :: queue :: put
        return out

export fn request_exit(edit cx: arcana_desktop.types.AppContext, code: Int):
    cx.control.exit_requested = true
    cx.control.exit_code = code

export fn set_control_flow(edit cx: arcana_desktop.types.AppContext, flow: arcana_desktop.types.ControlFlow):
    cx.control.control_flow = flow

fn contains_window_id(read ids: List[arcana_desktop.types.WindowId], read id: arcana_desktop.types.WindowId) -> Bool:
    for candidate in ids:
        if candidate.value == id.value:
            return true
    return false

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

export fn run_device_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.DeviceEvent) -> arcana_desktop.types.ControlFlow:
    return app :: cx, event :: device_event

export fn run_about_to_wait[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    return app :: cx :: about_to_wait

export fn run_wake[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.ControlFlow:
    return app :: cx :: wake

export fn run_exiting[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext):
    app :: cx :: exiting

fn targeted_event(read cx: arcana_desktop.types.AppContext, read dispatch: arcana_desktop.types.WindowDispatchEvent) -> arcana_desktop.types.TargetedEvent:
    return arcana_desktop.app.targeted_event_value :: dispatch, (dispatch.window_id.value == cx.runtime.main_window_id.value) :: call

fn targeted_event_value(read dispatch: arcana_desktop.types.WindowDispatchEvent, is_main_window: Bool) -> arcana_desktop.types.TargetedEvent:
    let mut target = arcana_desktop.types.TargetedEvent :: window_id = dispatch.window_id, is_main_window = is_main_window :: call
    target.event = dispatch.event
    return target

fn dispatch_targeted_window_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read dispatch: arcana_desktop.types.WindowDispatchEvent) -> arcana_desktop.types.ControlFlow:
    let target = arcana_desktop.app.targeted_event :: cx, dispatch :: call
    return dispatch_targeted_window_event_ready :: app, cx, target :: call

fn dispatch_targeted_window_event_ready[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.ControlFlow:
    arcana_desktop.app.set_current_target :: cx, target.window_id, target.is_main_window :: call
    let flow = arcana_desktop.app.run_window_event :: app, cx, target :: call
    arcana_desktop.app.clear_current_target :: cx :: call
    return flow

fn dispatch_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.AppEvent) -> arcana_desktop.types.ControlFlow:
    return match event:
        arcana_desktop.types.AppEvent.AppResumed => arcana_desktop.app.dispatch_resumed :: app, cx :: call
        arcana_desktop.types.AppEvent.AppSuspended => arcana_desktop.app.dispatch_suspended :: app, cx :: call
        arcana_desktop.types.AppEvent.Wake => arcana_desktop.app.run_wake :: app, cx :: call
        arcana_desktop.types.AppEvent.AboutToWait => arcana_desktop.app.run_about_to_wait :: app, cx :: call
        arcana_desktop.types.AppEvent.Unknown(_) => cx.control.control_flow
        arcana_desktop.types.AppEvent.Window(dispatch) => arcana_desktop.app.dispatch_targeted_window_event :: app, cx, dispatch :: call
        arcana_desktop.types.AppEvent.Device(value) => arcana_desktop.app.run_device_event :: app, cx, value :: call

export fn wake_handle(read cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.WakeHandle:
    return cx.runtime.wake

export fn device_events(edit cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.DeviceEvents:
    return arcana_desktop.events.device_events :: cx.runtime.session :: call

export fn set_device_events(edit cx: arcana_desktop.types.AppContext, read value: arcana_desktop.types.DeviceEvents):
    arcana_desktop.events.set_device_events :: cx.runtime.session, value :: call

export fn window_for_id(read cx: arcana_desktop.types.AppContext, read id: arcana_desktop.types.WindowId) -> Option[arcana_desktop.types.Window]:
    return arcana_desktop.events.window_for_id :: cx.runtime.session, id :: call

export fn event_window_id(read event: arcana_desktop.types.AppEvent) -> Option[arcana_desktop.types.WindowId]:
    return arcana_desktop.events.window_id :: event :: call

export fn main_window_id(read cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.WindowId:
    return cx.runtime.main_window_id

export fn main_window(read cx: arcana_desktop.types.AppContext) -> Result[arcana_desktop.types.Window, Str]:
    return match (arcana_desktop.app.best_main_window :: cx :: call):
        Option.Some(win) => Result.Ok[arcana_desktop.types.Window, Str] :: win :: call
        Option.None => Result.Err[arcana_desktop.types.Window, Str] :: "missing main window" :: call

export fn cached_main_window(read cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.Window:
    return cx.runtime.main_window

export fn main_window_or_cached(read cx: arcana_desktop.types.AppContext) -> arcana_desktop.types.Window:
    return (arcana_desktop.app.main_window :: cx :: call) :: (arcana_desktop.app.cached_main_window :: cx :: call) :: unwrap_or

export fn require_main_window(read cx: arcana_desktop.types.AppContext) -> Result[arcana_desktop.types.Window, Str]:
    return arcana_desktop.app.main_window :: cx :: call

export fn request_window_redraw(edit cx: arcana_desktop.types.AppContext, read win: arcana_desktop.types.Window):
    let _ = cx
    arcana_desktop.app.owner_queue_redraw :: (arcana_desktop.window.id :: win :: call) :: call

export fn request_window_redraw_id(edit cx: arcana_desktop.types.AppContext, read id: arcana_desktop.types.WindowId):
    let _ = cx
    arcana_desktop.app.owner_queue_redraw :: id :: call

export fn request_main_window_redraw(edit cx: arcana_desktop.types.AppContext):
    let main = arcana_desktop.app.main_window :: cx :: call
    return match main:
        Result.Ok(win) => arcana_desktop.app.request_window_redraw :: cx, win :: call
        Result.Err(_) => 0

export fn event_window(read cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.AppEvent) -> Option[arcana_desktop.types.Window]:
    return arcana_desktop.events.event_window :: cx.runtime.session, event :: call

fn target_window_for_id(read cx: arcana_desktop.types.AppContext, read id: arcana_desktop.types.WindowId) -> Option[arcana_desktop.types.Window]:
    if id.value == cx.runtime.main_window_id.value:
        return arcana_desktop.app.cached_main_window_if_live :: cx :: call
    return arcana_desktop.app.window_for_id :: cx, id :: call

fn cached_main_window_if_live(read cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.Window]:
    let ids = arcana_desktop.events.window_ids :: cx.runtime.session :: call
    for id in ids:
        if id.value == cx.runtime.main_window_id.value:
            return Option.Some[arcana_desktop.types.Window] :: cx.runtime.main_window :: call
    return Option.None[arcana_desktop.types.Window] :: :: call

fn live_main_window(read cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.Window]:
    let cached = arcana_desktop.app.cached_main_window_if_live :: cx :: call
    if cached :: :: is_some:
        return cached
    return arcana_desktop.app.window_for_id :: cx, cx.runtime.main_window_id :: call

fn best_main_window(read cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.Window]:
    return arcana_desktop.app.live_main_window :: cx :: call

fn first_live_window(read cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.Window]:
    let ids = arcana_desktop.events.window_ids :: cx.runtime.session :: call
    for id in ids:
        let found = arcana_desktop.app.window_for_id :: cx, id :: call
        if found :: :: is_some:
            return found
    return Option.None[arcana_desktop.types.Window] :: :: call

fn assign_main_window(edit cx: arcana_desktop.types.AppContext, take win: arcana_desktop.types.Window) -> Bool:
    let window_id = arcana_desktop.window.id :: win :: call
    cx.runtime.main_window_id = window_id
    cx.runtime.main_window = win
    return true

fn sync_main_window(edit cx: arcana_desktop.types.AppContext) -> Bool:
    let live_main = arcana_desktop.app.live_main_window :: cx :: call
    return match live_main:
        Option.Some(win) => sync_main_window_live_ready :: cx, win :: call
        Option.None => false

fn sync_main_window_live_ready(edit cx: arcana_desktop.types.AppContext, take win: arcana_desktop.types.Window) -> Bool:
    return arcana_desktop.app.assign_main_window :: cx, win :: call

export fn event_targets_window(read event: arcana_desktop.types.AppEvent, read win: arcana_desktop.types.Window) -> Bool:
    return match (arcana_desktop.app.event_window_id :: event :: call):
        Option.Some(id) => id.value == (arcana_desktop.window.id :: win :: call).value
        Option.None => false

export fn event_targets_main_window(read cx: arcana_desktop.types.AppContext, read event: arcana_desktop.types.AppEvent) -> Bool:
    return match (arcana_desktop.app.event_window_id :: event :: call):
        Option.Some(id) => id.value == (arcana_desktop.app.main_window_id :: cx :: call).value
        Option.None => false

export fn is_main_window(read cx: arcana_desktop.types.AppContext, read win: arcana_desktop.types.Window) -> Bool:
    return (arcana_desktop.window.id :: win :: call).value == (arcana_desktop.app.main_window_id :: cx :: call).value

fn clear_current_target(edit cx: arcana_desktop.types.AppContext):
    cx.current_window_id = Option.None[arcana_desktop.types.WindowId] :: :: call
    cx.current_is_main_window = false

fn set_current_target(edit cx: arcana_desktop.types.AppContext, read window_id: arcana_desktop.types.WindowId, is_main_window: Bool):
    cx.current_window_id = Option.Some[arcana_desktop.types.WindowId] :: window_id :: call
    cx.current_is_main_window = is_main_window

export fn target_window(read cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> Option[arcana_desktop.types.Window]:
    return arcana_desktop.app.target_window_for_id :: cx, target.window_id :: call

export fn require_target_window(read cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> Result[arcana_desktop.types.Window, Str]:
    return match (arcana_desktop.app.target_window :: cx, target :: call):
        Option.Some(win) => Result.Ok[arcana_desktop.types.Window, Str] :: win :: call
        Option.None => Result.Err[arcana_desktop.types.Window, Str] :: "missing target event window" :: call

export fn target_is_main_window(read target: arcana_desktop.types.TargetedEvent) -> Bool:
    return target.is_main_window

export fn current_window_id(read cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.WindowId]:
    return cx.current_window_id

export fn current_window(read cx: arcana_desktop.types.AppContext) -> Option[arcana_desktop.types.Window]:
    return match cx.current_window_id:
        Option.Some(id) => arcana_desktop.app.target_window_for_id :: cx, id :: call
        Option.None => Option.None[arcana_desktop.types.Window] :: :: call

export fn require_current_window(read cx: arcana_desktop.types.AppContext) -> Result[arcana_desktop.types.Window, Str]:
    return match (arcana_desktop.app.current_window :: cx :: call):
        Option.Some(win) => Result.Ok[arcana_desktop.types.Window, Str] :: win :: call
        Option.None => Result.Err[arcana_desktop.types.Window, Str] :: "missing current event window" :: call

export fn current_is_main_window(read cx: arcana_desktop.types.AppContext) -> Bool:
    return cx.current_is_main_window

export fn target_window_id(read target: arcana_desktop.types.TargetedEvent) -> arcana_desktop.types.WindowId:
    return target.window_id

export fn close_target_window(edit cx: arcana_desktop.types.AppContext, read target: arcana_desktop.types.TargetedEvent) -> Result[arcana_desktop.types.ControlFlow, Str]:
    return match (arcana_desktop.app.target_window :: cx, target :: call):
        Option.Some(win) => arcana_desktop.app.queue_window_close :: win :: call
        Option.None => Result.Ok[arcana_desktop.types.ControlFlow, Str] :: (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call

export fn close_window(edit cx: arcana_desktop.types.AppContext, read win: arcana_desktop.types.Window) -> Result[arcana_desktop.types.ControlFlow, Str]:
    let _ = cx
    return arcana_desktop.app.queue_window_close :: win :: call

export fn close_current_window(edit cx: arcana_desktop.types.AppContext) -> Result[arcana_desktop.types.ControlFlow, Str]:
    return match (arcana_desktop.app.current_window :: cx :: call):
        Option.Some(win) => queue_window_close :: win :: call
        Option.None => Result.Err[arcana_desktop.types.ControlFlow, Str] :: "missing current event window" :: call

fn queue_window_close(read win: arcana_desktop.types.Window) -> Result[arcana_desktop.types.ControlFlow, Str]:
    arcana_desktop.app.owner_queue_close :: (arcana_desktop.window.id :: win :: call) :: call
    arcana_desktop.app.owner_request_main_sync :: :: call
    return Result.Ok[arcana_desktop.types.ControlFlow, Str] :: (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call

export fn open_window(edit cx: arcana_desktop.types.AppContext, title: Str, size: (Int, Int)) -> Result[arcana_desktop.types.Window, Str]:
    let mut cfg = arcana_desktop.window.default_config :: :: call
    let mut bounds = arcana_desktop.types.WindowBounds :: size = size, position = cfg.bounds.position, visible = cfg.bounds.visible :: call
    bounds.min_size = cfg.bounds.min_size
    bounds.max_size = cfg.bounds.max_size
    cfg.title = title
    cfg.bounds = bounds
    return arcana_desktop.app.open_window_cfg :: cx, cfg :: call

export fn open_window_cfg(edit cx: arcana_desktop.types.AppContext, read cfg: arcana_desktop.types.WindowConfig) -> Result[arcana_desktop.types.Window, Str]:
    let opened = arcana_desktop.window.open_in :: cx.runtime.session, cfg :: call
    return match opened:
        Result.Ok(win) => open_window_cfg_ready :: cx, cfg, win :: call
        Result.Err(err) => Result.Err[arcana_desktop.types.Window, Str] :: err :: call

fn open_window_cfg_ready(edit cx: arcana_desktop.types.AppContext, read cfg: arcana_desktop.types.WindowConfig, take win: arcana_desktop.types.Window) -> Result[arcana_desktop.types.Window, Str]:
    let win_id = arcana_desktop.window.id :: win :: call
    if cfg.bounds.visible:
        arcana_desktop.app.owner_queue_redraw :: win_id :: call
    return Result.Ok[arcana_desktop.types.Window, Str] :: win :: call

fn redraw_window(take win: arcana_desktop.types.Window):
    let mut win = win
    arcana_desktop.window.request_redraw :: win :: call

DesktopPendingActions
fn owner_queue_close(read id: arcana_desktop.types.WindowId):
    if arcana_desktop.app.contains_window_id :: DesktopPendingActions.close_ids, id :: call:
        return
    let mut ids = DesktopPendingActions.close_ids
    ids :: id :: push
    DesktopPendingActions.close_ids = ids

DesktopPendingActions
fn owner_queue_redraw(read id: arcana_desktop.types.WindowId):
    if arcana_desktop.app.contains_window_id :: DesktopPendingActions.redraw_ids, id :: call:
        return
    let mut ids = DesktopPendingActions.redraw_ids
    ids :: id :: push
    DesktopPendingActions.redraw_ids = ids

DesktopPendingActions
fn owner_request_main_sync():
    DesktopPendingActions.sync_main_window = true

DesktopPendingActions
fn owner_take_close_ids() -> List[arcana_desktop.types.WindowId]:
    let ids = DesktopPendingActions.close_ids
    DesktopPendingActions.close_ids = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
    return arcana_desktop.app.reverse_window_ids :: ids :: call

DesktopPendingActions
fn owner_take_redraw_ids() -> List[arcana_desktop.types.WindowId]:
    let ids = DesktopPendingActions.redraw_ids
    DesktopPendingActions.redraw_ids = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
    return arcana_desktop.app.reverse_window_ids :: ids :: call

fn reverse_window_ids(take ids: List[arcana_desktop.types.WindowId]) -> List[arcana_desktop.types.WindowId]:
    let mut ids = ids
    let mut out = std.collections.list.new[arcana_desktop.types.WindowId] :: :: call
    while not (ids :: :: is_empty):
        out :: (ids :: :: pop) :: push
    return out

DesktopPendingActions
fn owner_take_sync_main_window() -> Bool:
    let sync = DesktopPendingActions.sync_main_window
    DesktopPendingActions.sync_main_window = false
    return sync

fn apply_close_id(edit cx: arcana_desktop.types.AppContext, read id: arcana_desktop.types.WindowId):
    return match (arcana_desktop.app.target_window_for_id :: cx, id :: call):
        Option.Some(win) => apply_close_id_ready :: win :: call
        Option.None => 0

fn apply_close_id_ready(take win: arcana_desktop.types.Window):
    let mut win = win
    let _ = arcana_desktop.window.close :: win :: call

fn apply_redraw_id(edit cx: arcana_desktop.types.AppContext, read id: arcana_desktop.types.WindowId):
    return match (arcana_desktop.app.target_window_for_id :: cx, id :: call):
        Option.Some(win) => apply_redraw_id_ready :: win :: call
        Option.None => 0

fn apply_redraw_id_ready(take win: arcana_desktop.types.Window):
    redraw_window :: win :: call

fn sync_window_lifetime(edit cx: arcana_desktop.types.AppContext):
    if cx.control.exit_requested:
        return
    let _ = arcana_desktop.app.sync_main_window :: cx :: call
    let live_window = arcana_desktop.app.first_live_window :: cx :: call
    if live_window :: :: is_some:
        return
    arcana_desktop.app.request_exit :: cx, 0 :: call

fn dispatch_polled_event[A, where arcana_desktop.app.Application[A]](edit app: A, edit cx: arcana_desktop.types.AppContext, read next: Option[arcana_desktop.types.AppEvent]) -> arcana_desktop.types.ControlFlow:
    return match next:
        Option.Some(event) => arcana_desktop.app.dispatch_event :: app, cx, event :: call
        Option.None => cx.control.control_flow

DesktopAppState
fn owner_exit_requested() -> Bool:
    return DesktopAppState.cx.control.exit_requested

DesktopAppState
fn owner_wait_timeout(read app_loop: arcana_desktop.types.AppLoop) -> Int:
    return arcana_desktop.app.wait_timeout_for_flow :: DesktopAppState.cx.control.control_flow, app_loop :: call

DesktopAppState
fn owner_next_frame(read app_loop: arcana_desktop.types.AppLoop) -> arcana_desktop.types.FrameInput:
    let timeout = arcana_desktop.app.owner_wait_timeout :: app_loop :: call
    let mut session = DesktopAppState.cx.runtime.session
    let frame = match timeout:
        -2 => arcana_desktop.events.pump_session :: session :: call
        _ => arcana_desktop.events.wait_session :: session, timeout :: call
    DesktopAppState.cx.runtime.session = session
    return frame

DesktopAppState
fn owner_dispatch_polled_event[A, where arcana_desktop.app.Application[A]](edit app: A, read next: Option[arcana_desktop.types.AppEvent]):
    let mut cx = DesktopAppState.cx
    let flow = arcana_desktop.app.dispatch_polled_event :: app, cx, next :: call
    cx.control.control_flow = flow
    DesktopAppState.cx = cx

DesktopAppState
DesktopPendingActions
fn owner_apply_pending_actions():
    let mut cx = DesktopAppState.cx
    let mut closes = arcana_desktop.app.owner_take_close_ids :: :: call
    while not (closes :: :: is_empty):
        arcana_desktop.app.apply_close_id :: cx, (closes :: :: pop) :: call
    if arcana_desktop.app.owner_take_sync_main_window :: :: call:
        let _ = arcana_desktop.app.sync_main_window :: cx :: call
    let mut redraws = arcana_desktop.app.owner_take_redraw_ids :: :: call
    while not (redraws :: :: is_empty):
        arcana_desktop.app.apply_redraw_id :: cx, (redraws :: :: pop) :: call
    DesktopAppState.cx = cx

DesktopAppState
fn owner_run_frame[A, where arcana_desktop.app.Application[A]](edit app: A, edit frame: arcana_desktop.types.FrameInput):
    while true:
        let next = arcana_desktop.events.poll :: frame :: call
        if next :: :: is_none:
            return
        arcana_desktop.app.owner_dispatch_polled_event :: app, next :: call
        arcana_desktop.app.owner_apply_pending_actions :: :: call
        let mut cx = DesktopAppState.cx
        arcana_desktop.app.sync_window_lifetime :: cx :: call
        DesktopAppState.cx = cx
        if DesktopAppState.cx.control.exit_requested:
            return

DesktopOwner
DesktopAppState
DesktopLifecycle
fn finish_run[A, where arcana_desktop.app.Application[A]](edit app: A) -> Int:
    let mut cx = DesktopAppState.cx
    arcana_desktop.app.run_exiting :: app, cx :: call
    let code = cx.control.exit_code
    let mut session = cx.runtime.session
    arcana_desktop.events.close_session :: session :: call
    DesktopAppState.cx = cx
    DesktopLifecycle.finished = true
    return code

DesktopOwner
DesktopAppState
DesktopLifecycle
fn run_with_window[A, where arcana_desktop.app.Application[A]](edit app: A, take run: arcana_desktop.app.RunWindow) -> Int:
    let control = arcana_desktop.types.RunControl :: exit_requested = false, exit_code = 0, control_flow = (arcana_desktop.types.ControlFlow.Wait :: :: call) :: call
    let mut cx = arcana_desktop.types.AppContext :: runtime = run.runtime, control = control :: call
    cx.current_window_id = Option.None[arcana_desktop.types.WindowId] :: :: call
    cx.current_is_main_window = false
    let active = DesktopOwner :: :: call
    let _ = active
    DesktopAppState.cx = cx
    while not (arcana_desktop.app.owner_exit_requested :: :: call):
        let mut frame = arcana_desktop.app.owner_next_frame :: run.cfg.loop :: call
        arcana_desktop.app.owner_run_frame :: app, frame :: call
    return arcana_desktop.app.finish_run :: app :: call

fn run_open_failed(edit session: arcana_desktop.types.Session) -> Int:
    arcana_desktop.events.close_session :: session :: call
    return 1

export fn run[A, where arcana_desktop.app.Application[A]](edit app: A, read cfg: arcana_desktop.types.AppConfig) -> Int:
    let mut session = arcana_desktop.events.open_session :: :: call
    let wake = arcana_desktop.events.create_wake :: session :: call
    let opened = arcana_desktop.window.open_in :: session, cfg.window :: call
    let launch = arcana_desktop.app.LaunchConfig :: cfg = cfg, session = session, wake = wake :: call
    return match opened:
        std.result.Result.Ok(value) => run_with_window_ready :: app, launch, value :: call
        std.result.Result.Err(_) => run_open_failed_launch :: launch :: call

fn run_open_failed_launch(read launch: arcana_desktop.app.LaunchConfig) -> Int:
    let mut session = launch.session
    return arcana_desktop.app.run_open_failed :: session :: call

fn run_with_window_ready[A, where arcana_desktop.app.Application[A]](edit app: A, read launch: arcana_desktop.app.LaunchConfig, take value: arcana_desktop.types.Window) -> Int:
    let main_window_id = arcana_desktop.window.id :: value :: call
    let mut runtime = arcana_desktop.types.RuntimeContext :: session = launch.session, wake = launch.wake, main_window_id = main_window_id :: call
    runtime.main_window = value
    let run = arcana_desktop.app.RunWindow :: cfg = launch.cfg, runtime = runtime :: call
    if launch.cfg.window.bounds.visible:
        let mut main_window = run.runtime.main_window
        arcana_desktop.window.request_redraw :: main_window :: call
    return arcana_desktop.app.run_with_window :: app, run :: call
