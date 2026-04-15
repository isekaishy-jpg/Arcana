# Arcana v0

Arcana v0 is a small, typed, indentation-based language with explicit access modes (`read`, `edit`, `take`, `hold`).

Scope note:
- This document summarizes the frozen source-language, grimoire/module shape, and first-party standard surface carried toward selfhost.
- It is not the authority for backend implementation strategy, bootstrap/oracle history, artifact internals, or historical runtime modes.
- Rewrite authority lives in `POLICY.md`, `docs/specs/spec-status.md`, `PLAN.md`, and `docs/rewrite-roadmap.md`.

Repository policy note:
- `POLICY.md` defines the current direction: no new public builtins, migrate toward Arcana-authored stdlib/shelves, and reject pseudo-builtin wrappers.

Arcana v0 currently includes:

- `fn`, `let`, `mut`, `if`, `while`, `return`
- `Int`, `Bool`, `Str`, `Unit`
- Access-mode parameters and compile-time checks for `read`/`edit`/`take`
- `record` declarations, construction, field access, and field assignment
- `obj` declarations plus `create ... scope-exit` owner domains with explicit activation and hold/re-entry
- Multi-file Grimoires (`book.toml`, `import`, `export`, `reexport`)
- Arcana-native concurrency/behavior support (`async fn`, `weave`, `split`, `>> await`, `behavior[...] fn`)
- Core operators: unary `-` / `not` / `~`, `%`, `!=`, `<=`, `>=`, `and`, `or`, bitwise `& | ^ << shr`, `Str + Str`, compound assignments
- Collections v0.9: `List[T]`, list literals, slicing with ranges, indexed assignment, `RangeInt`, pair tuples `(A, B)`
- Memory phrases v0.35+: `arena|frame|pool|temp|session|ring|slab: instance :> ... <: qualifier` with typed allocator storage plus explicit views/publication state
- Headed regions v0.42: `recycle`, `construct`, `bind`, and `Memory` as structural inner blocks with head-defined rides, default modifiers, and head-specific completion/target slots
- Ownership/lifetimes v0.32 surface: explicit capability types (`&read[T, 'a]`, `&edit[T, 'a]`, `&take[T, 'a]`, `&hold[T, 'a]`), capability/deref expressions (`&read x`, `&edit x`, `&take x`, `&hold x`, `*x`), explicit borrowed-slice forms (`&read x[a..b]`, `&edit x[a..b]`), `reclaim x` for held-capability release, a carried lexical capability contract, and `#boundary[target="lua|sql"]` signature contracts
- Trait v2: associated types, default trait methods, supertrait bounds, projection equality in `where`

## Commands

Current shipped rewrite commands:

- `arcana check`
- `arcana build --plan`
- `arcana build`

Current rewrite public interfaces stop there until the first runnable AOT backend lands.
Carried `run`/`compile`/artifact-execution expectations remain future-facing backend work, not current rewrite CLI contract.

Planned before selfhost, but not shipped yet:

- `arcana test`
- `arcana format`
- `arcana review` as a smaller advisory layer after the rewrite has enough real showcase-scale Arcana code to justify Arcana-native usage guidance

Tooling roles are intentionally distinct:

- `arcana check` is the correctness gate for parser/frontend/type/ownership/borrow/lifetime/boundary validation and other compiler-owned diagnostics
- `arcana test` is the first-party test discovery/execution surface
- `arcana format` is the first-party deterministic formatter
- `arcana review` is not a second typechecker; it is the later advisory layer and should stay small pre-selfhost

Single-file checking is intentionally minimal: it does not resolve `import`/`use`/`reexport`.
Use grimoire/workspace mode (`arcana check <grimoire-dir>`, `arcana build <workspace-dir>`) for std/module-based programs.
Source files are hard-standardized to `.arc` in file-mode commands and module loading.
Migration helper: `powershell -ExecutionPolicy Bypass -File scripts/migrate_source_extensions_to_arc.ps1` (add `-Apply` to execute).

Current rewrite builds operate at workspace scope; cache invalidation and rebuild decisions are
driven by member fingerprints inside `arcana build <workspace-dir>`.

Detailed backend, selfhost, and bootstrap workflow policy is tracked outside this document.

## Objects And Owners (v0.41)

Arcana now includes an explicit object/owner lifetime model.

- `obj Name:` declares nominal packaged state with optional nested methods.
- `create Owner [ObjectA, ObjectB] scope-exit:` declares a managed owner lifetime domain.
- Owners may optionally declare `context: Ctx` before `scope-exit:` to make activation payload shape explicit.
- Owner exits use `exit when ...` or `name: when ...`, optionally with `retain [...]`.
- Bare path lines immediately above block-owning headers attach owner/object availability.
- Availability does not create live state by itself.
- Explicit owner activation uses qualified phrases:
  - `let active = Session :: ctx :: call`
  - `Session :: ctx :: call`
- Direct attached object names are usable only while the owner is active on that execution path.
- That active-owner state carries through ordinary routine calls and newly entered attached blocks on the same execution path; attached helpers do not require re-entry when the caller already has the owner active.
- Re-attaching the object name is enough for direct object access on that path; helpers and nested blocks do not need to re-attach the owner when the same owner is already active.
- Owned objects may define lifecycle hooks with nested `init` / `resume` methods; first realization runs `init`, and held-state re-entry runs `resume`.
- Owners without `context:` accept zero activation args.
- Owners with `context:` require exactly one activation arg of that type.
- Activation context is only meaningful through those lifecycle hooks and must match the owner-declared context type for that domain.
- Suspension is modeled as owner exit plus `retain [...]`; retained state requires explicit re-entry before it becomes active again.
- When multiple owner exit conditions are true at the same checkpoint, the first matching exit in source order wins.
- Callable objects and context objects are ordinary `obj` roles inside this same model.
- Dispatch remains static; closures, lambdas, and general function values remain outside the selfhost baseline.

## Grimoires (v0.3)

