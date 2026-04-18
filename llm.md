# Arcana LLM Guide

This file is derived from the Rust rewrite implementation, not from higher-level docs or Arcana corpus examples.

If behavior is unclear, inspect the referenced Rust functions and tests directly. Prefer file paths plus function or test names over line numbers; line numbers drift.

## How To Use This File

- `crates/arcana-syntax/src/lib.rs` tells you what parses.
- `crates/arcana-frontend/src/lib.rs` tells you what semantic restrictions and diagnostics are enforced after parsing.
- `crates/arcana-runtime/src/lib.rs` and `crates/arcana-runtime/src/tests.rs` tell you how the feature actually executes.
- Default debugging order:
  1. parser
  2. frontend
  3. runtime

## Global Rules

- Phrase syntax is a first-class surface in the rewrite. If you are working with call-like Arcana code, inspect qualified phrases first.
- Qualified phrases and memory phrases share the same parser-side top-level arity check in `parse_phrase_args`: at most 3 top-level arguments.
- The 3-arg limit is top-level only. `parse_phrase_args` splits on top-level commas, so nested expressions inside one argument do not count as extra top-level args.
- A trailing comma immediately before the final qualifier is rejected for both qualified phrases and memory phrases.
- Attached blocks are only valid on standalone statement-form qualified or memory phrases.
- When you need the exact error text for a surface rule, syntax tests in `crates/arcana-syntax/src/lib.rs` are often the fastest lookup.

## Qualified Phrases

### What it is

Qualified phrases are the parser's call-like surface. They are parsed by `parse_qualified_phrase`.

### Current surface shape

- Shape: `subject :: args :: qualifier`
- Qualifier kinds accepted by `classify_qualified_phrase_qualifier`:
  - `call`
  - bare method name, like `push`
  - named path qualifier, like `pkg.module.fn`
  - `?`
  - `>`
  - `>>`
  - `await`
  - `weave`
  - `split`
  - `must`
  - `fallback`
- `call`, bare-method, and named-path qualifiers may carry explicit qualifier type args:
  - `subject :: args :: call[T]`
  - `subject :: args :: method[T]`
  - `subject :: args :: pkg.fn[T]`
- `call` is a real parser qualifier kind, and frontend/runtime still resolve the callable from the subject.
- Named arguments are supported by `parse_phrase_args`.
- Attached blocks are parsed by `parse_header_attachments`.

### Hard limits / rejections

- More than 3 top-level args is rejected by `parse_phrase_args`.
- A trailing comma before the final qualifier is rejected by `parse_phrase_args`.
- Attached blocks are rejected unless the phrase is a standalone statement-form phrase.
- Attached block entries may only be:
  - `name = expr`
  - chain lines
- Named header entries are only allowed for:
  - bare-method qualifiers
  - `>`
- Named header entries are rejected for:
  - named-path qualifiers
  - `?`
  - `>>`
  - `await`
  - `weave`
  - `split`
  - `must`
  - `fallback`
- `must` accepts zero args and only works on `Option[T]` or `Result[T, Str]`.
- `fallback` accepts exactly one positional fallback arg and only works on `Option[T]` or `Result[T, Str]`.
- Bare-method lookup can fail semantically if the receiver type has ambiguous candidates; inspect `lookup_method_symbol_for_type` and `validate_bare_method_resolution`.

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_qualified_phrase`
  - `parse_phrase_args`
  - `classify_qualified_phrase_qualifier`
  - `parse_header_attachments`
  - `validate_qualified_phrase_attachment_contract`
  - `validate_expr_phrase_contract`
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - `resolve_qualified_phrase_target_symbol`
  - `lookup_method_symbol_for_type`
  - `collect_qualified_phrase_param_exprs`
  - `validate_bare_method_resolution`
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `execute_runtime_apply_phrase`
  - `execute_runtime_named_qualifier_call`
  - `eval_qualifier`
  - `must_unwrap_runtime_value`
  - `fallback_runtime_value`
  - `capture_spawned_phrase_call`
  - `validate_spawned_call_capabilities`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_collects_extended_phrase_qualifier_kinds`
  - `parse_module_rejects_bad_memory_family_trailing_commas_and_phrase_over_arity`
  - `parse_module_rejects_unsupported_attached_dash_footer_forms`
  - `crates/arcana-frontend/src/lib.rs`
  - `check_sources_accept_must_and_fallback_for_option_and_result`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_runs_phrase_await_weave_split_must_and_fallback_qualifiers`
  - `runtime_dynamic_bare_method_fallback_keeps_owner_identity`

### Minimal example

```arc
Node :: :: call
    value = 10
