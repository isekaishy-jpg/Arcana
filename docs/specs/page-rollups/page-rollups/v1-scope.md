# Cleanup Footers v1 Scope

Status: `approved-pre-selfhost`

This file is the active authority for the former page-rollup domain.
`page rollup` remains only as the historical file-path label for continuity.

## Core Contract

- `[subject, handler]#cleanup` is removed from the accepted language.
- The approved surface is the cleanup footer:
  - `-cleanup`
  - `-cleanup[target = name]`
  - `-cleanup[target = name, handler = path]`
- Cleanup footers are attached post-owner declarations.
- `-cleanup` is valid only immediately after the owning body dedents back to owner indentation.
- This scope does not define a generic `-name` footer family.
- Future headed-region or other dash-form work may reuse `-cleanup` or other `-name` spellings in other contexts, but that is outside this v1 contract.

## Valid Owners

- `fn`
- `async fn`
- `behavior`
- `system`
- full `if/else` constructs
- `while`
- `for`
- statement-form qualified phrases with attached blocks
- statement-form memory phrases with attached blocks

## Attachment Rules

- A cleanup footer must appear immediately after the owning body dedents to the owner indentation.
- It attaches to that owner, never to an inner nested block.
- For `if/else`, attachment happens only after the full construct, not after the `then` branch alone.
- Multiple consecutive cleanup footers may attach to the same owner.
- If any ordinary sibling item or statement appears first, attachment is lost and the cleanup footer is invalid.

## Footer Forms

- Bare cleanup:
  - `-cleanup`
- Targeted default cleanup:
  - `-cleanup[target = name]`
- Targeted override cleanup:
  - `-cleanup[target = name, handler = path]`

Named-field rules:

- field order is unrestricted on input
- canonical field order is `target`, then `handler`
- unknown fields are invalid
- duplicate fields are invalid
- `handler` requires `target`
- an explicit `target` name must resolve uniquely within the owning scope

Stacking rules:

- at most one bare `-cleanup` footer per owner
- targeted footers must use unique `target` bindings
- targeted footers override bare cleanup for those bindings
- `-cleanup[target = x]` is invalid when bare `-cleanup` is already present because it is redundant

## Scope Meaning

- Bare `-cleanup` covers every cleanup-capable owning binding activated in the owning scope.
- For routines that means:
  - owning parameters
  - cleanup-capable locals activated in the routine body
- For block-owning statements that means:
  - cleanup-capable locals activated in that owner block
- `read` / `edit` / ref-style parameters are not cleanup-capable for footer coverage.
- Non-owning bindings are not cleanup-capable for footer coverage.

## Cleanup Contract

The default cleanup contract is:

```arcana
trait Cleanup[T]:
    fn cleanup(take self: T) -> Result[Unit, Str]
```

with:

- `lang cleanup_contract = std.cleanup.Cleanup`

Default cleanup requirements:

- the binding must be ownership-eligible
- a concrete statically resolved `Cleanup[T]` impl must exist
- trait-bound-only or otherwise unresolved cleanup dispatch is not part of v1

Explicit handler requirements:

- `handler` is an override, not a substitute for cleanup eligibility
- `target` must still be cleanup-capable
- `handler` must resolve statically to a named callable path
- `handler` must be synchronous
- `handler` must accept exactly one compatible `take` parameter
- `handler` must return `Result[Unit, Str]`

## Execution Model

- Cleanup footer work runs on:
  - normal fallthrough
  - `return`
  - `break`
  - `continue`
  - `?` propagation that exits the owner
- Covered bindings clean up in reverse lexical activation order.
- Targeted overrides replace the default cleanup callee for their binding.
- `Err(Str)` from cleanup is fail-fast:
  - later cleanup entries do not run
  - the cleanup error overrides the original exit outcome

## Loop Semantics

- Loop cleanup is attached to the loop body scope, not to one final loop epilogue.
- Covered loop-body bindings clean up on each body exit, including `continue`.
- This is true for both bare cleanup coverage and targeted overrides.

## Cleanup Ordering

- Inner nested scopes clean up before outer scopes.
- Local `defer` work for the same owner runs before that owner's cleanup footer work.
- Owner/object cleanup runs after those local cleanup rings.

## Static Restrictions

- Cleanup-covered bindings may not be moved after activation.
- Cleanup-covered bindings may not be reassigned after activation.
- There is no disarm or transfer operation in v1.
- Cleanup handlers are named callable paths only.
- Cleanup footers do not depend on closures, callable values, or dynamic dispatch.

## Explicit Exclusions

- implicit destructors or RAII
- closure or callable-value cleanup
- dynamic cleanup dispatch
- arbitrary expression targets
- generic dash-form footer semantics beyond `-cleanup`
