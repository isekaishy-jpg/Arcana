# Rebuild `arcana_desktop` to winit-class parity with an owner-driven lifecycle core

**Summary**
- Rebuild `arcana_desktop` as Arcana’s authoritative public desktop/app-shell package, targeting winit-class feature parity rather than wrapper-level convenience.
- Keep `std.window`, `std.events`, `std.input`, `std.text_input`, and `std.clipboard` public as substrate domains, but demote them to host-facing support APIs instead of the normal app boundary.
- Make the change fully breaking. Remove the current dual-lane/public-wrapper shape and replace it with one desktop-owned contract used by first-party apps, tests, graphics, text, and bundle smokes.
- Treat the historical multi-window stack overflow as a required closure item. The rebuilt runner must be iterative and phase-driven so multi-window operations cannot recurse through open/close/attach/detach/redraw paths.
- Use Arcana’s object/owner model where it adds real value: as the lifecycle backbone for session, live-window, wake, mailbox, and cleanup machinery, with selective public exposure only where it materially improves semantics.

**Public Contract**
- `arcana_desktop` owns the public shell types: `Window`, `Session`, `WakeHandle`, `WindowId`, `AppEvent`, `WindowEvent`, `DeviceEvent`, `MonitorInfo`, `WindowConfig`, `WindowSettings`, `TextInputSettings`, `InputSnapshot`, `ControlFlow`, and the fixed-step/ECS types.
- `arcana_desktop.app.Application[...]` keeps the static runner model, but every callback uses desktop-owned types only. No exported desktop callback, context field, or helper may require `std.events.*` or `std.window.Window`.
- `arcana_desktop` covers the full shell floor expected of a winit analogue: lifecycle/control flow, session/event-loop ownership, multi-window management, wake delivery, window creation/state/configuration, monitor/DPI/theme reporting, keyboard/mouse/raw-device input, text input/IME, clipboard, drag/drop, redraw flow, and blocking wait semantics.
- Do not expose a general raw-substrate or Rust-handle escape hatch as the normal public API. If first-party interop needs a bridge, keep it narrow and deliberate.
- Do not force maximal owner-activation syntax onto app code. Public owner/object usage is allowed only where it makes the desktop lifecycle clearer than the current function/record shape.

**Implementation Changes**
- Replace the current lift-and-forward package shape with a desktop-owned contract over an internal substrate adapter layer. Translate `std.*` handles and event records once at the boundary, then keep the rest of the package in desktop-owned types.
- Build the runner around an owner-driven lifecycle core:
  - one session owner for the active desktop run
  - owned objects for live window state, wake state, mailbox state, and deferred structural mutations
  - deterministic cleanup on exit and on non-main-window close
  - no callback path may directly recurse into session pumping or structural window mutation
- Use queued structural actions for open/close/attach/detach/request-redraw/control-flow changes so callback execution, mutation application, and event pumping happen in stable phases.
- Rebuild `RuntimeContext`, target lookup, current-window helpers, multi-window open/close, event routing, wake/mailbox flow, and redraw helpers so they operate only on desktop-owned handles and event families.
- Change event delivery from desktop callbacks receiving `std.events.AppEvent` to explicit desktop-owned event families with one authoritative mapping layer from substrate events to desktop events.
- Fix the known behavioral bugs inside the rebuild:
  - mailbox FIFO ordering
  - interactive second-window repaint invalidation
  - fixed-step zero-tick spin when `tick_hz > 1000`
  - historical multi-window stack overflow / recursive re-entry failure mode
- Audit substrate `std.*` domains and keep them host-shaped only. Leave low-level capability there, but move desktop policy, routing, app-runner behavior, and convenience ownership fully into `arcana_desktop`.
- Harden the runtime/native host where the desktop contract depends on it: session bookkeeping, wake lifetime, attach/detach cleanup, window lookup after close, redraw scheduling, text-input/composition state, drag/drop delivery, raw input routing, and deterministic multi-window cleanup.
- Update `arcana_graphics` and `arcana_text` to consume the desktop-owned window or draw-target contract instead of `std.window.Window`. They are in scope for any changes needed to remove substrate leakage from normal desktop rendering.
- Rewrite first-party runtime fixture apps, CLI-generated test apps, and later the proof app so ordinary desktop code imports `arcana_desktop` plus graphics/text packages, not raw `std.*` shell APIs.

**Test Plan**
- Add desktop-layer runtime tests for session lifecycle, wake delivery, wait/poll behavior, target lookup, multi-window open/attach/redraw/close/reopen behavior, mailbox ordering, monitor/DPI/theme, clipboard, text input/IME, drag/drop, raw mouse/device input, and fixed-step clamping.
- Add explicit anti-recursion and anti-overflow coverage:
  - open a second window from callbacks repeatedly
  - close and reopen windows across multiple frames
  - request redraw and structural window mutation from callbacks without recursive pump
  - stress repeated multi-window attach/detach/open/close until the old stack-overflow pattern would have appeared
- Keep substrate tests for `std.*`, but add separate desktop tests proving the same capabilities through `arcana_desktop` alone.
- Update Windows bundle smokes so the desktop-owned API is exercised end-to-end for close-button exit, page navigation click, interactive second-window open/repaint/click, settings roundtrip, clipboard, and IME/text-input drive.
- Acceptance bar:
  - no first-party desktop consumer needs raw `std.*` shell APIs for ordinary work
  - `arcana_desktop` exports no raw substrate shell types in its normal public contract
  - multi-window flows are iterative and stack-safe
  - Windows native bundle smoke remains green through the desktop package

**Assumptions**
- Breaking changes are allowed; there is no need to preserve the old public API or maintain a long-term compatibility lane.
- Windows remains the first implementation target, but the public contract should stay platform-agnostic and shaped for full desktop parity.
- The object/owner feature should be used where it provides lifecycle clarity and cleanup guarantees, primarily in the desktop core, not forced across the entire public API.
- `arcana_graphics` and `arcana_text` are in scope for any changes needed to align them to the new desktop boundary.
- The checked-in proof app is a final migration target, not a blocker for the core contract rebuild.
