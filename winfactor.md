# `arcana_winapi` Refactor to a True `windows-sys`-Style Grimoire

## Summary

- Finish the `winapi` reset by making `arcana_winapi` a real `windows-sys`-style source package built on shackle.
- Public package surface stays `arcana_winapi.raw.*` only.
- `shackle.arc` becomes the sole source-level owner of binding implementation.
- Delete the package-visible `backend/*` Arcana module layer entirely.
- Keep any unavoidable implementation support Rust-private under the shackle/backend seam; removing the backend module layer does **not** forbid private implementation support.
- `arcana_process.fs.FileStream` remains the only public stream-handle type; `winapi` must not retain even an internal host-core process/file lane.

## Implementation Changes

### 1. Final package shape
- Reduce `grimoires/arcana/winapi/src/` to:
  - `book.arc`
  - `raw.arc`
  - `raw/*`
  - `shackle.arc`
  - `types.arc` only if the loader still requires a parseable top-level `src/types.arc`
  - implementation-only support files only if they do **not** become package modules
- Delete:
  - `backend.arc`
  - `src/backend/*`
  - the current package-visible implementation files such as `backend_audio_impl.arc`, `backend_desktop_impl.arc`, `backend_process_impl.arc`, and `backend_support_impl.arc`
- Any surviving support must be shackle/private support, not Arcana package surface.

### 2. Hard-cut the non-raw WinAPI domains
- Delete the remaining non-raw domains from `winapi` source:
  - foundation/module helpers
  - font catalog helpers
  - hidden-window/message probe helpers
  - wake/message helper layer
  - window management layer
  - text-input composition helpers
  - clipboard helpers
  - graphics surface/bootstrap helpers
  - audio playback/bootstrap helpers
  - the leftover internal process/path/fs lane
- Delete the corresponding internal Arcana opaque handle families:
  - desktop/window/wake handles
  - graphics surface handles
  - audio device/buffer/playback handles
  - leftover module/font/hidden-window wrapper types
- Keep only raw Win32/COM/layout/callback declarations in `raw.*`.

### 3. Rebuild shackle around raw declarations only
- Make [shackle.arc](/C:/Users/Weaver/Documents/GitHub/Arcana/grimoires/arcana/winapi/src/shackle.arc) the sole source-level owner of the binding implementation.
- Remove the old domain vocabulary as actual source API surface:
  - no surviving `foundation.*`, `fonts.*`, `windows.*`, `helpers.*`, or `backend.process.*` module layer
- After the structural cut, normalize remaining private symbol families only where it improves clarity; deletion of the old layer comes first.
- Keep only what belongs in a `windows-sys`-style package:
  - raw import declarations
  - raw callback signatures
  - raw layouts/constants
  - Rust-private support needed to implement those declarations
- If internal mutable state is still unavoidable for callbacks or binding support, keep it Rust-private under shackle support. Do not model it as Arcana source handles or helper modules.

### 4. Remove WinAPI host-core residue
- Delete the internal process/path/fs lane from `winapi` completely.
- Delete any shackle implementation lane that still models `winapi` as a host-core process/file provider.
- Keep all host-core args/env/path/fs/process behavior solely in `arcana_process` and runtime host code.
- If binding internals still need file/process support for loading or generated/native backend work, that support must come from generic runtime/loader/backend infrastructure or shackle-private implementation support, not from a hidden `winapi` host-core sidecar.

### 5. Replace helper-layer smoke coverage
- Delete probe/demo-style WinAPI routines such as hidden-window roundtrips and bootstrap smoke helpers from the grimoire.
- If validation is still needed, move it to crate-level Rust tests or native-product tests that exercise:
  - raw declarations
  - callback lowering
  - binding transport
  - generated/native product correctness
- Do not preserve those probes as any source-level `winapi` API, public or private.

### 6. Docs and repo invariants
- Update the authoritative and active docs so they all describe the same end state:
  - `docs/specs/os-bindings/os-bindings/v1-scope.md`
  - `llm.md`
  - any active roadmap/reference docs that still imply helper/wrapper domains inside `winapi`
- Explicitly state:
  - `arcana_winapi` public surface is `raw.*` only
  - there is no surviving helper/wrapper/handle module layer in source
  - any remaining implementation support is not Arcana package surface
- Strengthen repo invariants so they fail if:
  - `arcana_winapi` republishes any non-raw namespace
  - any package-visible `backend.*` module ids survive
  - repo consumers import anything under `arcana_winapi` except `raw.*`
  - helper/handle domain names reappear in docs or package-shape tests as living `winapi` design

## Test Plan

- Package-shape tests:
  - `arcana_winapi` publishes only `arcana_winapi.raw.*`
  - no package-visible module ids remain for `backend.*`, `helpers.*`, `foundation`, `fonts`, `windows`, `desktop_handles`, `graphics_handles`, `audio_handles`, or `process_handles`
- Source-tree checks:
  - no `grimoires/arcana/winapi/src/backend/*` Arcana module layer remains
  - any surviving support files do not become package modules
- Consumer checks:
  - no repo consumer depends on `arcana_winapi` for typed handles or helper routines; remaining imports are `raw.*` only
- Binding/native checks:
  - native-product generation for `grimoires/arcana/winapi` still succeeds
  - raw callback/type/layout exports still lower and compile correctly
  - removed helper/probe domains no longer appear in generated binding metadata
- Full hygiene:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --quiet`

## Assumptions

- This is a hard-cut refactor, not a compatibility migration.
- Non-raw WinAPI domains are removed now, not preserved as “internal for later.”
- The intended model is `windows-sys`-style raw declarations on top of shackle, not raw plus a hidden Arcana helper framework.
- Removing the backend module layer does not ban implementation support; it bans package-visible Arcana helper modules. Remaining support is allowed only as shackle/private implementation support or generic runtime/loader/backend infrastructure.
