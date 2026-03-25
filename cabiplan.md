# Plan: General Arcana DLL System on a Single `arcana-cabi` Foundation

## Summary
- Build a general DLL/product system as if `arcana_desktop` did not exist.
- Put all Arcana C-ABI ownership in one crate: `arcana-cabi`.
- Support three DLL roles in v1:
  - `export`: source-level Arcana DLLs
  - `child`: startup-loaded dependency DLLs
  - `plugin`: bundle-declared manually opened DLLs
- `arcana_desktop` is the first migrated `child` product, not a special case.
- Hard requirement: `child` and `plugin` DLLs must be real `cdylib` system DLLs with no staged Rust `std-*.dll` closure.

## Public Interfaces
- Add named native products to `book.toml`:
  ```toml
  [native.products.default]
  kind = "dll"
  role = "child"
  producer = "rust-cdylib"
  file = "arcana_desktop.dll"
  contract = "arcana.cabi.child.v1"
  rust_cdylib_crate = "../../crates/arcana-desktop-runtime-dll"
  sidecars = []
  ```

  ```toml
  [native.products.api]
  kind = "dll"
  role = "export"
  producer = "arcana-source"
  file = "my_api.dll"
  contract = "arcana.cabi.export.v1"
  ```

  ```toml
  [native.products.tools]
  kind = "dll"
  role = "plugin"
  producer = "rust-cdylib"
  file = "my_tools.dll"
  contract = "arcana.cabi.plugin.v1"
  rust_cdylib_crate = "../../crates/my-tools-dll"
  ```
- Replace dependency-edge `native_delivery = "dll"` with explicit selection:
  ```toml
  [deps.sdl]
  path = "../sdl"
  native_child = "default"
  native_plugins = ["tools"]
  ```
- Compatibility:
  - read legacy `native_delivery = "dll"` as deprecated alias for `native_child = "default"`
  - write only the new fields
- Root build selection:
  - keep `--target windows-exe`
  - keep `--target windows-dll` as the legacy export-DLL target
  - add `--product <name>` for named DLL products
  - `windows-dll` without `--product` resolves only if exactly one `role = "export"` product exists, otherwise fail

## `arcana-cabi` Contract
- `arcana-cabi` owns all C-ABI symbols, descriptors, headers, Windows loader helpers, error transport, buffer ownership, opaque handles, and versioning.
- Do not create a second ABI crate family. Other crates depend on `arcana-cabi`; they do not redefine ABI shapes.
- `arcana-cabi` exports one generic handshake:
  - `arcana_cabi_get_product_api_v1`
- The returned descriptor includes:
  - package name
  - product name
  - role
  - contract id and version
  - descriptor size
  - function table pointer
  - reserved fields for forward-compatible expansion
- Contract modules live inside `arcana-cabi`:
  - `arcana.cabi.export.v1`
  - `arcana.cabi.child.v1`
  - `arcana.cabi.plugin.v1`
- The future Arcana selfhost `cabi` grimoire mirrors these contracts; it does not invent a second ABI model.

## Runtime and Binding Semantics
- `export` products:
  - root-build products only
  - no runtime instance model
  - current `windows-dll` header/definition/export path is migrated onto `arcana-cabi.export.v1`
- `child` products:
  - dependency-scoped, startup-loaded
  - each dependency alias may select at most one `native_child`
  - the DLL is loaded once per bundle, but `create_instance` is called once per dependency binding
  - instance registration key is `(consumer_member, dependency_alias)`
  - if the same provider product is selected by multiple aliases, stage once, instantiate per alias
- `plugin` products:
  - bundle-declared and staged, but not opened at startup
  - loaded only through `arcana-cabi` runtime loader APIs
  - v1 load scope is bundle-declared products only, not arbitrary filesystem paths
  - runtime API supports:
    - enumerate available plugins
    - query by `(package, product)` or by contract id
    - open plugin instance
    - release plugin instance
  - every open creates a fresh instance
- Conflict rules:
  - `child` conflicts are edge-local, not bundle-global
  - `plugin` products may share a contract id; discovery returns all matches
  - duplicate staged output filenames in one bundle are a hard error

## Build, Lockfile, and Packaging
- Bump lockfile to v3.
- Add native product metadata section keyed by member and product name.
- Build entries become product-aware:
  - export product builds keyed by `(member, target, product)`
  - app/exe builds remain target-rooted, but include a native-product closure digest
- Build identity for release-grade bundles must include:
  - selected child bindings `(consumer, alias) -> (provider, product)`
  - selected plugin products
  - product role, contract id/version, output filename, producer kind
  - declared sidecars
  - produced DLL hashes
- `rust-cdylib` child/plugin products must be dependency-closed:
  - stage only the product DLL plus declared sidecars
  - if produced DLLs require undeclared non-system dependencies, fail packaging
  - never scavenge or stage Rust `std-*.dll` as an implicit workaround

## Migration and Docs
- Replace the hardcoded `AotRuntimeBinding::DesktopRuntimeDll` path with manifest-driven product resolution.
- Migrate `arcana_desktop` to one declared `role = "child"` product named `default`.
- Keep existing `windows-dll` behavior via the new `role = "export"` path during transition.
- Add/update docs in the same patch:
  - new approved scope for `arcana-cabi` and native product semantics
  - `spec-status.md` registry update
  - rewrite roadmap update
  - grimoire status/deferred note that future `cabi` grimoire mirrors `arcana-cabi`
  - migration note for `native_delivery = "dll"` to `native_child = "default"`

## Test Plan
- Manifest parsing and validation for named products and dependency selections.
- Lockfile/read-write roundtrips with v3 product metadata.
- Product-aware build caching and invalidation.
- Export DLL emission still produces correct header/definition/manifest outputs.
- Child product bundles stage the declared DLL, instantiate per dependency binding, and contain no `std-*.dll`.
- Plugin products can be enumerated, opened, used, and released from a packaged bundle.
- `arcana_desktop` showcase still packages and runs after migration.
- Legacy manifests with `native_delivery = "dll"` continue to read with deprecation coverage.

## Assumptions
- Windows-first only.
- Scope is Arcana-authored DLL products and Arcana-owned contracts.
- Opaque foreign DLL ecosystems stay on plain support-file staging until explicitly promoted into this model.
- No language changes, and no package-registry/git work in this plan.
