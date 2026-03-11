# Arcana Grimoire Roles v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the required future Arcana-owned app/media grimoire roles before bootstrap.

Scope notes:
- This file freezes responsibilities, not long-term package names.
- Carried MeadowLang grimoires under `grimoires/reference/*` are behavioral seed corpus only unless this scope ratifies an app/media role they currently illustrate.
- Future Arcana-owned grimoires sit above `std` and consume rewrite-owned substrate. They must not force `std` to absorb framework-level policy by accident.
- Rewrite-owned app/media grimoires live under `grimoires/owned/*`.
- The reference corpus under `grimoires/reference/*` is not a promise that Arcana's final selfhost/bootstrap package layout matches MeadowLang's directory split.
- The reference toolchain corpus under `grimoires/reference/toolchain/*` is for bootstrap pressure and selfhost validation, not for freezing future grimoire architecture.
- App/media grimoire topology remains flexible as long as the approved substrate and required capabilities are preserved.

## Required Roles Before Selfhost

- Desktop/media facade grimoire
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-desktop`
  - current carried seed role: `grimoires/reference/app/winspell`
  - responsibility: ergonomic desktop/window/run-loop/frame convenience above `std.window`, `std.input`, `std.events`, `std.canvas`, and `std.time`
  - responsibility: may also own event routing, frame-input snapshots, keybind/action helpers, and similar desktop-facing utility layers if Arcana's rewrite-native layout folds those into one package
- Graphics facade grimoire
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-graphics`
  - current carried seed role: `grimoires/reference/app/winspell`
  - responsibility: 2D graphics/image convenience above `std.canvas`
- Text facade grimoire
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-text`
  - current carried seed role: `grimoires/reference/app/winspell`
  - responsibility: text draw and text-asset convenience above `std.canvas`, `std.text`, and `std.fs`
  - note: file IO remains in `std.fs`; this layer may add text-asset convenience rather than replace host-core file APIs
- Audio facade grimoire
  - rewrite-owned scaffold: `grimoires/owned/app/arcana-audio`
  - current carried seed role: `grimoires/reference/app/spell-audio`
  - responsibility: miniaudio-style higher-level playback/convenience above `std.audio`

## Rules

- Future Arcana-owned grimoires may add ergonomic layers, but they must consume the rewrite-owned std substrate rather than relying on compiler special cases.
- If a future Arcana-owned grimoire repeatedly needs the same low-level capability, that may justify a std-scope update only when the need is clearly substrate-level.
- Arcana-owned grimoire replacement or renaming is allowed before selfhost as long as the required role remains satisfied and the status ledger is updated.
- Arcana-owned grimoire merging or splitting is allowed before selfhost as long as the required responsibilities remain satisfied and the status ledger is updated.
- Required Arcana-owned grimoires must not become thin public adapters over third-party Rust crates. If they use Rust crates underneath, those crates must remain replaceable implementation details under Arcana-owned contracts.
- Reference toolchain corpus may be validated against the rewrite, but it does not define required future grimoire roles or package decomposition.

## Not Required In This Bumper Plan

- UI framework grimoires
- gameplay helper grimoires
- physics grimoires
- networking grimoires
- Lua/SQL interop grimoires
- post-selfhost ecosystem planning
