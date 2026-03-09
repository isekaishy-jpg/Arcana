# Arcana Backend Roadmap (Post-Plan 40 Snapshot)

## Current Shape

Compiler/runtime pipeline today:
1. `lexer` -> tokens
2. `parser` -> AST
3. semantic + typechecking
4. typed IR construction (`arcana-ir-v2-ssa`)
5. IR lowering to bytecode (`ARCB`)
6. runtime execution
   - native backend (Cranelift) is canonical/default
   - VM backend is historical archive path (not active workflow)

Key facts:
- Native execution prefers CLIF-direct; non-strict `run`/`chant` paths may fall back to interpreter execution when bytecode is not CLIF-direct-compatible.
- `arcana chant` is the bytecode command and remains supported.
- `arcana check` is selfhost-canonical (Arcana frontend path); legacy Rust check oracle is retired.
- Oracle lifecycle state is governed by `docs/specs/backend/check_oracle_state.toml` (now `sunset`).
- `arcana compile` and `arcana build` are selfhost-canonical in sunset mode; hidden Rust compile/build oracle commands are retired.
- Compile/build lifecycle state is governed by `docs/specs/backend/compile_oracle_state.toml` (`sunset`).
- Compile/build sunset policy now includes hard bootstrap proof (`selfhost_bootstrap_hard_guard.ps1`) in CI.
- Selfhost core no-seed closure policy is tracked in `docs/specs/backend/selfhost_core_state.toml`.
- Canonical compile/build/bootstrap guard lanes enforce both:
  - `ARCANA_FORBID_SEED_FALLBACK=1`
  - `ARCANA_FORBID_SELFHOST_BRIDGE=1`
- Runnable compiled artifact execution is validated separately in `selfhost_runnable_artifact_guard.ps1`.
- Bytecode remains the shared artifact format (`.arcbc`), with typed function signatures and scheduler behavior metadata (`ARCB` v29).
- Library artifacts remain `arclib-v1` and lockfile remains v3.
- Contract-scheduled `std.behaviors.step` execution is native-only.

## What Is Stable

1. Language surface remains backend-agnostic.
2. Compiler backend boundary is IR-first (`IR -> bytecode`, `IR -> native lowering`).
3. VM is retained in-repo as historical code only, not active parity/runtime infrastructure.
4. Build artifacts and workspace cache layout stay deterministic.

## Next Backend/Selfhost Priorities

1. Host-tooling substrate expansion in `std.*` (file/path/env/args/text/bytes follow-ons).
2. Native runtime determinism hardening and parity guard expansion.
3. Selfhost compiler/tooling grimoires moving from MVP tooling toward full bootstrap workflows.
4. Eventually: AOT and packaging/distribution support (after selfhost tool maturity).

## Deferred Work Ledger

Each deferred item must include:
- `id`
- `title`
- `reason_deferred`
- `target_window`
- `trigger_condition`
- `owner`
- `acceptance_criteria`
- `status`

### IR-D1
id: IR-D1  
title: AOT native backend target (in addition to JIT)  
reason_deferred: current priority is strict native JIT correctness and selfhost tooling closure.  
target_window: post-selfhost tooling stabilization  
trigger_condition: ready_when=native strict parity remains green across release cycle; verify=AOT binary smoke matrix passes on primary platforms; blocked_by=packaging/runtime image design.  
owner: Arcana backend team  
acceptance_criteria: Arcana emits runnable native binaries with deterministic behavior parity against JIT for scoped suites.  
status: deferred

### IR-D2
id: IR-D2  
title: Native optimization pass stack on SSA IR  
reason_deferred: correctness and tooling capability are prioritized over optimization throughput.  
target_window: performance hardening phase  
trigger_condition: ready_when=selfhost tooling MVP graduates and perf baselines are captured; verify=opt/non-opt output parity with measurable perf wins; blocked_by=baseline profiling dataset.  
owner: Arcana compiler/backend team  
acceptance_criteria: deterministic optimization pipeline with parity-safe transforms and CI regression gates.  
status: deferred

### IR-D3
id: IR-D3  
title: VM repository cleanup/removal follow-on  
reason_deferred: VM execution is already removed from active user workflows in native-only mode; remaining work is archival cleanup policy.  
target_window: post-selfhost stabilization phase  
trigger_condition: ready_when=native_golden_guard + scheduler/host/selfhost guards remain green for one full cycle; verify=vm_historical_guard confirms no active dependency edges; blocked_by=historical retention decision.  
owner: Arcana runtime/toolchain team  
acceptance_criteria: VM is either fully removed from repository or retained strictly as documented historical artifact with no build/CI coupling.  
status: deferred
