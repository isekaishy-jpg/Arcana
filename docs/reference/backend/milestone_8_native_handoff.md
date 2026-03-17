# Milestone 8 Native Backend Completion Note

Status: `reference-only`

This note captures the completed Milestone 8 native-backend state.
It is implementation context, not rewrite contract authority.

Use current authority first:
- `PLAN.md`
- `docs/rewrite-roadmap.md`
- `docs/specs/spec-status.md`
- approved/frozen backend and std/grimoire scope docs under `docs/specs/**`

## Goal

Milestone 8 is no longer about polishing the old minimal internal AOT lane.
The target shape is:
- package-specific native `exe` / `dll` outputs
- compiled artifacts that run without Arcana installed
- backend-owned launch/export contracts
- backend seams that can later retarget from generated Rust to Cranelift without redoing package/build/run/distribution

The current native Rust backend is scaffolding toward that end state, not the final backend architecture.

## Important Architecture Decisions

- Do not revive the old generic launcher/shim model as the target design.
  `windows-exe` and `windows-dll` should stay package-specific native outputs.
- Keep backend-neutral lowering in `crates/arcana-aot/src/native_*`.
  Backend-specific rendering belongs in emitter/toolchain files such as `rust_codegen.rs` and any future Cranelift emitter.
- Treat the current generated-Rust backend as replaceable.
  Package/build/distribution/native manifest/runtime ABI seams should survive a later Cranelift swap.
- Do not let package or CLI crates absorb backend logic.
  Native emission policy should remain inside `arcana-aot`.

## Current State

The native path is already split into stable seams instead of one monolith:
- artifact/codec/validation/emission split in `crates/arcana-aot/src/*`
- native package planning in `native_plan.rs`
- native ABI collection in `native_abi.rs`
- native bundle manifest contract in `native_manifest.rs`
- backend-neutral direct-lowering plan in `native_lowering.rs`
- generated-Rust backend in `rust_codegen.rs`
- toolchain execution in `rust_toolchain.rs`

Package/build/CLI are already wired to those seams:
- target-aware package planning and cache/distribution in `crates/arcana-package/src/*`
- `arcana build`, `arcana run`, and `arcana package` use the native target path in `crates/arcana-cli/src/*`

Runtime-side support already exists for native bundles:
- runtime package image embedding/loading
- typed native ABI conversion in `crates/arcana-runtime/src/native_abi.rs`
- stable entrypoint execution keyed by routine identity rather than loose symbol lookup
- generated native entry/runtime fallback now calls `current_process_runtime_host()`
- on Windows, `current_process_runtime_host()` constructs `NativeProcessHost`, which provides real stdout/stderr/stdin, time/sleep, Win32 window/input/canvas/event behavior, BMP image loading/blitting, and WinMM WAV playback for emitted bundles
- non-Windows callers still fall back to `BufferedHost`, but the Milestone 8 native `windows-exe` / `windows-dll` path now runs on the real Windows host seam instead of the synthetic app/runtime host

## What Is Implemented Now

Native outputs:
- `windows-exe` emits a compiled package-specific executable bundle
- `windows-dll` emits a compiled package-specific DLL bundle with typed exports and generated header/definition files

Bundle/runtime contract:
- native bundles stage and run without Arcana installed
- runtime package images are generated at native build time and embedded into the emitted native project
- native bundle manifests are backend-owned artifacts
- generated native runtime fallback uses the current-process runtime host rather than a synthetic buffered launcher host
- emitted native bundles now execute real host-backed window/canvas and audio flows on Windows

Direct native lowering:
- direct lowering supports local `let` blocks, including mutable locals
- direct lowering supports terminal `if`
- direct lowering supports statement-form `if`
- direct lowering supports `while` with local-name assignment plus `break` / `continue`
- direct lowering supports direct statement-form calls and early `return` in the structured subset
- direct lowering supports Int `+`, `-`, `*`, `/`, `%`
- direct lowering supports Int comparisons `==`, `!=`, `<`, `<=`, `>`, `>=`
- direct lowering supports resolved positional and named `:: call` where the resolved signature stays inside the current native ABI/direct subset
- direct lowering still falls back to runtime dispatch when the routine/body shape is outside the supported subset

Typed DLL ABI:
- current ABI covers `Int`, `Bool`, `Str`, `Array[Int]`, `Pair[...]`, and `Unit`
- exports are package-specific typed native functions, not the old generic JSON shim ABI

Native runtime-host coverage:
- `NativeProcessHost` delegates host-core filesystem/path/process behavior through `BufferedHost` where that is already rewrite-owned and sufficient, but owns the real OS-bound window/canvas/audio/time/stdin/stdout seams itself
- canvas support covers fill/rect/line/circle/label/present plus BMP image load and blit/blit_scaled/blit_region
- audio support covers default-output selection, WAV decode to PCM16, playback start/stop/pause/resume, looping, gain, and position queries
- playback-open failure paths now clean up native audio handles before returning errors instead of leaking partially opened playback state
- audio `stop` and `output_close` now follow the approved consuming-lifecycle contract in both the native host and the buffered host used by synthetic runtime tests
- Win32 window-class registration is now module-specific, so multiple Arcana-generated DLLs can create windows in the same host process without colliding on a shared class name
- native asset decoders now have direct unit coverage and stricter overflow/shape validation in addition to the emitted-binary smokes

## Recent Slices

