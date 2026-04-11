# Memory Expansion: Views, Compiler Publication, and New Families

## Summary
- Expand Arcana memory now as a real substrate pass for desktop apps, games, `arcana_text`, and compiler/selfhost workloads.
- Keep the surface semantic and explicit: no public `stack` or `heap`, no implicit coercion, no compiler-only memory exceptions.
- Add first-class views, ratify `temp`, `session`, `ring`, and `slab`, add `std.binary`, and explicitly add compiler-oriented publish semantics through `seal` / `unseal`.
- This plan is a deliberate language-freeze exception in the memory domain. The exception covers only:
  - new memory families
  - new `Memory` detail keys
  - broadened memory-phrase behavior for `ring`
  - borrowed-slice view syntax

## Key Changes
### Memory Contract and Freeze Exception
- Update the approved memory and headed-region scopes so the active family set is:
  - existing: `arena`, `frame`, `pool`
  - new: `temp`, `session`, `ring`, `slab`
- Keep `Memory` as the spec-defining form and memory phrases as the consumer surface.
- Broaden memory phrases only where the family model is explicit:
  - id families still construct a value and return a typed id
  - `ring` phrases construct a value, push it, and return `RingId[T]`
  - views are not a phrase family
- Add `reset_on` as a `Memory` detail key with atoms:
  - `manual`
  - `frame`
  - `owner_exit`
- Keep auto-reset opt-in only through `reset_on`.
- Keep common detail keys where they fit:
  - `capacity`, `growth`, `pressure`, `handle`, `reset_on`
- Add family-only keys explicitly:
  - `page` for `slab`
  - `overwrite` and `window` for `ring`
- Update the memory and headed-region deferred ledgers in the same patch so these additions are no longer tracked as deferred.

### Family Semantics
- `temp`
  - scratch bump family
  - append-only
  - no per-item `remove`
  - ids valid only until reset
  - supports `reset_on = manual | frame | owner_exit`
- `session`
  - append-only long-lived family
  - stable ids
  - no per-item `remove`
  - explicit reset only
  - intended backing for compiler intern tables and frozen compiler data
- `ring`
  - circular sequence/buffer family
  - overwrite-oldest is fixed behavior
  - `push` and ring phrases return `RingId[T]`
  - overwritten entries deterministically stale evicted ids
  - operation surface includes `push`, `pop`, readable/writable window views, and `reset`
  - no arbitrary remove-by-id
- `slab`
  - paged stable-slot family
  - free-slot reuse
  - no compaction
  - generation-checked ids
  - intended backing for stable compiler graph/node tables that need reuse before publication
- `pool`
  - keep as the dense compactable family
  - add live-entry iteration
  - add explicit compaction
  - compaction invalidates old ids/views and returns an explicit relocation mapping

### Compiler-Oriented Publication Semantics
- Add explicit publication operations for `session` and `slab`:
  - `seal(edit self)`
  - `unseal(edit self)`
  - `is_sealed(read self) -> Bool`
- `seal` / `unseal` are not added to `temp`, `arena`, `frame`, `pool`, or `ring` in this window.
- `seal`
  - transitions the allocator into read-only published state
  - while sealed, all mutating operations are rejected
  - this includes alloc/edit/remove/reset and any family-specific mutator
- `unseal`
  - returns the allocator to mutable state
  - allowed only when there are no live views/borrows and no exported/shared descriptor views referencing that allocator
- Compiler guidance is explicit:
  - `session` is the default append-only published backing for interners and frozen tables
  - `slab` is the default stable-slot backing where pre-publication reuse matters
  - publication between mutable phases and parallel/read-only phases happens through `seal`, not through undocumented convention

### Deterministic Order Guarantees
- Lock deterministic iteration and relocation order now:
  - `session` ids allocate monotonically and iteration is increasing id order
  - `slab` live iteration is increasing slot-id order
  - `pool` live iteration is increasing current slot order
  - `pool` compaction preserves relative live-entry order
  - `pool` relocation mappings are emitted in increasing old-slot order
  - `ring` read order is oldest-to-newest
- These guarantees are part of the public contract because compiler caches, diagnostics, and stable output depend on them.

### Views, Borrowing, and Syntax
- Add first-class view types:
  - `ReadView[T]`
  - `EditView[T]`
  - `ByteView`
  - `ByteEditView`
  - `StrView`
- Lock aliasing to shared-read / exclusive-edit.
- Invalidating mutations are rejected while a live conflicting view exists:
  - reset
  - remove
  - overwrite
  - compaction
  - unseal
- Keep current owned slice behavior unchanged:
  - `x[a..b]` remains an owned slice/copy
- Add explicit borrowed-slice creation:
  - `&x[a..b]`
  - `&mut x[a..b]`
