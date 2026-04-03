# Provider Lane, Opaque Ownership, and `arcana_text` Migration

## Summary
- Add a new native product role `provider` for typed package-library APIs. Keep `export`, `child`, and `plugin` unchanged.
- Make provider activation explicit on dependency edges with `native_provider = "default"`.
- Migrate `arcana_text` off runtime package-name dispatch onto the provider lane, with completion defined as: no `arcana_text.*` hardcoded runtime behavior and no text-specific opaque families in the fixed runtime enum.
- Fold opaque-family cleanup into the same effort so provider-backed packages have a complete ownership model and no separate opaque redesign is needed afterward.
- Keep `arcana_desktop`’s current special-case debt explicitly grandfathered as a follow-up after text migration; ban any new package-name runtime special-cases immediately.

## Key Changes
- **Provider contract**
  - Extend `arcana-cabi` with `ArcanaCabiProductRole::Provider`, `ARCANA_CABI_PROVIDER_CONTRACT_ID = "arcana.cabi.provider.v1"`, and `ArcanaCabiProviderOpsV1`.
  - `provider` is a typed package-library role with instance semantics, not an entrypoint runner and not a raw plugin byte channel.
  - Provider ops include: enumerate callable/opaque metadata, invoke callable, allocate provider-owned opaque values, retain/release provider-owned opaque values, and report errors through cabi-owned owned-byte helpers.
  - Provider products are package-scoped and dependency-edge-scoped, with lazy instance creation and process-lifetime caching per `(consumer_member, dependency_alias)`.

- **Opaque ownership model**
  - Split opaque handles into two permanent classes:
    - runtime-owned substrate handles: fixed runtime families such as `Window`, `Image`, `Audio*`, `FileStream`, memory/concurrency handles
    - provider-owned package handles: dynamic families declared by provider metadata, such as `arcana_text.types.FontCollection`, `ParagraphBuilder`, and `Paragraph`
  - Keep fixed runtime opaque families only for runtime/substrate-owned resources.
  - Remove grimoire-owned text families from the fixed `RuntimeOpaqueFamily` enum during migration.
  - Add provider-declared opaque family metadata to package/AOT/runtime plans: family key, type path, package id, product name, ownership kind, and move/copy/boundary details needed to validate calls.
  - Runtime stores provider-owned opaques generically as provider binding + family key + opaque id. It must not know package names or canonical type names through hardcoded enums.
  - Source-level `opaque type` remains the language surface. Lowering changes so package-defined provider-backed opaques are resolved from provider metadata instead of a fixed runtime lang-item list.
  - This plan fully resolves provider-backed opaque ownership; no separate post-plan opaque redesign remains for grimoires.

- **Typed value transport**
  - Add a cabi-owned structured provider value codec. It is versioned, typed, and generic; it is not JSON, not plugin request bytes, and not an `AnyBox`-style erased carrier.
  - The codec must carry: `Int`, `Bool`, `Str`, `Unit`, `Array[Int]` bytes, `Pair`, `List`, `Map`, `Range`, named records, named variants, runtime-owned substrate handles, and provider-owned opaque handles.
  - Provider calls use explicit arg rows plus explicit write-back rows for `edit`; no raw runtime refs, owner handles, or erased values cross the boundary.
  - The codec is validated against package-plan type metadata so provider calls remain typed and auditable through the existing no-`AnyBox` policy.

- **Package/build/runtime integration**
  - Extend `book.toml`, lockfile rows, publish snapshots, fingerprints, and distribution manifests for `role = "provider"` and dependency-edge `native_provider`.
  - Add provider binding metadata to distribution manifests keyed by consumer member, dependency alias, package id, and provider product name.
  - Extend `arcana-aot` generated native products so `arcana-source` can emit provider DLLs. This plan does not add `rust-cdylib` provider support.
  - Generated provider products must embed package/provider metadata, expose provider ops, and route provider callable execution through generic runtime execution of the package image, not package-name-specific Rust branches.
  - Add provider callable metadata to AOT/runtime plans so ordinary source calls can resolve to provider dispatch generically before normal routine resolution, replacing today’s `try_execute_arcana_owned_api_call` package-name branching.
  - Enforcement rule: runtime must not gain new package-name grimoire dispatch. The only temporary grandfathered exception after this migration is `arcana_desktop`.

- **Minimal substrate changes**
  - Expand `std.canvas` only enough to let provider-backed text paint without text-specific host methods.
  - Add a generic mutable image floor:
    - create an empty image by size
    - replace full-surface RGBA bytes on an image
    - continue using existing blit operations to present that image
  - Do not add text-specific paint APIs to runtime or `std.canvas`.
  - Provider host callbacks stay generic and narrow: package asset root lookup, package/provider identity, and existing runtime-owned substrate handles. Text painting must go through the minimal image/canvas substrate above, not `text_paragraph_*` host methods.

- **`arcana_text` migration**
  - Add a default provider product to `arcana_text` and require first-party dependers to select it explicitly with `native_provider = "default"`.
  - Move `arcana_text` public API dispatch to the provider lane and remove `arcana_text.*` branches from runtime.
  - Migrate text opaque types to provider-owned families and remove text lang-item handling from the fixed runtime opaque-family mapping.
  - Remove grimoire-specific text methods from `RuntimeHost` once provider-backed paint/layout is wired through generic substrate.
  - The migration target is architectural, not the full final text engine: this plan ends when the current bootstrap text implementation sits behind the package/provider boundary and runtime no longer contains text-specific logic. Full SkParagraph-class feature build-out continues afterward on that corrected boundary.
  - `arcana_desktop` migration is explicitly deferred until after text is complete.

## Test Plan
- C ABI/header tests for `provider` role, descriptor shape, error helpers, and structured value codec round-trips.
- Package/build/distribution tests for `native_provider`, provider products, provider bindings, fingerprints, publish snapshots, and bundle manifests.
- Runtime tests for provider activation, lazy instance creation, per-edge isolation, provider callable dispatch, dynamic provider-owned opaque families, retain/release lifecycle, and rejection of undeclared family/type mismatches.
- Value-codec tests covering records, variants, lists, maps, ranges, write-backs, runtime-owned substrate handles, and provider-owned opaque handles, with explicit rejection of owner/ref/erased crossings.
- `std.canvas` tests for image creation, full-surface RGBA upload, and blit behavior in both buffered and native hosts.
- End-to-end tests with a synthetic external provider-backed library proving a non-first-party package can ship typed APIs plus opaque handles without runtime package-name branches.
- `arcana_text` migration tests proving paragraph/font/builder APIs run through the provider lane in local execution and packaged bundles, and proving no text-specific runtime dispatch or fixed runtime opaque-family entries remain.
- Enforcement tests that fail on new package-name runtime dispatch outside the explicit grandfather list and confirm the post-migration grandfather list contains only desktop debt.

## Assumptions and Defaults
- New role name is `provider`.
- Provider activation is explicit with `native_provider`, not implicit by dependency presence.
- `export`, `child`, and `plugin` keep their current meanings.
- This plan adds `arcana-source` provider support only.
- Runtime-owned substrate opaque families remain fixed; provider-owned package opaque families become dynamic and metadata-driven.
- Minimal `std.canvas` growth is allowed and is the only paint substrate expansion in this plan.
- Done means `arcana_text` is no longer runtime-special and no longer relies on fixed runtime text opaque families, not that the full paragraph engine rewrite is feature-complete.
- `arcana_desktop` runtime-special debt is intentionally deferred as the next migration follow-up after text.
