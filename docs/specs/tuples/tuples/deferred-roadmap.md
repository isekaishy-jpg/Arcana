# Tuple Deferred Roadmap

Status: `authoritative-deferred-ledger`

This ledger is authoritative for deferred tuple work.
It exists because 2/3-tuple support is the current selfhost baseline, not a claim that Arcana should stop there.

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

id: TUP-D1
title: generalized tuples beyond 3-tuples
reason_deferred: the current selfhost baseline only requires 2/3-tuples; expanding arity before typed ownership and layout rules settle would add surface area without clear pre-selfhost payoff, even though richer tuples remain an expected language direction.
target_window: post-selfhost tuple expansion window
trigger_condition: ready_when=typed frontend, ownership flow, and tuple layout policy are stable or 2/3-tuple support becomes a demonstrated selfhost blocker; verify=parser/typecheck/borrow tests cover 4+-element tuples and nested access deterministically; blocked_by=no approved generalized tuple contract.
owner: Arcana language/type-system team
acceptance_criteria: 4+-element tuples are specified with deterministic syntax, access, equality, and ownership behavior and do not weaken the 2/3-tuple guarantees.
status: deferred

id: TUP-D2
title: tuple destructuring in bindings, params, and patterns
reason_deferred: destructuring interacts directly with ownership, moves, and diagnostics quality; it should not be added before the typed ownership pass is real.
target_window: ownership-complete tuple follow-up
trigger_condition: ready_when=ownership and borrow diagnostics are stable enough to explain partial moves and binding splits; verify=tuple destructuring tests cover lets, params, loops, and match bindings with no ambiguity regressions; blocked_by=missing ownership-grade destructuring rules.
owner: Arcana type-system/compiler team
acceptance_criteria: tuple destructuring compiles with explicit move/borrow rules, deterministic diagnostics, and no implicit closure-like capture behavior.
status: deferred
