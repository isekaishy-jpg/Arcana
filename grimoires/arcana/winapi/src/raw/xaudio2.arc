// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/imports.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn XAudio2CreateWithVersionInfo(pp_xaudio2: arcana_winapi.raw.types.PIXAUDIO2, flags: arcana_winapi.raw.types.U32, xaudio2_processor: arcana_winapi.raw.types.U32, ntddi_version: arcana_winapi.raw.types.U32) -> arcana_winapi.raw.types.HRESULT = xaudio2.XAudio2CreateWithVersionInfo
