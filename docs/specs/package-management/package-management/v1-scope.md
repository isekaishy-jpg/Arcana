# Arcana Package Management v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the pre-selfhost package/dependency contract for `book.toml`, `Arcana.lock`, the built-in local registry source, and resolved package identity.

## Goals

- Support deterministic versioned dependencies before selfhost without forcing source-language churn.
- Keep the current package manager ready for later named remote registries and Git sources without repainting manifests, lockfiles, or graph identity.
- Allow multiple versions of the same display package in one resolved graph while keeping source-visible imports explicit and stable.
- Keep native-product packaging/runtime identity compatible with versioned dependency graphs.

## Non-Goals

- No named remote registry transport before selfhost.
- No Git dependency materialization before selfhost.
- No same-member side-by-side direct use of two resolved versions of the same display package in v1.
- No source-visible version syntax in imports or module roots.
- No expansion from source-package publication into a remote artifact registry in v1.

## Dependency Source Taxonomy

- `Path`
  - enabled before selfhost
  - used for local workspace/path dependencies
- `Registry { registry_name }`
  - enabled only for the built-in `local` registry before selfhost
  - omitted `registry` means the default registry, which is `local` in v1
- `Git { url, selector }`
  - recognized by manifest parsing so the future grammar is fixed now
  - not enabled before selfhost

Phase-1 rule:
- only `Path` and `Registry { registry_name = "local" }` may resolve successfully
- recognized but unsupported remote-registry and Git dependency forms must fail with explicit gated diagnostics rather than schema rejection

## Manifest Surface

Top-level package fields:
- `name = "..."`
- `kind = "app" | "lib"`
- optional `version = "MAJOR.MINOR.PATCH"`

Enabled dependency forms:

```toml
[deps]
core = { path = "../core" }
foo = { version = "^1.2.3" }
foo_v1 = { package = "foo", version = "~1.4.0", registry = "local" }
```

Recognized-but-disabled future forms:

```toml
[deps]
foo = { version = "^1.2.3", registry = "central" }
bar = { git = "https://example.com/repo.git", rev = "abc123" }
```

Reserved dependency keys:
- `path`
- `package`
- `version`
- `registry`
- `git`
- `rev`
- `tag`
- `branch`
- `checksum`
- `native_delivery`
- `native_child`
- `native_plugins`

Version requirement grammar in v1:
- exact pins
- caret requirements
- tilde requirements

Out of scope in v1:
- prerelease/build metadata
- wildcard ranges
- inequality chains
- composite comparator sets

## Resolved Identity

- `package_name` remains the display/source-visible package name and module root.
- `package_id` is the unique resolved identity used by the package graph, lockfile, HIR/IR/AOT/runtime package records, build planning, cache keys, distribution metadata, and runtime native-product catalogs.

Phase-1 serialized ids:
- local/path package: `path:<normalized-relpath-from-workspace-root>`
- local-registry package: `registry:local:<package>@<version>`

Rules:
- a local/path package and a registry package may share the same display `package_name` in one resolved graph
- local/path packages with duplicate display names remain rejected
- source-visible import/module behavior stays rooted in `package_name`, not `package_id`

## Resolution Rules

- exact requirements resolve to that exact published version
- caret and tilde requirements resolve to the highest compatible published version in the enabled registry source
- once resolved, the exact selection is pinned in `Arcana.lock`
- if the existing lockfile pin still satisfies the manifest requirement and still exists locally, it is reused
- same-member rejection is a resolver policy:
  - if one member directly resolves two aliases to the same display `package_name` with different `package_id`, resolution fails
  - this restriction does not remove the internal ability for future same-member side-by-side support
- different members and transitive branches may resolve different versions of the same display package

## Local Registry And Publish

- `arcana publish <workspace-dir> --member <member>` publishes to the built-in machine-local `local` registry.
- `--member` is required in v1.
- only `kind = "lib"` packages are publishable in v1.
- publish walks the transitive local/path lib closure and publishes it in topological order.
- every published package in that closure must declare `version`.
- `ARCANA_HOME` is the root for machine-local source materialization and registry state.
- registry materializations live under a source-generic namespace so future registry and Git backends do not require resolver redesign.
- publication is immutable by `name@version`:
  - identical normalized content is a no-op
  - different content at the same `name@version` is an error
- published dependency metadata is canonical for registry resolution; resolution must not reinterpret stored packages as ad hoc live path dependencies
- optional dependency `checksum` is part of the recognized manifest surface and must be enforced when present

## Lockfile Contract

- the active lockfile schema is `Arcana.lock` `version = 4`
- readers may preserve explicit legacy compatibility, but rewrite-owned writers emit v4
- root metadata includes:
  - `workspace`
  - `workspace_root`
  - `order`
  - `workspace_members`
- package rows are keyed by `package_id`
- dependency edges are keyed by resolved `package_id`
- build and native-product metadata are keyed by `package_id`
- package rows always carry:
  - `name`
  - `kind`
  - `source_kind`
- source-specific package-row fields are preserved in v4:
  - path packages use `path`
  - registry packages may use `version`, `registry`, and `checksum`
  - git packages may use `git` and `git_selector`

Required v4 section families:
- `[packages]`
- `[dependencies]`
- `[builds]`

Rules:
- `workspace` is human-facing metadata
- `workspace_root` is the resolved identity anchor for the root package
- deterministic rendering/order remain part of the contract

## Native Products

- the public `arcana-cabi` descriptor remains display-name-facing in v1
- bundle/runtime metadata must carry `package_id` wherever graph-level disambiguation is required
- runtime child/plugin selection and dedupe must use `(package_id, product_name)` as the stable identity
- package-name-only helper lookup may remain only as an ambiguity-checking wrapper above package-id-aware APIs

## Future Compatibility Rules

Later support for named remote registries, Git sources, broader publishable package kinds, or same-member side-by-side direct multi-version use must not require changing:
- manifest grammar
- dependency source taxonomy
- `package_id` model
- source-aware lockfile v4 core layout
- the resolver/materializer separation

Any such expansion must remain additive to this contract rather than replacing it implicitly.