Arcana's multi-file package/library unit is a `Grimoire`.

App Grimoire:

- `book.toml`
- `src/shelf.arc` (entry root, must define `main`)
- `src/types.arc` (required structural types module)

Library Grimoire:

- `book.toml`
- `src/book.arc` (public root, no `main` required)
- `src/types.arc` (required structural types module)

Minimal manifest:

```toml
name = "my_grimoire"
kind = "app" # or "lib"
```

Workspace root additions (local path deps only):

```toml
[workspace]
members = ["core", "app"]
```

Member deps:

```toml
[deps]
core = { path = "../core" }
```

### Module Syntax

- `import foo.bar` loads `src/foo/bar.arc` and binds the module namespace
- `use foo.bar.Baz` or `use foo.bar as bar` brings selected names into local scope
- `export fn ...` / `export record ...` marks declarations public
- `reexport foo.bar` re-exports all exported names from another module

Notes:

- Imports are namespace-first; unqualified use requires `use`.
- `run` / `compile` only support `kind = "app"` Grimoires (`check` supports both)
- all grimoires must include a parseable `src/types.arc` even if the file is currently minimal
- first-party library grimoires conventionally expose `types` from root with `reexport types`

## Standard Type Shelves (v0.37)

Arcana now ships reusable standard type modules:

- `std.types.core`
- `std.types` (reexports `core`)

Current `std.types.core` includes:

- `Vec2i`, `Size2i`, `Recti`, `ColorRgb`, `Tick`, `FrameIndex`
- helpers: `vec2`, `size2`, `rect`, `rgb`

Arcana also ships soft behavior phase trait standards:

- `std.behavior_traits.StartupPhase`
- `std.behavior_traits.FixedPhase`
- `std.behavior_traits.UpdatePhase`
- `std.behavior_traits.RenderPhase`
- helpers: `run_startup`, `run_fixed`, `run_update`, `run_render`

### Anti-Monolith Warnings (Soft Lints)

Warnings are non-fatal and printed to stderr.

- Root file warning if `src/shelf.arc` / `src/book.arc` exceeds `200` lines
- Root file warning if root has more than `12` top-level `fn`/`record` declarations
- Any-file warning if any loaded `.arc` file exceeds `1000` LOC

## Access Modes (v0)

- `read` is the default parameter mode
- `edit` gives exclusive mutable access during the call (for example: `bump :: x :: call`)
- `take` moves non-`Copy` values (`Str`, records)

## Cleanup Footers (v0.38)

- Arcana supports attached cleanup footers after an owning block dedents:
  - `-cleanup`
  - `-cleanup[target = name]`
  - `-cleanup[target = name, handler = path]`
- Bare `-cleanup` covers cleanup-capable owning bindings activated in that owner scope.
- Local `defer` still runs before the owner's cleanup footer work.
- Old `[subject, handler]#cleanup` syntax is no longer part of the accepted language.
- Headed regions are separate inner structural blocks, not attached footer forms.

## Generic Phrase Family (v0.24)

Arcana’s phrase family has three forms:

| Phrase | Form | Scope |
|---|---|---|
| Qualified phrase | `subject :: args? :: qualifier` | expression + statement |
| Memory phrase | `memory_type: instance :> args? <: qualifier` | expression + statement |
| Chain phrase | `style :=(>|<) stage...` with `=>`/`<=` connectors | expression + statement |

Header phrases are statement-form qualified/memory phrases. A header phrase may own an attached block. In attached blocks:

- `name = expr` remains valid header metadata/named-arg overflow
- chain phrase lines are valid and execute in source order
- forewords may annotate attached entries as header-local metadata, similar in spirit to Rust attributes
- each attached chain receives the prior result implicitly

Evaluation order for attached header blocks:

1. Apply header `name = expr` entries as named args.
2. Execute header phrase.
3. Execute first attached chain from header result.
4. Pipe through remaining attached chains in source order.

## Headed Regions (v0.42)

Arcana now includes headed regions as a separate structural family from phrases and cleanup footers.

- A headed region is a structural inner block whose head defines the ride over top-level participating lines.
- Headed regions are not attached forms.
- The approved heads are:
  - `recycle`
  - `construct`
  - `bind`
  - `Memory`
- Each headed region declares one default modifier.
- Some heads also declare a required completion/target slot.
- Participating lines must fit the selected head’s ride; headed regions are not freeform imperative blocks.

Implementation note:

- Headed regions are approved source-language contract now.
- Parser/frontend/runtime support for the approved head family is landed on the current rewrite path.
- Current selfhost/conformance coverage tracks the approved head family explicitly.

## Qualified Phrases (v0.22)

Arcana now supports qualified phrase invocation syntax:

- `subject :: args :: qualifier`
- args are comma-separated top-level inline items, with at most 3 top-level args
- trailing comma before the qualifier is rejected
- qualifier forms:
  - `call`
  - bare method / named path
  - symbols: `?`, `>`, `>>`
  - `await`
  - `weave`
  - `split`
  - `must`
  - `fallback`
- attached blocks are valid only on standalone statement-form qualified phrases
- `call`, bare method, and named-path qualifiers may carry explicit qualifier type args such as `:: call[T]`

Core behavior:

- `f :: a, b :: call` calls `f(a, b)`
- `obj :: x :: method_name` dispatches method `method_name` on `obj`
- `value :: :: arcana_process.io.print` dispatches `arcana_process.io.print(value)`
- `result_expr :: :: ?` applies try-propagation
- `task_expr :: :: >>` awaits task value
- `task_expr :: :: await` awaits task/thread value
- `callable :: args :: weave` spawns a task-qualified phrase call
- `callable :: args :: split` spawns a thread-qualified phrase call
- `value_expr :: :: must` hard-unwraps `Option[T]` or `Result[T, Str]`
- `value_expr :: fallback_value :: fallback` supplies a fallback for `Option[T]` or `Result[T, Str]`

Statement-form phrases can carry attached blocks:

