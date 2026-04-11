shackle flags WinapiHelperInternals:
    pub(crate) const D3D_FEATURE_LEVEL_11_0: u32 = 0xb000;
    pub(crate) const DWRITE_FONT_WEIGHT_NORMAL: u32 = 400;
    pub(crate) const DWRITE_FONT_STYLE_NORMAL: u32 = 0;
    pub(crate) const DWRITE_FONT_STRETCH_NORMAL: u32 = 5;
    pub(crate) const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
    pub(crate) const GCS_COMPSTR: u32 = 0x0008;
    pub(crate) const SPEED_OF_SOUND_METERS_PER_SECOND: f32 = 343.0;

    pub(crate) fn wide_nul(text: &str) -> Vec<u16> {
        text.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    }

    pub(crate) fn guid(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> crate::raw::types::GUID {
        crate::raw::types::GUID {
            data1,
            data2,
            data3,
            data4,
        }
    }

    pub(crate) fn hresult_succeeded_native(code: crate::raw::types::HRESULT) -> bool {
        code >= 0
    }

    pub(crate) fn hresult_failed_native(code: crate::raw::types::HRESULT) -> bool {
        code < 0
    }

    pub(crate) fn iid_idxgi_factory4() -> crate::raw::types::GUID {
        guid(0x1bc6ea02, 0xef36, 0x464f, [0xbf, 0x0c, 0x21, 0xca, 0x39, 0xe5, 0x16, 0x8a])
    }

    pub(crate) fn iid_idxgi_adapter1() -> crate::raw::types::GUID {
        guid(0x29038f61, 0x3839, 0x4626, [0x91, 0xfd, 0x08, 0x68, 0x79, 0x01, 0x1a, 0x05])
    }

    pub(crate) fn iid_id3d12_device() -> crate::raw::types::GUID {
        guid(0x189819f1, 0x1db6, 0x4b57, [0xbe, 0x54, 0x18, 0x21, 0x33, 0x9b, 0x85, 0xf7])
    }

    pub(crate) fn iid_id3d12_command_queue() -> crate::raw::types::GUID {
        guid(0x0ec870a6, 0x5d7e, 0x4c22, [0x8c, 0xfc, 0x5b, 0xaa, 0xe0, 0x76, 0x16, 0xed])
    }

    pub(crate) fn iid_id3d12_command_allocator() -> crate::raw::types::GUID {
        guid(0x6102dee4, 0xaf59, 0x4b09, [0xb9, 0x99, 0xb4, 0x4d, 0x73, 0xf0, 0x9b, 0x24])
    }

    pub(crate) fn iid_id3d12_graphics_command_list() -> crate::raw::types::GUID {
        guid(0x5b160d0f, 0xac1b, 0x4185, [0x8b, 0xa8, 0xb3, 0xae, 0x42, 0xa5, 0xa4, 0x55])
    }

    pub(crate) fn iid_id3d12_fence() -> crate::raw::types::GUID {
        guid(0x0a753dcf, 0xc4d8, 0x4b91, [0xad, 0xf6, 0xbe, 0x5a, 0x60, 0xd9, 0x5a, 0x76])
    }

    pub(crate) fn iid_idwrite_factory() -> crate::raw::types::GUID {
        guid(0xb859ee5a, 0xd838, 0x4b5b, [0xa2, 0xe8, 0x1a, 0xdc, 0x7d, 0x93, 0xdb, 0x48])
    }

    pub(crate) fn iid_id2d1_factory1() -> crate::raw::types::GUID {
        guid(0xbb12d362, 0xdaee, 0x4b9a, [0xaa, 0x1d, 0x14, 0xba, 0x40, 0x1c, 0xfa, 0x1f])
    }

    pub(crate) fn clsid_wic_imaging_factory2() -> crate::raw::types::GUID {
        guid(0x317d06e8, 0x5f24, 0x433d, [0xbd, 0xf7, 0x79, 0xce, 0x68, 0xd8, 0xab, 0xc2])
    }

    pub(crate) fn iid_iwic_imaging_factory() -> crate::raw::types::GUID {
        guid(0xec5ec8a9, 0xc395, 0x4314, [0x9c, 0x77, 0x54, 0xd7, 0xa9, 0x35, 0xff, 0x70])
    }

    pub(crate) fn clsid_mmdevice_enumerator() -> crate::raw::types::GUID {
        guid(0xbcde0395, 0xe52f, 0x467c, [0x8e, 0x3d, 0xc4, 0x57, 0x92, 0x91, 0x69, 0x2e])
    }

    pub(crate) fn iid_immdevice_enumerator() -> crate::raw::types::GUID {
        guid(0xa95664d2, 0x9614, 0x4f35, [0xa7, 0x46, 0xde, 0x8d, 0xb6, 0x36, 0x17, 0xe6])
    }

    pub(crate) fn iid_iaudio_client() -> crate::raw::types::GUID {
        guid(0x1cb9ad4c, 0xdbfa, 0x4c32, [0xb1, 0x78, 0xc2, 0xf5, 0x68, 0xa7, 0x03, 0xb2])
    }

    pub(crate) fn iid_iaudio_client2() -> crate::raw::types::GUID {
        guid(0x726778cd, 0xf60a, 0x4eda, [0x82, 0xde, 0xe4, 0x76, 0x10, 0xcd, 0x78, 0xaa])
    }

    pub(crate) fn iid_iaudio_render_client() -> crate::raw::types::GUID {
        guid(0xf294acfc, 0x3146, 0x4483, [0xa7, 0xbf, 0xad, 0xdc, 0xa7, 0xc2, 0x60, 0xe2])
    }

    pub(crate) fn iid_iaudio_endpoint_volume() -> crate::raw::types::GUID {
        guid(0x5cdf2c82, 0x841e, 0x4546, [0x97, 0x22, 0x0c, 0xf7, 0x40, 0x78, 0x22, 0x9a])
    }

    pub(crate) unsafe fn com_release(object: *mut std::ffi::c_void) {
        if object.is_null() {
            return;
        }
        let vtbl = *(object as *mut *const crate::raw::types::IUnknownVTable);
        ((*vtbl).Release)(object);
    }

    pub(crate) fn enter_com(flags: crate::raw::types::DWORD) -> Result<bool, String> {
        let hr = unsafe { crate::raw::ole32::CoInitializeEx(std::ptr::null_mut(), flags) };
        if hr == crate::raw::constants::RPC_E_CHANGED_MODE {
            return Ok(false);
        }
        if hresult_failed_native(hr) {
            return Err(format!("CoInitializeEx failed with HRESULT {hr}"));
        }
        Ok(true)
    }

    pub(crate) fn leave_com(entered: bool) {
        if entered {
            unsafe {
                crate::raw::ole32::CoUninitialize();
            }
        }
    }

    pub(crate) unsafe fn create_hidden_window_handle(instance: *mut crate::BindingInstance) -> Result<crate::raw::types::HWND, String> {
        crate::shackle::register_hidden_window_class()?;
        let module = crate::shackle::current_module_handle_for_address(
            crate::shackle::hidden_window_proc as usize as crate::shackle::LPCVOID
        )?;
        let class_name = crate::shackle::window_class_name();
        let hwnd = crate::raw::user32::CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            crate::raw::constants::WS_OVERLAPPED,
            crate::raw::constants::CW_USEDEFAULT,
            crate::raw::constants::CW_USEDEFAULT,
            crate::raw::constants::CW_USEDEFAULT,
            crate::raw::constants::CW_USEDEFAULT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            module,
            instance as *mut std::ffi::c_void,
        );
        if hwnd.is_null() {
            return Err(format!(
                "CreateWindowExW failed with Win32 error {}",
                crate::raw::kernel32::GetLastError()
            ));
        }
        Ok(hwnd)
    }

    pub(crate) unsafe fn destroy_hidden_window_handle(hwnd: crate::raw::types::HWND) -> Result<(), String> {
        if hwnd.is_null() {
            return Ok(());
        }
        let ok = crate::raw::user32::DestroyWindow(hwnd);
        if ok == 0 {
            return Err(format!(
                "DestroyWindow failed with Win32 error {}",
                crate::raw::kernel32::GetLastError()
            ));
        }
        Ok(())
    }

    pub(crate) unsafe fn pump_pending_messages() -> i64 {
        let mut message = std::mem::zeroed::<crate::shackle::MSG>();
        let mut count = 0i64;
        while crate::raw::user32::PeekMessageW(
            &mut message as *mut _ as _,
            std::ptr::null_mut(),
            0,
            0,
            crate::raw::constants::PM_REMOVE,
        ) != 0
        {
            crate::raw::user32::TranslateMessage(&message as *const _ as _);
            crate::raw::user32::DispatchMessageW(&message as *const _ as _);
            count += 1;
        }
        count
    }

