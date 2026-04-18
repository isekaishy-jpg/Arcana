# Generic Memory System Spec (Saved Reference)

Status: `reference-only`

Historical carryover note:
- This Meadow-era planning/spec text is kept for audit context.
- Current rewrite authority for memory surface now lives in:
  - `docs/specs/memory/memory/v1-scope.md`
  - `docs/specs/headed-regions/headed-regions/v1-scope.md`

This is the canonical generic reference for Arcana memory-system evolution after Plan 31.

## Goals

- Keep storage context explicit in source.
- Keep deterministic runtime behavior.
- Keep diagnostics explicit for stale/invalid handles.
- Keep public APIs shelf-first under `std.memory`.
- Preserve a path to richer models without forcing borrow syntax early.

## Memory Phrase Shape

- `memory_type: instance :> args? <: qualifier`
- Inline args are comma-separated and limited to 3 top-level items.
- Constructor/qualifier must be `path` or `path[type_args]`.
- Attached blocks are statement-context only.
- Attached blocks support `name = expr` entries and chain lines.
- Attached entries may carry forewords as header-local metadata.
- If more than 3 independent inputs are needed, group them into ordinary tuple/record data rather than treating phrase arity as a reason to add broader callable transport.

## Core Model

- `memory_type` selects allocator family semantics.
- `instance` is a typed allocator value.
- qualifier call constructs a value of allocator item type.
- allocation returns an ID handle type, not a borrowed reference.

## Handle Rules

- Handles are typed IDs (`*Id[T]`).
- Stale detection is mandatory.
- Invalid/stale access is a deterministic runtime error.
- No implicit lifetime/borrow surface in this phase family.

## Allocator Families (Design Envelope)

- `arena`: simple typed arena with generation invalidation.
- `frame`: reset-oriented arena for per-tick scratch lifetimes.
- `pool`: slot reuse allocator with per-slot generation checks.
- Deferred families: `temp`, `session`, and other policy-specific classes.

## Safety and Concurrency Defaults

- Allocator values are non-Copy.
- ID handles may be copy-like by family decision.
- Cross-thread sendability is opt-in and requires explicit validity model.

## Evolution Targets

- Borrowed read views with clear aliasing rules.
- Shared/sendable allocator classes with cross-thread safety model.
- Iterator/compaction APIs for pool-style allocators.
- Optional scheduler-integrated reset policies once semantics are locked.