```arc
Counter :: :: call
    value = 1
```

For `call`, `>`, and bare method qualifiers, attached `name = expr` entries are treated as
additional named arguments.

For dotted path qualifiers, `?`, `>>`, `await`, `weave`, `split`, `must`, and `fallback`, attached `name = expr` entries are rejected.

If a call needs more than 3 independent top-level inline inputs, group them explicitly into
pair/record data or move named overflow into a statement-form attached block. The 3-arg cap is
intentional.

Classic user call syntax (`f(...)`, `obj.method(...)`) is removed. Use qualified phrases.

## Operators (v0.5)

Arcana now supports a pragmatic core operator set:

- Unary: `-`, `not`, `~` (bitwise NOT on `Int`)
- Arithmetic: `+`, `-`, `*`, `/`, `%` (`Int` only), plus `Str + Str`
- Bitwise/shift: `&`, `|`, `^`, `<<`, `shr` (`Int` only)
- Comparison: `<`, `<=`, `>`, `>=` (`Int` only)
- Equality: `==`, `!=` (same-type operands)
- Logical: `and`, `or` (Bool-only, short-circuiting)
- Compound assignment: `+= -= *= /= %= &= |= ^= <<= shr=`

Notes:

- `and`/`or` are keyword operators and short-circuit.
- `not` is the logical negation operator (`!` is not used).
- `shr` is the arithmetic right-shift keyword (reserved; cannot be used as an identifier).
- `+` supports `Int + Int` and `Str + Str` only (no coercions).
- `>>` remains reserved for the await pipe (`task >> await`), not a shift operator.
- Integer overflow in `+ - * / %` and unary `-` is reported deterministically at runtime:
  `integer overflow in add|sub|mul|division|modulo|neg`.

Historical note: the archived MeadowLang operator examples now live outside this repo. Current in-repo behavioral pressure comes from rewrite-owned `std/src`, `grimoires/arcana/*`, conformance fixtures, and crate tests.

## Collections (v0.9)

Arcana now includes a first collection surface centered on `List[T]`.

- Builtin types: `List[T]`, `RangeInt`, pair tuples `(A, B)`
- List literals: `[1, 2, 3]` (non-empty only; use `std.collections.list.new[T]()` for empty)
- Indexing/slicing:
  - `xs[i]`
  - `xs[1..4]`, `xs[..]`, `xs[..=n]`, `xs[n..]`
- Indexed assignment / compounds:
  - `xs[i] = v`
  - `xs[i] += 1`
  - `xs[i] shr= 1`
- Shelf-first list API:
  - `std.collections.list.new[T]()`
  - `xs :: :: len`
  - `xs :: v :: push`
  - `xs :: :: pop`
  - `xs :: fallback :: try_pop_or` -> `(Bool, T)`
- Direct legacy collection builtins are rejected by default in CLI/toolchain.
- Legacy collection call names are removed; use shelf-first APIs.

Notes:

- `List[T]` is owned and non-Copy.
- Slicing returns a copied `List[T]`.
- Bounds are strict runtime errors (no clamping / no negative indexing).
- `RangeInt` values support equality and are primarily used for slicing.
- Generic calls use phrase style (`foo[T] :: ... :: call`), while `foo[T]` without a qualifier remains subscript syntax.
- Pair tuples are the current selfhost baseline. Richer tuple expansion is intentionally deferred rather than rejected outright; see `docs/specs/tuples/tuples/v1-scope.md` and `docs/specs/tuples/tuples/deferred-roadmap.md`.
- Exact recursive pair destructuring in `let` and `for` is part of the current pair-tuple baseline; parameter destructuring and tuple `match` patterns remain deferred.

Historical note: the archived MeadowLang collection examples now live outside this repo. Current in-repo behavioral pressure comes from rewrite-owned `std/src`, `grimoires/arcana/*`, conformance fixtures, and crate tests.

## Collections Expansion (v0.10)

Arcana now extends collections with `Array[T]`, `Map[K, V]`, and `for` loops.

- Builtin types:
  - `Array[T]` (fixed runtime length)
  - `Map[K, V]` (keys restricted to `Int` or `Str`)
- `for x in expr:` iteration over:
  - `RangeInt`
  - `List[T]`
  - `Array[T]`
  - `Map[K, V]` (yields `(K, V)` pairs)
- `break` / `continue` in `while` and `for`
- Map literals (non-empty only): `{key: value, ...}`
- Shelf-first array API:
  - `std.collections.array.new[T](len, fill)`
  - `std.collections.array.from_list[T](xs)`
  - `arr :: :: len`
  - `arr :: :: to_list`
- Shelf-first map API:
  - `std.collections.map.new[K, V]()`
  - `m :: :: len`
  - `m :: key :: has`
  - `m :: key :: get`
  - `m :: key, value :: set`
  - `m :: key, fallback :: try_get_or`
- Direct legacy collection builtins are rejected by default in CLI/toolchain.
- Legacy collection call names are removed; use shelf-first APIs.

Notes:

- Collection `for` iteration uses snapshot semantics (original collection remains usable).
- `m[key]` is strict and errors at runtime when the key is missing.
- Empty map literals are not supported yet; use `std.collections.map.new[K, V]()`.
- Array literals are not part of v0.10 (use constructors).

Historical note: archived MeadowLang array/map/`for` examples now live outside this repo. Current in-repo behavioral pressure comes from rewrite-owned `std/src`, `grimoires/arcana/*`, conformance fixtures, and crate tests.

## Library-Ready Language Additions (v0.13, current subset)

Implemented in the compiler/runtime:

- `enum` with payload variants
- `match` expressions over enums/tuples/literals
- `or` patterns in `match` (currently without binding capture in a single arm)
- `impl` blocks with method calls via phrases (`value :: ... :: method`)
- Trait v2 additions:
  - associated types in traits/trait impls
  - default trait method bodies (impls may omit defaulted methods)
  - supertrait bounds via trait `where`
  - projection equality bounds (`where Iterator[I], Iterator[I].Item = U`)
