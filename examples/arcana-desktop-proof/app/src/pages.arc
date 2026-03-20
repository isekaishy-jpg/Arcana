fn overview_page() -> Str:
    let mut text = ""
    text = text + "Arcana Desktop Proof\n"
    text = text + "\n"
    text = text + "This example is a real checked-in app built on arcana_desktop.\n"
    text = text + "It exercises the public facade instead of private runtime seams.\n"
    text = text + "\n"
    text = text + "What this window demonstrates:\n"
    text = text + "- live window settings and runtime queries\n"
    text = text + "- keyboard metadata: key, logical key, physical key, location, repeat, modifiers\n"
    text = text + "- mouse movement, wheel, enter/leave, buttons, and raw mouse motion events\n"
    text = text + "- cursor visibility, icon, grab mode, and explicit repositioning\n"
    text = text + "- text input, committed text, and IME composition state\n"
    text = text + "- monitor and theme queries\n"
    text = text + "- clipboard text and bytes roundtrips\n"
    text = text + "- wake signaling through the session wake handle\n"
    text = text + "- multi-window open, redraw, resize, and close handling\n"
    text = text + "- fixed-step ECS adapter stepping through arcana_desktop.ecs\n"
    text = text + "\n"
    text = text + "Global keyboard controls:\n"
    text = text + "- Esc: exit\n"
    text = text + "- Q / E: previous or next guide page\n"
    text = text + "- F1 / F2 / F3: Poll, Wait, or WaitUntil loop modes\n"
    text = text + "- O: apply the targeted live window profile\n"
    text = text + "- W: signal a wake event\n"
    text = text + "- N: open a second session window\n"
    text = text + "\n"
    text = text + "The left panel buttons drive the same APIs a later UI grimoire would use.\n"
    text = text + "The right panel shows the live state read back from the substrate.\n"
    return text

fn window_page() -> Str:
    let mut text = ""
    text = text + "Window Settings And Shell\n"
    text = text + "\n"
    text = text + "The showcase exercises the live targeted window controls used by later settings UIs.\n"
    text = text + "Use the left panel or keyboard shortcuts to change these live settings:\n"
    text = text + "- fullscreen\n"
    text = text + "- maximized\n"
    text = text + "- minimized\n"
    text = text + "- resizable\n"
    text = text + "- decorated\n"
    text = text + "- transparent\n"
    text = text + "- topmost\n"
    text = text + "- theme override\n"
    text = text + "- min / max size constraints via the profile action\n"
    text = text + "\n"
    text = text + "Relevant buttons:\n"
    text = text + "- Profile applies a full settings profile through the targeted hooks\n"
    text = text + "- Fullscreen / Maximize / Minimize / Resizable / Decorated / Transparent / Topmost\n"
    text = text + "- Attention requests shell attention without changing ownership\n"
    text = text + "\n"
    text = text + "Observed status on the right:\n"
    text = text + "- title, alive, focused, visible\n"
    text = text + "- current position and size\n"
    text = text + "- min and max size constraints\n"
    text = text + "- scale factor, theme, theme override\n"
    text = text + "- fullscreen, minimized, maximized, resized\n"
    text = text + "\n"
    text = text + "The close button should work. WindowCloseRequested exits cleanly.\n"
    return text

fn cursor_page() -> Str:
    let mut text = ""
    text = text + "Cursor And Mouse\n"
    text = text + "\n"
    text = text + "Arcana Desktop exposes cursor and pointer behavior as reusable substrate.\n"
    text = text + "This example lets you verify:\n"
    text = text + "- cursor visibility on and off\n"
    text = text + "- cursor icon cycling across the public icon set\n"
    text = text + "- cursor grab mode cycling between Free, Confined, and Locked\n"
    text = text + "- explicit cursor repositioning\n"
    text = text + "- raw mouse motion events in the device event lane\n"
    text = text + "- mouse enter / leave, button, move, and wheel events in the window event lane\n"
    text = text + "\n"
    text = text + "Try this sequence:\n"
    text = text + "1. Click Cursor Icon a few times and confirm the OS cursor changes.\n"
    text = text + "2. Toggle Grab and move the cursor.\n"
    text = text + "3. Use Center Cursor to force a reposition.\n"
    text = text + "4. Move the mouse with grab active and watch raw motion totals grow.\n"
    text = text + "\n"
    text = text + "The live state panel reports the current cursor state and the last mouse event.\n"
    return text

