shackle type BOOL = i32
shackle type DWORD = u32
shackle type UINT = u32
shackle type WORD = u16
shackle type LONG = i32
shackle type LONG_PTR = isize
shackle type WPARAM = usize
shackle type LPARAM = isize
shackle type LRESULT = isize
shackle type ATOM = u16
shackle type HMODULE = *mut c_void
shackle type HWND = *mut c_void
shackle type HMENU = *mut c_void
shackle type HICON = *mut c_void
shackle type HCURSOR = *mut c_void
shackle type HBRUSH = *mut c_void
shackle type LPVOID = *mut c_void
shackle type LPCVOID = *const c_void
shackle type LPCWSTR = *const u16
shackle type LPWSTR = *mut u16
shackle type PHMODULE = *mut HMODULE
shackle type RAW_WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>
shackle type PCWNDCLASSW = *const WNDCLASSW
shackle type LPMSG = *mut MSG
shackle type PCMSG = *const MSG
shackle type PCREATESTRUCTW = *const CREATESTRUCTW

shackle const ERROR_CLASS_ALREADY_EXISTS: DWORD = 1410
shackle const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: DWORD = 2
shackle const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: DWORD = 4
shackle const GWLP_USERDATA: i32 = -21
shackle const WM_CLOSE: UINT = 16
shackle const WM_NCCREATE: UINT = 129
shackle const WM_APP: UINT = 32768
shackle const PM_REMOVE: UINT = 1
shackle const WS_OVERLAPPED: DWORD = 0
shackle const CW_USEDEFAULT: i32 = -2147483648

shackle struct POINT:
    x: i32,
    y: i32,

shackle struct MSG:
    hwnd: HWND,
    message: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
    time: DWORD,
    pt: POINT,
    lPrivate: DWORD,

shackle struct WNDCLASSW:
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

shackle struct CREATESTRUCTW:
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

