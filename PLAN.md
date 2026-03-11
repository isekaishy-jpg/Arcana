# Arcana Rewrite Plan, Tightened for Language Freeze

## Summary
- Hard rule: the Arcana language surface is frozen until selfhost is reached. No syntax changes, no semantic expansions, no new public builtins, and no “temporary” language features. The only allowed pre-selfhost language edits are contract-preserving bug fixes or clarifications that do not expand expressiveness.
- The previous draft was lazy in three places and this corrects them:
  - it treated the freeze as soft instead of absolute,
  - it underspecified incremental build scope,
  - it was too vague about which legacy materials to carry forward.
- The rewrite stays Rust-first, but the architecture is explicitly shaped for four non-language goals before selfhost: deterministic package management, real incremental builds, early first-party host plus app/runtime packages, and an eventual AOT backend.

## Seed Import and Governance
- Copy forward only the source-of-truth docs and corpus that define the frozen contract:
  - the language contract and freeze docs,
  - the selfhost language matrix,
  - the std and grimoire scope/status docs,
  - the chain, memory, host, backend, and policy docs that describe behavior rather than old implementation.
- Copy forward source reference corpus that defines required behavior:
  - `std`,
  - `grimoires/reference/toolchain/arcana-frontend`,
  - `grimoires/reference/toolchain/arcana-compiler-core`,
  - `grimoires/reference/toolchain/arcana-selfhost-compiler`,
  - `grimoires/reference/app/winspell`,
  - `grimoires/reference/app/spell-events`,
  - `grimoires/reference/examples/*` and the conformance targets referenced by the selfhost language matrix.
- Do not copy old Rust implementation crates, old bytecode/runtime binaries, `PLAN*.md`, `tmp/`, `target/`, `.arcana/` artifacts, or generated golden outputs.
- Add a repo policy document on day 1 that states:
  - language frozen until selfhost,
  - package/build/backend work must not force source-language churn,
  - host/platform features must surface through Arcana-owned packages above `std`, not compiler special cases,
  - any requested pre-selfhost language change is rejected unless it is a contract-preserving bug fix.

## Architecture and Milestones
- Initialize a Rust workspace with distinct crates for:
  - syntax and parser,
  - HIR and symbol graph,
  - frontend/typecheck,
  - package graph, lockfile, and cache,
  - internal IR,
  - AOT backend,
  - CLI/driver.
- Use a symbol-based pipeline from the start:
  - source text -> CST/AST,
  - AST -> HIR,
  - HIR -> name-resolved and typed HIR,
  - typed HIR -> internal IR,
  - internal IR -> AOT backend.
- Do not reintroduce module flattening or string-based name rewriting. Grimoires resolve through a real module graph and symbol table.
- Incremental build is not deferred as “later polish”. It is part of the core design:
  - cache parsed modules by source-content hash,
  - cache resolved/typed modules by module graph plus dependency fingerprint,
  - cache package/member build results by typed-HIR fingerprint,
  - invalidate downstream members transitively when exported API fingerprints change,
  - support no-op rebuilds and selective rebuilds before the backend milestone is considered complete.
- Package manager scope before selfhost is fixed:
  - support local path dependencies and workspaces only,
  - ship `Arcana.lock` v1,
  - internally model future dependency sources as path/git/registry, but enable only path in the CLI and manifest validator,
  - registry and Git transport stay post-selfhost.
- Public interfaces before selfhost:
  - `book.toml` remains the package manifest,
  - `Arcana.lock` remains the lockfile,
  - `arcana check`,
  - `arcana test`,
  - `arcana format`,
  - `arcana review` as a smaller advisory layer only after enough real Arcana showcase/tooling corpus exists to justify Arcana-native usage guidance,
  - `arcana build --plan`,
  - `arcana build`.
- Tooling boundary before selfhost is explicit:
  - `arcana check` remains the correctness gate for parsing, typing, ownership/borrowing, lifetime, boundary, and other compiler-owned diagnostics,
  - `arcana test` is a pre-selfhost requirement,
  - `arcana format` is also expected before selfhost, but should be driven by explicit formatting doctrine and syntax maturity rather than ad hoc style guesses,
  - `arcana review` must stay intentionally smaller pre-selfhost and should only grow beyond compiler-owned/spec-backed advice once the rewrite has real showcase-scale Arcana code to learn from.
