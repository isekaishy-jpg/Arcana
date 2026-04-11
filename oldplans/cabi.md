# Generic Two-Way Binding CABI With No Throwaway Architecture

## Summary
- Finish the binding foreign boundary in `crates/arcana-cabi`, not in `arcana-runtime`.
- Make binding imports and binding callbacks fully symmetric, including `edit` write-backs, ownership, and metadata semantics.
- Keep this plan generic and downstream-safe: `arcana_winapi`, Lua interop, and binding-product rollout are not implemented here, but the CABI must not block them or force a later redesign.

## Public Interface Changes
- Revise the approved binding docs so the binding contract is owned by `arcana-cabi` and no longer framed around a dedicated Windows Rust leaf.
- Extend `ArcanaCabiBindingCallbackFn` to match import semantics by adding `out_write_backs` alongside `out_result`.
- Keep one write-back slot per declared param row, in declaration order. Params without write-back semantics must return `Unit` in their slot.
- Make binding metadata authoritative for both directions:
  - `source_mode`, `pass_mode`, `input_type`, `write_back_type`, and `return_type` define the contract.
  - `edit` is a real public binding mode, not a proof-only note.
- Keep the transport surface generic and unchanged in kind: `Int`, `Bool`, `Str`, `Bytes`, `Opaque`, and `Unit`.
  - `Str` and `Bytes` inputs use views.
  - `Str` and `Bytes` outputs and write-backs use cabi-owned buffers and cabi-owned free helpers.
  - `Opaque` remains package-owned handle transport only.
- Add canonical Rust-side helper APIs inside `arcana-cabi` for binding metadata validation, slot layout, and owned-buffer decode/free rules so consumers do not reimplement semantics locally.

## Consumer Rules And Stability Constraints
- `arcana-runtime`, packaging, AOT, JSON ABI, and native manifests are projections or consumers only. They may validate, mirror, load, and invoke the binding contract, but they do not define callback shape, write-back meaning, or ownership rules.
- The following architectural invariants are locked now and must not be revisited without a versioned successor contract:
  - `arcana-cabi` remains the semantic owner of the binding boundary.
  - imports and callbacks remain symmetric operations over the same value model and param metadata model.
  - write-backs remain explicit slots derived from metadata, never runtime-only magic.
  - the boundary stays producer-agnostic and platform-agnostic.
  - no Win32-specific, Lua-specific, or provider-specific value tags are allowed into the generic core.
- ABI status stays experimental, but only at the boundary-detail level. If a later break is required, it must ship as a new versioned binding contract alongside the existing descriptor scheme, with consumer-side adapter work if needed. It must not replace the entire CABI architecture or move semantics back into runtime.

## Test Plan
- `arcana-cabi` contract tests for:
  - callback ABI shape, including `out_write_backs`
  - C header generation for the revised callback signature
  - write-back slot ordering and `Unit` filling rules
  - owned `Str`/`Bytes` output and free-helper behavior
- Generic binding fixture tests, with no grimoire dependency, covering:
  - import and callback round-trips for `Int`, `Bool`, `Str`, `Bytes`, `Opaque`, and `Unit`
  - `edit` import and `edit` callback write-backs
  - metadata mismatch rejection for names, counts, modes, types, and return shape
- Downstream viability acceptance scenarios proving the CABI shape is sufficient without further redesign:
  - Win32-style opaque handles, narrow/wide string adaptation, out-params, and callback registration patterns fit the contract.
  - Lua-style opaque state/function references, strings/bytes, and callback reentry fit the contract.
  - rich Lua tables or other dynamic foreign values remain adapter-layer concerns unless a later additive or versioned value-model extension is intentionally designed.

## Assumptions
- This plan changes the binding contract in place under the current experimental phase, but it is not a throwaway prototype.
- No `arcana_winapi` work, Lua binding work, or `role = "binding"` producer/codegen expansion is part of this plan.
- The current six-tag value model is the foundation to harden now. Future growth must be additive or versioned; it must not require rebuilding the foreign boundary from scratch.
