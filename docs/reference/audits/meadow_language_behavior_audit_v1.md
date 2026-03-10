# Meadow Language Behavior Audit v1

Status: `reference-only`

This audit records a completed MeadowLang-to-Arcana parity pass. It is retained as rewrite context and does not override the frozen matrix or scoped v1 domain specs.

## Scope
- Date: 2026-03-10
- Purpose: record which MeadowLang language behaviors were explicitly audited against Arcana so the rewrite does not re-litigate the same surface drift later.

## Audited Domains Now Aligned
- Phrase arg lanes: trailing-comma rejection, top-level comma shape rejection, and the hard cap of 3 top-level inline args are aligned for qualified and memory phrases.
- Phrase qualifier/constructor surface: qualified phrase qualifiers are constrained to named paths or `? | > | >>`, memory constructors are constrained to `path` or `path[type_args]`, and qualifier-specific named-overflow rules now match Meadow statement behavior.
- Phrase attachment context: attached blocks are limited to standalone qualified/memory phrase statements, matching Meadow statement placement while preserving Arcana's own metadata model on the attached entries.
- Chain phrases: style qualifier, introducer family, connector direction, reverse-introducer restrictions, and invalid-style rejection are aligned with Meadow-era behavior and current Arcana chain docs.
- Memory phrases: the current allocator family allowlist is `arena | frame | pool`, matching the carried contract.
- Built-in forewords: target validation, payload validation, `#test`, `#only`, `#boundary[target="lua"|"sql"]`, and statement-level `#chain[...]` handling are now explicitly checked.
- Chain contracts: `#stage[...]` and `#chain[...]` payload keys and value domains now reject Meadow-invalid shapes instead of being treated as pass-through metadata.
- Boundary interop: Lua/SQL varietal compile-time contracts are carried, mutable-borrow and reference-return limits are enforced, and recursive boundary-safe typing now follows nested record/enum surfaces.
- Tuples: pair-only tuple stabilization is explicit and enforced, while future tuple enrichment stays deferred rather than silently prohibited forever.
- Page rollups: adopted pre-selfhost with explicit ownership/subject rules rather than leaving them as a post-selfhost surprise.
- ECS/behaviors: first-class behavior/system direction and `std.ecs` scheduler/component direction are carried; broad query authoring remains intentionally outside the frozen selfhost baseline.

## Verified Divergences
- Forewords on attached header entries: Arcana intentionally keeps forewords available on attached header entries because forewords are the language metadata system and are meant to sit above block/header items similarly to Rust attributes. Meadow only carried statement forewords on chain phrases.

## Intentional Non-Carryovers
- No public bytecode compatibility contract in the new repo.
- No `AnyBox`/erased Arcana-value carrier in public language, HIR, IR, or host ABI.
- No restoration of legacy VM-host behavior as active language/tooling contract.

## Still Pending Implementation, But Not Drift
- Full ownership/borrow flow and move analysis.
- Full chain-contract aggregation and scheduler/runtime enforcement beyond explicit payload validation and required `#chain[...]` presence in behavior/system chains.
- Host/backend implementation for Lua/SQL hot-path and reload workflows.
- General ECS query authoring.

## Policy Use
- When Arcana behavior differs from Meadow, this file should either:
  - record the difference as intentional, or
  - point to the commit/spec that restored parity.
- Descriptive notes in imported docs do not outrank the frozen matrix or scoped v1 domain specs.
