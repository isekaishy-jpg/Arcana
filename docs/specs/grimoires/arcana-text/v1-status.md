# `arcana_text` v1 Status

Status: `approved-pre-selfhost`

This ledger records the current rewrite state of the `arcana_text` grimoire against its approved role.

## Current State

- `arcana_text` is now back to a plain `kind = "lib"` package with no native `provider` product.
- Runtime no longer carries a package-library provider lane for `arcana_text`; text-specific provider dispatch, source-provider bridging, and dependency-edge provider activation have been removed from the rewrite.
- The previous bootstrap paragraph/provider implementation has been cleared so the engine can be rebuilt cleanly in Arcana source.
- The pinned Monaspace `v1.400` Variable asset set remains vendored under `assets/monaspace/v1.400`.
- `arcana_text` still does not depend on `arcana_desktop`; proof callers compose desktop and text directly.
- The new generic OS-binding lane and `arcana_winapi` grimoire now exist for host-installed font discovery and related Windows metadata, so text no longer needs a provider-style workaround for that host seam.

## Still Missing

- A real Arcana-owned text engine is not implemented yet.
- Real font parsing, shaping, bidi, line breaking, fallback resolution, glyph rasterization, editing state, and query surfaces are still missing.
- Host-installed font fallback still needs to be consumed from `arcana_winapi` during the text rebuild.

## Exit Criteria

- `arcana_text` provides rewrite-owned paragraph behavior across layout, paint, metrics, hit testing, placeholders, fallback, and bundled assets as a normal source library dependency rather than a runtime-special package.
- Proof apps and templates exercise the paragraph path directly.
- No first-party path depends on the old label-wrapper model or runtime-special text opaque families.
