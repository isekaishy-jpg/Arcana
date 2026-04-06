# Native Products and C ABI v1 Scope

## Status

This scope is `approved-pre-selfhost`.

## Purpose

This document freezes the rewrite-owned native product and foreign ABI direction for the current Windows-first lane.

It defines:
- `crates/arcana-cabi` as the only owner of Arcana's foreign ABI contract
- the native product roles `export`, `child`, `plugin`, and `binding`
- the generic descriptor/ops-table shape
- the binding import/callback contract for package-owned native OS seams
- the exported `edit` write-back contract
- the rule that generated headers, runtime JSON ABI, and native bundle manifests are projections of the cabi contract rather than independent ABI owners

## Contract Owner

- `crates/arcana-cabi` owns the public foreign ABI contract.
- Contract-owned surface includes:
  - product descriptor ids and helper symbol names
  - product role ids
  - owned/view value structs
  - export metadata shape
  - instance ops shape for `child`, `plugin`, and `binding`
  - contract ids and versions
- `arcana-runtime`, `arcana-aot`, and package tooling may project, consume, or serialize this contract, but they must not define a separate competing public ABI.

## Product Roles

### `export`

- Root-build product only.
- Exposes typed exported routine symbols as the primary foreign-call surface.
- Uses the generic product descriptor for discovery, versioning, and helper ownership.
- Does not use a runtime instance model.

### `child`

- Dependency-scoped native child loaded from bundle-declared child bindings.
- Each dependency alias selects at most one `native_child`.
- Activation is manifest-driven, not inferred from directory contents.
- Instances are keyed by `(consumer_member, dependency_alias)`.
- Distribution manifests may include an optional `[runtime_child_binding]` table naming the root child binding that should handle `windows-exe` entrypoint execution.
- Packaging must fail when the bundle root selects more than one direct child binding, because runtime child selection would be ambiguous.
- `windows-exe` entrypoint execution must first consult the activated child-binding table and only fall back to the in-process host when no runtime child binding is present.

### `plugin`

- Bundle-declared plugin staged into the bundle but not activated at startup.
- Opened only through Arcana runtime loader APIs.
- Enumeration/open scope comes from bundle manifest product rows, not arbitrary bundle directory scans.
- Every open yields a fresh instance.

### `binding`

- Library-scoped native binding product used by a package's own `native fn` and `native callback` declarations.
- In v1, a binding-owning library package declares one default `binding` product in `book.toml`.
- Source in that same package lowers against that default binding product automatically; consumer packages depend on the library package only and do not configure the sidecar directly.
- Binding products are activated per package id, not by dependency-edge aliases.
- Binding products are the generic native OS/callback seam for external libraries; they must not become a provider-style dynamic service surface or a text-only escape hatch.
- The v1 binding contract id is `arcana.cabi.binding.v1`.
- In the current proof lane, the dedicated Windows binding crate is the only low-level `windows-sys` leaf for library-owned OS work. Existing rewrite runtime host/native-loader calls and Windows-only test harness usage remain transitional plumbing outside that package binding boundary.

## Native Product Declaration

- Named native products are declared under `[native.products.<name>]` in `book.toml`.
- Current product kind is `dll`.
- Current roles are `export`, `child`, `plugin`, and `binding`.
- Current producers are:
  - `arcana-source`
  - `rust-cdylib`

## Root Build Selection

- `windows-exe` remains the executable target.
- `windows-dll` remains the compatibility target name for root DLL products.
- `--product <name>` selects a named root DLL product.
- `windows-dll` without `--product` resolves only when the member has exactly one `role = "export"` product, or no declared products and therefore the implicit default export lane applies.
- Non-export root DLL products require explicit `--product`.

## Generic Descriptor

- Products export `arcana_cabi_get_product_api_v1`.
- The descriptor includes:
  - `descriptor_size`
  - `package_name`
  - `product_name`
  - `role`
  - `contract_id`
  - `contract_version`
  - `role_ops`
  - reserved fields
- Metadata strings are static UTF-8, NUL-terminated, and process-lifetime.

## Role Ops

### `export`

- `role_ops` points at `ArcanaCabiExportOpsV1`.
- Export entries carry:
  - `export_name`
  - `routine_key`
  - `symbol_name`
  - `return_type`
  - param rows with:
    - `name`
    - `source_mode`
    - `pass_mode`
    - `input_type`
    - optional `write_back_type`

### `child`

- `role_ops` points at `ArcanaCabiChildOpsV1`.
- Its first field is the shared `ArcanaCabiInstanceOpsV1` base.
- It additionally provides:
  - `run_entrypoint`
  - `last_error_alloc`
  - `owned_bytes_free`

