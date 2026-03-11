# Bootstrap Std Plan With Arcana-Owned App/Media Grimoire Roles

## Summary

Rebuild `std` now as a rewrite-owned, pre-selfhost substrate that is sufficient for:
- bootstrapping against the reference compiler/tooling corpus on the Rust rewrite
- supporting a later 5k-10k LOC desktop showcase game through future Arcana-owned grimoire layers built on top of `std`

This plan is explicitly pre-selfhost only. It does not attempt to define post-selfhost std or the full long-range ecosystem. The contract is: build the minimum stable first-party substrate and the minimum required future Arcana-owned app/media grimoire roles, document it heavily, and leave update notes wherever the shape may evolve later.

Reference boundary:
- `grimoires/reference/*` is carried reference corpus only.
- The reference packages may be checked or mined for behavior pressure during the rewrite, but they are not the active Arcana package layout.
- Rewrite-owned app/media grimoires live under `grimoires/owned/*`.

## Required Docs And Ownership

- Keep `docs/specs/std/std/v1-scope.md` as the authoritative std contract.
- Add `docs/specs/std/std/v1-status.md` as the living bootstrap/readiness ledger.
- Add `docs/specs/std/std/deferred-roadmap.md` for deferred std work.
- Add `docs/specs/grimoires/grimoires/v1-scope.md` to freeze the required future Arcana-owned app/media grimoire roles before bootstrap.
- Add `docs/specs/grimoires/grimoires/v1-status.md` to track which future Arcana-owned app/media grimoire roles are reference-backed, rewrite-owned, transitional, or still missing.
- Register those docs in `docs/specs/spec-status.md`.
- Update `PLAN.md`, `docs/rewrite-roadmap.md`, and `README.md` to point at the std scope/status docs and the grimoires scope/status docs.
- Add a standing rule: every std or Arcana-owned grimoire surface change must update the relevant scope or status ledger in the same patch.
- Add a standing rule: transitional reference-backed modules must carry an explicit `update note` in docs stating what should be revisited and what would cause promotion, rewrite, relocation, or removal.

## Std Contract To Build Now

- Host/core std:
  - `std.args`, `std.env`, `std.path`, `std.fs`, `std.process`, `std.io`
- Core runtime/data std:
  - `std.result`, `std.option`, `std.bytes`, `std.text`, `std.iter`, `std.collections.*`, `std.memory`
- ECS/runtime std:
  - `std.ecs`, `std.behaviors`, `std.behavior_traits`
- Low-level app/media std:
  - `std.window`, `std.input`, `std.events`, `std.canvas`
  - new `std.time`
  - new `std.audio`
- Shared low-level types:
  - geometry/color/time/frame wrappers and opaque media handles in `std.types.core` or equivalent low-level type modules

Required behavior of the low-level app/media std:
- `std.window` owns raw window lifecycle/state/control only.
- `std.input` owns raw keyboard/mouse polling and code lookup only.
- `std.events` owns the typed event queue and frame pump boundary.
- `std.canvas` owns primitive render, text draw, image load/size/blit, and simple graphical primitives.
- `std.time` owns monotonic time, durations, and frame-timing primitives.
- `std.audio` owns low-level audio output/device/buffer/playback primitives, only far enough to support a later higher-level audio grimoire.
- ECS remains first-party Arcana surface and is not treated as a showcase helper.

Not rewrite-defining std in this plan:
- `std.app`
- `std.tooling`
- compiler/bootstrap escape hatches exposed through public std
- app/game/demo convenience helpers
- higher-level desktop framework APIs
- broad gameplay/math/physics/network kits

## Required Arcana-Owned App/Media Grimoire Roles

Freeze responsibilities now, but do not freeze long-term package names yet.

Required pre-selfhost Arcana-owned grimoire roles:
- Desktop app facade grimoire:
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-desktop`
  - current reference corpus: `grimoires/reference/app/winspell`
  - owns ergonomic desktop/window/run-loop/frame convenience above low-level std
  - must not force std to absorb framework-level policies
- Graphics facade grimoire:
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-graphics`
  - current reference corpus: `grimoires/reference/app/winspell`
  - owns 2D graphics/image convenience above `std.canvas`
- Text facade grimoire:
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-text`
  - current reference corpus: `grimoires/reference/app/winspell`
  - owns text draw and text-asset convenience above `std.canvas`, `std.text`, and `std.fs`
  - file IO remains in `std.fs`; this layer may add text-asset convenience only
- Audio facade grimoire:
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-audio`
  - current reference corpus: `grimoires/reference/app/spell-audio`
  - owns miniaudio-style higher-level playback/convenience above `std.audio`
  - must be buildable without expanding std into a full audio framework

