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

Trigger Condition Template:
- `trigger_condition: ready_when=<objective readiness>; verify=<objective verification>; blocked_by=<current blocker>`

id: FW-D1
title: `#derive` convenience foreword
reason_deferred: v1 now supports user-defined and executable forewords, but it does not ship a compiler-owned derive convenience layer.
target_window: post-v1 metadata expansion
trigger_condition: ready_when=the current executable/basic foreword lane is stable across selfhost fixtures; verify=derive expansion is deterministic across repeated frontend runs; blocked_by=missing approved derive library contract.
owner: Arcana language team
acceptance_criteria: `#derive[...]` expands through an approved deterministic contract with target and payload validation plus coverage for repeated builds.
status: deferred

id: FW-D2
title: user-defined statement and expression targets
reason_deferred: v1 keeps user-defined forewords on declarations, methods, fields, and parameters only.
target_window: parser local-target window
trigger_condition: ready_when=placement and diagnostics law for local targets is approved; verify=parser and frontend suites pass with no precedence regressions; blocked_by=unapproved statement/expression attachment model.
owner: Arcana parser/compiler team
acceptance_criteria: user-defined forewords can target statements/expressions with clear placement rules, deterministic transforms, and no precedence regressions.
status: deferred

id: FW-D3
title: multi-phase foreword execution beyond `frontend`
reason_deferred: v1 freezes execution to frontend validation/expansion only.
target_window: post-selfhost compiler phase expansion
trigger_condition: ready_when=additional compiler phase ownership is approved; verify=phase ordering and artifact carriage tests stay deterministic across rebuilds; blocked_by=missing approved cross-phase execution contract.
owner: Arcana compiler team
acceptance_criteria: additional phases are explicit in spec and executable forewords can target them without ambiguous ordering or duplicate execution.
status: deferred