```

## Memory Phrases

### What it is

Memory phrases are parsed by `parse_memory_phrase`. They allocate through a memory family expression and a constructor expression.

Do not confuse these with `Memory ...` specs. Memory phrases allocate values. `Memory` specs declare/configure arenas or buffers.

### Current surface shape

- Shape: `family: arena_expr :> init_args <: constructor`
- Supported families in `parse_memory_phrase`:
  - `arena`
  - `frame`
  - `pool`
  - `temp`
  - `session`
  - `ring`
  - `slab`
- Constructors must be path-like or generic path-like; see `is_memory_constructor_like`.
- `Memory` specs, including statement-form `Memory frame:scratch -alloc`, are parsed by `parse_memory_spec_decl`.

### Hard limits / rejections

- More than 3 top-level init args is rejected by `parse_phrase_args`.
- A trailing comma before `<:` is rejected by `parse_phrase_args`.
- Unknown families are rejected by `MemoryFamily::parse` usage in `parse_memory_phrase` and `parse_memory_spec_decl`.
- Invalid constructor shapes are rejected by `parse_memory_phrase`.
- Attached blocks follow the same standalone-statement restriction as qualified phrases.
- Family-specific details and defaults are runtime materialization behavior, not parser behavior. For those, inspect the runtime match over `MemoryDetailKey` in `build_runtime_memory_spec_materialization`.

### Runtime materialization semantics

- `Memory` detail values are parsed as expressions, not just literals; `MemoryDetailLine.value` is an `Expr`.
- Statement-form `Memory` specs are runtime statements. Block-scope specs enter the current scope at execution time and are materialized when a matching memory phrase resolves them.
- Integer-valued details such as `capacity`, `growth`, `page`, and `window` are evaluated through `eval_expr` during runtime materialization, so a block-scope spec can depend on locals computed earlier in the same flow.
- Handle reuse is family- and policy-sensitive:
  - stable handle policy reuses the same materialized allocator handle for that spec identity
  - unstable handle policy rematerializes a fresh allocator on each resolution
- `reset` clears allocator contents, but it does not rebuild the spec policy for an already materialized stable handle.
- Do not assume every family supports unstable handles. Current approved surface keeps `session.handle` and `slab.handle` stable-only; dynamic rematerialization is mainly relevant for families such as `arena` and `pool`.

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `MemoryDetailLine`
  - `parse_memory_phrase`
  - `parse_memory_spec_decl`
  - `parse_phrase_args`
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `memory_family_from_text`
  - `build_runtime_memory_spec_materialization`
  - `eval_expr` in `build_runtime_memory_spec_materialization`
  - `ParsedStmt::MemorySpec`
  - `materialize_runtime_arena_spec_hook`
  - `materialize_runtime_frame_spec_hook`
  - `materialize_runtime_pool_spec_hook`
  - `materialize_runtime_temp_spec_hook`
  - `materialize_runtime_session_spec_hook`
  - `materialize_runtime_ring_spec_hook`
  - `materialize_runtime_slab_spec_hook`
  - runtime memory intrinsics, including:
    - `frame_alloc`
    - `pool_alloc`
    - `pool_compact`
    - `temp_alloc`
    - `session_alloc`
    - `ring_window_read`
    - `ring_window_edit`
    - `slab_alloc`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_rejects_bad_memory_family_trailing_commas_and_phrase_over_arity`
  - `parse_module_collects_chain_collection_and_memory_expressions`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_runs_memory_views_and_new_family_workspace`
  - `execute_main_runs_memory_phrase_attachment_routines`
  - `execute_main_memory_specs_apply_runtime_policies`
  - `ring_window_read_tracks_live_ring_updates`
  - `ring_window_edit_writes_through_to_ring_buffer`
  - `pool_compact_rejects_live_reference_views`

### Minimal example

```arc
arena: arena_nodes :> 21 <: make_node
```

## Chain Phrases

### What it is

Chain phrases are parsed by `parse_chain_expression`. They build a staged call pipeline.

### Current surface shape

- Intro forms:
  - `style :=> ...`
  - `style :=< ...`
- Supported styles from `validate_chain_style`:
  - `forward`
  - `lazy`
  - `parallel`
  - `async`
  - `plan`
  - `broadcast`
  - `collect`
- Bound stage args use `with (...)`; see `split_chain_with_args` and `parse_chain_bind_args`.
- Valid stage expressions are intentionally narrow; see `is_chain_stage_expr`:
  - path
  - member access over a valid path-like stage
  - generic apply over a valid stage

### Hard limits / rejections

- `reverse` is not a valid style token. `validate_chain_style` explicitly tells you to use `<style> :=<` with reverse connectors instead.
- Reverse-introduced chains are only allowed for styles accepted by `chain_style_supports_reverse_introducer`.
- Reverse connectors are only allowed for styles accepted by `chain_style_supports_reverse_connectors`.
- Forward-introduced chains must begin with a forward `=>` segment.
- Forward-introduced chains allow at most one direction change.
- Malformed `with (...)` clauses are rejected by `split_chain_with_args`.
- `split` remains conservative for stages whose resolved callable needs `edit` parameters or `edit` intrinsic arguments; `parallel` may route `edit`-capable stages through the task substrate instead. Inspect `reject_edit_chain_stage_call` and `validate_spawned_call_capabilities`.
- Current runtime behavior in `eval_runtime_chain_expr`:
  - `forward` is a directional serial pipeline in normalized order
  - `async` is a directional pipeline that auto-awaits task/thread results between stages
  - `parallel` spawns all downstream stages first, then awaits them in normalized order
  - `broadcast` is sequential same-input fanout returning a `List`
  - `collect` is a directional pipeline that returns downstream outputs in normalized order, excluding the initial seed/input
  - `plan` validates the chain contract, evaluates only the seed/input expression, skips downstream stage execution, and returns the original input unchanged
  - `lazy` returns a deferred chain value that executes once when forced

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `validate_chain_style`
  - `chain_style_supports_reverse_introducer`
  - `chain_style_supports_reverse_connectors`
  - `parse_chain_expression`
  - `parse_chain_steps`
  - `tokenize_chain_steps`
  - `split_chain_with_args`
  - `parse_chain_bind_args`
  - `is_chain_stage_expr`
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - `validate_chain_step_semantics`
  - `validate_chain_stage_semantics`
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `normalized_chain_indices`
  - `build_runtime_call_args_from_chain_stage`
  - `reject_edit_chain_stage_call`
  - `execute_runtime_chain_stage`
  - `spawn_runtime_chain_stage`
  - `eval_runtime_chain_expr`
  - `drive_runtime_lazy`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_collects_mixed_and_bound_chain_steps`
  - `parse_module_rejects_invalid_chain_styles`
  - `crates/arcana-frontend/src/lib.rs`
  - `check_path_handles_mixed_chain_package`
  - `check_path_handles_bound_chain_workspace`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_runs_chain_expressions_with_parallel_fanout`
  - `execute_main_runs_collect_broadcast_plan_and_async_chain_styles`
  - `execute_main_forces_lazy_chain_once_and_skips_unused_values`

### Minimal example

```arc
forward :=> stage.seed with (seed) => stage.inc <= stage.dec <= stage.emit
```

## Tuples

### What it is

Tuples are 2- and 3-element only in current v1 and use `HirTypeKind::Tuple`. Tuple destructuring support is exact-shape only.

### Current surface shape

- Tuple type syntax:
  - `(A, B)`
  - `(A, B, C)`
- Tuple literal syntax:
  - `(a, b)`
  - `(a, b, c)`
- Tuple field access: `.0`, `.1`, and `.2`
- Exact recursive tuple destructuring is supported in:
  - `let (left, right) = pair`
  - `let (first, second, third) = triple`
  - `for (left, right) in values:`
  - `for (first, second, third) in values:`
- Nested tuples are valid.

### Hard limits / rejections

- Tuple types must have exactly 2 or 3 elements.
- Tuple field selectors beyond `.2` are rejected.
- Tuple destructuring is not supported in parameter lists.
- Tuple `match` patterns are not supported.
- Tuple field assignment is not supported.
- 4+ tuples are out of scope.

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - tuple parsing in `parse_expression`
  - `is_valid_tuple_binding_pattern`
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - `infer_iterable_binding_type`
  - `parse_binding_pattern`
  - `collect_typed_binding_pattern_entries`
  - `bind_pattern_into_scope`
- IR/runtime lowering:
  - `crates/arcana-ir/src/lib.rs`
  - `parse_binding_pattern`
  - `lower_exec_stmt_block`
  - `lower_exec_stmt_resolved`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_accepts_exact_pair_destructuring_in_let_and_for_statements`
  - `crates/arcana-frontend/src/lib.rs`
  - `check_sources_accept_tuple_destructuring_in_let_and_for`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_runs_tuple_destructuring_in_let_and_for`

### Minimal example

```arc
let pair = (1, 2)
let (left, right) = pair
```

## Value Surface

### What it is

Arcana now has a broader first-class value surface than the older `record + Int/Bool/Str` subset. `record` remains semantic data; `struct`, `union`, and nominal fixed `array` are the current layout-bearing value kinds.

### Current surface shape

- Declaration kinds:
  - `record`
  - `struct`
  - `union`
  - `array Name[Elem, Len]:`
- `record` keeps constructor sugar only:
  - `RecordType :: named_fields :: call`
- record/struct constructor sugar may omit `Option[T]` fields, and `Type :: :: call` is valid when no required fields remain.
- `struct` is constructor-callable and may also be value-callable through explicit callable contracts.
- `array` is constructor-callable.
- `union` is a real declaration kind, but it is not callable.
- Region heads now include:
  - `record yield` / `record deliver` / `record place`
  - `struct yield` / `struct deliver` / `struct place`
  - `union yield` / `union deliver` / `union place`
  - `array yield` / `array deliver` / `array place`
  - `construct yield` / `construct deliver` / `construct place`
- `record` remains distinct from `struct`; do not treat them as aliases in analysis or rewrites.
- `array` is nominal fixed-length value storage, not the same thing as builtin `Array[T]`.
- Callable struct contracts are:
  - `CallableRead0[Out]`
  - `CallableEdit0[Out]`
  - `CallableTake0[Out]`
  - `CallableRead[Args, Out]`
  - `CallableEdit[Args, Out]`
  - `CallableTake[Args, Out]`
- Matching lang items are:
  - `call_contract_read0`
  - `call_contract_edit0`
  - `call_contract_take0`
  - `call_contract_read`
  - `call_contract_edit`
  - `call_contract_take`
- Packed callable struct args follow:
  - `f :: a :: call` => `Args = A`
  - `f :: a, b :: call` => `Args = (A, B)`
  - `f :: a, b, c :: call` => `Args = (A, B, C)`
- Numeric builtin surface now includes:
  - `Int`, `Bool`
  - `I8/U8`
  - `I16/U16`
  - `I32/U32`
  - `I64/U64`
  - `ISize/USize`
  - `F32/F64`
- Decimal float literals are supported:
  - unsuffixed decimal literals default to `F64`
  - `f32` / `f64` suffixes are supported
- Bitfields are declared on `struct` fields with:
  - `name: U32 bits 3`

### Hard limits / rejections

- `union` construction, reads, and writes require an active `#unsafe["trace.id"]` foreword in scope.
- `union` is not callable in the current surface.
- `record` values are not callable; record `:: call` remains constructor sugar only.
- `struct` value dispatch is `:: call` only and is struct-only in this phase.
- `array` is constructor-callable only, not value-callable.
- `array yield` is expression-form only in expression position, just like `construct yield` and `record/struct/union yield`.
- callable struct receivers are `read`, `edit`, or `take`; `hold self` callable contracts are rejected in this phase.
- packed callable args are always `take args`.
- Bitfields are currently allowed only on `struct`.
- Bitfield bases must be fixed-width integer types.
- Floats do not support `%`, shifts, or bitwise operators.
- There are no implicit mixed signed/unsigned conversions.
- Current widening is same-signed only.
- `I128` / `U128` are not part of the current crate-side surface.

