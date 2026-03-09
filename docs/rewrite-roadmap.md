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
- explicit rejection of unsupported top-level syntax instead of silent skipping
- structured top-level declaration parsing for functions, async functions, systems, behaviors, lang items, intrinsic declarations, built-in forewords, generic/where headers, parameter modes, and impl declarations
- structured interior-member parsing for records, enums, traits, and impl bodies
- structured statement-block parsing for function-like bodies, including `defer`
- structured block-form `match` expression and pattern parsing for the imported enum/result corpus
- structured qualified phrase, collection literal, direct chain phrase, memory phrase, unary/binary operator, `>> await`, and `weave`/`split` expression parsing over the imported operator/async/selfhost corpus
- structured member access, pair tuple literals, generic-argument bracket applications, standalone ranges, and the unambiguous index/slice subset over the imported list/array/selfhost corpus, with opaque fallback still covering the remaining hard cases
- structured header attachments for qualified and memory phrases, with named entries and chain lines lowering through syntax/HIR instead of raw attached entries
- structured assignment targets for name, member, and index mutation paths over the imported behavior/list/selfhost corpus
- enforced pair-tuple contract over current syntax/frontend coverage, including `.0`/`.1`-only access, no tuple destructuring in bindings/params, no tuple field assignment, and new negative conformance fixtures wired into the frozen matrix
- page rollups now parse and lower through syntax/HIR for function-like owners and block-owning statements, with subject-scope validation, cleanup-subject reassignment diagnostics, and example/negative conformance coverage wired into the frozen matrix
- frontend semantic validation now includes unresolved `lang` item targets after workspace resolution
- impl header generic/lifetime params now survive syntax/HIR lowering instead of being discarded
- frontend semantic validation now includes declaration-surface type and lifetime resolution for params, returns, fields, enum payloads, trait defaults, impl headers, and inherited trait/impl method scopes
- frontend semantic validation now includes conservative body-level value resolution for locals, chain steps, memory constructors, rollup handlers, package/module-qualified value roots, and expression generic-argument type references over the imported selfhost corpus

## Next Milestones

1. Replace the remaining raw opaque-expression fallbacks and any leftover bracket ambiguities with fully structured parsing.
2. Extend the typed frontend from declaration-surface plus body-resolution checks into expression typing, ownership, and borrow/lifetime flow.
3. Move package fingerprints from declaration-surface/source hashes to typed-HIR/API fingerprints.
4. Compile first-party host/io/window/input grimoires against the new frontend.
5. Replace the AOT placeholder with the first runnable backend.
6. Port `arcana-frontend`, `arcana-compiler-core`, and `arcana-selfhost-compiler` onto the new toolchain.

## Non-Goals Before Selfhost

- no language expansion
- no Git or registry dependencies
- no public bytecode compatibility promise