- qualified phrase invocation (`subject :: args :: qualifier`)
- `use` aliasing and grimoire namespace flattening support
- `defer` with scope-exit LIFO execution on normal exits
- postfix `?` for `Result`-shaped enums (`Result` leaf name, including generic-specialized internal names)
- generic monomorphization support in the current compiler pipeline for:
  - `fn[T]` with explicit call-site type args (`foo[Int] :: ... :: call`)
  - `record[T]` with explicit constructor type args (`Box[Int] :: ... :: call`)
  - `enum[T, E]` with explicit variant constructors (`Result.Ok[Int, Str] :: ... :: call`)
  - `impl[T]` specialization against concrete receiver/value types used by the program

Notes:

- Generic constructors currently require explicit type args.
- Pattern matching for generic enums currently rewrites by instantiated enum path and is deterministic when a single concrete instantiation is in scope.
- Or-pattern binding capture (`A(x) | B(x)`) is not supported yet.
- Trait dispatch remains static/monomorphized (no trait objects/dynamic dispatch).

Historical note: the archived MeadowLang generic/trait proof examples now live outside this repo. Current in-repo behavioral pressure comes from rewrite-owned `std/src`, `grimoires/arcana/*`, conformance fixtures, and crate tests.

## Iterator + ECS Trait Proof Slice (v0.27)

Std now includes a trait-based iterator foundation:

- `std.iter.Iterator[I]` with associated type `Item`
- default-friendly trait impl model in language core
- helpers like `std.iter.count`

`std.ecs` now provides an iterator adapter:

- `std.ecs.SingletonCursor[T]`
- `impl std.iter.Iterator[SingletonCursor[T]] for SingletonCursor[T]`

This demonstrates associated-type-based abstractions across std modules without adding new builtins.
It does not freeze general ECS query syntax; broad query authoring remains outside the frozen selfhost baseline.

## Std-Style Shelf Foundation

A std-style shelf layout now works in Grimoires using `impl` extension methods over kernel collection intrinsics.

Current in-repo reference corpus for this direction lives primarily under `std/src` and `grimoires/arcana/*`; the broader MeadowLang examples are now archived outside the repo. Key retained surfaces include:

- `std.result` / `std.option` user enums
- `std.collections.list` extension methods (`len`, `push`, `pop`, `try_pop_or`)
- `std.collections.map` extension methods (`len`, `has`, `get`, `set`, `try_get_or`)
- `std.collections.array` extension methods (`len`, `to_list`)

This provides the intended direction where app code uses shelf methods instead of direct builtin calls.

## Collection De-Builtinization (v0.15)

Collections are now de-builtinized through an internal intrinsic bridge.

- Public usage is shelf-first via `std.collections.*`.
- Collection semantics no longer rely on collection callee-string dispatch branches.
- Direct calls to legacy collection builtins are hard errors by default.
- Legacy collection call names are removed; use shelf-first APIs.
- `intrinsic fn` is internal: restricted to trusted `std.kernel.*` modules.
- `native fn` is the package-scoped host-binding surface for binding-owning libraries such as `arcana_winapi`.
- `native callback` is the package-scoped explicit callback registration surface paired with `native fn`.
- `std.kernel.*` is internal-only and cannot be imported/reexported by user/app modules.

## Memory Phrase + Typed Allocators (v0.35)

Arcana supports explicit memory-context allocation phrases with seven allocator families plus explicit view/publication support.

Memory source surface now has two distinct roles:

- `Memory` headed regions define reusable memory specs
- memory phrases are the consumer surface for those specs

The current approved `Memory` headed-region family set is:

- `arena`
- `frame`
- `pool`
- `temp`
- `session`
- `ring`
- `slab`

Memory phrase syntax:

- `memory_type: instance :> args? <: qualifier`
- v2 supports `memory_type = arena | frame | pool | temp | session | ring | slab`
- inline args are comma-separated top-level items, with at most 3 top-level args
- arg items support positional and named (`name = expr`)
- trailing comma before the qualifier is rejected
- constructor/qualifier must be a path or `path[type_args]`
- attached blocks are valid only on standalone statement-form memory phrases

Allocator phrase lowering:

- `arena: ast :> lhs = 1, rhs = 2, op = 3 <: Node`
- compiles as:
  - build `Node` from qualifier call
  - allocate into `ast: Arena[Node]`
  - return `ArenaId[Node]`

- `frame: scratch :> value = 2 <: Temp`
  - allocates into `scratch: FrameArena[Temp]`
  - returns `FrameId[Temp]`

- `pool: entities :> hp = 100 <: Entity`
  - allocates into `entities: PoolArena[Entity]`
  - returns `PoolId[Entity]`

- `temp: scratch :> value = 7 <: Temp`
  - allocates into `scratch: TempArena[Temp]`
  - returns `TempId[Temp]`

- `session: interned :> value = 9 <: Node`
  - allocates into `interned: SessionArena[Node]`
  - returns `SessionId[Node]`

- `ring: recent :> value = 11 <: Sample`
  - pushes into `recent: RingBuffer[Sample]`
  - returns `RingId[Sample]`

- `slab: graph :> value = 13 <: Node`
  - allocates into `graph: Slab[Node]`
  - returns `SlabId[Node]`

New shelf module:

- `std.memory`

Public surface:

