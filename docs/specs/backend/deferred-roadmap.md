# Backend Deferred Roadmap

Status: `authoritative-deferred-ledger`

This ledger is authoritative for backend work intentionally deferred from the rewrite path.

Current backend architecture is defined by:
- `PLAN.md`
- `docs/rewrite-roadmap.md`
- `docs/specs/backend/selfhost_language_contract_v1.md`
- `docs/specs/backend/anybox-policy.md`

Guardrails:
- This ledger does not revive Meadow-era bytecode-first, VM-first, or post-selfhost-AOT assumptions.
- The rewrite backend path stays IR-first and typed, with the first runnable AOT backend on the pre-selfhost path.
- Deferred entries may schedule follow-on work, but they may not change the frozen selfhost language contract or milestone order by themselves.

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

id: BACKEND-D1
title: optimization pass stack over the internal IR
reason_deferred: rewrite priority is typed frontend closure, first runnable backend, and selfhost toolchain closure before performance tuning.
target_window: post-first-runnable-backend performance hardening
trigger_condition: ready_when=first runnable backend and selfhost bootstrap lanes are stable; verify=opt/non-opt parity suites and benchmark guards stay green; blocked_by=missing baseline performance dataset.
owner: Arcana compiler/backend team
acceptance_criteria: deterministic optimization pipeline with parity-safe transforms, regression coverage, and measurable performance wins on agreed benchmark suites.
status: deferred

id: BACKEND-D2
title: native packaging and distribution contract
reason_deferred: the rewrite needs a stable runnable backend and stable artifact layout before install/runtime-image policy can be frozen.
target_window: post-selfhost toolchain stabilization
trigger_condition: ready_when=first runnable backend and workspace artifact layout are stable; verify=packaging smoke matrix and reproducible artifact checks pass; blocked_by=unsettled runtime image and distribution contract.
owner: Arcana toolchain/backend team
acceptance_criteria: Arcana emits reproducible installable native deliverables with documented packaging, runtime image, and distribution rules.
status: deferred