### Rust lookup

- Syntax:
  - [crates/arcana-syntax/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-syntax/src/lib.rs)
  - `parse_array_symbol`
  - `parse_array_signature`
  - `parse_headed_region_statement`
  - `parse_record_yield_expression`
  - `parse_array_yield_expression`
  - float literal parsing in `parse_expression`
  - `builtin_type_info`
- Frontend:
  - [crates/arcana-frontend/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-frontend/src/lib.rs)
  - struct/union/array symbol/type validation in the main semantic pass
  - unsafe gating around union use
  - bitfield validation and struct layout checks
  - numeric widening/conversion validation
- Runtime:
  - [crates/arcana-runtime/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-runtime/src/lib.rs)
  - struct/array evaluation paths
  - struct bitfield layout helpers
  - float/fixed-width numeric execution
- Representative tests:
  - [crates/arcana-syntax/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-syntax/src/lib.rs)
  - `parse_module_accepts_member_access_plus_decimal_float_literal`
  - `parse_module_accepts_nested_qualified_phrase_inside_named_arg`
  - [crates/arcana-frontend/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-frontend/src/lib.rs)
  - `check_path_accepts_struct_array_float_and_bitfield_surface`
  - `check_path_rejects_union_usage_without_unsafe`
  - `check_path_accepts_union_usage_with_unsafe`
  - [crates/arcana-runtime/src/tests.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-runtime/src/tests.rs)
  - `execute_main_runs_value_surface_struct_array_and_float_routines`
  - `execute_main_runs_struct_bitfield_layout_semantics`

