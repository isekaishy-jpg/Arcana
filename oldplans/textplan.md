# `arcana_text` Engine Rebuild v4

## Summary
- Rebuild `grimoires/libs/arcana-text` as a real Arcana text engine, not a paragraph wrapper and not a provider-shaped package.
- Use the current rewrite surface in `llm.md` as the working law for implementation decisions, so the grimoire actively proves Arcana’s object, owner, memory, phrase, qualifier, and chain surfaces.
- Keep `arcana_winapi` narrow: installed-font discovery, family/face metadata, and stable identity/path lookup only. The engine itself stays in Arcana.
- Treat stale `arcana_text` docs/examples as migration work at the end, not as design authority now.

## Engine Architecture
- Public modules are:
  - `types`
  - `fonts`
  - `buffer`
  - `editor`
  - `layout`
  - `raster`
  - `cache`
  - `queries`
  - `assets`
  - `monaspace`
- Canonical public objects are:
  - `obj FontSystem`
  - `obj TextBuffer`
  - `obj TextEditor`
  - `obj LayoutSnapshot`
  - `obj TextCache`
  - `obj GlyphDrawStream`
- Canonical public records/enums are:
  - `FontSource`, `FontFaceId`, `FontMatch`, `FontQuery`, `FontFeature`, `FontAxis`
  - `TextStyle`, `SpanStyle`, `ParagraphStyle`, `LayoutConfig`, `RasterConfig`
  - `Cursor`, `Selection`, `CompositionRange`, `PlaceholderSpec`
  - `LineMetrics`, `HitTest`, `CaretBox`, `RangeBox`, `UnresolvedGlyph`
- Public API stays object-and-record centered. Do not bring back `builder`, `paragraphs`, or a helper-heavy constructor garden.
- Public call shapes must stay within current phrase rules:
  - no more than 3 top-level args
  - config records for larger option sets
  - attached blocks only on valid statement-form calls
  - no tuple-heavy API design beyond exact pairs where that is the natural return shape

## Arcana Surface Proof Requirements
- Use `obj` and `impl` as the main organizing surface. Core behavior belongs on objects, not in a flat free-function namespace.
- Use `trait` only for explicit static capability boundaries that fit current law. Good uses are:
  - byte/font source readers
  - style/span resolution
  - optional shaping/raster helper capability glue
  - query sink/collector traits
- Do not use trait objects, dynamic dispatch, or closure-shaped callback patterns.
- Use `create ... context:` owners internally where there is a real scoped work domain:
  - one layout-pass owner
  - one raster/cache mutation owner
  - optional one font-scan/import owner if import staging benefits from explicit activation
- Use `Memory` specs and memory phrases as real engine storage, not as decoration:
  - `session` for immutable loaded font bytes and interned text/font names
  - `slab` for stable face records, layout snapshots, cache entries, and placeholder records
  - `temp` for pass-local shaping, bidi, line-break, and query scratch with `reset_on = owner_exit`
  - `pool` only if dense reusable rows are clearly better than `slab` for a specific table
  - do not force every family into the engine just to “use more Arcana”
- Use modern qualifier and chain surface where it is semantically natural:
  - `must` and `fallback` for explicit `Option` / `Result` handling
  - `forward` chains for serial text pipeline stages
  - `collect` only where multiple downstream outputs are intentionally returned
  - do not use `parallel` or spawned qualifiers in the first engine milestone unless a specific subsystem is proven independent and deterministic under that model

## Internal Object Model
- `FontSystem` owns the catalog and resolution state:
  - bundled Monaspace registration
  - explicit file/dir/bytes sources
  - installed-font discovery through `arcana_winapi`
  - parsed family/style/full-name metadata
  - face lookup and fallback lists
  - feature/axis defaults
- `TextBuffer` owns:
  - text storage
  - style spans
  - paragraph boundaries
  - placeholder/embed slots
  - dirty regions for relayout
- `TextEditor` owns:
  - current cursor
  - current selection
  - composition range
  - edit commands and movement commands over a `TextBuffer`
- `LayoutSnapshot` owns immutable layout results:
  - paragraph/run/line segmentation
  - glyph placements
  - caret stops
  - line metrics
  - unresolved glyph records
  - fonts-used set
- `TextCache` owns:
  - glyph raster cache
  - bitmap/color glyph cache
  - per-face raster state
  - cache invalidation keys
- `GlyphDrawStream` owns render-facing text output:
  - positioned glyph draws
  - positioned bitmap/color glyph draws
  - clip and range segmentation needed for later graphics integration
  - no dependency on `arcana_graphics` paint/image types in the core contract

## Subsystem Design
### 1. Fonts
- `assets` and `monaspace` provide the packaged Monaspace variable-font defaults and names.
- `FontSystem` methods are the only public font entry surface:
  - register bundled defaults
  - add explicit source by file/dir/bytes
  - discover installed fonts
  - resolve family/style queries
- `arcana_winapi` returns discovery metadata only. `arcana_text` then opens and parses bytes itself.
- The font parser must support the tables needed for:
  - family/style/full-name metadata
  - cmap
  - metrics
  - glyph locations/outlines
  - variable axes
  - OpenType feature selection
