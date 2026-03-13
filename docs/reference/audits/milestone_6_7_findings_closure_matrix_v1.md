# Milestone 6/7 Findings Closure Matrix v1

Status: `reference-only`

This is the working closure ledger for `tmp/milestone_6_7_review_findings.md`.
All findings are now at terminal status.

Terminal closure types:
- `implementation`
- `spec-clarification`
- `intentional-narrowing`
- `retired-stale-or-invalid`

This file is keyed by finding number and groups each finding under the superseding workstream that should close it. The temp review remains the source narrative list; this ledger is the execution tracker.

| Finding | Root workstream | Tranche | Current status | Planned terminal closure |
|---|---|---|---|---|
| 1 | ownership-resource law | 6 | terminal | implementation + spec-clarification |
| 2 | scheduler-worker substrate | 6 | terminal | implementation |
| 3 | executable backend contract closure | 4 | terminal | implementation |
| 4 | executable backend contract closure | 3 | terminal | implementation + spec-clarification |
| 5 | executable backend contract closure | 3 | terminal | implementation |
| 6 | scheduler-worker substrate | 6 | terminal | implementation |
| 7 | scheduler-worker substrate | 6 | terminal | implementation |
| 8 | language-surface closure | 4 | terminal | implementation + intentional-narrowing |
| 9 | executable backend contract closure | 3 | terminal | spec-clarification |
| 10 | executable backend contract closure | 3 | terminal | implementation |
| 11 | non-gap preservation | none | terminal | retired-stale-or-invalid |
| 12 | executable backend contract closure | 3 | terminal | implementation + spec-clarification |
| 13 | executable backend contract closure | 3 | terminal | implementation |
| 14 | executable backend contract closure | 3 | terminal | implementation |
| 15 | language-surface closure | 4 | terminal | implementation |
| 16 | implementation hygiene | post-tranche | terminal | implementation |
| 17 | executable backend contract closure | 4 | terminal | implementation |
| 18 | authority and contract hygiene | 2 | terminal | spec-clarification |
| 19 | ownership-resource law | 2 / 6 | terminal on contract side | spec-clarification |
| 20 | scheduler-worker substrate | 2 / 6 | terminal on contract side | spec-clarification |
| 21 | authority and contract hygiene | 2 | terminal | retired-stale-or-invalid |
| 22 | language-surface closure | 4 | terminal | intentional-narrowing or spec-clarification |
| 23 | authority and contract hygiene | 2 | terminal | spec-clarification |
| 24 | authority and contract hygiene | 2 | terminal | spec-clarification |
| 25 | authority and contract hygiene | 2 | terminal | spec-clarification |
| 26 | authority and contract hygiene | 2 | terminal | retired-stale-or-invalid |
| 27 | authority and contract hygiene | 2 | terminal | retired-stale-or-invalid |
| 28 | language-surface closure | 4 | terminal | implementation or intentional-narrowing |
| 29 | language-surface closure | 4 | terminal | implementation |
| 30 | scheduler-worker substrate | 6 | terminal | implementation + spec-clarification |
| 31 | non-gap preservation | none | terminal | retired-stale-or-invalid |
| 32 | non-gap preservation | none | terminal | retired-stale-or-invalid |
| 33 | language-surface closure | 4 | terminal | intentional-narrowing or spec-clarification |
| 34 | scheduler-worker substrate | 6 | terminal | implementation + spec-clarification |
| 35 | scheduler-worker substrate | 6 | terminal | implementation |
| 36 | scheduler-worker substrate | 6 | terminal | implementation + spec-clarification |
| 37 | ownership-resource law | 6 | terminal | implementation + spec-clarification |
| 38 | ownership-resource law | 6 | terminal | implementation |
| 39 | executable backend contract closure | 3 | terminal | implementation + spec-clarification |
| 40 | executable backend contract closure | 3 | terminal | implementation |
| 41 | executable backend contract closure | 3 | terminal | implementation |
| 42 | executable backend contract closure | 3 | terminal | implementation |
| 43 | executable backend contract closure | 3 | terminal | implementation |
| 44 | language-surface closure | 4 | terminal | implementation |
| 45 | executable backend contract closure | 3 | terminal | implementation |
| 46 | language-surface closure | 4 | terminal | implementation or intentional-narrowing |
| 47 | language-surface closure | 4 | terminal | intentional-narrowing or implementation |
| 48 | executable backend contract closure | 3 | terminal | implementation |
| 49 | structured where semantics | 5 | terminal | implementation + spec-clarification |
| 50 | non-gap preservation | none | terminal | retired-stale-or-invalid |
| 51 | structured where semantics | 5 | terminal | implementation + spec-clarification |
| 52 | structured where semantics | 5 | terminal | implementation + spec-clarification |
| 53 | non-gap preservation | none | terminal | retired-stale-or-invalid |
| 54 | non-gap preservation | none | terminal | retired-stale-or-invalid |
| 55 | rewrite gain preservation | none | terminal | retired-stale-or-invalid |