### Minimal examples

```arc
struct Vec2:
    x: F32
    y: F32

let point = Vec2 :: x = 1.5f32, y = 2.5f32 :: call
```

```arc
array Trio[Int, 3]:

let ys = array yield Trio -return 0
    [0] = 1
    [1] = 2
    [2] = 3
```

## Access Capabilities

### What it is

Arcana now uses one explicit capability model for place access and ownership flow. The current general access modes are `read`, `edit`, `take`, and `hold`.

### Current surface shape

- Parameter/access modes:
  - `read`
  - `edit`
  - `take`
  - `hold`
- Capability expressions:
  - `&read x`
  - `&edit x`
  - `&take x`
  - `&hold x`
- Capability type forms:
  - `&read[T, 'a]`
  - `&edit[T, 'a]`
  - `&take[T, 'a]`
  - `&hold[T, 'a]`
- Explicit borrowed-slice forms are supported:
  - `&read x[a..b]`
  - `&edit x[a..b]`
- Unary `*` is the capability-use surface:
  - deref/project through `&read` / `&edit`
  - redeem `&take`
  - temporarily project through `&hold`
- `reclaim x` is the explicit hold-ending statement form.
- Owner exit retention is now spelled `retain [...]`; do not describe the owner surface as `hold [...]`.
- Plain `hold` params are valid call-boundary modes and are ephemeral for the duration of the call unless an explicit `&hold[...]` capability is created and kept.

### Hard limits / rejections

- Old canonical borrow spellings are no longer the language contract:
  - bare `&x`
  - bare `&mut x`
  - `&'a T`
  - `&'a mut T`
- `&read[...]` is duplicable/shared.
- `&edit[...]`, `&take[...]`, and `&hold[...]` are linear.
- `&edit` cannot be created from an immutable local.
- `&take x` reserves `x` immediately; direct use after creation is rejected.
- `&hold x` suspends direct use of `x` until explicit `reclaim`.
- Unreclaimed `&hold[...]` capability locals are rejected at scope exit.
- `reclaim` must target a local `&hold[...]` capability binding, not an arbitrary expression.
- Assignment through `*` is only allowed for `&edit[...]` and `&hold[...]`.
- String slices are read-only; `&edit x[a..b]` is rejected for `Str`.
- Borrowed slices require contiguous backing; `List` does not currently satisfy that surface.

### Rust lookup

- Syntax:
  - [crates/arcana-syntax/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-syntax/src/lib.rs)
  - `parse_capability_unary`
  - `parse_statement`
  - `StatementKind::Reclaim`
- HIR:
  - [crates/arcana-hir/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-hir/src/lib.rs)
  - `HirUnaryOp::CapabilityRead`
  - `HirUnaryOp::CapabilityEdit`
  - `HirUnaryOp::CapabilityTake`
  - `HirUnaryOp::CapabilityHold`
  - `HirStatementKind::Reclaim`
- Frontend:
  - [crates/arcana-frontend/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-frontend/src/lib.rs)
  - capability conflict validation in the unary-op checks
  - `validate_reclaim_expr_semantics`
  - `reclaim_expr_hold_token`
  - `validate_unreclaimed_hold_tokens`
  - writable-capability checks for assignment-through-deref
- Runtime:
  - [crates/arcana-runtime/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-runtime/src/lib.rs)
  - runtime reference-mode selection from unary capability ops
  - `reclaim_hold_capability_root_local`
  - `reclaim_held_target_local`
  - `redeem_take_reference`
  - runtime hold/take scope validation
- Representative tests:
  - [crates/arcana-syntax/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-syntax/src/lib.rs)
  - `parse_module_collects_capability_and_deref_expressions`
  - `parse_module_collects_reclaim_statement`
  - `parse_module_collects_deferred_reclaim_statement`
  - [crates/arcana-frontend/src/lib.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-frontend/src/lib.rs)
  - `check_path_accepts_hold_capability_with_reclaim`
  - `check_path_accepts_plain_hold_param_for_call_duration_only`
  - `check_path_rejects_use_after_take_capability_creation`
  - `check_path_rejects_reclaim_of_nonlocal_hold_expression`
  - [crates/arcana-runtime/src/tests.rs](C:/Users/Weaver/Documents/GitHub/Arcana/crates/arcana-runtime/src/tests.rs)
  - `execute_main_runs_hold_capability_reclaim_flow`
  - `execute_main_runs_deferred_hold_reclaim_flow`
  - `execute_main_runs_take_capability_once`
  - `execute_main_runs_plain_hold_param_for_call_duration_only`

### Minimal examples

```arc
let x_ref = &read local_x
let y_cap = &edit local_y
let sum = *x_ref + *y_cap
```

