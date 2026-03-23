fn overview_page() -> Str:
    let mut text = ""
    text = text + "Arcana Desktop Proof\n"
    text = text + "\n"
    text = text + "This is the checked-in showcase for arcana_desktop.\n"
    text = text + "It is meant to exercise the public desktop shell directly.\n"
    text = text + "\n"
    text = text + "What the rebuilt showcase covers:\n"
    text = text + "- multi-window session control and last-window shutdown\n"
    text = text + "- live window queries plus direct setters and apply_settings\n"
    text = text + "- explicit text-input and composition-area control\n"
    text = text + "- clipboard, monitor, theme, wake, and raw-device policy lanes\n"
    text = text + "- raw mouse motion, raw button, raw wheel, and raw key events\n"
    text = text + "\n"
    text = text + "The control deck on the left changes shell state.\n"
    text = text + "The guide page explains the current group.\n"
    text = text + "The right panel reads state back from the desktop API.\n"
    return text

fn window_page() -> Str:
    let mut text = ""
    text = text + "Window Shell\n"
    text = text + "\n"
    text = text + "This deck now drives the broader window surface:\n"
    text = text + "- Theme, Topmost, Decorated, Resizable, and Transparent\n"
    text = text + "- Full, Max, and Min state toggles\n"
    text = text + "- Move+ and Clamp for live position/size/min/max changes\n"
    text = text + "- Preset for the diffed window.settings/apply_settings path\n"
    text = text + "- 2nd Window, 2nd Vis, and 2nd End for secondary-window control\n"
    text = text + "\n"
    text = text + "Close requests should close only that window.\n"
    text = text + "The process exits after the last live window closes.\n"
    return text

fn cursor_page() -> Str:
    let mut text = ""
    text = text + "Pointer And Cursor\n"
    text = text + "\n"
    text = text + "Window input and raw device input stay separate.\n"
    text = text + "Useful pointer checks:\n"
    text = text + "- Cursor Icon cycles native cursor shapes\n"
    text = text + "- Cursor Vis hides and shows the cursor\n"
    text = text + "- Grab cycles Free, Confined, and Locked grab modes\n"
    text = text + "- Center repositions the cursor inside the client area\n"
    text = text + "- mouse move and wheel stay in the window lane\n"
    text = text + "- raw motion, button, and wheel stay in the device lane\n"
    return text

fn text_page() -> Str:
    let mut text = ""
    text = text + "Keyboard, Text, And IME\n"
    text = text + "\n"
    text = text + "Text input is explicit in this build.\n"
    text = text + "Nothing enables it until you press a control.\n"
    text = text + "\n"
    text = text + "The text controls split the API paths deliberately:\n"
    text = text + "- Text Input flips the direct enabled setter\n"
    text = text + "- Comp toggles composition area through direct set/clear calls\n"
    text = text + "- TextCfg cycles window.text_input_settings/apply_text_input_settings\n"
    text = text + "- committed text and composition updates appear live on the right\n"
    text = text + "- raw key events stay separate from window key events\n"
    return text

fn clipboard_page() -> Str:
    let mut text = ""
    text = text + "Clipboard, Monitor, And Theme\n"
    text = text + "\n"
    text = text + "Desktop shell work is more than windows and events.\n"
    text = text + "This page checks the integration surface too:\n"
    text = text + "- Copy Text writes a page-tagged string payload\n"
    text = text + "- Copy Bytes writes bytes and reports the byte count\n"
    text = text + "- the live panel shows current monitor, primary monitor, and monitor count\n"
    text = text + "- scale factor and theme/override are read back live from the shell\n"
    text = text + "\n"
    text = text + "Monitor helpers stay inside arcana_desktop rather than leaking substrate APIs.\n"
    return text

fn events_page() -> Str:
    let mut text = ""
    text = text + "Events And Loop Control\n"
    text = text + "\n"
    text = text + "The proof app runs through the public Application callbacks:\n"
    text = text + "- resumed and suspended\n"
    text = text + "- window_event for resize, move, focus, theme, scale, key, mouse, text, and drop\n"
    text = text + "- device_event for raw mouse motion/button/wheel and raw key\n"
    text = text + "- about_to_wait, wake, and exiting\n"
    text = text + "\n"
    text = text + "Wake signals the session wake handle directly.\n"
    text = text + "Policy cycles raw-device delivery between Never, Focused, and Always.\n"
    text = text + "Resize, move, focus, and drag-drop are manual checks on this page.\n"
    return text

fn notes_page() -> Str:
    let mut text = ""
    text = text + "Manual Checks\n"
    text = text + "\n"
    text = text + "Useful correctness checks for the current proof build:\n"
    text = text + "- button 1 should still move from Overview to Window\n"
    text = text + "- raw device totals and mouse position should keep updating live\n"
    text = text + "- text input should stay off until Text Input or TextCfg is used\n"
    text = text + "- Comp and TextCfg should surface composition state on the right\n"
    text = text + "- 2nd Window should open another live window without coupling clicks to shutdown\n"
    text = text + "- Full, Max, Min, Move+, Clamp, and Preset should all visibly affect shell state\n"
    text = text + "- WM_CLOSE should still shut the process down cleanly after the last window closes\n"
    return text

export fn count() -> Int:
    return 7

export fn title(index: Int) -> Str:
    if index == 0:
        return "Overview"
    if index == 1:
        return "Window"
    if index == 2:
        return "Cursor"
    if index == 3:
        return "Text"
    if index == 4:
        return "Clipboard"
    if index == 5:
        return "Events"
    return "Notes"

export fn body(index: Int) -> Str:
    if index == 0:
        return pages.overview_page :: :: call
    if index == 1:
        return pages.window_page :: :: call
    if index == 2:
        return pages.cursor_page :: :: call
    if index == 3:
        return pages.text_page :: :: call
    if index == 4:
        return pages.clipboard_page :: :: call
    if index == 5:
        return pages.events_page :: :: call
    return pages.notes_page :: :: call
