export shackle import fn StringFromGUID2(guid: arcana_winapi.raw.types.REFGUID, buffer: arcana_winapi.raw.types.LPWSTR, length: arcana_winapi.raw.types.I32) -> arcana_winapi.raw.types.I32 = ole32.StringFromGUID2
export shackle import fn CLSIDFromString(text: arcana_winapi.raw.types.LPCWSTR, clsid: arcana_winapi.raw.types.PGUID) -> arcana_winapi.raw.types.HRESULT = ole32.CLSIDFromString
