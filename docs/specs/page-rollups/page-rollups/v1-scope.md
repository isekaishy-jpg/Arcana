# Page Rollups v1 Scope

Status: `approved-pre-selfhost`

This file records the v1 surface and semantics for Page Rollups.

Page Rollups are compiler-owned block terminators.
They are not forewords, not expressions, and not general postfix metadata.

## Goals

- Keep `defer` as the low-friction cleanup tool.
- Add an optional block-level cleanup form for larger headers.
- Keep cleanup explicit and statically resolved.
- Avoid RAII, closures, callable values, and dynamic dispatch.

## Non-Goals

- No implicit type destructors.
- No mandatory footer ceremony.
- No async cleanup in v1.
- No transfer/disarm model in v1.

## Core Rule

A block-owning header may be followed by zero or more rollup lines after its body dedents back to the header indentation.

If no rollup is present, behavior is unchanged from today.

## Syntax

General Page Rollup form:

- `[args...]#name`

v1 cleanup form:

- `[subject, handler]#cleanup`

v1 standardizes `#cleanup` only.

## Valid Owners

- `fn`
- `async fn`
- `behavior`
- `system`
- `if ... else ...` as one full construct
- `while`
- `for`
- statement-form qualified phrases with attached blocks
- statement-form memory phrases with attached blocks

## Attachment

- A rollup must appear immediately after the owning header body dedents to header indentation.
- It attaches to that owning header, never to an inner nested block.
- For `if/else`, the rollup appears only after the full construct, not after the `then` block alone.
- Multiple consecutive rollups may attach to the same header.
- If a normal sibling statement or item appears first, attachment is lost and the rollup is invalid.

## No-Footer Behavior

- A header is valid with no rollup.
- No footer means no header-level cleanup policy was declared.
- `defer` inside the block remains fully available and unchanged.
- Small or minimal operations should continue to use plain `defer` or no cleanup form at all.

## Subject Resolution

- `subject` must be a binding name.
- It may name:
  - a parameter of the owning header
  - a binding declared in the owning header body
- It may not name:
  - arbitrary expressions
  - field paths
  - bindings local only to a nested child scope
- Although written after the block, the rollup resolves names against the owning header's body scope.

## Handler Resolution

- `handler` must resolve statically to a named callable path.
- No closures.
- No function values.
- No dynamic dispatch objects.
- No `await` in cleanup handlers in v1.
- Page rollups do not depend on future function/context object work.

Lowering model:

```arc
[x, h]#cleanup
```

lowers semantically as:

```arc
h :: x :: call
```

## Activation Model

- Each owning header conceptually has a cleanup ledger.
- A `#cleanup` rollup declares an entry in that ledger.
- A parameter subject becomes active at header entry.
- A local subject becomes active only after successful initialization.
- If initialization never completes, no cleanup runs for that subject.

## Execution Model

- On exit from the owning header, active cleanup entries run in reverse textual rollup order.
- They run on:
  - normal fallthrough
  - `return`
  - `break`
  - `continue`
  - `?` propagation that exits the header
- For loops, a header rollup runs once when the loop statement exits, not once per iteration.

## Interaction With `defer`

- `defer` remains the preferred low-friction cleanup feature.
- Use `defer` for shorter-lived or local obligations.
- Use `#cleanup` when the whole header owns the cleanup policy.
- Inner nested scopes still run their own `defer` behavior as today.
- On exit from the owning header, ordinary `defer` entries in that header run before header rollup cleanup entries.
- This makes rollups the outer cleanup ring of the header.

## Static Restrictions v1

- A cleanup-bound subject may not be moved out of the owning header after it becomes active.
- A cleanup-bound subject may not be reassigned after it becomes active.
- There is no disarm or transfer operation in v1.
- Cleanup handler failure is fail-fast; later cleanup entries do not run.

## Recommended Positioning

- `defer` is the common path.
- `#cleanup` is the structured path.
- Footer use is optional and should not be required for routine small cleanup.

## Explicit Exclusions in v1

- RAII or implicit type-driven destruction
- Cleanup by callable value
- Cleanup by closure
- Dynamic cleanup dispatch
- Arbitrary expression subjects
- Per-iteration loop footer cleanup
