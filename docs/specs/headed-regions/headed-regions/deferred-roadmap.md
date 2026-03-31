# Headed Regions Deferred Roadmap

This ledger is authoritative for deferred headed-region work.
Required fields per entry:
- `id`
- `title`
- `reason_deferred`
- `target_window`
- `trigger_condition`
- `owner`
- `acceptance_criteria`
- `status`

Matrix note:
- `conformance/selfhost_language_matrix.toml` is intentionally unchanged in the headed-regions docs-adoption patch.
- Any implementation work on headed regions must update the matrix in the same implementation window.

---

Trigger Condition Template (required single-line format):
- `trigger_condition: ready_when=<objective readiness>; verify=<objective verification>; blocked_by=<current blocker>`

id: HR-D1
title: user-defined headed-region heads
reason_deferred: v1 intentionally freezes a closed language-defined head family first.
target_window: post-v1 headed-region expansion
trigger_condition: ready_when=headed-region parser/frontend/runtime semantics for built-in heads are implemented and stable; verify=domain and conformance coverage stay green while adding extension hooks; blocked_by=no implemented baseline headed-region lane yet.
owner: Arcana language team
acceptance_criteria: user-defined heads have explicit syntax, validation, lowering, and diagnostics without weakening the built-in family contract.
status: deferred

id: HR-D2
title: broader nesting rules beyond current conservative rejection
reason_deferred: nested ride composition needs explicit readability and control-flow law before approval.
target_window: post-v1 headed-region semantics window
trigger_condition: ready_when=the built-in heads have implemented and tested standalone semantics; verify=nested-region regression suite passes with explicit per-head rules; blocked_by=no approved nested ride law beyond the current conservative floor.
owner: Arcana language/frontend team
acceptance_criteria: allowed nesting combinations are explicitly specified, implemented, and covered by parser/frontend/runtime tests.
status: deferred

id: HR-D3
title: additional `Memory` region families
reason_deferred: v1 keeps `Memory` aligned with the already-approved `arena` / `frame` / `pool` memory family set.
target_window: next memory expansion window
trigger_condition: ready_when=memory-family expansion is approved in the memory domain and consumer semantics stay explicit; verify=headed-region and memory-phrase tests pass for every new family; blocked_by=current memory family set is intentionally limited in approved memory scope.
owner: Arcana memory/runtime team
acceptance_criteria: each added `Memory` family has explicit strategy modifiers, participating-line classes, and consumer semantics documented and implemented.
status: deferred

id: HR-D4
title: foreword participation inside headed-region bodies
reason_deferred: v1 does not automatically make headed-region participating lines new foreword targets.
target_window: post-v1 foreword/metadata integration window
trigger_condition: ready_when=headed-region line classes and validation model are implemented and foreword target law is ready to expand; verify=foreword target diagnostics stay deterministic across headed-region cases; blocked_by=no approved target-specific foreword law for headed-region lines.
owner: Arcana language/frontend team
acceptance_criteria: any foreword-target expansion is explicit per head or per participating-line class and is covered by updated foreword/domain docs plus tests.
status: deferred

id: HR-D5
title: later headed-region family growth beyond `recycle` / `construct` / `bind` / `Memory`
reason_deferred: v1 freezes the initial built-in family and avoids open-ended region expansion.
target_window: post-v1 language-design window
trigger_condition: ready_when=the initial four heads are implemented and stable and a new head solves a distinct job without overlapping existing rides; verify=spec, parser, frontend, runtime, and conformance updates land together; blocked_by=no implemented baseline headed-region support yet.
owner: Arcana language team
acceptance_criteria: any new head has a distinct ride, modifier/completion law, participating-line contract, diagnostics, and full implementation/test coverage.
status: deferred

id: HR-D6
title: broader native direct lowering for headed regions
reason_deferred: the current native direct backend only carries ABI-shaped primitive/string/bytes/pair values, so general record/variant headed-region values and named owner-exit propagation still route through runtime dispatch inside native bundles.
target_window: post-v1 native backend expansion
trigger_condition: ready_when=the native direct model can carry runtime record/variant values plus structured control flow without weakening export ABI guarantees; verify=headed-region native tests cover direct `construct`, payload-bearing `recycle` / `bind`, and named owner exits end to end; blocked_by=current native direct representation does not model general runtime values or owner-exit flow.
owner: Arcana runtime/backend team
acceptance_criteria: native bundles execute the full headed-region family directly when the surrounding routine otherwise qualifies for the direct lane, with parity tests covering record construction, payload acquisition, and owner-exit routing.
status: deferred
