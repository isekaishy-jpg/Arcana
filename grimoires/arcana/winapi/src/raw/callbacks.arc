export shackle callback WNDPROC(read hwnd: arcana_winapi.raw.types.HWND, message: arcana_winapi.raw.types.UINT, wparam: arcana_winapi.raw.types.WPARAM, lparam: arcana_winapi.raw.types.LPARAM) -> arcana_winapi.raw.types.LRESULT
export shackle callback XAUDIO2_ENGINE_ON_PROCESSING_PASS_START() -> Unit
export shackle callback XAUDIO2_ENGINE_ON_PROCESSING_PASS_END() -> Unit
export shackle callback XAUDIO2_ENGINE_ON_CRITICAL_ERROR(read error: arcana_winapi.raw.types.HRESULT) -> Unit
export shackle callback XAUDIO2_VOICE_ON_BUFFER_END(read context: arcana_winapi.raw.types.LPVOID) -> Unit
