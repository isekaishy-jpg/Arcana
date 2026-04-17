// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn CLSIDFromProgID(lpsz_prog_id: arcana_winapi.raw.types.PWSTR, lpclsid: arcana_winapi.raw.types.PGUID) -> arcana_winapi.raw.types.HRESULT = ole32.CLSIDFromProgID
export shackle import fn CLSIDFromProgIDEx(lpsz_prog_id: arcana_winapi.raw.types.PWSTR, lpclsid: arcana_winapi.raw.types.PGUID) -> arcana_winapi.raw.types.HRESULT = ole32.CLSIDFromProgIDEx
export shackle import fn CLSIDFromString(lpsz: arcana_winapi.raw.types.PWSTR, pclsid: arcana_winapi.raw.types.PGUID) -> arcana_winapi.raw.types.HRESULT = ole32.CLSIDFromString
export shackle import fn IIDFromString(lpsz: arcana_winapi.raw.types.PWSTR, lpiid: arcana_winapi.raw.types.PGUID) -> arcana_winapi.raw.types.HRESULT = ole32.IIDFromString
export shackle import fn ProgIDFromCLSID(clsid: arcana_winapi.raw.types.PGUID, lplpsz_prog_id: arcana_winapi.raw.types.PPWSTR) -> arcana_winapi.raw.types.HRESULT = ole32.ProgIDFromCLSID
export shackle import fn StringFromCLSID(rclsid: arcana_winapi.raw.types.PGUID, lplpsz: arcana_winapi.raw.types.PPWSTR) -> arcana_winapi.raw.types.HRESULT = ole32.StringFromCLSID
export shackle import fn StringFromGUID2(rguid: arcana_winapi.raw.types.PGUID, lpsz: arcana_winapi.raw.types.PWSTR, cch_max: arcana_winapi.raw.types.I32) -> arcana_winapi.raw.types.I32 = ole32.StringFromGUID2
export shackle import fn StringFromIID(rclsid: arcana_winapi.raw.types.PGUID, lplpsz: arcana_winapi.raw.types.PPWSTR) -> arcana_winapi.raw.types.HRESULT = ole32.StringFromIID
