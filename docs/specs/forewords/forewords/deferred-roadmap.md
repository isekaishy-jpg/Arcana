# Foreword Deferred Roadmap

This ledger is authoritative for deferred foreword work.  
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

id: FW-D1
title: #derive transform foreword
reason_deferred: v1 is limited to compiler-owned metadata validation and lint/deprecation control to keep parser/type pipeline risk bounded.
target_window: next metadata expansion window
trigger_condition: ready_when=v1 warning/lint infra is green and `arcana test --list` is shipped; verify=parser regression suite stays green across consecutive post-change runs; blocked_by=open parser or metadata validation regressions.
owner: Arcana compiler team
acceptance_criteria: `#derive[...]` parses, validates targets/payloads, and performs deterministic transform expansion with compile tests.
status: deferred

id: FW-D2
title: user-defined forewords (`foreword ...`)
reason_deferred: owner-dispatch and conflict semantics are not finalized in v1.
target_window: after FW-D1 completion
trigger_condition: ready_when=FW-D1 is done and owner-dispatch/conflict rules are spec-locked; verify=compiler accepts valid user foreword definitions and rejects conflicts deterministically; blocked_by=unresolved owner dispatch or conflict semantics.
owner: Arcana language team
acceptance_criteria: user-defined foreword definitions compile, attach/validate by target, and dispatch by owner phase with deterministic diagnostics.
status: deferred

id: FW-D3
title: runtime-retained metadata and introspection
reason_deferred: bytecode/runtime metadata carriage and reflection surface are not part of v1 compiler-only forewords.
target_window: runtime metadata window
trigger_condition: ready_when=reflection API milestone and bytecode metadata carriage design are approved; verify=runtime/tooling can load retained metadata with retention-policy tests green; blocked_by=missing runtime metadata carriage contract.
owner: Arcana VM/runtime team
acceptance_criteria: retained foreword metadata is emitted, loadable at runtime/tooling, and covered by retention-policy tests.
status: deferred

id: FW-D4
title: statement/expression foreword targets
reason_deferred: local-target attachment increases parser and diagnostics complexity beyond v1 declaration-only scope.
target_window: parser local-target window
trigger_condition: ready_when=parser complexity budget and local-target diagnostics model are approved; verify=statement/expression target attachment tests pass with no precedence regressions; blocked_by=unlocked local-target placement/diagnostics rules.
owner: Arcana parser/compiler team
acceptance_criteria: statement/expression target attachment parses and validates with clear scope/placement diagnostics and no precedence regressions.
status: deferred
