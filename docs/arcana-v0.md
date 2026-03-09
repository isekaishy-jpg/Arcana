# Arcana v0

Arcana v0 is a small, typed, indentation-based language with access-mode parameters (`read`, `edit`, `take`) and a native JIT runtime (default, strict CLIF execution).

Repository policy note:
- `POLICY.md` defines the current direction: no new public builtins, migrate toward Arcana-authored stdlib/shelves, and reject pseudo-builtin wrappers.

This repository currently implements a working toy subset with:

- `fn`, `let`, `mut`, `if`, `while`, `return`
- `Int`, `Bool`, `Str`, `Unit`
- Access-mode parameters and compile-time checks for `read`/`edit`/`take`
- `record` declarations, construction, field access, and field assignment
- Multi-file Grimoires (`book.toml`, `import`, `export`, `reexport`)
- Arcana-native concurrency/behavior support (`async fn`, `weave`, `split`, `>> await`, `behavior[...] fn`)
- Core operators: unary `-` / `not` / `~`, `%`, `!=`, `<=`, `>=`, `and`, `or`, bitwise `& | ^ << shr`, `Str + Str`, compound assignments
- Collections v0.9: `List[T]`, list literals, slicing with ranges, indexed assignment, `RangeInt`, pair tuples `(A, B)`
- Memory phrases v0.35: `arena|frame|pool: instance :> ... <: qualifier` with typed allocator storage (`Arena[T]`, `FrameArena[T]`, `PoolArena[T]` + corresponding ID handles)
- Ownership/lifetimes v0.32: explicit refs (`&'a T`, `&'a mut T`), borrow/deref expressions (`&x`, `&mut x`, `*x`), lexical borrow checking, and `#boundary[target="lua|sql"]` signature contracts
- Trait v2: associated types, default trait methods, supertrait bounds, projection equality in `where`
- `arcana run/check/compile`

## Commands

- `arcana run <file.arc | grimoire-dir> [--native-strict] [--host-root <dir>] [--allow-process] [--emit-scheduler-trace <file>] [-- [app args...]]`
- `arcana check <file.arc | grimoire-dir> [--emit-summary <file>]`
- `arcana selfhost-check <grimoire-dir> [--native-strict] [--host-root <dir>] [--allow-process] [--emit-summary <file>]`
- `arcana compile <file.arc | grimoire-dir> -o <file.arcbc> [--emit-summary <file>]`
- `arcana build <workspace-dir> [--member <name>] [--clean] [--plan] [--emit-ir-dump] [--emit-summary <file>]`
- `arcana chant <file.arcbc> [--native-strict] [--host-root <dir>] [--allow-process] [--emit-scheduler-trace <file>] [-- [app args...]]`

Single-file mode is intentionally minimal: it does not resolve `import`/`use`/`reexport`.
Use grimoire mode (`arcana run/check <grimoire-dir>`) for std/module-based programs.
Default runtime backend is `native`.
`--backend vm` is no longer supported on public commands and returns:
`vm backend is historical-only; use native runtime commands`.
Historical VM execution is available only through the hidden `vm-legacy` command with explicit opt-in:
`ARCANA_ENABLE_VM_LEGACY=1 arcana vm-legacy <file.arcbc>`.
Native paths are strict by default; `--native-strict` is accepted for compatibility and is redundant.
`arcana chant --backend native` requires signature-bearing scheduler metadata bytecode modules (`ARCB` v29+).
Source files are hard-standardized to `.arc` in file-mode commands and module loading.
Migration helper: `powershell -ExecutionPolicy Bypass -File scripts/migrate_source_extensions_to_arc.ps1` (add `-Apply` to execute).

`arcana build --member <name>` is strict: if non-target members have source fingerprint drift,
build fails and requires a full `arcana build <workspace-dir>` refresh.

### Check Cutover (Plan 50 Sunset)

- `arcana check` is selfhost-canonical and runs the Arcana frontend checker path.
- Legacy Rust check-oracle wiring has been retired.
- Oracle phase state is tracked in `docs/specs/backend/check_oracle_state.toml`.
- CI phase enforcement remains in `scripts/ci/check_oracle_phase_guard.ps1`.

