use arcana_winapi.desktop_handles.WakeHandle

// Internal backend declarations for the generic Win32 wake/message glue.
// This module is intentionally not reexported from `arcana_winapi`.
native fn take_last_error() -> Str = backend.message.take_last_error
native fn wake_create() -> WakeHandle = backend.message.wake_create
native fn wake_close(take handle: WakeHandle) -> Bool = backend.message.wake_close
native fn wake_signal(read handle: WakeHandle) = backend.message.wake_signal
native fn wake_take_pending(edit handle: WakeHandle) -> Int = backend.message.wake_take_pending
native fn wait_wake_or_messages(read handle: WakeHandle, timeout_ms: Int) -> Bool = backend.message.wait_wake_or_messages
native fn pump_messages() -> Int = backend.message.pump_messages
