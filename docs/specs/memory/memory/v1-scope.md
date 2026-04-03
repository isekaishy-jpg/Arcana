# Memory v1 Scope

Status: `approved-pre-selfhost`

This scope defines the current rewrite-era Arcana memory contract.

Headed-region note:
- `docs/specs/headed-regions/headed-regions/v1-scope.md` defines the `Memory` headed region as the spec-defining source form.
- This memory scope remains the authority for allocator families, views, typed ids, stale detection, `reset` / `remove`, `seal` / `unseal`, borrow law, deterministic ordering, and memory-phrase consumer semantics.

Freeze-exception note:
- Memory is now under an explicit pre-selfhost freeze exception.
- The approved exception covers:
  - new families `temp`, `session`, `ring`, and `slab`
  - new `Memory` detail keys `reset_on`, `page`, `overwrite`, and `window`
  - broadened memory-phrase behavior for `ring`
  - explicit borrowed-slice syntax `&x[a..b]` and `&mut x[a..b]`

## Included Families

- `arena`
- `frame`
- `pool`
- `temp`
- `session`
- `ring`
- `slab`

These are the current approved allocator families before selfhost.

## Family-Set Note

- The memory family set is no longer intentionally capped at the original three-family bootstrap floor.
- The approved family inventory is now large enough to support:
  - ordinary arena/frame/pool ownership
  - scratch lifetimes
  - long-lived published compiler data
  - circular game/app buffers
  - stable-slot reusable tables
- Public family names are semantic, not backend-implementation names:
  - no public `stack`
  - no public `heap`

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
- Phrase semantics are family-explicit:
  - id-allocator families construct a value and return a typed id handle
  - `ring` constructs a value, pushes it into the buffer, and returns `RingId[T]`
- Views are not a phrase family and do not define a separate memory-phrase ride.

## `Memory` Headed Region Relationship

- `Memory` is an approved headed-region head.
- `Memory` uses the target slot `<memory_type>:<name>` to establish entries in a memory-spec namespace.
- `Memory` specs are legal at module scope and block scope.
- Module-scope specs are path-addressable in the memory-spec namespace.
- Block-scope specs are scoped to the enclosing execution body.
- Current approved `Memory` families are:
  - `arena`
  - `frame`
  - `pool`
  - `temp`
  - `session`
  - `ring`
  - `slab`
- Current detail-line key family for v1:
  - common keys:
    - `capacity`
    - `growth`
    - `pressure`
    - `handle`
    - `reset_on`
  - carried family-specific keys:
    - `recycle` for `frame` and `pool`
    - `overwrite` and `window` for `ring`
    - `page` for `slab`
- Current approved `Memory` modifiers are:
  - `arena`: `alloc`, `grow`, `fixed`
  - `frame`: `alloc`, `grow`, `recycle`
  - `pool`: `alloc`, `grow`, `fixed`, `recycle`
  - `temp`: `alloc`, `grow`, `fixed`
  - `session`: `alloc`, `grow`, `fixed`
  - `ring`: `alloc`, `grow`, `fixed`
  - `slab`: `alloc`, `grow`, `fixed`
- `Memory` modifiers do not take payload expressions in v1.
- Current family-specific atom tables are:
  - `arena.pressure`: `bounded`, `elastic`
  - `arena.handle`: `stable`, `unstable`
  - `frame.pressure`: `bounded`, `elastic`
  - `frame.recycle`: `manual`, `frame`
  - `frame.reset_on`: `manual`, `frame`, `owner_exit`
  - `pool.pressure`: `bounded`, `elastic`
  - `pool.recycle`: `free_list`, `strict`
  - `pool.handle`: `stable`, `unstable`
  - `temp.pressure`: `bounded`, `elastic`
  - `temp.reset_on`: `manual`, `frame`, `owner_exit`
  - `session.pressure`: `bounded`, `elastic`
  - `session.handle`: `stable`
  - `session.reset_on`: `manual`
  - `ring.pressure`: `bounded`, `elastic`
  - `ring.overwrite`: `oldest`
  - `slab.pressure`: `bounded`, `elastic`
  - `slab.handle`: `stable`
- `Memory` is for explicit budgeting, lifetime shape, pressure behavior, publication state, and reusable role-shaped specs.
- `Memory` specs may be defined in modules, including `std`, and consumed from non-local code.
- `Memory` is not an alternate direct allocation surface and does not replace the current memory-phrase consumer model.

## Family Semantics

### `arena`

- append-oriented general arena
- typed ids
- `remove`
- `reset`
- borrow-read / borrow-edit
- no sealing

### `frame`

- frame-oriented scratch arena
- typed ids
- `reset`
- no per-item `remove`
- `recycle = frame` remains the current reset-on-saturation policy spelling for the existing frame family

### `pool`

- dense reusable-slot family
- typed ids with stale detection
- `remove`
- `reset`
- live-entry iteration
- explicit compaction
- compaction invalidates old ids/views and returns explicit relocation rows

### `temp`

- scratch bump family
- append-only
- no per-item `remove`
- ids are valid only until reset
- supports `reset_on = manual | frame | owner_exit`

### `session`

- append-only long-lived family
- stable ids
- no per-item `remove`
- explicit reset only
- intended backing for compiler intern tables and frozen compiler data
- supports publication through `seal` / `unseal`

