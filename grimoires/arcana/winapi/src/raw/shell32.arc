export shackle import fn DragQueryFileW(drop: arcana_winapi.raw.types.HDROP, file: arcana_winapi.raw.types.UINT, buffer: arcana_winapi.raw.types.LPWSTR, length: arcana_winapi.raw.types.UINT) -> arcana_winapi.raw.types.UINT = shell32.DragQueryFileW
export shackle import fn DragFinish(drop: arcana_winapi.raw.types.HDROP) = shell32.DragFinish
