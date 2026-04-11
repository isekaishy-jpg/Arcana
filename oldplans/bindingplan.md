# Generic OS Binding Seam With `arcana.winapi`

## Summary
- Add a real generic native-binding lane for Arcana packages, separate from the removed provider lane.
- Keep the low-level Windows leaf in one new workspace crate, `crates/arcana-winapi`, which is the only place raw `windows-sys` usage lives.
- Add a new Arcana grimoire at `grimoires/arcana/winapi` as the public binding surface and long-term self-host migration target.
- Make the first proof slice cover both:
  - foundation primitives needed by any OS binding layer
  - concrete Windows font discovery/metadata for `arcana_text`
  - concrete Windows window/message/input callback plumbing for future `arcana_desktop` migration

## Key Changes
- Extend the native-product contract with a new role: `binding`.
  - Keep the good generic pieces from the old provider work: CABI descriptor ownership, last-error transport, opaque/view/write-back handling, package staging, and runtime loader validation.
  - Do not revive provider-style dynamic library APIs or text-specific host callbacks.
- Add a package-scoped Arcana source surface for native bindings.
  - Introduce `native fn` declarations for imported host calls.
  - Introduce `native callback` declarations for explicit Arcana-to-native callback registration.
  - Keep `intrinsic fn` as the internal std/kernel surface; do not reuse it for library OS bindings.
- Broaden handle support for binding grimoires.
  - Allow exported opaque handle types in non-`std` packages where the package owns a binding product.
  - Use opaque types for Win32 handles and callback tokens in `arcana.winapi`.
- Keep the package model simple in v1.
  - A library package with native bindings declares one default `binding` product in `book.toml`.
  - Source in that same package lowers `native fn` / `native callback` against that binding product automatically.
  - Consumer packages only depend on the library; they do not configure the binding sidecar directly.
- Build the native half around one Rust crate.
  - `crates/arcana-winapi` wraps `windows-sys` and exports the `binding` product for `grimoires/arcana/winapi`.
  - No other crate or grimoire talks to `windows-sys` directly.
- Make the Arcana grimoire the public API and future migration target.
  - Create `grimoires/arcana/winapi` with raw-but-typed Windows modules under it.
  - Keep naming close to Windows concepts so later self-host replacement is 1:1, not a second abstraction rewrite.
  - Higher-level grimoires like text and desktop consume `arcana.winapi`; they do not get direct runtime or raw native hooks.
- Define bidirectional callbacks explicitly.
  - Use explicit typed callback registration only.
  - No arbitrary symbol lookup from native code into Arcana.
  - No event-queue-only fallback in v1.
  - The callback proof path must be strong enough for Win32-style callback cases such as window/message procedures and similar explicit registrations.
- Scope the first Windows API coverage deliberately.
  - Foundation:
    - UTF-16 string conversion helpers
    - error/result normalization
    - module/library handles and basic loader helpers
    - opaque handle ownership rules
  - Fonts:
    - system font enumeration
    - family/face metadata lookup
    - stable font identity/path or equivalent lookup suitable for `arcana_text`
    - use the modern Windows font stack for this proof, not manual filesystem probing
  - Desktop shell:
    - hidden/basic window creation
    - message posting/pumping
    - explicit callback/trampoline flow for window/message handling
    - enough input/message coverage to prove the seam can later replace `winit`-owned pieces incrementally
- Update the approved contract in the same patch series.
  - `native-products-cabi` scope adds the `binding` role.
  - package management scope explains package-owned binding products for libraries.
  - frozen/approved language docs stop treating std-only opaque handles and std-only intrinsic-style native hooks as the only model; the new library binding path becomes explicit.
  - add a new approved OS/bindings scope that makes `arcana.winapi` the first concrete consumer of the generic seam.

## Public Interfaces
- `book.toml`
  - new native product role: `binding`
  - `grimoires/arcana/winapi/book.toml` declares a default binding product backed by `producer = "rust-cdylib"` and `rust_cdylib_crate = "arcana-winapi"`
- Arcana source
  - new package-scoped `native fn` declaration surface
  - new package-scoped `native callback` declaration surface
  - exported opaque handles allowed for binding-owning grimoires
- Public grimoire
  - new package: `arcana.winapi`
  - v1 modules cover foundation, fonts, and desktop-shell/message proof surfaces
- Runtime/toolchain
  - loader/build/CABI/AOT support for `binding` products
  - automatic package-local binding activation for source using native declarations

## Test Plan
- Contract and packaging
  - manifest parsing/validation for `role = "binding"`
  - build/distribution staging and fingerprinting for binding products
  - runtime loader validation for binding descriptors and contract IDs
- Language/toolchain
  - parser/frontend/IR coverage for `native fn`, `native callback`, and non-`std` opaque binding handles
  - AOT/native lowering coverage for imported calls, explicit callback registration, callback invocation, error propagation, and write-back
- WinAPI proof
  - foundation smoke for UTF-16 conversion, error transport, and module/library helpers
  - font smoke that enumerates installed system fonts and returns non-empty metadata on Windows
  - hidden-window/message-loop smoke proving create/post/pump/destroy through the new seam
  - bidirectional callback smoke proving a native Win32 callback path can enter Arcana and return typed results
- Boundary checks
  - `arcana_text` and future `arcana_desktop` consume `arcana.winapi` instead of adding new runtime special cases
  - no package besides `arcana-winapi` reaches `windows-sys` directly

## Assumptions
- Windows is the only implementation target in v1; Linux/macOS stay out of scope until after self-host.
- The plan mirrors Rust at the architectural level: low-level OS crate plus typed public package, not a huge provider-style service boundary.
- `binding` is the new native-product role name.
- `arcana.winapi` is the first public OS-binding grimoire and the long-term Arcana-side migration target.
- Bidirectional support means explicit typed callbacks now, not late async queues and not native-side symbol lookup.
- The first milestone proves the generic seam with both fonts and desktop shell primitives, not a text-only slice.