### `ring`

- circular sequence/buffer family
- overwrite-oldest is fixed behavior
- `window` is the configured maximum readable or writable ring-window length
- `push` and memory phrases return `RingId[T]`
- overwritten entries deterministically stale evicted ids
- operation surface includes `push`, `pop`, readable/writable windows, and `reset`
- no arbitrary remove-by-id
- read order is oldest-to-newest

### `slab`

- paged stable-slot family
- free-slot reuse
- `page` is the slab growth quantum when elastic growth is enabled
- no compaction
- generation-checked ids
- intended backing for stable compiler graph/node tables that need reuse before publication
- supports publication through `seal` / `unseal`

## Publication Contract

- `seal(edit self)`
- `unseal(edit self)`
- `is_sealed(read self) -> Bool`
- Publication-state operations are approved only for:
  - `session`
  - `slab`
- `seal` transitions the allocator into read-only published state.
- While sealed, all mutating operations are rejected.
- `unseal` returns the allocator to mutable state.
- `unseal` is allowed only when there are no conflicting live views/borrows or exported descriptor views still tied to that allocator.
- Compiler guidance:
  - `session` is the default append-only published backing for interners and frozen tables
  - `slab` is the default stable-slot backing where pre-publication reuse matters

## Deterministic Order Guarantees

- `session` ids allocate monotonically and iteration is increasing id order
- `slab` live iteration is increasing slot-id order
- `pool` live iteration is increasing current slot order
- `pool` compaction preserves relative live-entry order
- `pool` relocation mappings are emitted in increasing old-slot order
- `ring` read order is oldest-to-newest

These guarantees are public contract because compiler caches, diagnostics, and stable output depend on them.

## Views and Borrowed Slices

- Approved first-class view types:
  - `ReadView[T]`
  - `EditView[T]`
  - `ByteView`
  - `ByteEditView`
  - `StrView`
- Views are contiguous-only in this window.
- View aliasing is shared-read / exclusive-edit.
- Invalidating mutations must reject conflicting live views:
  - `reset`
  - `remove`
  - overwrite
  - compaction
  - `unseal`
- Current owned slice behavior remains unchanged:
  - `x[a..b]` is an owned slice/copy
- Borrowed-slice creation is explicit:
  - `&x[a..b]`
  - `&mut x[a..b]`
- Borrowed-slice syntax is intentionally narrow:
  - built-in/indexable std surfaces
  - view types
  - generic trait users use explicit methods
- `StrView` is UTF-8-valid and read-only.
- No mutable string view is approved in this window.
- Approved explicit trait layer over the concrete memory/view surface:
  - `ViewSource[S]`
  - `EditViewSource[S]`
  - `ContiguousBytes[S]`
  - `ContiguousBytesEdit[S]`
  - `Resettable[S]`
  - `IdAllocating[S]`
  - `LiveIterable[S]`
  - `Compactable[S]`
  - `SequenceBuffer[S]`
  - `Sealable[S]`
- These traits are capability glue for explicit generic code; they are not implicit coercion or autoderef mechanisms.

## Borrow Contract

- `borrow_read` and `borrow_edit` remain part of the active memory surface.
- Borrow behavior is current contract, not deferred hypothesis.
- Borrowed-slice view creation is also current contract.
- Borrow semantics must integrate with ordinary Arcana ownership/place law rather than using a separate erased allocator exception.

## Concurrency and Sharing

- `arena`, `frame`, `pool`, and `temp` remain local in this window.
- `session` and `slab` become shareable only while sealed.
- `ring` is move-only across workers by default; it is not approved as shared mutable state.
- Read views may be sent/shared only when their backing family allows it.
- Edit views remain exclusive and non-shareable.

## Runtime Materialization Contract

- `pressure = elastic` allows the live budget to expand by `growth` when the allocator saturates
- `pressure = bounded` rejects allocation once the current budget is full
- `frame.recycle = frame` may reset the frame arena on saturation instead of rejecting immediately
- `temp.reset_on = frame` may reset the temp arena on saturation instead of rejecting immediately
- `frame.reset_on = owner_exit` and `temp.reset_on = owner_exit` reset owner-bound stable materialized specs when the bound owner exits
- `pool.recycle = free_list` reuses removed slots before minting new ones, while `strict` waits until reset
- `ring.overwrite = oldest` evicts the oldest live slot when the ring is saturated and growth does not occur
- `ring.window` rejects `window_read` / `window_edit` requests whose requested length exceeds the configured cap
- `slab.page` determines the minimum capacity growth step when an elastic slab expands after saturation
- `handle = stable` reuses the same materialized spec handle for the spec lifetime, while `unstable` rematerializes a fresh handle per consumer resolution
- `seal` / `unseal` are publication-state operations, not hidden synchronization

## Current Boundaries

- No public `stack` or `heap` family names are approved.
- No implicit coercion or autoderef growth is implied by view support.
- Views are contiguous-only in this window.
- Descriptor views at the native/provider boundary must stay typed and explicit; no raw refs or erased value carriers are approved.
- `docs/specs/memory/memory/generic-memory-spec.md` remains historical reference context, not rewrite authority.