shackle flags WinapiInternals:
    static HIDDEN_WINDOW_CLASS: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    pub(crate) type WideText = Vec<u16>;
    pub(crate) type ResultUnit = Result<(), String>;
    pub(crate) type ResultModule = Result<HMODULE, String>;
    pub(crate) type ResultString = Result<String, String>;
    pub(crate) struct SystemFontEntry {
        pub(crate) family_name: String,
        pub(crate) face_name: String,
        pub(crate) full_name: String,
        pub(crate) postscript_name: String,
        pub(crate) path: String,
    }
    pub(crate) struct SystemFontCatalogState {
        pub(crate) entries: Vec<SystemFontEntry>,
    }
    pub(crate) type ResultCatalog = Result<SystemFontCatalogState, String>;
    pub(crate) type CatalogStateRef = &'static SystemFontCatalogState;
    pub(crate) type CatalogEntryRef = &'static SystemFontEntry;
    pub(crate) type ResultCatalogRef = Result<CatalogStateRef, String>;
    pub(crate) type ResultCatalogEntryRef = Result<CatalogEntryRef, String>;
    pub(crate) struct SoftwareSurfaceState {
        pub(crate) hwnd: HWND,
        pub(crate) width: i64,
        pub(crate) height: i64,
        pub(crate) stride: i64,
        pub(crate) pixels: Vec<u8>,
        pub(crate) current_map: i64,
        pub(crate) presented_once: bool,
    }
    pub(crate) struct SoftwareSurfaceMapState {
        pub(crate) surface: i64,
    }
    pub(crate) struct WinapiPackageState {
        pub(crate) last_error_text: String,
        pub(crate) desktop_state_handle: u64,
        pub(crate) next_file_stream_handle: u64,
        pub(crate) file_streams: std::collections::BTreeMap<u64, crate::helpers_process_impl::WinapiFileStreamState>,
        pub(crate) next_surface_handle: i64,
        pub(crate) next_surface_map_handle: i64,
        pub(crate) software_surfaces: std::collections::BTreeMap<i64, SoftwareSurfaceState>,
        pub(crate) software_surface_maps: std::collections::BTreeMap<i64, SoftwareSurfaceMapState>,
        pub(crate) next_audio_device_handle: u64,
        pub(crate) audio_devices: std::collections::BTreeMap<u64, crate::helpers_audio_impl::WinapiAudioDeviceState>,
        pub(crate) next_audio_buffer_handle: u64,
        pub(crate) audio_buffers: std::collections::BTreeMap<u64, crate::helpers_audio_impl::WinapiAudioBufferState>,
        pub(crate) next_audio_playback_handle: u64,
        pub(crate) audio_playbacks: std::collections::BTreeMap<u64, crate::helpers_audio_impl::WinapiAudioPlaybackState>,
    }

    pub(crate) fn window_class_name() -> WideText {
        "ArcanaHiddenWindow\0".encode_utf16().collect::<Vec<u16>>()
    }

    pub(crate) fn current_module_handle_for_address(address: LPCVOID) -> ResultModule {
        let mut module: HMODULE = std::ptr::null_mut();
        let ok = unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                address,
                &mut module,
            )
        };
        if ok == 0 {
            return Err(format!(
                "GetModuleHandleExW failed with Win32 error {}",
                unsafe { GetLastError() }
            ));
        }
        Ok(module)
    }

    pub(crate) fn module_path_text(module: HMODULE) -> ResultString {
        let mut buffer = vec![0u16; 260];
        loop {
            let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as DWORD) };
            if len == 0 {
                return Err(format!(
                    "GetModuleFileNameW failed with Win32 error {}",
                    unsafe { GetLastError() }
                ));
            }
            let len = len as usize;
            if len + 1 < buffer.len() {
                return Ok(String::from_utf16_lossy(&buffer[..len]));
            }
            buffer.resize(buffer.len() * 2, 0);
        }
    }

    pub(crate) fn register_hidden_window_class() -> ResultUnit {
        HIDDEN_WINDOW_CLASS
            .get_or_init(|| {
                let module = current_module_handle_for_address(hidden_window_proc as usize as LPCVOID)?;
                let class_name = window_class_name();
                let class = WNDCLASSW {
                    style: 0,
                    lpfnWndProc: Some(hidden_window_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: module,
                    hIcon: std::ptr::null_mut(),
                    hCursor: std::ptr::null_mut(),
                    hbrBackground: std::ptr::null_mut(),
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                };
                let atom = unsafe { RegisterClassW(&class) };
                if atom == 0 {
                    let err = unsafe { GetLastError() };
                    if err != ERROR_CLASS_ALREADY_EXISTS {
                        return Err(format!(
                            "RegisterClassW failed with Win32 error {err}"
                        ));
                    }
                }
                Ok(())
            })
            .clone()
    }

    pub(crate) fn build_system_font_catalog() -> ResultCatalog {
        let font_root = std::env::var("WINDIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("C:\\Windows"))
            .join("Fonts");
        let mut entries = Vec::new();
        let read_dir = std::fs::read_dir(&font_root)
            .map_err(|err| format!("failed to read `{}`: {err}", font_root.display()))?;
        for entry in read_dir {
            let entry = entry
                .map_err(|err| format!("failed to enumerate `{}`: {err}", font_root.display()))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|text| text.to_ascii_lowercase());
            if !matches!(extension.as_deref(), Some("ttf") | Some("ttc") | Some("otf") | Some("fon")) {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("font")
                        .to_string()
                });
            let display_path = path.to_string_lossy().replace('\\', "/");
            entries.push(SystemFontEntry {
                family_name: stem.clone(),
                face_name: stem.clone(),
                full_name: stem.clone(),
                postscript_name: stem.clone(),
                path: display_path,
            });
        }
        entries.sort_by(|left, right| left.full_name.cmp(&right.full_name));
        if entries.is_empty() {
            return Err(format!("no fonts found under `{}`", font_root.display()));
        }
        Ok(SystemFontCatalogState { entries })
    }

    pub(crate) fn catalog_ref(handle: u64) -> ResultCatalogRef {
        let ptr = handle as usize as *const SystemFontCatalogState;
        if ptr.is_null() {
            return Err("SystemFontCatalog handle must not be null".to_string());
        }
        Ok(unsafe { &*ptr })
    }

    pub(crate) fn catalog_entry(handle: u64, index: i64) -> ResultCatalogEntryRef {
        if index < 0 {
            return Err(format!("font catalog index `{index}` must be >= 0"));
        }
        let catalog = catalog_ref(handle)?;
        catalog
            .entries
            .get(index as usize)
            .ok_or_else(|| format!("font catalog index `{index}` is out of range"))
    }

    pub(crate) fn destroy_catalog_handle(handle: u64) {
        let ptr = handle as usize as *mut SystemFontCatalogState;
        if !ptr.is_null() {
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }

    pub(crate) fn software_surface_stride(width: i64, height: i64) -> Result<i64, String> {
        if width <= 0 || height <= 0 {
            return Err("surface dimensions must be > 0".to_string());
        }
        width
            .checked_mul(4)
            .ok_or_else(|| "surface stride overflowed".to_string())
    }

    pub(crate) fn package_state_data_ref(
        instance: &crate::BindingInstance,
    ) -> Result<&WinapiPackageState, String> {
        let ptr = instance.package_state.state_handle as usize as *const WinapiPackageState;
        if ptr.is_null() {
            return Err("binding package state handle must not be null".to_string());
        }
        Ok(unsafe { &*ptr })
    }

    pub(crate) fn package_state_data_mut(
        instance: &mut crate::BindingInstance,
    ) -> Result<&mut WinapiPackageState, String> {
        let ptr = instance.package_state.state_handle as usize as *mut WinapiPackageState;
        if ptr.is_null() {
            return Err("binding package state handle must not be null".to_string());
        }
        Ok(unsafe { &mut *ptr })
    }

    pub(crate) fn destroy_package_state_handle(handle: u64) {
        let ptr = handle as usize as *mut WinapiPackageState;
        if !ptr.is_null() {
            unsafe {
                crate::helpers_desktop_impl::destroy_desktop_state_handle((*ptr).desktop_state_handle);
                drop(Box::from_raw(ptr));
            }
        }
    }

    pub(crate) fn clear_helper_error(instance: &mut crate::BindingInstance) {
        if let Ok(state) = package_state_data_mut(instance) {
            state.last_error_text.clear();
        }
    }

    pub(crate) fn set_helper_error(instance: &mut crate::BindingInstance, message: String) {
        if let Ok(state) = package_state_data_mut(instance) {
            state.last_error_text = message;
        }
    }

    pub(crate) fn take_helper_error(instance: &mut crate::BindingInstance) -> String {
        match package_state_data_mut(instance) {
            Ok(state) => std::mem::take(&mut state.last_error_text),
            Err(_) => String::new(),
        }
    }

    pub(crate) fn software_surface_ref(
        instance: &crate::BindingInstance,
        handle: i64,
    ) -> Result<&SoftwareSurfaceState, String> {
        if handle <= 0 {
            return Err("software surface handle must be > 0".to_string());
        }
        package_state_data_ref(instance)?
            .software_surfaces
            .get(&handle)
            .ok_or_else(|| format!("invalid software surface handle `{handle}`"))
    }

    pub(crate) fn software_surface_mut(
        instance: &mut crate::BindingInstance,
        handle: i64,
    ) -> Result<&mut SoftwareSurfaceState, String> {
        if handle <= 0 {
            return Err("software surface handle must be > 0".to_string());
        }
        package_state_data_mut(instance)?
            .software_surfaces
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid software surface handle `{handle}`"))
    }

    pub(crate) fn software_surface_map_ref(
        instance: &crate::BindingInstance,
        handle: i64,
    ) -> Result<&SoftwareSurfaceMapState, String> {
        if handle <= 0 {
            return Err("software surface map handle must be > 0".to_string());
        }
        package_state_data_ref(instance)?
            .software_surface_maps
            .get(&handle)
            .ok_or_else(|| format!("invalid software surface map handle `{handle}`"))
    }

    pub(crate) fn software_surface_map_remove(
        instance: &mut crate::BindingInstance,
        handle: i64,
    ) -> Result<SoftwareSurfaceMapState, String> {
        if handle <= 0 {
            return Err("software surface map handle must be > 0".to_string());
        }
        let map = package_state_data_mut(instance)?
            .software_surface_maps
            .remove(&handle)
            .ok_or_else(|| format!("invalid software surface map handle `{handle}`"))?;
        if let Some(surface) = package_state_data_mut(instance)?.software_surfaces.get_mut(&map.surface)
            && surface.current_map == handle
        {
            surface.current_map = 0;
        }
        Ok(map)
    }

shackle import fn GetLastError() -> DWORD = kernel32.GetLastError
shackle import fn GetModuleFileNameW(module: HMODULE, buffer: LPWSTR, size: DWORD) -> DWORD = kernel32.GetModuleFileNameW
shackle import fn GetModuleHandleExW(flags: DWORD, address: LPCVOID, module: PHMODULE) -> BOOL = kernel32.GetModuleHandleExW
shackle import fn RegisterClassW(class: PCWNDCLASSW) -> ATOM = user32.RegisterClassW
shackle import fn CreateWindowExW(ex_style: DWORD, class_name: LPCWSTR, window_name: LPCWSTR, style: DWORD, x: i32, y: i32, width: i32, height: i32, parent: HWND, menu: HMENU, instance: HMODULE, param: LPVOID) -> HWND = user32.CreateWindowExW
shackle import fn DestroyWindow(window: HWND) -> BOOL = user32.DestroyWindow
shackle import fn PostMessageW(window: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> BOOL = user32.PostMessageW
shackle import fn PeekMessageW(message: LPMSG, window: HWND, min_filter: UINT, max_filter: UINT, remove: UINT) -> BOOL = user32.PeekMessageW
shackle import fn TranslateMessage(message: PCMSG) -> BOOL = user32.TranslateMessage
shackle import fn DispatchMessageW(message: PCMSG) -> LRESULT = user32.DispatchMessageW
shackle import fn GetWindowLongPtrW(window: HWND, index: i32) -> LONG_PTR = user32.GetWindowLongPtrW
shackle import fn SetWindowLongPtrW(window: HWND, index: i32, value: LONG_PTR) -> LONG_PTR = user32.SetWindowLongPtrW
shackle import fn DefWindowProcW(window: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT = user32.DefWindowProcW

shackle fn package_state_init() = __binding.package_state_init:
    let state = Box::new(WinapiPackageState {
        last_error_text: String::new(),
        desktop_state_handle: crate::helpers_desktop_impl::new_desktop_state_handle(),
        next_file_stream_handle: 1,
        file_streams: std::collections::BTreeMap::new(),
        next_surface_handle: 1,
        next_surface_map_handle: 1,
        software_surfaces: std::collections::BTreeMap::new(),
        software_surface_maps: std::collections::BTreeMap::new(),
        next_audio_device_handle: 1,
        audio_devices: std::collections::BTreeMap::new(),
        next_audio_buffer_handle: 1,
        audio_buffers: std::collections::BTreeMap::new(),
        next_audio_playback_handle: 1,
        audio_playbacks: std::collections::BTreeMap::new(),
    });
    Ok(PackageState {
        last_callback_code: 0,
        state_handle: Box::into_raw(state) as usize as u64,
    })

shackle fn package_state_drop(read state: PackageState) = __binding.package_state_drop:
    destroy_package_state_handle(state.state_handle);

shackle struct PackageState:
    last_callback_code: i64,
    state_handle: u64,

shackle thunk hidden_window_proc(hwnd: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT = win32.hidden_window_proc:
    if message == WM_NCCREATE {
        let create = lparam as PCREATESTRUCTW;
        if !create.is_null() {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as LONG_PTR);
            }
        }
        return 1;
    }
    let instance = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut crate::BindingInstance };
    if message == WM_APP + 1 && !instance.is_null() {
        let args = [
            binding_layout(hwnd),
            binding_layout(message),
            binding_layout(wparam),
            binding_layout(lparam),
        ];
        match unsafe { invoke_callback_value_result(&mut *instance, "window_proc", &args) } {
            Ok(out) => {
                let result = read_layout_arg::<LRESULT>(&out, "window_proc result");
                if let Some(callback) = unsafe { (*instance).callbacks_by_name.get("window_proc").copied() } {
                    let _ = release_binding_output_value(out, callback.owned_bytes_free, callback.owned_str_free);
                }
                match result {
                    Ok(result) => {
                        unsafe {
                            (*instance).package_state.last_callback_code = result as i64;
                        }
                        return result as LRESULT;
                    }
                    Err(err) => {
                        set_last_error(err);
                        return 0;
                    }
                }
            }
            Err(err) => {
                set_last_error(err);
                return 0;
            }
        }
    }
    if message == WM_CLOSE {
        unsafe {
            DestroyWindow(hwnd);
        }
        return 0;
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }

shackle fn foundation_current_module_impl() -> arcana_winapi.raw.types.HMODULE = foundation.current_module:
    Ok(binding_layout(
        current_module_handle_for_address(hidden_window_proc as usize as LPCVOID)?
    ))

shackle fn foundation_module_is_null_impl(read module: arcana_winapi.raw.types.HMODULE) -> Bool = foundation.module_is_null:
    Ok(binding_bool(module.is_null()))

shackle fn foundation_module_path_impl(read module: arcana_winapi.raw.types.HMODULE) -> Str = foundation.module_path:
    Ok(binding_owned_str(module_path_text(module)?))

shackle fn foundation_utf16_len_impl(read text: Str) -> Int = foundation.utf16_len:
    Ok(binding_int(text.encode_utf16().count() as i64))

shackle fn foundation_fail_sample_impl(read message: Str) -> Int = foundation.fail_sample:
    Err(format!("arcana_winapi sample failure: {message}"))

shackle fn fonts_system_font_catalog_impl() -> U64 = fonts.system_font_catalog:
    let catalog = Box::new(build_system_font_catalog()?);
    Ok(binding_u64(Box::into_raw(catalog) as usize as u64))

shackle fn fonts_catalog_count_impl(read catalog: U64) -> Int = fonts.catalog_count:
    Ok(binding_int(catalog_ref(catalog)?.entries.len() as i64))

shackle fn fonts_catalog_family_name_impl(read catalog: U64, index: Int) -> Str = fonts.catalog_family_name:
    Ok(binding_owned_str(catalog_entry(catalog, index)?.family_name.clone()))

shackle fn fonts_catalog_face_name_impl(read catalog: U64, index: Int) -> Str = fonts.catalog_face_name:
    Ok(binding_owned_str(catalog_entry(catalog, index)?.face_name.clone()))

shackle fn fonts_catalog_full_name_impl(read catalog: U64, index: Int) -> Str = fonts.catalog_full_name:
    Ok(binding_owned_str(catalog_entry(catalog, index)?.full_name.clone()))

shackle fn fonts_catalog_postscript_name_impl(read catalog: U64, index: Int) -> Str = fonts.catalog_postscript_name:
    Ok(binding_owned_str(catalog_entry(catalog, index)?.postscript_name.clone()))

shackle fn fonts_catalog_path_impl(read catalog: U64, index: Int) -> Str = fonts.catalog_path:
    Ok(binding_owned_str(catalog_entry(catalog, index)?.path.clone()))

shackle fn fonts_catalog_destroy_impl(take catalog: U64) = fonts.catalog_destroy:
    destroy_catalog_handle(catalog);
    Ok(binding_unit())

shackle fn windows_create_hidden_window_impl() -> arcana_winapi.raw.types.HWND = windows.create_hidden_window:
    register_hidden_window_class()?;
    let module = current_module_handle_for_address(hidden_window_proc as usize as LPCVOID)?;
    let class_name = window_class_name();
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            module,
            instance as *mut crate::BindingInstance as LPVOID,
        )
    };
    if hwnd.is_null() {
        return Err(format!(
            "CreateWindowExW failed with Win32 error {}",
            unsafe { GetLastError() }
        ));
    }
    Ok(binding_layout(hwnd))