shackle fn strings_utf16_units_impl(read text: Str) -> Int = helpers.strings.utf16_units:
    Ok(binding_int(text.encode_utf16().count() as i64))

shackle fn strings_utf16_units_with_nul_impl(read text: Str) -> Int = helpers.strings.utf16_units_with_nul:
    Ok(binding_int(text.encode_utf16().count() as i64 + 1))

shackle fn errors_last_error_impl() -> arcana_winapi.raw.types.DWORD = helpers.errors.last_error:
    Ok(binding_layout(unsafe { crate::raw::kernel32::GetLastError() }))

shackle fn errors_hresult_succeeded_impl(read code: arcana_winapi.raw.types.HRESULT) -> Bool = helpers.errors.hresult_succeeded:
    Ok(binding_bool(hresult_succeeded_native(code)))

shackle fn errors_hresult_failed_impl(read code: arcana_winapi.raw.types.HRESULT) -> Bool = helpers.errors.hresult_failed:
    Ok(binding_bool(hresult_failed_native(code)))

shackle fn com_initialize_multithreaded_impl() -> arcana_winapi.raw.types.HRESULT = helpers.com.initialize_multithreaded:
    Ok(binding_layout(unsafe {
        crate::raw::ole32::CoInitializeEx(std::ptr::null_mut(), crate::raw::constants::COINIT_MULTITHREADED)
    }))

shackle fn com_initialize_apartment_threaded_impl() -> arcana_winapi.raw.types.HRESULT = helpers.com.initialize_apartment_threaded:
    Ok(binding_layout(unsafe {
        crate::raw::ole32::CoInitializeEx(std::ptr::null_mut(), crate::raw::constants::COINIT_APARTMENTTHREADED)
    }))

shackle fn com_uninitialize_impl() = helpers.com.uninitialize:
    unsafe {
        crate::raw::ole32::CoUninitialize();
    }
    Ok(binding_unit())

shackle fn com_guid_to_text_impl(read guid: arcana_winapi.raw.types.GUID) -> Str = helpers.com.guid_to_text:
    let mut buffer = [0u16; 39];
    let len = unsafe { crate::raw::combase::StringFromGUID2(&guid, buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 1 {
        return Err("StringFromGUID2 failed".to_string());
    }
    Ok(binding_owned_str(String::from_utf16_lossy(&buffer[..(len as usize - 1)])))

shackle fn com_make_property_key_impl(read fmtid: arcana_winapi.raw.types.GUID, read pid: arcana_winapi.raw.types.DWORD) -> arcana_winapi.raw.types.PROPERTYKEY = helpers.com.make_property_key:
    Ok(binding_layout(crate::raw::types::PROPERTYKEY {
        fmtid,
        pid,
    }))

shackle fn com_property_key_pid_impl(read key: arcana_winapi.raw.types.PROPERTYKEY) -> arcana_winapi.raw.types.DWORD = helpers.com.property_key_pid:
    Ok(binding_layout(key.pid))

shackle fn windowing_hidden_window_roundtrip_impl(read code: Int) -> Int = helpers.windowing.hidden_window_roundtrip:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let result = (|| -> Result<i64, String> {
        let ok = unsafe {
            crate::raw::user32::PostMessageW(
                hwnd,
                crate::raw::constants::WM_APP + 1,
                code as usize,
                0,
            )
        };
        if ok == 0 {
            return Err(format!(
                "PostMessageW failed with Win32 error {}",
                unsafe { crate::raw::kernel32::GetLastError() }
            ));
        }
        let _ = unsafe { pump_pending_messages() };
        let value = instance.package_state.last_callback_code;
        instance.package_state.last_callback_code = 0;
        Ok(value)
    })();
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_int(result?))

shackle fn windowing_hidden_window_dpi_impl() -> arcana_winapi.raw.types.UINT = helpers.windowing.hidden_window_dpi:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let dpi = unsafe { crate::raw::user32::GetDpiForWindow(hwnd) };
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    if dpi == 0 {
        return Err(format!(
            "GetDpiForWindow failed with Win32 error {}",
            unsafe { crate::raw::kernel32::GetLastError() }
        ));
    }
    Ok(binding_layout(dpi))

shackle fn windowing_hidden_window_monitor_dpi_impl() -> arcana_winapi.raw.types.UINT = helpers.windowing.hidden_window_monitor_dpi:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let result = (|| -> Result<crate::raw::types::UINT, String> {
        let monitor = unsafe {
            crate::raw::user32::MonitorFromWindow(hwnd, crate::raw::constants::MONITOR_DEFAULTTONEAREST)
        };
        if monitor.is_null() {
            return Err("MonitorFromWindow returned null".to_string());
        }
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        let hr = unsafe {
            crate::raw::shcore::GetDpiForMonitor(
                monitor,
                crate::raw::constants::MDT_EFFECTIVE_DPI,
                &mut dpi_x,
                &mut dpi_y,
            )
        };
        if hresult_failed_native(hr) {
            return Err(format!("GetDpiForMonitor failed with HRESULT {hr}"));
        }
        Ok(dpi_x.max(dpi_y))
    })();
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_layout(result?))

