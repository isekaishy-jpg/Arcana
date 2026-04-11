# Replace Transitional WinAPI Crate With a Grimoire-Owned `shackle` Win32 System

## Summary
- Remove `crates/arcana-winapi` and eliminate `binding_support_crate` from the `arcana-source` binding path.
- Keep the architecture fixed as `arcana-cabi -> arcana_winapi -> consumers`.
- Make `arcana_winapi` own:
  - the public raw Win32 surface
  - the thin Win32 helper layer above it
  - the raw callback signatures consumers need
- Add a new generic foreign source family named `shackle`, restricted to binding-owning packages but usable publicly once exported.
- Revise the binding CABI in place so raw Windows-native layouts, pointers, callbacks, and bitfields can cross the boundary directly.

## Key Changes
- Replace the support-crate lane with source-owned binding implementation.
  - Remove manifest/package/AOT/runtime support for `binding_support_crate`.
  - Generated binding products must lower directly from package source and emit their own import dispatch, host routines, and callback thunks.

- Add a `shackle` source family for binding-owning packages.
  - New declaration kinds:
    - `shackle type`
    - `shackle struct`
    - `shackle union`
    - `shackle flags`
    - `shackle const`
    - `shackle import fn`
    - `shackle callback`
    - `shackle fn`
    - `shackle thunk`
  - Only packages with an approved `binding` product may declare `shackle`.
  - Exported `shackle` items are the public raw foreign layer. Consumers may use them, but may not declare new ones unless they own a binding product.
  - `shackle fn` is the package-owned host-side logic surface. It may call raw imports and may be exported when the helper layer should stay host-side.

- Make `arcana_winapi` public shape explicit.
  - `arcana_winapi.raw.*` exposes exported `shackle` types, constants, callback signatures, and raw imported Win32 calls with names kept close to Win32.
  - `arcana_winapi.helpers.*` exposes thin exported helper routines for Win32 ceremony consumers should not reimplement.
  - Existing proof-slice modules like `foundation`, `fonts`, and `windows` become compatibility wrappers or reorganized helper modules on top of `raw.*`.

- Let consumers participate in Win32 callbacks cleanly.
  - Binding-owning packages may export `shackle callback` signature types.
  - Consumer packages may declare dependency callbacks by type reference, for example:
    - `native callback my_proc: arcana_winapi.raw.user32.WNDPROC = app.win.handle_proc`
  - The declared callback value is passable anywhere the exported callback type is accepted.
  - `shackle thunk` is the generated/native bridge that turns that callback value into a real Win32 function pointer under the owning binding product.

- Expand the raw layout model.
  - Support explicit-width integers, floats, pointers, function pointers, fixed arrays, handle aliases, `repr(c)` structs, `repr(c)` unions, integer-backed enums, flags/newtypes, and callback types.
  - Add bitfields now, but with a narrow Windows-only rule:
    - allowed only in `shackle struct`
    - named fields only
    - fixed-width integer base types only
    - no anonymous bitfields
    - no zero-width bitfields
    - adjacent bitfields pack with MSVC-compatible Windows layout rules
    - overflow starts a new storage unit
    - low-order-bit-first within the storage unit
  - Bitfields in `shackle` are part of the raw layout model, not a separate helper abstraction.

- Revise `arcana-cabi` in place to carry raw layouts directly.
  - Extend binding metadata and value transport beyond the current six-tag proof model to support raw native scalars, pointers/function pointers, and native-layout values keyed by stable layout ids.
  - Keep owned `Str`/`Bytes`, opaque handles, and `edit` write-backs as part of the same unified binding contract.
  - Runtime, manifests, and JSON ABI remain projections/consumers of `arcana-cabi`, not parallel ABI owners.

- Extend frontend, IR, AOT, and runtime.
  - Parser/frontend/IR must carry all `shackle` declarations, exported callback types, dependency-callback declarations, and raw layout ids.
  - AOT must emit generated `extern "system"` imports, lowered `shackle fn` bodies, callback thunks, and direct binding import dispatch without a support crate.
  - Runtime remains only the loader/invoker/registrar for the binding contract and dependency callback registrations.

## Public Interfaces
- `book.toml`
  - `producer = "arcana-source"` with `role = "binding"` no longer accepts or requires `binding_support_crate`
- Arcana source
  - new `shackle` declaration family
  - exported `shackle` items form the public raw layer of binding grimoires
  - dependency callback syntax uses exported callback types, not inline callback-owner metadata
- Public `arcana_winapi`
  - `raw.*` mirrors Win32 closely
  - `helpers.*` contains Win32-specific ceremony and bridge helpers
  - consumers build higher-level policy above those layers

## Test Plan
- Language/toolchain:
  - parser/frontend/IR coverage for every `shackle` declaration form
  - rejection tests for `shackle` use outside binding-owning packages
  - dependency-callback tests for exported callback types and consumer `native callback` declarations

- CABI/runtime:
  - contract tests for raw scalar, pointer, function-pointer, struct, union, array, flags, enum, and bitfield transport
  - callback registration/invocation tests through exported callback types
  - continued coverage for owned strings/bytes and `edit` write-backs under the revised contract

- ABI/layout validation:
  - Windows/MSVC layout tests for size, align, offset, and bitfield packing
  - representative real Win32 struct/flag/callback cases plus synthetic fixtures for edge cases
  - generated callback thunk tests proving correct function-pointer identity and teardown behavior

- Migration:
  - `arcana_winapi` proof slice still passes after migration
  - broader raw Win32 smoke coverage proves public `raw.*` calls, helper routines, and consumer-owned callbacks
  - workspace checks confirm `crates/arcana-winapi` is gone and no library binding seam depends on `windows-sys`

## Assumptions
- Windows is the only supported foreign-binding target until after selfhost.
- `shackle` is the literal keyword family name.
- The binding CABI revision lands in place rather than as a new versioned contract.
- Consumers own higher-level policy and framework behavior; `arcana_winapi` owns Win32-native mechanics, raw declarations, helper ceremony, and exported callback types.
- Linux/macOS bindings remain out of scope for this work and must not shape the pre-selfhost ABI or layout rules.
