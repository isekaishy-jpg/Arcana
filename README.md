# Arcana

Arcana is a Rust-first rewrite of the frozen Arcana language and tooling stack. The language surface is frozen until selfhost; current work is focused on package management, deterministic builds, frontend structure, and the eventual AOT path.

## Current State

- Rust workspace scaffold for syntax, HIR, frontend, package manager, IR, AOT, and CLI layers
- Language-freeze policy and CI guardrails
- `AnyBox` policy guard for code-bearing paths
- Cross-cutting spec-status and contract docs now explicitly lock page rollups, pair tuples, callable/context direction, and the AnyBox ban before typed frontend hardening
- Path-only package graph, deterministic workspace planning, `Arcana.lock` v1, placeholder build artifacts, and declaration-surface API fingerprints for rebuild propagation
- Shared HIR module, package, and workspace summaries now sit between syntax parsing, frontend checks, and package graph consumers
- Symbol-based module and imported-name resolution now lives in HIR and is consumed by frontend diagnostics
- Unsupported top-level syntax now fails explicitly instead of being silently skipped
- The syntax/HIR layer now captures structured top-level declarations for functions, async functions, systems, behaviors, lang items, intrinsic declarations, generics/where clauses, parameter modes, built-in forewords, and impl headers
- Record fields, enum variants, trait members, and impl members are now parsed into structured interior members instead of staying opaque body text
- Function-like bodies now parse structured statement blocks for `let`, `return`, `defer`, `if`/`else`, `while`, `for`, assignments, `break`, and `continue`
- Block-form `match` expressions now lower into structured expression and pattern nodes, including wildcard, literal, variant, and `A | B` arm shapes
- Non-`match` expressions now lower structured qualified phrases, named phrase args, collection literals, direct chain phrases, memory phrases, unary/binary operators, `>> await`, `weave`/`split`, member access, standalone ranges, and the unambiguous index/slice subset
- Assignment statements now carry structured name/member/index targets instead of raw target strings
- Pair-tuple rules are now enforced in syntax/frontend diagnostics: `.0`/`.1` only, no tuple destructuring in `let`/`for`/params, no tuple field assignment, and no three-element tuple types or literals
- Page rollups now parse and lower through syntax/HIR for function-like owners and block-owning statements, with subject-scope validation, cleanup-subject reassignment diagnostics, fixture coverage, and a real example package
- `arcana check` now validates unresolved `lang` item targets after workspace resolution
- Remaining opaque expression debt is now mostly deeper attached-block bodies, ownership/type semantics, and the still-ambiguous generic-bracket versus index-bracket edge cases
- Seed-imported docs, grimoires, `std`, examples, and conformance fixtures from MeadowLang
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
- The imported `arcana-compiler-core` direct-emit corpus includes one placeholder shard where the original generated payload exceeded GitHub's hard file limit; see `docs/seed-import.md`
