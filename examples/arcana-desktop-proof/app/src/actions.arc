import pages

export fn button_count() -> Int:
    return 36

export fn button_label(id: Int) -> Str:
    if id == 0:
        return "Prev"
    if id == 1:
        return "Next"
    if id == 2:
        return "Overview"
    if id == 3:
        return "Window"
    if id == 4:
        return "Cursor"
    if id == 5:
        return "Text"
    if id == 6:
        return "Clipboard"
    if id == 7:
        return "Events"
    if id == 8:
        return "Notes"
    if id == 9:
        return "Theme"
    if id == 10:
        return "Text Input"
    if id == 11:
        return "Cursor Icon"
    if id == 12:
        return "Cursor Vis"
    if id == 13:
        return "Center"
    if id == 14:
        return "Topmost"
    if id == 15:
        return "Decorated"
    if id == 16:
        return "Resizable"
    if id == 17:
        return "Attention"
    if id == 18:
        return "Copy Text"
    if id == 19:
        return "Copy Bytes"
    if id == 20:
        return "Wake"
    if id == 21:
        return "Policy"
    if id == 22:
        return "2nd Window"
    if id == 23:
        return "Exit"
    if id == 24:
        return "Full"
    if id == 25:
        return "Max"
    if id == 26:
        return "Min"
    if id == 27:
        return "Transp"
    if id == 28:
        return "Grab"
    if id == 29:
        return "Move+"
    if id == 30:
        return "Clamp"
    if id == 31:
        return "Preset"
    if id == 32:
        return "Comp"
    if id == 33:
        return "TextCfg"
    if id == 34:
        return "2nd Vis"
    if id == 35:
        return "2nd End"
    return "Action"

export fn button_page(id: Int) -> Int:
    if id >= 2 and id <= 8:
        return id - 2
    return -1

export fn next_page_index(current: Int, delta: Int) -> Int:
    let total = pages.count :: :: call
    let mut next = current + delta
    if next < 0:
        next = total - 1
    if next >= total:
        next = 0
    return next
