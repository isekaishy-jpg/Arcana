# Chain Adoption Audit v1

## Scope
- Plan: PLAN45 (chain-aware IR regions + conservative first-party adoption)
- Date: 2026-03-03
- Scan scope:
  - `std/src/**/*.arc`
  - `grimoires/**/*.arc`
  - `examples/**/*.arc`

## Rewrite Criteria
- Statement-only chain context (no expression-result dependency).
- Unary stage flow compatibility.
- Behavior-preserving shape (no control-flow or call-shape drift).
- Readability improvement over existing code.

## Rewritten Locations
- None in this pass.
- Rationale: first-party code already uses chain phrases where pipeline semantics are explicit, and remaining candidates are mostly return-oriented wrappers or multi-argument side-effect wrappers where forced chain rewrites would either reduce clarity or require semantic assumptions.

## Unchanged Std Modules
- std/src/app.arc: App configuration helpers are return/value oriented and not unary stage pipelines.
- std/src/args.arc: Direct host wrappers return values; chain conversion adds no clarity.
- std/src/array.arc: Thin reexport/wrapper surface; no statement pipeline opportunity.
- std/src/behavior_traits.arc: Trait definitions and helper routing are declaration-centric.
- std/src/behaviors.arc: ECS behavior APIs are wrapper style with explicit argument lists.
- std/src/book.arc: Reexport surface only.
- std/src/bytes.arc: Value-returning conversion helpers and predicates; no statement pipelines.
- std/src/canvas.arc: Multi-argument draw wrappers; chain stages are unary and not a fit.
- std/src/collections/array.arc: Collection wrappers are return-oriented and explicit.
- std/src/collections/list.arc: Collection wrappers are return-oriented and explicit.
- std/src/collections/map.arc: Collection wrappers are return-oriented and explicit.
- std/src/collections/set.arc: Set wrappers are return-oriented and explicit.
- std/src/concurrent.arc: Wrapper methods and value returns dominate; no safe unary pipeline gain.
- std/src/ecs.arc: ECS API wrappers require explicit non-unary argument passing.
- std/src/env.arc: Host wrappers are direct returns and conditionals.
- std/src/events.arc: Event wrappers are return/value oriented.
- std/src/fs.arc: Fallible Result wrappers are call-return centered; no pipeline readability win.
- std/src/input.arc: Input wrappers are direct return helpers.
- std/src/io.arc: Minimal wrapper surface; no useful chain rewrite.
- std/src/iter.arc: Iterator wrappers and control forms are return-oriented.
- std/src/kernel/collections.arc: Intrinsic declarations only.
- std/src/kernel/concurrency.arc: Intrinsic declarations only.
- std/src/kernel/ecs.arc: Intrinsic declarations only.
- std/src/kernel/events.arc: Intrinsic declarations only.
- std/src/kernel/gfx.arc: Intrinsic declarations only.
- std/src/kernel/host.arc: Intrinsic declarations only.
- std/src/kernel/io.arc: Intrinsic declarations only.
- std/src/kernel/memory.arc: Intrinsic declarations only.
- std/src/kernel/text.arc: Intrinsic declarations only.
- std/src/list.arc: Thin wrapper with explicit return/value behavior.
- std/src/manifest.arc: Parser and manifest transforms are expression/return oriented.
- std/src/memory.arc: Memory APIs are method wrappers with explicit value flow and mutability semantics.
- std/src/option.arc: Enum/value helpers are expression-oriented.
- std/src/path.arc: Path helpers are return/value oriented.
- std/src/prelude.arc: Reexport surface only.
- std/src/process.arc: Result-returning host wrapper API, no unary stage gain.
- std/src/result.arc: Enum/value helpers are expression-oriented.
- std/src/text.arc: Predicate/format helpers are value-returning, not statement pipelines.
- std/src/tooling.arc: Reexport surface only.
- std/src/tooling/graph.arc: Graph topo logic is branch/recursion heavy; chain insertion would reduce clarity.
- std/src/types.arc: Reexport surface only.
- std/src/types/core.arc: Type declarations and simple constructors.
- std/src/types/game.arc: Type declarations and simple constructors.
- std/src/window.arc: Window wrappers are explicit multi-arg calls; unary chain stages are not a fit.

## Non-Std Findings
- `grimoires/**`: existing chain usage already covers clear pipeline-style behavior paths (notably showcase/winspell flows); additional rewrites were not high-confidence non-breaking improvements.
- `examples/**`: chain-focused examples already exercise forward/reverse/mixed forms; additional forced rewrites in non-chain demos would reduce readability.

## Parity Evidence
- `cargo test -p arcana-compiler`
- `cargo test -p arcana-native`

## Plan 47 Boundary Migration Addendum
- Scheduler-boundary files were rechecked for explicit boundary contracts under Plan 47.
- Guard coverage now includes:
  - `examples/behavior_phases`
  - `examples/grimoire_behavior_app`
  - `examples/grimoire_ecs_schedule`
  - `examples/grimoire_ecs_mini_game`
  - `examples/topdown_arena_showcase/showcase_core`
- Contract-scheduled behavior execution is native-only; VM emits deterministic migration diagnostics.