```arc
let held = &hold x
*held = 2
reclaim held
```

## Headed Regions

### What it is

Headed regions are parsed by `parse_headed_region_statement`. They are region-form statements with specialized inner lines or contributions.

### Current surface shape

- Parsed heads:
  - `recycle`
  - `bind`
  - `construct`
  - `record`
  - `struct`
  - `union`
  - `array`
  - statement-form `Memory`
- `construct yield`, `record yield`, `struct yield`, `union yield`, and `array yield` are expression-form.
- `construct deliver` and `construct place` are statement-form region heads.
- `record deliver` and `record place` are statement-form region heads.
- `struct deliver` and `struct place` are statement-form region heads.
- `union deliver` and `union place` are statement-form region heads.
- `array deliver` and `array place` are statement-form region heads.
- Statement-form `Memory` uses `parse_memory_spec_decl`, not `parse_memory_phrase`.

### Hard limits / rejections

- `recycle` and `bind` require indented region bodies.
- `construct yield` is rejected in statement position.
- `record yield`, `struct yield`, `union yield`, and `array yield` are rejected in statement position.
- Expression position only allows `construct yield`, `record yield`, `struct yield`, `union yield`, and `array yield`; `parse_construct_yield_expression`, `parse_record_yield_expression`, and `parse_array_yield_expression` reject other completions there.
- Nested headed regions are rejected in v1; frontend tracks headed-region depth and rejects a headed region that appears inside another headed region.
- Frontend headed-region validation is extensive. Representative constraints:
  - `recycle -break` and `recycle -continue` only make sense inside loops
  - named recycle exits must be active and unambiguous
  - `bind -default`, `bind -preserve`, and `bind -replace` are restricted to the appropriate gate forms
  - `bind -break` and `bind -continue` are restricted to `require <expr>` lines
  - `construct place` target type must match constructor result type
  - `record` / `struct` / `union` targets must resolve to the matching nominal kind
  - `record place`, `struct place`, and `union place` target types must match the region result type
  - `record` / `struct` / `union ... from ...` only copy same-name, exact-type fields from the base
  - `array ... from ...` requires the same nominal array type
  - array region indices are compile-time integer literals in-range

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_headed_region_statement`
  - `parse_recycle_line`
  - `parse_bind_line`
  - `parse_construct_region`
  - `parse_construct_yield_expression`
  - `parse_record_region`
  - `parse_record_yield_expression`
  - `parse_array_region`
  - `parse_array_yield_expression`
  - `parse_memory_spec_decl`
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - `validate_recycle_modifier_semantics`
  - `validate_bind_modifier_semantics`
  - `validate_bind_fallback_type_semantics`
  - `validate_bind_refinement_stability_semantics`
  - `validate_construct_modifier_semantics`
  - `validate_construct_contribution_semantics`
  - `validate_construct_region_semantics`
  - `validate_record_region_semantics`
  - `validate_array_region_semantics`
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `resolve_named_owner_exit_target`
  - `apply_explicit_owner_exit`
  - `eval_record_region_value`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_collects_headed_regions_v1_shapes`
  - `crates/arcana-frontend/src/lib.rs`
  - `check_sources_rejects_headed_region_semantic_violations`
  - `check_path_accepts_same_region_headed_bindings_and_matching_construct_place`
  - `check_path_accepts_record_headed_regions_with_base_copy`
  - `check_path_accepts_struct_array_float_and_bitfield_surface`
  - `check_path_accepts_union_usage_with_unsafe`
  - `crates/arcana-ir/src/lib.rs`
  - `lower_workspace_package_with_resolution_collects_record_copied_fields`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_consumes_named_recycle_owner_exits`
  - `execute_main_runs_bind_recovery_regions`
  - `execute_main_runs_bind_require_loop_exits`
  - `execute_main_construct_regions_preserve_direct_values_and_payload_acquisition`
  - `execute_main_runs_record_headed_regions_with_base_copy`
  - `execute_main_runs_record_headed_regions_with_cross_record_lift`
  - `execute_main_runs_value_surface_struct_array_and_float_routines`

### Minimal example

```arc
bind -return 0
    let value = Result.Ok[Int, Str] :: 1 :: call
```

```arc
let next = record yield Widget from base -return 0
    ready = true
