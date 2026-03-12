# Arcana Grimoire Roles v1 Status

Status: `approved-pre-selfhost`

This ledger tracks bootstrap-readiness for required future Arcana-owned app/media grimoire roles.

Rules:
- Every Arcana-owned grimoire role change must update this ledger or `docs/specs/grimoires/grimoires/v1-scope.md` in the same patch.
- Carried roles must include an update note explaining what still needs to be rebuilt or replaced before they can be treated as rewrite-owned.
- This ledger may classify required app/media roles; it may not expand the required grimoire set by itself.
- Archived historical MeadowLang corpus is bootstrap context only, not future grimoire architecture.

id: GRIMOIRE-DESKTOP-FACADE
classification: bootstrap-required
role: desktop/media facade grimoire
current_scaffold: `grimoires/owned/app/arcana-desktop`
historical_seed: archived MeadowLang desktop app corpus
why: ergonomic desktop/window/run-loop/frame layer above the low-level app/runtime substrate
current_source: scaffolded-rewrite-owned
still_needs_rebuild: flesh out the rewrite-owned facade and replace Meadow-era runtime/backend assumptions with owned package behavior above approved `std.window` / `std.input` / `std.events` / `std.canvas` / `std.time`
update_note: naming and package split may change; the required role is what is frozen here, not the carried package name or Meadow-era package boundary, and fixed-step/frame-loop convenience should live here rather than in `std.app`
promotion_condition: the rewrite-owned desktop/media facade grows beyond scaffold status and no longer depends on Meadow-era implementation assumptions

id: GRIMOIRE-GRAPHICS-FACADE
classification: bootstrap-required
role: graphics facade grimoire
current_scaffold: `grimoires/owned/app/arcana-graphics`
historical_seed: archived MeadowLang desktop app corpus
why: 2D graphics/image convenience above the low-level canvas substrate
current_source: scaffolded-rewrite-owned
still_needs_rebuild: grow the facade beyond direct wrapper shape and keep it aligned with approved `std.canvas` primitives
update_note: this layer should stay focused on graphics/image convenience and avoid turning into a retained-mode UI or scene framework by accident
promotion_condition: a rewrite-owned graphics facade exists and proves the low-level canvas/image substrate is sufficient for showcase work

id: GRIMOIRE-TEXT-FACADE
classification: bootstrap-required
role: text facade grimoire
current_scaffold: `grimoires/owned/app/arcana-text`
historical_seed: archived MeadowLang desktop app corpus
why: text draw and text-asset convenience above `std.canvas`, `std.text`, and `std.fs`
current_source: scaffolded-rewrite-owned
still_needs_rebuild: grow the facade beyond label wrappers while keeping file IO itself in `std.fs`
update_note: file IO remains host-core std surface; text asset helpers may layer on top, but this grimoire should not redefine `std.fs`
promotion_condition: a rewrite-owned text facade exists and proves text draw plus text-asset flows without pushing file APIs into grimoire policy

id: GRIMOIRE-AUDIO-FACADE
classification: bootstrap-required
role: audio facade grimoire
current_scaffold: `grimoires/owned/app/arcana-audio`
historical_seed: archived MeadowLang audio app corpus
why: higher-level playback/convenience layer above the low-level `std.audio` substrate
current_source: scaffolded-rewrite-owned
still_needs_rebuild: keep the facade thin and bootstrap-oriented until the runtime/backend audio seam exists for real smoke execution
update_note: keep ergonomic playback policy here instead of expanding `std.audio` into a full framework
promotion_condition: a rewrite-owned audio facade grimoire builds against `std.audio`, grows beyond scaffold status, and powers a basic audio smoke demo
