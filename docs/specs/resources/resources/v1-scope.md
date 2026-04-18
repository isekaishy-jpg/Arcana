# Opaque Handles And Resource Lifecycle v1 Scope

Status: `approved-pre-selfhost`

This scope extracts the current rewrite-era contract for typed opaque handles and runtime resources.

## Baseline Contract

- Source-declared opaque handle families remain part of the active Arcana surface.
- Current approved families include:
  - `arcana_process.fs.FileStream`
- These are typed families, not erased generic runtime handles.
- `arcana_process` and any future higher-level layers may use these handles in their public signatures, but apps/tooling must treat the owning public package declaration as the canonical type path.

## Ownership Contract

- Resource handles obey the same `read` / `edit` / `take` law as other Arcana values.
- Consuming operations invalidate the consumed handle by ordinary ownership law.
- Resource lifecycle rules must be explicit and diagnosable.
- Runtime/backend layers must treat them as ordinary canonical opaque types, not as duplicated public handle families stitched together with hidden aliasing.

## Rewrite Guidance

- The concrete ABI/representation of handles is not frozen here.
- The typed-family boundary, ownership rules, validity rules, and diagnostics are frozen enough for pre-selfhost runtime/backend work.
- Runtime/backend work must not replace this with an erased catch-all resource carrier.

## Relationship To Host/App Scopes

- This scope complements:
  - `docs/specs/selfhost-host/selfhost-host/v1-scope.md`
- Those scopes define approved domain surfaces; this scope defines the common typed-handle/resource law they rely on.
