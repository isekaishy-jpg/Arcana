foreword tool.meta.trace:
    tier = basic
    visibility = public
    targets = [fn, field, param]
    retention = runtime
    payload = [label: Str]

foreword tool.exec.note:
    tier = executable
    visibility = public
    action = metadata
    targets = [fn]
    retention = runtime
    payload = [slot: Str]
    handler = tool.exec.note_handler

foreword handler tool.exec.note_handler:
    protocol = "stdio-v1"
    product = "tool-forewords"
    entry = "note"