```

## Cleanup Footers

### What it is

Cleanup footers are post-block footer lines parsed by `parse_cleanup_footer_entry`. The only currently supported footer is `-cleanup`.

### Current surface shape

- Bare cleanup:
  - `-cleanup`
- Targeted cleanup with optional explicit handler:
  - `-cleanup[target = value]`
  - `-cleanup[target = value, handler = cleanup.path]`
- Bare cleanup and targeted cleanup without `handler` rely on the default cleanup contract; inspect `resolve_cleanup_contract_trait_path` and `binding_supports_default_cleanup_contract`.
- Cleanup footers can attach only to owning headers/statements:
  - symbols: `fn`, `behavior`, `system`
  - statements: `if`, `while`, `for`, or expression statements whose expression has an attached block

### Hard limits / rejections

- Footer position only accepts `-cleanup`; `-defer` and other `-name` forms are rejected here.
- Cleanup footer lines cannot own nested blocks.
- Footer payload fields must be named fields.
- `target`, if present, must be a binding name.
- `handler`, if present, must be a named callable path and requires `target`.
- Frontend validates target reachability and cleanup capability:
  - target must be available in the owning header scope
  - shadowed targets can be ambiguous
  - activated cleanup targets cannot be reassigned afterward
- Frontend and runtime both validate explicit handlers:
  - callable symbol only
  - not async
  - exactly one parameter
  - parameter mode must be `take`
  - return type must be `Result[Unit, Str]`

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_cleanup_footer_entry`
  - `collect_following_cleanup_footers`
  - `symbol_can_own_cleanup_footers`
  - `statement_can_own_cleanup_footers`
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - `has_bare_cleanup_rollup`
  - `push_cleanup_footer_candidate`
  - `collect_cleanup_footer_candidates_recursive`
  - `cleanup_target_supports_default_cleanup_contract`
  - `validate_cleanup_footer_targets`
  - `resolve_cleanup_contract_trait_path`
  - `binding_supports_default_cleanup_contract`
  - `should_activate_cleanup_binding`
  - `activate_current_cleanup_binding`
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `validate_runtime_cleanup_footer_handlers`
  - `validate_runtime_cleanup_footer_handlers_in_statements`
  - `push_runtime_cleanup_footer_frame`
  - `pop_runtime_cleanup_footer_frame`
  - `activate_runtime_cleanup_footer_binding`
  - `update_runtime_cleanup_footer_binding_value`
  - `resolve_cleanup_footer_handler_callable_path`
  - `validate_cleanup_footer_handler_routine_plan`
  - `execute_cleanup_footers`
  - `finish_runtime_cleanup_footers`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_collects_cleanup_footers`
  - `parse_module_rejects_ownerless_cleanup_footers`
  - `parse_module_rejects_unsupported_attached_dash_footer_forms`
  - `crates/arcana-frontend/src/lib.rs`
  - `check_path_accepts_cleanup_footer_package`
  - `check_path_rejects_unknown_cleanup_footer_target`
  - `check_path_rejects_reassigned_cleanup_footer_target`
  - `check_path_rejects_async_cleanup_footer_handler`
  - `check_path_rejects_non_callable_cleanup_footer_handler`
  - `check_path_rejects_wrong_arity_cleanup_footer_handler`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_runs_cleanup_footers_on_loop_exit_and_try_propagation`
  - `execute_main_cleanup_footers_refresh_subject_value_after_mutation`
  - `execute_main_bare_cleanup_footer_covers_whole_routine_scope`
  - `execute_main_bare_cleanup_footer_covers_nested_scope_bindings`
  - `execute_main_cleanup_footer_targets_nested_scope_binding`
  - `execute_main_manual_routine_cleanup_footers_run_after_defers`

### Minimal example

```arc
fn main() -> Int:
    let value = Box :: value = 1 :: call
    return 0
-cleanup[target = value, handler = cleanup]
```

## Defer

### What it is

`defer` is a statement-form deferred-work surface parsed directly in `parse_statement`.

### Current surface shape

- Shape: `defer <expr>`
- The deferred expression is stored as `StatementKind::Defer`.

### Hard limits / rejections

- There is no `-defer` footer form. Footer position accepts only `-cleanup`.
- Runtime ordering matters:
  - scope defers are executed by `run_scope_defers`
  - cleanup footers run afterward
- Runtime also keeps deferred spawned work until join/await points where required; inspect `execute_deferred_work` and related tests.

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_statement`
  - the `StatementKind::Defer` parse branch
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - defer statements flow through the same expression validation passes as expression statements; see `HirStatementKind::Defer` handling
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `execute_deferred_work`
  - `run_scope_defers`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - defer parsing in the syntax test block around `StatementKind::Defer`
  - `parse_module_rejects_unsupported_attached_dash_footer_forms`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_manual_routine_cleanup_footers_run_after_defers`
  - `execute_main_defers_non_call_spawned_values_until_join`

### Minimal example

```arc
defer io.print[Str] :: "bye" :: call
```

## Objects, Owners, and Owner Activation

### What it is

Objects and owners are the current owner-state surface. Owners are parsed by `parse_owner_symbol` and activated through qualified phrases that resolve to owner symbols.

### Current surface shape

- Object declaration:
  - `obj Counter:`
- Owner declaration:
  - `create Session [Counter] scope-exit:`
  - `create Session [Counter] context: SessionCtx scope-exit:`
- Owner exits from `parse_owner_exit_decl`:
  - `exit when <expr>`
  - `<name>: when <expr>`
- Optional retain list:
  - `retain [Counter]`
- Availability attachments are bare path lines parsed by `parse_availability_attachment`.
- Owner activation is frontend-recognized only for qualified phrases whose qualifier is `call` and whose subject resolves to an owner; see `resolve_owner_activation_expr`.

### Hard limits / rejections

- Owners must declare at least one scope-exit; see frontend owner validation.
- Owner exit conditions must type-check as boolean.
- Owner activation does not support named arguments.
- Owner activation accepts at most one context argument.
- If an owner does not declare `context: ...`, providing an activation context is rejected.
- If an owner declares `context: ...`, activation requires exactly one arg of that type.
- Owned object lifecycle hooks may either omit context or use that exact owner context type.
- Availability attachments are only valid when the following symbol/statement can own availability:
  - symbols: `fn`, `behavior`, `system`
  - statements: `if`, `while`, `for`, or expression statements with attached phrase blocks
