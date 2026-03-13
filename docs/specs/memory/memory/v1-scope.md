# Memory v1 Scope

Status: `approved-pre-selfhost`

This scope defines the current rewrite-era Arcana memory contract.

## Included Families

- `arena`
- `frame`
- `pool`

These are the current approved allocator families before selfhost.

## Family-Set Note

- The current family count is intentionally limited, but the memory model is not a placeholder-minimal carryover from Meadow-era Arcana.
- The rewrite baseline already depends on typed ids, stale detection, `reset`, `remove`, memory phrases, and borrow-read / borrow-edit behavior.
- Future family expansion is separate from the question of whether the current approved memory model is semantically real enough for the rewrite. It is.

## Memory Phrase Contract

- Memory phrases remain part of the active source surface.
- Current shape:
  - `memory_type: instance :> args? <: qualifier`
- Inline args remain comma-separated and capped at 3 top-level items.
- Attached blocks follow header-phrase attachment rules.

## Allocator Model

- Allocators are typed values.
- Allocation returns typed id handles.
- Stale detection is required.
- `reset` and `remove` are part of the active surface.

## Borrow Contract

- `borrow_read` and `borrow_edit` are part of the active memory surface.
- Borrow behavior is not a deferred hypothetical; it is current contract.
- Borrow semantics must integrate with ordinary Arcana ownership/place law rather than using a separate erased allocator exception.

## Current Boundaries

- Future allocator families are not implied by this scope.
- Cross-thread/shared allocator expansion remains future work unless later approved explicitly.
- `docs/specs/memory/memory/generic-memory-spec.md` remains historical reference context, not the rewrite-defining memory authority.
