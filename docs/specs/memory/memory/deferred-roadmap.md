# Memory Deferred Roadmap

This ledger is authoritative for deferred memory-system work.
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

id: MEM-D1
title: executable `temp` and `session` memory categories
reason_deferred: completed in the memory views/publication expansion window.
target_window: completed
trigger_condition: ready_when=frame/pool runtime is stable and no open frame/pool regressions remain; verify=memory/compiler/vm validation matrix stays green across consecutive post-change runs; blocked_by=none.
owner: Arcana language/runtime team
acceptance_criteria: `temp` and `session` compile, lower, and run with deterministic semantics and full parser/compiler/vm tests.
status: done

id: MEM-D2
title: borrowed read views for allocator-backed values
reason_deferred: completed in Plan 32 ownership closure.
target_window: completed
trigger_condition: ready_when=allocator borrow APIs are implemented for Arena/Frame/Pool and reset/remove borrow-live checks are enforced; verify=memory borrow regression suite passes with deterministic diagnostics; blocked_by=none.
owner: Arcana type-system team
acceptance_criteria: read-view surface is specified and implemented with sound aliasing checks and deterministic diagnostics.
status: done

id: MEM-D3
title: sendable/shared memory allocators
reason_deferred: completed in Plan 32 ownership closure.
target_window: completed
trigger_condition: ready_when=concurrency memory-safety model with cross-thread validity rules is approved; verify=send/share allocator tests pass across split worker scenarios; blocked_by=none.
owner: Arcana concurrency/runtime team
acceptance_criteria: selected allocator families are safely sendable/shareable across split workers with passing runtime safety tests.
status: done

id: MEM-D4
title: pool iterators and compaction
reason_deferred: completed in the memory views/publication expansion window.
target_window: completed
trigger_condition: ready_when=profiling shows sustained compaction/iteration need and invalidation semantics are spec-locked; verify=pool iterator/compaction determinism and regression tests are green; blocked_by=none.
owner: Arcana std/runtime team
acceptance_criteria: pool iteration/compaction APIs are documented, implemented, and covered by determinism/regression tests.
status: done
