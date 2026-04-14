# Core Place/View Rewrite, Owned Buffer Types, and `arcsb` Migration

## Summary

- Replace the current library-owned view model with a core place/view substrate driven by the existing accessors `read`, `edit`, `take`, and `hold`.
- Make `read` and `edit` explicitly place-aware and view-aware through a builtin `View[Elem, Family]` kind with first-wave families `Contiguous`, `Strided`, and `Mapped`.
- Remove the old public view surface everywhere: `std.memory.ReadView`, `EditView`, `ByteView`, `ByteEditView`, `StrView`, the related lang items, the old view traits/helpers, and `std.bytes`.
- Add core owned payload/buffer types:
  - `Bytes` and `ByteBuffer`
  - `Utf16` and `Utf16Buffer`
- Keep the original softbuffer goal intact: `arcana_graphics.arcsb` becomes the first consumer of the new substrate, backed by reusable WinAPI-owned software surfaces, while current Windows runtime special cases are retired.

## Public Surface

- **Core projections and views**
  - `[]` is the canonical projection surface.
  - `x[a..b]` stays as sugar for contiguous projection.
  - Add keyworded projection specs inside `[]`:
    - `x[contiguous start: a, end: b]`
    - `x[strided start: a, len: n, stride: s]`
  - `Mapped` is a real family but is produced by runtime/native providers, not fabricated from arbitrary local values.
  - `read` and `edit` must operate on both ordinary places and `View[...]` values.
  - `take` and `hold` remain ownership accessors, not general view constructors.

- **Core owned payload/buffer types**
  - `Bytes`: immutable owned contiguous byte payload.
  - `ByteBuffer`: mutable owned byte buffer with builder/edit operations and `freeze() -> Bytes`.
  - `Utf16`: immutable owned UTF-16 code-unit payload.
  - `Utf16Buffer`: mutable owned UTF-16 buffer with builder/edit operations and `freeze() -> Utf16`.
  - Add explicit thaw/buffer conversion from immutable owned payloads back to mutable companions.
  - `Str` remains the canonical language text type.
  - Put encoding conversions on core surfaces:
    - `Str.encode_utf8() -> Bytes`
    - `Bytes.decode_utf8() -> Result[Str, Str]`
    - `Str.encode_utf16() -> Utf16`
    - `Utf16.to_str() -> Result[Str, Str]`

- **Boundary semantics**
  - `Bytes`, `ByteBuffer`, `Utf16`, and `Utf16Buffer` are all first-class boundary transport types.
  - `View[...]` is also a first-class boundary transport type.
  - Param lowering rules:
    - `read Bytes` / `read Utf16` lower to read views of owned immutable payloads.
    - `take Bytes` / `take Utf16` transfer owned immutable payloads.
    - `read ByteBuffer` / `edit ByteBuffer` and `read Utf16Buffer` / `edit Utf16Buffer` lower to read/edit views of buffer backing.
    - `take ByteBuffer` / `take Utf16Buffer` transfer owned mutable buffers.
    - `edit Bytes` and `edit Utf16` are invalid because the owned payload types are immutable.
  - Returns and callback results may return owned `Bytes`, `ByteBuffer`, `Utf16`, `Utf16Buffer`, and `View[...]`.
  - `edit` write-back slots remain only for ordinary non-view, non-buffer value params; live view/buffer edits are in-place and do not use whole-value write-back.
  - `hold` remains local-only in v1 boundary semantics.

- **`arcana_graphics.arcsb`**
  - Preserve the softbuffer-shaped public API:
    - `Context`, `Surface`, `Buffer`, `Rect`, `AlphaMode`
    - `new_context()`
    - `new_surface(read cx, read win: std.window.Window)`
    - `Surface.configure`, `resize`, `supports_alpha_mode`, `next_buffer`
    - `Buffer.present`, `present_with_damage`
  - `Buffer` fields stay:
    - `width`
    - `height`
    - `byte_stride`
    - `age`
    - `pixels: View[U8, Mapped]`
  - Buffer rules stay:
    - one live buffer per surface
    - `present` consumes the buffer and invalidates the mapped view
    - abandon-without-present still releases the mapping through cleanup
  - Win32 v1 surface contract stays:
    - 4 bytes per pixel
    - explicit `byte_stride`
    - DIB-compatible byte order
    - `AlphaMode.Opaque` and `AlphaMode.Ignored` only

## Implementation Changes

- **Language/compiler core**
  - In `arcana-syntax`, replace slice-only borrowing/projection parsing with general projection parsing while preserving contiguous sugar.
  - In `arcana-hir`, add builtin `View[Elem, Family]`, builtin family markers, and builtin core payload/buffer types.
  - In `arcana-hir`, remove the ambient aliases and inference hooks for the old `std.memory.*View` names.
  - In `arcana-frontend`, replace `BorrowedSliceSurfaceKind` and the old view-handle `OpaqueLangFamily` cases with real view-family/type validation.
  - Reserve future family growth for things like atomic views, but do not implement them in this pass.

