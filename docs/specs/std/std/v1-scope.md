# Arcana Standard Library v1 Scope

Status: `approved-pre-selfhost`

This scope defines how the Arcana standard library is interpreted during the rewrite.

Scope notes:
- `std` is a rewrite-owned first-party library surface. It must be rebuilt for Arcana's architecture rather than inherited mechanically from MeadowLang imports.
- Imported MeadowLang `std` files are behavioral seed corpus only. They may preserve useful behavior examples, but they do not define correct layering, public surface, or package boundaries by themselves.
- The source language is mostly independent from the exact `std` module inventory. Rebuilding `std` should not require language churn unless a surface is already frozen elsewhere in the language contract.
- What *does* affect the language/runtime contract is the subset explicitly ratified by active scope docs, the selfhost language matrix, and any compiler/runtime seams that are intentionally first-party.
- Bootstrap readiness and transitional carried status are tracked in `docs/specs/std/std/v1-status.md`.
- Deferred std work is tracked in `docs/specs/std/std/deferred-roadmap.md`.
- The current approved `std` domain inventory, public surface shape, and split `std.kernel.*` topology are now considered frozen for the pre-selfhost rewrite unless a concrete roadmap blocker proves they must change.

## Governing Rules

- Do not treat imported `std` modules as architecture authority just because carried grimoires/examples compile against them.
- Keep language syntax/semantics separate from library layering wherever possible. `std` shape should follow the rewrite architecture, not the other way around.
- If a `std` surface is needed before selfhost, it should be approved by scope and owned by rewrite architecture.
- If a helper is primarily for examples, showcases, bootstrap convenience, or temporary corpus carryover, it should not become default `std` contract without explicit ratification.
- Imported-std review must preserve Arcana's explicit and unambiguous doctrine: prefer narrowly named, typed, auditable surfaces over convenience bundles, implicit policy, or backend-shaped leakage.
- If bootstrap seams such as typed opaque app/runtime handles are later replaced, the successor model must stay explicit about resource family, ownership/validity expectations, and diagnostics; no erased generic-handle fallback is permitted.
- Third-party Rust crates may sit under the implementation, but they must remain replaceable private details. Public `std` must not collapse into wrapper-shaped mirrors of crate APIs or crate-specific semantics.
- Kernel/intrinsic bindings are implementation seams, not public-library design guidance.
- Kernel/intrinsic bindings should stay split by runtime domain and should carry failure through operation-local `Result[...]` returns; do not reintroduce a catch-all host bucket or out-of-band global error slot for args/env/path/fs/process/resource failure state.
- Every `std` surface change must update this scope or `docs/specs/std/std/v1-status.md` in the same patch.
- After this freeze point, `std` changes before selfhost should be limited to:
  - contract-preserving bug fixes,
  - runtime/backend implementation work that satisfies the already approved surface,
  - narrowly justified additions or corrections proven necessary by Milestone 6 or by owned grimoire development on approved domains.

## Approved First-Party `std` Domains Before Selfhost

- Core host packages approved in `docs/specs/selfhost-host/selfhost-host/v1-scope.md`
- First-party app/runtime substrate approved in `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md`
- ECS/runtime surface:
  - `std.ecs`
  - `std.behaviors`
  - `std.behavior_traits`
  - plus the corresponding `behavior[...] fn` / `system[...] fn` language/runtime contract already frozen elsewhere
- Concurrency/runtime surface already required by the frozen matrix:
  - `std.concurrent`
- Core value/container/text support that is required by the carried compiler/tooling corpus and does not conflict with rewrite architecture:
  - `std.result`
  - `std.option`
  - `std.bytes`
  - `std.text`
  - `std.iter`
  - `std.collections.*`
  - `std.memory`
- Toolchain/bootstrap support that remains required before selfhost:
  - `std.manifest`
    - scoped to Arcana `book.toml` / `Arcana.lock` parsing helpers
    - not a generic TOML/JSON/YAML/serialization surface
- Shared low-level types needed by the app/runtime substrate:
  - `std.types.core`
- Low-level time and audio substrate needed by future Arcana-owned grimoire layers:
  - `std.time`
  - `std.audio`

## Not Yet Ratified As Rewrite-Defining `std`

- `std.app` fixed-step helpers
- `std.tooling` local planning helpers
- `std.types.game`
- Showcase/game/demo convenience helpers that leaked into imported `std`
- Compiler-bootstrap escape hatches exposed through public `std`
- Meadow-era layering decisions that combined runtime substrate, app helpers, and showcase logic into one flat standard surface

## Layering Intent

- `std.kernel.*`
  - implementation-facing intrinsic/runtime seam
  - not the public design center for `std`
- approved public `std.*`
  - rewrite-owned first-party library surface
  - stable enough to support the selfhost path
- grimoires/examples
  - consumer/reference corpus
  - valid source of behavioral examples, but not authority for `std` layering

## Rewrite Guidance

- Rebuild `std` around what Arcana itself needs:
  - language-adjacent runtime surface
  - deterministic toolchain support
  - first-party host support
  - first-party app/runtime substrate for real showcases
  - ECS/runtime extras that are intentionally part of Arcana's direction
- The pre-selfhost `std` freeze is a real reusable baseline for future Arcana libraries in the approved domains, not a temporary surface meant only for owned grimoires or bootstrap demos.
- It is acceptable for Arcana to add more std domains later through explicit scope, but the domains approved here should already be shaped as forward-looking third-party library substrate rather than one-off migration scaffolding.
- Public std additions are allowed before selfhost when they are genuinely bootstrap-required or substrate-required, but they must be documented as such in `docs/specs/std/std/v1-status.md`.
- Move app/demo/showcase-specific convenience back out of `std` unless it earns explicit first-party scope.
- Avoid broad root/prelude reexports of unratified convenience layers.
- If Milestone 6 or owned-grimoire work discovers a missing substrate capability, prefer adding it inside an already approved domain rather than reopening top-level std architecture.
