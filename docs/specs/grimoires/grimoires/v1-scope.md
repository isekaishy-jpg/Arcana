# Arcana First-Party Grimoires v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the required first-party grimoire roles before bootstrap.

Scope notes:
- This file freezes responsibilities, not long-term package names.
- Carried MeadowLang grimoires are behavioral seed corpus only unless this scope ratifies their role.
- First-party grimoires sit above `std` and consume rewrite-owned substrate. They must not force `std` to absorb framework-level policy by accident.

## Required Roles Before Selfhost

- Frontend grimoire
  - current carried role: `grimoires/arcana-frontend`
  - responsibility: Arcana-authored frontend/checker consumer of rewrite-owned std and package/runtime substrate
- Compiler-core grimoire
  - current carried role: `grimoires/arcana-compiler-core`
  - responsibility: Arcana-authored compiler-core consumer of rewrite-owned std and backend/toolchain seams needed for bootstrap
- Selfhost-compiler grimoire
  - current carried role: `grimoires/arcana-selfhost-compiler`
  - responsibility: selfhost compiler consumer of rewrite-owned std/bootstrap path
- Desktop app facade grimoire
  - current carried seed role: `grimoires/winspell`
  - responsibility: ergonomic desktop/window/run-loop/frame convenience above `std.window`, `std.input`, `std.events`, `std.canvas`, and `std.time`
- Event/input utility grimoire
  - current carried seed role: `grimoires/spell-events`
  - responsibility: event routing, frame-input snapshots, keybind/action helpers, and event convenience above `std.events` and `std.input`
- Audio facade grimoire
  - current rewrite-owned seed role: `grimoires/spell-audio`
  - responsibility: miniaudio-style higher-level playback/convenience above `std.audio`

## Rules

- First-party grimoires may add ergonomic layers, but they must consume the rewrite-owned std substrate rather than relying on compiler special cases.
- If a first-party grimoire repeatedly needs the same low-level capability, that may justify a std-scope update only when the need is clearly substrate-level.
- First-party grimoire replacement or renaming is allowed before selfhost as long as the required role remains satisfied and the status ledger is updated.
- Required first-party grimoires must not become thin public adapters over third-party Rust crates. If they use Rust crates underneath, those crates must remain replaceable implementation details under Arcana-owned contracts.

## Not Required In This Bumper Plan

- UI framework grimoires
- gameplay helper grimoires
- physics grimoires
- networking grimoires
- Lua/SQL interop grimoires
- post-selfhost ecosystem planning