- `std.memory.new[T](capacity) -> Arena[T]`
- `std.memory.frame_new[T](capacity) -> FrameArena[T]`
- `std.memory.pool_new[T](capacity) -> PoolArena[T]`
- `std.memory.temp_new[T](capacity) -> TempArena[T]`
- `std.memory.session_new[T](capacity) -> SessionArena[T]`
- `std.memory.ring_new[T](capacity) -> RingBuffer[T]`
- `std.memory.slab_new[T](capacity) -> Slab[T]`
- `arena :: :: len`
- `arena :: id :: has`
- `arena :: id :: get`
- `arena :: id, value :: set`
- `arena :: id :: remove`
- `arena :: :: reset`
- `frame_arena :: :: len`
- `frame_arena :: id :: has`
- `frame_arena :: id :: get`
- `frame_arena :: id, value :: set`
- `frame_arena :: :: reset`
- `pool_arena :: :: len`
- `pool_arena :: id :: has`
- `pool_arena :: id :: get`
- `pool_arena :: id, value :: set`
- `pool_arena :: id :: remove`
- `pool_arena :: :: reset`
- `pool_arena :: :: live_ids`
- `pool_arena :: :: compact`
- `temp_arena :: :: len`
- `temp_arena :: id :: has`
- `temp_arena :: id :: get`
- `temp_arena :: id, value :: set`
- `temp_arena :: :: reset`
- `session_arena :: :: len`
- `session_arena :: id :: has`
- `session_arena :: id :: get`
- `session_arena :: id, value :: set`
- `session_arena :: :: reset`
- `session_arena :: :: seal`
- `session_arena :: :: unseal`
- `session_arena :: :: is_sealed`
- `session_arena :: :: live_ids`
- `ring_buffer :: value :: push`
- `ring_buffer :: :: pop`
- `ring_buffer :: :: len`
- `ring_buffer :: id :: has`
- `ring_buffer :: id :: get`
- `ring_buffer :: id, value :: set`
- `ring_buffer :: start, len :: window_read`
- `ring_buffer :: start, len :: window_edit`
- `slab :: :: len`
- `slab :: id :: has`
- `slab :: id :: get`
- `slab :: id, value :: set`
- `slab :: id :: remove`
- `slab :: :: reset`
- `slab :: :: seal`
- `slab :: :: unseal`
- `slab :: :: is_sealed`
- `slab :: :: live_ids`
- explicit view surface:
  - `View[Elem, Family]`
  - first-wave families:
    - `Contiguous`
    - `Strided`
    - `Mapped`
- explicit borrowed slices:
  - `&read x[a..b]`
  - `&edit x[a..b]`
- `std.binary` provides explicit reader/writer helpers over `View[...]` and `Bytes`

Runtime semantics:

- `Arena[T]` and `FrameArena[T]` are non-Copy and non-sendable.
- `PoolArena[T]` is sendable/shareable when `T` is sendable.
- `ArenaId[T]` and `FrameId[T]` are copy-like and non-sendable.
- `PoolId[T]` is copy-like and sendable when `T` is sendable.
- `frame` is append-only with explicit `reset` and generation invalidation.
- `pool` supports `remove` + free-list reuse with generation checks plus explicit compaction.
- `temp` is scratch append-only storage with `reset_on` policy.
- `session` is long-lived append-only storage with publication through `seal` / `unseal`.
- `ring` is overwrite-oldest circular storage with oldest-to-newest window order.
- `slab` is stable-slot reusable storage with publication through `seal` / `unseal`.
- `reset` is explicit only (no auto-reset semantics).
- stale/invalid id access raises deterministic runtime errors:
  - `arena id is invalid or stale`
  - `frame id is invalid or stale`
  - `pool id is invalid or stale`
  - `temp id is invalid or stale`
  - `session id is invalid or stale`
  - `ring id is invalid or stale`
  - `slab id is invalid or stale`

`Memory` headed-region shape:

```arcana
Memory <memory_type>:<name> -<memory_modifier>
    <memory_detail_line>
    <memory_detail_line> -<override_modifier>
```

Core `Memory` meaning:

- `<memory_type>:<name>` establishes a reusable memory spec in the memory-spec namespace
- `Memory` defines budgeting, lifetime shape, and pressure behavior
- `Memory` does not perform allocation itself
- memory phrases and other approved memory-aware forms are the consumers of those specs
- `Memory` specs may be defined in modules, including `std`

Notes:

- Attached blocks on memory phrase statements follow header-phrase rules: `name = expr` entries and chain lines are both valid, and attached entries may carry forewords as header-local metadata.
- Unknown memory types are rejected with a future-reserved diagnostic.
- If a memory phrase needs more than 3 independent top-level inline inputs, group them explicitly into pair/record data. The 3-arg cap is intentional.
- `std.memory` exposes allocator borrow APIs:
  - `borrow_read` / `borrow_edit` for `Arena`, `FrameArena`, `PoolArena`, `TempArena`, `SessionArena`, `RingBuffer`, and `Slab`.
- `std.memory` also exposes the explicit `View[Elem, Family]` surface plus publication-state operations:
  - `View[Elem, Contiguous]`, `View[Elem, Strided]`, `View[U8, Mapped]`
  - `seal` / `unseal` / `is_sealed` for `SessionArena` and `Slab`
- `reset`/`remove` are compile-time rejected when live allocator borrows would be invalidated.

Historical note: the broader MeadowLang memory examples now live outside this repo. Current in-repo behavioral pressure comes from rewrite-owned `std/src`, conformance fixtures, and crate tests.

## Ownership And Lifetimes (v0.32)

Arcana now enforces lexical ownership and capability rules with explicit lifetimes.

Ownership rationale:

- Arcana follows Rust where Rust's mutability, borrowing, and ownership rules are explicit and unambiguous.
- Arcana does not copy Rust wholesale; the carried Arcana surface and any explicitly ratified Arcana-specific needs still control the final rule shape.
- Any ownership or borrowing behavior that would otherwise be implicit or ambiguous must be made explicit in Arcana syntax, static rules, and diagnostics rather than left as convention or hidden inference.
- Milestone work should therefore prefer explicit place-based reasoning, explicit conflict checks, and explicit lifetime ties over convenience-heavy or erased behavior.

Core syntax:

- lifetime params: `'a`, `'b`
- capability types: `&read[T, 'a]`, `&edit[T, 'a]`, `&take[T, 'a]`, `&hold[T, 'a]`
- capability/deref expressions: `&read x`, `&edit x`, `&take x`, `&hold x`, `&read x[a..b]`, `&edit x[a..b]`, `*x`
- hold release: `reclaim x`
- where outlives predicates: `'a: 'b`, `T: 'a`

