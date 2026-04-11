export shackle import fn CoInitializeEx(reserved: arcana_winapi.raw.types.LPVOID, flags: arcana_winapi.raw.types.DWORD) -> arcana_winapi.raw.types.HRESULT = ole32.CoInitializeEx
export shackle import fn CoUninitialize() = ole32.CoUninitialize
export shackle import fn CoCreateInstance(clsid: arcana_winapi.raw.types.REFGUID, outer: arcana_winapi.raw.types.LPVOID, clsctx: arcana_winapi.raw.types.DWORD, iid: arcana_winapi.raw.types.REFIID, object: arcana_winapi.raw.types.PPVOID) -> arcana_winapi.raw.types.HRESULT = ole32.CoCreateInstance
export shackle import fn CoTaskMemFree(memory: arcana_winapi.raw.types.LPVOID) = ole32.CoTaskMemFree
