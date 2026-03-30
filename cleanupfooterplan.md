# Cleanup Footer Replacement, Narrowed For Future Headed Regions

## Summary
- Replace `[subject, handler]#cleanup` with a first-class **cleanup footer**:
  - `-cleanup`
  - `-cleanup[target = name]`
  - `-cleanup[target = name, handler = path]`
- `-cleanup` still means **whole owning scope cleanup**, not single-binding cleanup by default.
- Do **not** define a generic `-name` footer family in this patch.
  - Cleanup is frozen now as a **cleanup footer** only.
  - Future headed regions may later use `-cleanup` or other `-name` forms as rides/modifiers/overrides in other syntactic contexts.
  - This patch must leave room for that without requiring another cleanup rewrite.
- Make cleanup first-class through syntax, HIR, frontend, IR, runtime, and native direct lowering. Footer presence alone must no longer force runtime-dispatch fallback.

## Contract Changes
- Rewrite [docs/specs/page-rollups/page-rollups/v1-scope.md](/c:/Users/Weaver/Documents/GitHub/Arcana/docs/specs/page-rollups/page-rollups/v1-scope.md) into the active cleanup-footer contract and update [POLICY.md](/c:/Users/Weaver/Documents/GitHub/Arcana/POLICY.md), [docs/specs/spec-status.md](/c:/Users/Weaver/Documents/GitHub/Arcana/docs/specs/spec-status.md), [docs/specs/objects/objects/v1-scope.md](/c:/Users/Weaver/Documents/GitHub/Arcana/docs/specs/objects/objects/v1-scope.md), [docs/arcana-v0.md](/c:/Users/Weaver/Documents/GitHub/Arcana/docs/arcana-v0.md), and [conformance/selfhost_language_matrix.toml](/c:/Users/Weaver/Documents/GitHub/Arcana/conformance/selfhost_language_matrix.toml) in the same freeze-exception patch.
- Freeze the current placement narrowly:
  - `-cleanup` is valid only as an attached post-owner footer after the owning block dedents.
  - The language does **not** claim that all future `-name` forms share this placement.
- Add `std.cleanup.Cleanup[T]` with:
  - `fn cleanup(take self: T) -> Result[Unit, Str]`
  - `lang cleanup_contract = std.cleanup.Cleanup`
- Bare `-cleanup` covers every cleanup-capable owning binding activated in the owning scope.
  - Routine scope: owning params plus eligible locals.
  - Block-owning statement scope: eligible locals in that owner’s block scope.
  - `read`/`edit`/ref-style params and non-owning bindings are excluded.
- Explicit `handler = path` remains an override.
  - `target` is required.
  - target must still be cleanup-capable
  - handler must be synchronous, statically resolved, one-arg, compatible, and return `Result[Unit, Str]`
- Allow one bare footer plus targeted overrides with unique targets.
  - Targeted overrides replace bare default cleanup for those bindings.
- Loop semantics are body-scope cleanup, not loop-statement epilogue.
- `defer` still runs before owner cleanup footer work.

## Implementation Changes
- Keep the implementation **cleanup-specific**, not a generic dash-family substrate.
  - Use cleanup-specific syntax/HIR/IR/runtime nodes such as `CleanupFooter` / `CleanupFooterPolicy`.
  - Do not introduce a generic `DashDecl` or generic headed-region AST now.
- Parser:
  - recognize exact `-cleanup...` only in attached post-owner position
  - reject old `#cleanup` with deterministic migration diagnostics
  - reject other `-name` forms in this attached footer position as unsupported here, without claiming they are globally invalid language forms
  - keep future headed-region contexts free to introduce their own `-name` uses later
- Frontend:
  - normalize footers into owner cleanup policy keyed by binding id
  - model bare coverage plus targeted override map
  - require concrete resolved `Cleanup[T]` impls for default cleanup
  - extend move/reassign-after-activation enforcement to all covered bindings, not just named old rollup subjects
- IR/runtime:
  - lower cleanup by binding id and resolved cleanup callee
  - bare cleanup covers all eligible activated bindings except explicit overrides
  - execute in reverse lexical activation order of the final covered set
  - run on fallthrough, `return`, `break`, `continue`, and `?`
  - `Err(Str)` is fail-fast and overrides the original exit path
- Native lowering:
  - stop rejecting routines/statements merely because cleanup footers exist
  - lower cleanup through explicit cleanup scopes in the direct native lane
  - support fallthrough, `return`, `break`, `continue`, and `?`
  - lower both default `Cleanup[T]` calls and explicit handler overrides as ordinary calls
  - teach the direct lowerer the concrete cleanup-impl call path so cleanup footers are not blocked just because the default callee came from a trait/impl surface
- `std`:
  - add `std.cleanup` and reexport it from `std.book`
  - add initial `Cleanup[T]` impls only for first-party types with existing explicit close/stop semantics:
    - `std.fs.FileStream`
    - `std.window.Window`
    - `std.audio.AudioDevice`
    - `std.audio.AudioPlayback`
- Reference docs:
  - update [docs/reference/page-rollups/cleanup_footer_replacement_design.md](/c:/Users/Weaver/Documents/GitHub/Arcana/docs/reference/page-rollups/cleanup_footer_replacement_design.md) so it no longer overclaims a generic `-name` footer family and instead records cleanup footer as the current replacement plus headed-region compatibility constraints.

## Test Plan
- Parser/tests:
  - parse all three cleanup footer forms
  - parse bare plus targeted override stack
  - reject old `#cleanup`
  - reject malformed payloads, duplicate fields, duplicate bare footer, duplicate target, bad attachment
  - reject non-`cleanup` dash forms in attached footer position with “unsupported here / reserved for other dash contexts” diagnostics
- Frontend/tests:
  - bare cleanup covers whole owning scope as defined above
  - reject non-owning targets and missing/non-concrete `Cleanup[T]` impls
  - reject handler async/arity/return/access-mode mismatch
  - reject move/reassign after activation for both bare and targeted cleanup
- Runtime/tests:
  - fallthrough, `return`, `break`, `continue`, and `?`
  - nested ordering, `defer` before cleanup footer, owner/object cleanup after local cleanup
  - fail-fast `Err(Str)` overriding the original exit
  - bare cleanup plus targeted override
  - loop-body per-iteration cleanup
- Native lowering/tests:
  - routine with `-cleanup` remains directly lowered when callees are otherwise supported
  - early return and loop `continue` emit cleanup correctly
  - default `Cleanup[T]` and explicit handler overrides match runtime behavior
- Conformance/tests:
  - replace page-rollup fixtures with cleanup-footer fixtures
  - include at least one positive bare `-cleanup` case over a std handle type

## Assumptions And Defaults
- This is a corrective pre-selfhost freeze exception.
- Cleanup is still a **footer in its core form**.
- Future headed regions are out of scope for this patch, but this patch must not freeze a broader “all `-name` forms are footers” rule.
- If future headed regions later use `-cleanup` as a ride or override, that will be a separate syntactic/contextual feature layered on top of the narrower cleanup-footer contract, not a rewrite of cleanup semantics.
