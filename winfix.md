# `arcana_winapi` Item 5 Cleanup, With the Correct Helper Boundary

## Summary

- Clean up `arcana_winapi` to match the real 3-layer split:
  1. `raw.*` stays as direct Win32 surface
  2. thin Win32 substrate helpers stay in `arcana_winapi`
  3. composed policy helpers move out of the public `winapi` surface or are deleted if no higher layer exists yet
- In this wave, remove the public policy-shaped event/frame/input layer and keep the direct Win32 convenience layer intact.
- Do not overcorrect `arcana_winapi` into a raw-binding dump. Keep substrate helpers such as strings/errors/com, window operations, clipboard/IME/file-drop helpers, graphics bootstrap, audio bootstrap, and generic wake/message primitives.

## Key Changes

### Public surface: keep substrate helpers, remove policy helpers
- Keep public `arcana_winapi.helpers` modules that are still thin Win32 convenience:
  - `strings`, `errors`, `com`
  - `window`
  - `clipboard`
  - `text_input`
  - `graphics`
  - `text`
  - `audio`
- Remove public `arcana_winapi.helpers.events`.
- Remove public `arcana_winapi.helpers.input`.
- Remove public `FrameInput` from `arcana_winapi.desktop_handles`.
- Keep `WakeHandle` public at `arcana_winapi.desktop_handles.WakeHandle`, but narrow it to a generic wake primitive only:
  - make it `move`, not `copy`
  - make close consuming with `take`
  - propagate `CloseHandle` failure
- Introduce one small canonical helper module for the surviving generic wake/message substrate, under `arcana_winapi.helpers.message`:
  - `wake_create`
  - `wake_close`
  - `wake_signal`
  - `wake_take_pending`
  - `wait_wake_or_messages`
  - one thin pending-message pump/dispatch primitive
- Keep compatibility wrappers like `arcana_winapi.windows.*` only as wrappers/migration surface; they are not the canonical helper lane.

### Policy layer removal
- Delete the public typed event record surface (`EventRaw`) and the public frame/event API (`pump`, `poll`).
- Delete the public `helpers.input` polling API built on `FrameInput`.
- Remove frame snapshot and typed queued-event ownership from the canonical `winapi` helper surface.
- If any of the current event/input machinery is still needed for internal callback translation, keep it only as backend/private glue with no public export and no canonical handle type.

### `windowing.arc` cut
- Treat the current `helpers.windowing` file as suspicious probe/demo surface, not canonical substrate.
- Remove or internalize the current roundtrip/probe-style exports such as hidden-window roundtrips, clipboard roundtrips, and IME probe routines.
- If any individual routine is truly reusable substrate behavior, rehome it into the relevant thin helper module instead of keeping a grab-bag `windowing` helper surface:
  - window primitives go to `helpers.window`
  - clipboard probes become real clipboard helpers or are deleted
  - IME helpers stay in `helpers.text_input`
- Do not keep public test/probe helpers as part of the canonical `winapi.helpers` story.

### Backend and runtime fallout
- Move any surviving Win32 wake/message glue and any unavoidable internal translation code under backend/internal `grimoires/arcana/winapi/src/backend/*`, following the same pattern used for process.
- Remove `arcana_winapi.helpers.events` and `arcana_winapi.helpers.input` from runtime-owned embedded-source and recognition paths.
- Remove `FrameInput` from runtime-visible canonical handle assumptions.
- Keep runtime/docs aligned with the new story: `winapi` provides substrate handles and thin helpers, not a half-built desktop runner API.

### Docs/specs
- Update `docs/specs/os-bindings/os-bindings/v1-scope.md` so `arcana_winapi.helpers.*` explicitly means thin Win32 substrate helpers, not typed event/input policy.
- Update `docs/specs/resources/resources/v1-scope.md`:
  - remove `arcana_winapi.desktop_handles.FrameInput`
  - keep `WakeHandle`
- Update `docs/rewrite-roadmap.md` and other active docs so they stop implying a public `FrameInput`/events-input lane inside `winapi`.
- Keep the future higher-level desktop/windowing layer out of scope here; this is a subtraction/cleanup wave, not a replacement-layer wave.

## Test Plan

- Boundary/source-shape tests:
  - no public `arcana_winapi.helpers.events`
  - no public `arcana_winapi.helpers.input`
  - no public `FrameInput` in `desktop_handles`
  - no public `helpers.windowing` probe/demo surface remains
- Wake primitive tests:
  - create/signal/take-pending/wait still works
  - `WakeHandle` is non-copyable
  - `wake_close(take ...)` consumes and reports failure correctly
- Message primitive tests:
  - pending-message pump works as a thin Win32 helper with no typed event/frame API
  - wake-plus-message wait still uses real Win32 waiting
- Regression checks:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --quiet`

## Assumptions

- The correct boundary is “thin direct Win32 convenience stays, composed policy moves up or dies.” This wave follows that rule exactly.
- No new higher-level windowing grimoire is introduced in this wave.
- `WakeHandle` survives publicly only because a generic wake primitive belongs in the substrate; `FrameInput` does not.
- Compatibility wrappers may remain, but they do not define the canonical helper surface and should not preserve the removed policy layer by another name.
