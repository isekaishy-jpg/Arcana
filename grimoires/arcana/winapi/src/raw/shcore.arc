// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn GetDpiForMonitor(hmonitor: arcana_winapi.raw.types.HMONITOR, dpi_type: arcana_winapi.raw.types.I32, dpi_x: arcana_winapi.raw.types.PUINT, dpi_y: arcana_winapi.raw.types.PUINT) -> arcana_winapi.raw.types.HRESULT = shcore.GetDpiForMonitor
export shackle import fn GetProcessDpiAwareness(hprocess: arcana_winapi.raw.types.HANDLE, value: arcana_winapi.raw.types.PI32) -> arcana_winapi.raw.types.HRESULT = shcore.GetProcessDpiAwareness
export shackle import fn SetProcessDpiAwareness(value: arcana_winapi.raw.types.I32) -> arcana_winapi.raw.types.HRESULT = shcore.SetProcessDpiAwareness
