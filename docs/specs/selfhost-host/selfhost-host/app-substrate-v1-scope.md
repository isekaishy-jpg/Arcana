# Arcana First-Party App Substrate v1 Scope

This scope freezes the rewrite-owned first-party app/runtime substrate required before selfhost.

Scope notes:
- This file covers the app-facing first-party substrate that sits above host-core packages and below consumer grimoires.
- The rewrite is expected to own the backend/runtime boundary for this substrate. Historical MeadowLang desktop/event corpus is consumer/reference context, not implementation authority.
- The substrate defined here is a real pre-selfhost Rust runtime commitment of the rewrite. It is not a placeholder promise to keep using imported library layers until after bootstrap.
- Third-party Rust crates may be used under this substrate only as replaceable implementation details; the public substrate contract, semantics, diagnostics, and resource model must remain Arcana-owned.
- ECS scheduling/components remain first-party language/runtime surface. They are not demoted to showcase-only helpers.
- Carried convenience modules such as `std.app` fixed-step helpers and `std.tooling` planning helpers are not ratified here; they may be rebuilt, relocated, or dropped unless later approved by an explicit scope.
- Higher-level desktop loop, routing, and audio policy belongs in Arcana-owned grimoire layers above this substrate, not in the substrate itself.
- Current source-declared opaque resource handles in this substrate are binding-owned seams reexported by public grimoires, not rewrite-owned runtime handle families.
- Any future handle-model review must preserve Arcana's explicit/unambiguous doctrine: typed resource families, explicit validity/ownership rules, deterministic diagnostics, and no erased catch-all runtime handle.
- Runtime/backend work must not keep parallel family-alias machinery for window/session/frame/wake, file-stream, or audio handles once the canonical binding-owned declarations exist.

## Included
- `arcana_desktop.window`:
  - `open -> Result[Window, Str]`, `open_cfg -> Result[Window, Str]`, `open_in(edit session, cfg) -> Result[Window, Str]`, `alive`
  - `id`, `title`, `size`, `position`, `visible`, `decorated`, `resizable`, `topmost`, `cursor_visible`, `min_size`, `max_size`, `scale_factor_milli`, `theme`, `theme_override`, `transparent`, `cursor_icon`, `cursor_grab_mode`, `cursor_position`, `text_input_enabled`, `current_monitor`, `primary_monitor`, `monitor_count`, `monitor`, `resized`, `fullscreen`, `minimized`, `maximized`, `focused`
  - explicit config/settings records for title, bounds, style, state, cursor policy, and text-input enablement
  - `request_redraw`
  - `set_title`, `set_position`, `set_visible`, `set_decorated`, `set_resizable`, `set_min_size`, `set_max_size`, `set_fullscreen`, `set_minimized`, `set_maximized`, `set_topmost`, `set_cursor_visible`, `set_transparent`, `set_theme_override`, `set_cursor_icon`, `set_cursor_grab_mode`, `set_cursor_position`, `set_text_input_enabled`, `request_attention`, `close -> Result[Unit, Str]`
  - low-level window config records remain explicit and substrate-shaped: title, logical size/position, visibility, min/max constraints, resizable/decorated/transparent style, topmost/maximized/fullscreen plus theme-override state, cursor policy, and text-input enablement
- `arcana_desktop.events`:
  - `poll(edit frame) -> Option[arcana_desktop.types.AppEvent]`, `drain(take frame) -> List[arcana_desktop.types.AppEvent]`
  - `open_session -> Session`, `attach_window`, `detach_window`, `pump_session(edit session) -> FrameInput`, `wait_session(edit session, timeout_ms) -> FrameInput`
  - `create_wake(edit session) -> WakeHandle`, `wake(read handle)`
  - typed app/window/device event queue surface sourced from the same frame pump boundary
  - current low-level event floor includes app resumed/suspended/about-to-wait plus wake notifications, resize, move, close-request, focus, redraw-request, DPI/theme change, key, mouse-button, mouse-move, mouse-wheel, pointer-enter/leave, committed text-input, text-composition start/update/commit/cancel, raw mouse-motion, and file-drop events
  - key events carry low-level key metadata including physical key, logical key/text, location, modifiers, and repeat state
  - file-drop events carry external filesystem paths only; they do not imply packaging or asset ingestion