Canonical selfhost check protocol records:
- `CHECK_DIAG_V1`:
  - `code`
  - `severity`
  - `path`
  - `line`
  - `column`
  - `end_line`
  - `end_column`
  - `message`
- `CHECK_FINAL_V1`:
  - `error_count`
  - `warning_count`
  - `checksum`

Parity contract compares `{code,severity,path,line,column,end_line,end_column}` only.
`message` text is informational and not parity-critical.

### Compile/Build Cutover (Plan 53 Sunset)

- `arcana compile` and `arcana build` are selfhost-canonical.
- Legacy compile/build oracle commands were sunset-removed.
- The compile/build oracle environment gate is no longer used.
- Compile/build oracle phase state is tracked in:
  - `docs/specs/backend/compile_oracle_state.toml` (`phase = "sunset"`).
- Compile/build conformance is now canonical-only:
  - committed native golden snapshots
  - repeated-run determinism checks
  - hard selfhost bootstrap proof
- Selfhost core closure policy is tracked separately in:
  - `docs/specs/backend/selfhost_core_state.toml`
  - current phase: `cutover`
- CI enforcement:
  - `scripts/ci/selfhost_compile_parity_guard.ps1`
  - `scripts/ci/selfhost_build_parity_guard.ps1`
  - `scripts/ci/selfhost_bootstrap_hard_guard.ps1`
  - `scripts/ci/selfhost_runnable_artifact_guard.ps1`
  - `scripts/ci/selfhost_no_proxy_guard.ps1`
  - `scripts/ci/selfhost_no_seed_guard.ps1`
  - `scripts/ci/selfhost_core_phase_guard.ps1`
  - `scripts/ci/compile_oracle_phase_guard.ps1`
- canonical selfhost compile/build/bootstrap guard lanes also run with
  `ARCANA_FORBID_HOST_COMPILER=1` so host compiler intrinsics are blocked in canonical mode
- canonical selfhost compile/build/bootstrap guard lanes also run with
  `ARCANA_FORBID_SEED_FALLBACK=1` so seed/template fallback paths are blocked in canonical mode
- canonical selfhost compile/build/bootstrap guard lanes also run with
  `ARCANA_FORBID_SELFHOST_BRIDGE=1` so hidden bridge/proxy subprocess paths fail deterministically
- canonical `compile` and `build` execute an installed runnable selfhost compiler artifact
  through the normal bytecode path with `allow_process=false`
- canonical selfhost artifact emit no longer uses host compiler intrinsics
- current first-party `book.toml` targets emit through generated compiler-core registry data synced
  from checked-in direct-emit specs, enforced by
  `scripts/ci/selfhost_direct_emit_spec_coverage_guard.ps1`
- current first-party `book.toml` targets also compile canonically under
  `ARCANA_FORBID_HOST_COMPILER=1`, `ARCANA_FORBID_SEED_FALLBACK=1`, and
  `ARCANA_FORBID_SELFHOST_BRIDGE=1` via `scripts/ci/selfhost_firstparty_compile_guard.ps1`
- shared compile-grade source/workspace logic is being extracted into
  `grimoires/arcana-compiler-core`
- install or refresh the canonical compiler artifact with:
  - `arcana selfhost-install`
- CI bootstraps a temporary installed toolchain via `selfhost-install` before running
  canonical selfhost compile/build/bootstrap guard lanes
- selfhost compiler artifacts are real runnable `ARCB` modules; `arcana chant <compiler.arcbc> -- compile ...`
  uses the normal bytecode runtime path rather than token routing or source-side Rust recompilation

Compile/build recovery policy:
- Permanent Rust recovery path is intentionally non-default.
- Hidden recovery commands exist only in feature-enabled builds:
  - `arcana recover-compile ...`
  - `arcana recover-build ...`
- Build requirement:
  - compile `arcana-cli` with `--features recovery-rust-oracle`
- Runtime gate:
  - `ARCANA_ENABLE_RECOVERY_ORACLE=1`

Canonical selfhost compile/build protocol records:
- `COMPILE_DIAG_V1`:
  - `code`
  - `severity`
  - `path`
  - `line`
  - `column`
  - `end_line`
  - `end_column`
  - `message`
