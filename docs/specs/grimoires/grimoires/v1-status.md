# Arcana Grimoire Roles v1 Status

Status: `approved-pre-selfhost`

This ledger tracks bootstrap-readiness for required future Arcana-owned app/media grimoire roles.

Rules:
- Every Arcana-owned grimoire role change must update this ledger or `docs/specs/grimoires/grimoires/v1-scope.md` in the same patch.
- Carried roles must include an update note explaining what still needs to be rebuilt or replaced before they can be treated as rewrite-owned.
- This ledger may classify required app/media roles; it may not expand the required grimoire set by itself.
- Archived historical MeadowLang corpus is bootstrap context only, not future grimoire architecture.

id: GRIMOIRE-DESKTOP-APP-SHELL
classification: bootstrap-required
role: desktop/app-shell grimoire
current_scaffold: `grimoires/owned/libs/arcana-desktop`
historical_seed: archived MeadowLang desktop app corpus
why: Arcana-owned public desktop/window/event-loop boundary for native desktop apps above the low-level app/runtime substrate
current_source: rewrite-owned-in-progress
still_needs_rebuild: broaden the current real session runner/mailbox/window layer into the full long-term desktop contract, with remaining parity work focused on additional low-level window config knobs and richer input metadata only where future grimoires prove those belong in shared `std.*` substrate rather than desktop-only shortcuts; keep raw app-shell power in `arcana_desktop` itself instead of splitting the public boundary across a weaker wrapper plus raw `std.*`
update_note: the scaffold is now replaced by a real rewrite-owned app-shell package with static `Application[...]` runner callbacks, session-backed window orchestration, blocking wait support, configurable wait slicing, wake/mailbox helpers, monitor wrappers, clipboard wrappers, settings-facing window and text-input records, cursor/theme hooks, richer key metadata and composition-event routing, optional ECS adapter helpers, and native bundle proof; naming and package split may still change, but fixed-step/frame-loop convenience stays here rather than in `std.app`, and the remaining parity work is contract growth plus substrate hardening rather than reviving Meadow-era runtime assumptions. This package is the canonical public desktop boundary and may expose raw window/event/session power directly where the contract needs it instead of forcing apps through a thin facade over separately-public `std.*` shell APIs. Current crate-side proof now includes native bundle execution through the package itself, a simple `arcana_desktop + arcana_graphics + arcana_text` sample path, native multi-window clipboard coverage, native settings/text-input roundtrip plus create-time cursor/text-input config proof through the package, committed-text plus IME lifecycle smoke on the real host message path, and in-process native host proof for committed IME composition payload delivery.
promotion_condition: the rewrite-owned desktop/app-shell package reaches the approved long-term role breadth and powers native showcase apps as the normal public shell boundary, with `std.*` serving as substrate/support rather than the primary app-facing desktop API

id: GRIMOIRE-GRAPHICS-FACADE
classification: bootstrap-required
role: graphics facade grimoire
current_scaffold: `grimoires/owned/libs/arcana-graphics`
historical_seed: archived MeadowLang desktop app corpus
why: 2D graphics/image convenience above the low-level canvas substrate
current_source: scaffolded-rewrite-owned
still_needs_rebuild: grow the facade beyond direct wrapper shape and keep it aligned with approved `std.canvas` primitives
update_note: this layer should stay focused on graphics/image convenience and avoid turning into a retained-mode UI or scene framework by accident
promotion_condition: a rewrite-owned graphics facade exists and proves the low-level canvas/image substrate is sufficient for showcase work

id: GRIMOIRE-TEXT-FACADE
classification: bootstrap-required
role: text facade grimoire
current_scaffold: `grimoires/owned/libs/arcana-text`
historical_seed: archived MeadowLang desktop app corpus
why: text draw and text-asset convenience above `std.canvas`, `std.text`, and `std.fs`
current_source: scaffolded-rewrite-owned
still_needs_rebuild: grow the facade beyond label wrappers while keeping file IO itself in `std.fs`
update_note: file IO remains host-core std surface; text asset helpers may layer on top, but this grimoire should not redefine `std.fs`
promotion_condition: a rewrite-owned text facade exists and proves text draw plus text-asset flows without pushing file APIs into grimoire policy

id: GRIMOIRE-AUDIO-FACADE
classification: bootstrap-required
role: audio facade grimoire
current_scaffold: `grimoires/owned/libs/arcana-audio`
historical_seed: archived MeadowLang audio app corpus
why: higher-level playback/convenience layer above the low-level `std.audio` substrate
current_source: scaffolded-rewrite-owned
still_needs_rebuild: keep the facade thin and bootstrap-oriented until the runtime/backend audio seam exists for real smoke execution
update_note: keep ergonomic playback policy here instead of expanding `std.audio` into a full framework
promotion_condition: a rewrite-owned audio facade grimoire builds against `std.audio`, grows beyond scaffold status, and powers a basic audio smoke demo
