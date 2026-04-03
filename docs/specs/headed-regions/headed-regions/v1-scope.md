# Headed Regions v1 Scope

Status: `approved-pre-selfhost`

This scope defines the current rewrite-era headed-region contract.

Implementation note:
- Parser/frontend/HIR/IR/runtime support is now landed for the current rewrite.
- `conformance/selfhost_language_matrix.toml` now includes `headed_regions_v1`.
- The implementation keeps headed regions first-class through executable IR/runtime instead of desugaring them away.
- Native/AOT support is backend-stable in v1: `Memory`, bool-only `recycle`, and `bind require` participate in the current direct subset, while general `construct`, payload-bearing headed-region lines, and named owner exits fall back to runtime dispatch inside native bundles.

## Core Contract

- A headed region is a structural inner block whose head defines what the region does with its top-level participating lines.
- A headed region is not an ordinary execution block.
- A headed region is not an attached footer form.
- The current approved heads are:
  - `recycle`
  - `construct`
  - `bind`
  - `Memory`
- User-defined heads are not part of v1.

## Family Doctrine

- A headed region has one ride.
- The ride is defined by the head.
- Each headed region declares one default modifier.
- Each head defines whether it also requires, allows, or forbids a second target/completion slot.
- Top-level lines are interpreted through the head ride, not as arbitrary freeform statements.
- Per-line override may replace the default modifier only where that head allows it.
- Participating lines that do not fit the head ride are invalid.
- Headed regions do not invent hidden rollback, transactional semantics, or special cleanup behavior beyond ordinary Arcana control flow.

## Generic Shape

Generic headed-region shape:

```arcana
<head> <target_or_completion> -<default_modifier>
    <line>
    <line> -<override_modifier>
    <line>
```

Fallthrough-only heads may omit the second slot:

```arcana
<head> -<default_modifier>
    <line>
    <line>
```

Placement rules:

- Headed regions are inner indented blocks only.
- `Memory` is also legal at module scope as a memory-spec declaration.
- `construct yield` is the only expression-form headed region in v1.
- They are not top-level attachments.
- This scope does not define any generic `-name` footer family.
- Cleanup footers remain governed by `docs/specs/page-rollups/page-rollups/v1-scope.md`.

## Approved Heads

### `recycle`

- `recycle` is a structural exit-routing region.
- Ride:
  - each participating top-level line either proves continued progression or triggers an explicit scoped exit on failure
- Shape:

```arcana
recycle -<exit_action>
    <participating_line>
    <participating_line> -<exit_action>
```

- Success:
  - no participating line triggered an exit
  - any bindings the participating forms define as surviving remain available after the region
  - execution falls through normally
- Default/per-line modifier class:
  - explicit exit action such as `return`, `continue`, `break`, or named owner exit

### `construct`

- `construct` is a structured construction region.
- Ride:
  - each participating top-level line contributes to explicit materialization of a structured result, and successful completion finishes that result in one declared way
- Shape:

```arcana
construct <completion_clause> -<default_modifier>
    <construction_line>
    <construction_line> -<override_modifier>
```

- Success:
  - the construction ride is satisfied
  - the constructed result is completed by the declared completion clause
- Current completion-clause family includes:
  - `yield <ctor_path>`
  - `deliver <ctor_path> -> <name>`
  - `place <ctor_path> -> <target>`
- Default/per-line modifier class:
  - failed contribution or sanctioned acquisition behavior
- Participating lines are named contributions:
  - `field = expr` for records
  - `payload = expr` for single-payload enum variants

### `bind`

- `bind` is a structured establishment region.
- Ride:
  - each participating top-level line attempts to establish a binding or refinement under one declared establishment policy
- Shape:

```arcana
bind -<binding_modifier>
    <binding_line>
    <binding_line> -<override_modifier>
```

- Success:
  - required bindings are established
  - bindings the head defines as surviving remain available after the region
  - execution falls through normally
- Default/per-line modifier class:
  - failed establishment behavior such as `return`, `default`, `preserve`, or `replace`
- `require <expr>` is the dedicated boolean guard line spelling in v1.
- `require <expr>` supports `return`, `break`, and `continue` failure handling in v1.
- `break` / `continue` remain legal only inside loops.

### `Memory`

- `Memory` is a memory-spec defining region.
- `Memory` does not perform allocation itself.
- `Memory` defines a reusable memory specification.
- Memory phrases and other memory-aware language forms are consumers of those specs.
- Ride:
  - each participating top-level line contributes explicit family-defined memory-spec detail for one named memory spec
- Shape:

```arcana
Memory <memory_type>:<name> -<memory_modifier>
    <memory_detail_line>
    <memory_detail_line> -<override_modifier>
```

