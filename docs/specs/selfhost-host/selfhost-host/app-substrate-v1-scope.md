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
- Current source-declared opaque resource handles in this substrate are bootstrap-approved seams, not a promise that the long-term rewrite-owned runtime resource model will keep the exact same public handle shape after bootstrap.
- Any future handle-model review must preserve Arcana's explicit/unambiguous doctrine: typed resource families, explicit validity/ownership rules, deterministic diagnostics, and no erased catch-all runtime handle.

## Included
- `std.window`:
  - `open -> Result[Window, Str]`, `alive`
  - `size`, `resized`, `fullscreen`, `minimized`, `maximized`, `focused`
  - `set_title`, `set_resizable`, `set_fullscreen`, `set_minimized`, `set_maximized`, `set_topmost`, `set_cursor_visible`, `close -> Result[Unit, Str]`
- `std.input`:
  - `key_code`
  - `key_down`, `key_pressed`, `key_released` on `AppFrame`
  - `mouse_button_code`, `mouse_pos`, `mouse_down`, `mouse_pressed`, `mouse_released`, `mouse_wheel_y`, `mouse_in_window` on `AppFrame`
  - named key/button lookup includes the common modifier, navigation, function-key, and auxiliary-mouse families needed by future grimoires while staying low-level
- `std.canvas`:
  - bootstrap compatibility wrappers `open -> Result[Window, Str]`, `alive`
  - `fill`, `rect`, `rect_draw`, `line`, `line_draw`, `circle_fill`, `circle_fill_draw`
  - `label`, `label_draw`, `label_size`, `present`, `rgb`
  - `image_load -> Result[Image, Str]`, `image_size`, `blit`, `blit_scaled`, `blit_region`
  - current bootstrap seam uses source-declared opaque `Window` and `Image` handles plus explicit failure results for resource-creation/load boundaries
  - primitive draw records `RectSpec`, `LineSpec`, `CircleFillSpec`, and `LabelSpec`
- `std.events`:
  - `poll(edit frame) -> Option[std.events.AppEvent]`, `drain(take frame) -> List[std.events.AppEvent]`, `pump(edit win) -> AppFrame`
  - typed `std.events.AppEvent` queue surface sourced from the same frame pump boundary, with polling defined by a single backend event record per step rather than separate kind/payload probes
  - current low-level event floor includes resize, move, close-request, focus, key, mouse-button, mouse-move, mouse-wheel, and pointer-enter/leave events
  - `AppFrame` is a move-only source-declared opaque per-step frame handle shared by `std.events` and `std.input`
  - edge-triggered input state is advanced explicitly by `std.events.pump`, which mutates frame state through `edit win`
  - `std.events.poll` consumes one event from `AppFrame`, and `std.events.drain` consumes the remaining frame-local queue instead of rereading it
- `std.time`:
  - monotonic time points and durations
  - low-level sleep/frame-timing primitives
- `std.audio`:
  - low-level audio device, buffer, and playback substrate
  - device info/config hooks: `default_output -> Result[AudioDevice, Str]`, `output_close -> Result[Unit, Str]`, `output_sample_rate_hz`, `output_channels`, `output_set_gain_milli`
  - buffer hooks: `buffer_load_wav -> Result[AudioBuffer, Str]`, `buffer_frames`, `buffer_channels`, `buffer_sample_rate_hz`
  - playback hooks: `play_buffer(edit device, read buffer) -> Result[AudioPlayback, Str]`, `stop -> Result[Unit, Str]`, `pause`, `resume`, `playing`, `paused`, `finished`, `set_gain_milli`, `set_looping`, `looping`, `position_frames`
  - low-level playback does not implicitly resample or remix; the current bootstrap lane requires `AudioBuffer` sample rate and channel count to match the selected `AudioDevice` config and returns `Result.Err(...)` otherwise
  - current bootstrap seam uses source-declared opaque audio handles plus explicit failure results for device/buffer/playback acquisition
- Primitive graphics/text support sufficient for real apps/showcases:
  - solid fills
  - rectangle draw
  - line draw
  - filled circle draw
  - label/text draw
  - basic text measurement for layout
  - image load/size/blit
  - stable per-frame window/input/event pump semantics with an explicit frame boundary
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
- Higher-level desktop app facade, event routing helpers, and audio playback convenience policies.
- Meadow-era `winit`, VM, or bytecode coupling assumptions.
- General ECS query authoring beyond the already frozen language/runtime baseline.
- Automatic long-term ratification of the current typed opaque app/runtime handle model.
