# Arcana Grimoire Roles v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the required future Arcana-owned app/media grimoire roles before bootstrap.

Scope notes:
- This file freezes responsibilities, not long-term package names.
- Historical MeadowLang grimoires are archived outside the repo and are not rewrite authority.
- Future Arcana-owned grimoires sit above `std` and consume rewrite-owned substrate. They must not force `std` to absorb framework-level policy by accident.
- The current repo layout uses `grimoires/libs/*` for app/media grimoires and `grimoires/arcana/*` for Arcana-owned host/binding grimoires.
- Historical MeadowLang package layout is not a promise that Arcana's final selfhost/bootstrap package layout matches Meadow-era directory splits.
- App/media grimoire topology remains flexible as long as the approved substrate and required capabilities are preserved.

## Required Roles Before Selfhost

- Desktop/app-shell grimoire
  - rewrite-owned package: `grimoires/libs/arcana-desktop`
  - responsibility: Arcana-owned public desktop/window/event-loop boundary for native desktop apps, with winit-class role breadth over rewrite-owned substrate rather than a thin wrapper above a separately-public raw desktop layer
  - responsibility: may own the canonical session runner, raw window/session/event/wake contracts, blocking wait policy, multi-window coordination, input/timing helpers, monitor helpers, clipboard helpers, event routing, frame-input snapshots, keybind/action helpers, IME/text-input hooks, and similar desktop-shell utilities if Arcana's rewrite-native layout folds those into one package
  - note: window events remain window-ID centric; `TargetedEvent.window_id` is the authoritative routing identity, and any higher-level "main window" convenience must stay phase-separated from callback dispatch and may only promote another live window after the callback/reconcile phase
  - note: `arcana_desktop` is the public desktop shell boundary; the historical parallel std desktop shell is retired rather than kept as a second public lane
  - responsibility: must not absorb graphics/text draw policy that belongs in sibling grimoires above the shared low-level substrate
- Graphics grimoire
  - rewrite-owned package: `grimoires/libs/arcana-graphics`
  - responsibility: Arcana-owned graphics/image boundary that may host multiple rendering backends, with small skia-class role breadth rather than a thin wrapper over retired canvas primitives
- Text grimoire
  - rewrite-owned package: `grimoires/libs/arcana-text`
  - responsibility: Arcana-owned text draw, shaping, layout, and text-asset boundary above graphics/backing surfaces plus `std.text` and `arcana_process.fs`, with cosmic-text-class role breadth rather than label-only wrappers
  - note: file IO remains in `arcana_process.fs`; this layer may add text-asset convenience rather than replace host-core file APIs
- Audio grimoire
  - rewrite-owned package: `grimoires/libs/arcana-audio`
  - responsibility: Arcana-owned public low-level audio boundary above the WinAPI-backed helper substrate, with miniaudio-class role breadth rather than a bootstrap-only shim

## Rules

- Future Arcana-owned grimoires may add ergonomic layers, but they must consume the rewrite-owned std substrate rather than relying on compiler special cases.
- Public owner grimoires may depend on binding-owned opaque handle types from `arcana_winapi`, but they must not keep duplicate handle declarations or alias-only public type paths alive once the binding-owned canonical modules exist.
- If a future Arcana-owned grimoire repeatedly needs the same low-level capability, that may justify a std-scope update only when the need is clearly substrate-level.
- Arcana-owned grimoire replacement or renaming is allowed before selfhost as long as the required role remains satisfied and the status ledger is updated.
- Arcana-owned grimoire merging or splitting is allowed before selfhost as long as the required responsibilities remain satisfied and the status ledger is updated.
- Required Arcana-owned grimoires must not become thin public adapters over third-party Rust crates. If they use Rust crates underneath, those crates must remain replaceable implementation details under Arcana-owned contracts.
- Archived historical corpus may be consulted manually, but it does not define required future grimoire roles or package decomposition.
- Generic OS-binding grimoires are tracked separately under the OS-binding scope; they are not folded into this app/media role list.

## Not Required In This Bumper Plan

- UI framework grimoires
- gameplay helper grimoires
- physics grimoires
- networking grimoires
- Lua/SQL interop grimoires
- post-selfhost ecosystem planning