- Success:
  - the named memory spec is established in the memory-spec namespace
  - that spec becomes available to memory phrases and other approved memory-aware forms
  - it does not become an ordinary value binding merely by being established
- Target slot:
  - `<memory_type>:<name>`
- Current approved `Memory` families are:
  - `arena`
  - `frame`
  - `pool`
  - `temp`
  - `session`
  - `ring`
  - `slab`
- Default/per-line modifier class:
  - memory strategy / pressure behavior such as `alloc`, `grow`, `fixed`, or `recycle`, constrained by family
- Current approved `Memory` modifiers are:
  - `arena`: `alloc`, `grow`, `fixed`
  - `frame`: `alloc`, `grow`, `recycle`
  - `pool`: `alloc`, `grow`, `fixed`, `recycle`
  - `temp`: `alloc`, `grow`, `fixed`
  - `session`: `alloc`, `grow`, `fixed`
  - `ring`: `alloc`, `grow`, `fixed`
  - `slab`: `alloc`, `grow`, `fixed`
- `Memory` modifiers do not take payload expressions in v1.
- Current detail-line keys are:
  - common:
    - `capacity`
    - `growth`
    - `pressure`
    - `handle`
    - `reset_on`
  - carried family-specific:
    - `recycle` for `frame` and `pool`
    - `overwrite` and `window` for `ring`
    - `page` for `slab`
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
- Intended scope:
  - explicit budgeting
  - lifetime shape
  - reset/recycle behavior
  - growth and pressure behavior
  - stable/unstable-handle policy
  - reusable role-shaped specs, including from `std`
- Runtime materialization contract in v1:
  - `pressure = elastic` allows the allocator budget to expand by `growth` when full
  - `pressure = bounded` rejects additional allocation once the current budget is exhausted
  - `frame.recycle = frame` may reset the frame arena on saturation instead of rejecting immediately
  - `temp.reset_on = frame` may reset the temp arena on saturation instead of rejecting immediately
  - `pool.recycle = free_list` reuses removed slots before issuing fresh ones, while `strict` withholds reuse until reset
  - `ring.overwrite = oldest` evicts the oldest live slot when the ring is saturated and growth does not occur
  - `handle = stable` reuses the same materialized spec handle for the spec lifetime, while `unstable` rematerializes a fresh handle on each consumer resolution
- `Memory` default modifiers provide the default strategy for omitted policy dimensions, and per-detail modifiers locally override that strategy for the participating detail.
- Explicitly not the purpose of `Memory`:
  - allocator trivia
  - backend implementation quirks
  - an alternate direct allocation surface
- Memory specs may be defined in modules, including `std`, and consumed from non-local code through the approved memory surface.

Memory relationship note:
- `docs/specs/memory/memory/v1-scope.md` remains the authority for the current memory families, allocator model, typed ids, stale detection, `reset` / `remove`, borrow behavior, and memory-phrase consumer contract.
- This headed-region scope is the authority for the `Memory` region as a source-language structural form.

## Modifier And Completion Families

- The headed-region family shares a modifier slot, not one universal modifier meaning.
- `recycle` uses exit-routing modifiers.
- `construct` uses construction-failure modifiers.
- `bind` uses establishment modifiers.
- `Memory` uses memory-strategy / pressure-behavior modifiers.
- Completion/target slot meaning is head-specific:
  - `recycle`: usually omitted because success is ordinary fallthrough
  - `construct`: explicit completion clause
  - `bind`: none in v1
  - `Memory`: required memory-spec identity

## Nesting Policy

- `recycle` may not nest another `recycle`.
- `Memory` rejects nested `Memory` in v1.
- Other nesting is conservatively rejected unless and until a later approved scope makes the resulting ride explicit.

## Compile-Time Enforcement

Compilation must reject:

- invalid or unknown region heads
- missing required target/completion clauses
- missing default modifiers
- invalid target/completion clauses for the selected head
- invalid default modifiers for the selected head or family
- invalid per-line overrides
- participating lines whose forms are not sanctioned for the selected head
- invalid context exits such as `recycle -continue` outside a loop
- invalid owner exits
- invalid `Memory` families
- invalid memory-strategy modifiers for the selected memory family
- ambiguous bodies that no longer read as one ride

Compilation may also reject or warn on:

- empty headed regions
- regions with no participating lines
- redundant defaults
- `Memory` regions with no real tuning detail
- bodies that read like ordinary imperative sequencing rather than headed-region input

## Explicit Boundaries

- This scope approves all four heads as language contract now.
- It does not claim current implementation support.
- It does not define user-defined heads.
- It does not define generalized nesting beyond the current conservative rules.
- It does not redefine cleanup footers, qualified phrases, memory phrases, or foreword targets beyond the explicit interactions noted above.
