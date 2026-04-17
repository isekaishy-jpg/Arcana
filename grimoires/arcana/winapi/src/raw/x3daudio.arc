// GENERATED FILE. DO NOT EDIT BY HAND.
// Source of truth: grimoires/arcana/winapi/generation/exceptions.toml
// Projection config: grimoires/arcana/winapi/generation/projection.toml
// Source authority: Pinned Windows SDK metadata snapshot
// Metadata authority: Windows.Win32.winmd Microsoft.Windows.SDK.Win32Metadata 63.0.31 sha256:97D24CF1A9DC3E50782BBF1DBA0952BF6A025FA583D8B3AE6C5EF713B463C869
// Parity target: windows-sys; pinned metadata wins on disagreement.

export shackle import fn X3DAudioInitialize(channel_mask: arcana_winapi.raw.types.DWORD, speed_of_sound: arcana_winapi.raw.types.FLOAT, handle: arcana_winapi.raw.types.LPVOID) -> arcana_winapi.raw.types.HRESULT = x3daudio.X3DAudioInitialize:
    pub(crate) type X3DAudioInitializeFn = unsafe extern "system" fn(
        crate::raw::types::DWORD,
        crate::raw::types::FLOAT,
        crate::raw::types::LPVOID,
    ) -> crate::raw::types::HRESULT;
    pub(crate) unsafe fn X3DAudioInitialize(
        channel_mask: crate::raw::types::DWORD,
        speed_of_sound: crate::raw::types::FLOAT,
        handle: crate::raw::types::LPVOID,
    ) -> crate::raw::types::HRESULT {
        let library_name = "X3DAudio1_7.dll\0".encode_utf16().collect::<Vec<u16>>();
        let module = unsafe { crate::raw::kernel32::LoadLibraryW(library_name.as_ptr().cast_mut()) };
        if module.is_null() {
            return -1i32;
        }
        let symbol = b"X3DAudioInitialize\0";
        let proc = unsafe { crate::raw::kernel32::GetProcAddress(module, symbol.as_ptr().cast::<i8>().cast_mut()) };
        if proc.is_null() {
            unsafe {
                crate::raw::kernel32::FreeLibrary(module);
            }
            return -1i32;
        }
        let init: X3DAudioInitializeFn = unsafe { std::mem::transmute(proc) };
        let hr = unsafe { init(channel_mask, speed_of_sound, handle) };
        unsafe {
            crate::raw::kernel32::FreeLibrary(module);
        }
        hr
    }
