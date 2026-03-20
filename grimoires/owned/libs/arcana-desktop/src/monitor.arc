import arcana_desktop.types
import arcana_desktop.window
use std.window.Window

export fn current(read win: Window) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.current_monitor :: win :: call

export fn primary() -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.primary_monitor :: :: call

export fn count() -> Int:
    return arcana_desktop.window.monitor_count :: :: call

export fn get(index: Int) -> arcana_desktop.types.MonitorInfo:
    return arcana_desktop.window.monitor :: index :: call
