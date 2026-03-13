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
- Exact scheduler grouping and worker execution details are governed by `docs/specs/concurrency/concurrency/v1-scope.md`.
- The older Meadow-era matrix-style scheduling notes are historical reference, not direct rewrite runtime law.

## Boundaries

- This scope does not revive the archived Meadow scheduler tables as approved current law.
- This scope does preserve chain surface, style names, and contract metadata as current Arcana behavior.
