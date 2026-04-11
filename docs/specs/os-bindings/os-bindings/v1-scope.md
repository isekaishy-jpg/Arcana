# OS Bindings v1 Scope

Status: `approved-pre-selfhost`

This scope freezes the pre-selfhost generic OS-binding seam for Arcana library packages.

## Role

- OS bindings are package-owned native seams used by external-style Arcana libraries that need narrow host capability access.
- The binding mechanism is generic; it must not be hard-coded for text, desktop, or any single grimoire.
- The first concrete consumer is `grimoires/arcana/winapi`, exposed as package `arcana_winapi`.

## Public Model

- A binding-owning library package remains a normal `kind = "lib"` package.
- It may declare one default native product with:
  - `role = "binding"`
  - `producer = "arcana-source"` in the current proof lane
- Arcana source in that same package may declare:
  - `native fn ... = host.path`
  - `native callback ... = package.path`
  - `shackle ...` declarations for raw foreign imports, callbacks, layouts, constants, and host-side binding routines
- Consumers depend on the library package only. They do not activate or configure the binding sidecar explicitly in their dependency edge.
- Consumers may use exported `shackle` dependency surface directly:
  - exported `shackle type` in type positions
  - exported `shackle const` as value paths
  - exported `shackle import fn` / exported `shackle fn` as callable paths
  - exported `shackle callback` as typed `native callback` references

## Language Surface

- `native fn`
  - package-scoped imported host call surface
  - used by libraries like `arcana_winapi`
  - not a replacement for `intrinsic fn`
- `native callback`
  - package-scoped explicit callback registration surface
  - callbacks target ordinary Arcana routines by path
  - callbacks are typed and declared up front; they are not ad hoc symbol lookups
  - callback imports and callback exports follow the same typed metadata contract as `native fn`, including `edit` write-backs
- `shackle`
  - binding-owning-package-only raw foreign declaration family
  - owns raw host imports, callback signatures, native layouts, constants, and package-local host routines
  - lowers into typed raw binding metadata; generated Rust is a projection of that typed contract, not the owner of layout semantics
  - exported `shackle` items form the public raw layer of binding grimoires such as `arcana_winapi.raw.*`
  - exported `shackle import fn`, exported `shackle fn`, and exported `shackle const` must be dependency-visible through ordinary path resolution; consumers must not need a parallel special binding lookup model
- `opaque type`
  - binding-owning packages may export source-declared opaque handle types for native values such as module handles, font catalogs, windows, and callback tokens

## Rules

- `intrinsic fn` remains the trusted std/kernel-only surface.
- Generic OS binding work must not be reintroduced as runtime package-name special cases.
- `arcana-cabi` owns the foreign-boundary semantics; runtime owns only generic loading and invocation as a consumer of that contract.
- The binding lane now carries raw native layout/value transport for Windows-first shackle declarations:
  - fixed-width ints and floats
  - pointer/function-pointer-bearing raw layouts
  - structs, unions, fixed arrays, flags/enums, callbacks, interfaces, and named bitfields by stable layout id
- Binding grimoires must own the Arcana-facing API; runtime must not invent binding-only callback, ownership, or write-back behavior locally.
- Binding grimoires must keep host-native scope narrow:
  - host capability discovery
  - host handles
  - explicit callbacks
  - thin raw API coverage
- Binding-owning packages must lower their own host implementation through `shackle`; runtime must not route through a handwritten support crate.
- Higher-level policy remains in ordinary grimoires above the binding layer.
- The first proof lane is Windows only.
- The transitional handwritten `crates/arcana-winapi` bridge is removed; `grimoires/arcana/winapi` owns the Win32 binding implementation directly.

## `arcana_winapi` v1 Proof Slice

`arcana_winapi` is the first public OS-binding grimoire.

Its public package shape is:
- `arcana_winapi.raw.*` for raw binding-facing surface such as exported callback signatures
- `arcana_winapi.helpers.*` for thin Win32 helper routines consumers should build on
- compatibility wrappers like `arcana_winapi.foundation`, `arcana_winapi.fonts`, and `arcana_winapi.windows` remain available during the migration

Its v1 proof slice covers:
- foundation helpers
  - UTF-16 length utility
  - current module handle/path
  - normalized error transport
- fonts
  - system font catalog enumeration
  - family/face/full-name/PostScript/path metadata
  - stable file-path lookup when DirectWrite exposes it
- desktop-shell proof
  - hidden/basic window creation
  - message posting/pumping
  - explicit callback registration/trampoline for window procedures

## Boundaries

- `arcana_text` may consume `arcana_winapi` for host-installed font discovery and related metadata.
- Future `arcana_desktop` migration may consume `arcana_winapi` for incremental Win32 ownership work.
- `arcana_text` and `arcana_desktop` must not regain direct runtime special cases once this seam exists.
- No library package should talk to `windows-sys` directly in the library binding seam.
- Existing `windows-sys` usage in rewrite runtime host code and Windows-only CLI/runtime test harnesses is transitional host debt, not part of the library binding seam.