shackle fn windowing_hidden_window_dark_mode_roundtrip_impl() -> Bool = helpers.windowing.hidden_window_dark_mode_roundtrip:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let enabled = (|| -> bool {
        let request = 1i32;
        let set_hr = unsafe {
            crate::raw::dwmapi::DwmSetWindowAttribute(
                hwnd,
                crate::raw::constants::DWMWA_USE_IMMERSIVE_DARK_MODE,
                &request as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<crate::raw::types::BOOL>() as u32,
            )
        };
        if hresult_failed_native(set_hr) {
            return false;
        }
        let mut actual = 0i32;
        let get_hr = unsafe {
            crate::raw::dwmapi::DwmGetWindowAttribute(
                hwnd,
                crate::raw::constants::DWMWA_USE_IMMERSIVE_DARK_MODE,
                &mut actual as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<crate::raw::types::BOOL>() as u32,
            )
        };
        hresult_succeeded_native(get_hr) && actual != 0
    })();
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_bool(enabled))

shackle fn windowing_hidden_window_client_rect_impl() -> arcana_winapi.raw.types.RECT = helpers.windowing.hidden_window_client_rect:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let mut rect = unsafe { std::mem::zeroed::<crate::raw::types::RECT>() };
    let ok = unsafe { crate::raw::user32::GetClientRect(hwnd, &mut rect) };
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    if ok == 0 {
        return Err(format!(
            "GetClientRect failed with Win32 error {}",
            unsafe { crate::raw::kernel32::GetLastError() }
        ));
    }
    Ok(binding_layout(rect))

shackle fn windowing_hidden_window_frame_rect_impl() -> arcana_winapi.raw.types.RECT = helpers.windowing.hidden_window_frame_rect:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let mut rect = unsafe { std::mem::zeroed::<crate::raw::types::RECT>() };
    let hr = unsafe {
        crate::raw::dwmapi::DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<crate::raw::types::RECT>() as u32,
        )
    };
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    if hresult_failed_native(hr) {
        return Err(format!("DwmGetWindowAttribute failed with HRESULT {hr}"));
    }
    Ok(binding_layout(rect))

shackle fn windowing_clipboard_open_roundtrip_impl() -> Bool = helpers.windowing.clipboard_open_roundtrip:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let opened = unsafe { crate::raw::user32::OpenClipboard(hwnd) != 0 };
    if opened {
        unsafe {
            crate::raw::user32::CloseClipboard();
        }
    }
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_bool(opened))

shackle fn windowing_enable_file_drop_roundtrip_impl() -> Bool = helpers.windowing.enable_file_drop_roundtrip:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    unsafe {
        crate::raw::user32::DragAcceptFiles(hwnd, 1);
        crate::raw::user32::DragAcceptFiles(hwnd, 0);
    }
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_bool(true))

shackle fn windowing_ime_composition_bytes_impl() -> Int = helpers.windowing.ime_composition_bytes:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let bytes = unsafe {
        let context = crate::raw::imm32::ImmGetContext(hwnd);
        if context.is_null() {
            0i64
        } else {
            let size = crate::raw::imm32::ImmGetCompositionStringW(
                context,
                GCS_COMPSTR,
                std::ptr::null_mut(),
                0,
            );
            crate::raw::imm32::ImmReleaseContext(hwnd, context);
            if size < 0 { 0 } else { size as i64 }
        }
    };
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_int(bytes))

shackle fn graphics_gdi_memory_surface_stride_impl(read width: Int, read height: Int) -> Int = helpers.graphics.gdi_memory_surface_stride:
    if width <= 0 || height <= 0 {
        return Err("surface dimensions must be > 0".to_string());
    }
    Ok(binding_int(width * 4))

