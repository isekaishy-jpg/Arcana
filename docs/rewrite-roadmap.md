# Rewrite Roadmap

Arcana is being rewritten under a hard language freeze until selfhost.

## Current State

Completed foundation work:
- Rust workspace scaffold with isolated crates for syntax, HIR, frontend, package/build, IR, AOT, and CLI
- language freeze policy and CI freeze guard
- seed import of frozen contract docs, conformance matrix, first-party grimoires, std, and source examples
- deterministic path-only package graph, lockfile v1, and foundation build cache
- shared workspace/package HIR loading and symbol-based module and imported-name resolution over the current placeholder parser
- structured top-level declaration parsing for functions, async functions, generic/where headers, parameter modes, and impl declarations

## Next Milestones

1. Replace the syntax placeholder with a real parser for the frozen language.
2. Build a symbol-based HIR and name-resolution pipeline.
3. Add typed frontend checking against the copied selfhost language matrix.
4. Move package fingerprints from source-content hashes to typed-HIR/API fingerprints.
5. Compile first-party host/io/window/input grimoires against the new frontend.
6. Replace the AOT placeholder with the first runnable backend.
7. Port `arcana-frontend`, `arcana-compiler-core`, and `arcana-selfhost-compiler` onto the new toolchain.

## Non-Goals Before Selfhost

- no language expansion
- no Git or registry dependencies
- no public bytecode compatibility promise
