# Arcana Rewrite Working Language Inventory

Status: `reference-only` working spec inventory.

This document is meant to be fuller and more concrete than a summary or guide. It is still not authoritative by itself.

Important assumption:

- some language bits are probably still missing from this document
- some entries may still be misclassified
- when review work finds a missed surface or a wrong status, this file should be updated immediately

This is a working full-sheet, not a claim that the inventory is already complete.

## Authority Order

Use this order when judging anything in this file:

1. `POLICY.md`
2. `docs/specs/spec-status.md` and any file it marks frozen or approved
3. `docs/rewrite-roadmap.md` and `PLAN.md`
4. `crates/*`
5. this document

## Status Keys

- `current`: present enough in the rewrite to treat as active current Arcana behavior
- `partial`: present, but not closed enough to treat as settled
- `gap`: expected surface is missing or still non-functional
- `intentional`: rewrite appears to have intentionally narrowed or changed the old behavior
- `needs-review`: we have signals, but the exact current status still needs another verification pass

## Package And Module Surface

- `current`: packages use `book.toml`
- `current`: source roots live under `src/`
- `current`: `import`, `use`, `export`, and `reexport`
- `current`: multi-package workspace/package loading in the rewrite toolchain
- `current`: only local path dependencies are supported pre-selfhost
- `current`: git and registry dependency sources remain rejected
- `intentional`: archived Meadow-era package topology is not rewrite authority

## Declaration Surface

- `current`: `fn`
- `partial`: `async fn`
  current for source surface and parsing; runtime semantics are not closed
- `current`: `record`
- `current`: `enum` with payload variants
- `current`: `trait`
- `current`: `impl`
- `current`: associated types in traits and impls
- `current`: associated type defaults and impl bindings as source surface
- `current`: `intrinsic fn`
- `current`: `opaque type`
- `current`: `behavior[...] fn`
- `current`: `system[...] fn`

## Core Value And Type Surface

- `current`: `Int`
- `current`: `Bool`
- `current`: `Str`
- `current`: `Unit`
- `current`: records
- `current`: enums
- `current`: 2/3-tuples `(A, B)` and `(A, B, C)`
- `intentional`: 2/3-tuple baseline; wider tuple support is not the current baseline
- `current`: `List[T]`
- `current`: `Array[T]`
- `current`: `Map[K, V]`
- `partial`: `RangeInt`
  frozen/current source contract still depends on it, but runtime closure is not complete
- `current`: rewrite-owned opaque std/runtime handle types such as window/image/frame/audio/file-stream handles

## Ownership, Access Modes, And Borrowing

- `current`: `read`
- `current`: `edit`
- `current`: `take`
  present at source level and in frontend ownership checking
- `current`: borrow and deref source surface
- `current`: explicit ownership-sensitive std surface in rewrite-owned `std`
- `partial`: `edit` write-through across all runtime call paths
- `gap`: full `take` move/invalidation semantics across user-defined runtime routine boundaries
- `partial`: consuming-handle equivalence between user-defined routines and host intrinsics

## Expression Surface

- `current`: int literals
- `current`: bool literals
- `current`: string literals
- `current`: pair expressions
- `current`: path expressions
- `current`: member access
- `current`: generic application surface
- `current`: binary operators and unary operators in the current frontend
- `current`: record construction
- `current`: enum variant construction
- `current`: `match` expressions
- `current`: qualified phrases as source surface
- `current`: memory phrases as source surface
- `current`: chain expressions as source surface
- `current`: `await expr` surface
- `current`: unary `weave` and `split` surface
- `partial`: collection literal `[]` surface
  currently accepted as list-only rewrite baseline; empty-list typing still depends on surrounding typing context
- `intentional`: non-empty map literals `{key: value, ...}` are not part of the current rewrite baseline
- `gap`: direct `expr :: :: ?` execution lane
- `gap`: direct `task_expr :: :: >>` execution lane
- `gap`: expression-level index/slice/range execution closure in runtime

## Statement Surface

- `current`: `let`
- `current`: `return`
- `current`: expression statements
- `current`: `if`
- `current`: `while`
- `current`: `for`
- `current`: `defer`
- `current`: `break`
- `current`: `continue`
- `current`: assignment statements
- `partial`: `for` execution
  currently present, but not closed over `RangeInt`/old range-loop expectations
- `gap`: indexed assignment/runtime closure

## Qualified Phrase Surface

- `current`: path qualifiers
- `current`: `?` qualifier is recognized in syntax
- `current`: `>` qualifier is recognized in syntax
- `current`: `>>` qualifier is recognized in syntax
- `current`: generic callable phrases
- `partial`: general qualified phrase execution
- `partial`: qualifier-specific attachment validation in syntax
- `gap`: direct try-propagation execution for `?`
- `gap`: direct await-apply execution for `>>`
- `gap`: stable dotted qualifier callable identity through runtime/backend
- `gap`: complete attachment execution tolerance in runtime

## Collection Surface

- `current`: rewrite-owned list API through `std.collections.list`
- `current`: rewrite-owned array API through `std.collections.array`
- `current`: rewrite-owned map API through `std.collections.map`
- `current`: set/map-backed collection layers in rewrite-owned `std`
- `current`: pair-return collection helpers
- `partial`: collection literal rules
  current rewrite accepts `[]`, but the frozen contract still says list literals are non-empty only
- `gap`: map literal syntax/execution
- `gap`: closed `RangeInt` list/slice/loop behavior
- `needs-review`: exact current literal-pattern/runtime parity for collection-heavy `match` and index cases after Milestone 7 changes

## Match And Pattern Surface

