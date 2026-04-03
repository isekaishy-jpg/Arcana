# Headed Regions v1 Detailed Implementation Plan

## Summary
- Land headed regions as a full, first-class feature across parser, syntax AST, HIR, frontend validation/flow, executable IR, runtime execution, direct/native lowering, fixtures, and selfhost-matrix coverage in one coordinated window.
- Treat this as a contract-closing feature, not a syntax-only slice. The same patch updates the approved docs so the implemented ride law is the frozen v1 law.
- Keep the `Memory` head architecturally extension-ready. V1 source acceptance remains closed to `arena` / `frame` / `pool`, but the implementation uses a shared family-descriptor layer so later families such as `views` / `slices` can plug in without another headed-region redesign.

## Language Law To Freeze In This Patch
- Headed-region headers stay colonless and indentation-owned.
- Dedented leading `-cleanup` remains footer-only. Trailing `-modifier` inside headed-region headers and participating lines is contextual headed-region syntax, not a generic dash footer family.

### `recycle`
- Statement-form only.
- Legal line families:
  - bare gate expression
  - `let [mut] name = gate`
  - `name = gate`
- `name = gate` targets plain local names only. No member/index assignment inside `recycle`.
- Gate families:
  - `Bool`: `true` succeeds, `false` fails
  - `Option[T]`: `Some(value)` succeeds, `None` fails
  - `Result[T, E]`: `Ok(value)` succeeds, `Err(error)` fails
- Successful payload-bearing lines establish or replace the local immediately for later lines in the same region and leave the survivor visible after normal fallthrough.
- Legal modifiers:
  - `-break`
  - `-continue`
  - `-<owner_exit_name>`
  - `-return`
  - `-return <expr>`
- Bare `-return` is legal only on `Result[...]` gates and propagates the original `Err(...)` unchanged.
- `Bool` and `Option[...]` failures require `-return <expr>` if they route to return.
- `-break` / `-continue` remain loop-only.
- Named owner exits are validated against the active owner scope and become a real runtime control-flow signal.

### `bind`
- Statement-form only.
- Legal line families:
  - `let [mut] name = gate`
  - `name = gate`
  - `require <bool_expr>`
- Payload-bearing `let` / `name =` lines accept only `Option[...]` and `Result[...]`.
- `require <bool_expr>` is the only `Bool`-based bind line.
- Legal modifiers:
  - `-return`
  - `-return <expr>`
  - `-break`
  - `-continue`
  - `-default <expr>`
  - `-preserve`
  - `-replace <expr>`
- Bare `-return` is legal only on `Result[...]` payload-bearing lines and propagates the original `Err(...)`.
- `-default <expr>` is legal only on `let ... = gate` lines and establishes the new binding from the fallback when the gate fails.
- `-preserve` and `-replace <expr>` are legal only on `name = gate` refinement lines. `preserve` keeps the existing binding; `replace` overwrites it with the fallback.
- `require <bool_expr>` supports `return`, `break`, and `continue` failure handling. It is invalid under `default`, `preserve`, or `replace`.
- Successful bind lines update the visible local state immediately for later lines in the same region and leave the final established locals visible after normal fallthrough.

### `construct`
- Uses explicit constructor targets in the header so the type/variant source is never inferred from field names.
- Header spellings:
  - `construct yield <ctor_path> -<modifier>`
  - `construct deliver <ctor_path> -> <name> -<modifier>`
  - `construct place <ctor_path> -> <target> -<modifier>`
- `yield` is the only expression-form headed region in v1.
- `deliver` introduces a fresh local binding named `<name>` whose type comes from `<ctor_path>`.
- `place` targets an existing assignable local/member/index place whose type must match `<ctor_path>`.
- Allowed constructor targets:
  - record paths
  - single-payload enum variant constructor paths
- Legal line families:
  - `field = expr`
  - `payload = expr`
- `field = expr` is valid only when `<ctor_path>` names a record.
- `payload = expr` is valid only when `<ctor_path>` names a single-payload enum variant.
- Duplicate `field` / `payload` contribution names are compile-time errors.
- Modifier family lands fully in v1:
  - `-return`
  - `-return <expr>`
  - `-default <expr>`
  - `-skip`
