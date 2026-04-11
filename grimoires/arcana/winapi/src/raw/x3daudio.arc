shackle flags X3DAudioDynloadInternals:
    pub(crate) type X3DAudioInitializeFn = unsafe extern "system" fn(
        crate::raw::types::DWORD,
        crate::raw::types::FLOAT,
        crate::raw::types::LPVOID,
    ) -> crate::raw::types::HRESULT;

export shackle fn X3DAudioInitialize(channel_mask: arcana_winapi.raw.types.DWORD, speed_of_sound: arcana_winapi.raw.types.FLOAT, handle: arcana_winapi.raw.types.LPVOID) -> arcana_winapi.raw.types.HRESULT = x3daudio.X3DAudioInitialize:
    let library_name = "X3DAudio1_7.dll\0".encode_utf16().collect::<Vec<u16>>();
    let module = unsafe { crate::raw::kernel32::LoadLibraryW(library_name.as_ptr()) };
    if module.is_null() {
        return -1i32;
    }
    let symbol = b"X3DAudioInitialize\0";
    let proc = unsafe { crate::raw::kernel32::GetProcAddress(module, symbol.as_ptr().cast()) };
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
