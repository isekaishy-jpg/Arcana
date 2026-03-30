# Future-Compatible Versioned Dependencies

## Summary
- Add versioned dependencies now through a **local registry source** backed by a machine-local materialization store under `ARCANA_HOME`, but structure the system so later **remote registries** and **git sources** fit the same resolver/materializer model.
- Support **exact**, **caret**, and **tilde** requirements now. Allow multiple versions in one resolved graph, but **same-member side-by-side direct versions stay rejected in phase 1**.
- Split **display package name** from **resolved package identity** everywhere. `package_name` stays source-visible; `package_id` becomes the unique identity for graph/build/cache/runtime/bundles.
- Include packages with `[native.products]` in phase 1. Version disambiguation lives in lock/build/bundle/runtime metadata, not in the public `arcana-cabi` descriptor.

## Implementation Changes
- **Manifest parsing**
  - Add optional top-level `version = "x.y.z"`.
  - Keep enabled dependency forms:
    - `foo = { path = "../foo" }`
    - `foo = { version = "^1.2.3" }`
    - `foo_v1 = { package = "foo", version = "~1.4.0", registry = "local" }`
  - Recognize but reject in phase 1 with explicit gated diagnostics:
    - `registry = "<non-local>"`
    - `git`, `rev`, `tag`, `branch`
  - Reserved dependency keys are fixed now: `path`, `package`, `version`, `registry`, `git`, `rev`, `tag`, `branch`, `checksum`, `native_delivery`, `native_child`, `native_plugins`.
  - Move source enablement out of `parse_manifest`; parsing accepts the future grammar, while resolution enforces phase support.

- **Source model**
  - Replace the ad hoc “published” idea with a real source taxonomy now:
    - `Path`
    - `Registry { registry_name }`
    - `Git { url, selector }`
  - Phase 1 enables only:
    - `Path`
    - `Registry { registry_name = "local" }`
  - Omitted `registry` means “default registry”; in phase 1 the default registry is `local`.

- **Resolved identity**
  - Add a typed internal `PackageId` and `SourceId`.
  - Serialize package ids in phase 1 as:
    - `path:<normalized-relpath-from-workspace-root>`
    - `registry:local:<package>@<version>`
  - Add `package_id` to:
    - workspace graph members
    - HIR workspace/resolved targets
    - IR packages
    - AOT artifacts
    - build statuses and cache metadata
    - distribution bundle metadata
    - runtime native-product catalog records
  - Keep `package_name` and `module_id` source-facing. `root_module_id` remains rooted in `package_name`, not `package_id`.

- **Resolver**
  - Build the resolved graph keyed by `package_id`, not display name.
  - Keep local member selection for CLI (`--member`) display-name based and limited to local/path members.
  - Allow a local/path package and a registry package with the same display name in one graph.
  - Keep local/path packages with duplicate display names rejected.
  - Treat same-member multi-version rejection as a **resolver policy**, not a data-model limitation:
    - if one member directly resolves two aliases to the same display `package_name` with different `package_id`, fail
    - the alias-to-`package_id` model remains capable of future side-by-side support

- **Local registry and publish**
  - Add `arcana publish <workspace-dir> --member <member>`.
  - `--member` is required in phase 1.
  - Only `kind = "lib"` is publishable in phase 1, but the stored metadata format remains kind-aware so apps/tools can be added later.
  - Publish walks the transitive local/path lib closure and publishes it in topo order.
  - Each published package in that closure must declare `version`.
  - Store layout is source-generic under `ARCANA_HOME`, not a bespoke one-off package folder:
    - registry materializations live under a registry namespace such as `sources/registry/local/...`
    - future git materializations get their own namespace without changing the resolver contract
  - Publish writes:
    - normalized source snapshot
    - canonical published package metadata record
    - content checksum
    - dependency requirements in source-generic form
    - native-product metadata and sidecar hashes
  - The canonical published metadata record is authoritative for registry dependencies. Do not rely on raw path-based `book.toml` semantics inside stored snapshots.
  - Republishing identical normalized content to the same `name@version` is a no-op. Different content at the same `name@version` is an error.

- **Lockfile**
  - Bump `Arcana.lock` to `version = 4`.
  - Reader accepts v1-v4 during transition. Writer emits v4 only.
  - Add `workspace_root = "<package_id>"`.
  - Keep `workspace = "<display-name>"` only as human-facing metadata.
  - Use exact sections:
```toml
version = 4
workspace = "app"
workspace_root = "path:."
order = ["path:.", "path:tools", "registry:local:foo@1.2.3"]
workspace_members = ["path:.", "path:tools"]

[packages."path:."]
name = "app"
kind = "app"
source_kind = "path"
path = "."

[packages."registry:local:foo@1.2.3"]
name = "foo"
kind = "lib"
version = "1.2.3"
source_kind = "registry"
registry = "local"
checksum = "sha256:..."

[dependencies."path:."]
foo = "registry:local:foo@1.2.3"
tools = "path:tools"
```
  - Preserve `[native_products]` and `[builds]`, but key them by `package_id`.

- **Build, cache, and artifacts**
  - Bump internal AOT artifact format from `arcana-aot-v7` to `arcana-aot-v8`.
  - `AotPackageArtifact` gains `package_id` and replaces package-name-only dependency identity with `direct_dep_ids`.
  - Cached artifact metadata and build planning must key on `package_id`.
  - Artifact/cache directory names must stop using bare `package_name`; use display name plus a short stable hash of `package_id`.
  - Build diagnostics render:
    - local/path packages as `name`
    - registry packages as `name@version`
    - ambiguous same-name cases with extra source/id context

- **Native products**
  - Keep `arcana-cabi` stable. Its descriptor continues to carry display `package_name`, `product_name`, `role`, and `contract_id`.
  - Keep `arcana-native-manifest-v3` stable in phase 1. It is per-root-artifact metadata and does not need graph-level disambiguation.
  - Bump the distribution bundle manifest from `arcana-distribution-bundle-v1` to `arcana-distribution-bundle-v2`.
  - Bundle/runtime metadata adds `package_id` to:
    - root native product row
    - `[[native_products]]`
    - `[[child_bindings]]`
    - optional `[runtime_child_binding]`
  - Runtime child/plugin selection and dedupe must key on `(package_id, product_name)`.
  - Any package-name-only plugin open helper becomes a wrapper that fails on ambiguity and delegates to a package-id-aware API.

- **Docs and std tooling**
  - Update `README.md`, `PLAN.md`, `docs/rewrite-roadmap.md`, and the relevant spec/status docs so the repo stops claiming “path-only deps”.
  - Add an approved package/dependency scope doc that freezes:
    - manifest `version`
    - source taxonomy
    - local registry phase
    - same-member rejection rule
    - lockfile v4
    - package-id/display-name split
    - native-product bundle identity rules
  - Update `std.manifest` to the active lockfile contract instead of leaving it behind the Rust driver.

## Test Plan
- Manifest parsing accepts enabled path/local-registry forms and preserves future source forms long enough to emit gated diagnostics.
- Unsupported remote registry and git deps fail with explicit “recognized but not enabled yet” errors.
- Resolver picks the highest compatible local-registry version for exact, caret, and tilde when no compatible pin exists.
- Existing compatible lockfile pins are reused.
- Same-member multi-version direct deps fail even under different aliases.
- Different members and transitive branches can resolve different versions of the same display package.
- Local/path and registry packages with the same display name can coexist without HIR, IR, lockfile, cache, or artifact collisions.
- `arcana publish` publishes transitive lib closure in topo order, no-ops on identical republish, and fails on conflicting republish.
- `Arcana.lock` v4 renders deterministically and round-trips.
- `std.manifest` parses v4 correctly.
- Internal artifact `arcana-aot-v8` round-trips with `package_id` and `direct_dep_ids`.
- Distribution bundle v2 round-trips and supports two native-product packages with the same display `package_name` at different versions in one bundle.
- Runtime native-product loader resolves child/plugin bindings by `package_id` and rejects package-name-only ambiguity.

## Assumptions and Future Compatibility
- Phase 1 supports only path deps and the built-in `local` registry source.
- The architecture must remain ready for:
  - named remote registries
  - git sources
  - later same-member side-by-side multi-version support
  - later publishability of non-lib package kinds
- Those later expansions must not require changing:
  - manifest grammar
  - source taxonomy
  - package-id model
  - resolver/materializer interface
  - lockfile v4 core layout
- Remote transport, auth, mirrors, git materialization, checksummed download policy, and dependency update/add commands are explicitly deferred.
