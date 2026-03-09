# Arcana Rewrite Policy

This policy applies to the rewrite repository from day 1.

## 1. Language Freeze (Hard Rule)

- The Arcana source language is frozen until selfhost is reached.
- No syntax changes.
- No semantic expansions.
- No new public builtins.
- No temporary or transitional language features.

Allowed pre-selfhost language edits are limited to:
- contract-preserving bug fixes
- clarifications that do not expand expressiveness
- diagnostics and tooling improvements that do not alter accepted programs

## 2. Surface Stability During Rewrite

- Package, build, cache, backend, and host/runtime work must not force source-language churn.
- If an implementation problem appears to require a language change before selfhost, the change is rejected and the implementation must be redesigned.

## 3. Pre-Selfhost Contract Closure

- Contract questions that materially affect parser shape, typed HIR, IR, or selfhost grimoires must be settled before those layers solidify.
- `docs/specs/spec-status.md` defines which spec files are frozen, approved, reserved, or deferred-only.
- Page rollups are an approved pre-selfhost contract.
- Pair-tuple rules are an approved pre-selfhost contract.
- `AnyBox`-style erased Arcana values are banned from the rewrite contract.
- Closures and general callable values are not part of the selfhost baseline; future callable capability is expected to use explicit function/context objects instead.

## 4. First-Party Package Boundary

- Host and platform capability must surface through first-party grimoires.
- Compiler special cases and name-based privilege are prohibited for library APIs.
- Internal host capabilities may exist for bootstrap purposes, but they are not public language surface.

## 5. Dependency Scope Before Selfhost

- Only local path dependencies are supported before selfhost.
- Git and registry dependency sources may be modeled internally, but they must remain rejected by manifest validation and CLI workflows.

## 6. Artifact Strategy Before Selfhost

- No public bytecode compatibility contract exists in this repository.
- Internal IR serialization is allowed for tests, cache keys, and bootstrap work only.
- AOT is the intended public delivery target.

## 7. Freeze Guard

- CI must fail when protected language-contract files change without an explicit freeze exception.
- Protected files are:
  - `docs/arcana-v0.md`
  - `conformance/selfhost_language_matrix.toml`
  - `crates/arcana-syntax/src/freeze.rs`
  - `crates/arcana-hir/src/freeze.rs`
  - `POLICY.md`
  - `docs/specs/spec-status.md`
  - `docs/specs/page-rollups/page-rollups/v1-scope.md`
  - `docs/specs/tuples/tuples/v1-scope.md`
  - `docs/specs/backend/anybox-policy.md`
  - `docs/specs/callables/callables/v1-status.md`

## 8. Selfhost Exit Condition

- The language freeze remains in force until the new Arcana compiler can build its compiler corpus with no fallback to the legacy MeadowLang implementation.