- First-party package milestone includes both host-core and windowing layers:
  - host/core packages for text, fs, path, process, args/env,
  - app/runtime packages for window/input/canvas/events/time/audio plus primitive graphics/text,
  - ECS/behavior runtime substrate remains first-party and is not treated as showcase-only logic,
  - then future Arcana-owned app/media grimoires for desktop facade, event/input utility, and audio facade prove the package surface is usable,
  - reference compiler corpus under `grimoires/reference/toolchain/*` remains validation pressure and selfhost-closure corpus, not the target package architecture to preserve,
  - and those packages are real Rust-side runtime commitments of the rewrite, not temporary compatibility shims to be deferred until after selfhost.
- Artifact strategy is explicit:
  - no public bytecode compatibility contract in the new repo,
  - internal IR may be serialized for tests/cache/bootstrap only,
  - AOT is the intended public delivery path,
  - if a temporary interpreter or bootstrap artifact exists, it stays internal and unstable until after selfhost,
  - but the required host/app/runtime substrate still lands as rewrite-owned Rust implementation work before selfhost.
- Selfhost sequence is fixed:
  1. repo scaffold + copied docs/corpus + freeze policy,
  2. package graph + lockfile + deterministic planning,
  3. parser/HIR/frontend for the frozen language matrix,
  4. incremental build and cache correctness,
  5. first-party host/io plus app/runtime packages compile on the new frontend,
  6. internal IR and first AOT backend with rewrite-owned host/window/input/canvas/events/graphics/text substrate,
  7. runnable proof on carried-over examples such as `hello`, one host tool, and one window demo,
  8. add first-party `arcana test` and `arcana format` on the rewrite-owned toolchain, and keep `arcana review` limited to compiler-owned/spec-backed advice until showcase-scale corpus exists,
  9. write any needed Arcana-owned layers on top of the new toolchain and validate against the reference compiler corpus without preserving Meadow-era package decomposition by default,
  10. declare selfhost only when the new compiler can build the reference compiler corpus without using the old MeadowLang implementation.
- Reference-corpus quarantine policy:
  - architectural quarantine is immediate: `grimoires/reference/*` and other carried corpus remain behavioral reference only and must not define current rewrite architecture,
  - operational quarantine begins once Milestone 6 is complete, the owned app/media grimoires are usable on the rewrite-owned runtime substrate, and at least one real owned showcase exists,
  - from that point, reference corpus should stop being part of normal default validation and move to selective migration/conformance pressure only,
  - post-selfhost, keep only distilled conformance fixtures and narrowly useful historical reference; do not keep broad carried reference trees in the default development loop by inertia.

## Tests and Acceptance Criteria
- Freeze enforcement:
  - CI fails if grammar tokens, AST/HIR node kinds, or source-language docs change without an explicit freeze-exception flag,
  - the selfhost language matrix is copied and becomes the canonical required-feature list.
- Package/build:
  - deterministic workspace plan ordering,
  - deterministic `Arcana.lock` rendering,
  - no-op rebuild is cache-hit only,
  - edit in a leaf package rebuilds only that package,
  - edit in a shared dependency rebuilds all dependents and nothing else.
- Frontend:
  - parser/typecheck goldens for every required feature family in the copied selfhost matrix,
  - diagnostics preserve path/line/column stability for the curated negative corpus.
- First-party packages:
  - compile tests for core host packages,
  - compile tests for window/input/canvas/time/audio packages,
  - package-level tests proving the required future Arcana-owned app/media grimoire roles build against the new package/runtime boundary.
- Backend/selfhost:
  - first AOT milestone must run `hello`, one host-tool example, and one window example,
  - selfhost milestone must build the reference compiler corpus with the new toolchain and no fallback to the legacy MeadowLang repo.

## Assumptions and Defaults
- The frozen baseline is the current Arcana v0 language contract plus the existing selfhost language matrix.
- Pre-selfhost work may change manifests, lockfiles, caches, host APIs, backend internals, and package tooling, but may not change the source language.
- Path dependencies are the only supported dependency source until after selfhost.
- `Arcana.lock` is restarted at `version = 1` in the new repo rather than inheriting MeadowLang’s lockfile schema wholesale.
- Early host/app substrate support means “supported before selfhost”, not “before the frontend exists”; runnable demos arrive with the first AOT backend milestone.