Core rules:

- many shared borrows or one mutable borrow per place
- no move while borrowed
- no mutable access through non-borrow paths while borrowed
- any lifetime used in a signature must be declared in that declaration's lifetime parameter list
- references are non-sendable
- returned references must be tied to valid input lifetimes

Interop boundary contracts (compile-time only):

- `#boundary[target = "lua"]`
- `#boundary[target = "sql"]`
- boundary fns/methods cannot return references and cannot accept mutable references
- the carried contract is varietal interop, not embedding
- optional hot-path/reload workflows remain part of the intended boundary direction, but the rewrite has not implemented that host/backend layer yet

## Chain Phrase (v0.24)

Arcana chain phrases support directional staged flow:

- style qualifier: selects chain execution semantics such as `forward`, `lazy`, `parallel`, `async`, `plan`, `broadcast`, `collect`
- forward introducer: `style :=>` for forward and mixed chains
- composition introducer: `style :=<` for composition/reverse chains
- connectors: `=>` (forward edge), `<=` (reverse/composition edge)
- supported styles:
  - directional-enabled: `forward`, `lazy`, `async`, `plan`, `collect`
  - forward-only fan-out: `parallel`, `broadcast`
- stages are callable paths (optionally with type args)
- chains are valid in both statement and expression position

Surface model:

- a chain is three things: `style`, introducer, and connector-directed stage edges
- the style is not just decoration; it selects execution semantics and capability rules
- the introducer selects the family of chain being written: forward/mixed or composition
- the connectors still matter inside the chain because they describe per-edge flow

Standalone chains:

```arc
forward :=> load_config => parse_config => build_state
forward :=< build_state <= parse_config <= load_config
forward :=> load_config => parse_config <= fallback_transform <= fallback_seed
```

Expression chains:

```arc
let score = forward :=> seed => clamp with (0, 100) => emit
let score2 = sink :: (forward :=> seed => add with (2)) :: call
```

Header-attached chains:

```arc
query :: Position, Velocity :: read
    include_sleeping = false
    forward :=> validate => execute => present
    collect :=> metric_a => metric_b => metric_c
```

Attachment behavior:

- header attached blocks may mix `name = expr` entries and chain lines
- named entries are applied to the header phrase first
- header phrase executes
- attached chains run in source order
- each attached chain consumes the previous result implicitly

Style notes:

| Style | v1 semantics | Output |
|---|---|---|
| `forward` | unary pipeline in source order | final stage value |
| `lazy` | demand-sensitive left-to-right pipeline style; runtime lowering may skip unnecessary downstream work when needed, but only when that does not change required observable behavior | final stage value |
| `async` | unary pipeline with auto-await for `Task[T]` stages | unwrapped final value |
| `parallel` | fan-out with deterministic ordered collection; lowering may use threads or async-task fanout when qualifier/caller/stage capabilities permit, with deterministic fallback otherwise | `List[T]` |
| `broadcast` | sequential fan-out over same input | `List[T]` |
| `collect` | directional pipeline collecting intermediates in normalized order | `List[T]` |
| `plan` | validate/typecheck the pipeline/chain contract only; no stage execution, and expression-position use yields the original input unchanged | pass-through input |

Directional topology rules:

- pure forward: `style :=> a => b => c`
- pure reverse: `style :=< c <= b <= a`
- mixed single pivot: `style :=> a => b => c <= d <= e`
- mixed executes as segment reorder: left forward segment then reversed right segment (`a, b, c, e, d`)
- only one direction change is allowed
- reverse-introduced chains are pure reverse only in this version

Reading/composition model:

- chain source is always written and read left-to-right
- connectors determine the normalized execution order within the chosen introducer family
- conventional function-composition reasoning is still right-to-left over the normalized stage list; the source pipe itself is not

Standalone seed rule:

- pure forward/mixed: first executable stage must be zero-arg seed
- pure reverse: rightmost textual stage must be zero-arg seed
- remaining stages are unary transforms from prior output

Bound-stage adapters:

- syntax: `stage with (arg1, arg2, ...)`
- adapter call shape: chain input is argument 1, bound args are appended
- stage signature must match `(In, B1, B2, ...) -> Out`

Contracts:

- `#stage[...]` on `fn`/trait methods/impl methods
- `#chain[...]` on chain statements
- the chain contract matrix defines resolved chain-contract fields for behavior/system scheduling
- any chain inside `behavior[...]` or `system[...]` must declare explicit `#chain[...]`

Supported `#stage` keys:

- `pure`, `deterministic`, `effect`, `thread`, `authority`, `rollback_safe`
- `reads`, `writes`, `excludes` (repeatable)

Supported `#chain` keys:

- `phase`, `deterministic`, `thread`, `authority`, `rollback_safe`

Compatibility note:

- legacy `reverse :=> ...` style is removed
- use an existing directional style with reverse introducer and reverse connectors (`<style> :=< ... <= ...`)

See also:

- `docs/specs/chain/contract_matrix_v1.md`

Metadata blocks:

- standalone chain attachment blocks are currently validated as `name = expr` only
- metadata payloads are parsed/validated in v1; full contract aggregation and scheduler enforcement beyond explicit chain-presence checks are later typed/runtime work

Historical note: the broader MeadowLang chain examples now live outside this repo. Current in-repo behavioral pressure comes from rewrite-owned `std/src`, conformance fixtures, and crate tests.

## Current Implementation Limits (Not Frozen Language Law)

These are current checker/runtime limits from the in-progress rewrite. They are not, by themselves, endorsed long-term language design and must not be treated as selfhost contract unless promoted explicitly.
Where a domain scope exists under `docs/specs/**/v1-scope.md`, that domain scope wins over these notes.