- `COMPILE_ARTIFACT_V1`:
  - `kind`
  - `path`
  - `fingerprint`
  - `ir_fingerprint`
  - `bytecode_version`
- `BUILD_EVENT_V1`:
  - `member`
  - `status`
  - `artifact_path`
- `COMPILE_FINAL_V1`:
  - `error_count`
  - `warning_count`
  - `checksum`

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
- `std.types.game`
- `std.types` (reexports `core` + `game`)

Current `std.types.core` includes:

- `Vec2i`, `Size2i`, `Recti`, `ColorRgb`, `Tick`, `FrameIndex`
- helpers: `vec2`, `size2`, `rect`, `rgb`

Current `std.types.game` includes:

- `EntityId`, `Health`, `Damage`, `Score`, `TeamId`
- helpers: `entity_id`, `health`, `damage`, `score`, `team_id`

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

## Generic Phrase Family (v0.24)

Arcana’s phrase family has three forms:

| Phrase | Form | Scope |
|---|---|---|
| Qualified phrase | `subject :: args? :: qualifier` | expression + statement |
| Memory phrase | `memory_type: instance :> args? <: qualifier` | expression + statement |
| Chain phrase | `style :=(>|<) stage...` with `=>`/`<=` connectors | expression + statement |

Header phrases are qualified/memory phrases. A header phrase may own an attached block. In attached blocks:

- `name = expr` remains valid header metadata/named-arg overflow
- chain phrase lines are valid and execute in source order
- each attached chain receives the prior result implicitly

Evaluation order for attached header blocks:

1. Apply header `name = expr` entries as named args.
2. Execute header phrase.
3. Execute first attached chain from header result.
4. Pipe through remaining attached chains in source order.

## Qualified Phrases (v0.22)

Arcana now supports qualified phrase invocation syntax:

- `subject :: args :: qualifier`
- args are comma-separated and limited to at most 3 top-level inline items
- qualifier forms:
  - named/path (for example `call`, `join`, `std.io.print`)
  - symbols: `?`, `>`, `>>`

Core behavior:

- `f :: a, b :: call` calls `f(a, b)`
- `obj :: x :: method_name` dispatches method `method_name` on `obj`
- `value :: :: std.io.print` dispatches `std.io.print(value)`
- `result_expr :: :: ?` applies try-propagation
- `task_expr :: :: >>` awaits task value

Statement-form phrases can carry attached blocks:

```arc
Counter :: :: call
    value = 1
```

For `call`, `>`, and bare method qualifiers, attached `name = expr` entries are treated as
additional named arguments.

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

See `examples/operators_core`.
See `examples/operators_bitwise`.
See `examples/operators_concat_assign`.

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
- Shelf-first list API (Plan 15):
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

See `examples/list_core`.
See `examples/list_slice_range`.
See `examples/list_index_compound`.
See `examples/pair_try_pop`.

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

See `examples/array_core`.
See `examples/map_core`.
See `examples/for_collections`.
See `examples/map_index_compound`.

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

See `examples/plan13_box_generic`.
See `examples/plan13_generics_match`.
See `examples/plan13_result_generic`.
See `examples/trait_v2_iter_ecs`.

## Iterator + ECS Trait Proof Slice (v0.27)

Std now includes a trait-based iterator foundation:

- `std.iter.Iterator[I]` with associated type `Item`
- default-friendly trait impl model in language core
- helpers like `std.iter.count`

`std.ecs` now provides an iterator adapter:

- `std.ecs.SingletonCursor[T]`
- `impl std.iter.Iterator[SingletonCursor[T]] for SingletonCursor[T]`

This demonstrates associated-type-based abstractions across std modules without adding new builtins.

## Std-Style Shelf Foundation

A std-style shelf layout now works in Grimoires using `impl` extension methods over kernel collection intrinsics.

See `examples/grimoire_std_methods_app` for:

- `std.result` / `std.option` user enums
- `std.collections.list` extension methods (`len`, `push`, `pop`, `try_pop_or`)
- `std.collections.map` extension methods (`len`, `has`, `get`, `set`, `try_get_or`)
- `std.collections.array` extension methods (`len`, `to_list`)