- Keep borrowed-slice syntax narrow:
  - built-in/indexable std surfaces
  - view types
  - generic trait users use explicit methods
- `StrView` is UTF-8-valid and read-only.
- No mutable string view in this window.
- No implicit coercion or autoderef expansion.

### Std and Trait Surface
- Expand `std.memory` with concrete family and view types:
  - `TempArena[T]`, `TempId[T]`
  - `SessionArena[T]`, `SessionId[T]`
  - `RingBuffer[T]`, `RingId[T]`
  - `Slab[T]`, `SlabId[T]`
  - the new view types
- Add `std.binary` as a narrow binary parsing/emission module:
  - reader
  - writer
  - seek/skip/remaining
  - endian-aware integer operations
  - subview operations
- Keep `std.bytes` and `std.text` as convenience layers over the view substrate.
- Add only narrow explicit traits:
  - view/binary traits:
    - `ViewSource[T]`
    - `EditViewSource[T]`
    - `ContiguousBytes`
    - `ContiguousBytesEdit`
    - `BinaryReadable[T]`
    - `ByteSink`
  - family capability traits:
    - resettable
    - id-allocating
    - live-iterable
    - compactable
    - sequence-buffer
    - sealable
- Keep concrete types primary.

### Native and Provider Boundary
- Extend the cabi/provider contract so full views can cross boundaries as typed descriptor views, not raw refs.
- Descriptor views are allowed only over:
  - approved memory-family backing stores
  - byte buffers
  - string buffers
- Ordinary `List`/`Array` values do not cross as shared view windows in this plan.
- Descriptor metadata must include:
  - backing owner/binding identity
  - family/backing kind
  - element type/layout identity
  - start
  - length
  - mutability
- `StrView` stays UTF-8-valid and read-only at the boundary too.
- Descriptor views over `session` and `slab` obey seal rules:
  - read descriptors may cross only while sealed
  - edit descriptors remain exclusive and local

### Concurrency and Sharing
- Refine cross-thread rules now:
  - `session` and `slab` become shareable only while sealed
  - `ring` is move-only across workers by default, not shared mutable state
- Read views may be sent/shared only when their backing family allows it.
- Edit views remain exclusive and non-shareable.

### Compiler and Runtime Work
- Update parser, frontend, HIR, IR, runtime, std, and kernel seams for:
  - new families
  - new `Memory` keys and atoms
  - broadened ring phrase lowering
  - seal/unseal state
  - deterministic iteration guarantees
  - view types and borrowed-slice syntax
  - explicit share rules
  - pool iteration and compaction
  - descriptor-view ABI support
- Update the selfhost/conformance matrices in the same window.
- Do not add compiler-only memory builtins, hidden interning exceptions, or special parser/runtime lanes.

## Test Plan
- Parser/frontend tests for:
  - `Memory` specs with `temp`, `session`, `ring`, `slab`
  - `reset_on`
  - borrowed-slice syntax
  - broadened ring phrases
  - seal/unseal method and type checking
- Runtime tests for:
  - `temp` manual/frame/owner-exit reset
  - `session` append-only stable ids
  - `session` and `slab` seal/unseal mutation rejection
  - `ring` overwrite-oldest staling
  - `slab` page growth and reused-slot generation checks
  - `pool` live iteration and explicit compaction with relocation mapping
- Determinism tests for:
  - session iteration order
  - slab iteration order
  - pool iteration order
  - pool compaction relocation-map order
  - ring oldest-to-newest view order
- Borrow/view tests for:
  - shared-read / exclusive-edit enforcement
  - rejection of invalidating mutations while views are live
  - rejection of `unseal` while exported/shared views are live
  - borrowed slices on bytes, text, arrays, and views
  - UTF-8-valid `StrView`
- `std.binary` tests for:
  - endian reads/writes
  - seek/skip/subview
  - `BinaryReadable[T]`
  - `ByteSink`
- Native/provider tests for:
  - descriptor-view encoding/decoding
  - type/layout validation
  - seal-gated read-descriptor export for `session`/`slab`
  - rejection of unsupported backing kinds
- Compiler-facing regression tests proving the new memory surface can back:
  - interner-style append-only published tables
  - stable-node tables with pre-publication reuse
  - parallel read-only access after publication

## Assumptions and Defaults
- No public `stack` or `heap` family names are added.
- Views are contiguous-only in this window.
- `x[a..b]` remains owned/copying; borrowed views use explicit borrowed-slice syntax.
- `StrView` is UTF-8-valid and read-only.
- Auto-reset is opt-in only and lives under `reset_on`.
- `seal` / `unseal` are publication-state operations, not generic hidden synchronization.
- `slab` is the stable non-compacting slot family; `pool` is the compactable dense family.
- No implicit coercion or autoderef growth is part of this plan.
- Descriptor views are the only full-view ABI model in this window.
