# Opaque Handles And Resource Lifecycle v1 Scope

Status: `approved-pre-selfhost`

This scope extracts the current rewrite-era contract for typed opaque handles and runtime resources.

## Baseline Contract

- Source-declared opaque handle families remain part of the active Arcana surface.
- Current approved families include:
  - `arcana_winapi.desktop_handles.Window`
  - `arcana_winapi.desktop_handles.FrameInput`
  - `arcana_winapi.desktop_handles.Session`
  - `arcana_winapi.desktop_handles.WakeHandle`
  - `arcana_winapi.process_handles.FileStream`
  - `arcana_winapi.audio_handles.AudioDevice`
  - `arcana_winapi.audio_handles.AudioBuffer`
  - `arcana_winapi.audio_handles.AudioPlayback`
- These are typed families, not erased generic runtime handles.
- `arcana_desktop`, `arcana_process`, and `arcana_audio` may use these handles in their public signatures, but apps/tooling must treat the `arcana_winapi.*_handles` declarations as the only canonical type paths.

## Ownership Contract

- Resource handles obey the same `read` / `edit` / `take` law as other Arcana values.
- Consuming operations invalidate the consumed handle by ordinary ownership law.
- Resource lifecycle rules must be explicit and diagnosable.
- Runtime/backend layers must treat them as ordinary canonical opaque types, not as duplicated public handle families stitched together with hidden aliasing.

## Rewrite Guidance

- The concrete ABI/representation of handles is not frozen here.
- The binding-owned typed-family boundary, ownership rules, validity rules, and diagnostics are frozen enough for pre-selfhost runtime/backend work.
- Runtime/backend work must not replace this with an erased catch-all resource carrier.

## Relationship To Host/App Scopes

- This scope complements:
  - `docs/specs/selfhost-host/selfhost-host/v1-scope.md`
  - `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md`
- Those scopes define approved domain surfaces; this scope defines the common typed-handle/resource law they rely on.
