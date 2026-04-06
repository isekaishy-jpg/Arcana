# Record Head One-Shot Plan

## Summary
- Add a dedicated `record` headed-region family to cover the full second group in one pass: record construction, same-type refinement, and cross-record lift/copy by matching fields.
- Keep `construct` unchanged for generic constructor-driven work. Do not add `from` or record-specific refinement behavior to `construct`.
- Ship the full `record` family now: `yield`, `deliver`, and `place`.
- Make `record` the canonical first-party surface for record-specific structural work, while keeping existing `construct` record targets legal for v1 compatibility.

## Public Surface
- New headed-region head: `record`
- New shapes:
  - `record yield <RecordPath> -<default_modifier>`
  - `record yield <RecordPath> from <base_expr> -<default_modifier>`
  - `record deliver <RecordPath> -> <name> -<default_modifier>`
  - `record deliver <RecordPath> from <base_expr> -> <name> -<default_modifier>`
  - `record place <RecordPath> -> <target> -<default_modifier>`
  - `record place <RecordPath> from <base_expr> -> <target> -<default_modifier>`
- Participating lines are record field contributions only:
  - `field = expr`
  - `field = expr -<override_modifier>`
- `record` targets must resolve to record symbols only. No enum variants, opaque types, objects, or generic callable constructors.
- `record` uses the current construction-failure modifier family already used by `construct` record contributions:
  - same default/per-line modifier class and acquisition behavior
  - no new modifier family for v1

## Semantics
- `record` without `from`:
  - explicit field construction for a target record type
  - any non-optional target field not contributed is an error
  - omitted `Option[...]` fields use the same v1 completion/default behavior currently used by record-shaped `construct`
- `record ... from <base_expr>`:
  - explicit contributions override base-provided values
  - omitted target fields are copied from `base_expr` only when `base_expr` exposes a same-named field with exactly the same type
  - omitted fields not satisfied by base still follow normal target-field rules:
    - optional fields use the existing record default behavior
    - required fields are errors
- This is shallow structural copy only:
  - no recursive merge
  - no nested field path assignment
  - no inferred field renames
  - no implicit conversions
- Base precedence:
  - explicit field line wins
  - otherwise exact compatible field from base
  - otherwise optional/default behavior
  - otherwise missing required field error
- Deliver/place behavior:
  - same completion semantics as current headed-region family
  - `record place` target type must exactly match the record result type
  - `record deliver` binding must not collide with an existing binding in scope

## Implementation Changes
- Syntax:
  - add `record` as a headed-region head in the parser
  - parse the full completion family (`yield` / `deliver` / `place`)
  - parse optional `from <expr>` in the head
  - reuse the current construct-line shape for named field contributions
  - reject `payload = ...` under `record`
- Frontend:
  - add record-headed-region semantic validation parallel to current `construct`
  - resolve record target by type path, not constructor path
  - validate base compatibility field-by-field using exact name + exact type matching
  - preserve current modifier validation behavior for contribution lines
  - preserve existing nested-headed-region rejection
- IR/runtime:
  - lower `record` as its own headed-region kind, not a parser alias for `construct`
  - execution materializes a target record value, optionally seeded from a compatible base value, then applies explicit contributions in source order
  - runtime field-copy path must only read matching fields actually used by the target
  - native/AOT follows current generic headed-region behavior: runtime-backed execution inside native bundles is acceptable for v1
- Docs and canon:
  - update headed-region approved scope so `record` is part of the approved family
  - clarify the split:
    - `record` is record-only structural work
    - `construct` remains generic constructor-driven work
  - update `llm.md` to make `record` the canonical record refinement/construction surface
- Migration:
  - migrate first-party std/grimoire code that is doing record-specific copy/update/lift work to `record`
  - do not remove existing `construct` record support in this pass
  - do not add new first-party uses of `construct` for record refinement after this lands

## Test Plan
- Syntax:
  - parse all six header shapes
  - parse `from <expr>` with `yield`, `deliver`, and `place`
  - reject malformed header ordering
  - reject `record` with empty body
  - reject `payload = ...` under `record`
- Frontend:
  - accept plain record construction
  - accept same-type refinement from base
  - accept lift from a different record with matching field names/types
  - reject non-record targets
  - reject incompatible base fields when they are the only source for a required target field
  - reject missing required fields after base/default resolution
  - reject duplicate field contributions
  - reject `record place` target type mismatch
  - reject nested headed regions as today
- Runtime:
  - `record yield` builds the expected value
  - explicit field override beats base field
  - omitted compatible fields copy from base
  - omitted optional fields take the current default behavior
  - `record deliver` binds the result
  - `record place` writes the result into an existing target
  - contribution modifiers behave the same way they do for current record-shaped `construct`
- Migration/acceptance:
  - first-party code can express current copy/update/lift hotspots with `record`
  - no new helper explosion is needed for ordinary record refinement after this lands
  - `construct` still works for existing generic-construction call sites

## Assumptions
- Dedicated `record` head is the canonical long-term shape.
- Full `yield` / `deliver` / `place` parity lands in the first implementation.
- This plan covers the whole second group:
  - explicit record refinement
  - record copy/lift from a base/source
  - broader record-specific structural work
- This plan does not add a broad trait system for copy/refine/lift.
- This plan does not broaden `construct`; overlap is transitional compatibility only.
- Copy-from-base is exact-field and shallow in v1.
