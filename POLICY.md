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

## 3. First-Party Package Boundary

- Host and platform capability must surface through first-party grimoires.
- Compiler special cases and name-based privilege are prohibited for library APIs.
- Internal host capabilities may exist for bootstrap purposes, but they are not public language surface.

## 4. Dependency Scope Before Selfhost

- Only local path dependencies are supported before selfhost.
- Git and registry dependency sources may be modeled internally, but they must remain rejected by manifest validation and CLI workflows.

## 5. Artifact Strategy Before Selfhost

- No public bytecode compatibility contract exists in this repository.
- Internal IR serialization is allowed for tests, cache keys, and bootstrap work only.
- AOT is the intended public delivery target.

## 6. Freeze Guard

- CI must fail when protected language-contract files change without an explicit freeze exception.
- Protected files are:
  - `docs/arcana-v0.md`
  - `conformance/selfhost_language_matrix.toml`
  - `crates/arcana-syntax/src/freeze.rs`
  - `crates/arcana-hir/src/freeze.rs`
  - `POLICY.md`

## 7. Selfhost Exit Condition

- The language freeze remains in force until the new Arcana compiler can build its compiler corpus with no fallback to the legacy MeadowLang implementation.

