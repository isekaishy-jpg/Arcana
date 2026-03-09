# Rewrite Roadmap

Arcana is being rewritten under a hard language freeze until selfhost.

## Current State

Completed foundation work:
- Rust workspace scaffold with isolated crates for syntax, HIR, frontend, package/build, IR, AOT, and CLI
- language freeze policy and CI freeze guard
- AnyBox policy guard over code-bearing paths
- explicit spec-status taxonomy plus pre-selfhost contract docs for page rollups, tuples, callable/context direction, and AnyBox ban
- seed import of frozen contract docs, conformance matrix, first-party grimoires, std, and source examples
- deterministic path-only package graph, lockfile v1, and foundation build cache
- shared workspace/package HIR loading and symbol-based module and imported-name resolution over the current parser foundation
- structured top-level declaration parsing for functions, async functions, behavior headers, generic/where headers, parameter modes, and impl declarations
- structured interior-member parsing for records, enums, traits, and impl bodies
- structured statement-block parsing for function-like bodies, including `defer`
- structured block-form `match` expression and pattern parsing for the imported enum/result corpus
- structured qualified phrase, unary/binary operator, `>> await`, and `weave`/`split` expression parsing over the imported operator/async corpus
- structured member access, standalone ranges, and the unambiguous index/slice subset over the imported list/array/selfhost corpus, with opaque fallback still covering the remaining hard cases
- structured assignment targets for name, member, and index mutation paths over the imported behavior/list/selfhost corpus
- enforced pair-tuple contract over current syntax/frontend coverage, including `.0`/`.1`-only access, no tuple destructuring in bindings/params, no tuple field assignment, and new negative conformance fixtures wired into the frozen matrix
- page rollups now parse and lower through syntax/HIR for function-like owners and block-owning statements, with example/negative conformance coverage wired into the frozen matrix

## Next Milestones

1. Replace the remaining opaque expression/phrase cases with real parsing for collection forms, chain phrases, memory phrases, and the still-ambiguous generic-bracket versus index-bracket leftovers.
2. Add typed frontend checking against the copied selfhost language matrix.
3. Move package fingerprints from declaration-surface/source hashes to typed-HIR/API fingerprints.
4. Compile first-party host/io/window/input grimoires against the new frontend.
5. Replace the AOT placeholder with the first runnable backend.
6. Port `arcana-frontend`, `arcana-compiler-core`, and `arcana-selfhost-compiler` onto the new toolchain.

## Non-Goals Before Selfhost

- no language expansion
- no Git or registry dependencies
- no public bytecode compatibility promise
