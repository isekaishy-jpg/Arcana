# Combined Windows Desktop Migration and `arcsb` Backend Rebase

## Summary

- Do one combined migration with `arcana_desktop` first on the critical path, then wire `arcana_graphics.arcsb` onto that final desktop contract in the same pass.
- Rebuild `arcana_desktop` as the Windows winit-class public shell plus one extra: clipboard.
- Move the current runtime/std desktop debt into grimoires:
  - `arcana_desktop` becomes the public shell
  - `arcana_winapi` becomes the Windows binding substrate
- Keep `arcana_graphics.arcsb` as a reusable graphics backend module, publicly available as `arcana_graphics.arcsb`, implemented as a folder inside the graphics grimoire.
- Retire the old canvas-era stack now:
  - `std.window`
  - `std.events`
  - `std.input`
  - `std.clipboard`
  - `std.canvas`
  - `arcana_desktop.canvas`
  - `arcana_desktop.loop`
  - `arcana_desktop.ecs`
- Do not add a new showcase proof app in this pass. Retire the old proofs and replace them with targeted runtime/package smoke coverage. A new proof comes later.
- Do not implement `iced` or D2D here. This plan only lands the backend shape those later ports will build on.

## Key Changes

### Specs and Public Contracts
- Update approved specs so the public desktop shell is `arcana_desktop`, not `std.*`.
- Update approved specs so the old low-level `std.canvas` substrate is retired rather than preserved.
- Update graphics-role docs so `arcana_graphics` is backend-ready and may host multiple rendering backends; `arcsb` is the first one.
- Update OS-binding/docs so `arcana_winapi` is the binding-owned Windows substrate for desktop and graphics, not just helper glue.

### `arcana_desktop`
- Keep only the winit-class shell surface:
  - `arcana_desktop.app`
  - `arcana_desktop.window`
  - `arcana_desktop.events`
  - `arcana_desktop.input`
  - `arcana_desktop.monitor`
  - `arcana_desktop.text_input`
  - `arcana_desktop.clipboard`
  - `arcana_desktop.types`
- Remove from the public desktop surface:
  - `canvas`
  - `loop`
  - `ecs`
- Preserve the current app/session runner role:
  - app lifecycle callbacks
  - session/window orchestration
  - wake/mailbox support
  - window-ID-centric event routing
- Keep desktop clipboard as the only non-winit extra.
- Add the public raw/native-handle seam on the desktop side, since that is the winit-equivalent contract graphics backends should consume.
- Make `arcana_desktop` depend on `arcana_winapi` for the Windows implementation instead of routing through `std.kernel.gfx` and related runtime special cases.

### `arcana_winapi`
- Expand the Windows binding substrate to own the real desktop implementation seam:
  - app/session runner hooks
  - wake/event pump hooks
  - window create/open/close/configure/state
  - window event and input translation
  - monitor/theme/cursor/text-input hooks
  - raw/native handle export
  - clipboard operations
- Keep `arcana_winapi` as the only Win32 binding owner.
- Keep `raw.*` for types/layouts/imports and `helpers.*` for thin consumable helper routines.
- Keep `helpers.graphics` for reusable software-surface ownership/present logic.
- Remove the need for runtime-owned handwritten public window/input/canvas/clipboard host lanes.

### `arcana_graphics` and `arcsb`
- Keep the public backend path as `arcana_graphics.arcsb` so other grimoires can depend on it later.
- Replace the flat `src/arcsb.arc` with a folder module under `grimoires/libs/arcana-graphics/src/arcsb/`.
- Keep `arcsb` raw and softbuffer-shaped:
  - `Context`
  - `Surface`
  - `Buffer`
  - `Rect`
  - `AlphaMode`
  - `new_context`
  - `new_surface`
  - `configure`
  - `resize`
  - `supports_alpha_mode`
  - `next_buffer`
  - `present`
  - `present_with_damage`
- Switch `arcsb.new_surface` to the final canonical window contract: `arcana_winapi.desktop_handles.Window`.
- Keep `Buffer.pixels` as `View[U8, Mapped]`.
- Keep all source-level policy in `arcsb`, not in WinAPI helpers:
  - configured-state tracking
  - alpha validation
  - damage coalescing
  - one-live-buffer rule
  - cleanup semantics
  - surface/buffer ownership checks
- Remove canvas-era graphics wrappers and types that do not port from softbuffer:
  - `arcana_graphics.canvas`
  - `arcana_graphics.images`
  - `arcana_graphics.color`
  - `arcana_graphics.paint`
  - canvas-spec helper records tied only to that wrapper path
- Leave `arcana_graphics` as the package that will later host:
  - `arcsb` backend
  - later `iced_graphics` work over that backend
  - later D2D backend work
- Do not pre-scaffold future backend modules beyond the `arcsb/` folder in this pass.

### `std` / runtime / kernel retirement
- Retire the old public std desktop shell APIs:
  - `std.window`
  - `std.events`
  - `std.input`
  - `std.clipboard`
  - `std.canvas`
- Remove the corresponding runtime/kernel public lanes that only existed to serve those APIs.
- Remove or collapse `std.kernel.gfx` and any sibling kernel modules that were only the old desktop/canvas bridge.
- Keep runtime focused on generic host/runtime concerns and binding support, not as the public desktop shell.
- Update CLI/package/runtime fixtures, tests, docs, and generated samples so they no longer import retired `std.*` desktop/canvas APIs.

## Test Plan

- Desktop substrate tests:
  - app run/resume/suspend/exit
  - wake/mailbox behavior
  - open/close window
  - multi-window routing by window ID
  - monitor and window settings queries
  - cursor/theme/text-input and IME lifecycle behavior
  - clipboard read/write
- `arcsb` backend tests:
  - create surface from `arcana_winapi.desktop_handles.Window`
  - configure and resize
  - `next_buffer`
  - write mapped pixels
  - full present
  - bounded-damage present
  - cleanup of abandoned buffers
  - wrong-surface and unsupported-alpha rejection
- Combined native smoke:
  - live desktop window created through the migrated desktop shell
  - `arcana_graphics.arcsb` presents successfully on that window
  - packaged Windows bundle smoke for the same path
- Cleanup gates:
  - no active references to retired `std.window` / `std.events` / `std.input` / `std.clipboard` / `std.canvas`
  - no active references to `arcana_desktop.canvas`, `arcana_desktop.loop`, or `arcana_desktop.ecs`
  - no old canvas-era proof apps left in green gates

## Assumptions

- Windows is the only required platform in this pass.
- `arcana_desktop` stays source-only; `arcana_winapi` stays the binding owner.
- Desktop is a winit-class shell plus clipboard only.
- No immediate draw/image replacement is introduced here; canvas-era drawing APIs are retired.
- No new showcase proof app is required in this pass; a new proof is created later.
- `arcana_text` is not rebuilt here; if it depends on retired canvas-era surfaces, remove it from active gates rather than porting it in this plan.
- Later work will port `iced_graphics` over the new graphics backend shape and add a Direct2D backend separately.
