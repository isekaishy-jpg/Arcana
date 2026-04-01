# `arcana_text` Arcana-Owned Paragraph Rebuild

## Summary
- Rebuild `arcana_text` into a real SkParagraph-class grimoire: styled runs, font collections, shaping, bidi, line breaking, layout, paint, metrics, hit testing, selection boxes, placeholders, paragraph updates, and bundled font assets.
- Keep hard sibling boundaries:
  - `arcana_desktop` owns shell/runtime parity, events, committed-text and composition events, text-input settings, and composition-area application.
  - `arcana_graphics` owns shared paint-facing graphics types, even though it is still scaffold-level.
  - `arcana_text` owns paragraph/font/layout behavior and depends on graphics types where needed, but not on desktop.
- No shims and no new workspace Rust crate. Remove the scaffold-era `arcana_text -> arcana_desktop` dependency, replace the label scaffold as the design center, and land runtime internals as new modules inside `crates/arcana-runtime/src/`.
- No third-party Rust crates for the text stack. The implementation is Arcana-owned code in-tree, using a hybrid source strategy: directly port/adapt the hard SkParagraph-class logic where useful, rewrite surrounding API/engine layers as Arcana code, and retain required provenance/notices for any imported logic.
- OS text APIs are not part of the paragraph engine. They are allowed only for host-installed font discovery/matching when host fallback is enabled; shaping, layout, fallback policy, and rasterization remain Arcana-owned.

## Key Changes
- Add dedicated `arcana_text` scope/status docs and update the grimoire ledger so text is tracked as a real paragraph engine rather than a label wrapper.
- Replace the current public surface with `types`, `fonts`, `builder`, `paragraphs`, `assets`, and `monaspace`.
- Remove `labels` as the public design center. Rewrite the proof app, desktop proof, and other first-party text callers onto the paragraph API in the same effort.
- `types` defines opaque handles `FontCollection`, `ParagraphBuilder`, and `Paragraph`, plus the full supporting record/enum family for paragraph work: paragraph style, text style, strut, placeholders, line metrics, text boxes, ranges, affinity, alignment, direction, baselines, text-height behavior, decorations, shadows, font features, and font axes.
- Public geometry stays integer-pixel based. Any fractional controls use explicit integer milli-style fields rather than float public APIs.
- `fonts` owns collection creation, bundled Monaspace registration, extra-font registration from file/directory/bytes, collection-scoped fallback policy, and cache reset. Fallback order is fixed to explicit added fonts first, bundled Monaspace second, host-installed font fallback last.
- `builder` owns styled-run construction: open, push/pop style, add text, add placeholder, build, reset.
- `paragraphs` owns layout, paint, intrinsic metrics, baselines, line metrics, max lines, ellipsis, range boxes, placeholder boxes, hit testing, word boundaries, unresolved glyph reporting, fonts-used reporting, and paragraph update operations analogous to SkParagraph.
- `monaspace` owns family/form enums and helpers for Neon, Argon, Xenon, Radon, and Krypton across variable/static/frozen/Nerd builds, plus helpers for `calt`, `liga`, `ss01`-`ss10`, `cv01`-`cv62`, and `wght`/`wdth`/`slnt`.
- Vendor one pinned Monaspace release under `grimoires/owned/libs/arcana-text/assets/monaspace/<release>/...` and ship the full upstream desktop inventory. Default paragraph style uses Monaspace Neon Variable with no preset implicitly enabled.
- Add `arcana_text.assets` as the public bundled-font loader/resolver. Package/build/runtime gain the private machinery underneath it so package `assets/` participate in fingerprints, publish snapshots, bundle staging, and packaged runtime lookup by package id.
- Add only the minimum graphics-owned paint floor needed to make text real. Keep the graphics boundary hard: no text-owned parallel paint model, no graphics canvas policy inside text, and an explicit note that the richer paint surface will be revisited when `arcana_graphics` itself is properly rebuilt.
- Implement the runtime text engine as Arcana-owned modules under `arcana-runtime`: font parsing/loading, font collection state, shaping, bidi, segmentation, layout, glyph rasterization, caches, and buffered/native host integration.
- Replace the current `font8x8` measurement/draw path in both buffered and native hosts with the real paragraph engine.
- Limit OS text APIs to host font discovery/matching only. They may help enumerate or resolve installed fonts for the final fallback tier, but they do not shape text, break lines, rasterize glyphs, or define fallback behavior.
- Keep remaining desktop parity work out of this plan. Extra window/input/winit-parity growth stays in `arcana_desktop` unless text proves a concrete blocker.

## Test Plan
- Add regression coverage for paragraph building, wrap, alignment, bidi, max lines, ellipsis, placeholders, strut, baselines, line metrics, range boxes, placeholder boxes, hit testing, word boundaries, unresolved glyphs, fonts-used reporting, and paragraph updates.
- Add Monaspace coverage for bundled-family discovery, metrics compatibility, variable axes, stylistic sets, character variants, frozen/static/variable selection, and fallback with added external fonts.
- Add host-fallback tests that prove system-font lookup is used only after explicit fonts and bundled Monaspace fail, and that host lookup affects font resolution only, not shaping/layout ownership.
- Add package/bundle tests proving `assets/` affect fingerprints and publish metadata, proving native bundles stage `arcana_text` assets, and proving packaged runtime asset resolution works without source-path assumptions.
- Add a dedicated paragraph proof app that composes `arcana_desktop`, `arcana_graphics`, and `arcana_text` directly: desktop supplies IME/events/settings, text supplies caret and selection geometry, graphics supplies shared paint types.
- Update the existing desktop proof and first-party sample/template text usage to the real paragraph path. Completion requires no first-party dependence on the old label scaffold.

## Assumptions
- Windows remains the first proof target.
- This is paragraph-engine work, not editor/document-state work.
- `arcana_desktop` remains the only owner of IME/text-input lifecycle and composition-area application.
- `arcana_graphics` is still scaffold-level; this effort only adds the narrow graphics-owned paint/types floor text requires, with an explicit follow-up note for the later graphics rebuild.
- No third-party Rust crates are added for font parsing, shaping, layout, fallback, or rasterization. Any borrowed SkParagraph-class logic is brought in as Arcana-owned in-tree code, not as an external dependency.
- OS text APIs are allowed only for host-installed font lookup/matching in the final fallback tier.