## Root Workstreams

1. `executable backend contract closure`
   - findings: 3, 4, 5, 9, 10, 12, 13, 14, 15, 17, 39, 40, 41, 42, 43, 45, 48
2. `language-surface closure`
   - findings: 8, 15, 22, 28, 29, 33, 44, 46, 47
3. `structured where semantics`
   - findings: 49, 51, 52
4. `ownership-resource law`
   - findings: 1, 19, 24, 37, 38
5. `scheduler-worker substrate`
   - findings: 2, 6, 7, 20, 30, 34, 35, 36
6. `authority and contract hygiene`
   - findings: 18, 21, 23, 24, 25, 26, 27

## This-Patch Terminal Closures

- `18`: fixed by replacing dead in-repo reference paths in the frozen contract matrix and frozen summary.
- `19`: closed on the contract side by `docs/specs/access-modes/access-modes/v1-scope.md`; implementation closure remains tracked by finding `1`.
- `20`: closed on the contract side by `docs/specs/concurrency/concurrency/v1-scope.md`; execution closure remains tracked by findings `2`, `6`, `7`, `30`, `34`, `35`, and `36`.
- `21`: reclassified from active approved-memory-authority issue to stale-authority issue by introducing `docs/specs/memory/memory/v1-scope.md` and demoting the Meadow-era generic memory reference.
- `23`: closed on the contract side by `docs/specs/qualified-phrases/qualified-phrases/v1-scope.md`.
- `24`: closed on the contract side by `docs/specs/resources/resources/v1-scope.md`.
- `25`: closed by splitting missing rewrite-era language domains into dedicated approved scopes.
- `26`: retired as stale-authority drift rather than a live rewrite blocker.
- `27`: retired as stale-authority drift rather than a live rewrite runtime contract.

## Tranche 3 Progress Notes

- `5`: closed by making enum/variant receiver methods execute through linked std routine resolution instead of executor-owned `Option`/`Result` qualifier shims.
- `14`: closed by the same variant-receiver method dispatch fix; removing the old `Option`/`Result` shims no longer breaks linked enum impl methods.
- `45`: closed by adding explicit phrase qualifier kinds to IR rows and a runtime named-path execution lane that preserves dotted qualifier callable identity, including module-alias heads like `texts.starts_with`.
- `13`: closed by removing the remaining executor-owned collection qualifier fast paths; `len`, `push`, `pop`, and `try_pop_or` now execute through linked std methods, with regression coverage for direct collection method calls.
- `10`: closed by making `execute_main` validate `plan.runtime_requirements` against host-reported support instead of treating the requirement list as informational only. Buffered host tests now prove missing capabilities are rejected.
- `4`: closed. The executor no longer carries public `Option`/`Result` or collection qualifier behavior directly, and concrete bare-method qualifiers now carry lowered callable identity plus exact routine identity where needed instead of leaving runtime to reconstruct the call target from receiver shape.
- `12`: closed. Named-path qualifiers preserve lowered callable identity end to end, and bare-method lowering now distinguishes concrete resolved routine identity from the smaller dynamic-dispatch case that remains for trait-bound generic methods.
- `42`: closed by routing memory-phrase constructor execution through the shared apply path with attachment support instead of rejecting attached memory arguments at runtime.
- `41`: closed by promoting memory-phrase constructors from carried text to real lowered expressions across syntax, HIR, IR, and runtime. The executor now consumes the lowered constructor expression directly instead of reparsing bootstrap text at execution time.

## Tranche 4 Progress Notes

