# Arcana

Arcana is a Rust-first rewrite of the frozen Arcana language and tooling stack. The language surface is frozen until selfhost; current work is focused on package management, deterministic builds, frontend structure, and the eventual AOT path.

## Current State

- Rust workspace scaffold for syntax, HIR, frontend, package manager, IR, AOT, and CLI layers
- Language-freeze policy and CI guardrails
- Path-only package graph, deterministic workspace planning, `Arcana.lock` v1, placeholder build artifacts, and exported-surface API fingerprints for rebuild propagation
- Shared HIR module summaries and package-level HIR graphs now sit between syntax parsing and both frontend checks and package API hashing
- Seed-imported docs, grimoires, `std`, examples, and conformance fixtures from MeadowLang
- `arcana check` with line-based module loading, import/reexport/use resolution, direct-dependency enforcement, implicit `std`, and stable file/line/column diagnostics
- `arcana build` now runs that frontend validation before writing placeholder artifacts
- Placeholder artifacts now include package module counts, dependency-edge counts, and exported-surface rows for debugging/cache inspection

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
