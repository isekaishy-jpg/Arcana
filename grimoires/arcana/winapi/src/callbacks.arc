fn handle_window_proc(read window: arcana_winapi.types.HiddenWindow, message: Int, wparam: Int, lparam: Int) -> Int:
    let _window = window
    let _message = message
    let _lparam = lparam
    return wparam + 1