- `edit` call arguments currently must be local bindings (not field expressions)
- Access checking is currently root-binding based (conservative)
- Moves inside `while` loops are currently rejected
- boundary checks now enforce payload/target rules, direct signature-shape safety, and recursive record/enum boundary-safe typing after HIR resolution; later work still needs deeper backend/runtime boundary flow
- the current frontend now covers declaration-surface lifetimes plus conservative body-level expression typing, ownership, borrow, move, and return-lifetime diagnostics on the rewrite path

## Opaque Types (v0.17)

Arcana now has source-level opaque type declarations for trusted std-owned runtime/resource handles and package-owned binding handles:

- syntax: `export opaque type Window as move, boundary_unsafe`
- required inline policy atoms:
  - one ownership atom: `copy` or `move`
  - one boundary atom: `boundary_safe` or `boundary_unsafe`
- opaque types are type-like for signatures, impl targets, trait impl targets, and API fingerprints
- opaque types are non-constructible in expressions:
  - no phrase construction
  - no implied fields or payload access
  - no record/enum behavior by default
- v1 restriction:
  - opaque type declarations are currently allowed in package `std` and in packages that own an approved `binding` native product

Current binding-owned opaque handle families such as `Window`, `FileStream`, `AudioDevice`, `AudioBuffer`, `AudioPlayback`, and `FrameInput` now live as source-declared opaque types rather than Rust-only reserved builtin names.
Binding-owning libraries such as `arcana_winapi` may also expose source-declared opaque handle types such as module handles, font catalogs, and hidden windows.

## Native Bindings (v0.18)

Arcana now distinguishes two native-facing declaration families:

- `intrinsic fn`
  - trusted std/kernel-only surface
- `native fn`
  - package-scoped host-binding import surface for binding-owning libraries
- `native callback`
  - package-scoped explicit callback registration surface paired with `native fn`

Binding examples:

- `export native fn current_module() -> arcana_winapi.types.ModuleHandle = foundation.current_module`
- `native callback window_proc(read window: arcana_winapi.types.HiddenWindow, message: Int, wparam: Int, lparam: Int) -> Int = arcana_winapi.callbacks.handle_window_proc`

Rules:

- `native fn` / `native callback` belong to packages that own an approved `binding` native product.
- They are not a replacement for `intrinsic fn`.
- They are the generic library/native seam for OS binding grimoires such as `arcana_winapi`.

## Legacy Desktop Note

The old v0 desktop shell description has been retired from this document.

Current rewrite authority for the desktop/runtime shell is:

- any future desktop/runtime shell layer must be re-approved outside this archival document
- the approved scopes and ledgers under `docs/specs/`

Historical v0 details for the legacy std desktop shell remain archival context only and should not be read as current rewrite surface.

- `step_startup()`
- `step_fixed_update()`
- `step_update()`
- `step_render()`
- `step_phase(phase)`
- singleton component helpers:
  - `set_component[T](value)`
  - `has_component[T]()`
  - `get_component[T]()`
- entity/component helpers:
  - `spawn() -> Int`
  - `despawn(entity)`
  - `set_component_at[T](entity, value)`
  - `has_component_at[T](entity) -> Bool`
  - `get_component_at[T](entity) -> T`
  - `remove_component_at[T](entity)`
  - `remove_component[T]()` (singleton helper equivalent to entity `0`)

Runtime note:

- First-class ECS scheduling/components are part of the carried v0 surface through `behavior[...] fn`, `system[...] fn`, and `std.ecs`.
- General ECS query authoring is not yet part of the frozen selfhost baseline.
- `affinity=worker` component systems run via worker execution when component arguments are transferable; non-transferable component values fall back to deterministic main-thread execution.
- Worker-applied component writes are fail-fast checked; if a targeted component changed before apply, runtime reports an ECS worker apply conflict.
- Worker-affinity systems require sendable component parameter types.
- A single system cannot declare multiple `edit` parameters for the same component type.

Rewrite note:

- carried historical `std.tooling` helpers are not part of the current std surface and remain reference-only unless a future scope explicitly reintroduces them elsewhere.

Workspace/build notes:

- `arcana build <workspace-dir> --plan` prints deterministic build order.
- `arcana build <workspace-dir>` builds workspace members in topo order and writes `Arcana.lock`.
- Grimoire `[deps]` path dependencies are import-resolvable in compile/check/run flows (`import <dep>.*`).
- Build/cache artifact layout is a toolchain detail and is not frozen here; current build artifacts are internal backend-contract output, not a public execution or bytecode format.

Current in-repo app/showcase behavioral pressure comes from rewrite-owned `grimoires/arcana/*`, `std/src`, conformance fixtures, and crate runtime tests. The broader MeadowLang showcase/app corpus is archived outside this repo.

## Concurrency / IO Surface (v0.17)

New shelf-first modules:

- `std.concurrent`
- `std.behaviors`
- `arcana_process.io`

Canonical surface:

- `std.concurrent.channel[T](capacity)`, `std.concurrent.mutex[T](value)`
- `std.concurrent.atomic_int(value)`, `std.concurrent.atomic_bool(value)`
- `ms :: :: std.concurrent.sleep`, `std.concurrent.thread_id :: :: call`
- `task :: :: done` / `task :: :: join`, `thread :: :: done` / `thread :: :: join`
- `channel :: value :: send` / `channel :: :: recv`, `mutex :: :: pull` / `mutex :: value :: put`
- `atomic :: :: load`, `atomic :: value :: store`, `atomic :: delta :: add`, ...
- `phase :: :: std.behaviors.step`
- `value :: :: arcana_process.io.print`

Surface policy:

- Legacy concurrency/behavior call names are removed from compiler special handling; use `std.concurrent` + methods and `std.behaviors.step`.
- Legacy call-name interception for `print` is removed; use `value :: :: arcana_process.io.print`.

## Concurrency + Behaviors (v0.4)

Implemented surface:

