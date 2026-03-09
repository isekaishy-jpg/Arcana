# Memory v2 Scope (Plan 31)

This file is the exact implemented scope for Plan 31.

## Implemented Now

- Memory phrase registry supports:
  - `arena`
  - `frame`
  - `pool`
- New public types:
  - `FrameArena[T]`, `FrameId[T]`
  - `PoolArena[T]`, `PoolId[T]`
- `std.memory` adds:
  - `frame_new[T](capacity)`
  - `pool_new[T](capacity)`
  - method surface for `FrameArena[T]` and `PoolArena[T]`
- Bytecode version bump to `22`.
- VM intrinsic dispatch for all frame/pool memory intrinsics.
- Deterministic stale-ID runtime errors:
  - `frame id is invalid or stale`
  - `pool id is invalid or stale`

## Explicit Runtime Semantics

- `FrameArena[T]`:
  - append-only allocation
  - explicit `reset`
  - reset invalidates old IDs
- `PoolArena[T]`:
  - free-list slot reuse
  - per-slot generation checks
  - explicit `remove` and `reset`
  - reset invalidates old IDs

## Explicitly Not in v2

- Borrow/reference views
- Implicit/automatic reset behavior
- Cross-thread allocator sharing
- `temp` / `session` executable families
- Pool iterator/compaction APIs