- `44`: closed by adding first-class runtime `RangeInt` execution and teaching `for` to iterate `RangeInt` values directly, which restores native range-driven `for` loops instead of requiring only `List` / `Array` / `Map`.
- `29`: closed. Runtime executable expressions now parse and execute `index(...)`, `slice(...)`, and `range(...)`, including strict bounds, copied-list slicing, `RangeInt` equality, and indexed assignment for `List` / `Array`.
- `15`: closed on the same implementation slice. The runtime now covers the frozen language execution surface that finding was tracking: `Index`, `Slice`, `RangeInt`, and indexed assignment.
- `8`: closed. Runtime `match` now handles approved integer, boolean, and string literal patterns, and the remaining tuple-pattern ambiguity was resolved by narrowing the approved tuple scope so tuple-specific `match` patterns are not part of the rewrite v1 baseline.
- `40`: closed by making runtime plan loading accept attached phrase foreword rows instead of rejecting them outright. Attachment metadata is now tolerated end to end in the runtime lane.
- `3`: closed. Runtime execution now honors page-rollup cleanup semantics on header exit, including routine-level cleanup after local defers and loop-exit cleanup on `?` propagation, so lowered first-party rollup-bearing executable forms no longer stop at metadata tolerance.
- `17`: closed on the same implementation slice. Attached-entry forewords and page-rollup-bearing executable forms now survive plan loading and execute with explicit cleanup behavior instead of remaining parser/runtime-only metadata.
- `48`: closed by implementing runtime `AwaitApply` execution. `task_expr :: :: >>` now awaits task/thread handles directly, and call-with-args `:: >>` now executes and awaits in the current runtime lane.
- `43`: closed by introducing an explicit evaluator return signal for expression-level try-propagation and wiring `expr :: :: ?` as a real runtime qualifier. Linked rewrite-owned `std.result` routines now prove both the success path and early `Result.Err(...)` propagation.
- `39`: closed. Qualified phrases now execute explicit `>`, `>>`, `?`, named-path, and bare-method qualifier kinds in the runtime lane, with concrete bare methods carrying exact lowered callable/routine identity and only trait-bound generic methods retaining dynamic receiver-directed dispatch.
- `22`: closed by clarifying the approved tuple scope. Pair tuples remain current, but tuple-specific `match` patterns are explicitly outside the rewrite v1 baseline.
- `28`: closed by the same tuple-scope narrowing. Meadow-era tuple patterns are treated as intentionally out of baseline until a redesign reintroduces them explicitly.
- `33`: closed as intentional narrowing rather than an accidental regression. The approved tuple scope now says the older broader tuple/match wording is not the operative rewrite contract.
- `46`: closed by clarifying the approved collections scope. Non-empty map literals are not part of the rewrite v1 baseline; constructor-driven maps are.
- `47`: closed by ratifying current empty `[]` behavior as list-only rewrite baseline surface and stating that its element typing comes from surrounding typing context rather than implied fallback defaults.

## Ownership And Resource Law Progress Notes

- `1`: closed on the runtime side. Caller-side `take` now invalidates non-`Copy` local arguments across ordinary routine calls, linked std wrapper calls, intrinsic-backed routines, and direct intrinsic fallback in the shared call path. Later reads fail with `use of moved local`, while plain reassignment clears the moved state again.
- `37`: closed by clarifying the approved memory scope. The current allocator family set remains intentionally limited to `arena` / `frame` / `pool`, but the rewrite memory model is explicitly not a placeholder-minimal carryover; typed ids, stale detection, `reset`, `remove`, memory phrases, and borrow-read / borrow-edit semantics are part of the approved current contract.
- `38`: closed by the earlier memory-phrase attachment execution slice. The runtime no longer rejects attached memory arguments; constructor calls now flow through the shared apply path instead of the old attachment-rejecting bootstrap lane.

## Scheduler And `where` Progress Notes

- `2`: closed. `std.concurrent.thread_id()` is now a real runtime query, with the main execution lane reporting `0` and split work executing under distinct scheduler thread ids.
- `6`: closed. `execute_main` now opens the async runtime lane for `async fn main() -> Int | Unit`, with regression coverage proving async entrypoint execution.
- `7`: closed. `weave` and `split` no longer collapse into immediately completed handles; spawned work now enters an explicit pending/running/completed task-thread state machine, including non-call expression spawns.
- `9`: closed by contract clarification rather than a backend rewrite. The current string-row AOT artifact is explicitly ratified only as an internal bootstrap contract in `PLAN.md`, `README.md`, and `docs/rewrite-roadmap.md`, not as the long-term native artifact format for Milestone 8.
- `30`: closed. The runtime now models task/thread lifecycle with deferred execution instead of immediately done handles, which satisfies the current approved concurrency contract even though later native/backend work may still harden worker implementation details.
- `34`: closed. Async/task/thread behavior and async/parallel chain execution now sit on the same explicit deferred scheduler substrate instead of the old eager done-handle shortcut, and the approved concurrency scope now makes exact worker realization a backend detail.
- `35`: closed. Chain expressions are now real runtime expressions with executable styles, not parse/lower-only metadata.
- `36`: closed. The rewrite now has an explicit chain execution contract in the approved chain/concurrency scopes, and runtime `parallel`/`async` chain styles execute through the shared task/thread substrate rather than ad hoc shortcuts.
- `49`: closed. The rewrite now parses `where` into a structured predicate model in frontend/type-law work, validates projection-equality shape, and checks associated-type references against real trait bodies.
- `51`: closed. Ordinary trait-bound and trait-`where` requirements are now enforced semantically enough for the current rewrite baseline, including impl-time failures when required trait bounds are missing.
- `52`: closed. Outlives predicates now parse as structured predicates and validate declared lifetime/type references under the approved v1 `where` contract.

## Implementation Hygiene Progress Notes

- `16`: closed by splitting the large inline regression slab out of `crates/arcana-runtime/src/lib.rs` into `crates/arcana-runtime/src/tests.rs`. The runtime core still has room for future modular cleanup, but the most obvious layer-mixing problem from the review is no longer present in the main executor file.
