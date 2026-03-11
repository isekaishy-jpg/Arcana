# Rewrite Roadmap

Arcana is being rewritten under a hard language freeze until selfhost.
This roadmap tracks the current rewrite slice, not the full post-selfhost program.

## Current State

Completed foundation work:
- Rust workspace scaffold with isolated crates for syntax, HIR, frontend, package/build, IR, AOT, and CLI
- language freeze policy and CI freeze guard
- AnyBox policy guard over code-bearing paths
- explicit spec-status taxonomy plus pre-selfhost contract docs for page rollups, tuples, callable/context direction, and AnyBox ban
- clarified frozen-doc interpretation so domain scopes beat descriptive implementation limits, tuple docs stay forward-looking, and chain-style semantics are explicit rather than inherited from Meadow-era behavior
- seed import of frozen contract docs, conformance matrix, first-party grimoires, std, and source examples
- explicit recognition that imported `std` and carried grimoires are behavioral seed corpus, not rewrite architecture authority
- explicit std/grimoire governance docs so bootstrap-required surface and transitional carried roles are tracked in-repo instead of inferred from imports
- explicit review boundary: approved docs plus `crates/*` define rewrite status, while carried `std`/grimoires/examples/generated snapshots are migration corpus unless a current scope ratifies them
- deterministic path-only package graph, lockfile v1, and foundation build cache
- package/member rebuild planning now uses normalized HIR member fingerprints instead of raw source-byte hashes, so whitespace-only edits no longer trigger rebuild drift on the current planner path
- downstream rebuild invalidation now uses resolved API fingerprints on the real build path, including public impl-method surface, so equivalent exported type spelling no longer perturbs dependent rebuilds while callable API changes still propagate
- shared workspace/package HIR loading and symbol-based module and imported-name resolution over the current parser foundation
- explicit rejection of unsupported top-level syntax instead of silent skipping
- structured top-level declaration parsing for functions, async functions, systems, behaviors, lang items, intrinsic declarations, built-in forewords, generic/where headers, parameter modes, and impl declarations
- syntax-level contract enforcement for phrase comma-shape rules, chain-style families and reverse-introducer limits, memory-family allowlists, built-in foreword payload/target rules, validated `#stage/#chain` contract payloads, `#test` function constraints, `#boundary[target = "lua" | "sql"]` signature checks, and current-target `#only[...]` filtering
- structured interior-member parsing for records, enums, traits, and impl bodies
- structured statement-block parsing for function-like bodies, including `defer`
- structured block-form `match` expression and pattern parsing for the imported enum/result corpus
- structured qualified phrase, path and scalar literal leaves, collection literal, chain phrases with explicit style plus introducer plus connector structure and bound `with (...)` adapters, memory phrase, unary/binary operator, `>> await`, and `weave`/`split` expression parsing over the imported operator/async/selfhost corpus
- structured member access, pair tuple literals, generic-argument bracket applications, standalone ranges, and the unambiguous index/slice subset over the imported list/array/selfhost corpus, with the raw opaque-expression fallback removed from the syntax/HIR/frontend path
- structured header attachments for qualified and memory phrases, with named entries and chain lines lowering through syntax/HIR instead of raw attached entries
- structured assignment targets for name, member, and index mutation paths over the imported behavior/list/selfhost corpus
- enforced pair-tuple contract over current syntax/frontend coverage, including `.0`/`.1`-only access, no tuple destructuring in bindings/params, no tuple field assignment, and new negative conformance fixtures wired into the frozen matrix
- page rollups now parse and lower through syntax/HIR for function-like owners and block-owning statements, with subject-scope validation, cleanup-subject reassignment diagnostics, and example/negative conformance coverage wired into the frozen matrix
- frontend semantic validation now includes unresolved `lang` item targets after workspace resolution
- impl header generic/lifetime params now survive syntax/HIR lowering instead of being discarded
- frontend semantic validation now includes declaration-surface type and lifetime resolution for params, returns, fields, enum payloads, trait defaults, impl headers, and inherited trait/impl method scopes
- frontend semantic validation now includes recursive boundary-safe typing across nested record/enum surfaces for carried Lua/SQL boundary contracts
- frontend semantic validation now includes conservative body-level value resolution for locals, namespace-qualified member chains, enum variant constructors, module impl-method paths, structured chain stages and bound args, memory constructors, rollup handlers, package/module-qualified value roots, and expression generic-argument type references over the imported selfhost corpus
- frontend semantic validation now covers conservative expression typing plus ownership/borrow/lifetime flow on the current frontend path: type-shape checks for `if`/`while` conditions, unary/logical/bitwise/shift operators, tuple projection bases, slice bounds, explicit `&` / `&mut` place validation, lexical local-borrow conflict checks, direct-access/assignment rejection while locals are borrowed, `read` / `edit` / `take` call-site flow for resolved qualified phrases, use-after-move and move-while-borrowed diagnostics, and conservative return-lifetime / returned-local-borrow validation
- boundary-varietal example/negative conformance now covers Lua/SQL compile-time interop contracts, and ECS docs now preserve first-class scheduler/component direction without freezing general query authoring into the selfhost baseline
- current rewrite CLI remains intentionally narrow: `arcana check` is the compiler correctness gate, while `arcana test`, `arcana format`, and the smaller advisory `arcana review` layer are planned pre-selfhost tooling work rather than implied current commands

