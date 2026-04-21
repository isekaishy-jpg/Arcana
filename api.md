# Boundary-Only `api` + `binding.v2` For Full WinAPI Buildout

## Summary
- Introduce `api` as a boundary-only transport feature, not a regular local call surface and not a general function-value feature.
- Keep the 3-top-level-arg cap unchanged for ordinary direct calls.
- Rebind cross-package and binding/foreign callable transport through `api`, while owner-package internals keep using ordinary routines/calls.
- Migrate the binding ABI to a new `arcana.cabi.binding.v2` contract with no legacy-compat obligation.
- Expand `api` far enough to cover both major WinAPI blockers:
  - packed request/response transport for fixed high-arity APIs
  - ownership-transfer normalization for callee-created results

## Core Model
- `api` exists only at package/binding seams:
  - package exports/imports
  - binding/foreign projections
  - cross-package dependency transport
- Ordinary local code does not use `api` unless it is explicitly working with a boundary API handle.
- Boundary call shape is always:
  - one packed request contract in
  - one packed response/update contract out
- Request/response payloads reuse struct-shaped surface and callable-struct-compatible lowering where useful, but `api` remains a distinct semantic feature.

- Raw path relationship:
  - keep existing module/name organization as the authority
  - across package boundaries, callable raw exports now transport as `api` handles instead of ordinary direct-call exports
  - inside the owning package, implementation may still lower to ordinary routines/raw declarations

## `api` Contract Contents
- Each `api` contract must carry backend-neutral metadata for:
  - request contract type
  - response/update contract type
  - field names and stable order
  - field modes and call-direction semantics
  - lane classification
  - callback compatibility
  - backend target kind
  - ownership-transfer / release metadata

- Lane classification must cover:
  - plain scalar/value fields
  - opaque handles/interfaces/function-pointer tokens
  - typed pointee read/edit fields
  - memory-backed buffer fields
  - callback token fields
  - owned-transfer result fields

- Ownership envelope is pinned in `api` metadata:
  - transfer mode: borrowed, caller-edited, callee-owned
  - owned result kind: opaque, string, buffer, array-with-companion-count/len, interface
  - release family: `Release`, `CoTaskMemFree`, `LocalFree`, custom API, or explicitly unsupported
  - companion-field coupling: count/len/status fields tied to a returned owned result
  - partial-failure cleanup behavior for multi-output responses

## Backend and Lowering
- Add `arcana.cabi.binding.v2` for boundary `api` transport.
  - No legacy compatibility layer is required.
  - `binding.v1` assumptions do not constrain this redesign.
  - `export` role remains separate; this plan changes the binding/boundary seam.

- Every boundary `api` lowers through CABI to one backend target:
  - ordinary Arcana/package implementation
  - direct foreign symbol
  - embedded `C` shim
- Embedded `C` is allowed only as an implementation backend under an `api` contract; semantics still live in `api` metadata.

- Add one generic normalized backend-result envelope plus one generic collector/materializer.
  - Backend returns normalized results, not unresolved raw pointer slots.
  - Collector/materializer:
    - builds the packed response/update value
    - applies edit/write-back fields
    - materializes owned foreign results
    - couples companion count/len/status fields
    - attaches cleanup behavior from `api` ownership metadata

- Cleanup behavior:
  - cleanup policy is specified on packed owned response fields via `api` metadata
  - when owned results are materialized/unpacked into caller-visible bindings, those bindings become cleanup-capable using the API-provided cleanup handler/release family
  - cleanup handlers are part of the implementation strategy, not a substitute for ownership metadata

## Public Interfaces
- New boundary language surface:
  - `api` declarations
  - `api` region head
- `api` is explicitly boundary-only and not a replacement for ordinary local direct calls.
- Cross-package callable transport uses `api`; owner-package internals keep ordinary callable/routine lowering.
- Existing raw WinAPI paths remain the authority for package/module naming, but their boundary callability is now mediated by `api`.

## Test Plan
- Parser/frontend tests for:
  - `api` declarations and `api` region head
  - boundary-only restrictions
  - no accidental general function-value behavior
  - request/response pack/unpack flows

- HIR/IR/artifact tests for:
  - `api` contract lowering
  - stable request/response contract identity
  - lane classification round-trip
  - ownership envelope round-trip
  - raw-path boundary rebinding to `api`

- `binding.v2` backend/runtime tests for:
  - direct foreign symbol backend
  - embedded `C` shim backend
  - generic normalized result collection/materialization
  - cleanup activation from packed owned response fields
  - no package-name/runtime special cases

- WinAPI integration tests for representative classes:
  - high-arity call via packed request
  - opaque handle round-trip
  - typed pointee read/edit
  - callback token invocation
  - memory-backed buffer lane
  - callee-owned string/buffer/interface result with release metadata and companion count/len
  - partial-failure cleanup on multi-output owned responses

## Assumptions
- This is the full architecture pass required before large-scale raw WinAPI buildout.
- Remaining post-plan work should be classification/projection/coverage volume, not new seam invention.
- `api` is transport-only at boundaries; it must not become a stealth bypass for the general 3-arg direct-call rule.
- Cleanup handlers are implementation support for owned response fields, but ownership meaning is defined by `api` metadata, not inferred from cleanup alone.