shackle fn graphics_gdi_hidden_window_present_impl() -> Bool = helpers.graphics.gdi_hidden_window_present:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let presented = (|| -> Result<bool, String> {
        let device = unsafe { crate::raw::user32::GetDC(hwnd) };
        if device.is_null() {
            return Err(format!(
                "GetDC failed with Win32 error {}",
                unsafe { crate::raw::kernel32::GetLastError() }
            ));
        }
        let memory = unsafe { crate::raw::gdi32::CreateCompatibleDC(device) };
        if memory.is_null() {
            unsafe {
                crate::raw::user32::ReleaseDC(hwnd, device);
            }
            return Err(format!(
                "CreateCompatibleDC failed with Win32 error {}",
                unsafe { crate::raw::kernel32::GetLastError() }
            ));
        }
        let mut info = crate::raw::types::BITMAPINFO {
            bmiHeader: crate::raw::types::BITMAPINFOHEADER {
                biSize: std::mem::size_of::<crate::raw::types::BITMAPINFOHEADER>() as u32,
                biWidth: 8,
                biHeight: -8,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: crate::raw::constants::BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [crate::raw::types::RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let bitmap = unsafe {
            crate::raw::gdi32::CreateDIBSection(
                memory,
                &mut info,
                crate::raw::constants::DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            )
        };
        if bitmap.is_null() || bits.is_null() {
            unsafe {
                crate::raw::gdi32::DeleteDC(memory);
                crate::raw::user32::ReleaseDC(hwnd, device);
            }
            return Ok(false);
        }
        unsafe {
            *(bits as *mut u32) = 0x00ff8040;
        }
        let previous = unsafe { crate::raw::gdi32::SelectObject(memory, bitmap as *mut std::ffi::c_void) };
        let blitted = unsafe {
            crate::raw::gdi32::StretchDIBits(
                device,
                0,
                0,
                8,
                8,
                0,
                0,
                8,
                8,
                bits,
                &info,
                crate::raw::constants::DIB_RGB_COLORS,
                crate::raw::constants::SRCCOPY,
            )
        } > 0;
        unsafe {
            if !previous.is_null() {
                crate::raw::gdi32::SelectObject(memory, previous);
            }
            crate::raw::gdi32::DeleteObject(bitmap as *mut std::ffi::c_void);
            crate::raw::gdi32::DeleteDC(memory);
            crate::raw::user32::ReleaseDC(hwnd, device);
        }
        Ok(blitted)
    })();
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_bool(presented.unwrap_or(false)))

shackle fn graphics_dxgi_adapter_count_impl() -> Int = helpers.graphics.dxgi_adapter_count:
    let mut factory: crate::raw::types::LPVOID = std::ptr::null_mut();
    let hr = unsafe { crate::raw::dxgi::CreateDXGIFactory2(0, &iid_idxgi_factory4(), &mut factory) };
    if hresult_failed_native(hr) || factory.is_null() {
        return Ok(binding_int(0));
    }
    let mut count = 0i64;
    unsafe {
        let vtbl = *(factory as *mut *const crate::raw::types::IDXGIFactory4VTable);
        loop {
            let mut adapter: crate::raw::types::IDXGIAdapter1 = std::ptr::null_mut();
            let hr = ((*vtbl).EnumAdapters1)(factory, count as u32, &mut adapter as *mut _ as *mut _);
            if hresult_failed_native(hr) || adapter.is_null() {
                break;
            }
            com_release(adapter);
            count += 1;
        }
        com_release(factory);
    }
    Ok(binding_int(count))

shackle fn graphics_bootstrap_d3d12_warp_impl() -> Bool = helpers.graphics.bootstrap_d3d12_warp:
    let ok = unsafe {
        let mut factory: crate::raw::types::LPVOID = std::ptr::null_mut();
        let hr = crate::raw::dxgi::CreateDXGIFactory2(0, &iid_idxgi_factory4(), &mut factory);
        if hresult_failed_native(hr) || factory.is_null() {
            false
        } else {
            let mut warp: crate::raw::types::LPVOID = std::ptr::null_mut();
            let factory_vtbl = *(factory as *mut *const crate::raw::types::IDXGIFactory4VTable);
            let enum_hr = ((*factory_vtbl).EnumWarpAdapter)(factory, &iid_idxgi_adapter1(), &mut warp);
            if hresult_failed_native(enum_hr) || warp.is_null() {
                com_release(factory);
                false
            } else {
                let mut device: crate::raw::types::LPVOID = std::ptr::null_mut();
                let device_hr = crate::raw::d3d12::D3D12CreateDevice(
                    warp,
                    D3D_FEATURE_LEVEL_11_0,
                    &iid_id3d12_device(),
                    &mut device,
                );
                if hresult_failed_native(device_hr) || device.is_null() {
                    com_release(warp);
                    com_release(factory);
                    false
                } else {
                    let device_vtbl = *(device as *mut *const crate::raw::types::ID3D12DeviceVTable);
                    let mut queue: crate::raw::types::LPVOID = std::ptr::null_mut();
                    let mut allocator: crate::raw::types::LPVOID = std::ptr::null_mut();
                    let mut list: crate::raw::types::LPVOID = std::ptr::null_mut();
                    let mut fence: crate::raw::types::LPVOID = std::ptr::null_mut();
                    let desc = crate::raw::types::D3D12_COMMAND_QUEUE_DESC {
                        Type: crate::raw::constants::D3D12_COMMAND_LIST_TYPE_DIRECT,
                        Priority: crate::raw::constants::D3D12_COMMAND_QUEUE_PRIORITY_NORMAL,
                        Flags: crate::raw::constants::D3D12_COMMAND_QUEUE_FLAGS_NONE,
                        NodeMask: 0,
                    };
                    let queue_hr = ((*device_vtbl).CreateCommandQueue)(device, &desc, &iid_id3d12_command_queue(), &mut queue);
                    let allocator_hr = ((*device_vtbl).CreateCommandAllocator)(device, crate::raw::constants::D3D12_COMMAND_LIST_TYPE_DIRECT, &iid_id3d12_command_allocator(), &mut allocator);
                    let list_hr = ((*device_vtbl).CreateCommandList)(device, 0, crate::raw::constants::D3D12_COMMAND_LIST_TYPE_DIRECT, allocator, std::ptr::null_mut(), &iid_id3d12_graphics_command_list(), &mut list);
                    let fence_hr = ((*device_vtbl).CreateFence)(device, 0, crate::raw::constants::D3D12_FENCE_FLAGS_NONE, &iid_id3d12_fence(), &mut fence);
                    let succeeded = ((*device_vtbl).GetNodeCount)(device) > 0
                        && hresult_succeeded_native(queue_hr)
                        && hresult_succeeded_native(allocator_hr)
                        && hresult_succeeded_native(list_hr)
                        && hresult_succeeded_native(fence_hr);
                    com_release(fence);
                    com_release(list);
                    com_release(allocator);
                    com_release(queue);
                    com_release(device);
                    com_release(warp);
                    com_release(factory);
                    succeeded
                }
            }
        }
    };
    Ok(binding_bool(ok))

shackle fn graphics_bootstrap_dxgi_hidden_window_swapchain_impl() -> Bool = helpers.graphics.bootstrap_dxgi_hidden_window_swapchain:
    let hwnd = unsafe { create_hidden_window_handle(instance as *mut BindingInstance)? };
    let ok = unsafe {
        let mut factory: crate::raw::types::LPVOID = std::ptr::null_mut();
        let factory_hr = crate::raw::dxgi::CreateDXGIFactory2(0, &iid_idxgi_factory4(), &mut factory);
        if hresult_failed_native(factory_hr) || factory.is_null() {
            false
        } else {
            let mut warp: crate::raw::types::LPVOID = std::ptr::null_mut();
            let factory_vtbl = *(factory as *mut *const crate::raw::types::IDXGIFactory4VTable);
            let enum_hr = ((*factory_vtbl).EnumWarpAdapter)(factory, &iid_idxgi_adapter1(), &mut warp);
            if hresult_failed_native(enum_hr) || warp.is_null() {
                com_release(factory);
                false
            } else {
                let mut device: crate::raw::types::LPVOID = std::ptr::null_mut();
                let device_hr = crate::raw::d3d12::D3D12CreateDevice(
                    warp,
                    D3D_FEATURE_LEVEL_11_0,
                    &iid_id3d12_device(),
                    &mut device,
                );
                if hresult_failed_native(device_hr) || device.is_null() {
                    com_release(warp);
                    com_release(factory);
                    false
                } else {
                    let device_vtbl = *(device as *mut *const crate::raw::types::ID3D12DeviceVTable);
                    let mut queue: crate::raw::types::LPVOID = std::ptr::null_mut();
                    let desc = crate::raw::types::D3D12_COMMAND_QUEUE_DESC {
                        Type: crate::raw::constants::D3D12_COMMAND_LIST_TYPE_DIRECT,
                        Priority: crate::raw::constants::D3D12_COMMAND_QUEUE_PRIORITY_NORMAL,
                        Flags: crate::raw::constants::D3D12_COMMAND_QUEUE_FLAGS_NONE,
                        NodeMask: 0,
                    };
                    let queue_hr = ((*device_vtbl).CreateCommandQueue)(
                        device,
                        &desc,
                        &iid_id3d12_command_queue(),
                        &mut queue,
                    );
                    if hresult_failed_native(queue_hr) || queue.is_null() {
                        com_release(queue);
                        com_release(device);
                        com_release(warp);
                        com_release(factory);
                        false
                    } else {
                        let mut swapchain: crate::raw::types::LPVOID = std::ptr::null_mut();
                        let swapchain_desc = crate::raw::types::DXGI_SWAP_CHAIN_DESC1 {
                            Width: 64,
                            Height: 64,
                            Format: crate::raw::constants::DXGI_FORMAT_B8G8R8A8_UNORM,
                            Stereo: 0,
                            SampleDesc: crate::raw::types::DXGI_SAMPLE_DESC {
                                Count: 1,
                                Quality: 0,
                            },
                            BufferUsage: crate::raw::constants::DXGI_USAGE_RENDER_TARGET_OUTPUT,
                            BufferCount: 2,
                            Scaling: crate::raw::constants::DXGI_SCALING_STRETCH,
                            SwapEffect: crate::raw::constants::DXGI_SWAP_EFFECT_FLIP_DISCARD,
                            AlphaMode: crate::raw::constants::DXGI_ALPHA_MODE_IGNORE,
                            Flags: 0,
                        };
                        let swapchain_hr = ((*factory_vtbl).CreateSwapChainForHwnd)(
                            factory,
                            queue,
                            hwnd,
                            &swapchain_desc,
                            std::ptr::null(),
                            std::ptr::null_mut(),
                            &mut swapchain,
                        );
                        let ok = hresult_succeeded_native(swapchain_hr) && !swapchain.is_null();
                        com_release(swapchain);
                        com_release(queue);
                        com_release(device);
                        com_release(warp);
                        com_release(factory);
                        ok
                    }
                }
            }
        }
    };
    let _ = unsafe { destroy_hidden_window_handle(hwnd) };
    Ok(binding_bool(ok))

shackle fn graphics_bootstrap_d2d_factory_impl() -> Bool = helpers.graphics.bootstrap_d2d_factory:
    let mut factory: crate::raw::types::LPVOID = std::ptr::null_mut();
    let options = crate::raw::types::D2D1_FACTORY_OPTIONS { debugLevel: 0 };
    let hr = unsafe {
        crate::raw::d2d1::D2D1CreateFactory(
            crate::raw::constants::D2D1_FACTORY_TYPE_MULTI_THREADED,
            &iid_id2d1_factory1(),
            &options,
            &mut factory,
        )
    };
    let ok = hresult_succeeded_native(hr) && !factory.is_null();
    unsafe {
        com_release(factory);
    }
    Ok(binding_bool(ok))

shackle fn graphics_bootstrap_wic_factory_impl() -> Bool = helpers.graphics.bootstrap_wic_factory:
    let entered = enter_com(crate::raw::constants::COINIT_MULTITHREADED).unwrap_or(false);
    let mut factory: crate::raw::types::LPVOID = std::ptr::null_mut();
    let hr = unsafe {
        crate::raw::ole32::CoCreateInstance(
            &clsid_wic_imaging_factory2(),
            std::ptr::null_mut(),
            crate::raw::constants::CLSCTX_INPROC_SERVER,
            &iid_iwic_imaging_factory(),
            &mut factory,
        )
    };
    let ok = hresult_succeeded_native(hr) && !factory.is_null();
    unsafe {
        com_release(factory);
    }
    leave_com(entered);
    Ok(binding_bool(ok))

shackle fn text_directwrite_system_font_count_impl() -> Int = helpers.text.directwrite_system_font_count:
    let mut factory: crate::raw::types::LPVOID = std::ptr::null_mut();
    let hr = unsafe {
        crate::raw::dwrite::DWriteCreateFactory(
            crate::raw::constants::DWRITE_FACTORY_TYPE_SHARED,
            &iid_idwrite_factory(),
            &mut factory,
        )
    };
    if hresult_failed_native(hr) || factory.is_null() {
        return Ok(binding_int(0));
    }
    let count = unsafe {
        let factory_vtbl = *(factory as *mut *const crate::raw::types::IDWriteFactoryVTable);
        let mut collection: crate::raw::types::LPVOID = std::ptr::null_mut();
        let get_hr = ((*factory_vtbl).GetSystemFontCollection)(factory, &mut collection, 0);
        if hresult_failed_native(get_hr) || collection.is_null() {
            0i64
        } else {
            let collection_vtbl = *(collection as *mut *const crate::raw::types::IDWriteFontCollectionVTable);
            let count = ((*collection_vtbl).GetFontFamilyCount)(collection) as i64;
            com_release(collection);
            count
        }
    };
    unsafe {
        com_release(factory);
    }
    Ok(binding_int(count))

shackle fn text_bootstrap_text_layout_impl() -> Bool = helpers.text.bootstrap_text_layout:
    let mut factory: crate::raw::types::LPVOID = std::ptr::null_mut();
    let hr = unsafe {
        crate::raw::dwrite::DWriteCreateFactory(
            crate::raw::constants::DWRITE_FACTORY_TYPE_SHARED,
            &iid_idwrite_factory(),
            &mut factory,
        )
    };
    if hresult_failed_native(hr) || factory.is_null() {
        return Ok(binding_bool(false));
    }
    let ok = unsafe {
        let factory_vtbl = *(factory as *mut *const crate::raw::types::IDWriteFactoryVTable);
        let family = wide_nul("Segoe UI");
        let locale = wide_nul("en-us");
        let text = wide_nul("Arcana");
        let mut format: crate::raw::types::LPVOID = std::ptr::null_mut();
        let mut layout: crate::raw::types::LPVOID = std::ptr::null_mut();
        let format_hr = ((*factory_vtbl).CreateTextFormat)(
            factory,
            family.as_ptr(),
            std::ptr::null_mut(),
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            16.0,
            locale.as_ptr(),
            &mut format,
        );
        let layout_hr = if hresult_succeeded_native(format_hr) && !format.is_null() {
            ((*factory_vtbl).CreateTextLayout)(
                factory,
                text.as_ptr(),
                (text.len() - 1) as u32,
                format,
                240.0,
                80.0,
                &mut layout,
            )
        } else {
            -1
        };
        let ok = hresult_succeeded_native(format_hr)
            && hresult_succeeded_native(layout_hr)
            && !layout.is_null();
        com_release(layout);
        com_release(format);
        com_release(factory);
        ok
    };
    Ok(binding_bool(ok))

shackle fn audio_render_device_count_impl() -> Int = helpers.audio.render_device_count:
    let entered = enter_com(crate::raw::constants::COINIT_MULTITHREADED).unwrap_or(false);
    let count = unsafe {
        let mut enumerator: crate::raw::types::LPVOID = std::ptr::null_mut();
        let hr = crate::raw::ole32::CoCreateInstance(
            &clsid_mmdevice_enumerator(),
            std::ptr::null_mut(),
            crate::raw::constants::CLSCTX_INPROC_SERVER,
            &iid_immdevice_enumerator(),
            &mut enumerator,
        );
        if hresult_failed_native(hr) || enumerator.is_null() {
            0i64
        } else {
            let enumerator_vtbl = *(enumerator as *mut *const crate::raw::types::IMMDeviceEnumeratorVTable);
            let mut collection: crate::raw::types::LPVOID = std::ptr::null_mut();
            let enum_hr = ((*enumerator_vtbl).EnumAudioEndpoints)(
                enumerator,
                crate::raw::constants::EDATAFLOW_RENDER,
                crate::raw::mmdeviceapi::DEVICE_STATE_ACTIVE,
                &mut collection,
            );
            let count = if hresult_failed_native(enum_hr) || collection.is_null() {
                0
            } else {
                let collection_vtbl = *(collection as *mut *const crate::raw::types::IMMDeviceCollectionVTable);
                let mut count = 0u32;
                let _ = ((*collection_vtbl).GetCount)(collection, &mut count);
                com_release(collection);
                count as i64
            };
            com_release(enumerator);
            count
        }
    };
    leave_com(entered);
    Ok(binding_int(count))

shackle fn audio_bootstrap_wasapi_default_render_impl() -> Bool = helpers.audio.bootstrap_wasapi_default_render:
    let entered = enter_com(crate::raw::constants::COINIT_MULTITHREADED).unwrap_or(false);
    let ok = unsafe {
        let mut enumerator: crate::raw::types::LPVOID = std::ptr::null_mut();
        let create_hr = crate::raw::ole32::CoCreateInstance(
            &clsid_mmdevice_enumerator(),
            std::ptr::null_mut(),
            crate::raw::constants::CLSCTX_INPROC_SERVER,
            &iid_immdevice_enumerator(),
            &mut enumerator,
        );
        if hresult_failed_native(create_hr) || enumerator.is_null() {
            false
        } else {
            let enumerator_vtbl = *(enumerator as *mut *const crate::raw::types::IMMDeviceEnumeratorVTable);
            let mut device: crate::raw::types::LPVOID = std::ptr::null_mut();
            let endpoint_hr = ((*enumerator_vtbl).GetDefaultAudioEndpoint)(
                enumerator,
                crate::raw::constants::EDATAFLOW_RENDER,
                crate::raw::constants::EROLE_CONSOLE,
                &mut device,
            );
            if hresult_failed_native(endpoint_hr) || device.is_null() {
                com_release(enumerator);
                false
            } else {
                let device_vtbl = *(device as *mut *const crate::raw::types::IMMDeviceVTable);
                let mut client: crate::raw::types::LPVOID = std::ptr::null_mut();
                let activate_hr = ((*device_vtbl).Activate)(
                    device,
                    &iid_iaudio_client(),
                    crate::raw::constants::CLSCTX_INPROC_SERVER,
                    std::ptr::null_mut(),
                    &mut client,
                );
                if hresult_failed_native(activate_hr) || client.is_null() {
                    com_release(device);
                    com_release(enumerator);
                    false
                } else {
                    let client_vtbl = *(client as *mut *const crate::raw::types::IAudioClientVTable);
                    let mut mix_format: *mut crate::raw::types::WAVEFORMATEX = std::ptr::null_mut();
                    let mix_hr = ((*client_vtbl).GetMixFormat)(client, &mut mix_format);
                    let supported = if hresult_succeeded_native(mix_hr) && !mix_format.is_null() {
                        let support_hr = ((*client_vtbl).IsFormatSupported)(
                            client,
                            crate::raw::constants::AUDCLNT_SHAREMODE_SHARED,
                            mix_format,
                            std::ptr::null_mut(),
                        );
                        let init_hr = if hresult_succeeded_native(support_hr) {
                            ((*client_vtbl).Initialize)(
                                client,
                                crate::raw::constants::AUDCLNT_SHAREMODE_SHARED,
                                0,
                                0,
                                0,
                                mix_format,
                                std::ptr::null(),
                            )
                        } else {
                            -1
                        };
                        crate::raw::ole32::CoTaskMemFree(mix_format as *mut std::ffi::c_void);
                        hresult_succeeded_native(support_hr) && hresult_succeeded_native(init_hr)
                    } else {
                        false
                    };
                    com_release(client);
                    com_release(device);
                    com_release(enumerator);
                    supported
                }
            }
        }
    };
    leave_com(entered);
    Ok(binding_bool(ok))

shackle fn audio_bootstrap_wasapi_render_client_impl() -> Bool = helpers.audio.bootstrap_wasapi_render_client:
    let entered = enter_com(crate::raw::constants::COINIT_MULTITHREADED).unwrap_or(false);
    let ok = unsafe {
        let mut enumerator: crate::raw::types::LPVOID = std::ptr::null_mut();
        let create_hr = crate::raw::ole32::CoCreateInstance(
            &clsid_mmdevice_enumerator(),
            std::ptr::null_mut(),
            crate::raw::constants::CLSCTX_INPROC_SERVER,
            &iid_immdevice_enumerator(),
            &mut enumerator,
        );
        if hresult_failed_native(create_hr) || enumerator.is_null() {
            false
        } else {
            let enumerator_vtbl = *(enumerator as *mut *const crate::raw::types::IMMDeviceEnumeratorVTable);
            let mut device: crate::raw::types::LPVOID = std::ptr::null_mut();
            let endpoint_hr = ((*enumerator_vtbl).GetDefaultAudioEndpoint)(
                enumerator,
                crate::raw::constants::EDATAFLOW_RENDER,
                crate::raw::constants::EROLE_CONSOLE,
                &mut device,
            );
            if hresult_failed_native(endpoint_hr) || device.is_null() {
                com_release(enumerator);
                false
            } else {
                let device_vtbl = *(device as *mut *const crate::raw::types::IMMDeviceVTable);
                let mut client: crate::raw::types::LPVOID = std::ptr::null_mut();
                let activate_hr = ((*device_vtbl).Activate)(
                    device,
                    &iid_iaudio_client(),
                    crate::raw::constants::CLSCTX_INPROC_SERVER,
                    std::ptr::null_mut(),
                    &mut client,
                );
                if hresult_failed_native(activate_hr) || client.is_null() {
                    com_release(device);
                    com_release(enumerator);
                    false
                } else {
                    let client_vtbl = *(client as *mut *const crate::raw::types::IAudioClientVTable);
                    let mut mix_format: *mut crate::raw::types::WAVEFORMATEX = std::ptr::null_mut();
                    let mix_hr = ((*client_vtbl).GetMixFormat)(client, &mut mix_format);
                    let ok = if hresult_succeeded_native(mix_hr) && !mix_format.is_null() {
                        let init_hr = ((*client_vtbl).Initialize)(
                            client,
                            crate::raw::constants::AUDCLNT_SHAREMODE_SHARED,
                            crate::raw::audioclient::AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
                            0,
                            0,
                            mix_format,
                            std::ptr::null(),
                        );
                        let mut render_client: crate::raw::types::LPVOID = std::ptr::null_mut();
                        let service_hr = if hresult_succeeded_native(init_hr) {
                            ((*client_vtbl).GetService)(
                                client,
                                &iid_iaudio_render_client(),
                                &mut render_client,
                            )
                        } else {
                            -1
                        };
                        crate::raw::ole32::CoTaskMemFree(mix_format as *mut std::ffi::c_void);
                        let ok = hresult_succeeded_native(init_hr)
                            && hresult_succeeded_native(service_hr)
                            && !render_client.is_null();
                        com_release(render_client);
                        ok
                    } else {
                        false
                    };
                    com_release(client);
                    com_release(device);
                    com_release(enumerator);
                    ok
                }
            }
        }
    };
    leave_com(entered);
    Ok(binding_bool(ok))

shackle fn audio_bootstrap_endpoint_volume_impl() -> Bool = helpers.audio.bootstrap_endpoint_volume:
    let entered = enter_com(crate::raw::constants::COINIT_MULTITHREADED).unwrap_or(false);
    let ok = unsafe {
        let mut enumerator: crate::raw::types::LPVOID = std::ptr::null_mut();
        let create_hr = crate::raw::ole32::CoCreateInstance(
            &clsid_mmdevice_enumerator(),
            std::ptr::null_mut(),
            crate::raw::constants::CLSCTX_INPROC_SERVER,
            &iid_immdevice_enumerator(),
            &mut enumerator,
        );
        if hresult_failed_native(create_hr) || enumerator.is_null() {
            false
        } else {
            let enumerator_vtbl = *(enumerator as *mut *const crate::raw::types::IMMDeviceEnumeratorVTable);
            let mut device: crate::raw::types::LPVOID = std::ptr::null_mut();
            let endpoint_hr = ((*enumerator_vtbl).GetDefaultAudioEndpoint)(
                enumerator,
                crate::raw::constants::EDATAFLOW_RENDER,
                crate::raw::constants::EROLE_CONSOLE,
                &mut device,
            );
            if hresult_failed_native(endpoint_hr) || device.is_null() {
                com_release(enumerator);
                false
            } else {
                let device_vtbl = *(device as *mut *const crate::raw::types::IMMDeviceVTable);
                let mut endpoint: crate::raw::types::LPVOID = std::ptr::null_mut();
                let activate_hr = ((*device_vtbl).Activate)(
                    device,
                    &iid_iaudio_endpoint_volume(),
                    crate::raw::constants::CLSCTX_INPROC_SERVER,
                    std::ptr::null_mut(),
                    &mut endpoint,
                );
                if hresult_failed_native(activate_hr) || endpoint.is_null() {
                    com_release(device);
                    com_release(enumerator);
                    false
                } else {
                    let endpoint_vtbl = *(endpoint as *mut *const crate::raw::types::IAudioEndpointVolumeVTable);
                    let mut scalar = 0.0f32;
                    let volume_hr = ((*endpoint_vtbl).GetMasterVolumeLevelScalar)(endpoint, &mut scalar);
                    com_release(endpoint);
                    com_release(device);
                    com_release(enumerator);
                    hresult_succeeded_native(volume_hr)
                }
            }
        }
    };
    leave_com(entered);
    Ok(binding_bool(ok))

shackle fn audio_bootstrap_session_policy_game_effects_impl() -> Bool = helpers.audio.bootstrap_session_policy_game_effects:
    let entered = enter_com(crate::raw::constants::COINIT_MULTITHREADED).unwrap_or(false);
    let ok = unsafe {
        let mut enumerator: crate::raw::types::LPVOID = std::ptr::null_mut();
        let create_hr = crate::raw::ole32::CoCreateInstance(
            &clsid_mmdevice_enumerator(),
            std::ptr::null_mut(),
            crate::raw::constants::CLSCTX_INPROC_SERVER,
            &iid_immdevice_enumerator(),
            &mut enumerator,
        );
        if hresult_failed_native(create_hr) || enumerator.is_null() {
            false
        } else {
            let enumerator_vtbl = *(enumerator as *mut *const crate::raw::types::IMMDeviceEnumeratorVTable);
            let mut device: crate::raw::types::LPVOID = std::ptr::null_mut();
            let endpoint_hr = ((*enumerator_vtbl).GetDefaultAudioEndpoint)(
                enumerator,
                crate::raw::constants::EDATAFLOW_RENDER,
                crate::raw::constants::EROLE_CONSOLE,
                &mut device,
            );
            if hresult_failed_native(endpoint_hr) || device.is_null() {
                com_release(enumerator);
                false
            } else {
                let device_vtbl = *(device as *mut *const crate::raw::types::IMMDeviceVTable);
                let mut client: crate::raw::types::LPVOID = std::ptr::null_mut();
                let activate_hr = ((*device_vtbl).Activate)(
                    device,
                    &iid_iaudio_client2(),
                    crate::raw::constants::CLSCTX_INPROC_SERVER,
                    std::ptr::null_mut(),
                    &mut client,
                );
                if hresult_failed_native(activate_hr) || client.is_null() {
                    com_release(device);
                    com_release(enumerator);
                    false
                } else {
                    let client_vtbl = *(client as *mut *const crate::raw::types::IAudioClient2VTable);
                    let properties = crate::raw::types::AUDIOCLIENT_PROPERTIES {
                        cbSize: std::mem::size_of::<crate::raw::types::AUDIOCLIENT_PROPERTIES>() as u32,
                        bIsOffload: 0,
                        eCategory: crate::raw::audiopolicy::AUDIO_STREAM_CATEGORY_GAME_EFFECTS,
                        Options: 0,
                    };
                    let set_hr = ((*client_vtbl).SetClientProperties)(client, &properties);
                    com_release(client);
                    com_release(device);
                    com_release(enumerator);
                    hresult_succeeded_native(set_hr)
                }
            }
        }
    };
    leave_com(entered);
    Ok(binding_bool(ok))

shackle fn audio_register_pro_audio_thread_impl() -> Bool = helpers.audio.register_pro_audio_thread:
    let task = wide_nul("Pro Audio");
    let mut task_index = 0u32;
    let handle = unsafe { crate::raw::avrt::AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut task_index) };
    if handle.is_null() {
        return Ok(binding_bool(false));
    }
    let reverted = unsafe { crate::raw::avrt::AvRevertMmThreadCharacteristics(handle) != 0 };
    Ok(binding_bool(reverted))

shackle fn audio_bootstrap_xaudio2_impl() -> Bool = helpers.audio.bootstrap_xaudio2:
    let ok = unsafe {
        let mut engine: crate::raw::types::IXAudio2 = std::ptr::null_mut();
        let hr = crate::raw::xaudio2::XAudio2Create(
            &mut engine,
            0,
            crate::raw::constants::XAUDIO2_DEFAULT_PROCESSOR,
        );
        if hresult_failed_native(hr) || engine.is_null() {
            false
        } else {
            let engine_vtbl = *(engine as *mut *const crate::raw::types::IXAudio2VTable);
            let mut voice: crate::raw::types::IXAudio2MasteringVoice = std::ptr::null_mut();
            let start_hr = ((*engine_vtbl).StartEngine)(engine);
            let voice_hr = ((*engine_vtbl).CreateMasteringVoice)(
                engine,
                &mut voice as *mut _ as *mut _,
                crate::raw::constants::XAUDIO2_DEFAULT_CHANNELS,
                crate::raw::constants::XAUDIO2_DEFAULT_SAMPLERATE,
                0,
                std::ptr::null(),
                std::ptr::null(),
                0,
            );
            if !voice.is_null() {
                let voice_vtbl = *(voice as *mut *const crate::raw::types::IXAudio2VoiceVTable);
                ((*voice_vtbl).DestroyVoice)(voice);
            }
            ((*engine_vtbl).StopEngine)(engine);
            com_release(engine);
            hresult_succeeded_native(start_hr) && hresult_succeeded_native(voice_hr)
        }
    };
    Ok(binding_bool(ok))

shackle fn audio_bootstrap_x3daudio_impl() -> Bool = helpers.audio.bootstrap_x3daudio:
    let mut handle = [0u8; 20];
    let hr = crate::raw::x3daudio::X3DAudioInitialize(
        crate::raw::ksmedia::KSAUDIO_SPEAKER_STEREO,
        SPEED_OF_SOUND_METERS_PER_SECOND,
        handle.as_mut_ptr() as *mut std::ffi::c_void,
    );
    Ok(binding_bool(hresult_succeeded_native(hr)))
