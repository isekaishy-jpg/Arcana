# Memory v1 Scope

Status: `approved-pre-selfhost`

This scope defines the current rewrite-era Arcana memory contract.

Headed-region note:
- `docs/specs/headed-regions/headed-regions/v1-scope.md` now defines the `Memory` headed region as a spec-defining source form.
- This memory scope remains the authority for current families, allocator behavior, typed ids, stale detection, `reset` / `remove`, borrow law, and memory-phrase consumer semantics.

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
- Memory phrases are the consumer surface, not the sole source of memory definition.
- `Memory` headed regions coexist with memory phrases:
  - `Memory` defines reusable memory specs
  - memory phrases consume those specs through the approved memory surface
- Current shape:
  - `memory_type: instance :> args? <: qualifier`
- Inline args remain comma-separated and capped at 3 top-level items.
- Attached blocks follow header-phrase attachment rules.

## `Memory` Headed Region Relationship

- `Memory` is an approved headed-region head.
- `Memory` uses the target slot `<memory_type>:<name>` to establish entries in a memory-spec namespace.
- `Memory` specs are legal at module scope and block scope.
- Module-scope specs are path-addressable in the memory-spec namespace.
- Block-scope specs are scoped to the enclosing execution body.
- Current approved `Memory` families remain:
  - `arena`
  - `frame`
  - `pool`
- Current detail-line key family for v1:
  - `capacity`
  - `growth`
  - `recycle`
  - `handle`
  - `pressure`
- Current approved `Memory` modifiers are:
  - `arena`: `alloc`, `grow`, `fixed`
  - `frame`: `alloc`, `grow`, `recycle`
  - `pool`: `alloc`, `grow`, `fixed`, `recycle`
- `Memory` modifiers do not take payload expressions in v1.
- Current family-specific atom tables are:
  - `arena.pressure`: `bounded`, `elastic`
  - `arena.handle`: `stable`, `unstable`
  - `frame.pressure`: `bounded`, `elastic`
  - `frame.recycle`: `manual`, `frame`
  - `pool.pressure`: `bounded`, `elastic`
  - `pool.recycle`: `free_list`, `strict`
  - `pool.handle`: `stable`, `unstable`
- `Memory` is for explicit budgeting, lifetime shape, pressure behavior, and reusable role-shaped specs.
- `Memory` specs may be defined in modules, including `std`, and consumed from non-local code.
- `Memory` is not an alternate direct allocation surface and does not replace the current memory phrase consumer model.
- Runtime policy application in v1:
  - `pressure = elastic` allows the live budget to expand by `growth` when the allocator saturates
  - `pressure = bounded` rejects allocation once the current budget is full
  - `frame.recycle = frame` may reset the frame arena on saturation instead of rejecting immediately
  - `pool.recycle = free_list` reuses removed slots before minting new ones, while `strict` waits until reset
  - `handle = stable` reuses the same materialized spec handle for the spec lifetime, while `unstable` rematerializes a fresh handle per consumer resolution
- `Memory` default modifiers provide the strategy defaults for omitted policy dimensions, and per-detail modifiers locally override that strategy for the participating detail.

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
- Additional `Memory` families are not implied by this scope.
- Cross-thread/shared allocator expansion remains future work unless later approved explicitly.
- `docs/specs/memory/memory/generic-memory-spec.md` remains historical reference context, not the rewrite-defining memory authority.