shackle fn windows_post_ping_impl(read window: arcana_winapi.raw.types.HWND, code: Int) = windows.post_ping:
    let hwnd = window;
    let ok = unsafe { PostMessageW(hwnd, WM_APP + 1, code as usize, 0) };
    if ok == 0 {
        return Err(format!(
            "PostMessageW failed with Win32 error {}",
            unsafe { GetLastError() }
        ));
    }
    Ok(binding_unit())

shackle fn windows_pump_messages_impl() -> Int = windows.pump_messages:
    let mut message = unsafe { std::mem::zeroed::<MSG>() };
    let mut count = 0i64;
    while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        count += 1;
    }
    Ok(binding_int(count))

shackle fn windows_take_last_callback_code_impl() -> Int = windows.take_last_callback_code:
    let value = instance.package_state.last_callback_code;
    instance.package_state.last_callback_code = 0;
    Ok(binding_int(value))

shackle fn windows_destroy_hidden_window_impl(take window: arcana_winapi.raw.types.HWND) = windows.destroy_hidden_window:
    let hwnd = window;
    let ok = unsafe { DestroyWindow(hwnd) };
    if ok == 0 {
        return Err(format!(
            "DestroyWindow failed with Win32 error {}",
            unsafe { GetLastError() }
        ));
    }
    Ok(binding_unit())