This provides the intended direction where app code uses shelf methods instead of direct builtin calls.

## De-Builtinization Phase 1 (v0.15)

Collections are now de-builtinized through an internal intrinsic bridge.

- Public usage is shelf-first via `std.collections.*`.
- Compiler/VM collection semantics no longer rely on collection callee-string dispatch branches.
- Direct calls to legacy collection builtins are hard errors by default.
- Legacy collection call names are removed; use shelf-first APIs.
- `intrinsic fn` is internal: restricted to trusted `std.kernel.*` modules.
- `std.kernel.*` is internal-only and cannot be imported/reexported by user/app modules.

## Memory Phrase + Typed Allocators (v0.35)

Arcana supports explicit memory-context allocation phrases with three allocator families.

Memory phrase syntax:

- `memory_type: instance :> args? <: qualifier`
- v2 supports `memory_type = arena | frame | pool`
- inline args are comma-separated, up to 3 top-level items
- arg items support positional and named (`name = expr`)

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

New shelf module:

- `std.memory`

Public surface:

- `std.memory.new[T](capacity) -> Arena[T]`
- `std.memory.frame_new[T](capacity) -> FrameArena[T]`
- `std.memory.pool_new[T](capacity) -> PoolArena[T]`
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

Runtime semantics:

- `Arena[T]` and `FrameArena[T]` are non-Copy and non-sendable.
- `PoolArena[T]` is sendable/shareable when `T` is sendable.
- `ArenaId[T]` and `FrameId[T]` are copy-like and non-sendable.
- `PoolId[T]` is copy-like and sendable when `T` is sendable.
- `frame` is append-only with explicit `reset` and generation invalidation.
- `pool` supports `remove` + free-list reuse with generation checks.
- `reset` is explicit only (no auto-reset semantics).
- stale/invalid id access raises deterministic runtime errors:
  - `arena id is invalid or stale`
  - `frame id is invalid or stale`
  - `pool id is invalid or stale`

Notes:

- Attached blocks on memory phrase statements only accept `name = expr`.
- Unknown memory types are rejected with a future-reserved diagnostic.
- `std.memory` exposes allocator borrow APIs:
  - `borrow_read` / `borrow_edit` for `Arena`, `FrameArena`, and `PoolArena`.
- `reset`/`remove` are compile-time rejected when live allocator borrows would be invalidated.

See:

- `examples/arena_ast_builder`
- `examples/arena_reset_cycle`
- `examples/arena_id_safety`
- `examples/frame_scratch_cycle`
- `examples/pool_reuse_cycle`
- `examples/memory_phrase_frame_pool`

## Ownership And Lifetimes (v0.32)

Arcana now enforces lexical ownership and borrowing rules with explicit lifetimes.

Core syntax:

- lifetime params: `'a`, `'b`
- reference types: `&'a T`, `&'a mut T`
- borrow/deref expressions: `&x`, `&mut x`, `*x`
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
- system/behavior boundary checks enforce resolved chain contract fields
- any chain inside `behavior[...]` or `system[...]` must declare explicit `#chain[...]`

Supported `#stage` keys:

- `pure`, `deterministic`, `effect`, `thread`, `authority`, `rollback_safe`
- `reads`, `writes`, `excludes` (repeatable)

Supported `#chain` keys:

- `phase`, `deterministic`, `thread`, `authority`, `rollback_safe`

Migration note:

- legacy `reverse :=> ...` style is removed
- use an existing directional style with reverse introducer and reverse connectors (`<style> :=< ... <= ...`)

See also:

- `docs/specs/chain/contract_matrix_v1.md`

Metadata blocks:

- standalone chain attachment blocks are currently validated as `name = expr` only
- metadata is parsed/validated in v1 and ignored semantically

See:

- `examples/chain_forward_pipeline`
- `examples/chain_header_attached`
- `examples/chain_styles_matrix`

## Current Implementation Limits (Not Frozen Language Law)

