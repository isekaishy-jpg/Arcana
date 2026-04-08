export shackle type BOOL = i32
export shackle type DWORD = u32
export shackle type UINT = u32
export shackle type WORD = u16
export shackle type LONG = i32
export shackle type LONG_PTR = isize
export shackle type WPARAM = usize
export shackle type LPARAM = isize
export shackle type LRESULT = isize
export shackle type ATOM = u16
export shackle type HMODULE = *mut c_void
export shackle type HWND = *mut c_void
export shackle type HMENU = *mut c_void
export shackle type HICON = *mut c_void
export shackle type HCURSOR = *mut c_void
export shackle type HBRUSH = *mut c_void
export shackle type LPVOID = *mut c_void
export shackle type LPCVOID = *const c_void
export shackle type LPCWSTR = *const u16
export shackle type LPWSTR = *mut u16
export shackle type PHMODULE = *mut arcana_winapi.raw.types.HMODULE
export shackle type RAW_WNDPROC = Option<unsafe extern "system" fn(arcana_winapi.raw.types.HWND, arcana_winapi.raw.types.UINT, arcana_winapi.raw.types.WPARAM, arcana_winapi.raw.types.LPARAM) -> arcana_winapi.raw.types.LRESULT>
export shackle type PCWNDCLASSW = *const arcana_winapi.raw.types.WNDCLASSW
export shackle type LPMSG = *mut arcana_winapi.raw.types.MSG
export shackle type PCMSG = *const arcana_winapi.raw.types.MSG
export shackle type PCREATESTRUCTW = *const arcana_winapi.raw.types.CREATESTRUCTW

export shackle struct POINT:
    x: i32,
    y: i32,

export shackle struct MSG:
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
    lPrivate: DWORD,

export shackle struct WNDCLASSW:
    style: UINT,
    lpfnWndProc: RAW_WNDPROC,
    cbClsExtra: i32,
    cbWndExtra: i32,
    hInstance: HMODULE,
    hIcon: HICON,
    hCursor: HCURSOR,
    hbrBackground: HBRUSH,
    lpszMenuName: LPCWSTR,
    lpszClassName: LPCWSTR,

export shackle struct CREATESTRUCTW:
    lpCreateParams: LPVOID,
    hInstance: HMODULE,
    hMenu: HMENU,
    hwndParent: HWND,
    cy: i32,
    cx: i32,
    y: i32,
    x: i32,
    style: LONG,
    lpszName: LPCWSTR,
    lpszClass: LPCWSTR,
    dwExStyle: DWORD,