- **Runtime, CABI, and native toolchain**
  - In `arcana-runtime`, replace the separate `ReadView` / `EditView` / `ByteView` / `ByteEditView` / `StrView` runtime lanes with one generic runtime view representation carrying family, element surface, len, stride, provenance, mutability, and cleanup state.
  - In `arcana-runtime`, add core runtime values for `Bytes`, `ByteBuffer`, `Utf16`, and `Utf16Buffer`.
  - Delete old view intrinsics and foreign-byte helpers from the runtime path; replace them with core view operations and core payload/buffer operations.
  - In `arcana-cabi`, stop treating `Bytes` as `Array[Int]`.
  - Add first-class CABI transport for:
    - owned `Bytes`
    - owned `ByteBuffer`
    - owned `Utf16`
    - owned `Utf16Buffer`
    - generic `View[...]` descriptors
  - `View[...]` boundary metadata must include:
    - family
    - element scalar or raw layout id
    - length
    - stride bytes
    - read/edit access mode
    - mapped-view provenance and release token when applicable
  - In `arcana-aot`, `arcana-cli`, `arcana-ir`, and `arcana-package`, update lowering, layout generation, codegen, packaging fixtures, runtime-requirement mapping, and native header/golden coverage to the new payload/buffer/view contract.

- **Windows and softbuffer lane**
  - Move the reusable Windows CPU software-surface ownership and GDI present path into `arcana_winapi.helpers.graphics`.
  - Add boundary/runtime support so binding packages can accept live `std.window.Window` values without package-name runtime special cases.
  - Use `arcana_winapi.helpers.graphics` to produce mapped `View[U8, Mapped]` buffers plus metadata for `arcsb`.
  - Migrate the current Windows CPU `std.canvas` lane onto the same WinAPI-owned software-surface seam in this pass.
  - Keep `arcana_graphics` source-only; keep `arcana_winapi` as the binding owner.
  - Keep `arcana_desktop` source-compatible while ensuring current desktop windows remain usable through the migrated `std.window`/WinAPI substrate.

- **Std/grimoires/examples migration**
  - Remove `std.bytes` completely.
  - Remove from `std.memory` all public view types, view traits, view helpers, and foreign-byte helpers; leave only storage facilities that still make sense after the rewrite.
  - Rewrite `std`, grimoires, and examples to use:
    - `Bytes` / `ByteBuffer`
    - `Utf16` / `Utf16Buffer`
    - `View[...]` projections
  - Migrate especially:
    - `std.process`, `std.fs`, `std.binary`, `std.text`, `std.prelude`
    - `arcana_text` font/text/shape/raster loaders and parsers
    - `arcana_graphics.arcsb`
    - desktop/text proof apps and runtime fixture workspaces
  - WinAPI text handling must use `Utf16` plus explicit terminated adapters at the boundary for `PCWSTR` / `PWSTR`; `Utf16` itself remains non-terminated.

- **Docs/specs**
  - Update approved specs covering memory/view surface, backend CABI, std surface, OS bindings, and any status documents that currently ratify the old view or `std.bytes` model.
  - Keep `llm.md` synchronized only after the approved spec updates are settled.

## Test Plan

- **Compiler**
  - Syntax/HIR coverage for contiguous sugar and keyworded strided projections.
  - Frontend coverage for valid and invalid projection families, immutable-root rejection, string read-only projection, and mapped-view provenance rules.

- **Runtime**
  - Replace old view-handle tests with generic `View[...]` tests for contiguous, strided, and mapped families.
  - Add tests for `Bytes`, `ByteBuffer`, `Utf16`, and `Utf16Buffer` ownership, freeze/thaw, projection, and conversion behavior.
  - Add tests proving `read`/`edit` buffer params lower to live views while `take` transfers ownership.

- **CABI/AOT/toolchain**
  - Round-trip imports, returns, and callbacks for all four owned payload/buffer types plus `View[...]`.
  - Verify that editable view/buffer params mutate backing storage directly and do not use whole-value write-back.
  - Update generated header/codegen/package tests and goldens that currently assert `Array[Int]` bytes or old view structs.

- **Windows/softbuffer**
  - WinAPI helper tests for software-surface open/configure/resize/map/present/discard/destroy.
  - `arcsb` end-to-end tests for create, configure, map, draw, present, present-with-damage, resize, and cleanup.
  - Desktop proof coverage that the `arcsb` member stages and runs through the mapped-view path.
  - Migration-proof tests that the Windows CPU `std.canvas` lane now uses the shared WinAPI software-surface path rather than runtime-owned special-case GDI code.

- **Repo-wide cleanup gates**
  - No remaining references to:
    - `std.bytes`
    - `std.memory.ReadView`
    - `std.memory.EditView`
    - `std.memory.ByteView`
    - `std.memory.ByteEditView`
    - `std.memory.StrView`
    - `foreign_bytes_view`
    - view-handle lang items like `read_view_handle`
    - `Array[Int]` as the meaning of `Bytes`

## Assumptions

- This is a big-bang migration with no compatibility aliases in the final tree.
- First implemented view families are `Contiguous`, `Strided`, and `Mapped`.
- Future family growth like atomic views is intentionally left open by the kind design but not implemented here.
- `softbuffer`/`arcsb` depends on `View[U8, Mapped]`; owned `ByteBuffer` transport is additive, not a replacement for mapped views.
- `Str` stays the canonical language text type; `Utf16` exists as an encoding/interoperability surface, especially for Windows.
