# Bring-Up Plan: Finish `arcana_desktop` to Windows-First winit-Class Parity

## Summary
- Complete the desktop bring-up as a real parity pass, not a wrapper pass: broaden the shared `std.*` substrate first, then lift it into `arcana_desktop`.
- Target parity for the desktop shell and input/text stack needed by future app/UI grimoires: window lifecycle/config/state, cursor/mouse behavior, keyboard metadata, committed text, IME composition, monitor/theme/clipboard/drag-drop, and runtime settings hooks.
- Keep policy split clean:
  - `std.*` owns raw host-facing capability and typed settings/state records.
  - `arcana_desktop` owns the canonical app-shell package and session runner.
  - later app/UI grimoires own keybinds, text editing widgets, shortcut routing, and settings-screen UX.
  - `arcana_desktop` is the authoritative public desktop boundary, analogous in role breadth to winit-class app-shell libraries; it should not collapse into a thin wrapper over separately-public raw `std.*` shell APIs.

## Key Changes
- Expand the low-level substrate with typed records and live settings/query hooks.
  - `std.window` gains the missing parity surface:
    - create-time `WindowConfig` expands to include min/max size, transparency, theme override, cursor defaults, and text-input defaults.
    - add runtime `WindowSettings` and `CursorSettings` records plus `settings(read win)`, `apply_settings(edit win, settings)`, and targeted setters/getters where hot-path convenience matters.
    - add cursor grab/capture API as `CursorGrabMode` (`Free`, `Confined`, `Locked`), cursor icon enum, cursor-position set/query, and transparent/theme/IME query hooks.
  - Add a new low-level `std.text_input` domain.
    - own `TextInputSettings` and composition-target records.
    - expose `settings(read win)`, `apply_settings(edit win, settings)`, `set_composition_area(edit win, rect)`, and `clear_composition_area(edit win)`.
    - this is the reusable substrate for future UI/text grimoires; it does not own text rendering or editing policy.
  - `std.events` expands additively.
    - keep existing `TextInput` as the committed-text event for compatibility.
    - add `TextCompositionStarted`, `TextCompositionUpdated`, `TextCompositionCommitted`, and `TextCompositionCancelled`.
    - extend `KeyEvent` metadata with physical key, logical key/text, location, modifiers, and repeat state without removing the current code-based lane.
  - `std.input` stays low-level but grows parity helpers.
    - keep current frame queries.
    - add helpers for logical vs physical key interpretation and key location constants; do not add shortcut/action routing here.

- Rebuild the Windows runtime/backend around those contracts.
  - Buffered host:
    - model the expanded settings/state records, cursor grab modes, text-input settings, and composition event flow deterministically.
    - keep it strict enough that future UI/settings grimoires can rely on it for substrate behavior tests.
  - Native Windows host:
    - implement window settings live where Win32 supports live mutation: min/max constraints, transparency, theme override state, cursor visibility/icon, cursor grab/confine/lock, cursor reposition, IME enablement, and composition target rect.
    - implement text-input/IME v1 through Win32 IME APIs rather than fake `WM_CHAR`-only behavior:
      - committed text remains on the committed-text path.
      - composition start/update/commit/cancel comes from native composition handling.
      - composition/candidate positioning is driven by the new composition-area hook.
    - expand keyboard metadata to produce both physical and logical key information for key events.
    - preserve the current corrected close-request semantics and deterministic session/window cleanup.

- Lift the substrate cleanly into `arcana_desktop`.
  - Add `arcana_desktop.text_input`.
  - Expand `arcana_desktop.types` with lifted settings/state records and enums that mirror the new std substrate.
  - `arcana_desktop.window` becomes the settings-facing window-shell module:
    - `default_config`, `open/open_cfg/open_in`, current state queries, and live `settings/apply_settings`.
    - targeted helpers remain for convenience, but records are the primary settings path for later settings UIs.
  - `arcana_desktop.input` exposes the richer key metadata helpers and stays free of shortcut policy.
  - `arcana_desktop.app` keeps the current callback model; new text/IME/window events flow through `window_event`, raw device events stay on `device_event`.
  - Do not add app/UI policy here: no text editor widgets, no shortcut manager, no retained UI framework.

- Update approved contracts and planning docs in the same patch series.
  - expand `app-substrate-v1-scope` to ratify the new window/cursor/text-input substrate.
  - add `STD-TEXT-INPUT` to std status and update `STD-WINDOW`, `STD-INPUT`, and `STD-EVENTS`.
  - update grimoire status and `desktopplan.md` so the remaining parity bar matches the real delivered surface.

## Bring-Up Order
1. Ratify the substrate additions on paper first: expanded `std.window`, additive `std.events`, expanded `std.input`, and new `std.text_input`.
2. Land the Arcana surface in `std` and `arcana_desktop` with typed records and additive APIs, keeping existing working calls source-compatible.
3. Implement buffered-host behavior for the new records/events/settings so the runtime path and tests are stable before the native host grows.
4. Implement native Windows window/cursor/theme settings and cursor grab/capture/reposition.
5. Implement native keyboard metadata plus committed text and full IME composition flow.
6. Lift and verify the final `arcana_desktop` app-shell API over the new substrate.
7. Close with native bundle proofs and doc/status updates in the same series.

## Test Plan
- Low-level runtime tests:
  - `WindowSettings` and `CursorSettings` roundtrip through buffered host.
  - min/max constraint state, transparency/theme override state, cursor grab mode, cursor icon, and cursor position query/set behavior.
  - additive key metadata: physical key, logical key/text, location, modifiers, repeat.
  - committed text and full composition flow: start, update, commit, cancel, composition-area updates.
- Native Windows tests:
  - `arcana_desktop` bundle can open a window, toggle minimize/maximize/fullscreen/resizable/topmost, and query/apply settings records.
  - native cursor visibility/icon/grab/reposition works through `arcana_desktop`.
  - native text-input/IME bundle proves committed text plus composition events and composition-area updates.
  - existing redraw/wake/clipboard/multi-window bundle proofs stay green.
- Acceptance apps:
  - one non-ECS native sample through `arcana_desktop` only that:
    - opens a window,
    - changes runtime settings,
    - receives committed text and composition events,
    - prints logical/physical key info,
    - and exits cleanly.
  - one small settings-oriented sample that round-trips `WindowSettings`, `CursorSettings`, and `TextInputSettings` with `arcana_desktop` as the primary app-shell API and any direct `std.*` use limited to explicit substrate checks.

## Assumptions And Defaults
- Windows is still the only required backend target for this bring-up; the public Arcana API remains platform-agnostic.
- `TextInput` remains the committed-text event for compatibility; composition is added through new events rather than renaming existing surface.
- `std.text_input` is introduced as a new bootstrap-required low-level domain.
- Typed records are the primary settings surface; individual setters/getters remain only where they materially improve ergonomics.
- Shortcut routing, keybind maps, text editing controls, and settings-screen UX stay out of this bring-up and belong in later app/UI grimoires.
- Touch, gamepad, tablet, and other non-core desktop-device work are out of scope unless implementation proves a concrete blocker to the desktop parity target above.
