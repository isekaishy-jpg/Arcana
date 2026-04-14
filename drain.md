# Runtime Drain and Ownership Reset: `cabi -> winapi -> consumers`

## Summary

- Remove everything from `arcana-runtime` that is not runtime-engine work.
- Delete `crates/arcana-runtime-host`.
- Remove `windows-sys` from workspace Rust crates entirely.
- Make the real stack:
  - `arcana-cabi`
  - `arcana_winapi`
  - consumers: `arcana_process`, `arcana_desktop`, `arcana_graphics.arcsb`, `arcana_audio`, `arcana_text`
- Restore the dropped commitments from the prior plan:
  - `arcana_process` takes over most old public std host-core surface
  - `arcana_winapi.helpers.window/events/clipboard/text_input` stop being runtime-intrinsic wrappers
  - `arcana_winapi.helpers.audio` becomes the real low-level backend
  - runtime text-rendering/text-grimoire scaffolding is removed
  - AOT/CLI/generated output stop depending on `arcana-runtime-host`
  - docs, tests, and compile-time gates are updated together

## Public Ownership Changes

- Add `grimoires/arcana/process` as the public owner for:
  - stdio
  - args/env/cwd
  - filesystem and process operations
  - current-process bootstrap conveniences used by runner/AOT
  - path manipulation that is public API, with pure/path-string logic kept in Arcana source where possible
- Retire the public `std` host-facing lane:
  - `std.io`
  - `std.args`
  - `std.env`
  - `std.path`
  - `std.fs`
  - `std.process`
  - `std.audio`
- Promote `arcana_audio` to the public low-level audio owner.
- Keep `arcana_desktop` as the public desktop shell owner.
- Keep `arcana_graphics.arcsb` as the software-buffer graphics backend owner.
- Keep `arcana_text` as the public paragraph/layout/text grimoire owner.
- Keep `std.text` only as core text substrate. Runtime must not own text rendering, label helpers, or text-grimoire behavior.

## Implementation Changes

### Runtime
- Delete the broad `RuntimeHost` contract.
- Replace it only with narrow internal microservices if something is truly unavoidable for runtime-engine execution.
- Remove runtime-owned platform/process domains:
  - current-process bootstrap ownership
  - window/session/frame/wake
  - events/input/clipboard/text-input
  - graphics/software-present
  - audio device/buffer/playback
- Remove corresponding runtime artifacts:
  - `RuntimeIntrinsic` cases
  - runtime opaque families/handles for those domains
  - host-facing dispatch modules
  - `BufferedHost` production support
  - synthetic app-shell/audio host support
- Keep runtime focused on:
  - evaluator/execution
  - routine/package/runtime-plan handling
  - CABI/native product loading and binding transport
  - mapped views/core memory/value behavior
  - core `std.text` string/bytes/utf16 behavior only

### Text Removal From Runtime
- Delete `crates/arcana-runtime/src/text_engine.rs`.
- Remove any runtime-owned glyph painting, label-style drawing, or text-render helper code.
- Remove runtime tests and proof scaffolding that imply runtime text-grimoire ownership.
- Remove any `arcana_text`-specific runtime smoke or package-name assumptions.
- Keep only true core text intrinsics in runtime:
  - string length/byte ops
  - find/split/trim/join/repeat
  - utf8/utf16 conversions
  - bytes/bytebuffer/utf16/utf16buffer core support

### WinAPI Binding Layer
- Convert the current WinAPI helper wrappers from runtime intrinsics to real binding-backed helpers:
  - `arcana_winapi.helpers.window`
  - `arcana_winapi.helpers.events`
  - `arcana_winapi.helpers.clipboard`
  - `arcana_winapi.helpers.text_input`
- Keep these helpers thin and host-native only; desktop policy stays in `arcana_desktop`.
- Expand `arcana_winapi.helpers.audio` into the full low-level audio substrate currently exposed by `std.audio`.
- Add WinAPI-backed process-side substrate for `arcana_process`:
  - current-process stdio
  - args/env/cwd
  - filesystem calls
  - process exec/status/capture
