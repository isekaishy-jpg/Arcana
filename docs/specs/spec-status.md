# Spec Status Taxonomy

This document defines how Arcana spec files are interpreted during the rewrite.

## Status Classes

`frozen-selfhost-baseline`
- Part of the contract the rewrite is expected to preserve through selfhost.
- Changes require an explicit freeze exception and corresponding contract updates.

`approved-pre-selfhost`
- A design contract that must be settled before typed frontend and backend seams harden.
- Clarifications are allowed, but casual redesign is not.

`reserved-post-selfhost`
- Direction is chosen, but it is not part of the selfhost baseline.
- Frontend, HIR, IR, std, and runtime work must not assume the feature exists.

`reference-only`
- Historical or exploratory material copied for context.
- It does not define current Arcana behavior by itself.

`authoritative-deferred-ledger`
- Tracks deferred work for a parent domain spec that is already approved or frozen.
- A deferred ledger may delay or schedule work, but it may not expand surface area on its own.

## Interpretation Rules

- `docs/arcana-v0.md` remains the top-level frozen language summary until its contents are fully split into finer-grained domain specs.
- Domain `v1-scope.md` or equivalent scope files define the current approved contract for that domain.
- Domain `deferred-roadmap.md` files are authoritative only for items explicitly deferred from their parent domain scope.
- Descriptive "current implementation limits" notes do not become language law unless they are promoted into a domain scope or the frozen matrix.
- If a contract question materially affects parser shape, typed HIR, IR, or selfhost grimoires, it should not remain implicit.

## Current Registry Seed

`frozen-selfhost-baseline`
- `docs/arcana-v0.md`
- `conformance/selfhost_language_matrix.toml`

`approved-pre-selfhost`
- `docs/specs/selfhost-host/selfhost-host/v1-scope.md`
- `docs/specs/memory/memory/generic-memory-spec.md`
- `docs/specs/forewords/forewords/v1-scope.md`
- `docs/specs/chain/contract_matrix_v1.md`
- `docs/specs/page-rollups/page-rollups/v1-scope.md`
- `docs/specs/tuples/tuples/v1-scope.md`
- `docs/specs/backend/anybox-policy.md`

`reserved-post-selfhost`
- `docs/specs/callables/callables/v1-status.md`

`reference-only`
- `docs/specs/chain/chain_adoption_audit_v1.md`

`authoritative-deferred-ledger`
- `docs/specs/selfhost-host/selfhost-host/deferred-roadmap.md`
- `docs/specs/memory/memory/deferred-roadmap.md`
- `docs/specs/forewords/forewords/deferred-roadmap.md`
- `docs/specs/page-rollups/page-rollups/deferred-roadmap.md`
- `docs/specs/tuples/tuples/deferred-roadmap.md`

## Immediate Rewrite Guidance

- Page rollups are a pre-selfhost contract, not a post-selfhost cleanup idea.
- Chain surface should stay explicit as style qualifier plus introducer family plus connector-directed edges.
- Pair-tuple rules must be explicit before selfhost because the imported corpus already depends on them heavily.
- Pair-only tuples are the current baseline, not a statement that generalized tuples are off the table forever.
- `plan` and `lazy` chain semantics must stay explicit in the frozen docs so pipeline validation and demand-sensitive execution are not inferred from old implementation shortcuts.
- `AnyBox` or equivalent erased Arcana value carriers are banned from the rewrite contract.
- Closures are not the intended direction; if first-class callable capability is added later, it should be through explicit function/context objects.
