# Arcana Desktop Proof

This is the checked-in `arcana_desktop` proof workspace.

It packages as a normal native desktop bundle:
- `app.exe`
- `arcana_desktop.dll`

The app is the thing to open and use when checking the grimoire manually. It demonstrates:
- window shell controls: resize, minimize, maximize, fullscreen, resizable, decorated, transparent, topmost, theme override
- whole-record window settings application, including live size/min/max/profile updates
- cursor and mouse hooks: visibility, icon, grab mode, reposition, move, wheel, enter/leave, raw motion
- keyboard and text-input IO: key metadata, modifiers, committed text, IME lifecycle state
- clipboard text and bytes
- monitor and theme reporting
- wake/control-flow behavior through the app runner
- live secondary window open, redraw, resize, and close handling through the public desktop shell
- a simple button-driven settings surface that later UI/settings grimoires can build on

Run it normally:
- `arcana package --member app --target windows-exe`
- launch the staged `app.exe` from the bundle directory beside `arcana_desktop.dll`

The packaged app should stay open until you close it. The window close button should exit cleanly.

For deterministic automated proof, run the packaged exe with:
- `app.exe --smoke`

That prints:
- `controls=36`
- `pages=7`
- `smoke_score=767`

The desktop runtime DLL is selected through the normal dependency metadata on `app/book.toml`:
- `arcana_desktop = { path = "../../../grimoires/owned/libs/arcana-desktop", native_delivery = "dll" }`

That keeps the Arcana source-level desktop APIs in the app package while moving the native runtime host/session boundary into the sibling `arcana_desktop.dll` provider.

Bundle note:
- the staged native bundle now includes the Rust `std-*.dll` closure needed by `arcana_desktop.dll`
