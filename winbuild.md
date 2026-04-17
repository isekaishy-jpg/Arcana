# `arcana_winapi` Windows-Sys Parity Buildout

## Summary

- Build `arcana_winapi` into a broad `windows-sys`-style raw binding grimoire on top of the already-cleaned `raw.*` public boundary.
- Keep the current public module ids stable:
  - `book.arc` stays `reexport arcana_winapi.raw`
  - `raw.arc` stays handwritten
  - existing public leaf paths like `arcana_winapi.raw.user32`, `kernel32`, `dxgi`, `types`, `constants`, and `callbacks` stay valid
- Switch the raw surface from handwritten curation to generator-owned checked-in leaves.
- Use a pinned Windows SDK metadata snapshot as the authoritative source of truth.
- Treat `windows-sys` as the parity target for exposed names, signatures, and coverage shape inside the chosen surface, but not as the ultimate authority when it disagrees with the pinned metadata source.
- Target broader Windows API coverage in phases:
  - first expand the current Win32/core COM/graphics/text/audio families already present
  - then add additional families through the same generator path
- Keep raw modules declarative by default. Allow only a narrow explicit raw-shim exception path for cases like awkward linkage/dynload, backed by shackle-private support and exported with exact raw signatures.

## Implementation Changes

### 1. Generator-owned raw surface
- Add a dedicated generator tool, as a Rust workspace binary, for `arcana_winapi` raw surface generation.
- Add one stable human entrypoint script, for example under `scripts/dev/`, that runs the generator in:
  - write mode
  - check/no-diff mode for CI
- Add a checked-in generator config area owned by `grimoires/arcana/winapi`, containing:
  - pinned Windows SDK metadata source/version/hash
  - module projection/mapping config
  - explicit skiplist for unsupported metadata constructs
  - explicit exception manifest for rare raw shims
- Generated output is checked in and deterministic.
- Humans edit only:
  - generator code
  - metadata pin/config
  - skip/exception manifests
  - handwritten boundary files
- Humans do not hand-edit generated raw leaves.

### 2. File/layout shape
- Keep handwritten boundary files:
  - `grimoires/arcana/winapi/src/book.arc`
  - `grimoires/arcana/winapi/src/raw.arc`
  - top-level `src/types.arc` if still needed structurally
- Generate the public raw leaf files directly under `grimoires/arcana/winapi/src/raw/`, including:
  - per-module declaration leaves like `user32.arc`, `kernel32.arc`, `dxgi.arc`
  - generated `types.arc`
  - generated `constants.arc`
  - generated `callbacks.arc`
- Every generated file gets a short header that states:
  - generated file
  - pinned source of truth
  - do not edit by hand
- Do not hide generated output outside the repo.
- Do not mix handwritten and generated declarations in the same public leaf file unless a case is forced by the raw-shim exception mechanism.

### 3. Public API and projection rules
- Preserve existing public raw module ids; do not rename or reshuffle the current public module layout.
- The set of public raw leaves is projection-config-driven, not ad hoc.
- New public raw leaves are added only through generator projection config, never by handwritten one-off module creation.
- `raw.arc` remains handwritten as the public routing boundary, but it may reexport only leaves produced by the configured projection set.
- Adding a new public raw leaf requires all of:
  - projection-config entry
  - pinned-metadata coverage
  - generated file output
  - `raw.arc` reexport update
  - parity/check test update where needed
- Within each exposed raw leaf, match `windows-sys` names and FFI signatures for generated declarations where the pinned metadata supports that shape.
- If `windows-sys` and the pinned metadata source differ, the pinned metadata source wins; `windows-sys` is the parity target, not the authority.
- Continue to allow Arcana-specific packaging differences:
  - flat current `raw.*` leaf layout
  - shared generated `types`, `constants`, and `callbacks` leaves
- Expand the generated surface to broad parity for the current family set first:
  - `kernel32`, `user32`, `gdi32`, `dwmapi`, `shcore`, `shell32`, `imm32`
  - `ole32`, `combase`, `propsys`
  - `dxgi`, `d3d12`, `dwrite`, `d2d1`, `wic`
  - `mmdeviceapi`, `audioclient`, `audiopolicy`, `endpointvolume`, `avrt`, `mmreg`, `ksmedia`, `xaudio2`, `x3daudio`
  - `types`, `constants`, `callbacks`
