# `arcana_text` v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the rewrite direction for `arcana_text` as the Arcana-owned text paragraph grimoire.

## Role

- `arcana_text` is the public Arcana-owned paragraph/font/layout boundary.
- It sits above low-level substrate such as `std.canvas`, `std.text`, and `std.fs`.
- It is analogous in role breadth to SkParagraph: paragraph construction, styled runs, layout, metrics, hit testing, selection boxes, placeholders, and text paint.

## Boundaries

- `arcana_desktop` owns app-shell work, text-input enablement, committed-text and composition events, and composition-area application.
- `arcana_graphics` owns shared graphics-facing paint types used by text.
- `arcana_text` owns paragraph construction, font collections, fallback policy, shaping/layout behavior, and text asset helpers.
- `arcana_text` must not depend on `arcana_desktop` for its public contract.
- `arcana_text` implementation may use the generic package `provider` lane, but runtime must not special-case `arcana_text` by package name or fixed text opaque families.
- File IO remains in `std.fs`; `arcana_text` may layer asset helpers on top, but it must not redefine host-core file APIs.

## Public Surface

- `types`
  - opaque handles for `FontCollection`, `ParagraphBuilder`, and `Paragraph`
  - paragraph/style/metric/query records and enums
- `fonts`
  - font collection creation, source registration, family registration, and fallback control
- `builder`
  - paragraph builder lifecycle and styled-run construction
- `paragraphs`
  - layout, paint, metrics, hit testing, range boxes, placeholder boxes, and related paragraph queries
- `assets`
  - bundled text-asset helpers layered on runtime package asset lookup
- `monaspace`
  - Monaspace family/form names plus feature and axis helpers

## Rules

- `arcana_text` must not collapse back into label-only wrappers.
- The text stack must remain Arcana-owned. Third-party Rust crates must not define the public paragraph engine contract.
- OS text APIs are allowed only for host-installed font discovery/matching in the final fallback tier. They do not own shaping, layout, or rasterization behavior.
- The default proof path must use the paragraph API directly rather than compatibility label helpers.
