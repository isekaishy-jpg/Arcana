# Arcana

Arcana is a Rust-first rewrite of the frozen Arcana language and tooling stack. The language surface is frozen until selfhost; current work is focused on package management, deterministic builds, frontend structure, and the eventual AOT path.

## Current State

- Rust workspace scaffold for syntax, HIR, frontend, package manager, IR, AOT, runtime, and CLI layers
- Language-freeze policy and CI guardrails
- `AnyBox` policy guard for code-bearing paths
- Cross-cutting spec-status and contract docs now explicitly lock page rollups, pair tuples, callable/context direction, and the AnyBox ban before typed frontend hardening
- Path-only package graph, deterministic workspace planning, `Arcana.lock` v1, internal backend-contract build artifacts, normalized HIR member fingerprints, and resolved API-fingerprint-based rebuild propagation
- Shared HIR module, package, and workspace summaries now sit between syntax parsing, frontend checks, and package graph consumers
- Symbol-based module and imported-name resolution now lives in HIR and is consumed by frontend diagnostics
- Unsupported top-level syntax now fails explicitly instead of being silently skipped
- The syntax/HIR layer now captures structured top-level declarations for functions, async functions, systems, behaviors, lang items, intrinsic declarations, generics/where clauses, parameter modes, built-in forewords, and impl headers
- Syntax-level contract enforcement now matches the carried source corpus for phrase comma-shape rules, chain-style families and reverse-introducer limits, memory-family allowlists, built-in foreword payload/target rules, validated `#stage/#chain` contract payloads, `#test` function constraints, `#boundary[target = "lua" | "sql"]` signature checks, and current-target `#only[...]` filtering
- Record fields, enum variants, trait members, and impl members are now parsed into structured interior members instead of staying opaque body text
- Function-like bodies now parse structured statement blocks for `let`, `return`, `defer`, `if`/`else`, `while`, `for`, assignments, `break`, and `continue`
- Block-form `match` expressions now lower into structured expression and pattern nodes, including wildcard, literal, variant, and `A | B` arm shapes
- Non-`match` expressions now lower structured qualified phrases, named phrase args, path refs, bool/int/string literals, collection literals, chain phrases with explicit style plus introducer plus connector structure and bound `with (...)` adapters, memory phrases, unary/binary operators, `>> await`, `weave`/`split`, member access, standalone ranges, and the unambiguous index/slice subset
- Pair tuple literals now lower as structured expressions, and generic-argument brackets like `path[(K, V)]` are distinguished from runtime indexing so tuple type args no longer leak into value resolution
- Header-phrase attached blocks now lower as structured named attachments and chain attachments for qualified and memory phrases instead of raw block entries
- Assignment statements now carry structured name/member/index targets instead of raw target strings
- Pair-tuple rules are now enforced in syntax/frontend diagnostics: `.0`/`.1` only, no tuple destructuring in `let`/`for`/params, no tuple field assignment, and no three-element tuple types or literals
- Page rollups now parse and lower through syntax/HIR for function-like owners and block-owning statements, with subject-scope validation, cleanup-subject reassignment diagnostics, fixture coverage, and a real example package
- `arcana check` now validates unresolved `lang` item targets plus declaration-surface type and lifetime references after workspace resolution
- `arcana check` now also validates conservative body-level value resolution for locals, namespace-qualified member chains, enum variant constructors, module impl-method paths, structured chain stages and bound args, memory constructors, page-rollup handlers, and expression generic-argument type references within the active type scope
- `arcana check` now also enforces recursive boundary-safe typing for carried Lua/SQL boundary contracts across nested record/enum surfaces
- Lua/SQL boundary-varietal compile-time contracts now have example and negative conformance coverage, and the carried first-class ECS direction is documented without freezing generalized ECS query authoring into the selfhost baseline
- Impl header generic/lifetime params now survive syntax/HIR lowering, so inherited `T`/`'a` scope is available to later frontend work
- Next compiler debt is native AOT artifact emission, followed by backend hardening around richer scheduler/resource semantics beyond the current proven std-runtime lane, and pre-selfhost tooling work for `arcana test` and `arcana format`
- Seed-imported frozen docs, conformance fixtures, and historical MeadowLang corpus were used to bootstrap the rewrite; the broad reference tree is now archived outside this repo
- Meadow-vs-Arcana language-behavior audit captured in `docs/reference/audits/meadow_language_behavior_audit_v1.md`
- Current rewrite authority comes from `PLAN.md`, `docs/rewrite-roadmap.md`, the active scope docs under `docs/specs/`, and `crates/*`; archived MeadowLang corpus is migration context only, while `std` is rewrite-owned first-party surface
- Rewrite-owned app/media grimoires now scaffold under `grimoires/owned/*`
- Rewrite-owned app/media grimoire scaffolds now check in crate-side regression coverage against the new frontend, and pre-selfhost `std` shape is frozen unless Milestone 6/runtime work proves a concrete blocker
- `docs/specs/std/std/v1-scope.md` defines how the rewrite treats `std`: rebuild-owned first-party library surface, not MeadowLang layering to preserve wholesale
- `docs/specs/std/std/v1-status.md` and `docs/specs/grimoires/grimoires/v1-status.md` track which std modules and future Arcana-owned app/media grimoire roles are bootstrap-required, transitional-carried, or deferred
- `arcana check` with shared package/HIR loading, symbol-based module and `use` resolution, direct-dependency enforcement, implicit `std`, and stable file/line/column diagnostics
- `arcana build` now runs frontend validation, lowers packages through internal IR, and emits unstable internal backend-contract artifacts that carry package metadata plus lowered routine/body rows for the Milestone 6 runtime/backend slice
- `arcana-runtime` now loads that internal backend-contract artifact into a parsed Rust-side execution plan, so the Milestone 6 runtime path no longer starts from package/build-only strings or reparse executable rows on each routine call
- `arcana-runtime` now executes a real Milestone 6 backend lane over that parsed plan: `main`, local/control-flow semantics (`let`, arithmetic/comparison plus current string concat, `if`/`while`/`for`, assignment including record-member writeback, `break`/`continue`, scoped `defer`), routine parameters, named/attached phrase parsing, first-class `Record` / `List` / `Array` / `Map` values for the current data-model slice, and record-member / impl-method execution where the lowered backend artifact carries those routines
- `arcana build` now links the member artifact with the transitive workspace dependency closure plus implicit rewrite-owned `std` when used, and the internal IR/AOT contract now carries stable per-routine row identity, impl methods, behavior attrs, and routine type-parameter rows as executable runtime metadata, so `arcana-runtime` can execute linked std routine bodies without collapsing overloaded impl methods onto shared `(module, symbol)` buckets; current crate-side regression coverage proves counter, args/tool, local record/impl execution, explicit local `&` / `&mut` / `*` runtime execution, linked `std.text`, linked `std.option` / typed enum variants, linked `std.collections.array`, linked `std.collections.list` / `std.collections.map` / `std.collections.set` plus broader wrapper closure, linked `std.iter`, linked `std.config`, linked `std.manifest`, linked `std.path` / `std.env` / `std.io` / `std.bytes` / `std.time` / `std.types.core`, linked `std.concurrent` current async/task/thread/channel/mutex/atomic floor over an explicit deterministic deferred scheduler lane, linked `std.memory` arena/frame/pool core plus executable memory phrases plus current borrow_read/borrow_edit write-through semantics over an explicit runtime reference/place lane, linked full `std.fs` including stream APIs over explicit typed `FileStream` handles, linked `std.process`, synthetic host-core workspace apps against the current host-core std surface, synthetic app-substrate workspace apps against the current `std.window` / `std.events` / `std.input` / `std.canvas` / `std.time` / `std.audio` seams over explicit typed `Window` / `Image` / `AppFrame` / `AudioDevice` / `AudioBuffer` / `AudioPlayback` handles, linked/synthetic ECS runtime coverage for `behavior[...]`, `system[...]`, `std.behaviors`, `std.ecs`, and `std.kernel.ecs`, and runtime proof that an app built on the owned `arcana_desktop` / `arcana_audio` grimoires executes end to end on the current synthetic host; this completes the first runnable backend lane and Milestone 7 std-runtime closure, while the remaining backend work is hardening any broader borrow/resource model beyond the current reference/place lane, hardening any richer scheduler/worker semantics beyond the current deferred concurrency lane, and native AOT `exe` / `dll` artifact emission for real host smoke

## Commands

```powershell
cargo test --workspace
cargo run -q -p arcana-cli -- check grimoires\owned\app\arcana-desktop
cargo run -q -p arcana-cli -- check grimoires\owned\app\arcana-audio
```

## Boundaries

- No pre-selfhost language expansion
- No Git or registry dependencies yet; only local path dependencies are enabled
- No public bytecode compatibility contract in this repo
- `docs/specs/selfhost-host/selfhost-host/v1-scope.md` freezes host-core packages; `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md` freezes the rewrite-owned app/runtime substrate
- `docs/specs/grimoires/grimoires/v1-scope.md` freezes required future Arcana-owned app/media grimoire roles by responsibility rather than by carried Meadow-era package names
- Historical seed-import notes and archived-corpus context live in `docs/seed-import.md`
