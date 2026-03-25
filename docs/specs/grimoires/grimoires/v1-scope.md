# Arcana Grimoire Roles v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the required future Arcana-owned app/media grimoire roles before bootstrap.

Scope notes:
- This file freezes responsibilities, not long-term package names.
- Historical MeadowLang grimoires are archived outside the repo and are not rewrite authority.
- Future Arcana-owned grimoires sit above `std` and consume rewrite-owned substrate. They must not force `std` to absorb framework-level policy by accident.
- Rewrite-owned app/media grimoires live under `grimoires/owned/*`.
- Historical MeadowLang package layout is not a promise that Arcana's final selfhost/bootstrap package layout matches Meadow-era directory splits.
- App/media grimoire topology remains flexible as long as the approved substrate and required capabilities are preserved.

## Required Roles Before Selfhost

- Desktop/app-shell grimoire
  - rewrite-owned package: `grimoires/owned/libs/arcana-desktop`
  - responsibility: Arcana-owned public desktop/window/event-loop boundary for native desktop apps, with winit-class role breadth over rewrite-owned substrate rather than a thin wrapper above a separately-public raw desktop layer
  - responsibility: may own the canonical session runner, raw window/session/event/wake contracts, blocking wait policy, multi-window coordination, input/timing helpers, monitor/clipboard helpers, event routing, frame-input snapshots, optional ECS-loop adapters, keybind/action helpers, and similar desktop-shell utilities if Arcana's rewrite-native layout folds those into one package
  - note: window events remain window-ID centric; `TargetedEvent.window_id` is the authoritative routing identity, and any higher-level "main window" convenience must stay phase-separated from callback dispatch and may only promote another live window after the callback/reconcile phase
  - note: `std.window`, `std.input`, `std.events`, `std.canvas`, `std.time`, and `std.clipboard` remain rewrite-owned substrate and backend-support layers; future desktop apps and higher grimoires should normally treat `arcana_desktop` as the app-shell package boundary
  - responsibility: must not absorb graphics/text draw policy that belongs in sibling grimoires above the shared low-level substrate
- Graphics grimoire
  - rewrite-owned package: `grimoires/owned/libs/arcana-graphics`
  - responsibility: Arcana-owned 2D graphics/image boundary above `std.canvas`, with small skia-class role breadth rather than a thin wrapper over raw canvas primitives
- Text grimoire
  - rewrite-owned package: `grimoires/owned/libs/arcana-text`
  - responsibility: Arcana-owned text draw, shaping, layout, and text-asset boundary above `std.canvas`, `std.text`, and `std.fs`, with cosmic-text-class role breadth rather than label-only wrappers
  - note: file IO remains in `std.fs`; this layer may add text-asset convenience rather than replace host-core file APIs
- Audio grimoire
  - rewrite-owned package: `grimoires/owned/libs/arcana-audio`
  - responsibility: Arcana-owned playback/audio boundary above `std.audio`, with miniaudio-class role breadth rather than a bootstrap-only shim

## Rules

- Future Arcana-owned grimoires may add ergonomic layers, but they must consume the rewrite-owned std substrate rather than relying on compiler special cases.
- If a future Arcana-owned grimoire repeatedly needs the same low-level capability, that may justify a std-scope update only when the need is clearly substrate-level.
- Arcana-owned grimoire replacement or renaming is allowed before selfhost as long as the required role remains satisfied and the status ledger is updated.
- Arcana-owned grimoire merging or splitting is allowed before selfhost as long as the required responsibilities remain satisfied and the status ledger is updated.
- Required Arcana-owned grimoires must not become thin public adapters over third-party Rust crates. If they use Rust crates underneath, those crates must remain replaceable implementation details under Arcana-owned contracts.
- Archived historical corpus may be consulted manually, but it does not define required future grimoire roles or package decomposition.

## Not Required In This Bumper Plan

- UI framework grimoires
- gameplay helper grimoires
- physics grimoires
- networking grimoires
- Lua/SQL interop grimoires
- post-selfhost ecosystem planning
