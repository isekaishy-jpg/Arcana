# Forewords v1 Scope

Status: `approved-pre-selfhost`

This scope defines the rewrite-era foreword contract as a three-tier system:
- built-in compiler-owned forewords
- basic package-defined forewords
- executable package-defined forewords backed by toolchain adapter products

## Core Syntax

Application forms:
- `#name`
- `#name[arg]`
- `#name[arg1, arg2]`
- `#name[key = value]`
- `#name[arg1, key = value]`

Definition forms:
- `foreword <qualified_name>:`
- `foreword handler <qualified_name>:`
- `foreword alias <alias_name> = <source_name>`
- `foreword reexport <alias_name> = <source_name>`

Comment cutover:
- `#` comments are removed
- `//` is the line and inline comment form
- `#` is reserved for forewords

## Tiers

### Built-in

Built-in forewords stay bare and permanently reserved:
- `#deprecated["message"]`
- `#only[...]`
- `#test`
- `#allow[...]`
- `#deny[...]`
- `#inline`
- `#cold`
- `#boundary[...]`
- `#stage[...]`
- `#chain[...]`

Built-ins remain compiler-owned even though they participate in the same internal registry model as user-defined forewords.

### Basic

Basic forewords are package-defined declarative forewords.

They may:
- validate typed payloads
- validate allowed targets
- declare retention policy
- participate in deterministic catalog/index/registration emission
- survive into retained runtime/tooling metadata when retention allows it

They may not:
- execute host/toolchain adapters
- rewrite declarations, modules, or package structure

### Executable

Executable forewords are package-defined forewords that require both:
- a declared `foreword handler`
- an executable toolchain product declared in `book.toml`

Executable forewords are opt-in across dependency edges:
- same-package executable forewords are always allowed
- dependency-provided executable forewords require `executable_forewords = true` on that dependency edge

## User-Defined Foreword Law

- User-defined forewords are always referenced by qualified name in v1.
- Bare names are reserved for built-ins.
- Visibility is `package` or `public`.
- Public forewords may be reexported by dependent packages with `foreword reexport ...`.
- Package-local aliases use `foreword alias ...`.
- No implicit transitive visibility exists for dependency forewords.

Definition fields:
- required:
  - `tier = basic | executable`
  - `targets = [...]`
  - `retention = compile | tooling | runtime`
- optional:
  - `visibility = package | public`
  - `phase = frontend`
  - `action = metadata | transform`
  - `payload = [...]`
  - `repeatable = true | false`
  - `conflicts = [...]`
  - `diagnostic_namespace = ...`
  - `handler = <qualified_name>`

Payload field types:
- `Bool`
- `Int`
- `Str`
- `Symbol`
- `Path`

Defaults:
- `visibility = package`
- `phase = frontend`
- `action = metadata`
- `repeatable = false`

Validation rules:
- definition and handler names must use the owning package root
- executable forewords must resolve a handler
- handler `protocol` is `stdio-v1` in v1
- duplicate non-repeatable user forewords on one target are rejected
- declared `conflicts` are enforced on one target
- basic forewords with `diagnostic_namespace = ...` emit an attachment-site warning lane that routes through builtin `#allow/#deny[...]`

## Targets

User-defined v1 targets are:
- `import`
- `reexport`
- `use`
- top-level declarations
  - `fn`
  - `record`
  - `obj`
  - `owner`
  - `enum`
  - `opaque_type`
  - `trait`
  - `behavior`
  - `system`
  - `const`
- `trait_method`
- `impl_method`
- `field`
- `param`

User-defined forewords do not add general statement or expression targets in v1.

Existing built-in-only local-target behavior remains:
- chain-statement `#chain[...]`
- attached header entries inside qualified/memory phrase blocks

Those built-in local-target carriers do not imply general user-defined statement/expression foreword support.

## Toolchain Adapter Products

Executable foreword handlers resolve package-owned toolchain products declared in `book.toml`:

```toml
[toolchain.foreword_products.rewrite]
path = "forewords/rewrite.cmd"
runner = "cmd"
args = ["/c"]
```

Product fields:
- required:
  - `path`
- optional:
  - `runner`
  - `args`

