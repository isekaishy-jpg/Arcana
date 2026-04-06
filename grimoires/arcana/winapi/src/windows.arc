native callback window_proc(read window: arcana_winapi.types.HiddenWindow, message: Int, wparam: Int, lparam: Int) -> Int = arcana_winapi.callbacks.handle_window_proc

export native fn create_hidden_window() -> arcana_winapi.types.HiddenWindow = windows.create_hidden_window
export native fn post_ping(read window: arcana_winapi.types.HiddenWindow, code: Int) = windows.post_ping
export native fn pump_messages() -> Int = windows.pump_messages
export native fn take_last_callback_code() -> Int = windows.take_last_callback_code
export native fn destroy_hidden_window(take window: arcana_winapi.types.HiddenWindow) = windows.destroy_hidden_window
