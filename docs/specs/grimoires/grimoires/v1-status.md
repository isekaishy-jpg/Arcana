# Arcana First-Party Grimoires v1 Status

Status: `approved-pre-selfhost`

This ledger tracks bootstrap-readiness for required first-party grimoire roles.

Rules:
- Every first-party grimoire role change must update this ledger or `docs/specs/grimoires/grimoires/v1-scope.md` in the same patch.
- Carried roles must include an update note explaining what still needs to be rebuilt or replaced before they can be treated as rewrite-owned.
- This ledger may classify required roles; it may not expand the required grimoire set by itself.

id: GRIMOIRE-FRONTEND
classification: bootstrap-required
role: frontend grimoire
current_seed: `grimoires/arcana-frontend`
why: bootstrap checker/frontend consumer that proves rewrite std and package/runtime substrate are usable from Arcana code
current_source: transitional-carried
still_needs_rebuild: move off carried helper assumptions that do not belong in the approved std surface
update_note: keep this grimoire consuming rewrite-owned std only; do not preserve carried std convenience drift just to avoid rewrites here
promotion_condition: the grimoire builds and runs on rewrite-owned std/package/runtime seams without legacy-only assumptions

id: GRIMOIRE-COMPILER-CORE
classification: bootstrap-required
role: compiler-core grimoire
current_seed: `grimoires/arcana-compiler-core`
why: bootstrap compiler-core consumer that exercises std, manifests, files, text, and backend-adjacent flows
current_source: transitional-carried
still_needs_rebuild: remove dependence on carried helper expansions and old direct-emit/runtime assumptions where they conflict with the rewrite
update_note: preserve only the behavior needed for bootstrap, not Meadow-era architecture
promotion_condition: the grimoire runs on rewrite-owned std/backend seams and no longer depends on carried architecture shortcuts

id: GRIMOIRE-SELFHOST-COMPILER
classification: bootstrap-required
role: selfhost-compiler grimoire
current_seed: `grimoires/arcana-selfhost-compiler`
why: end-goal compiler corpus consumer for declaring selfhost
current_source: transitional-carried
still_needs_rebuild: align with rewrite-owned std/package/runtime surface and eliminate dependency on carried-only std helpers that are not ratified
update_note: this is the closure target for the bootstrap path; keep its needs explicit in std and grimoire ledgers
promotion_condition: the grimoire builds and verifies on the rewrite-owned toolchain without fallback to MeadowLang

id: GRIMOIRE-DESKTOP-FACADE
classification: bootstrap-required
role: desktop app facade grimoire
current_seed: `grimoires/winspell`
why: ergonomic desktop/window/run-loop/frame layer above the low-level app/runtime substrate
current_source: transitional-carried
still_needs_rebuild: replace Meadow-era runtime/backend assumptions and bind only to approved `std.window` / `std.input` / `std.events` / `std.canvas` / `std.time`
update_note: naming may change; the required role is what is frozen here, not the carried package name, and fixed-step/frame-loop convenience should live here rather than in `std.app`
promotion_condition: a rewrite-owned desktop facade grimoire exists and no longer depends on Meadow-era implementation assumptions

id: GRIMOIRE-EVENT-UTILITY
classification: bootstrap-required
role: event/input utility grimoire
current_seed: `grimoires/spell-events`
why: event routing, frame input snapshots, and keybind/action helpers above the low-level event/input substrate
current_source: transitional-carried
still_needs_rebuild: bind only to approved `std.events` / `std.input` semantics and move no extra framework policy into std or through the desktop facade
update_note: keep this as a utility layer above std; do not widen `std.events` just to preserve carried helper shapes, and avoid unnecessary coupling to `winspell`
promotion_condition: a rewrite-owned event/input utility grimoire exists and proves the low-level substrate is sufficient

id: GRIMOIRE-AUDIO-FACADE
classification: bootstrap-required
role: audio facade grimoire
current_seed: `grimoires/spell-audio`
why: higher-level playback/convenience layer above the low-level `std.audio` substrate
current_source: rewrite-owned
still_needs_rebuild: keep the facade thin and bootstrap-oriented until the runtime/backend audio seam exists for real smoke execution
update_note: keep ergonomic playback policy here instead of expanding `std.audio` into a full framework
promotion_condition: a rewrite-owned audio facade grimoire builds against `std.audio` and powers a basic audio smoke demo