- Bare `-return` on construct gate contributions is legal only for `Result[...]` contributions and propagates the original `Err(...)`.
- `-default <expr>` substitutes a contribution value and counts the contribution as fulfilled.
- `-skip` is legal only when the destination field/payload type is `Option[...]`; it materializes `Option.None` and counts the contribution as fulfilled.
- Record completion must be compile-time total: every non-`Option[...]` record field must have exactly one fulfilling contribution lane after modifier rules are applied.
- Variant completion requires exactly one fulfilling `payload` contribution lane.

### `Memory`
- Legal at module scope and block scope.
- Module-scope `Memory` specs live in a dedicated memory-spec namespace and are implicitly path-addressable in v1 without a separate `export` keyword.
- Block-scope `Memory` specs are visible only inside the enclosing execution scope.
- The current memory-phrase shape stays intact:
  - `memory_type: instance :> ... <: qualifier`
- In that `instance` slot, resolution order is:
  1. ordinary allocator value in value scope
  2. memory-spec handle in the memory-spec namespace for the same family
- `Memory` specs are not ordinary value bindings and cannot be used outside approved memory-aware forms.

## Shared Family Infrastructure
- Add a new lightweight workspace crate, `crates/arcana-language-law`, as the dependency-root source of truth for:
  - headed-region head enums
  - construct completion enums
  - modifier kinds and payload kinds
  - memory family descriptors
  - memory detail-key descriptors
  - per-family supported key/value tables
- `arcana-syntax`, `arcana-hir`, `arcana-frontend`, `arcana-ir`, and `arcana-runtime` all depend on this crate.
- Family descriptors must define:
  - family name
  - whether module/block specs are legal
  - supported detail keys
  - value kind for each key
  - supported header/default modifiers
  - lazy materialization hook id
  - phrase-consumer compatibility
- Seed the registry with `arena`, `frame`, and `pool` only in this patch.
- Future memory families become usable by the `Memory` head by adding a new descriptor entry plus docs/tests/runtime hooks, not by changing headed-region syntax/HIR/IR shapes.

## `Memory` v1 Detail Surface
- All detail lines use `key = value`.
- Initial key set:
  - `capacity = <int_expr>`
  - `growth = <int_expr>`
  - `recycle = <atom>`
  - `handle = <atom>`
  - `pressure = <atom>`
- Value kinds:
  - `capacity` / `growth`: `Int`
  - `recycle` / `handle` / `pressure`: identifier atoms validated by the family descriptor
- Operational profile:
  - `capacity`, `growth`, and `pressure` are behavior-driving for the current three families in this patch.
  - `recycle` and `handle` are behavior-driving wherever the current family/runtime descriptor exposes a concrete hook; otherwise they are rejected hard for that family.
- Runtime policy effect in v1:
  - elastic pressure expands live budget by `growth`
  - bounded pressure rejects saturation
  - frame recycle policy may reset on saturation
  - pool recycle policy controls whether removed slots are reused before reset
  - handle policy controls whether memory-spec consumers reuse a cached materialized handle or rematerialize fresh handles per use
- Unsupported per-family keys are compile-time errors, never warnings/no-ops.
- The docs patch and shared family registry must enumerate the exact allowed atoms for `arena`, `frame`, and `pool`, and the implementation must use those exact tables.

## Parser / Syntax Changes
- Add a dedicated `MemorySpecDecl` collection to the parsed module summary for module-scope `Memory`.
- Add a reusable `HeadedRegion` syntax struct for statement-form regions and a `ConstructRegionExpr` syntax node for `yield`.
- Add dedicated headed-region line nodes instead of reusing generic `Statement` for body lines. Line parsing is contextual by head.
- Extend raw block handling by parsing:
  - module-scope `Memory ... -...`
  - statement-form `recycle`, `bind`, `construct deliver`, `construct place`, block-scope `Memory`
  - expression-form `construct yield`
- Add trailing modifier parsing with optional payload expressions for:
  - `return`
  - `default`
  - `replace`
  - `skip`