These are current checker/runtime limits from the in-progress rewrite. They are not, by themselves, endorsed long-term language design and must not be treated as selfhost contract unless promoted explicitly.
Where a domain scope exists under `docs/specs/**/v1-scope.md`, that domain scope wins over these notes.

- `edit` call arguments currently must be local bindings (not field expressions)
- Access checking is currently root-binding based (conservative)
- Moves inside `while` loops are currently rejected

## Canvas/Window/Input (v0.16 shelf-first)

Desktop APIs are now shelf-first through toolchain std modules:

- `std.canvas`:
  - `open`, `alive`, `fill`, `rect`, `label`, `present`, `rgb`
  - `image_load`, `image_size`, `blit`, `blit_scaled`, `blit_region`
- `std.window`:
  - `size`, `resized`, `fullscreen`, `minimized`, `maximized`, `focused`
  - `set_title`, `set_resizable`, `set_fullscreen`, `set_minimized`, `set_maximized`, `set_topmost`, `set_cursor_visible`, `close`
- `std.input`:
  - `key_code`, `key_down`, `key_pressed`, `key_released`
  - `mouse_button_code`, `mouse_pos`, `mouse_down`, `mouse_pressed`, `mouse_released`, `mouse_wheel_y`, `mouse_in_window`

Recommended app-facing layer:

- `winspell` (first-party lib grimoire) depends on `std.*` and provides a maintainable windowing facade for demos/apps.
- `spell_events` (first-party companion lib grimoire) provides event/keybind/frame-input helpers on top of `winspell` + `std.events`.
- Demo/window-focused apps should prefer `winspell.*`; `std.*` remains the low-level stable substrate.

Plan 17 migration status:

- Direct legacy calls to `sigil_*`, `window_*`, `key_*`, `mouse_*` are hard errors.
- The compiler now lowers desktop API calls through intrinsic bindings rather than owning semantics by callee-name dispatch.
- `std.*` is reserved for toolchain std modules.

Notes:

- `Window` and `Image` are typed opaque values.
- `std.canvas.fill`, `std.canvas.rect`, `std.canvas.label`, and `std.canvas.present` require `edit win`.
- `std.canvas.rgb` clamps channels to `0..255`.
- `std.canvas.rect` rejects negative width/height at runtime.
- Input state is updated on `canvas.alive :: win :: call` and remains stable for that frame.
- Input/window/image APIs are main-thread-only.
- `std.input.key_code` / `std.input.mouse_button_code` resolve string names at runtime.
- Public typed event queue APIs are available through `std.events`.
- `canvas.alive :: win :: call` is also the input/window lifecycle pump boundary.
- `window.resized :: win :: call` is a per-frame edge flag.
- `std.canvas.image_load` supports PNG-only in v0.12+.
- Image blits use nearest-neighbor scaling and source-over alpha blending.

Backend note:

- The Windows runtime backend is `winit + softbuffer` with immediate present semantics.
- `std.window.set_topmost` is supported on the Windows backend.
- Non-Windows builds keep deterministic unsupported diagnostics for window runtime paths.

## Events + App Helpers (v0.17)

New modules:

- `std.events`
- `std.app`
- `std.ecs`
- `std.tooling`

`std.events` provides typed queue access sourced from the same frame pump boundary:

- `std.events.poll(read win) -> Option[std.events.AppEvent]`
- `std.events.drain(read win) -> List[std.events.AppEvent]`

`std.events.AppEvent` variants:

- `WindowResized((Int, Int))`
- `WindowCloseRequested`
- `WindowFocused(Bool)`
- `KeyDown(Int)` / `KeyUp(Int)`
- `MouseDown(Int)` / `MouseUp(Int)`
- `MouseMove((Int, Int))`
- `MouseWheelY(Int)`

`std.app` provides deterministic fixed-step helpers:

- `fixed_tick_ms(tick_hz)`
- `fixed_consume_steps(edit accumulator_ms, frame_ms, tick_ms)`
- `fixed_alpha_milli(accumulator_ms, tick_ms)`
- `fixed_runner(tick_hz) -> std.app.FixedRunner`
- `fixed_runner_step(edit runner, frame_ms) -> (Int, Int)` (steps, alpha_milli)
- `fixed_runner_reset(edit runner)`

