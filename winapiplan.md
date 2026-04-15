# Generic `arcana_winapi` Cleanup Plan

## Summary

- Rebuild `arcana_winapi` as a generic Win32-over-CABI substrate, not a desktop runtime, graphics policy layer, or app-shell helper bundle.
- Make the public boundary `raw Win32 + generic helpers`, with no canonical desktop/session/event-loop semantics in `winapi`.
- Optimize for a full `winapi` sweep, but keep the architecture centered on reusable substrate rules so `arcana_desktop`, `arcana_graphics`, and future grimoires can all build on the same base.
- Treat packaging tests as final smoke, not as the primary design driver.

## Public Surface Changes

- Keep `arcana_winapi.raw.*` as the near-direct Win32/CABI lane: native scalar types, structs, constants, imports, and raw handle types like `HWND`, `HANDLE`, `RECT`, `MSG`, `GUID`, and related Win32 records.
- Keep `arcana_winapi.helpers.*` only for generic helper wrappers that are still substrate-level:
  - window creation/destruction, style/state mutation, monitor/DPI queries, cursor, raw input, IME/text-input primitives, clipboard primitives, generic wake/wait/message-pump primitives
  - generic GDI/software-surface primitives
  - generic process/fs helpers
  - generic audio helpers
- Hard-remove the desktop-owned compatibility surface from the canonical `winapi` contract:
  - `desktop_handles.Session`
  - `helpers.events.session_*`
  - any public API whose meaning is “desktop runner state” rather than “Win32 capability”
- Keep binding-owned opaque handles for owned resources Arcana creates, and use raw Win32 types only for borrowed/native interop:
  - opaque owned: window objects, file streams, audio devices/buffers/playbacks, GDI/software-surface objects
  - raw borrowed: `HWND`, `HANDLE`, `HMONITOR`, `RECT`, `MSG`, and similar native interop values
- Keep `Window` as a generic owned native-window resource if `winapi` itself creates/destroys the window. Do not make `Session`, `FrameInput`, or `WakeHandle` desktop-owned concepts part of the long-term generic contract unless they are reduced to generic OS primitives.
- Keep generic wake/wait primitives only if they are explicitly generic:
  - create/close/signal/take-pending wake object
  - wait for wake or Win32 messages
  - pump raw message queue
  These must not imply app lifecycle or typed desktop events.
- Keep GDI surface helpers only as generic Win32 software-presentation substrate:
  - open/configure/destroy surface
  - pixel buffer access
  - present / bounded present
  They must not define `softbuffer` policy such as age semantics, acquire rules, or damage behavior beyond the raw primitive.
- Keep process/fs and audio only if the public behavior is generic Windows substrate behavior, not Arcana app policy.

## Implementation Changes

- Split `arcana_winapi` mentally into three layers and enforce it:
  - `raw`: direct Win32/CABI declarations
  - `helpers`: generic substrate wrappers and owned-resource adapters
  - no public policy layer inside `winapi`
- Remove desktop policy from `helpers_desktop_impl` and adjacent public exports:
  - move lifecycle state, resumed bookkeeping, multi-window coordination, window routing, typed event framing, and `ControlFlow`-style wait semantics out of `winapi`
  - keep only raw window/message/input/text/monitor/cursor/clipboard/wake/wait capabilities
- Reduce event support in `winapi` to raw event transport, not desktop semantics:
  - raw per-window event extraction and raw input snapshots are acceptable if they are generic transport
  - app-level events like resumed/suspended/about-to-wait/user-event are not acceptable in `winapi`
- Make all waiting/blocking paths real Win32 waits:
  - no polling sleep loops in canonical substrate paths
  - use Win32 event/message wait primitives consistently
- Normalize the window resource model:
  - explicit open/close/alive/native-handle/state/style APIs
  - native setters must actually mutate HWND state, not just cached Arcana state
  - IME composition-area, cursor grab/icon/visibility, decorations, resizable, topmost, transparency, and attention request must all be real Win32 behavior
- Normalize the graphics substrate:
  - keep only generic GDI/DIB/software-surface ownership and presentation primitives in `winapi`
  - move buffer-age, acquire discipline, damage semantics, and `softbuffer`-class policy out of `winapi`
- Normalize process/fs:
  - path and filesystem behavior stays generic Windows substrate behavior
  - no app-shell/path-policy logic beyond what is necessary to present Windows functionality safely through CABI
- Normalize audio:
  - keep device/buffer/playback ownership and low-level Windows-backed behavior only
  - no convenience playback policy that belongs in `arcana_audio`
- Update higher-layer consumers after the substrate cut:
  - `arcana_desktop` owns its own runner/lifecycle/event-loop semantics over raw `winapi`
  - `arcana_graphics.arcsb` owns `softbuffer`-class behavior over generic GDI/window-surface helpers
- Update frozen docs/specs so the approved contract no longer ratifies leaked desktop-owned `winapi` concepts.

## Test Plan

- Add or update substrate-focused tests for `arcana_winapi` itself:
  - raw hidden-window/message-pump roundtrips
  - generic wake create/signal/wait/close behavior
  - window open/close/alive/native-handle/state/setter roundtrips
  - monitor/DPI queries
  - cursor and IME/text-input operations
  - clipboard text/bytes roundtrips
  - GDI surface open/configure/pixel/present behavior
  - process/fs path and stream behavior
  - audio device/buffer/playback handle validity and state transitions
- Add “boundary” tests proving the generic rule:
  - no canonical public `session_*` / app-lifecycle / desktop-runner helpers remain in `arcana_winapi`
  - higher-layer grimoires still build using the new substrate
- Keep Windows packaging tests only as end-to-end smoke:
  - single-window desktop bundle
  - multi-window desktop bundle
  - `arcsb_app` packaged bundle
  These verify the substrate is usable, but they are not the primary correctness spec.

## Assumptions And Defaults

- Default compatibility choice: hard-remove the leaked desktop-owned compatibility lane from canonical `winapi` rather than carrying a long bridge.
- Default handle policy: binding-owned opaque handles for owned resources, raw Win32 handles for borrowed/native interop.
- Default public surface policy: `raw + generic helpers`, not raw-only and not a broad host-policy layer.
- Scope includes all current `winapi` domains, but the architectural priority is still “generic reusable substrate first,” not “finish every higher-layer feature now.”
- `winit`-class and `softbuffer`-class behavior remains the semantic target for `arcana_desktop` and `arcana_graphics`; `arcana_winapi` is only the Windows substrate they build on.
