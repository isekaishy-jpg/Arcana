# Opaque Handles And Resource Lifecycle v1 Scope

Status: `approved-pre-selfhost`

This scope extracts the current rewrite-era contract for typed opaque handles and runtime resources.

## Baseline Contract

- Source-declared opaque handle families remain part of the active Arcana surface.
- Current approved families include:
  - `std.window.Window`
  - `std.canvas.Image`
  - `std.fs.FileStream`
  - `std.events.AppFrame`
  - `std.audio.AudioDevice`
  - `std.audio.AudioBuffer`
  - `std.audio.AudioPlayback`
- These are typed families, not erased generic runtime handles.

## Ownership Contract

- Resource handles obey the same `read` / `edit` / `take` law as other Arcana values.
- Consuming operations invalidate the consumed handle by ordinary ownership law.
- Resource lifecycle rules must be explicit and diagnosable.

## Rewrite Guidance

- The concrete ABI/representation of handles is not frozen here.
- The typed-family boundary, ownership rules, validity rules, and diagnostics are frozen enough for pre-selfhost runtime/backend work.
- Runtime/backend work must not replace this with an erased catch-all resource carrier.

## Relationship To Host/App Scopes

- This scope complements:
  - `docs/specs/selfhost-host/selfhost-host/v1-scope.md`
  - `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md`
- Those scopes define approved domain surfaces; this scope defines the common typed-handle/resource law they rely on.