- After that first broadening pass, new Windows families may be added only by:
  - explicit generator config entry
  - explicit new public raw leaf if no current module is an appropriate home
  - handwritten `raw.arc` reexport update
- Generate raw declarations for:
  - functions/imports
  - constants
  - typedef/alias families
  - structs/unions/fixed arrays/flags/enums
  - callbacks
  - COM/interface/vtable layouts already supported by the shackle raw model
- Unsupported upstream items must be explicitly skipped with checked-in rationale. Do not silently handwave gaps.

### 4. Shackle/private support model
- `shackle.arc` remains the sole source-level private implementation seam.
- No package-visible `backend.*`, helper, wrapper, or handle modules are reintroduced.
- Normal generated callable items use `export shackle import fn`.
- Rare raw-shim exceptions are allowed only when a direct declarative import is not viable.
- Raw-shim exceptions must obey all of these rules:
  - public path remains under `arcana_winapi.raw.*`
  - exported name and signature match the true raw Windows surface exactly
  - implementation lives in shackle-private support
  - no `Result`, policy, ownership reshaping, or helper semantics
  - every exception is declared in the checked-in exception manifest and is reviewable
- The currently approved legacy bootstrap-ish items named in `os-bindings v1-scope` must be classified explicitly in this wave:
  - items that are expressible as ordinary raw imports/constants/types stay as normal generated raw coverage, not exceptions
  - items that require non-declarative support survive only as explicit initial exception-manifest entries
  - items that cannot be justified as raw-only surface are removed from the scope text in the same wave
- Existing cases like `X3DAudioInitialize` should move to that explicit exception path rather than remain ad hoc handwritten logic in generated leaves.

### 5. Docs and repo invariants
- Update the active docs so they describe the generated raw-only model consistently:
  - `docs/specs/os-bindings/os-bindings/v1-scope.md`
  - `llm.md`
  - any active roadmap/reference docs that still describe handwritten helper-era `winapi`
- In `os-bindings v1-scope`, resolve the currently named bootstrap-ish survivors explicitly:
  - either restate them as ordinary generated raw coverage
  - or mark them as initial raw-shim exceptions
  - or retire them from the approved surface text
- Repo/package invariants must enforce:
  - only `arcana_winapi.raw.*` is public
  - no package-visible `backend.*` layer returns
  - generated leaves stay generated
  - no repo consumer depends on non-raw `arcana_winapi`
  - new raw leaves cannot appear outside the configured projection set
- Add a regeneration guard in CI:
  - run the generator in check/no-diff mode
  - fail if generated raw files are out of date

## Test Plan

- Generator determinism:
  - clean regen produces no diff after a second run
  - generated file headers exist and identify source of truth
  - stable sort/order rules keep diffs minimal
- Package-shape checks:
  - `arcana_winapi` still publishes only `arcana_winapi.raw.*`
  - no `backend.*`, helper, wrapper, or handle module ids appear
  - no public raw leaf exists without a matching projection-config entry
- Raw parity checks:
  - spot-check key functions/types/constants/callbacks across the major family set
  - compare generated declarations against the pinned metadata selection so missing/extra symbols are caught
  - verify `types.arc`, `constants.arc`, and `callbacks.arc` are fully generator-owned
  - verify `windows-sys` parity for exposed names/signatures across representative modules, while treating pinned metadata as the tiebreaker
- Exception-path checks:
  - raw-shim exceptions are allowed only if present in the checked-in exception manifest
  - exception exports keep exact raw signatures
  - no exception introduces Arcana-shaped policy or `Result` reconstruction
  - each approved legacy bootstrap-ish survivor is either normal generated coverage or an explicit exception-manifest entry, never an ambiguous special case
- Binding/native checks:
  - `grimoires/arcana/winapi` native-product generation still succeeds
  - generated shackle declarations still lower, validate, and compile
- Full validation:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --quiet`

## Assumptions

- The pinned Windows SDK metadata snapshot is authoritative.
- `windows-sys` is the parity target for exposed naming/signature shape, not the source of truth.
- Public raw module ids stay stable; broadening the surface is allowed, but renames/removals are not part of this wave.
- Coverage is broader than classic Win32-only, but rollout is phased:
  - current Win32/core COM/graphics/text/audio families first
  - broader families only through the same generator/config path afterward
- Checked-in generated leaves, handwritten boundary files, and one deterministic regeneration entrypoint are required, not optional.
- Raw-shim exceptions are allowed only as a constrained escape hatch, not as a second handwritten feature lane.
