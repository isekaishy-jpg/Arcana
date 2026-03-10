# Bootstrap Std Plan With First-Party Grimoire Inventory

## Summary

Rebuild `std` now as a rewrite-owned, pre-selfhost substrate that is sufficient for:
- bootstrapping the carried compiler/tooling grimoires on the Rust rewrite
- supporting a later 5k-10k LOC desktop showcase game through first-party grimoires built on top of `std`

This plan is explicitly pre-selfhost only. It does not attempt to define post-selfhost std or the full long-range ecosystem. The contract is: build the minimum stable first-party substrate and the minimum required first-party grimoire set, document it heavily, and leave update notes wherever the shape may evolve later.

## Required Docs And Ownership

- Keep `docs/specs/std/std/v1-scope.md` as the authoritative std contract.
- Add `docs/specs/std/std/v1-status.md` as the living bootstrap/readiness ledger.
- Add `docs/specs/std/std/deferred-roadmap.md` for deferred std work.
- Add `docs/specs/grimoires/grimoires/v1-scope.md` to freeze the required first-party grimoire roles before bootstrap.
- Add `docs/specs/grimoires/grimoires/v1-status.md` to track which first-party grimoire roles are rewrite-owned, carried, transitional, or still missing.
- Register those docs in `docs/specs/spec-status.md`.
- Update `PLAN.md`, `docs/rewrite-roadmap.md`, and `README.md` to point at the std scope/status docs and the grimoires scope/status docs.
- Add a standing rule: every std or first-party grimoire surface change must update the relevant scope or status ledger in the same patch.
- Add a standing rule: transitional carried modules must carry an explicit `update note` in docs stating what should be revisited and what would cause promotion, rewrite, relocation, or removal.

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

## Required First-Party Grimoires

Freeze responsibilities now, but do not freeze long-term package names yet.

Required pre-selfhost first-party grimoire roles:
- Frontend grimoire:
  - current role of `arcana-frontend`
  - must consume rewrite std without hidden compiler special cases
- Compiler-core grimoire:
  - current role of `arcana-compiler-core`
  - must consume rewrite std and the rewrite backend/toolchain seams needed for bootstrap
- Selfhost-compiler grimoire:
  - current role of `arcana-selfhost-compiler`
  - must compile on the rewrite-owned std/bootstrap path
- Desktop app facade grimoire:
  - successor role to `winspell`
  - owns ergonomic desktop/window/run-loop/frame convenience above low-level std
  - must not force std to absorb framework-level policies
- Event/input utility grimoire:
  - successor role to `spell-events`
  - owns event routing, frame input snapshots, keybind/action helpers, and event convenience above `std.events` and `std.input`
- Audio facade grimoire:
  - new required role
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

Every std module and required first-party grimoire role must be classified in its status ledger as:
- `bootstrap-required`
- `transitional-carried`
- `deferred`

Each entry must record:
- why it exists
- which concrete consumers require it
- whether the current implementation is rewrite-owned, carried, missing, or mixed
- what still needs to be rebuilt
- what note should be revisited later
- what condition would promote it out of transitional status

Defaults for this plan:
- bootstrap-required public std additions are allowed before selfhost if they are tied to a real compiler/tooling or showcase consumer
- prefer public std for true substrate-level bootstrap needs
- prefer grimoires for convenience layers
- keep names flexible, freeze responsibilities

## Implementation Sequence

1. Finalize docs first.
- std scope
- std status ledger
- std deferred roadmap
- first-party grimoires scope
- first-party grimoires status ledger

2. Inventory the carried repo against real consumers.
- `std/src`
- `grimoires/arcana-frontend/src`
- `grimoires/arcana-compiler-core/src`
- `grimoires/arcana-selfhost-compiler/src`
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

7. Rebuild or replace the required first-party grimoires on top of that substrate.
- frontend
- compiler-core
- selfhost-compiler
- desktop facade
- event/input utility
- audio facade

8. Validate against the intended consumer path.
- host/bootstrap tooling
- selfhost compiler grimoires
- window/input/event demo
- primitive render/text/image demo
- basic audio smoke demo
- ECS-driven desktop showcase path

9. Update docs after every classification or surface change.
- if a carried surface is retained, record why
- if a surface is provisional, record what triggers revisiting it
- if a surface is deferred, record why it is not needed before selfhost

## Public Interfaces And Types

Public std categories to expose or ratify in this plan:
- host/file/path/process status/output
- result/option/text/bytes/iter/collections/memory
- ECS/behavior stepping and component/entity helpers
- raw window/input/event/canvas/time/audio substrate
- low-level geometry/color/time/frame wrappers and opaque media handles

Everything above that belongs in first-party grimoires, not std.

## Test Plan

- Doc checks:
  - std scope, std status, std deferred roadmap, grimoires scope, and grimoires status all exist and are cross-linked
  - every bootstrap-required std item and grimoire role has a concrete consumer
- Rewrite/toolchain checks:
  - `grimoires/arcana-frontend`
  - `grimoires/arcana-compiler-core`
  - `grimoires/arcana-selfhost-compiler`
  - `examples/selfhost_host_tool_mvp`
  - `examples/selfhost_frontend_mvp`
- Runtime smoke checks:
  - file/path/process host tool flow
  - window open/close/input/events
  - rect/text/image render
  - basic audio playback
  - ECS/behaviors on the same toolchain/runtime path
- Showcase-readiness checks:
  - a desktop showcase/game can be built through grimoires without adding a new std category
  - any newly discovered need is first evaluated as grimoire-level, and only promoted into std if it is clearly substrate-level

## Assumptions

- This is a bootstrap-ready, pre-selfhost bumper plan only.
- `std` must support actual bootstrapping, not just generic app development.
- Post-selfhost std expansion is expected but intentionally not designed here.
- Heavy documentation and explicit update notes are part of the deliverable, not follow-up polish.
- First-party grimoire responsibilities are frozen now, but long-term package names remain flexible.