- Preserve existing cleanup-footer ownership rules and make the parser reject ambiguous dedented dash-lines rather than guessing.

## HIR / Frontend / Flow Changes
- Mirror headed-region nodes into HIR, including spans, resolved constructor paths, modifier payload expressions, and per-line kinds.
- Extend HIR render/fingerprint logic and freeze allowlists so headed regions participate in API/build identity.
- Add frontend validation passes per head:
  - head legality and nesting policy
  - line-family legality
  - modifier legality by head and line kind
  - constructor-target legality for `construct`
  - family/key legality for `Memory`
- Extend local-flow tracking so headed-region-established locals become visible to later lines in the same region and survive after fallthrough only when the ride guarantees they exist.
- Add a dedicated memory-spec scope stack parallel to value scope.
- For module-scope `Memory`, resolve qualified spec paths without mixing them into ordinary symbol resolution.
- For `construct`, validate:
  - constructor target exists
  - destination type matches for `place`
  - introduced local type for `deliver`
  - field/payload totality rules
  - `skip` only on `Option[...]` contributions

## Executable IR / Runtime / Backend Changes
- Add first-class executable IR variants for:
  - `RecycleRegion`
  - `BindRegion`
  - `ConstructDeliver`
  - `ConstructPlace`
  - `ConstructYield` in `ExecExpr`
  - `MemorySpecDecl`
- Add a runtime control-flow signal for named owner exits so `recycle` does not fake them through unrelated statement forms.
- Keep headed-region lowering explicit in IR and direct/native lowering instead of re-encoding them as ad hoc statement sequences.
- Add lazy persistent memory-spec handle storage to runtime state:
  - module-scope cache keyed by package/module/spec
  - block-scope cache keyed by scope activation/spec
- Materialize a spec-backed handle on first phrase use, then reuse it for the declared lifetime.
- Update runtime-plan parse/render helpers and AOT/native lowering so the new IR variants remain round-trippable and backend-stable.
- Native direct-lowering support in the current v1 window is explicit but intentionally narrower than the full runtime lane:
  - direct subset: `MemorySpec`, bool-only `recycle`, and `bind require` with `return` / `break` / `continue`
  - runtime-dispatch subset inside native bundles: general `construct`, payload-bearing `recycle` / `bind`, and named owner-exit routing
- Broader native direct lowering for runtime record/variant values and named owner-exit propagation moves to follow-on backend work rather than staying as an implied v1 deliverable.

## Tests And Acceptance
- Add a `headed_regions_v1` matrix lane to the selfhost language matrix.
- Positive fixtures:
  - one workspace covering all four heads
  - module-scope `Memory` in a reusable library/std-like module
  - block-scope `Memory`
  - `recycle` with owner exits
  - `bind` recovery cases
  - `construct` record and payload-variant cases
- Negative fixtures:
  - unknown head
  - missing modifier
  - invalid nesting
  - `recycle` invalid `break` / `continue` context
  - bare `-return` on non-`Result` gate
  - `bind` `preserve` on `let`
  - `bind` `require` under unsupported modifier
  - `construct` duplicate field/payload
  - `construct` `skip` on non-`Option` target
  - `construct` destination/type mismatch
  - invalid memory family
  - invalid memory key
  - unsupported key for family
- Unit coverage:
  - syntax parse tests
  - HIR lowering/render tests
  - frontend type/flow tests
  - IR rewrite/runtime requirement tests
  - runtime execution tests
  - AOT/native direct-lowering parity tests for the supported direct subset plus explicit runtime-dispatch boundary tests for the remaining headed-region shapes

## Assumptions And Defaults
- Arrow-form construct clauses are the v1 spelling because they stay explicit and match existing language use of `->`.
- Bare `-return` remains a special Result-propagation shorthand; explicit `-return <expr>` is the general return form for all heads.
- `deliver` introduces a new binding only because the constructor target is now explicit in the header; no field-name-based type inference is allowed.
- `Memory` remains contract-closed to the current three approved families in this patch, but the shared descriptor layer is mandatory so later families land by descriptor addition rather than feature rewiring.
