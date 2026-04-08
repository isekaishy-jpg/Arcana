export shackle import fn GetLastError() -> arcana_winapi.raw.types.DWORD = kernel32.GetLastError
export shackle import fn GetModuleFileNameW(module: arcana_winapi.raw.types.HMODULE, buffer: arcana_winapi.raw.types.LPWSTR, size: arcana_winapi.raw.types.DWORD) -> arcana_winapi.raw.types.DWORD = kernel32.GetModuleFileNameW
export shackle import fn GetModuleHandleExW(flags: arcana_winapi.raw.types.DWORD, address: arcana_winapi.raw.types.LPCVOID, module: arcana_winapi.raw.types.PHMODULE) -> arcana_winapi.raw.types.BOOL = kernel32.GetModuleHandleExW
