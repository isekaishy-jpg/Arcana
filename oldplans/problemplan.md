# Known Problems Cleanup Campaign

## Summary

Run this as four ordered tracks, not one giant patch:

1. runtime/behavior corrections that already contradict the visible surface
2. qualifier-model refactor plus qualifier-surface expansion
3. owner-context code spike using one explicit composite activation payload
4. tuple enrichment code spike limited to exact pair destructuring

For broader-surface items, use code-spike-first, then graduate each spike with the matching approved-scope and `llm.md` updates in the same merge. Do not leave new language behavior living only in crates/tests.

## Key Changes

### 1. Runtime correctness first

- Split chain execution into explicit per-style helpers instead of one shared branch.
- Fix `parallel` to spawn all downstream stages first, then await/join them in normalized order so result ordering stays deterministic while stages are actually in flight together.
- Fix `broadcast` to remain sequential same-input fanout.
- Fix `collect` to run a real directional pipeline and return the downstream stage outputs in normalized order, excluding the initial seed/input value.
- Fix `async` to auto-await any task/thread result before feeding it to the next stage, including the seed stage result when needed.
- Fix `plan` to validate the chain contract, evaluate only the seed/input expression needed to produce the pass-through value, skip downstream stage execution, and return that original input unchanged.
- Implement real `lazy` semantics with an internal deferred-chain runtime carrier plus centralized force-at-demand boundaries. The lazy chain should not execute when its value is never demanded, and should execute once in normalized order when forced.
- Replace the intrinsic-only positional binder with one shared call-argument binder so named phrase args work for intrinsics the same way they work for linked routines.
- Replace the blanket spawned-`edit` rejection with capability-based behavior:
  - `weave` may capture `edit` places when the runtime can preserve same-place mutation on the task substrate.
  - `parallel` may use the same task path for `edit`-capable stages.
  - `split` remains conservative and continues rejecting unsafe cross-thread `edit` place capture until there is an explicit transferable-place law.

### 2. Qualifier refactor and expansion

- Replace raw qualifier strings with a structured qualifier model that survives syntax, frontend, lowering, and runtime.
- Give `call` its own qualifier kind instead of treating it as an ordinary identifier and special-casing it later.
- Allow optional type args on `call`, bare-method, and named-path qualifiers so forms like `call[T]`, `method[T]`, and `pkg.fn[T]` parse and lower directly.
- Add qualifier-native execution/control forms:
  - `await`: `subject :: :: await`
  - `weave`: `callable_subject :: args :: weave`
  - `split`: `callable_subject :: args :: split`
- Add qualifier-native failure forms:
  - `must`: canonical hard unwrap, zero args, no `unwrap` alias in this campaign
  - `fallback`: exactly one positional fallback arg, no named args
- Define `must` and `fallback` only for `Result[T, Str]` and `Option[T]` in this pass:
  - `must` returns the inner value, or fails through the current runtime error lane
  - `fallback` returns the inner value or the supplied fallback
- Keep attachment rules explicit:
  - named header entries stay allowed only for `call`/bare-method/apply
  - new control/failure qualifiers reject named header entries

### 3. Owner-context spike

- Keep owner activation at one explicit payload argument, not multi-arg activation.
- Extend `create` with an explicit owner-level composite context clause, for example `context: SessionCtx`.
- Owners without a `context:` clause keep zero-arg activation.
- Owners with a `context:` clause require exactly one activation argument of that type on entry and re-entry.
- Owned object lifecycle hooks may either omit context or use that exact owner context type.
- Mixed per-object lifecycle context types stop being legal once the owner uses explicit composite context.
- This preserves multiple owned object states while making the activation payload explicit and audited.

### 4. Tuple spike

- Keep tuples pair-only in this campaign.
- Add exact recursive pair destructuring only in `let` and `for`.
- Keep parameter destructuring, `match` tuple patterns, tuple field assignment, and 3+ tuples out of scope.
- Keep tuple access positional-only with `.0` and `.1`.

## Test Plan

- Parser coverage:
  - structured qualifier parsing
  - generic qualifier args
  - new qualifier forms and their arity/attachment restrictions
  - owner `context:` syntax
  - exact pair destructuring in `let` and `for`
- Frontend coverage:
  - `call` as dedicated qualifier kind
  - typing for `must` and `fallback`
  - owner activation typechecking against owner-level context
  - tuple binding type propagation for recursive pair destructuring
  - named-arg intrinsic binding parity with routine calls
- Runtime coverage:
  - `parallel` proves observable overlap, not just ordered list output
  - `collect`, `broadcast`, `async`, `plan`, and `lazy` each get separate execution tests
  - `lazy` proves “unused means not executed” and “forced means executed once”
  - `weave` + `edit` mutates the original place correctly
  - `split` still rejects unsafe `edit` capture
  - `must` and `fallback` on both `Result` and `Option`
  - owner activation with one composite context across multiple owned objects
- Acceptance criteria:
  - each current `llm.md` known-problem item is either fixed, deliberately narrowed by explicit contract, or removed because the new surface supersedes it
  - no surface-expansion change ships without the matching scope/status update and `llm.md` refresh

## Assumptions And Defaults

- Broader-surface work is in scope, not just v1 contract mismatches.
- Broader-surface items use code-spike-first, then spec alignment when the spike is accepted.
- Owner work uses one composite activation payload, not multiple owner activation args.
- `must` is the only hard-unwrap spelling in this campaign; `unwrap` is not added as a duplicate alias.
- `split` does not gain arbitrary cross-thread mutable-place capture in this campaign.
- Tuple enrichment stays exact-shape and pair-only.
- If any chain-style fix changes approved behavior text, the corresponding approved scope and `llm.md` change in the same patch.