- `async fn`
- `weave fn :: ... :: call`
- `split fn :: ... :: call`
- `task >> await`
- `behavior[phase=..., affinity=...] fn ...`
- `"startup" | "fixed_update" | "update" | "render" :: :: std.behaviors.step`

Historical note: the broader MeadowLang concurrency/behavior examples now live outside this repo. Current in-repo behavioral pressure comes from rewrite-owned `std/src`, conformance fixtures, and crate tests.

Current behavior:

- `weave`/`split`/`await` and `std.behaviors.step` are supported by the current toolchain/runtime
- `async fn main() -> Int|Unit` is supported
- `split` now runs targets on real OS threads for sendable runtime values (`Int`/`Bool`/`Str`/sendable records), with thread/task completion via `h :: :: join` / `t :: :: join`
- `Channel[T]`, `Mutex[T]` (move-lock), `AtomicInt`, and `AtomicBool` are available via `std.concurrent` constructors and methods
- Channel ops now produce runtime errors for obvious closed/disconnected cases (`recv` on closed+empty, bounded `send` to a closed channel), with best-effort detection in the current single-handle channel model
- `weave` is executor-first in v0.4: it creates hot local tasks on the main-thread async runtime path; `split` remains the explicit OS-thread primitive
- `task :: :: done` now advances progress across the reachable local await-graph (not just the top task), and reports completion status without blocking
- `std.behaviors.step` executes metadata-driven scheduler groups with deterministic phase/group ordering
- the legacy desktop shell and `std.behaviors.step` are runtime-enforced as main-thread-only on that historical v0 lane


## Forewords (v1) and Comments

Arcana v1 forewords use prefix metadata syntax:

- `#name`
- `#name[arg]`
- `#name[arg1, arg2]`
- `#name[key = value]`
- `#name[arg1, key = value]`

Supported built-ins in v1:

- `#deprecated["message"]`
- `#only[os = "...", arch = "..."]`
- `#test`
- `#allow[...]`
- `#deny[...]`
- `#inline`
- `#cold`
- `#boundary[target = "lua" | "sql"]`

### Foreword lint names (`#allow[...]` / `#deny[...]`)

Currently recognized lint names:

- `deprecated_use`
- `unknown_foreword`
- `invalid_foreword_target`
- `invalid_foreword_payload`
- `type_like_name`
- `anon_shape_positional`

Heuristic lints:

- `type_like_name`:
  - warns when function/method names use PascalCase and likely represent missing type declarations
- `anon_shape_positional`:
  - warns when repeated anonymous tuple boundaries combine with repeated positional tuple access (`.0/.1/...`)
  - default threshold: at least 3 tuple boundaries and at least 3 positional accesses

v1 attachment targets:

- top-level declarations (`fn`, `record`, `enum`, `trait`, `impl`, `behavior`, `system`)
- `import`, `reexport`, `use`
- trait methods and impl methods
- chain statements for `#chain[...]` only
- attached header entries inside statement-form qualified/memory phrase blocks as header-local metadata carriers

Attached-header-entry note:

- these are header-local metadata carriers, not general statement/expression targets
- built-in declaration and chain-contract semantics do not automatically retarget there

Boundary note:

- `#boundary[...]` is compile-time only in v1
- it is valid on functions and impl methods
- it carries Lua/SQL varietal interop contracts, not embedding semantics

Not in v1:

- field/param forewords
- general statement/expression forewords outside chain-contract statements and attached header entries
- `#derive`
- user-defined forewords (`foreword ...`)

### Comment migration

`#` comments are removed. Use `//` comments for line and inline comments.

Compiler migration diagnostic:

`'#' comments were removed; use '//' comments and '#[...]' forewords`

Comment conversion helper:

- `powershell -ExecutionPolicy Bypass -File scripts/migrate_comments_to_slashslash.ps1` (dry-run)
- `powershell -ExecutionPolicy Bypass -File scripts/migrate_comments_to_slashslash.ps1 -Apply`

## Selfhost Host Platform v2

Host/tooling substrate is available under `arcana_process.*` plus `std.text`:

- `arcana_process.args`
- `arcana_process.env`
- `arcana_process.path`
- `arcana_process.fs`
- `arcana_process.process`
- `std.text`

Policy:

- Exact approved host-core surface lives in `docs/specs/selfhost-host/selfhost-host/v1-scope.md`.
- Imported `std` helper expansions beyond that approved host-core scope are not frozen here just because they were carried from MeadowLang.
- Runnable execution host-root and process-capability policy applies when execution commands return with the first runnable backend; it is not evidence of current CLI surface.

- Host filesystem APIs are sandboxed to a runtime host-root:
  - grimoire execution -> grimoire directory
  - file execution -> parent directory of file
  - artifact execution -> current working directory
- execution flows support app-argument pass-through via `--` and explicit host-root override via `--host-root <dir>`.
- `arcana_process.process.exec_status(program, args)` requires explicit capability opt-in:
  - `--allow-process` on runnable execution flows
  - without the flag: `process execution is disabled; rerun with --allow-process`
- `arcana_process.fs` includes binary file APIs: `read_bytes(path) -> Bytes` and `write_bytes(path, bytes)`.
- `arcana_process.fs` includes streaming APIs with a typed `FileStream` handle:
  - canonical handle path: `arcana_winapi.process_handles.FileStream`
  - `stream_open_read(path) -> Result[FileStream, Str]`
  - `stream_open_write(path, append) -> Result[FileStream, Str]`
  - `stream_read(edit stream, max_bytes) -> Result[Bytes, Str]`
  - `stream_write(edit stream, bytes) -> Result[Int, Str]`
  - `stream_eof(read stream) -> Result[Bool, Str]`
  - `stream_close(take stream) -> Result[Unit, Str]`

Host-tool/bootstrap note:

- The broader MeadowLang host/frontend bootstrap examples are now archived outside this repo.
- Current in-repo behavioral pressure comes from rewrite-owned `std/src`, `grimoires/arcana/*`, conformance fixtures, and crate tests.