fn text_page() -> Str:
    let mut text = ""
    text = text + "Keyboard, Text, And IME\n"
    text = text + "\n"
    text = text + "Desktop shell parity needs both key events and text-input IO.\n"
    text = text + "This app tracks:\n"
    text = text + "- key down and key up counts\n"
    text = text + "- logical and physical key identity\n"
    text = text + "- key location and repeat state\n"
    text = text + "- modifier flags decoded through the input helpers\n"
    text = text + "- committed TextInput events\n"
    text = text + "- IME composition started, updated, committed, and cancelled events\n"
    text = text + "\n"
    text = text + "Use these controls:\n"
    text = text + "- Text Input toggles the enabled state\n"
    text = text + "- Set Comp sets an active composition rectangle\n"
    text = text + "- Clear Comp removes that rectangle\n"
    text = text + "\n"
    text = text + "Then type into the window or use an IME.\n"
    text = text + "Committed text, composition text, and caret updates are echoed into the status area.\n"
    text = text + "This is intentionally low-level IO; text rendering stays outside the shell.\n"
    return text

fn clipboard_page() -> Str:
    let mut text = ""
    text = text + "Clipboard, Monitors, Theme, And Wake\n"
    text = text + "\n"
    text = text + "The grimoire exposes low-level integration points other libraries can build on.\n"
    text = text + "In this example you can test:\n"
    text = text + "- clipboard text write and read\n"
    text = text + "- clipboard bytes write and read\n"
    text = text + "- primary monitor, current monitor, and monitor count queries\n"
    text = text + "- live theme and scale-factor reporting\n"
    text = text + "- wake signaling through the session wake handle\n"
    text = text + "- multi-window session behavior through the public desktop facade\n"
    text = text + "\n"
    text = text + "Suggested checks:\n"
    text = text + "- Click Copy Text and then paste somewhere else.\n"
    text = text + "- Click Copy Bytes and inspect the byte count in the status panel.\n"
    text = text + "- Move the window between monitors if you have more than one display.\n"
    text = text + "- Click Wake and confirm the wake counter advances.\n"
    text = text + "- Click Second Win to open a live secondary window.\n"
    return text

fn events_page() -> Str:
    let mut text = ""
    text = text + "Events, Loop, And Adapter\n"
    text = text + "\n"
    text = text + "Arcana Desktop owns the app shell but does not hide event-loop structure.\n"
    text = text + "This example uses the public Application callbacks:\n"
    text = text + "- resumed\n"
    text = text + "- suspended\n"
    text = text + "- window_event\n"
    text = text + "- device_event\n"
    text = text + "- about_to_wait\n"
    text = text + "- wake\n"
    text = text + "- exiting\n"
    text = text + "\n"
    text = text + "It also runs an arcana_desktop.ecs.Adapter from about_to_wait using loop timing helpers.\n"
    text = text + "The adapter total in the status panel increments as the fixed runner advances.\n"
    text = text + "\n"
    text = text + "Control-flow shortcuts:\n"
    text = text + "- F1: Poll\n"
    text = text + "- F2: Wait\n"
    text = text + "- F3: WaitUntil for a short future deadline\n"
    text = text + "\n"
    text = text + "This keeps the example grounded in the real facade instead of bypassing it.\n"
    return text

fn notes_page() -> Str:
    let mut text = ""
    text = text + "Manual Checks And Known Boundaries\n"
    text = text + "\n"
    text = text + "This app can visibly demonstrate most of the public arcana_desktop surface today.\n"
    text = text + "A few paths still depend on real user or OS input instead of deterministic automation:\n"
    text = text + "- file drop requires dropping a path onto the window\n"
    text = text + "- live IME composition updates depend on your IME\n"
    text = text + "- raw mouse motion is easiest to observe with grab active\n"
    text = text + "- attention requests depend on shell behavior\n"
    text = text + "- Second Win opens a live secondary window through the same facade used by apps\n"
    text = text + "- the Profile action applies a whole-record window settings roundtrip live\n"
    text = text + "\n"
    text = text + "The example intentionally stays in the desktop-shell layer.\n"
    text = text + "It does not invent a retained UI framework, shortcut manager, or text editor widget.\n"
    text = text + "Those belong in later UI or app grimoires that consume these hooks.\n"
    text = text + "\n"
    text = text + "Use this proof app as the baseline when expanding arcana_desktop.\n"
    text = text + "If a future API change makes one of these controls impossible to implement cleanly,\n"
    text = text + "that is real grimoire friction and should be fixed at the substrate or facade layer.\n"
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
