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
  - `producer = "rust-cdylib"` in the current proof lane
- Arcana source in that same package may declare:
  - `native fn ... = host.path`
  - `native callback ... = package.path`
- Consumers depend on the library package only. They do not activate or configure the binding sidecar explicitly in their dependency edge.

## Language Surface

- `native fn`
  - package-scoped imported host call surface
  - used by libraries like `arcana_winapi`
  - not a replacement for `intrinsic fn`
- `native callback`
  - package-scoped explicit callback registration surface
  - callbacks target ordinary Arcana routines by path
  - callbacks are typed and declared up front; they are not ad hoc symbol lookups
- `opaque type`
  - binding-owning packages may export source-declared opaque handle types for native values such as module handles, font catalogs, windows, and callback tokens

## Rules

- `intrinsic fn` remains the trusted std/kernel-only surface.
- Generic OS binding work must not be reintroduced as runtime package-name special cases.
- Binding grimoires must own the Arcana-facing API; runtime owns only the generic loading/invocation plumbing.
- Binding grimoires must keep host-native scope narrow:
  - host capability discovery
  - host handles
  - explicit callbacks
  - thin raw API coverage
- Higher-level policy remains in ordinary grimoires above the binding layer.
- The first proof lane is Windows only.

## `arcana_winapi` v1 Proof Slice

`arcana_winapi` is the first public OS-binding grimoire.

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
- No library package besides the dedicated Windows binding crate should talk to `windows-sys` directly in the current proof lane.
- Existing `windows-sys` usage in rewrite runtime host code and Windows-only CLI/runtime test harnesses is transitional host debt, not part of the library binding seam.
