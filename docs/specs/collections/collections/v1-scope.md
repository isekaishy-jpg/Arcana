# Collections, Ranges, And Indexing v1 Scope

Status: `approved-pre-selfhost`

This scope extracts the current frozen collection/range/indexing contract from the rewrite baseline.

## Included Surface

- Builtin/container-facing types:
  - `List[T]`
  - `Array[T]`
  - `Map[K, V]`
  - `RangeInt`
- Pair tuples remain governed by `docs/specs/tuples/tuples/v1-scope.md`.

## Literal Contract

- Non-empty list literals are supported.
- Empty `[]` is part of the current rewrite baseline as an empty list literal.
- Empty `[]` should be understood as list-only surface; it does not imply array or map literal support.
- Empty-list element typing is expected to come from surrounding rewrite typing context; patches must not invent fallback element defaults in the language contract.
- Non-empty map literals are not part of the v1 rewrite baseline.
- Empty maps use explicit constructors such as `std.collections.map.new[K, V] :: :: call`.
- Array literals are not part of the current frozen baseline.

## Narrowing Note

- Older frozen summary text carried broader Meadow-era collection literal wording.
- The current rewrite baseline intentionally keeps bracket literals list-only and constructor-driven for maps.
- If map literals return, they must be reintroduced explicitly with dedicated syntax, typing, and runtime coverage.

## Index And Slice Contract

- Indexing is source-visible contract:
  - `xs[i]`
- Slicing is source-visible contract:
  - `xs[a..b]`
  - `xs[..]`
  - `xs[..=n]`
  - `xs[n..]`
- Indexed assignment and indexed compound assignment remain part of the frozen contract.
- Bounds behavior is deterministic and strict; there is no negative indexing or clamping.

## Range Contract

- `RangeInt` is part of the frozen language surface.
- `RangeInt` participates in slicing and `for` iteration.
- Range syntax is not a post-selfhost reservation; it is part of the active pre-selfhost contract.

## Iteration Contract

- `for x in expr:` is part of the active surface.
- Current required iteration families:
  - `RangeInt`
  - `List[T]`
  - `Array[T]`
  - `Map[K, V]`
- Map iteration yields pair values.

## Std Layering

- Public collection behavior is shelf-first through rewrite-owned `std.collections.*`.
- Kernel/intrinsic collection operations are implementation seams, not the public design center.
