# Windows `winit` / `softbuffer` Port Plan

**Summary**

- Rebuild `arcana_desktop` and `arcana_graphics.arcsb` as Arcana-owned Windows ports of the current upstream core models, using `winit` 0.30.x and `softbuffer` 0.4.x as the semantic reference, not as runtime dependencies.
- Treat this as an intentional public API/spec break. Update the frozen substrate contract in `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md` in the same change as the code.
- Keep Arcana-only conveniences in the same packages, but move them above the ported core so they are clearly helpers, not the defining contract.
- Windows is the only implementation target in this pass. `arcana_winapi` remains the Win32 seam. The public behavior should still follow the portable upstream core model wherever possible.

**Public Contract Changes**

- Replace the current desktop core with a `winit`-style event-loop contract: an `EventLoop[T]` / active-loop context equivalent, typed event proxy support, and `Application` callbacks equivalent to `new_events`, `resumed`, `user_event`, `window_event`, `device_event`, `about_to_wait`, `suspended`, `exiting`, and `memory_warning`.
- Replace the current core `wake` callback with typed `user_event`. If a zero-payload wake helper is kept, it becomes an Arcana convenience built on the proxy path, not a separate core event model.
- Replace core window dispatch from `TargetedEvent` to upstream-style `window_id + WindowEvent` delivery. `TargetedEvent` and `is_main_window` may remain only as derived helpers above the core.
- Keep `ControlFlow` aligned with upstream `Poll`, `Wait`, and `WaitUntil`, and make the Windows runner deliver lifecycle ordering that matches the upstream model, including real `new_events` and `about_to_wait` boundaries.
- Keep `arcana_desktop.input`, monitor, clipboard, and text-input helpers, but define them as derived host helpers layered over the ported event loop. They must not redefine the event model.
- Replace `arcana_graphics.arcsb.new_context()` with a display-bound constructor aligned to `softbuffer` semantics. The constructor should take the desktop/display seam rather than being a zero-arg singleton.
- Keep `Context`, `Surface`, `Buffer`, and `Rect`, but change the graphics core to `softbuffer`-style buffer acquisition: `buffer_mut` semantics, public pixel data as `u32` pixels in row-major `0x00RRGGBB` format, and no public byte-stride contract.
- Remove mapped-byte `Buffer.pixels: View[U8, Mapped]` from the public contract. If Arcana needs an explicit view type, use a `U32` view equivalent instead.
- Keep `present` and `present_with_damage`, with damage rectangles using the same public coordinate semantics as `softbuffer`.
- Do not preserve `open_session` / `pump_session` / `wait_session` / frame-drain polling as the primary public contract. If compatibility helpers survive, they live in a clearly secondary compatibility layer and are not the frozen core.

**Implementation Changes**

- Update the substrate/spec layer first so the approved contract stops freezing the old session/frame API and mapped-byte graphics buffer model.
- Rework `grimoires/libs/arcana-desktop` around a true Windows event-loop runner that copies `winit` lifecycle semantics, window routing, proxy-driven user events, and redraw ordering.
- Keep the existing package/module ownership, but reorganize the code so the ported event loop is the foundation and helpers like mailbox, main-window caching, window-opening shortcuts, and input snapshots sit above it.
- Make the Windows backend deliver native behavior for setters that are currently state-only, including decorations, resizable, topmost, transparency, theme override, cursor icon/grab, request-attention, and IME composition-area behavior.
- Collapse IME/text input into a `winit`-style event model at the core and keep higher-level text-input helpers as wrappers over that model.
- Rework `grimoires/libs/arcana-graphics` so the public API copies `softbuffer` behavior while the Windows backend still uses Arcana-owned Win32/GDI internals.
- Replace the current single-map / byte-stride presentation path with a buffer model that exposes contiguous `u32` pixels publicly and hides mapping details internally.
- Implement real buffer-age tracking with at least double-buffer semantics on Windows so `age` is meaningful for partial redraws, instead of the current boolean-style `0/1` approximation.
- Preserve `present_with_damage` publicly even if the Win32 backend internally unions damage rectangles before blitting. Public semantics must stay list-based and correct.
- Migrate the proof app and packaging tests to the new core APIs rather than preserving the old callback signatures.

**Test Plan**

- Update and keep the Windows native bundle tests in `crates/arcana-cli` passing against the new desktop API, including single-window, multi-window, clipboard, settings, and IME/text-input coverage.
- Add lifecycle-order tests for `new_events`, `resumed`, `user_event`, `window_event`, `about_to_wait`, `suspended`, and `exiting`, including `ControlFlow.WaitUntil` behavior.
- Add proxy/user-event tests proving cross-thread wake or payload delivery routes through the new typed event path.
- Add window-behavior tests for redraw requests, close requests, main-window helper correctness, and the setters that currently only mutate stored state.
- Add graphics tests for `u32` pixel layout, resize behavior, `buffer_mut` acquisition, age semantics across multiple presents, and `present_with_damage`.
- Keep the `examples/arcana-desktop-proof/arcsb_app` workspace as the end-to-end acceptance proof and require it to package and run through the real `windows-exe` path after the port.

**Assumptions And Defaults**

- Upstream crates are reference behavior only. Do not wrap `winit` or `softbuffer`, and do not make them required runtime dependencies.
- Windows is the only implementation target in this pass; public semantics should still track the portable upstream core where practical.
- `arcana_winapi` remains the Win32 binding seam for handles and backend calls in this phase; do not invent a second Windows binding layer.
- Package names stay the same: `arcana_desktop` and `arcana_graphics.arcsb` remain the public owners.
- Public breakage is allowed across source packages, the frozen substrate scope, the proof app, README wording, and existing CLI desktop tests if that is required to reach close `winit` / `softbuffer` parity.
