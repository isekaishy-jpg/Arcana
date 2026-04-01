# `arcana_text` v1 Status

Status: `approved-pre-selfhost`

This ledger records the current rewrite state of the `arcana_text` grimoire against its approved role.

## Current State

- Public rewrite-owned modules now center on `types`, `fonts`, `builder`, `paragraphs`, `assets`, and `monaspace`.
- The scaffold-era `arcana_text.labels.*` surface has been removed from first-party callers and from runtime dispatch.
- `arcana_text` no longer depends on `arcana_desktop`; proof callers compose desktop and text directly.
- `arcana_text` now ships a package-owned default `provider` product and first-party dependers select it explicitly with `native_provider = "default"`.
- Public paragraph, builder, and font-collection calls now route through the generic provider lane instead of runtime package-name dispatch.
- Provider-owned text opaques are now dynamic and metadata-driven; runtime no longer keeps text-specific opaque families in its fixed substrate enum.
- Runtime is now limited to generic provider hosting, packaged asset lookup, and the minimal canvas image substrate used by provider-backed text paint.
- The current source-backed bootstrap engine now covers placeholders, range boxes, word boundaries, unresolved glyph reporting, mutable paragraph updates, and basic alignment plus RTL placement.
- `arcana_text.assets` now resolves package-owned bundled assets through package-id-keyed staged roots instead of source-relative guesses.
- The grimoire now vendors the pinned Monaspace `v1.400` desktop inventory under `assets/monaspace/v1.400`, and `fonts.default_collection()` registers the bundled Neon variable source through that asset resolver.
- First-party proof coverage now includes the updated desktop proof path and a dedicated `examples/arcana-text-proof` workspace.

## Still Missing

- Real font parsing, shaping, bidi, line breaking, fallback resolution, and glyph rasterization are not complete yet.
- Host-installed font fallback is not implemented beyond contract scaffolding.
- The current in-tree engine is bootstrap-only and does not yet meet full SkParagraph-class behavior.

## Exit Criteria

- `arcana_text` provides rewrite-owned paragraph behavior across layout, paint, metrics, hit testing, placeholders, fallback, and bundled assets through the package/provider boundary rather than runtime package-name branches.
- Proof apps and templates exercise the paragraph path directly.
- No first-party path depends on the old label-wrapper model or runtime-special text opaque families.
