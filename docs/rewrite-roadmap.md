# Rewrite Roadmap

Arcana is being rewritten under a hard language freeze until selfhost.

## Current State

Completed foundation work:
- Rust workspace scaffold with isolated crates for syntax, HIR, frontend, package/build, IR, AOT, and CLI
- language freeze policy and CI freeze guard
- seed import of frozen contract docs, conformance matrix, first-party grimoires, std, and source examples
- deterministic path-only package graph, lockfile v1, and foundation build cache
- shared workspace/package HIR loading and symbol-based module and imported-name resolution over the current parser foundation
- structured top-level declaration parsing for functions, async functions, behavior headers, generic/where headers, parameter modes, and impl declarations
- structured interior-member parsing for records, enums, traits, and impl bodies
- structured statement-block parsing for function-like bodies
- structured block-form `match` expression and pattern parsing for the imported enum/result corpus

## Next Milestones

1. Replace the remaining non-`match` expression and phrase parsing placeholder logic with a real parser for the frozen language.
2. Add typed frontend checking against the copied selfhost language matrix.
3. Move package fingerprints from declaration-surface/source hashes to typed-HIR/API fingerprints.
4. Compile first-party host/io/window/input grimoires against the new frontend.
5. Replace the AOT placeholder with the first runnable backend.
6. Port `arcana-frontend`, `arcana-compiler-core`, and `arcana-selfhost-compiler` onto the new toolchain.

## Non-Goals Before Selfhost

- no language expansion
- no Git or registry dependencies
- no public bytecode compatibility promise