`std.ecs` currently provides scheduler-phase helpers for behavior/system stepping:

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

- Component-parameter systems iterate deterministic ECS query cursors over archetype-backed world data.
- `affinity=worker` component systems run via worker execution when component arguments are transferable; non-transferable component values fall back to deterministic main-thread execution.
- Worker-applied component writes are fail-fast checked; if a targeted component changed before apply, runtime reports an ECS worker apply conflict.
- Worker-affinity systems require sendable component parameter types.
- A single system cannot declare multiple `edit` parameters for the same component type.

`std.tooling` currently provides Arcana-authored local planning helpers:

- `plan_local_workspace(members, deps) -> Result[List[Str], Str]`
- `std.tooling.graph.topo_sort(members, deps)`

Toolchain integration:

- `arcana build <workspace-dir> --plan` compiles and executes `std.tooling` to print deterministic build order.
- `arcana build <workspace-dir>` compiles workspace members in topo order, reuses deterministic artifact cache entries, and writes `Arcana.lock`.
- `arcana build <workspace-dir> --emit-ir-dump` writes deterministic SSA CFG IR dump files (`arcana-ir-v2-ssa`) to `.arcana/logs/*.ir.txt` during build compilation.
- In `build` mode, members compile against artifact-derived dependency interfaces and app outputs are linked with compiled `.arclib` dependency modules (no dependency source flattening in the build path).
- Grimoire `[deps]` path dependencies are import-resolvable in compile/check/run flows (`import <dep>.*`).
- Build artifacts are written under `.arcana/artifacts/<member>/<fingerprint_hex>.<ext>`:
  - app members: `.arcbc`
  - lib members: `.arclib`
- Build summaries are written to `.arcana/logs/build-last.txt`.
- `Arcana.lock` (v3) includes members, order, paths, deps, fingerprints, artifact paths, and per-member artifact metadata (`kind`, `format`, `content_hash`).

See:

- `examples/window_hello`
- `examples/window_quads`
- `examples/input_tester`
- `examples/window_image_viewer`
- `examples/window_controls`
- `examples/events_poll_demo`
- `examples/grimoire_ecs_mini_game`
- `examples/arcana_showcase`
- `examples/life_lab`
- `examples/grimoire_counter_app`
- `examples/grimoire_window_app`
- `examples/grimoire_ui_lib`
- `examples/grimoire_ecs_schedule`
- `grimoires/winspell`
- `grimoires/spell-events`

## Concurrency / IO Migration (v0.17)

New shelf-first modules:

- `std.concurrent`
- `std.behaviors`
- `std.io`

Canonical surface:

- `std.concurrent.channel[T](capacity)`, `std.concurrent.mutex[T](value)`
- `std.concurrent.atomic_int(value)`, `std.concurrent.atomic_bool(value)`
- `ms :: :: std.concurrent.sleep`, `std.concurrent.thread_id :: :: call`
- `task :: :: done` / `task :: :: join`, `thread :: :: done` / `thread :: :: join`
- `channel :: value :: send` / `channel :: :: recv`, `mutex :: :: pull` / `mutex :: value :: put`
- `atomic :: :: load`, `atomic :: value :: store`, `atomic :: delta :: add`, ...
- `phase :: :: std.behaviors.step`
- `value :: :: std.io.print`

Plan 17 policy:

- Legacy concurrency/behavior call names are removed from compiler special handling; use `std.concurrent` + methods and `std.behaviors.step`.
- Legacy call-name interception for `print` is removed; use `value :: :: std.io.print`.

## Plan 4 Concurrency + Behaviors (v0.4)

Implemented surface:

- `async fn`
- `weave fn :: ... :: call`
- `split fn :: ... :: call`
- `task >> await`
- `behavior[phase=..., affinity=...] fn ...`
- `"startup" | "fixed_update" | "update" | "render" :: :: std.behaviors.step`

Examples:

- `examples/async_weave`
- `examples/async_main`
- `examples/split_join`
- `examples/behavior_phases`
- `examples/channel_ping`
- `examples/channel_async`
- `examples/async_channel`
- `examples/mutex_counter`
- `examples/atomic_counter`
- `examples/grimoire_behavior_app`