- The first implementation should target the subset needed for Monaspace plus modern Latin/OpenType support, but the module boundaries must already assume broader-script shaping later in the same architecture.

### 2. Buffer And Editor
- `TextBuffer` uses explicit Arcana-owned storage for:
  - text chunks
  - span runs
  - paragraph index rows
  - placeholder rows
- Keep the public editing surface on `TextEditor`, not on free functions:
  - insert
  - delete backward/forward
  - replace range
  - move by grapheme
  - move by word
  - extend/shrink selection
  - apply committed/composition text
- `arcana_desktop` remains the only owner of IME/text-input lifecycle. `arcana_text` only stores composition state and applies edits.

### 3. Layout Pipeline
- Layout is a staged pipeline with explicit intermediate records:
  - paragraph extraction
  - script/style itemization
  - bidi resolution
  - face fallback resolution
  - shaping
  - line breaking
  - alignment/overflow/ellipsis
  - snapshot finalization
- Implement the pipeline with `forward` chains internally where it makes the staging clearer, but keep the intermediate state explicit in records/objects rather than implicit closures or callback stacks.
- The layout-pass owner should activate with one explicit context record carrying:
  - target buffer id/version
  - layout config
  - current font system handle
  - scratch-memory handles
- That owner owns scratch objects for itemization, shaping, breaking, and snapshot assembly, then exits deterministically.

### 4. Queries
- Queries are methods over `LayoutSnapshot` and return explicit records:
  - hit test at point
  - caret box at text position
  - range boxes
  - line metrics
  - word boundary search
  - fonts used
  - unresolved glyphs
- Keep query state pure/read-only over snapshots; use `temp` scratch only when a query needs transient workspace.

### 5. Raster And Cache
- `TextCache` is the mutable engine state for glyph raster reuse.
- Rasterization should support:
  - grayscale/alpha output
  - LCD/subpixel output
  - color/bitmap glyph cases
- The raster pass owner activates with context that carries:
  - snapshot identity/version
  - raster config
  - cache handles
  - scratch memory
- `GlyphDrawStream` is produced from `LayoutSnapshot + TextCache + RasterConfig`. It is the stable render-facing contract until a later graphics adapter is built.

## Implementation Order
- Step 1: package root and public object/type shell.
  - Create the engine-centered module tree and define the public objects, records, and enums before any heavy behavior.
- Step 2: font system.
  - Land Monaspace defaults, explicit source loading, installed-font discovery via `arcana_winapi`, and metadata parsing/matching.
- Step 3: buffer and editor.
  - Land text storage, spans, paragraph tracking, placeholders, cursor/selection/composition state, and core edits.
- Step 4: layout pipeline.
  - Land itemization, fallback, shaping, bidi, line breaking, alignment, overflow, ellipsis, and immutable snapshots.
- Step 5: query surface.
  - Land hit testing, caret geometry, range boxes, metrics, fonts-used, and unresolved-glyph reporting.
- Step 6: raster and cache.
  - Land glyph cache, raster paths, and draw-stream generation.
- Step 7: migration cleanup.
  - Rewrite proof apps and stale docs/examples to the new engine API and delete any lingering imports of old `arcana_text` surfaces.

## Test Plan
- Arcana-surface proof tests:
  - object/impl-based API use compiles cleanly
  - owner/context layout and raster workspaces activate and exit correctly
  - memory specs/phrases back real engine storage and reset correctly on owner exit
  - chain-based pipeline stages run with the expected serial semantics
  - phrase constructors/config calls stay within current `llm.md` limits
- Font tests:
  - bundled Monaspace registration
  - explicit file/dir/bytes font loading
  - installed-font discovery through `arcana_winapi`
  - family/style/full-name matching
  - fallback resolution
  - variable-axis and OpenType feature behavior
- Buffer/editor tests:
  - insert/delete/replace
  - cursor and selection movement
  - composition range storage and replacement
  - paragraph edits
  - placeholder insertion/removal
- Layout/query tests:
  - English + Monaspace proof
  - multi-script shaping
  - bidi layout
  - wrapping, alignment, overflow, ellipsis
  - hit testing, caret geometry, range boxes, line metrics
  - fonts-used and unresolved glyph reporting
- Raster/cache tests:
  - alpha output
  - LCD/subpixel output
  - color/bitmap glyph cases
  - cache reuse and invalidation after edits or style/font changes
  - deterministic `GlyphDrawStream` output
- Migration acceptance:
  - proof apps compile against the new engine API
  - no first-party code imports `arcana_text.builder`, `arcana_text.paragraphs`, or provider-era paths

## Assumptions
- `llm.md` is the implementation law for current parser/frontend/runtime behavior while rebuilding `arcana_text`.
- Stale `arcana_text` docs/examples are migration output, not blockers.
- The slim Monaspace variable-font bundle remains the default packaged font payload.
- `arcana_winapi` is discovery-only for this rebuild.
- `arcana_text` stays a plain `kind = "lib"` package with no provider/product role and no runtime special cases.
