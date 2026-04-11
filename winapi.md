# Full `arcana_winapi` Buildout on Revised Binding CABI v1, Including Audio

## Summary
- Expand `arcana_winapi` from the current proof slice into the broad first-party Windows raw/helper layer needed by `arcana_desktop`, `arcana_text`, `arcana_audio`, and future `softbuffer` / HAL / `wgpu`-class consumers.
- Revise `arcana.cabi.binding.v1` in place so `shackle` can carry real Windows-native layouts, pointers, callbacks, COM-style interfaces, and bitfields through the binding seam.
- Keep runtime migration out of scope. Do not retire or rewrite runtime special cases in this phase. Only allow narrow generic runtime consumer updates that are strictly required to execute the revised CABI value/layout transport.
- Keep public networking out of scope. Audio raw families are explicitly in scope.

## Key Changes
- Revise `crates/arcana-cabi` so binding v1 carries real raw native shapes:
  - Extend the binding type model beyond the proof-slice transport to include fixed-width signed/unsigned integers, `isize` / `usize`, `f32` / `f64`, raw pointers, function pointers, and layout-bearing native values keyed by stable layout ids.
  - Add descriptor layout tables for `shackle` aliases, structs, unions, fixed arrays, integer-backed enums, flags/newtypes, callbacks, COM GUID-bearing interface layouts, and named bitfields.
  - Freeze Windows/MSVC bitfield rules in v1: `shackle struct` only, named fields only, fixed-width integer bases only, no anonymous or zero-width fields, low-order-bit-first, overflow starts a new storage unit.
  - Keep callback symmetry, `edit` write-backs, and owned `Str` / `Bytes`, and add canonical encode/decode helpers for raw scalar, pointer, callback, and native-layout values so runtime/AOT do not invent local ABI rules.

- Complete the `shackle` pipeline without changing Arcana syntax:
  - Carry stable layout ids, DLL/symbol metadata, calling convention, callback-thunk metadata, and raw module exports through frontend, HIR, IR, package metadata, and AOT.
  - Make exported `shackle type`, `const`, `import fn`, `fn`, and `callback` fully dependency-visible.
  - Generate binding products that emit raw `extern "system"` imports, native layouts, callback thunks, and package-owned host routines directly from source, with no handwritten support crate and no Win32-specific runtime branches.

- Expand `grimoires/arcana/winapi` into the broad Windows raw/helper layer:
  - Build out `arcana_winapi.raw.*` for `types`, `constants`, `kernel32`, `user32`, `gdi32`, `dwmapi`, `shcore`, `shell32`, `imm32`, `ole32` / `combase`, `dxgi`, `d3d12`, `dwrite`, `d2d1`, `wic`, and audio modules.
  - Audio modules must include at least `mmdeviceapi`, `audioclient`, `audiopolicy`, `endpointvolume`, `avrt`, `mmreg`, `ksmedia`, `propsys` / device-property key support, `xaudio2`, and `x3daudio`.
  - `raw.types` must cover the handle, pointer, GUID, COM vtable/interface, monitor, message, IME, bitmap, DXGI, D3D12, DirectWrite, Direct2D, and audio layout families these modules require.
  - Export the callback signatures needed by desktop and audio/native integrations, including window procedures and representative COM/audio callback signatures used by the supported families.

- Build the thin Win32 ceremony layer in `arcana_winapi.helpers.*`:
  - UTF-16/string conversion, Win32/HRESULT error helpers, COM init/query/release helpers, GUID/property-key helpers.
  - Window class/message-loop, DPI/monitor/theme, IME/composition, clipboard, and file-drop helpers.
  - DirectWrite font/text helpers, GDI softbuffer helpers, Direct2D/WIC image/text bootstrap helpers, and DXGI/D3D12 bootstrap helpers.
  - Audio helpers for MMDevice enumeration/default-device selection, WASAPI format negotiation and stream bootstrap, render-client buffer pump setup, endpoint/session policy helpers, AVRT thread registration, and XAudio2 engine/voice bootstrap.
  - Keep `foundation`, `fonts`, and `windows` as compatibility wrappers rebuilt on top of `raw.*` and `helpers.*`.

- Keep phase boundaries explicit:
  - Do not rewire `std.*`, `arcana_desktop`, `arcana_text`, or `arcana_audio` onto `arcana_winapi` in this phase.
  - Do not remove current runtime-owned `std.kernel.*` substrate branches in this phase.
  - Design the revised CABI/raw layer so later expansions are additive, not another ABI rebuild.

## Test Plan
- `arcana-cabi` contract coverage:
  - parse/render/validation for revised binding v1 metadata and layout tables
  - raw scalar, pointer, function-pointer, layout-value, COM-pointer, callback, and write-back roundtrips
  - Windows/MSVC size/align/offset and bitfield packing tests
  - C header generation for the revised binding value/layout contract

- `shackle` and AOT coverage:
  - frontend/HIR/IR tests for all existing `shackle` declaration kinds under the expanded raw layout model
  - dependency-resolution tests for exported raw types, consts, functions, and callback signatures
  - generated binding tests for raw imports, nested modules, callback thunks, COM-style layouts, DirectX/DirectWrite layouts, and audio layouts

- `arcana_winapi` acceptance coverage:
  - desktop/window smokes for class registration, window creation/destruction, message pump, DPI/monitor/theme/query, IME hooks, clipboard, and file-drop helpers
  - GDI softbuffer-style path proving software blit/present against a native window
  - DirectWrite/Direct2D/WIC tests covering font enumeration, text format/layout creation, image bootstrap, and metrics queries
  - DXGI/D3D12 bootstrap tests covering factory/adapter enumeration, device creation, queue/fence/allocator/list setup, and swapchain-capable window integration
  - audio tests covering MMDevice enumeration/default output, WASAPI client/bootstrap, format support queries, render-client setup, endpoint/session controls, AVRT registration helpers, and XAudio2 engine/voice bootstrap
  - callback tests for `WNDPROC` and representative exported callback signatures used by the supported desktop/audio modules

- Runtime safety coverage:
  - generic runtime consumer tests proving the revised binding v1 executes through the current runtime lane with no new WinAPI-specific special cases and no substrate-retirement work in this phase

## Assumptions And Defaults
- Windows is the only target for this buildout, and layout/ABI rules follow the Windows/MSVC lane.
- The binding contract is revised in place as `arcana.cabi.binding.v1`.
- No new Arcana syntax is added; `shackle` is the only raw-binding source surface.
- Public networking/Winsock remains deferred.
- Audio is part of the same buildout: land the WASAPI floor and XAudio2/X3DAudio now, while keeping the CABI/raw model open to additive future families without another architecture rewrite.