Implementation notes (current behavior):

- `weave`/`split`/`await` and `std.behaviors.step` are implemented end-to-end in compiler + bytecode + native
- `async fn main() -> Int|Unit` is supported
- `split` now runs targets on real OS threads for sendable runtime values (`Int`/`Bool`/`Str`/sendable records), with thread/task completion via `h :: :: join` / `t :: :: join`
- `Channel[T]`, `Mutex[T]` (move-lock), `AtomicInt`, and `AtomicBool` are available via `std.concurrent` constructors and methods
- Channel ops now produce runtime errors for obvious closed/disconnected cases (`recv` on closed+empty, bounded `send` to a closed channel), with best-effort detection in the current single-handle channel model
- `weave` is executor-first in v0.4: it creates hot local tasks on the main-thread async runtime path; `split` remains the explicit OS-thread primitive
- Local task execution now preserves/resumes VM call frames across suspension points for `std.concurrent.sleep`, `Channel.recv`, bounded-full `Channel.send`, and `Mutex.pull`
- Local channel/mutex waits in suspended tasks now use host condvars for wake blocking, and nested local-task `await` blocks directly on the child task instead of short polling
- `task :: :: done` now advances progress across the reachable local await-graph (not just the top task), and reports completion status without blocking
- native `std.behaviors.step` executes metadata-driven scheduler groups with deterministic phase/group ordering
- VM is historical-only and no longer part of active runtime command flows.
- `std.canvas.*`, `std.window.*`, `std.input.*`, and `std.behaviors.step` are runtime-enforced as main-thread-only
- Local tasks now use a thread-local ready queue + wake notifications and queue-driven progression across await-graphs; `split` threads still use blocking host-thread semantics by design (intentional for v0.4)


## Forewords (v1) and Comment Cutover

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

Not in v1:

- field/param/statement/expression forewords
- `#derive`
- user-defined forewords (`foreword ...`)

### Comment migration

`#` comments are removed. Use `//` comments for line and inline comments.

Compiler migration diagnostic:

`'#' comments were removed; use '//' comments and '#[...]' forewords`

Migration helper:

- `powershell -ExecutionPolicy Bypass -File scripts/migrate_comments_to_slashslash.ps1` (dry-run)
- `powershell -ExecutionPolicy Bypass -File scripts/migrate_comments_to_slashslash.ps1 -Apply`

## Selfhost Host Platform v2

Native-first host/tooling substrate is available under `std.*`:

- `std.args`
- `std.env`
- `std.path`
- `std.fs`
- `std.process`
- `std.bytes`
- `std.text`

Policy:

- Native backend is canonical for host APIs.
- Historical VM behavior is available only via hidden `vm-legacy` command and is not part of active host/tooling workflows.
- Host filesystem APIs are sandboxed to a runtime host-root:
  - `run <grimoire-dir>` -> grimoire directory
  - `run <file.arc>` -> parent directory of file
  - `chant <file.arcbc>` -> current working directory
- `run` and `chant` support app-argument pass-through via `--` and explicit host-root override via `--host-root <dir>`.
- native scheduler trace output can be emitted with:
  - `--emit-scheduler-trace <file>` on `run` and `chant` (native-only)
- `std.process.exec_status(program, args)` requires explicit capability opt-in:
  - `--allow-process` on `run`, `chant`, or `selfhost-check`
  - without the flag: `process execution is disabled; rerun with --allow-process`
- `std.fs` includes binary file APIs: `read_bytes(path) -> Array[Int]` and `write_bytes(path, bytes)`.
- `std.fs` includes streaming APIs:
  - `stream_open_read(path) -> Int`
  - `stream_open_write(path, append) -> Int`
  - `stream_read(stream_id, max_bytes) -> Array[Int]`
  - `stream_write(stream_id, bytes) -> Int`
  - `stream_eof(stream_id) -> Bool`
  - `stream_close(stream_id)`

Host-tool bootstrap example:

- `examples/selfhost_host_tool_mvp`
- `examples/selfhost_frontend_mvp` (Arcana frontend/typecheck MVP verification path)