The previous Milestone 8 slice widened the backend-neutral direct subset:
- `native_lowering.rs` now models direct routine bodies as `NativeDirectBlock`
- direct lowering can now carry immutable local bindings plus terminal `if`
- Int arithmetic/comparisons are lowered into backend-neutral direct nodes
- `rust_codegen.rs` renders those direct block/int-op nodes rather than assuming a single flat return expression
- native bundle smoke tests now prove source-level `if` plus Int ops through both `windows-exe` and `windows-dll`

The latest Milestone 8 slice widened the structured statement subset further:
- `native_lowering.rs` now carries mutable local bindings, local-name assignment, statement-form `if`, and `while` loop bodies with `break` / `continue`
- compound Int assignment on mutable local names (`+=`, `-=`, `*=`, `/=`, `%=`) lowers into backend-neutral direct nodes instead of forcing runtime fallback
- `rust_codegen.rs` renders the expanded direct statement subset without moving those semantics out of backend-neutral lowering
- native bundle smoke tests now prove source-level mutable locals and `while` loops through both `windows-exe` and `windows-dll`

The newest Milestone 8 slice widened direct-call/control-flow coverage again:
- direct lowering now carries statement-form direct calls and nested early `return` inside the supported structured subset instead of forcing those bodies onto runtime fallback
- direct call lowering now accepts named arguments when they match the resolved native routine signature, not only plain positional calls
- native bundle smoke tests now prove named-call lowering through both `windows-exe` and `windows-dll`

The latest Milestone 8 slice also closed a package-planning hole and broadened end-to-end native smoke depth:
- `arcana-package` no longer tries to build native `windows-exe` artifacts for library dependencies of app bundles; those library deps now stay on internal artifacts while still linking into the root native bundle
- native exe smoke coverage now includes app-substrate window/canvas and audio apps running as emitted bundles, not only trivial helper-style programs

The completion slice closed the remaining real-host gap:
- `arcana-runtime` now exposes `current_process_runtime_host()`
- generated `windows-exe` / `windows-dll` runtime fallback code now constructs that host instead of hardcoding `BufferedHost`
- the Windows implementation is `NativeProcessHost`, a real current-process runtime host backed by Win32 window/canvas/input/events plus WinMM audio
- native visual smoke now stages and loads a real BMP image from the emitted bundle and blits it through the real canvas host path
- native audio smoke now stages and loads a real WAV file from the emitted bundle and plays it through a real default audio output path
- `native_host.rs` now has direct unit tests for BMP/WAV decoding and stricter size/shape validation so the host seam is covered below the end-to-end bundle tests

The follow-up hardening slice closed the review findings from the first completion pass:
- `play_buffer` cleanup now closes native audio state on post-open failures instead of leaking a `waveOut` handle or leaving partially started playback behind
- finished native playbacks now release queued PCM data once playback completes, and explicit `stop` / `output_close` remove consumed playback or device handles from the active host maps
- the synthetic `BufferedHost` now matches that consuming audio lifecycle behavior, so runtime tests and the real Windows host do not drift on stop/close semantics
- native window class registration now derives a module-specific class name from the current runtime module, avoiding same-process collisions between multiple Arcana-generated DLLs

Relevant files:
- `crates/arcana-aot/src/native_lowering.rs`
- `crates/arcana-aot/src/rust_codegen.rs`
- `crates/arcana-runtime/src/native_host.rs`
- `crates/arcana-runtime/src/lib.rs`
- `crates/arcana-cli/src/package_cmd.rs`
- `crates/arcana-package/src/build.rs`

## After Milestone 8

Likely next backend slices after Milestone 8:
1. Broaden direct lowering beyond local-name `while` / statement-`if` / nested early-return into the next semantically safe structured subset.
2. Broaden direct call lowering beyond positional-or-named resolved `:: call` where the IR/runtime contract is clear.
3. Keep pushing logic downward into backend-neutral lowering/native-plan layers so a future Cranelift emitter consumes the same plan instead of cloning Rust-codegen logic.
4. Revisit the native ABI surface only when there is a clear stable calling-convention story; do not bolt on ad hoc export forms.

Good guardrails for future work:
- if a feature can be expressed in `native_lowering.rs`, put it there first
- if a change only exists to satisfy Rust code generation, it probably belongs in `rust_codegen.rs`
- if a change affects build identity, distribution layout, or target selection, keep it in `arcana-package` / `arcana-cli`, not in the runtime

## Avoid These Regressions

- Do not reintroduce global-name or textual-artifact runtime startup shortcuts in native outputs.
- Do not move native lowering semantics into string templates in `rust_codegen.rs`.
- Do not treat copied generic launchers/shims as the end-state native design.
- Do not bind Milestone 8 architecture to generated Rust specifics; Cranelift still needs a clean insertion point later.
- Do not make `docs/reference/*` the source of truth over approved specs or roadmap text.

## Verification

Milestone 8 completion was verified with:
- `cargo fmt`
- `cargo test -q -p arcana-runtime`
- `cargo test -q -p arcana-aot`
- `cargo test -q -p arcana-cli package_workspace_stages_runnable_windows_exe_bundle`
- `cargo test -q -p arcana-cli package_workspace_stages_loadable_windows_dll_bundle`
- `cargo test -q -p arcana-cli package_workspace_runs_native_window_canvas_app_bundle`
- `cargo test -q -p arcana-cli package_workspace_runs_native_audio_app_bundle`
- `cargo test -q -p arcana-cli`
- `cargo test -q`

## One Cleanup Note

If an IDE still shows `crates/arcana-native-shim/src/lib.rs`, that is stale context from the older transitional path.
The workspace has moved past the generic native shim design; native package emission is now driven by package-specific generated native projects.
