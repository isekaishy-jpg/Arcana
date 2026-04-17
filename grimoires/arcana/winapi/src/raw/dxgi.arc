// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn CreateDXGIFactory(riid: arcana_winapi.raw.types.PGUID, pp_factory: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = dxgi.CreateDXGIFactory
export shackle import fn CreateDXGIFactory1(riid: arcana_winapi.raw.types.PGUID, pp_factory: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = dxgi.CreateDXGIFactory1
export shackle import fn CreateDXGIFactory2(flags: arcana_winapi.raw.types.U32, riid: arcana_winapi.raw.types.PGUID, pp_factory: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = dxgi.CreateDXGIFactory2
export shackle import fn DXGIDeclareAdapterRemovalSupport() -> arcana_winapi.raw.types.HRESULT = dxgi.DXGIDeclareAdapterRemovalSupport
export shackle import fn DXGIGetDebugInterface1(flags: arcana_winapi.raw.types.U32, riid: arcana_winapi.raw.types.PGUID, p_debug: arcana_winapi.raw.types.HANDLE) -> arcana_winapi.raw.types.HRESULT = dxgi.DXGIGetDebugInterface1
