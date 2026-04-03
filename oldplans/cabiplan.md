# Plan: Unified Arcana Native Products and C ABI

## Summary

This is the single master plan for Arcana’s native ABI and DLL/product system. It replaces the old `cabiplan` and subsumes the exported `edit` work into Phase 1. There is no separate `edit` plan.

The end state is:
- `arcana-cabi` is the only owner of Arcana’s foreign ABI
- typed exported C symbols are the primary interop surface
- one generic descriptor entrypoint exposes product identity, contract identity, and role-specific ops
- exported `edit` is a first-class write-back contract, never public `final_args`
- native products support three roles: `export`, `child`, and `plugin`
- `child` and `plugin` providers are real `cdylib` system DLLs with no staged Rust `std-*.dll` closure
- the design is ready for C/Lua/SQL-style interop without revisiting the ABI shape again

## Public Interfaces

- Add named native products to `book.toml`:
```toml
[native.products.default]
kind = "dll"
role = "child"
producer = "arcana-source"
file = "arcwin.dll"
contract = "arcana.cabi.child.v1"
sidecars = []

[native.products.api]
kind = "dll"
role = "export"
producer = "arcana-source"
file = "my_api.dll"
contract = "arcana.cabi.export.v1"

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
  - read legacy `native_delivery = "dll"` as a deprecated alias for `native_child = "default"`
  - write only the new fields

- Root build selection:
  - keep `--target windows-exe`
  - keep `--target windows-dll` as the legacy export-DLL target name
  - add `--product <name>` for named native products
  - `windows-dll` without `--product` resolves only if exactly one `role = "export"` product exists

- `arcana-cabi` exports one generic handshake:
  - `arcana_cabi_get_product_api_v1`

- The generic descriptor is fixed as:
  - `descriptor_size`
  - `package_name`
  - `product_name`
  - `role`
  - `contract_id`
  - `contract_version`
  - `role_ops`
  - reserved fields

- Metadata strings in descriptors are static UTF-8, NUL-terminated, process-lifetime pointers.

- `role_ops` points to a role-specific ops table.
- `export` products use:
  - `ArcanaCabiExportOpsV1 { ops_size, exports, export_count, last_error_alloc, owned_bytes_free, owned_str_free, reserved }`
  - `ArcanaCabiExportEntryV1 { export_name, routine_key, symbol_name, return_type, params, param_count }`
  - `ArcanaCabiExportParamV1 { name, source_mode, pass_mode, input_type, write_back_type }`

- `child` and `plugin` products share an instance-bearing base shape:
  - `ArcanaCabiInstanceOpsV1 { ops_size, create_instance, destroy_instance, reserved }`
  - `role_ops` for `child` points at `ArcanaCabiChildOpsV1`, whose first field is that shared base and which adds `run_entrypoint`, `last_error_alloc`, and `owned_bytes_free`
  - `role_ops` for `plugin` points at `ArcanaCabiPluginOpsV1`, whose first field is that shared base and which adds `describe_instance`, `use_instance` (bytes request to owned-bytes response), `last_error_alloc`, and `owned_bytes_free`
  - `child` and `plugin` still differ in loader and packaging semantics, not in the existence of instances
  - `arcana-source` child/plugin products are emitted through one generic instance-product backend; no desktop-specific runtime crate is required

- Stable helper symbols owned by `arcana-cabi`:
  - `arcana_cabi_get_product_api_v1`
  - `arcana_cabi_last_error_alloc_v1`
  - `arcana_cabi_owned_bytes_free_v1`
  - `arcana_cabi_owned_str_free_v1`

- The export ops table exposes pointers to the same helper functions for generic loaders.

## Contract and Runtime Semantics

- `arcana-cabi` owns all foreign ABI shapes: headers, descriptors, helper symbols, owned/view structs, opaque handles, error transport, and versioning.
- No second ABI crate family is introduced.

- `export` products:
  - root-build products only
  - no runtime instance model
  - typed exported routine symbols remain the primary foreign-call API
  - the descriptor is for discovery, versioning, and helper ownership, not generic dynamic invocation

- `child` products:
  - dependency-scoped and startup-loaded
  - each dependency alias selects at most one `native_child`
  - a provider DLL is staged once per bundle
  - `create_instance` is called once per dependency binding
  - instance registration key is `(consumer_member, dependency_alias)`
  - distribution manifests may include an explicit `[runtime_child_binding]` naming the root child binding that should handle `windows-exe` entrypoint execution
  - packaging fails if the bundle root selects more than one direct child binding because runtime-provider selection would be ambiguous
  - distribution bundles record explicit `[[native_products]]` and `[[child_bindings]]` entries so startup activation is manifest-driven instead of inferred from directory contents
  - `windows-exe` runtime-dispatch entry now resolves through the activated child-provider table first and only falls back to the in-process host when no child runtime provider is present

- `plugin` products:
  - bundle-declared and staged, but not opened at startup
  - loaded only through Arcana runtime loader APIs
  - v1 load scope is bundle-declared products only
  - runtime API supports enumerate, query, open, describe/use, and release
  - every open creates a fresh instance
  - plugin enumeration/open scope comes from bundle manifest `[[native_products]]`, not arbitrary bundle directory scanning

- Conflict rules:
  - `child` conflicts are edge-local, not bundle-global
  - `plugin` products may share a contract id
  - duplicate staged output filenames in one bundle are a hard error

- `arcana.cabi.export.v1` freezes exported `edit` semantics:
  - call inputs are passed normally
  - return value is written through `out_result` when non-`Unit`
  - each `edit` param uses `in value + out_<name>` pointer
  - `Unit` `edit` emits metadata but no `out_<name>` pointer
  - public JSON/native export results expose only explicit write-backs, never `final_args`

- Owned/view C types are part of the contract:
  - params use `ArcanaStrView` and `ArcanaBytesView`
  - returns and `edit` write-backs use `ArcanaOwnedStr` and `ArcanaOwnedBytes`
  - strings are UTF-8 byte sequences with `ptr + len`
  - owned buffers are freed only through `arcana-cabi` helpers
  - pair structs keep the current split: `ArcanaPairView__*` for params and `ArcanaPairOwned__*` for returns/write-backs

- Internal runtime execution may keep using `RoutineExecutionOutcome { value, final_args }`, but `final_args` is private implementation detail only.

- `arcana-runtime-json-abi-v3` and `arcana-native-manifest-v3` are projections of `arcana.cabi.export.v1`.
- They mirror the same param modes, pass modes, types, and write-back semantics.
- They are not independent ABI owners.

## Implementation Phases

1. Phase 1: `arcana-cabi` export foundation and exported `edit`
- Create `arcana-cabi` and move all export-ABI ownership there.
- Add the minimum product-system pieces required for export descriptors:
  - named `role = "export"` products
  - `--product <name>` for export builds
  - export build/cache identity keyed by `(member, target, product)`
  - export bundle metadata includes `product_name`, `role`, `contract_id`, and `contract_version`
- Re-route current `windows-dll` generation through `arcana.cabi.export.v1`.
- Replace public export `final_args` with explicit `write_backs` in JSON ABI v3 and runtime native ABI.
- Upgrade AOT export metadata from `is_edit` to explicit `source_mode`, `pass_mode`, `input_type`, and `write_back_type`.
- Remove the root-level `RuntimeDispatch` bailout for exported `edit`.
- Extend direct lowering so direct export roots and direct callee chains can propagate edit write-backs through the current direct subset.
- Keep runtime-dispatch fallback only for documented current direct-subset limitations, including unsupported routine shapes, recursion/in-progress lowering, attached/rollup forms, signature mismatch, and non-`Name` write-back targets.
- Generated headers, manifests, and codegen all consume the same `arcana-cabi` export metadata.

2. Phase 2: full native product system and removal of Rust `dylib` dependency
- Add full `child` and `plugin` product selection and resolution.
- Replace hardcoded runtime-provider handling with manifest-driven product resolution.
- Migrate `arcana_desktop` to one declared `role = "child"` product named `default`.
- Make build and lockfile identity fully product-aware.
- Require `child` and `plugin` providers to be real `cdylib` system DLLs.
- Packaging stages only the product DLL plus declared sidecars.
- Packaging validates `rust-cdylib` products against their declared cabi package/product/role/contract/version before staging succeeds.
- If a produced DLL requires undeclared non-system dependencies, packaging fails.
- Never scavenge or stage Rust `std-*.dll` as a workaround.

3. Phase 3: cleanup and cutover
- Remove legacy ABI ownership from `arcana-runtime` and `arcana-aot`.
- Leave JSON ABI as a tooling/debug projection only.
- Keep `windows-dll` as a compatibility target name only; it emits the new export contract.
- Update approved docs in the same patch series:
  - new `arcana-cabi` and native-product scope
  - `spec-status.md` registry
  - rewrite roadmap
  - migration note for `native_delivery = "dll"`

## Build, Lockfile, and Packaging

- Bump lockfile to v3.
- Add native product metadata keyed by member and product name.
- Export product builds are keyed by `(member, target, product)`.
- App/exe builds remain target-rooted but include a native-product closure digest.
- Release-grade bundle identity includes:
  - optional `root_native_product`
  - selected child bindings
  - selected plugin products
  - `native_product_closure`
  - optional `runtime_child_binding`
  - product role, contract id/version, output filename, producer kind
  - declared sidecars
  - produced DLL hashes
  - distribution manifest entries for `native_products` and `child_bindings`

## Test Plan

- Manifest parsing and validation for named products and dependency selections.
- Lockfile read/write roundtrips with v3 product metadata.
- Product-aware build caching and invalidation.
- Descriptor tests for package/product/role/contract identity.
- Export ops-table tests for metadata and helper pointers.
- Header, descriptor, and manifest agreement tests.
- `edit` tests for:
  - single and multiple `edit` params
  - mixed `read` / `take` / `edit`
  - `Int`, `Bool`, `Str`, `Bytes`, nested `Pair`, and `Unit`
- JSON ABI v3 and runtime native ABI tests that return only explicit `write_backs`.
- Direct-lowering tests for exported `edit` roots and direct caller-to-callee write-back propagation.
- Fallback tests proving the documented current direct-subset boundaries, including non-`Name` write-back targets, stay on runtime dispatch.
- C harness test that compiles against the generated header and calls an `edit` export.
- Ownership tests for `ArcanaOwnedStr` and `ArcanaOwnedBytes` using cabi free helpers.
- Export DLL emission still produces correct header, definition, descriptor, and manifest outputs.
- Child product bundle tests prove declared DLL staging, per-binding instancing, and no `std-*.dll`.
- Plugin product tests prove enumerate, open, use, and release from a packaged bundle.
- `arcana_desktop` showcase still packages and runs after migration.
- Legacy `native_delivery = "dll"` inputs continue to read with deprecation coverage.

## Assumptions

- Windows-first only.
- Scope is Arcana-authored DLL products and Arcana-owned contracts.
- Opaque foreign DLL ecosystems remain on plain support-file staging until explicitly promoted into this model.
- No language changes, package-registry work, or Lua/SQL bindings are in scope here; this plan provides the stable ABI foundation they need.
- This plan is the only execution authority for native ABI and product work; there is no separate `edit` implementation plan.
