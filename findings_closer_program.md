## Findings Closure Program Before Milestone 8

Status: `completed`

### Summary
Use [milestone_6_7_review_findings.md](c:/Users/Weaver/Documents/GitHub/Arcana/tmp/milestone_6_7_review_findings.md) as the source checklist and close every item to a terminal state before returning to AOT work. Terminal means exactly one of: fixed in implementation, fixed by explicit contract/spec clarification, intentionally narrowed with updated authority docs, or removed as stale/invalid. No finding stays ambiguous or “known debt”.

The program should run in six ordered tranches. First focus is executable backend contract closure, because it supersedes the largest downstream symptom cluster and makes later scheduler/AOT work cleaner instead of compounding the current heuristic runtime shape.

### Implementation Changes
1. Create a findings closure matrix keyed by finding number.
   - Keep the temp review as source material.
   - Add a working closure ledger that groups items by root cause, current owner, target tranche, and terminal closure type.
   - Immediately separate stale-authority issues from real implementation/language gaps so later work is not driven by bad docs.

2. Close authority and contract hygiene blockers first.
   - Repair broken frozen-contract references to archived `grimoires/reference/*`.
   - Mark stale Meadow-era scope docs as historical where they are no longer rewrite authority.
   - Extract missing rewrite-era domain specs from the frozen summary for the domains that implementation now depends on: access modes/ownership, qualified phrases, collections-range-index-slice, `where` semantics, concurrency/behaviors, and opaque handles/resources.
   - This tranche closes or reframes findings 18, 21, 22, 23, 24, 25, 26, and 27.

3. Replace the current heuristic executable model with a real lowered backend contract.
   - Enrich the internal syntax/HIR/IR/AOT path so executable operations carry resolved callable identity, operation kind, qualifier kind, call mode, and place/index/slice/range semantics instead of leaving runtime to guess from strings and runtime values.
   - Qualified phrases, dotted qualifiers, memory phrases, member/index places, and method calls must execute from lowered identity, not runtime path synthesis.
   - Variant receiver dispatch must become a real primitive so `Option`/`Result` no longer depend on executor-owned qualifier shims.
   - Remove public-std behavior from the runtime once the lowered contract can execute linked std methods directly.
   - This tranche is the main closure path for findings 4, 5, 9, 12, 13, 14, 15, 39, 40, 41, 42, 43, 44, 45, and 48.

4. Close the remaining frozen language execution surface on top of the new contract.
   - Add real execution support for `Index`, `Slice`, `RangeInt`, range-driven `for`, literal `match` behavior, and any still-lowered first-party executable metadata forms.
   - Resolve tuple-match drift explicitly: either restore redesigned pair-pattern support or narrow the frozen tuple/match contract so it matches the rewrite; do not leave it half-promised.
   - Resolve collection literal policy explicitly: either implement required non-empty map literals and settled empty-`[]` behavior, or narrow the frozen contract and audit sheet so the rewrite baseline is explicit.
   - This tranche closes findings 3, 8, 15, 17, 28, 29, 46, and 47.

5. Add a structured type-law model for `where`.
   - Replace raw `where_clause` text carriage with structured predicate representation for trait bounds, projection equality, and outlives predicates.
   - Preserve the current rewrite lifetime surface, but add semantic enforcement for trait/supertrait `where`, `ProjectionEq`, `'a: 'b`, and `T: 'a`.
   - Keep supertrait-style requirements expressed through trait `where` if that remains the Arcana shape; do not invent new source syntax.
   - This tranche closes findings 49, 51, and 52, while preserving the correction in 53.

6. Close ownership/resource law and scheduler/worker execution.
   - Ratify access-mode law so `take` is fully consuming across user routines, std wrappers, and host intrinsics, with consistent invalidation/write-through behavior.
   - Ratify the opaque handle/resource lifecycle model enough that runtime behavior is not guessing.
   - Introduce a real scheduler/worker/task substrate for `async fn main`, `weave`, `split`, `thread_id`, behavior/system execution, and chain parallel/async execution.
   - Chain expressions must become executable plan nodes, not parse-only metadata.
   - This tranche closes findings 1, 2, 6, 7, 19, 20, 30, 34, 35, 36, 37, and 38.

### Test Plan
- Add tranche-local regression suites before removing any old shim path.
- For the backend-contract tranche, prove end-to-end execution for dotted qualifiers, `?`, `>>`, memory phrases, `Option`/`Result` methods, and linked std dispatch without executor-owned public-std shims.
- For the language-surface tranche, prove `RangeInt`, index/slice, range `for`, literal `match`, and the chosen tuple/map/empty-list policy with parser, HIR, frontend, and runtime tests.
- For the `where` tranche, add semantic tests for trait-bound satisfaction, supertrait-style trait `where`, projection equality, and outlives predicates.
- For the scheduler tranche, add real async entry, task/thread lifecycle, worker/main affinity, `thread_id`, and chain parallel/async execution tests.
- End with a full closure audit: every numbered finding in the temp review is re-read and marked terminal, and no item remains open before Milestone 8 resumes.

### Acceptance Criteria
- The temp review has no open findings; every item is fixed, reclassified, or explicitly retired with supporting authority.
- The rewrite no longer depends on executor-owned public-std shims for public language behavior.
- The runtime no longer reconstructs core call/qualifier semantics from heuristic runtime value inspection.
- The frozen language contract and current implementation agree on the remaining active surface.
- Milestone 8 work resumes only after this closure audit passes; AOT/native artifact work does not proceed on top of unresolved runtime/spec debt.

### Assumptions And Defaults
- Language freeze remains in force; do not add new public syntax unless the frozen contract already requires it.
- If a Meadow-era feature is bug-prone or intentionally narrowed, default to explicit contract clarification instead of blind parity restoration.
- The first implementation tranche is executable backend contract closure.
- “Move back to AOT work” means all findings are definitively closed, not merely reduced in count.
