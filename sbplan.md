# `arcana_graphics.arcsb` With WinAPI-Owned Software Surfaces and Runtime De-Specialization

## Summary

- Add `arcsb` as a low-level module under `arcana_graphics`, with a softbuffer-shaped public lifecycle adapted to Arcana.
- Treat the current Windows runtime-owned window/canvas software-present path as transitional debt; do not build new graphics API on `std.kernel.gfx` special cases.
- In the same pass, move the reusable Windows CPU-software-surface ownership and GDI present logic into `arcana_winapi.helpers.graphics`.
- Also in the same pass, add the runtime/binding substrate needed for binding packages to work with:
  - live `std.window.Window` values
  - mapped `std.memory.ByteView`
  - mapped `std.memory.ByteEditView`
- Use `arcsb` as the first public consumer of that new seam, and shape the seam so the current Windows CPU canvas path can migrate onto it immediately.

## Public API / Types

- `arcana_graphics.arcsb` exports:
  - `Context`
  - `Surface`
  - `Buffer`
  - `Rect`
  - `AlphaMode`
- Public entrypoints:
  - `new_context() -> arcana_graphics.arcsb.Context`
  - `new_surface(read cx: arcana_graphics.arcsb.Context, read win: std.window.Window) -> Result[arcana_graphics.arcsb.Surface, Str]`
  - `Surface.configure(edit self, width: Int, height: Int, read alpha: arcana_graphics.arcsb.AlphaMode) -> Result[Unit, Str]`
  - `Surface.resize(edit self, width: Int, height: Int) -> Result[Unit, Str]`
  - `Surface.supports_alpha_mode(read self, read alpha: arcana_graphics.arcsb.AlphaMode) -> Bool`
  - `Surface.next_buffer(edit self) -> Result[arcana_graphics.arcsb.Buffer, Str]`
  - `Buffer.present(take self, edit surface: arcana_graphics.arcsb.Surface) -> Result[Unit, Str]`
  - `Buffer.present_with_damage(take self, edit surface: arcana_graphics.arcsb.Surface, read damage: List[arcana_graphics.arcsb.Rect]) -> Result[Unit, Str]`
- `Buffer` is a wrapper, not a raw returned view:
  - `width: Int`
  - `height: Int`
  - `byte_stride: Int`
  - `age: Int`
  - `pixels: std.memory.ByteEditView`
- Buffer rules:
  - `pixels` is a mapped foreign-backed editable byte view.
  - `Surface` may have only one live `Buffer` at a time.
  - `next_buffer` returns `Result.Err` while a previous buffer is still live.
  - `present` consumes the buffer and invalidates its mapped view.
  - abandoning a buffer without `present` must still release the mapping through cleanup; it must not wedge the surface.
- Win32 pixel contract for v1:
  - 4 bytes per pixel
  - explicit `byte_stride`
  - Windows software-present byte order is the Win32 DIB-compatible 32-bit layout for the helper surface; docs and tests must state the exact byte order callers see in `pixels`
- `AlphaMode` for Win32 v1:
  - `Opaque`
  - `Ignored`
  - unsupported modes return `Result.Err`

## Implementation Changes

- In the runtime/binding substrate:
  - extend the binding/runtime contract so binding packages can accept live `std.window.Window` inputs without runtime package-name special cases
  - extend the view subsystem so binding packages can create and return foreign-backed `std.memory.ByteView` and `std.memory.ByteEditView` values that stay valid after the native call returns
  - make those foreign-backed views runtime-tracked for liveness, aliasing, invalidation, and cleanup, not just raw pointers escaping native code
  - keep this support generic and reusable; do not make it `arcsb`-specific
- In `arcana_winapi`:
  - add a reusable Windows software-surface opaque handle under `arcana_winapi.helpers.graphics`
  - implement helper routines for:
    - opening a software surface for a live `std.window.Window`
    - destroying it
    - configuring/resizing it
    - mapping a writable byte view plus metadata
    - releasing a mapping without present
    - presenting the whole surface
    - presenting bounded damage
    - querying stride and any required format metadata
  - keep all Win32/GDI/DIB/pointer choreography private to `shackle` and helper implementation
  - if needed, add a thin public window helper for resolving or validating live window/native-handle access, but keep it generic and WinAPI-owned
- In `arcana_graphics`:
  - add `arcsb` as a source-only wrapper over the new WinAPI helper seam
  - keep the public API softbuffer-shaped through `Context -> Surface -> Buffer -> present`
  - `Buffer` should wrap the mapped `ByteEditView` plus metadata and any cleanup ownership needed for safe release
- In the Windows CPU canvas lane:
  - replace the current runtime-owned Windows software-present implementation with the same WinAPI-owned software-surface seam in this pass
  - keep `std.window` / `std.canvas` public source APIs stable
  - remove any need to add new GDI software-present behavior directly in runtime-owned special-case code after this change
- In `arcana_desktop`:
  - keep public desktop APIs source-compatible
  - ensure live desktop windows remain usable with `arcsb` through the shared window-handle family and the new generic binding/runtime window interop

## Test Plan

- Add runtime/binding tests for the new generic substrate:
  - native binding can accept a live `std.window.Window`
  - native binding can return mapped `ByteView` / `ByteEditView`
  - foreign-backed edit views enforce exclusive-edit behavior
  - invalidated or cleaned-up views are rejected after release
  - cleanup of abandoned mapped views releases the underlying mapping
- Add Win32 helper tests in `arcana_winapi` for:
  - software-surface open/configure/resize/destroy
  - full present
  - damage present
  - single-live-buffer enforcement
  - map, edit, discard, remap cycle
- Add `arcsb` end-to-end Windows tests for:
  - creating a surface from a live window
  - writing through `Buffer.pixels`
  - presenting successfully
  - resize then next-buffer/present
  - `age`, `width`, `height`, and `byte_stride` behavior across first and reused frames
  - unsupported alpha-mode rejection
- Add migration-proof coverage for the existing CPU canvas lane:
  - current Windows `std.canvas` and window/canvas smoke tests still pass
  - they now use the shared WinAPI software-surface path rather than runtime-owned GDI logic
- Add one desktop-proof smoke path that visibly renders through `arcana_graphics.arcsb` on a live window and exits cleanly.

## Assumptions / Defaults

- Runtime-owned package special cases for Windows window/canvas software present are being retired; this work should reduce them, not deepen them.
- This pass intentionally goes beyond an `arcsb` wrapper and lands reusable substrate for live-window interop plus foreign-backed byte views because that capability is expected to be reused outside `softbuffer`.
- The new mapped-view support is scoped to the concrete reusable families needed here:
  - `std.window.Window`
  - `std.memory.ByteView`
  - `std.memory.ByteEditView`
  It does not attempt a full general pointer language model.
- `arcana_graphics` remains source-only.
- `arcana_winapi` remains the only Win32 binding owner.
- Windows is the only required backend in v1; backend expansion is deferred, but the API shape should not block it.
