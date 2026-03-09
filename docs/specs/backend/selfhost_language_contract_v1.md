# Selfhost Language Contract v1

This document freezes the canonical selfhost language target for Arcana as of **March 5, 2026**.

Source of truth:
- `docs/arcana-v0.md`

Companion matrix:
- `conformance/selfhost_language_matrix.toml`

Contract rules:
1. Each matrix entry must declare a stable `id` and a `status`.
2. Each entry must include at least one `positive` target and one `negative` target.
3. Matrix targets must resolve to real files/directories in the repository.
4. CI must fail if any entry is missing required fields or coverage.

Status values:
- `required`

Notes:
- This contract tracks language/runtime surface used by canonical selfhost check/compile/build conformance.
- Semantic determinism is enforced by the existing selfhost parity and bootstrap guards.
