# Page Rollups Deferred Roadmap

Status: `authoritative-deferred-ledger`

This ledger is authoritative for deferred page-rollup work.

Required fields per entry:
- `id`
- `title`
- `reason_deferred`
- `target_window`
- `trigger_condition`
- `owner`
- `acceptance_criteria`
- `status`

---

Trigger Condition Template (required single-line format):
- `trigger_condition: ready_when=<objective readiness>; verify=<objective verification>; blocked_by=<current blocker>`

id: ROLLUP-D1
title: async cleanup handlers
reason_deferred: v1 keeps cleanup synchronous so control-flow lowering, diagnostics, and runtime ordering stay simple before selfhost.
target_window: post-selfhost cleanup expansion
trigger_condition: ready_when=async cleanup ordering and failure semantics are spec-locked; verify=parser/frontend/runtime cleanup tests cover await and early-exit cases; blocked_by=unspecified async cleanup ordering.
owner: Arcana language/runtime team
acceptance_criteria: async cleanup semantics are deterministic, explicitly documented, and covered by regression tests.
status: deferred

id: ROLLUP-D2
title: cleanup transfer or disarm operations
reason_deferred: v1 intentionally forbids transfer/disarm so ownership and activation rules remain obvious.
target_window: post-selfhost ownership follow-up
trigger_condition: ready_when=ownership transfer semantics for rollup-bound subjects are approved; verify=borrow/move regression suite remains green; blocked_by=no approved transfer model.
owner: Arcana type-system team
acceptance_criteria: transfer/disarm behavior is specified with deterministic diagnostics and no hidden cleanup gaps.
status: deferred

id: ROLLUP-D3
title: callable/context-object cleanup handlers
reason_deferred: v1 cleanup handlers are named callable paths only; future function/context objects must be designed first and are not needed for v1 rollups.
target_window: callable object phase
trigger_condition: ready_when=function/context object contract is approved; verify=rollup lowering and callable object tests pass together; blocked_by=function/context object contract is reserved only.
owner: Arcana language team
acceptance_criteria: rollups can target callable objects without introducing closure semantics or ambiguous cleanup dispatch.
status: deferred

id: ROLLUP-D4
title: additional rollup kinds such as `#sendable` and `#shareable`
reason_deferred: v1 standardizes runtime cleanup only; contract rollups need a separate ownership/concurrency design pass.
target_window: post-selfhost contract-rollup phase
trigger_condition: ready_when=contract rollup semantics are approved across ownership and concurrency docs; verify=static checking and diagnostics remain deterministic; blocked_by=no cross-domain contract for these rollup kinds.
owner: Arcana language/concurrency team
acceptance_criteria: new rollup kinds are specified as static contracts without weakening v1 cleanup guarantees.
status: deferred
