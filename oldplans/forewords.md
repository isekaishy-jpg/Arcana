## Forewords Overhaul v3: Built-Ins, Basic Forewords, Executable Forewords

### Summary
- Rebuild Arcana forewords as a three-tier system:
- Built-in forewords: compiler-owned core behavior.
- Basic forewords: package-defined declarative metadata with validation, namespaced diagnostics, registration/index emission, and retention/query support, but no adapter execution or AST rewriting.
- Executable forewords: opt-in, adapter-backed advanced semantics and transforms.
- Keep the first target expansion limited to imports/reexports/use, declarations, methods, fields, and parameters. No statement/expression targets in this slice.

### Public Interfaces And Contract
- Add `foreword <qualified_name>:` for user-defined foreword definitions.
- Add `foreword handler <qualified_name>:` for executable foreword bindings only.
- Keep built-ins bare and reserved. User-defined forewords are always qualified.
- Add explicit foreword reexport/alias surface. No implicit transitive visibility.
- Add a typed foreword model across syntax/HIR/IR/runtime instead of flat string rows.
- Add a new toolchain-adapter product role for executable forewords. It is package-owned, resolved through the package graph, materialized into package-addressed `.arcana/foreword-products/...` cache artifacts, and invoked through a versioned structured-stdio protocol.
- Add CLI surfaces:
- `arcana test --list <grimoire-dir>`
- `arcana foreword list <path> [--format json]`
- `arcana foreword show <qualified-name> <path> [--format json]`
- `arcana foreword index <path> [--public-only] [--format json]`

### Behavior Model
- Built-ins stay on the same internal registry as user-defined forewords, but keep their compiler-owned semantics.
- Basic forewords are the default extensibility lane. They may:
- validate typed payloads and targets
- declare retention
- participate in namespaced `#allow/#deny`
- emit deterministic registration/index rows for tooling/build/runtime consumers
- surface through retained metadata/query APIs
- Basic forewords may not execute host adapters or rewrite AST/HIR/IR.
- Executable forewords require a declared handler and explicit consumer opt-in when they come from dependencies.
- Executable transform forewords may rewrite the annotated owner and emit adjacent sibling declarations plus adjacent impl blocks within the approved target slot. No arbitrary package-wide mutation.
- Generated siblings must carry explicit provenance, deterministic naming, and post-expansion API fingerprint participation.
- Runtime reflection is part of the first wave. Public/exported retained metadata is visible by default; private retained metadata is same-package only.
- `#inline` and `#cold` get real native semantics, and forewords must stop being a blanket blocker for direct lowering.
- Backfill missing built-in usefulness in the same program: real `#deprecated` call-site diagnostics, `#allow/#deny[deprecated_use]`, and deterministic `#test` discovery.

### Toolchain Adapter Contract
- Executable forewords do not reuse runtime plugin semantics.
- Adapter request/response is versioned and deterministic.
- Request includes resolved package_id-aware target identity, validated foreword definition, payload values, owner snapshot, visible foreword registry, opt-in state, and toolchain version.
- Response may return diagnostics, rewritten owner output, sibling declarations, retained metadata descriptors, and registration rows.
- Cache keys must include definition schema hash, handler binding, adapter protocol version, adapter artifact identity, visible dependency foreword registry, and consumer opt-in set.

### Test Plan
- Parser/HIR tests for definition syntax, handler syntax, qualified applications, field/param targets, and reexport visibility.
- Frontend tests for basic foreword validation, namespaced lint inheritance, registration/index emission, executable foreword opt-in failures, and transform provenance.
- CLI tests for `test --list`, `foreword list`, `foreword show`, and `foreword index`, including JSON output.
- IR/AOT/runtime tests for typed metadata carriage, retained metadata visibility, registration index queries, and package-id-stable lookup.
- Adapter tests for protocol versioning, deterministic replay, cache invalidation, and failure diagnostics.
- Native backend tests proving forewords no longer force runtime dispatch and that `#inline/#cold` behave as defined.

### Assumptions And Defaults
- Basic forewords are the preferred authoring model for most ecosystem/framework use cases.
- Executable forewords are advanced and explicitly gated.
- No statement/expression foreword targets in this slice.
- No bare-name import model for user-defined forewords in this slice.
- No hidden callable/closure semantics are introduced.
