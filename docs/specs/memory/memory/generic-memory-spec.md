# Generic Memory System Spec (Saved Reference)

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
- Attached blocks are statement-context only and currently use `name = expr` entries.

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