- `arcana_desktop.input`:
  - `key_code`
  - `key_down`, `key_pressed`, `key_released` on `FrameInput`
  - `mouse_button_code`, `mouse_pos`, `mouse_down`, `mouse_pressed`, `mouse_released`, `mouse_wheel_y`, `mouse_in_window` on `FrameInput`
- `arcana_desktop.text_input`:
  - `enabled(read win)`, `set_enabled(edit win, enabled)`
  - `composition_area(read win)`, `settings(read win)`, `apply_settings(edit win, settings)`
  - `set_composition_area(edit win, area)`, `clear_composition_area(edit win)`
  - explicit low-level text-input settings stay host-facing: enablement plus composition target state only, with no text-editing or widget policy
- `arcana_desktop.clipboard`:
  - `read_text -> Result[Str, Str]`, `write_text -> Result[Unit, Str]`
  - `read_bytes -> Result[Bytes, Str]`, `write_bytes -> Result[Unit, Str]`
  - clipboard stays low-level and host-facing: plain text plus raw bytes only, with explicit operation-local failure transport
- `arcana_graphics.arcsb`:
  - `Context`, `Surface`, `Buffer`, `Rect`, `AlphaMode`
  - `new_context`, `new_surface`, `configure`, `resize`, `supports_alpha_mode`, `next_buffer`, `present`, `present_with_damage`
  - `new_surface` consumes the canonical binding-owned window handle type `arcana_winapi.desktop_handles.Window`
  - `Buffer.pixels` is the mapped-byte backend view surface for low-level software presentation
- `std.time`:
  - monotonic time points and durations
  - low-level sleep/frame-timing primitives
- `arcana_audio`:
  - low-level audio device, buffer, and playback substrate
  - device info/config hooks: `default_output -> Result[AudioDevice, Str]`, `output_close -> Result[Unit, Str]`, `output_sample_rate_hz`, `output_channels`, `output_set_gain_milli`
  - buffer hooks: `buffer_load_wav -> Result[AudioBuffer, Str]`, `buffer_frames`, `buffer_channels`, `buffer_sample_rate_hz`
  - playback hooks: `play_buffer(edit device, read buffer) -> Result[AudioPlayback, Str]`, `stop -> Result[Unit, Str]`, `pause`, `resume`, `playing`, `paused`, `finished`, `set_gain_milli`, `set_looping`, `looping`, `position_frames`
  - low-level playback does not implicitly resample or remix; the current bootstrap lane requires `AudioBuffer` sample rate and channel count to match the selected `AudioDevice` config and returns `Result.Err(...)` otherwise
  - current bootstrap seam uses binding-owned opaque audio handles reexported by `arcana_audio` plus explicit failure results for device/buffer/playback acquisition
- Primitive graphics/text support sufficient for real apps/showcases:
  - stable per-frame window/input/event semantics with an explicit frame boundary
  - low-level software-buffer presentation through `arcana_graphics.arcsb`
- ECS/runtime surface required before selfhost:
  - `behavior[...] fn`
  - `system[...] fn`
  - `std.behaviors.step`
  - `std.ecs` phase helpers plus singleton/entity/component helpers

## Excluded
- `std.app` fixed-step helpers as rewrite-defining app architecture.
- `std.tooling` local planning helpers as rewrite-defining standard surface.
- Game/showcase-specific convenience helpers that leaked into imported `std`.
- Imported `winspell` / `spell-events` layering as implementation authority.
- Higher-level desktop app-shell grimoire responsibilities, event routing helpers, and audio playback convenience policies.
- Historical public std desktop shell modules retained only as internal compatibility shims during migration.
- Meadow-era `winit`, VM, or bytecode coupling assumptions.
- General ECS query authoring beyond the already frozen language/runtime baseline.
- Automatic long-term ratification of duplicate public handle declarations or runtime-side handle-family aliasing.