Imported-source guardrails:
- carried `std`, `winspell`, `spell-events`, and compiler grimoires are known hotspot imports and must be rebuilt against the rewrite's typed HIR, IR, package, and AOT architecture rather than preserved as implementation templates
- `std` itself is rewrite-owned first-party library surface; imports only provide carried behavior samples and transition corpus
- first-party grimoire responsibilities are frozen by role, not by carried Meadow-era package names
- imported standard-library contents do not automatically define correct standard-library layering; showcase/game/demo logic that leaked into `std` should move back into showcase or app grimoires unless it is explicitly ratified as general-purpose std surface
- first-party host/io/window/input/canvas/time/audio plus primitive graphics/text remain required before selfhost so the rewrite can show real apps/showcases and bootstrap required grimoires, but the old MeadowLang `winspell`/VM/bytecode/winit stack is not the mandated implementation path
- those host/app/runtime packages are real Rust rewrite commitments, not a promise that imported `std` or carried grimoires will keep satisfying them unchanged
- third-party Rust crates may still appear underneath the runtime/backend, but only as replaceable private implementation details rather than the defining first-party substrate
- parser opaque-expression fallback and typed resource handles are different concerns: the former should be eliminated from the frontend path before selfhost, while current typed `Window` / `Image` / `Audio*` std handles are only bootstrap seams and must be revisited once the rewrite-owned runtime/backend resource model is settled
- ECS scheduling/components remain first-party language/runtime surface during the rewrite; they are not merely showcase helpers carried from MeadowLang
- carried `std.app` fixed-step helpers and `std.tooling` planner helpers are convenience corpus, not rewrite-owned architecture until a scope explicitly ratifies them
- carried generated snapshot artifacts such as direct-emit spec shards are not rewrite architecture evidence by themselves; use them to track migration work, not to override approved contracts or crate-side implementation status

## Next Milestones

1. Completed: replace the remaining raw opaque-expression fallbacks and leftover grouped-comma bracket ambiguities with fully structured parsing on the current frontend path.
2. Completed: freeze the rewrite-owned first-party package split and bootstrap ledgers: host-core in `selfhost-host/v1-scope`, app/runtime substrate in `app-substrate-v1-scope`, std scope/status, grimoire scope/status, ECS/behaviors kept first-party, and carried convenience layers explicitly left unratified.
3. Completed: extend the typed frontend from declaration-surface plus body-resolution checks into conservative expression typing, ownership, and borrow/lifetime flow on the current frontend path.
4. Completed: move package fingerprints from declaration-surface/source hashes to typed-HIR/API fingerprints.
5. Compile rewrite-owned first-party host/io plus app/runtime packages and their consumer grimoires against the new frontend, trimming imported `std` back to approved responsibilities and moving showcase-specific helpers out where needed.
6. Replace the AOT placeholder with the first runnable backend sufficient for carried `hello`, host-tool, window/input/canvas showcase proof, and basic audio smoke proof, with rewrite-owned host/window/input/canvas/events/time/audio/graphics/text seams implemented in Rust as real runtime commitments rather than compatibility wrappers.
7. Add first-party `arcana test` and `arcana format` to the rewrite-owned CLI before selfhost; keep `arcana check` as the correctness gate and delay broader advisory-review heuristics until the rewrite has enough showcase-scale Arcana corpus to justify them.
8. Port `arcana-frontend`, `arcana-compiler-core`, and `arcana-selfhost-compiler` onto the new toolchain.

## Non-Goals Before Selfhost

- no language expansion
- no Git or registry dependencies
- no public bytecode compatibility promise
