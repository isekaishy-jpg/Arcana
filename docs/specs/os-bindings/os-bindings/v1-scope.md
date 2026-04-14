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
  - consumer grimoires may reference those binding-owned opaque types in their own public APIs, but they must not redeclare or re-alias owner-local handle families for the same host resource

## Rules

- `intrinsic fn` remains the trusted std/kernel-only surface.
- Generic OS binding work must not be reintroduced as runtime package-name special cases.
- `arcana-cabi` owns the foreign-boundary semantics; runtime owns only generic loading and invocation as a consumer of that contract.
- Binding-owned mapped byte views must ride the generic binding CABI backing-ops table; runtime must not fall back to package-name-specific hidden imports for foreign view access.
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

## `arcana_winapi` v1 Surface

`arcana_winapi` is the first public OS-binding grimoire.

Its public package shape is:
- `arcana_winapi.raw.*` for public raw Win32-facing surface
- `arcana_winapi.helpers.*` for thin Win32 helper routines consumers should build on
- `arcana_winapi.desktop_handles`, `arcana_winapi.process_handles`, and `arcana_winapi.audio_handles` for canonical binding-owned opaque handle declarations that higher-level grimoires must reference directly when they need those types
- compatibility wrappers like `arcana_winapi.foundation`, `arcana_winapi.fonts`, and `arcana_winapi.windows` remain available during the migration

Its current v1 raw surface covers:
- core type/layout families in `arcana_winapi.raw.types`
  - handles, pointers, GUIDs, messages, monitors, IME/bitmap layouts
  - DXGI, D3D12, DirectWrite, Direct2D, WIC, and audio-facing COM/layout families
- raw Win32 module families
  - `kernel32`, `user32`, `gdi32`, `dwmapi`, `shcore`, `shell32`, `imm32`
  - `ole32`, `combase`
  - `dxgi`, `d3d12`, `dwrite`, `d2d1`, `wic`
  - `mmdeviceapi`, `audioclient`, `audiopolicy`, `endpointvolume`, `avrt`, `mmreg`, `ksmedia`, `propsys`, `xaudio2`, `x3daudio`
- exported callback/type surface
  - window-proc callback declaration through `arcana_winapi.raw.callbacks.WNDPROC`
  - representative audio callback declarations such as `XAUDIO2_ENGINE_ON_CRITICAL_ERROR` and `XAUDIO2_VOICE_ON_BUFFER_END`
  - raw COM-style interface/vtable layouts for the supported graphics/text/audio families

Its current helper surface covers:
- strings/errors/com
  - UTF-16 helpers
  - HRESULT helpers
  - COM init/uninit, GUID text, property-key helpers
- windowing
  - hidden window creation/destruction
  - message posting/pumping
  - DPI/monitor queries
  - dark-mode attribute roundtrip
  - client/frame rect queries
  - clipboard, file-drop, and IME helper routines
- graphics/text
  - GDI software-present path
  - DXGI adapter enumeration
  - DXGI hidden-window swapchain bootstrap
  - D3D12 WARP bootstrap including queue/allocator/list/fence setup
  - Direct2D and WIC factory bootstrap
  - DirectWrite system font count and text-layout bootstrap
- audio
  - MMDevice enumeration/default render bootstrap
  - WASAPI default render bootstrap
  - WASAPI render-client bootstrap
  - endpoint volume bootstrap
  - session policy bootstrap
  - AVRT registration helper
  - XAudio2 and X3DAudio bootstrap helpers

## Boundaries

- `arcana_text` may consume `arcana_winapi` for host-installed font discovery and related metadata.
- `arcana_desktop` is expected to consume `arcana_winapi` for its Windows shell implementation rather than keep runtime-owned public desktop special cases alive.
- `arcana_desktop`, `arcana_process`, and `arcana_audio` may traffic in those binding-owned handles through their routines and records, but type references must still resolve to the `arcana_winapi.*_handles` declarations.
- Future graphics/audio grimoires such as `arcana_graphics`, `arcana_hal`, and `arcana_audio` may consume the raw/helper surface without reviving handwritten Rust bridge layers.
- `arcana_text` and `arcana_desktop` must not regain direct runtime special cases once this seam exists.
- No library package should talk to `windows-sys` directly in the library binding seam.
- Rewrite crates must not keep a parallel `windows-sys` host lane beside this binding seam; Win32 access should flow through `arcana_winapi` and consumer packages.
