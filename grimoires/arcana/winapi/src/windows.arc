native callback window_proc: arcana_winapi.raw.callbacks.WNDPROC = arcana_winapi.windows.handle_window_proc

export fn handle_window_proc(read hwnd: arcana_winapi.raw.types.HWND, message: arcana_winapi.raw.types.UINT, wparam: arcana_winapi.raw.types.WPARAM, lparam: arcana_winapi.raw.types.LPARAM) -> arcana_winapi.raw.types.LRESULT:
    let _hwnd = hwnd
    let _message = message
    let _wparam = wparam
    let _lparam = lparam
    return 42

export native fn create_hidden_window() -> arcana_winapi.raw.types.HWND = windows.create_hidden_window
export native fn post_ping(read window: arcana_winapi.raw.types.HWND, code: Int) = windows.post_ping
export native fn pump_messages() -> Int = windows.pump_messages
export native fn take_last_callback_code() -> Int = windows.take_last_callback_code
export native fn destroy_hidden_window(take window: arcana_winapi.raw.types.HWND) = windows.destroy_hidden_window
