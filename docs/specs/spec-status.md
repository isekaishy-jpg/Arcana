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
- Domain `v1-status.md` files are living readiness/classification companions for an approved scope; they may classify current bootstrap state but they may not expand domain surface by themselves.
- Domain `deferred-roadmap.md` files are authoritative only for items explicitly deferred from their parent domain scope.
- Descriptive "current implementation limits" notes do not become language law unless they are promoted into a domain scope or the frozen matrix.
- If a contract question materially affects parser shape, typed HIR, IR, or selfhost grimoires, it should not remain implicit.
- Imported MeadowLang planning docs do not define current rewrite architecture unless they are explicitly listed here as frozen, approved, reserved, or deferred authority.
- Reference-only material should live under `docs/reference/` whenever practical so historical context is not mixed into the active spec tree.
- Imported `std` and `grimoires/reference/*` are behavioral seed corpus only; they do not define rewrite layering, backend architecture, or public package surface except where current scope docs explicitly ratify them.
- Architecture/selfhost-progress reviews must privilege approved docs plus `crates/*` over carried corpus; issues found only in `std/`, `grimoires/reference/*`, or generated snapshots must be labeled as transitional/corpus drift unless a current scope explicitly makes them authoritative.
- `grimoires/reference/*` is intentionally temporary. Once the rewrite-owned runtime/app stack and owned showcase path are proven, the reference corpus should leave the default development loop and remain only as selective migration/conformance material.

## Current Registry Seed

`frozen-selfhost-baseline`
- `docs/arcana-v0.md`
- `docs/specs/backend/selfhost_language_contract_v1.md`
- `conformance/selfhost_language_matrix.toml`

`approved-pre-selfhost`
- `docs/specs/std/std/v1-scope.md`
- `docs/specs/std/std/v1-status.md`
- `docs/specs/grimoires/grimoires/v1-scope.md`
- `docs/specs/grimoires/grimoires/v1-status.md`
- `docs/specs/selfhost-host/selfhost-host/v1-scope.md`
- `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md`
- `docs/specs/memory/memory/generic-memory-spec.md`
- `docs/specs/forewords/forewords/v1-scope.md`
- `docs/specs/chain/contract_matrix_v1.md`
- `docs/specs/page-rollups/page-rollups/v1-scope.md`
- `docs/specs/tuples/tuples/v1-scope.md`
- `docs/specs/backend/anybox-policy.md`

`reserved-post-selfhost`
- `docs/specs/callables/callables/v1-status.md`

`reference-only`
- `docs/reference/backend/ir-backend-roadmap.md`
- `docs/reference/chain/chain_adoption_audit_v1.md`
- `docs/reference/forewords/generic-foreword-spec.md`
- `docs/reference/memory/v2-scope.md`
- `docs/reference/selfhost-host/generic-host-spec.md`
- `docs/reference/audits/meadow_language_behavior_audit_v1.md`

`authoritative-deferred-ledger`
- `docs/specs/backend/deferred-roadmap.md`
- `docs/specs/std/std/deferred-roadmap.md`
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
- The 3-top-level-arg phrase cap is intentional and does not, by itself, justify early callable/context-object work; use explicit data shaping until a dedicated callable-object contract exists.
- Legacy Meadow backend planning must not override the rewrite path; extract only explicitly approved deferred items and keep the original documents reference-only.
- Rebuild imported `std` for the rewrite architecture instead of preserving Meadow-era layering; showcase/game convenience logic does not become std contract just because it was carried over.
- `std` is rewrite-owned first-party library surface, not an imported MeadowLang artifact to preserve wholesale.
- Imported-std review and any app/runtime handle redesign must preserve Arcana's explicit/unambiguous doctrine: typed, named, auditable contracts over ambiguous convenience or erased fallback carriers.
- Ownership and borrowing work follows the same doctrine: copy Rust where its mutability/borrow/ownership rules are explicit and unambiguous, then tailor to Arcana's ratified surface while making any remaining behavior explicit in syntax, static rules, and diagnostics.
- First-party runtime/package ownership means more than wrapping upstream Rust crates: third-party crates may exist as private implementation details, but they must not define the public Arcana substrate or the reason first-party `std` exists.
- Future Arcana-owned grimoire responsibilities must be explicit before bootstrap so carried package names do not silently define the ecosystem.
- First-party window/input/canvas and primitive graphics/text remain pre-selfhost requirements for real apps/showcases, but carried `winspell`/`spell-events` code does not freeze the old implementation stack.
- Arcana-owned grimoire roles are capability commitments, not a promise to preserve the current reference `grimoires/` folder split or Meadow-era package topology.
- ECS scheduling/components remain first-party language/runtime surface during the rewrite; do not classify them as showcase-only helpers.
- `std.app` fixed-step helpers and `std.tooling` planner helpers are carried convenience layers, not rewrite-approved first-party architecture unless a scope doc explicitly ratifies them.
- Every std or Arcana-owned grimoire surface change must update the corresponding scope or status ledger in the same patch.
- Generated direct-emit snapshot files and similar carried artifacts are migration corpus, not primary rewrite authority; do not let them outweigh approved docs or `crates/*` during review.
