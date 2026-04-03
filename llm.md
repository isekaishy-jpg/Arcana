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

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_memory_phrase`
  - `parse_memory_spec_decl`
  - `parse_phrase_args`
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `memory_family_from_text`
  - `build_runtime_memory_spec_materialization`
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

Tuples are pair-only in current v1 and use `HirTypeKind::Tuple`. Tuple destructuring support is exact-shape only.

### Current surface shape

- Tuple type syntax: `(A, B)`
- Tuple literal syntax: `(a, b)`
- Tuple field access: `.0` and `.1`
- Exact recursive pair destructuring is supported in:
  - `let (left, right) = pair`
  - `for (left, right) in values:`
- Nested pairs are valid.

### Hard limits / rejections

- Tuple types must have exactly 2 elements.
- Tuple field selectors beyond `.1` are rejected.
- Tuple destructuring is not supported in parameter lists.
- Tuple `match` patterns are not supported.
- Tuple field assignment is not supported.
- 3+ tuples are out of scope.

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

## Headed Regions

### What it is

Headed regions are parsed by `parse_headed_region_statement`. They are region-form statements with specialized inner lines or contributions.

### Current surface shape

- Parsed heads:
  - `recycle`
  - `bind`
  - `construct`
  - statement-form `Memory`
- `construct yield` is expression-form.
- `construct deliver` and `construct place` are statement-form region heads.
- Statement-form `Memory` uses `parse_memory_spec_decl`, not `parse_memory_phrase`.

### Hard limits / rejections

- `recycle` and `bind` require indented region bodies.
- `construct yield` is rejected in statement position.
- Expression position only allows `construct yield`; `parse_construct_yield_expression` rejects other construct completions there.
- Nested headed regions are rejected in v1; frontend tracks headed-region depth and rejects a headed region that appears inside another headed region.
- Frontend headed-region validation is extensive. Representative constraints:
  - `recycle -break` and `recycle -continue` only make sense inside loops
  - named recycle exits must be active and unambiguous
  - `bind -default`, `bind -preserve`, and `bind -replace` are restricted to the appropriate gate forms
  - `bind -break` and `bind -continue` are restricted to `require <expr>` lines
  - `construct place` target type must match constructor result type

### Rust lookup

- Syntax:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_headed_region_statement`
  - `parse_recycle_line`
  - `parse_bind_line`
  - `parse_construct_region`
  - `parse_construct_yield_expression`
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
- Runtime:
  - `crates/arcana-runtime/src/lib.rs`
  - `resolve_named_owner_exit_target`
  - `apply_explicit_owner_exit`
- Representative tests:
  - `crates/arcana-syntax/src/lib.rs`
  - `parse_module_collects_headed_regions_v1_shapes`
  - `crates/arcana-frontend/src/lib.rs`
  - `check_sources_rejects_headed_region_semantic_violations`
  - `check_path_accepts_same_region_headed_bindings_and_matching_construct_place`
  - `crates/arcana-runtime/src/tests.rs`
  - `execute_main_consumes_named_recycle_owner_exits`
  - `execute_main_runs_bind_recovery_regions`
  - `execute_main_runs_bind_require_loop_exits`
  - `execute_main_construct_regions_preserve_direct_values_and_payload_acquisition`

### Minimal example

```arc
bind -return 0
    let value = Result.Ok[Int, Str] :: 1 :: call
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
- Optional hold list:
  - `hold [Counter]`
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
    done: when false hold [Counter]

Session
Counter
fn main() -> Int:
    let active = Session :: :: call
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

## Where To Look First

- Parse failure:
  - `crates/arcana-syntax/src/lib.rs`
- Type or semantic error:
  - `crates/arcana-frontend/src/lib.rs`
- Runtime behavior, ordering, lifecycle, or memory policy:
  - `crates/arcana-runtime/src/lib.rs`
  - `crates/arcana-runtime/src/tests.rs`

## Known Problems

- `split` still rejects unsafe cross-thread `edit` capture. In `crates/arcana-runtime/src/lib.rs`, `validate_spawned_call_capabilities` keeps thread-boundary `edit` capture conservative until a broader transferable-place law exists.
- Tuple surface is still intentionally narrow beyond the new `let`/`for` destructuring support. Pair-only tuples, `.0`/`.1` access, and the lack of parameter destructuring, tuple `match` patterns, tuple field assignment, and 3+ tuples are still current v1 boundaries.