### `plugin`

- `role_ops` points at `ArcanaCabiPluginOpsV1`.
- Its first field is the shared `ArcanaCabiInstanceOpsV1` base.
- It additionally provides:
  - `describe_instance`
  - `use_instance`
    - request: raw bytes (`request_ptr`, `request_len`)
    - result: owned response bytes plus `out_len`
  - `last_error_alloc`
  - `owned_bytes_free`

### `binding`

- `role_ops` points at `ArcanaCabiBindingOpsV1`.
- Its first field is the shared `ArcanaCabiInstanceOpsV1` base.
- It additionally provides:
  - `imports`
  - `callbacks`
  - `register_callback`
  - `unregister_callback`
  - `last_error_alloc`
  - `owned_bytes_free`
  - `owned_str_free`
- Binding import metadata rows carry:
  - `name`
  - `symbol_name`
  - `return_type`
  - param rows with:
    - `name`
    - `source_mode`
    - `pass_mode`
    - `input_type`
    - optional `write_back_type`
- Binding callback metadata rows carry:
  - `name`
  - `return_type`
  - param rows with the same typed metadata shape

## Export Contract

- Exported routine symbols remain the primary interop surface for ordinary DLL exports.
- `arcana.cabi.export.v1` freezes exported `edit` as:
  - normal input argument passing for the incoming value
  - ordinary `out_result` for the routine result when non-`Unit`
  - `in value + out_<name>` for each `edit` param
  - metadata-only `edit Unit` with no `out_<name>` pointer
- Public exported results must expose only explicit write-backs, never raw `final_args`.

## Binding Contract

- `native fn` is the package-scoped imported host-call surface for binding-owning libraries.
- `native callback` is the package-scoped explicit callback registration surface for those same libraries.
- `intrinsic fn` remains the trusted std/kernel-only surface and must not be reused for package-owned OS bindings.
- Binding callbacks are explicit typed registrations only:
  - no arbitrary symbol lookup from native code into Arcana
  - no event-queue-only fallback standing in for callback registration
  - no package-name-specific runtime hooks
- Binding products use `ArcanaCabiBindingValueV1` for typed import/callback transport.
- Supported v1 binding value tags are:
  - `Int`
  - `Bool`
  - `Str`
  - `Bytes`
  - `Opaque`
  - `Unit`
- `Str` and `Bytes` inputs use views.
- `Str` and `Bytes` outputs use owned buffers released through cabi-owned free helpers.
- Opaque values may cross only as package-owned opaque handle types declared in source by a binding-owning package.
- Binding metadata carries `write_back_type` rows for parity with the generic param description shape, but the current v1 binding proof slice does not require `edit` import write-backs to be a public design center.

## Value Ownership

- Params use `ArcanaStrView` and `ArcanaBytesView`.
- Returns and export `edit` write-backs use `ArcanaOwnedStr` and `ArcanaOwnedBytes`.
- Strings are UTF-8 bytes with `ptr + len`.
- Owned buffers are released only through cabi-owned helper functions.

## Projections

- `arcana-runtime-json-abi-v3` is a tooling/debug projection of the cabi export contract.
- `arcana-native-manifest-v3` is a bundle metadata projection of the cabi product contract.
- These projections must mirror the same export metadata, binding metadata, and ownership semantics; they do not own the ABI.

## Packaging and Identity

- Build/cache identity must be product-aware.
- Lockfile state must retain native product metadata keyed by member and product.
- Distribution manifests must retain:
  - optional `[root_native_product]` describing the selected root DLL product when the bundle root is itself a native product
  - `native_product_closure`
  - optional `[runtime_child_binding]`
  - `[[native_products]]`
  - `[[child_bindings]]`
- Distribution manifests may additionally retain `[[package_assets]]` rows keyed by `package_id` plus staged `asset_root` so packaged runtimes can resolve package-owned bundled assets without source-path assumptions.
- Packaging must stage only the declared product file plus declared sidecars.
- Packaging must also stage package-owned `assets/` trees under deterministic package-id-keyed bundle roots when the built member or its dependency closure exports such assets.
- Packaging must validate declared `rust-cdylib` products against their cabi package/product/role/contract/version descriptor before staging succeeds.
- Packaging must fail when a produced DLL depends on undeclared non-system DLLs.
- Rust `std-*.dll` scavenging is forbidden.

## Legacy Compatibility

- `native_delivery = "dll"` is a deprecated compatibility alias for `native_child = "default"`.
- New manifests should write `native_child` and `native_plugins` explicitly.
- `windows-dll` remains the compatibility target name even though it now covers the full root DLL product lane, not only legacy export builds.
