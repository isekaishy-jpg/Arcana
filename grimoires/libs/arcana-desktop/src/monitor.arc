import arcana_desktop.types
import arcana_desktop.window

export fn current(read win: arcana_winapi.desktop_handles.Window) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.current_monitor :: win :: call

export fn primary() -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.primary_monitor :: :: call

export fn count() -> Int:
    return arcana_desktop.window.monitor_count :: :: call

export fn get(index: Int) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.monitor :: index :: call