- Frontend availability resolution only accepts owners or objects.
- Runtime lifecycle hooks must preserve `self` and return `Unit`; inspect `execute_owner_object_lifecycle_hook`.

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_owner_symbol`
  - `parse_owner_signature`
  - `parse_owner_body`
  - `parse_owner_exit_decl`
  - `parse_owner_hold_list`
  - `parse_availability_attachment`
  - `symbol_can_own_availability`
  - `statement_can_own_availability`
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - `resolve_owner_activation_expr`
  - `validate_owner_activation_context`
  - `collect_owner_activation_context_types`
  - `resolve_available_owner_binding`
  - `apply_availability_attachments_to_scope`
  - owner validation around required scope exits in the `HirSymbolBody::Owner` checks
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `lower_owner`
  - `execute_owner_object_lifecycle_hook`
  - `owner_object_root_value`
  - `evaluate_owner_exit_checkpoints`
  - `apply_explicit_owner_exit`
  - `activate_owner_scope_binding`
  - `runtime_reset_owner_exit_memory_specs_in_scopes`
  - `runtime_reset_owner_exit_module_memory_specs`
- Representative tests:
  - `crates/arcana-frontend/src/lib.rs`
  - `check_sources_accepts_object_owner_activation_flow`
  - `check_sources_accepts_object_only_attached_owner_flow`
  - `check_sources_rejects_owner_without_scope_exit_clause`
  - `check_sources_rejects_non_bool_owner_exit_condition`
  - `check_sources_rejects_owner_activation_with_wrong_context_type`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_owner_multi_exit_uses_first_matching_exit`
  - `execute_main_rejects_stale_owner_access_after_exit`
  - `execute_main_runs_owner_init_hook_with_activation_context`
  - `execute_main_runs_owner_resume_hook_with_activation_context`
  - `execute_main_runs_owner_activation_with_explicit_context_clause`
  - `execute_main_runs_attached_owner_helper_with_active_state`
  - `execute_main_runs_object_only_attached_helper_through_unattached_helper_chain`
  - `execute_main_runs_late_attached_owner_block_with_active_state`
  - `execute_main_rejects_owner_object_init_without_required_context`

### Minimal example

```arc
obj Counter:
    value: Int

create Session [Counter] scope-exit:
    done: when false retain [Counter]

Session
Counter
fn main() -> Int:
    let active = Session :: :: call
    return 0
```

## Native Bindings, Shackle, and `arcana_winapi`

### What it is

Binding products are the current package-owned foreign seam for library packages that need host APIs.

- `crates/arcana-cabi` owns the foreign contract.
- Binding packages declare a default native product in `book.toml` with:
  - `role = "binding"`
  - `producer = "arcana-source"`
  - `contract = "arcana.cabi.binding.v1"`
- The generated binding product is self-hosted from package source.
- The transitional handwritten Rust bridge crate `crates/arcana-winapi` is gone. The current first-party Win32 binding lane lives under `grimoires/arcana/winapi`.

### Current surface shape

- Package-owned binding surface:
  - `export shackle fn current_process_id() -> Int = host.raw.kernel32.GetCurrentProcessId`
  - inline callback form:
    - `native callback report(read code: Int) -> Int = app.callbacks.handle_report`
  - typed callback form:
    - `native callback proc: arcana_winapi.raw.user32.WNDPROC = app.callbacks.handle_proc`
- Binding-owning packages may declare `shackle` items:
  - `shackle type`
  - `shackle struct`
  - `shackle union`
  - `shackle flags`
  - `shackle const`
  - `shackle import fn`
  - `shackle callback`
  - `shackle fn`
  - `shackle thunk`
- Exported `shackle` items form the public raw dependency surface:
  - type surface:
    - `hostapi.raw.types.HWND`
  - callable surface:
    - `hostapi.raw.kernel32.GetCurrentProcessId :: :: call`
  - const surface:
    - `hostapi.raw.constants.MAGIC`
  - callback-type surface:
    - `hostapi.raw.user32.WNDPROC`
- `arcana_winapi` currently exposes:
  - `arcana_winapi.raw.*`
  - no package-visible backend/helper/wrapper layer
- The raw leaves under `grimoires/arcana/winapi/src/raw/*.arc` are checked-in generated files.
- Handwritten boundary files stay:
  - `grimoires/arcana/winapi/src/book.arc`
  - `grimoires/arcana/winapi/src/raw.arc`
- Edit `grimoires/arcana/winapi/generation/*` and rerun `scripts/dev/regenerate-winapi-raw.ps1`; do not hand-edit generated raw leaves.
- The pinned Windows SDK metadata snapshot is the authority for raw coverage. `windows-sys` is only the parity target for exposed names/signatures.
- The current raw module set includes:
  - desktop/kernel families:
    - `callbacks`, `constants`, `kernel32`, `user32`, `gdi32`, `dwmapi`, `shcore`, `shell32`, `imm32`
  - COM/graphics/text families:
    - `ole32`, `combase`, `dxgi`, `d3d12`, `dwrite`, `d2d1`, `wic`
  - audio families:
    - `mmdeviceapi`, `audioclient`, `audiopolicy`, `endpointvolume`, `avrt`, `mmreg`, `ksmedia`, `propsys`, `xaudio2`, `x3daudio`
  - shared layout families:
    - `types`
- `arcana_winapi` does not expose a public helper, wrapper, or handle surface.
- There is no package-visible backend module layer under `arcana_winapi` anymore.
- If `winapi` regrows backend/helper/wrapper module namespaces or typed handle modules, that is drift. Remaining implementation support belongs under `shackle`/private support, not as Arcana package modules.
- Current explicitly classified bootstrap-ish survivors:
  - `audiopolicy` session policy constants, `avrt` registration imports, and `xaudio2.XAudio2CreateWithVersionInfo` are ordinary generated raw coverage
  - `x3daudio.X3DAudioInitialize` is the initial raw-shim exception-manifest entry
- In HIR, exported `shackle import fn`, exported `shackle fn`, and exported `shackle const` are projected into visible symbol surface so dependent packages can call/read them through ordinary path resolution.
- Current binding CABI semantics are symmetric for imports and callbacks:
  - same param metadata model
  - callbacks use `out_write_backs` plus `out_result`
  - `Str`/`Bytes` outputs use owned-buffer helpers

### Hard limits / rejections

- `shackle` declarations are rejected unless the package declares an `arcana-source` binding native product.
- `binding_support_crate` is no longer valid in manifests.
- `native callback` declarations:
  - do not support type parameters
  - do not support where clauses
  - typed callback refs must resolve to a visible `shackle callback` path
- Availability attachments and forewords cannot target:
  - `native callback`
  - `shackle`