Rules:
- products are package-owned and resolved through the package graph, not runtime plugin loading
- `path` is relative to the owning package root
- adapter products are materialized into package-addressed `.arcana/foreword-products/...` cache artifacts before execution
- staged adapter artifacts copy same-stem sidecars so executable forewords run against the built package-graph view, not raw source paths
- if `runner` is omitted, the product path is launched directly
- if `runner` is present, `args` are passed to the runner before the product path
- product bytes and manifest fields participate in source fingerprints and publish snapshots

## Executable Metadata Contract

Executable forewords with `action = metadata` run after executable transform rewriting and before workspace resolution / ordinary semantic checking.

Metadata law:
- metadata forewords may emit diagnostics
- metadata forewords may emit retained metadata descriptors
- metadata forewords may emit registration rows
- metadata forewords may not replace owners
- metadata forewords may not replace directives
- metadata forewords may not append sibling symbols or impl blocks

## Executable Transform Contract

Executable forewords with `action = transform` run during frontend validation before workspace resolution and ordinary semantic checking.

Bounded transform law:
- transforms may rewrite the annotated owner
- top-level declaration targets may emit adjacent sibling symbols
- directive, field, param, and method targets may rewrite the owner but may not emit sibling declarations outside an adjacent top-level declaration slot
- generated sibling symbols carry explicit foreword provenance, a stable generation key, and post-expansion API fingerprint participation
- transforms may not mutate arbitrary other modules or packages
- symbol replacement must preserve the owning symbol name and symbol kind
- directive replacement must preserve the directive kind

Adapter protocol:
- request/response transport is structured JSON over stdio
- protocol/version id is `arcana-foreword-stdio-v1`
- request includes:
  - deterministic `cache_key`
  - package/module identity
  - resolved foreword definition data
  - applied payload arguments with rendered text plus typed `Bool` / `Int` / `Str` / `Symbol` / `Path` values
  - target snapshot
  - visible foreword catalog
  - toolchain version
  - dependency opt-in state
  - adapter artifact identity
- identical executable-foreword requests must replay from the in-process adapter cache instead of relaunching the toolchain product
- response may include:
  - diagnostics
  - `replace_owner`
  - `replace_directive`
  - `append_symbols`
  - `append_impls`
  - `emitted_metadata`
  - `registration_rows`

Adapter diagnostics may carry namespaced lint ids. Warning diagnostics are surfaced at the foreword application site and routed through builtin `#allow/#deny[...]` policy.

## Retention, Catalog, And Runtime Metadata

Retention classes:
- `compile`
- `tooling`
- `runtime`

Current carriage:
- checked workspaces expose a foreword catalog and per-target foreword index, including builtin forewords on the same catalog surface as user-defined forewords
- checked workspaces also expose deterministic foreword registration rows from basic forewords and executable adapters
- IR/AOT/runtime artifacts carry typed retained foreword metadata
- IR/AOT/runtime artifacts also carry foreword registration rows with stable target identity and generating-foreword provenance
- runtime package plans expose retained foreword queries over package metadata, including public-only retained metadata helpers
- runtime package plans expose registration-row queries alongside retained foreword metadata

Visibility rules:
- public/exported targets remain the default externally visible retained surface
- private retained metadata remains same-package/runtime-plan local

## Built-In Semantics Required In v1

- `#deprecated["message"]` emits call-site diagnostics
- `#allow/#deny[...]` control `deprecated_use` and namespaced foreword warning lanes
- `#test` drives deterministic `arcana test --list`
- `#inline` and `#cold` must not force runtime lowering and must flow into native Rust codegen hints
- `#boundary[...]` remains compile-time only and continues to target function/impl-method interop contracts

## CLI Support

- `arcana test --list <grimoire-dir>`
- `arcana foreword list <path> [--format json]`
- `arcana foreword show <qualified-name> <path> [--format json]`
- `arcana foreword index <path> [--public-only] [--format json]`

Outputs are deterministic and sorted in v1.

## Explicit Exclusions In v1

- `#derive`
- user-defined statement-level foreword targets
- user-defined expression-level foreword targets
- phases beyond `frontend`
- implicit bare-name imports for user-defined forewords
- arbitrary package-wide adapter mutation
- reuse of runtime plugin ABI/loading as the executable foreword mechanism

## Policy

Any deferred foreword item must be tracked in:
- `docs/specs/forewords/forewords/deferred-roadmap.md`

with:
- target plan
- trigger condition
- owner
- acceptance criteria
