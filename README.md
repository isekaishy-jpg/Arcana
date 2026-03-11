# Arcana

Arcana is a Rust-first rewrite of the frozen Arcana language and tooling stack. The language surface is frozen until selfhost; current work is focused on package management, deterministic builds, frontend structure, and the eventual AOT path.

## Current State

- Rust workspace scaffold for syntax, HIR, frontend, package manager, IR, AOT, and CLI layers
- Language-freeze policy and CI guardrails
- `AnyBox` policy guard for code-bearing paths
- Cross-cutting spec-status and contract docs now explicitly lock page rollups, pair tuples, callable/context direction, and the AnyBox ban before typed frontend hardening
- Path-only package graph, deterministic workspace planning, `Arcana.lock` v1, placeholder build artifacts, normalized HIR member fingerprints, and resolved API-fingerprint-based rebuild propagation
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
- Next compiler debt is the runnable backend/runtime slice plus later typed-frontend deepening beyond the current conservative ownership and borrow-flow checks
- Seed-imported docs, grimoires, `std`, examples, and conformance fixtures from MeadowLang
- Meadow-vs-Arcana language-behavior audit captured in `docs/reference/audits/meadow_language_behavior_audit_v1.md`
- Imported `std` and first-party grimoires are behavioral seed corpus only; current rewrite authority comes from `PLAN.md`, `docs/rewrite-roadmap.md`, and the active scope docs under `docs/specs/`
- `docs/specs/std/std/v1-scope.md` defines how the rewrite treats `std`: rebuild-owned first-party library surface, not MeadowLang layering to preserve wholesale
- `docs/specs/std/std/v1-status.md` and `docs/specs/grimoires/grimoires/v1-status.md` track which std modules and first-party grimoire roles are bootstrap-required, transitional-carried, or deferred
- `arcana check` with shared package/HIR loading, symbol-based module and `use` resolution, direct-dependency enforcement, implicit `std`, and stable file/line/column diagnostics
- `arcana build` now runs frontend validation, lowers packages through placeholder IR, and emits placeholder AOT artifacts
- Placeholder artifacts now include package/module counts, dependency-edge counts, exported declaration-surface rows, and per-module summary rows for debugging/cache inspection

## Commands

```powershell
cargo test --workspace
cargo run -q -p arcana-cli -- check examples\workspace_vertical_slice
cargo run -q -p arcana-cli -- build examples\workspace_vertical_slice --plan
```

## Boundaries

- No pre-selfhost language expansion
- No Git or registry dependencies yet; only local path dependencies are enabled
- No public bytecode compatibility contract in this repo
- `docs/specs/selfhost-host/selfhost-host/v1-scope.md` freezes host-core packages; `docs/specs/selfhost-host/selfhost-host/app-substrate-v1-scope.md` freezes the rewrite-owned app/runtime substrate
- `docs/specs/grimoires/grimoires/v1-scope.md` freezes required first-party grimoire roles by responsibility rather than by carried Meadow-era package names
- The imported `arcana-compiler-core` direct-emit corpus includes one placeholder shard where the original generated payload exceeded GitHub's hard file limit; see `docs/seed-import.md`