- Exported `shackle` items that collide with an existing symbol name in the same module are rejected during HIR lowering.
- The public binding transport model is the revised raw-capable binding v1 model:
  - scalar tags: `Int`, `Bool`, `I8/U8`, `I16/U16`, `I32/U32`, `I64/U64`, `ISize/USize`, `F32/F64`
  - view/owned tags: `Str`, `Bytes`
  - handle/control tags: `Opaque`, `Unit`
  - raw layout tag: `Layout`
- Stable raw layout ids now flow through binding metadata and generated binding products for:
  - aliases
  - structs
  - unions
  - fixed arrays
  - flags/enums
  - callbacks / function-pointer signatures
  - COM-style interface pointers
  - named bitfields
- Pointer/function-pointer semantics remain a binding/`shackle` concern. Arcana still does not have a general pointer language model.

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_native_fn_decl`
  - `parse_native_callback_decl`
  - `parse_shackle_decl`
  - `parse_shackle_function_decl`
  - `collect_shackle_surface`
- Frontend:
  - `crates/arcana-frontend/src/lib.rs`
  - `validate_module_shackle_semantics`
  - typed callback checks in the `native callback` validation path
  - `crates/arcana-frontend/src/type_resolve.rs`
  - `crates/arcana-frontend/src/type_validate.rs`
- HIR/IR:
  - `crates/arcana-hir/src/lib.rs`
  - `extend_symbols_with_exported_shackle_callables`
  - `projected_shackle_binding_name`
  - `crates/arcana-ir/src/lib.rs`
  - typed callback lowering and `shackle_decls` lowering
- Packaging/AOT:
  - `crates/arcana-package/src/lib.rs`
  - manifest validation for binding products and `binding_support_crate` rejection
  - `crates/arcana-package/src/distribution.rs`
  - `crates/arcana-aot/src/instance_product.rs`
  - self-hosted binding-product generation
- Runtime:
  - `crates/arcana-runtime/src/native_product_loader.rs`
  - binding product activation and callback registration
  - `crates/arcana-runtime/src/lib.rs`
  - runtime callback execution and projected const-path evaluation
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_handles_typed_native_callbacks_and_shackle_declarations`
  - `crates/arcana-frontend/src/lib.rs`
  - `check_path_rejects_shackle_without_binding_product`
  - `check_path_accepts_typed_native_callback_from_dependency_shackle_callback`
  - `check_path_accepts_typed_native_callback_from_dependency_audio_shackle_callback`
  - `check_path_accepts_dependency_shackle_types_in_type_surface`
  - `check_path_accepts_dependency_shackle_import_fns_and_consts`
  - `crates/arcana-ir/src/lib.rs`
  - `lower_workspace_package_with_resolution_carries_shackle_decls_and_typed_callbacks`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_runs_arcana_winapi_binding_smoke`
  - `execute_main_reports_arcana_winapi_binding_errors`
  - `execute_main_runs_dependency_shackle_import_fn_and_const_surface`

### Minimal examples

```arc
native callback window_proc: arcana_winapi.raw.user32.WNDPROC = app.callbacks.handle_window_proc
```

```arc
export shackle import fn GetCurrentProcessId() -> Int = kernel32.GetCurrentProcessId
export shackle const MAGIC: Int = 7

fn main() -> Int:
    let pid = hostapi.raw.kernel32.GetCurrentProcessId :: :: call
    if pid >= 0:
        return hostapi.raw.constants.MAGIC
    return 0
```

## Common LLM Mistakes

- Treating docs or Arcana-side examples as more authoritative than the Rust rewrite.
- Forgetting that qualified and memory phrases are capped at 3 top-level args.
- Emitting a trailing comma before the final phrase qualifier.
- Using attached blocks on non-statement-form phrases.
- Assuming named header entries work on all qualifiers.
- Treating `Memory` specs and memory phrases as the same construct.
- Using tuple destructuring outside `let` and `for`, or expecting tuple `match` patterns/param destructuring.
- Emitting `-defer` as if it were a valid footer.
- Forgetting that owner activation is a `call`-qualified phrase and that `context:` owners require exactly one positional context arg.
- Suggesting `binding_support_crate` or `crates/arcana-winapi` as if the Win32 binding seam were still owned by a handwritten Rust bridge.
- Forgetting that `shackle` is binding-package-only and that dependency-facing raw Win32 surface now comes from exported `shackle` items under `arcana_winapi.raw.*`.
- Treating typed `native callback` refs as arbitrary path types instead of visible `shackle callback` paths.

## Where To Look First

- Parse failure:
  - `crates/arcana-syntax/src/lib.rs`
- Type or semantic error:
  - `crates/arcana-frontend/src/lib.rs`
- Binding surface or raw Win32 API issue:
  - `crates/arcana-syntax/src/lib.rs`
  - `crates/arcana-frontend/src/lib.rs`
  - `crates/arcana-hir/src/lib.rs`
  - `crates/arcana-aot/src/instance_product.rs`
  - `crates/arcana-runtime/src/native_product_loader.rs`
- Runtime behavior, ordering, lifecycle, or memory policy:
  - `crates/arcana-runtime/src/lib.rs`
  - `crates/arcana-runtime/src/tests.rs`

## Known Problems

- `split` still rejects unsafe cross-thread `edit` capture. In `crates/arcana-runtime/src/lib.rs`, `validate_spawned_call_capabilities` keeps thread-boundary `edit` capture conservative until a broader transferable-place law exists.
- Tuple surface is still intentionally narrow beyond the new `let`/`for` destructuring support. 2/3-tuples, `.0`/`.1`/`.2` access, and the lack of parameter destructuring, tuple `match` patterns, tuple field assignment, and 4+ tuples are still current v1 boundaries.
