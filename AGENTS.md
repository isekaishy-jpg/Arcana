# Arcana Repo Instructions

## Rewrite Authority Order

When judging rewrite architecture, selfhost readiness, or whether something is "current Arcana behavior", use this order:

1. `POLICY.md`
2. `docs/specs/spec-status.md` and any spec files it marks frozen or approved
3. `PLAN.md` and `docs/rewrite-roadmap.md` for sequencing and milestone intent
4. `crates/*` for the actual Rust rewrite implementation
5. `std/`, `grimoires/`, `examples/`, and conformance fixtures only as carried source corpus unless an approved scope explicitly ratifies the surface being discussed

## Review Boundary

- Do not treat imported `std`, first-party grimoires, examples, or generated direct-emit snapshots as rewrite architecture evidence by themselves.
- For architecture reviews and selfhost-progress reviews, findings must distinguish:
  - crate-side rewrite implementation
  - approved first-party contract
  - transitional carried corpus
- If a problem exists only in carried corpus, label it as `corpus-only` or `transitional`, not as crate-side rewrite dependence.
- If a carried corpus surface conflicts with approved docs, the docs and `crates/*` win; the corpus becomes migration work, not architecture authority.

## Imported Corpus Hotspots

- `std/`, `grimoires/`, and `examples/` are still useful for behavioral checks and migration pressure.
- `grimoires/arcana-compiler-core/src/direct_emit_specs_*` are carried/generated artifacts and should not be used as primary evidence for current rewrite architecture.
- If a review needs to talk about these paths, state explicitly whether the issue is:
  - rewrite-crate behavior
  - approved contract mismatch
  - carried-corpus drift