Not required in this bumper plan:
- UI framework grimoire
- gameplay helper grimoire
- physics grimoire
- networking grimoire
- Lua/SQL interop grimoire work
- post-selfhost ecosystem planning

## Classification And Tracking Rules

Every std module and required Arcana-owned app/media grimoire role must be classified in its status ledger as:
- `bootstrap-required`
- `transitional-carried`
- `deferred`

Each entry must record:
- why it exists
- which concrete consumers require it
- whether the current implementation is rewrite-owned, reference-backed, missing, or mixed
- what still needs to be rebuilt
- what note should be revisited later
- what condition would promote it out of transitional status

Defaults for this plan:
- bootstrap-required public std additions are allowed before selfhost if they are tied to a real compiler/tooling or showcase consumer
- prefer public std for true substrate-level bootstrap needs
- prefer Arcana-owned grimoires for convenience layers
- keep names flexible, freeze responsibilities

## Implementation Sequence

1. Finalize docs first.
- std scope
- std status ledger
- std deferred roadmap
- app/media grimoire-role scope
- app/media grimoire-role status ledger

2. Inventory the reference corpus against real consumers.
- `std/src`
- `grimoires/reference/toolchain/*/src` as bootstrap/reference pressure only
- current window/input/event/showcase examples

3. Classify each current std module and each required grimoire role.
- bootstrap-required
- transitional-carried
- deferred

4. Narrow obviously wrong public surface immediately.
- no compiler/bootstrap escape hatches in public std
- no broad prelude/root reexports of unratified convenience layers
- no framework policies leaking into low-level std

5. Rebuild bootstrap-required host/core and runtime/data std.

6. Rebuild bootstrap-required app/media std.
- window
- input
- events
- canvas
- time
- audio
- low-level geometry/color/time/media handle types

7. Rebuild or replace the required Arcana-owned app/media grimoire layers on top of that substrate.
- desktop facade
- graphics facade
- text facade
- audio facade

8. Validate against the intended consumer path.
- host/bootstrap tooling
- reference compiler/selfhost corpus compatibility
- window/input/event demo
- primitive render/text/image demo
- basic audio smoke demo
- ECS-driven desktop showcase path

9. Update docs after every classification or surface change.
- if a reference-backed surface is retained, record why
- if a surface is provisional, record what triggers revisiting it
- if a surface is deferred, record why it is not needed before selfhost

## Public Interfaces And Types

Public std categories to expose or ratify in this plan:
- host/file/path/process status/output
- result/option/text/bytes/iter/collections/memory
- ECS/behavior stepping and component/entity helpers
- raw window/input/event/canvas/time/audio substrate
- low-level geometry/color/time/frame wrappers and opaque media handles

Everything above that belongs in Arcana-owned grimoire layers, not std.

## Test Plan

- Doc checks:
  - std scope, std status, std deferred roadmap, grimoires scope, and grimoires status all exist and are cross-linked
  - every bootstrap-required std item and grimoire role has a concrete consumer
- Reference-corpus compatibility checks:
  - `grimoires/reference/toolchain/arcana-frontend` remains checkable as migration corpus pressure, not as active package layout
  - `grimoires/reference/toolchain/arcana-compiler-core` remains checkable as migration corpus pressure, not as active package layout
  - `grimoires/reference/toolchain/arcana-selfhost-compiler` remains checkable as migration corpus pressure, not as active package layout
  - `grimoires/reference/examples/selfhost_host_tool_mvp`
  - `grimoires/reference/examples/selfhost_frontend_mvp`
- Runtime smoke checks:
  - file/path/process host tool flow
  - window open/close/input/events
  - rect/text/image render
  - basic audio playback
  - ECS/behaviors on the same toolchain/runtime path
- Showcase-readiness checks:
  - a desktop showcase/game can be built through Arcana-owned grimoire layers without adding a new std category
  - any newly discovered need is first evaluated as grimoire-layer work, and only promoted into std if it is clearly substrate-level

## Assumptions

- This is a bootstrap-ready, pre-selfhost bumper plan only.
- `std` must support actual bootstrapping, not just generic app development.
- Post-selfhost std expansion is expected but intentionally not designed here.
- Heavy documentation and explicit update notes are part of the deliverable, not follow-up polish.
- Arcana-owned app/media grimoire responsibilities are frozen now, but long-term package names remain flexible.
