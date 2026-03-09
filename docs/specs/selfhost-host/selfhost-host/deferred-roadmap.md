# Selfhost Host Deferred Roadmap

This ledger is authoritative for deferred host-platform work.
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

id: HOST-D1
title: process spawn and external command execution APIs
reason_deferred: resolved in Plan 42 with explicit capability gating and host-root constrained executable paths.
target_window: Plan 42
trigger_condition: ready_when=implemented; verify=selfhost_frontend_parity_guard and runtime tests pass; blocked_by=none.
owner: Arcana tooling/runtime team
acceptance_criteria: process APIs are sandboxed, deterministic, and covered by policy + regression tests.
status: done

id: HOST-D2
title: network and socket APIs
reason_deferred: package/registry transport and timeout/error policy are not finalized in v1.
target_window: package and registry phase
trigger_condition: ready_when=package transport design is approved; verify=deterministic timeout/retry/error tests are green; blocked_by=security model for network capabilities.
owner: Arcana package/runtime team
acceptance_criteria: network APIs are specified with deterministic behavior and comprehensive security/timeout testing.
status: deferred

id: HOST-D3
title: streaming file handle APIs
reason_deferred: resolved in Plan 42 with deterministic native stream handles and VM unsupported-oracle behavior.
target_window: Plan 42
trigger_condition: ready_when=implemented; verify=stream runtime tests and host parity guards pass; blocked_by=none.
owner: Arcana std/runtime team
acceptance_criteria: streaming APIs are stable, deterministic, and validated against large-file scenarios.
status: done
