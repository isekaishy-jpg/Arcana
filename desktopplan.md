# Plan: Build `arcana_desktop` to Near-Full Desktop Parity

## Current Status
- Delivered in the current rewrite-owned lane:
  - session-backed `arcana_desktop.app.run`
  - blocking session wait plus configurable wait slicing
  - multi-window open/attach flow
  - wake handles and typed mailboxes
  - monitor queries, theme reporting, clipboard, redraw, attention requests
  - expanded window settings: min/max size, transparency, theme override, cursor icon/grab/position, and text-input enablement
  - low-level text-input settings plus composition target state
  - richer key metadata plus text-input, composition, file-drop, raw mouse-motion, and theme/DPI event transport
  - optional ECS adapter helpers
  - native Windows bundle proof through `arcana_desktop`, including graphics/text, multi-window clipboard, settings/text-input roundtrip smoke, create-time cursor/text-input config proof, and committed-text plus IME start/cancel smoke on the real host message path
- Remaining parity backlog before this plan is fully closed:
  - deterministic native bundle automation for live IME composition-update payloads beyond the current real committed-text/start-cancel bundle smoke and in-process native committed-composition proof
  - any additional desktop-device parity that future grimoires prove belongs in shared substrate rather than higher-level policy

## Summary
- Replace the current `arcana_desktop` scaffold with a real desktop app-shell grimoire that reaches near-full winit-style parity on a Windows-first, platform-agnostic Arcana API.
- Keep the app-shell grimoire Arcana-idiomatic: a statically-dispatched app runner with explicit session/window state, no dynamic dispatch, no hidden globals.
- Treat `arcana_desktop` as the canonical public desktop boundary, not as a thin ergonomic wrapper over separately-public raw app-shell APIs in `std.*`.
- Keep rendering policy out of `arcana_desktop`: it owns the app shell, multi-window orchestration, input/event/run-loop/redraw lifecycle, monitors, clipboard, wake/proxy, and timing; actual draw convenience stays in `arcana_graphics` and `arcana_text`.
- Land any missing low-level capability in reusable `std` substrate first, not as desktop-only special cases.

## Public APIs And Types
- Expand the low-level app substrate with a hybrid session model.
  - Keep the current simple per-window pump path for compatibility.
  - Add a low-level session/event-loop handle for broad parity work: session creation, multi-window coordination, wake/proxy, and advanced lifecycle features all target the session path.
- Expand `std.window` as the low-level window/monitor surface.
  - Add session-aware window creation and explicit window IDs.
  - Expand `WindowConfig` to cover the full broad-core builder shape: title, logical size/position, min/max size, visibility, resizable/decorated/transparent/topmost/maximized/fullscreen policy, IME enablement, theme override, and cursor policy.
  - Add reusable low-level queries and controls for position, scale factor/DPI, current monitor, primary monitor, monitor enumeration, redraw request, cursor mode/icon/visibility, and attention/focus state.
- Expand `std.events` as the low-level event transport.
  - Add session-scoped frame pumping and a wake/proxy primitive.
  - Expand the event families to include app lifecycle, window lifecycle, redraw, DPI/theme, keyboard/mouse, text input/IME, drag-and-drop, raw device input, and wake notifications.
  - File-drop events carry external filesystem paths only; they do not imply packaging or asset ingestion.
- Expand `std.input` as the low-level input helper surface.
  - Keep state queries low-level and reusable.
  - Add broader named key/button coverage, modifier state, logical vs physical key data, and raw device helper access where the event surface needs it.
- Add a new low-level `std.clipboard` domain.
  - Text and raw bytes read/write only, with explicit `Result[...]` failure transport.
  - No higher-level history, selection policy, or UI affordances.
- Build the real `arcana_desktop` app-shell package on top of that substrate.
  - Modules: `app`, `window`, `events`, `input`, `monitor`, `clipboard`, `loop`, and optional `ecs`.
  - Primary runner contract: a generic `Application` trait family with static dispatch only.
  - Fixed runner callback set: `resumed`, `suspended`, `window_event`, `device_event`, `about_to_wait`, `wake`, and `exiting`.
  - `redraw_requested` is delivered as part of the window-event family, not as a separate render framework.
  - Core public types: `AppConfig`, `AppContext`, `ControlFlow` (`Poll`, `Wait`, `WaitUntil`), `WindowId`, expanded `WindowConfig`, `InputSnapshot`, `WakeHandle`, and typed `Mailbox[T]`.
  - User-event support uses wake + mailbox, not a generic std-level typed event-loop payload model.
  - The optional ECS adapter is a separate layer in the same grimoire, not the default runner contract.

## Implementation Changes
- Substrate and backend
  - Introduce a low-level session handle in the runtime/backend and mirror it in the buffered synthetic host and the Windows native host.
  - Replace the current fixed `(kind, a, b)` internal event transport with a structured internal event record that can carry strings, paths, text/IME payloads, DPI/theme data, and raw-device payloads.
  - Implement Windows host support for multi-window session ownership, monitor/DPI queries, theme reporting, cursor modes/icons, clipboard, drag-and-drop, text input/IME, raw device input, and wake/proxy behavior.
  - Preserve the current simple per-window pump as a compatibility path, but route all new broad-parity desktop app-shell work through the session model.
- Desktop grimoire architecture
  - Rebuild `arcana_desktop` around the session runner as the canonical public app-shell boundary instead of thin wrapper modules.
  - Use the object/owner model internally for session lifetime, live window ownership, mailbox ownership, and deterministic cleanup.
  - Keep the core runner ECS-agnostic; add a separate adapter that steps `std.behaviors` / `std.ecs` from the same app loop.
  - Provide redraw orchestration and frame lifecycle only; do not absorb `arcana_graphics` / `arcana_text` drawing policy.
- Docs and contract updates
  - Update the grimoires scope/status, app-substrate scope, std status, and roadmap entries in the same patch series.
  - If `std.clipboard` is introduced as a new public low-level domain, add its approved scope/status entry in the same series.
  - Do not reopen language syntax or semantics for this work.

## Test Plan
- Low-level runtime tests
  - Session creation, multi-window pumping, wake/proxy signaling, clipboard roundtrip, richer key/button mapping, and structured event decode.
  - Synthetic host coverage for DPI/theme changes, drag-drop, text input/IME, raw device input, and wake delivery.
- Native Windows tests
  - Real native bundle smokes through `arcana_desktop`, not only direct `std.*` calls.
  - At minimum: multi-window open/close, redraw request flow, clipboard use, file-drop event delivery, IME/text input delivery, raw mouse/device input, theme/DPI change handling, and background wake/mailbox delivery.
- App-shell acceptance tests
  - One non-ECS sample app using `arcana_desktop.app.run` plus `arcana_graphics` / `arcana_text`.
  - One optional ECS-backed sample using the adapter layer.
  - Completion bar: at least one native Windows sample app runs through `arcana_desktop` as the canonical public desktop/app-shell API, with `std.window` / `std.events` / `std.input` remaining substrate/runtime support rather than a required parallel shell API for apps.

## Assumptions And Defaults
- Windows is the only required backend/runtime implementation target in this plan; the public Arcana API must stay platform-agnostic.
- The desktop app-shell grimoire owns the app shell only; graphics/text convenience stays in the separate grimoires.
- The hybrid session model is the long-term substrate direction; the old per-window pump remains only as a simple compatibility lane.
- User events are delivered through typed mailboxes plus wake handles, not a generic typed std event-loop proxy.
- External drag-and-drop/media inputs remain external paths/data; nothing in this plan changes packaging rules for external assets.
