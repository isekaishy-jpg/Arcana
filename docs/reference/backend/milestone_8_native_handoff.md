# Milestone 8 Native Backend Handoff

Status: `reference-only`

This note captures the current Milestone 8 native-backend state for the next agent.
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

## What Is Implemented Now

Native outputs:
- `windows-exe` emits a compiled package-specific executable bundle
- `windows-dll` emits a compiled package-specific DLL bundle with typed exports and generated header/definition files

Bundle/runtime contract:
- native bundles stage and run without Arcana installed
- runtime package images are generated at native build time and embedded into the emitted native project
- native bundle manifests are backend-owned artifacts

Direct native lowering:
- direct lowering supports immutable `let` blocks
- direct lowering supports terminal `if`
- direct lowering supports Int `+`, `-`, `*`, `/`, `%`
- direct lowering supports Int comparisons `==`, `!=`, `<`, `<=`, `>`, `>=`
- direct lowering supports direct same-package resolved positional `:: call`
- direct lowering still falls back to runtime dispatch when the routine/body shape is outside the supported subset

Typed DLL ABI:
- current ABI covers `Int`, `Bool`, `Str`, `Array[Int]`, `Pair[...]`, and `Unit`
- exports are package-specific typed native functions, not the old generic JSON shim ABI

## Recent Slice

The latest Milestone 8 slice widened the backend-neutral direct subset:
- `native_lowering.rs` now models direct routine bodies as `NativeDirectBlock`
- direct lowering can now carry immutable local bindings plus terminal `if`
- Int arithmetic/comparisons are lowered into backend-neutral direct nodes
- `rust_codegen.rs` renders those direct block/int-op nodes rather than assuming a single flat return expression
- native bundle smoke tests now prove source-level `if` plus Int ops through both `windows-exe` and `windows-dll`

Relevant files:
- `crates/arcana-aot/src/native_lowering.rs`
- `crates/arcana-aot/src/rust_codegen.rs`
- `crates/arcana-cli/src/package_cmd.rs`

## What To Do Next

Highest-value next slices:
1. Broaden direct lowering beyond terminal `if` into a small structured loop/statement subset where it is semantically safe.
2. Broaden direct call lowering beyond plain positional `:: call` where the IR/runtime contract is clear.
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

The latest slice was verified with:
- `cargo fmt`
- `cargo test -q -p arcana-aot`
- `cargo test -q -p arcana-cli package_workspace_stages_runnable_windows_exe_bundle`
- `cargo test -q -p arcana-cli package_workspace_stages_loadable_windows_dll_bundle`
- `cargo test -q -p arcana-package`
- `cargo test -q`

## One Cleanup Note

If an IDE still shows `crates/arcana-native-shim/src/lib.rs`, that is stale context from the older transitional path.
The workspace has moved past the generic native shim design; native package emission is now driven by package-specific generated native projects.
