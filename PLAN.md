# Arcana Rewrite Plan, Tightened for Language Freeze

## Summary
- Hard rule: the Arcana language surface is frozen until selfhost is reached. No syntax changes, no semantic expansions, no new public builtins, and no “temporary” language features. The only allowed pre-selfhost language edits are contract-preserving bug fixes or clarifications that do not expand expressiveness.
- The previous draft was lazy in three places and this corrects them:
  - it treated the freeze as soft instead of absolute,
  - it underspecified incremental build scope,
  - it was too vague about which legacy materials to carry forward.
- The rewrite stays Rust-first, but the architecture is explicitly shaped for four non-language goals before selfhost: deterministic package management, real incremental builds, early first-party host plus app/runtime packages, and an eventual AOT backend.
- One explicit freeze exception is now part of that contract: the object/owner model (`obj`, `create ... scope-exit`, availability attachments, owner activation, hold/re-entry) is approved pre-selfhost because later desktop/grimoire work depends on explicit lifetime-packaged state rather than the discarded historical selfhost grimoire designs.
- A second explicit freeze exception is now part of that contract at the docs layer: headed regions (`recycle`, `construct`, `bind`, `Memory`) are approved pre-selfhost language law, while parser/frontend/runtime implementation and selfhost-matrix coverage remain follow-through work rather than completed status.
- A third explicit freeze exception is now part of that contract in the memory domain: view types, borrowed-slice syntax, `temp` / `session` / `ring` / `slab`, publication state through `seal` / `unseal`, and the narrow `std.binary` reader/writer layer are approved pre-selfhost because compiler/selfhost work and the Arcana-owned text stack now depend on explicit binary/view substrate instead of ad hoc byte parsing.

## Seed Import and Governance
- Copy forward only the source-of-truth docs and corpus that define the frozen contract:
  - the language contract and freeze docs,
  - the selfhost language matrix,
  - the std and grimoire scope/status docs,
  - the chain, memory, host, backend, and policy docs that describe behavior rather than old implementation.
- Carry forward `std` and the frozen conformance material in-tree.
- Historical MeadowLang reference corpus was useful during bootstrap, but it is now archived outside this repo and no longer participates in default validation or architecture decisions.
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
  - support local path dependencies, workspaces, and the built-in machine-local published `local` registry source,
  - ship the current `Arcana.lock` format with source-aware package ids,
  - internally model future dependency sources as path/git/registry, but keep named remote registries and Git transport disabled,
  - remote registry and Git transport stay post-selfhost.
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
  - any future higher-level non-std desktop, event/input, or audio facade layer must be scoped explicitly on top of the retained substrate rather than inferred from deleted package names,
  - owned compiler/tooling corpus and explicit conformance fixtures provide validation pressure without preserving Meadow-era package decomposition,
  - and those packages are real Rust-side runtime commitments of the rewrite, not temporary compatibility shims to be deferred until after selfhost.
- Artifact strategy is explicit:
  - no public bytecode compatibility contract in the new repo,
  - internal IR may be serialized for tests/cache/bootstrap only,
  - the current internal artifact/runtime lane is a bootstrap step, not the final delivery contract,
  - AOT is the intended public delivery path, including native `exe` / `dll` artifact emission,
  - if a temporary interpreter or bootstrap artifact exists, it stays internal and unstable until after selfhost,
  - but the required host/app/runtime substrate still lands as rewrite-owned Rust implementation work before selfhost.
- Selfhost sequence is fixed:
  1. repo scaffold + copied docs/corpus + freeze policy,
  2. package graph + lockfile + deterministic planning,
  3. parser/HIR/frontend for the frozen language matrix,
  4. incremental build and cache correctness,
  5. first-party host/io plus app/runtime packages compile on the new frontend,
  6. internal IR and first runnable backend with rewrite-owned host/window/input/canvas/events/graphics/text substrate,
  7. close the approved rewrite-owned `std` runtime surface so `std` is broadly runnable rather than only the initial host/app substrate slice,
  8. native AOT artifact emission for the owned `hello`-class, host-core tool, window demo, and audio smoke paths on real hosts, with native `exe` / `dll` outputs rather than only internal backend artifacts,
  9. reintroduce only the higher-level non-std packages that are still justified after the substrate cleanup, with explicit scope approval before code lands,
  10. build and native-run at least one real owned showcase app through those completed grimoires so showcase proof is not just direct substrate smoke,
  11. add first-party `arcana test` and `arcana format` on the rewrite-owned toolchain,
  12. add the smaller advisory `arcana review` layer only after showcase/tooling corpus exists to justify it,
  13. validate against the owned compiler/tooling corpus plus explicit conformance fixtures without preserving Meadow-era package decomposition by default,
  14. declare selfhost only when the new compiler can build the owned Arcana compiler/tooling corpus with no fallback to the legacy MeadowLang implementation.
- Historical-corpus archive policy:
  - broad MeadowLang reference corpus is archived outside this repo and is not part of normal validation,
  - approved docs, `crates/*`, rewrite-owned `std`, rewrite-owned grimoires, and conformance fixtures define day-to-day rewrite work,
  - archived material may still be consulted manually for migration questions, but it must not re-enter architecture review or default validation by inertia.

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
  - package-level tests proving the retained first-party package roles build against the new package/runtime boundary.
- Backend/selfhost:
  - first runnable backend milestone must run `hello`, one owned host-core tool proof, and one owned window proof,
  - std-runtime-closure milestone must cover the approved rewrite-owned `std` surface needed by owned grimoires rather than only the initial Milestone 6 subset,
  - native-artifact milestone must emit real `exe` / `dll` outputs and support real window/audio smoke on non-synthetic hosts,
  - post-native grimoire/showcase work must complete the required Arcana-owned grimoires and then prove at least one real owned showcase app through those grimoires on the native artifact lane,
  - selfhost milestone must build the owned Arcana compiler/tooling corpus with the new toolchain and no fallback to the legacy MeadowLang repo.

## Assumptions and Defaults
- The frozen baseline is the current Arcana v0 language contract plus the existing selfhost language matrix.
- Pre-selfhost work may change manifests, lockfiles, caches, host APIs, backend internals, and package tooling, but may not change the source language.
- Path dependencies and the built-in machine-local `local` registry source are the only supported dependency sources until after selfhost.
- `Arcana.lock` now uses the rewrite-owned `version = 4` source-aware schema rather than inheriting MeadowLang’s lockfile contract wholesale.
- Early host/app substrate support means “supported before selfhost”, not “before the frontend exists”; runnable demos arrive with the first AOT backend milestone.
