import std.result
use arcana_winapi.desktop_handles.WakeHandle
use std.result.Result

fn take_last_error() -> Str:
    return arcana_winapi.backend.message.take_last_error :: :: call

fn wake_create_raw() -> WakeHandle:
    return arcana_winapi.backend.message.wake_create :: :: call

fn wake_close_raw(take handle: WakeHandle) -> Bool:
    return arcana_winapi.backend.message.wake_close :: handle :: call

fn result_handle[T](take value: T) -> Result[T, Str]:
    let err = take_last_error :: :: call
    if err == "":
        return Result.Ok[T, Str] :: value :: call
    return Result.Err[T, Str] :: err :: call

fn result_unit(ok: Bool) -> Result[Unit, Str]:
    if ok:
        return Result.Ok[Unit, Str] :: :: call
    return Result.Err[Unit, Str] :: (take_last_error :: :: call) :: call

export fn wake_create() -> Result[WakeHandle, Str]:
    return result_handle[WakeHandle] :: (wake_create_raw :: :: call) :: call

export fn wake_close(take handle: WakeHandle) -> Result[Unit, Str]:
    return result_unit :: (wake_close_raw :: handle :: call) :: call

export fn wake_signal(read handle: WakeHandle):
    return arcana_winapi.backend.message.wake_signal :: handle :: call

export fn wake_take_pending(edit handle: WakeHandle) -> Int:
    return arcana_winapi.backend.message.wake_take_pending :: handle :: call

export fn wait_wake_or_messages(read handle: WakeHandle, timeout_ms: Int) -> Bool:
    return arcana_winapi.backend.message.wait_wake_or_messages :: handle, timeout_ms :: call

export fn pump_messages() -> Int:
    return arcana_winapi.backend.message.pump_messages :: :: call
