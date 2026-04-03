# Access Modes And Ownership v1 Scope

Status: `approved-pre-selfhost`

This scope extracts the current rewrite-era contract for Arcana access modes and ownership behavior.

Scope notes:
- This scope defines source-visible law for `read`, `edit`, `take`, and reference/place behavior.
- It applies equally to user routines, linked `std` routines, and host/runtime intrinsics.
- Runtime/backend work may refine implementation strategy, but it must not weaken these source-visible ownership rules.

## Baseline Contract

- Arcana call boundaries use explicit access modes.
- Unannotated value parameters are `read`.
- `edit` grants mutable access to a caller-observable place.
- `take` is consuming access.
- Ownership behavior must be consistent across:
  - user-defined routines
  - trait methods and impl methods
  - linked `std` wrappers
  - kernel/intrinsic calls

## `read`

- `read` does not consume the argument.
- `read` does not grant mutation rights to the callee.
- `read` may be used for copyable values, non-copy values, and opaque handles, but it must not invalidate the caller binding.

## `edit`

- `edit` targets an addressable caller-visible place.
- Mutations performed through `edit` are observable after the call returns.
- `edit` does not consume the place.
- Borrow, alias, and conflict diagnostics must reason about `edit` as mutable access to the same underlying place, not as a detached copy.

## `take`

- `take` consumes the caller value.
- After a successful `take` call boundary, the original caller binding is no longer valid unless the value is explicitly replaced through a separate rule.
- `take` must not behave differently just because the callee is an intrinsic or a host-backed wrapper.
- Consuming resource/handle operations such as `close`, `stop`, `drain`, and `stream_close` follow the same `take` law as ordinary values.

## References And Places

- Borrow and dereference remain explicit source operations:
  - `&x`
  - `&mut x`
  - `&x[a..b]`
  - `&mut x[a..b]`
  - `*x`
- Ownership and borrow rules are place-based, not copy-shaped.
- Runtime and backend lowering must preserve place identity across member access, indexing, and call boundaries where the source contract treats the operation as acting on the same place.
- Borrowed-slice creation is explicit adaptation, not implicit coercion.
- This scope does not approve general implicit autoderef or coercion growth.

## Diagnostics

- Move-after-`take` diagnostics are required.
- Conflicting `edit` / borrow / take combinations must be diagnosed deterministically.
- Stale-handle and invalidated-allocator diagnostics must respect the same ownership model rather than inventing host-only exceptions.

## Exclusions

- No erased fallback ownership mode.
- No implicit "host handles behave differently" loophole.
- No special runtime-only rule that treats user routines as copy-shaped while intrinsics are consuming.
