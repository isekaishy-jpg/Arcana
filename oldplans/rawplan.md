# Raw Binding Substrate Before `winapi.md`

## Summary
- Treat current Arcana surface as sufficient. Do not add another general-language phase before WinAPI.
- The prerequisite for `winapi.md` is a raw foreign substrate plan: make `arcana-cabi` and `shackle` able to carry real Windows-native layouts and call shapes honestly.
- Keep this plan strictly enabling. It does not do the broad `arcana_winapi` buildout yet, and it does not migrate runtime-owned desktop/text/audio subsystems.

## Key Changes
- Revise `arcana-cabi` so **binding ABI** owns a raw native type/layout model separate from the ordinary export ABI.
  - Keep the existing simple export routine ABI untouched.
  - Extend binding metadata to describe raw scalar widths, `isize/usize`, `f32/f64`, raw pointers, callback/function-pointer signatures, and layout-bearing native values by stable layout id.
  - Add binding layout tables for aliases, structs, unions, fixed arrays, integer-backed enums, flags/newtypes, callbacks, COM-style interface layouts, and named bitfields.
  - Freeze Windows/MSVC layout rules now for the binding lane, including bitfield packing/order.
  - Keep existing binding guarantees: callback symmetry, `edit` write-backs, owned `Str`/`Bytes`, explicit free helpers.

- Replace text-backed `shackle` semantics with typed raw ABI semantics.
  - `shackle` declarations stay the source surface, but the compiler must lower them into typed raw metadata instead of relying on Rust-emitted text as the contract.
  - Carry stable layout ids, DLL/symbol metadata, calling convention, callback-thunk metadata, and exported raw item identity through syntax, HIR, IR, package metadata, and AOT.
  - Exported `shackle type/const/import fn/fn/callback` remain dependency-visible, but their meaning comes from the typed raw model.

- Generate binding products from the typed raw model.
  - AOT/package generation must emit binding descriptors, raw layout tables, callback thunk metadata, and raw import surfaces from typed `shackle` metadata.
  - Generated Rust remains an implementation detail only; it must be derived from the typed binding contract, not define it.
  - Do not add new Arcana syntax in this plan.

- Limit runtime work to generic binding execution support.
  - Add only the generic consumer changes needed to execute the richer binding value/layout transport and callback/function-pointer handling.
  - No new WinAPI-specific runtime branches.
  - No retirement of current runtime desktop/text/audio special cases in this plan.

## Public Interfaces
- `arcana-cabi`
  - binding ABI gains a raw native type/layout description model
  - ordinary export ABI stays on the existing simple Arcana-facing value model
- `shackle`
  - no new syntax
  - existing declaration forms become typed/raw-authoritative instead of mostly surface-text/carried-emission driven
- Package/binding artifacts
  - binding descriptors/manifests/JSON projections must expose raw layout ids and raw binding type metadata consistently enough for loader validation and codegen

## Test Plan
- `arcana-cabi`
  - roundtrip tests for raw scalar widths, floats, pointers, callbacks, layout ids, structs, unions, arrays, enums, flags, COM-style interface layouts, and bitfields
  - Windows/MSVC size/align/offset/bitfield packing validation
  - binding metadata parity tests for imports and callbacks under the richer type model
- `shackle` / compiler pipeline
  - parser-to-IR tests proving raw declarations lower into typed metadata rather than only surface text
  - dependency-resolution tests for exported raw types, consts, callables, and callback signatures
  - AOT/package tests proving generated binding products emit raw layout tables and callback thunk metadata from typed inputs
- Runtime
  - synthetic binding fixture tests for raw by-value structs, unions, fixed arrays, bitfields, callbacks, and write-backs
  - generic runtime tests proving the richer binding ABI executes without WinAPI-specific branches

## Assumptions And Defaults
- Windows is the only target for this substrate phase, and layout/ABI rules follow the Windows/MSVC lane.
- `arcana.cabi.binding.v1` is revised in place for the raw binding model; the ordinary export ABI is not expanded into a native-layout ABI.
- No more general Arcana surface is added first; current `struct/union/array`, numeric widths, bitfields, and `&read/&edit/&take/&hold` capability model are the foundation this plan targets.
- Raw pointer/function-pointer semantics remain a binding/`shackle` concern, not a new ordinary Arcana pointer model.
- Broad `arcana_winapi.raw.*` / `helpers.*` expansion is explicitly the next plan after this one, not part of this enabling phase.
