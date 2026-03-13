# Arcana Current Coding Practice Draft

Status: `reference-only` synthesis draft.

This document is a consolidation aid for the rewrite. It is not authoritative by itself. When it conflicts with `POLICY.md`, `docs/specs/spec-status.md`, approved/frozen spec files, or `crates/*`, those sources win.

## Purpose

Arcana currently has important language and coding-practice rules spread across:

- `docs/arcana-v0.md`
- approved domain scopes and status ledgers under `docs/specs/`
- rewrite implementation in `crates/*`
- rewrite-owned `std/`

This draft is meant to gather the current practical coding shape in one place so missing contracts, drift, and overlooked gaps are easier to see before they harden further.

## Authority Order

Use this order when reading this draft:

1. `POLICY.md`
2. `docs/specs/spec-status.md` plus any file it marks frozen or approved
3. `docs/rewrite-roadmap.md` and `PLAN.md` for milestone intent
4. `crates/*` for the current rewrite implementation
5. this draft as a synthesis layer only

## Current Stable Practice

These are the coding patterns that appear stable enough to teach as current rewrite practice.

### Package And Module Shape

- Packages use `book.toml`.
- Source lives under `src/`.
- `import`, `use`, `export`, and `reexport` are normal first-class source surface.
- Multi-package work should follow rewrite-owned package boundaries, not archived Meadow-era topology.

### Core Declaration Shape

- Normal declarations are `fn`, `record`, `enum`, `trait`, `impl`, and `opaque type`.
- Traits and impls may carry associated types.
- Generic type parameters are part of normal coding practice.
- `where` clauses exist in source surface, but see the provisional section for current limits.

### Data Shape

- Core value practice centers on `Int`, `Bool`, `Str`, `Unit`, records, enums, and first-party collections.
- Pair tuples are the current tuple baseline.
- Code should assume pair-only tuple practice unless and until richer tuple rules are re-ratified.

### Calls And Phrases

- Phrase-style calling is the normal calling model.
- Generic calls use explicit type application at the callable site.
- Method-style phrases are part of normal source practice.
- Prefer rewrite-owned `std` and first-party surfaces over ad hoc compiler/runtime special cases.

### Ownership And Mutation

- `read`, `edit`, and `take` are core source-level modes.
- Borrow/deref syntax is part of current practice.
- Handles, streams, windows, audio objects, and other runtime resources should be coded as owned values with explicit consumption points.
- When ownership-sensitive code matters, prefer the patterns already exercised by rewrite-owned `std` over historical Meadow habits.

### Collections

- Use rewrite-owned `std.collections.*` APIs as the default collection interface.
- Prefer explicit constructors for empty collections.
- Treat pair-return collection helpers as normal current practice.
- Do not assume map literal support is available just because `Map[K, V]` exists.

### Match And Control Flow

- `if`, `while`, `for`, `break`, `continue`, and `defer` are part of the working source model.
- `match` over enums and ordinary literal cases is part of normal practice.
- Do not assume tuple-pattern matching is part of safe current practice.

### Runtime-Facing Code

- Host/app capability should flow through rewrite-owned `std` and Arcana-owned grimoires.
- Window/input/canvas/events/time/audio usage should follow rewrite-owned package seams, not historical backend assumptions.
- First-party ECS/behavior surfaces are part of current Arcana practice, not showcase-only helpers.

## Provisional Or Avoid-For-Now Practice

These surfaces exist in some combination of frozen docs, syntax, old Meadow behavior, or partial rewrite implementation, but they are not stable enough to teach as safe everyday coding practice.

- `expr :: :: ?`
- `task_expr :: :: >>`
- chain expressions as if runtime execution is complete
- multithreaded `split` / `weave` semantics as if they are final
- dotted qualifier edge cases that depend on direct callable-path preservation
- projection-equality `where` clauses such as `Iterator[I].Item = U`
- non-empty map literals
- relying on empty `[]` as a settled language rule
- tuple pattern matching
- executable behavior assumptions for attached forewords and page rollups
- final opaque-handle/resource ABI assumptions

## Immediate Spec Work This Draft Points To

The most important domains that still need their own explicit rewrite-owned contract are:

- access modes and ownership (`read` / `edit` / `take`)
- qualified phrases and qualifier resolution
- collections, `RangeInt`, indexing, and slicing
- concurrency, async, task/thread, and behavior scheduling
- opaque handles and runtime resource lifecycle
- `where` clause semantics, especially associated-type projection equality

## Promotion Rule

Nothing in this draft should be treated as language law until it is promoted into:

- `docs/arcana-v0.md`, or
- an approved domain scope/status pair listed in `docs/specs/spec-status.md`

Until then, this file is a working consolidation document for rewrite review and spec extraction.