- Pure path-string shaping stays in Arcana source in `arcana_process`; actual OS operations use WinAPI helpers.
- Keep `arcana_winapi.helpers.graphics` as the software-surface substrate for `arcsb`.

### Consumers
- `arcana_process` absorbs the old public host-core surface and becomes the public owner above WinAPI-backed helpers.
- `arcana_audio` absorbs the current `std.audio` low-level handle + playback API:
  - `AudioDevice`
  - `AudioBuffer`
  - `AudioPlayback`
  - acquisition/query/control functions and methods
- `arcana_desktop` stays the typed owner for:
  - settings/config/application semantics
  - typed event lifting/routing
  - input/text-input behavior
  - clipboard-facing semantics
- `arcana_graphics.arcsb` stays on `arcana_desktop` + `arcana_winapi.helpers.graphics`.
- `arcana_text` remains above `std.text`, `arcana_graphics`, and host font discovery through binding; it gets no runtime-owned helper path.

### Toolchain and Generated Output
- Delete `crates/arcana-runtime-host`.
- Remove all `arcana_runtime_host::*` references from:
  - `arcana-aot`
  - generated child/exe products
  - CLI runner glue
- Generated products must not depend on a broad host crate.
- Current-process bootstrap routes through the new public owner stack, with `arcana_process` owning that surface.
- Remove any remaining root/workspace dependency path that drags an unused giant Windows host crate into normal `cargo check`.

### Specs and Docs
- Update approved docs in the same patch:
  - `native-products-cabi-v1-scope.md`
  - `os-bindings/v1-scope.md`
  - `app-substrate-v1-scope.md`
  - `std/v1-scope.md`
  - `std/v1-status.md`
  - `grimoires/v1-scope.md`
  - `grimoires/v1-status.md`
  - `arcana-text/v1-scope.md` or status if runtime/text cleanup wording needs alignment
- The new contract must explicitly say:
  - no runtime-owned Windows host/app-shell/audio/text-rendering lane
  - no public std host-core or std-audio lane
  - `arcana_winapi` is the Windows substrate
  - `arcana_process`, `arcana_desktop`, `arcana_graphics`, `arcana_audio`, and `arcana_text` are the public owners

## Test Plan

- Runtime:
  - `cargo check -p arcana-runtime --lib`
  - keep evaluator/CABI/mapped-view/package-image tests
  - remove runtime-owned desktop/audio/text-rendering tests
  - no runtime test should require `windows-sys`
- Consumer checks:
  - `grimoires/arcana/winapi`
  - `grimoires/arcana/process`
  - `grimoires/libs/arcana-desktop`
  - `grimoires/libs/arcana-graphics`
  - `grimoires/libs/arcana-audio`
  - `grimoires/libs/arcana-text`
- Toolchain:
  - `cargo check -p arcana-aot`
  - `cargo check -p arcana-cli`
  - generated AOT output contains no `arcana_runtime_host` dependency
- Build gates:
  - repo-root `cargo check`
  - `cargo check --workspace`
  - both complete in under 10 minutes on the current machine
- Cleanup grep gates:
  - no `windows-sys` in workspace crates
  - no `arcana-runtime-host`
  - no broad `RuntimeHost`
  - no runtime-owned `Window*`, `Events*`, `Clipboard*`, `TextInput*`, `Audio*`
  - no runtime text-engine / label-paint / text-grimoire scaffolding
  - no `arcana_text` special cases

## Assumptions and Defaults

- `arcana_process` is the new public process grimoire name.
- `std.text` remains core substrate; `arcana_text` remains the paragraph/layout/text engine owner.
- Runtime may keep only tiny internal microservices if absolutely unavoidable, but no broad public/system/domain host surface remains there.
- This is a big-bang ownership cleanup, not a compatibility phase.
- Windows is the only required implementation target in this pass.