- `current`: wildcard patterns
- `current`: name/binding patterns
- `current`: variant patterns
- `current`: literal patterns as source surface
- `partial`: runtime literal matching semantics
  this area has had review findings and should be re-verified again before it is treated as settled
- `intentional`: tuple patterns are not part of the current rewrite baseline
- `intentional`: tuple-pattern narrowing is deliberate because the old tuple behavior was bug-prone
- `old-only`: match guards existed in Meadow-era Arcana, but are not clearly part of the current rewrite contract

## Traits, Impls, And Where Semantics

- `current`: trait declarations
- `current`: impl declarations
- `current`: associated types
- `current`: associated type defaults
- `current`: associated type impl bindings
- `current`: `where` clauses as source text surface
- `partial`: trait-bound checking in the rewrite
- `gap`: structured projection-equality semantics such as `Iterator[I].Item = U`
- `partial`: associated-type-heavy type-law beyond surface validation

## Tuple Surface

- `current`: 2/3-tuples in expressions and types
- `current`: `.0` / `.1` / `.2`
- `intentional`: 2/3-tuple contract in current rewrite baseline
- `intentional`: tuple patterns in `match` are excluded from the current rewrite baseline
- `current`: exact recursive tuple destructuring in `let` and `for`
- `gap`: tuple destructuring in params
- `old-only`: richer Meadow-era tuple behavior should not be auto-restored without redesign

## Chain Surface

- `current`: chain expressions in syntax/HIR/IR
- `current`: chain styles such as `forward`, `lazy`, `parallel`, `async`, `plan`, `broadcast`, `collect`
- `current`: chain contract syntax and validation for `#stage[...]` / `#chain[...]`
- `partial`: chain metadata carriage through the frontend/backend pipeline
- `gap`: general chain expression runtime execution
- `gap`: settled lowering/execution contract for chain styles
- `gap`: multithreaded runtime substrate that chain parallel/async surfaces want to sit on
- `gap`: thread/task-backed execution for chain parallel/async behavior
  this is not just Meadow-era richness; it is part of the positive execution model Arcana still appears to want, and current runtime shims do not provide it

## Async, Tasks, Threads, And Behaviors

- `current`: async source surface exists
- `current`: task/thread/channel/mutex/atomic std surface exists in rewrite-owned `std`
- `current`: first-party behavior/system surface exists
- `partial`: behavior stepping/runtime scheduler lane exists
- `partial`: task/thread surface exists as runtime objects
- `gap`: `async fn main` as fully settled runtime behavior
- `gap`: real `weave` task semantics
- `gap`: real `split` thread semantics
- `gap`: task/thread execution substrate as real program behavior rather than done-handle shims
- `gap`: final scheduler/worker model
- `gap`: final chain/async interaction model
- `partial`: `thread_id`
  current runtime implementation is still stub-shaped

## Memory Surface

- `current`: allocator families `arena`, `frame`, `pool`
- `current`: typed allocator ids/slots in the rewrite runtime
- `current`: borrow-read / borrow-edit surface
- `current`: memory phrases as source surface
- `current`: rewrite-owned memory std/kernel surface is materially beyond the old minimal Meadow-era shape
- `partial`: memory phrase execution
- `gap`: memory phrase attachments in runtime
- `needs-review`: exact long-term contract split between approved memory docs and actual rewrite-owned memory behavior still needs cleanup

## Forewords And Page Rollups

- `current`: forewords as approved first-party surface
- `current`: page rollups as approved first-party surface
- `current`: syntax-side validation and ownership checks for these domains
- `gap`: runtime tolerance/execution when executable forms carry attached forewords or rollups

## Opaque Handles And Runtime Resources

- `current`: source-level opaque runtime/resource surface exists
- `current`: rewrite-owned window/input/canvas/events/time/audio seams exist
- `partial`: current handle model is bootstrap-approved and usable
- `gap`: final long-term handle/resource model, ownership law, and ABI/interop contract

## Backend And Runtime Model Notes

- `current`: rewrite owns the runtime/backend path
- `current`: linked rewrite-owned `std` executes
- `partial`: internal AOT/backend artifact contract exists
- `partial`: runtime plan loading/execution exists
- `gap`: final native emitted artifact shape
- `gap`: complete removal of executor-owned public-std shims
- `gap`: carrying resolved qualified-call identity cleanly through lowering/AOT instead of re-resolving in runtime

## Meadow-Era Surfaces That Are Real Current Gaps

These came through Meadow-era Arcana, but they should not be treated as mere historical richness. They are real current gaps because the frozen contract, rewrite direction, or active language surface still points at them.

- direct `?` phrase behavior
- direct `>>` phrase behavior
- real task/thread execution substrate
- chain parallel/async lowering and execution semantics
- range-driven `for` loops
- structured projection-equality `where` semantics

## Meadows-Era Richness That Still Exerts Pressure

These are not automatically current Arcana law, but they are important pressure points because Meadow-era Arcana really did have them in code.

- tuple patterns
- match guards
- non-empty map literals
- richer tuple destructuring behavior beyond the pair baseline

## Highest-Risk Missing Domains

If we keep missing things, they are most likely to come from these domains:

- access modes / ownership / `take`
- qualified phrases and qualifier-specific semantics
- collections / `RangeInt` / literals / indexing / slicing
- async / task / thread / behavior scheduling
- associated types and `where` semantics
- opaque handles and runtime resource lifecycle

## Maintenance Rule

When review work finds a missed language bit, this file should be updated with:

1. the concrete surface
2. its current status (`current` / `partial` / `gap` / `intentional` / `needs-review`)
3. whether the signal came from approved docs, rewrite crates, Meadow-era code, or all three

Do not wait for the domain spec split before correcting this inventory.
