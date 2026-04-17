// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/callbacks.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle callback WNDPROC(hwnd: arcana_winapi.raw.types.HWND, message: arcana_winapi.raw.types.U32, wparam: arcana_winapi.raw.types.WPARAM, lparam: arcana_winapi.raw.types.LPARAM) -> arcana_winapi.raw.types.LRESULT
export shackle callback XAUDIO2_ENGINE_ON_PROCESSING_PASS_START()
export shackle callback XAUDIO2_ENGINE_ON_PROCESSING_PASS_END()
export shackle callback XAUDIO2_ENGINE_ON_CRITICAL_ERROR(read error: arcana_winapi.raw.types.HRESULT)
export shackle callback XAUDIO2_VOICE_ON_BUFFER_END(read context: arcana_winapi.raw.types.HANDLE)
