# Chain v1 Scope

Status: `approved-pre-selfhost`

This scope defines the current rewrite-era chain surface contract.

## Surface Model

- Chain surface remains explicit as:
  - style qualifier
  - introducer family
  - connector-directed stage edges
- Current style family remains:
  - `forward`
  - `lazy`
  - `parallel`
  - `async`
  - `plan`
  - `broadcast`
  - `collect`

## Metadata Contract

- `#stage[...]` and `#chain[...]` remain part of the approved surface.
- Syntax/frontend must validate explicit chain metadata and system-boundary requirements.
- Approved chain metadata must survive parsing/lowering without being treated as stray unsupported text.

## Execution Guidance

- Async and parallel chain styles remain part of the active pre-selfhost contract.
- Current style semantics are:
  - `forward`: directional serial pipeline in normalized order
  - `async`: directional pipeline that auto-awaits task/thread results before feeding the next stage
  - `parallel`: true fanout spawn of downstream stages with deterministic result ordering
  - `broadcast`: sequential same-input fanout returning a `List`
  - `collect`: directional pipeline returning downstream outputs in normalized order, excluding the initial seed/input
  - `plan`: validate the chain contract, evaluate only the seed/input expression, skip downstream stage execution, and return the original input unchanged
  - `lazy`: produce a deferred chain value that executes at demand boundaries and only once
- Exact scheduler grouping and worker execution details are governed by `docs/specs/concurrency/concurrency/v1-scope.md`.
- `parallel` and `weave` may use task-substrate execution for `edit`-capable stages when same-place mutation can be preserved.
- `split` remains conservative for `edit`-capable stages across thread boundaries until an explicit transferable-place law is approved.
- The older Meadow-era matrix-style scheduling notes are historical reference, not direct rewrite runtime law.

## Boundaries

- This scope does not revive the archived Meadow scheduler tables as approved current law.
- This scope does preserve chain surface, style names, and contract metadata as current Arcana behavior.
