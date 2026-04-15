shackle flags WinapiDesktopInternals:
    static DESKTOP_WINDOW_CLASS: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();

    pub(crate) const EVENT_WINDOW_RESIZED: i64 = 1;
    pub(crate) const EVENT_WINDOW_CLOSE_REQUESTED: i64 = 2;
    pub(crate) const EVENT_WINDOW_FOCUSED: i64 = 3;
    pub(crate) const EVENT_KEY_DOWN: i64 = 4;
    pub(crate) const EVENT_KEY_UP: i64 = 5;
    pub(crate) const EVENT_MOUSE_DOWN: i64 = 6;
    pub(crate) const EVENT_MOUSE_UP: i64 = 7;
    pub(crate) const EVENT_MOUSE_MOVE: i64 = 8;
    pub(crate) const EVENT_MOUSE_WHEEL: i64 = 9;
    pub(crate) const EVENT_WINDOW_MOVED: i64 = 10;
    pub(crate) const EVENT_MOUSE_ENTERED: i64 = 11;
    pub(crate) const EVENT_MOUSE_LEFT: i64 = 12;
    pub(crate) const EVENT_WINDOW_REDRAW_REQUESTED: i64 = 13;
    pub(crate) const EVENT_TEXT_INPUT: i64 = 14;
    pub(crate) const EVENT_FILE_DROPPED: i64 = 15;
    pub(crate) const EVENT_WINDOW_SCALE_FACTOR_CHANGED: i64 = 16;
    pub(crate) const EVENT_WINDOW_THEME_CHANGED: i64 = 17;
    pub(crate) const EVENT_RAW_MOUSE_MOTION: i64 = 18;
    pub(crate) const EVENT_RAW_MOUSE_BUTTON: i64 = 19;
    pub(crate) const EVENT_TEXT_COMPOSITION_STARTED: i64 = 24;
    pub(crate) const EVENT_TEXT_COMPOSITION_UPDATED: i64 = 25;
    pub(crate) const EVENT_TEXT_COMPOSITION_COMMITTED: i64 = 26;
    pub(crate) const EVENT_TEXT_COMPOSITION_CANCELLED: i64 = 27;
    pub(crate) const EVENT_RAW_MOUSE_WHEEL: i64 = 28;
    pub(crate) const EVENT_RAW_KEY: i64 = 29;

    pub(crate) const WM_DESTROY_NATIVE: u32 = 2;
    pub(crate) const WM_MOVE_NATIVE: u32 = 3;
    pub(crate) const WM_SIZE_NATIVE: u32 = 5;
    pub(crate) const WM_SETFOCUS_NATIVE: u32 = 7;
    pub(crate) const WM_KILLFOCUS_NATIVE: u32 = 8;
    pub(crate) const WM_PAINT_NATIVE: u32 = 15;
    pub(crate) const WM_GETMINMAXINFO_NATIVE: u32 = 36;
    pub(crate) const WM_SETCURSOR_NATIVE: u32 = 32;
    pub(crate) const WM_CHAR_NATIVE: u32 = 258;
    pub(crate) const WM_DROPFILES_NATIVE: u32 = 563;
    pub(crate) const WM_IME_STARTCOMPOSITION_NATIVE: u32 = 269;
    pub(crate) const WM_IME_ENDCOMPOSITION_NATIVE: u32 = 270;
    pub(crate) const WM_IME_COMPOSITION_NATIVE: u32 = 271;
    pub(crate) const WM_THEMECHANGED_NATIVE: u32 = 794;
    pub(crate) const WM_DPICHANGED_NATIVE: u32 = 736;
    pub(crate) const WM_NCDESTROY_NATIVE: u32 = 130;

    pub(crate) const GWL_STYLE_NATIVE: i32 = -16;
    pub(crate) const GWL_EXSTYLE_NATIVE: i32 = -20;
    pub(crate) const WS_OVERLAPPEDWINDOW_NATIVE: u32 = 0x00CF0000;
    pub(crate) const WS_CAPTION_NATIVE: u32 = 0x00C00000;
    pub(crate) const WS_SYSMENU_NATIVE: u32 = 0x00080000;
    pub(crate) const WS_THICKFRAME_NATIVE: u32 = 0x00040000;
    pub(crate) const WS_MINIMIZEBOX_NATIVE: u32 = 0x00020000;
    pub(crate) const WS_MAXIMIZEBOX_NATIVE: u32 = 0x00010000;
    pub(crate) const WS_EX_TOPMOST_NATIVE: u32 = 0x00000008;
    pub(crate) const WS_EX_LAYERED_NATIVE: u32 = 0x00080000;
    pub(crate) const SW_HIDE_NATIVE: i32 = 0;
    pub(crate) const SW_SHOW_NATIVE: i32 = 5;
    pub(crate) const SW_MINIMIZE_NATIVE: i32 = 6;
    pub(crate) const SW_MAXIMIZE_NATIVE: i32 = 3;
    pub(crate) const SW_RESTORE_NATIVE: i32 = 9;
    pub(crate) const SWP_FRAMECHANGED_NATIVE: u32 = 0x0020;
    pub(crate) const SWP_NOSIZE_NATIVE: u32 = 0x0001;
    pub(crate) const SWP_NOMOVE_NATIVE: u32 = 0x0002;
    pub(crate) const SWP_NOACTIVATE_NATIVE: u32 = 0x0010;
    pub(crate) const SWP_NOOWNERZORDER_NATIVE: u32 = 0x0200;
    pub(crate) const LWA_ALPHA_NATIVE: u32 = 0x00000002;
    pub(crate) const FLASHW_STOP_NATIVE: u32 = 0;
    pub(crate) const FLASHW_ALL_NATIVE: u32 = 3;
    pub(crate) const MONITOR_DEFAULTTONEAREST_NATIVE: u32 = 2;
    pub(crate) const IACE_DEFAULT_NATIVE: u32 = 0x0010;
    pub(crate) const IACE_IGNORENOCONTEXT_NATIVE: u32 = 0x0020;
    pub(crate) const CFS_DEFAULT_NATIVE: u32 = 0;
    pub(crate) const CFS_RECT_NATIVE: u32 = 0x0001;
    pub(crate) const CFS_FORCE_POSITION_NATIVE: u32 = 0x0020;
    pub(crate) const GCS_COMPSTR_NATIVE: u32 = 0x0008;
    pub(crate) const GCS_CURSORPOS_NATIVE: u32 = 0x0080;
    pub(crate) const GCS_RESULTSTR_NATIVE: u32 = 0x0800;
    pub(crate) const HWND_TOPMOST_NATIVE: isize = -1;
    pub(crate) const HWND_NOTOPMOST_NATIVE: isize = -2;
    pub(crate) const IDC_ARROW_NATIVE: usize = 32512;
    pub(crate) const IDC_IBEAM_NATIVE: usize = 32513;
    pub(crate) const IDC_WAIT_NATIVE: usize = 32514;
    pub(crate) const IDC_CROSS_NATIVE: usize = 32515;
    pub(crate) const IDC_SIZENWSE_NATIVE: usize = 32642;
    pub(crate) const IDC_SIZENESW_NATIVE: usize = 32643;
    pub(crate) const IDC_SIZEWE_NATIVE: usize = 32644;
    pub(crate) const IDC_SIZENS_NATIVE: usize = 32645;
    pub(crate) const IDC_SIZEALL_NATIVE: usize = 32646;
    pub(crate) const IDC_NO_NATIVE: usize = 32648;
    pub(crate) const IDC_HAND_NATIVE: usize = 32649;
    pub(crate) const IDC_HELP_NATIVE: usize = 32651;

    #[repr(C)]
    pub(crate) struct PAINTSTRUCT {
        pub(crate) hdc: crate::raw::types::HDC,
        pub(crate) fErase: crate::raw::types::BOOL,
        pub(crate) rcPaint: crate::raw::types::RECT,
        pub(crate) fRestore: crate::raw::types::BOOL,
        pub(crate) fIncUpdate: crate::raw::types::BOOL,
        pub(crate) rgbReserved: [u8; 32],
    }

    #[repr(C)]
    pub(crate) struct FLASHWINFO {
        pub(crate) cbSize: u32,
        pub(crate) hwnd: crate::raw::types::HWND,
        pub(crate) dwFlags: u32,
        pub(crate) uCount: u32,
        pub(crate) dwTimeout: u32,
    }

    #[derive(Clone, Debug, Default)]
    pub(crate) struct PendingDesktopEvent {
        pub(crate) kind: i64,
        pub(crate) window_id: i64,
        pub(crate) a: i64,
        pub(crate) b: i64,
        pub(crate) flags: i64,
        pub(crate) text: String,
        pub(crate) key_code: i64,
        pub(crate) physical_key: i64,
        pub(crate) logical_key: i64,
        pub(crate) key_location: i64,
        pub(crate) pointer_x: i64,
        pub(crate) pointer_y: i64,
        pub(crate) repeated: bool,
    }

    pub(crate) struct WinapiWindowState {
        pub(crate) hwnd: crate::raw::types::HWND,
        pub(crate) title: String,
        pub(crate) width: i64,
        pub(crate) height: i64,
        pub(crate) position: (i64, i64),
        pub(crate) min_size: (i64, i64),
        pub(crate) max_size: (i64, i64),
        pub(crate) resized: bool,
        pub(crate) fullscreen: bool,
        pub(crate) minimized: bool,
        pub(crate) maximized: bool,
        pub(crate) focused: bool,
        pub(crate) visible: bool,
        pub(crate) decorated: bool,
        pub(crate) resizable: bool,
        pub(crate) topmost: bool,
        pub(crate) transparent: bool,
        pub(crate) theme_override_code: i64,
        pub(crate) cursor_visible: bool,
        pub(crate) cursor_icon_code: i64,
        pub(crate) cursor_grab_mode: i64,
        pub(crate) cursor_position: (i64, i64),
        pub(crate) text_input_enabled: bool,
        pub(crate) ime_composing: bool,
        pub(crate) composition_area_active: bool,
        pub(crate) composition_area_position: (i64, i64),
        pub(crate) composition_area_size: (i64, i64),
        pub(crate) fullscreen_restore_position: (i64, i64),
        pub(crate) fullscreen_restore_size: (i64, i64),
        pub(crate) fullscreen_restore_maximized: bool,
        pub(crate) closed: bool,
        pub(crate) events: std::collections::VecDeque<PendingDesktopEvent>,
    }

    #[derive(Clone, Debug, Default)]
    pub(crate) struct WinapiFrameInputState {
        pub(crate) key_down: Vec<i64>,
        pub(crate) key_pressed: Vec<i64>,
        pub(crate) key_released: Vec<i64>,
        pub(crate) mouse_pos: (i64, i64),
        pub(crate) mouse_down: Vec<i64>,
        pub(crate) mouse_pressed: Vec<i64>,
        pub(crate) mouse_released: Vec<i64>,
        pub(crate) mouse_wheel_y: i64,
        pub(crate) mouse_in_window: bool,
    }

    pub(crate) struct WinapiFrameState {
        pub(crate) events: std::collections::VecDeque<PendingDesktopEvent>,
        pub(crate) input: WinapiFrameInputState,
        pub(crate) last_polled: Option<PendingDesktopEvent>,
    }

    pub(crate) struct WinapiWakeState {
        pub(crate) event: crate::raw::types::HANDLE,
        pub(crate) pending: usize,
    }

    pub(crate) struct WinapiDesktopState {
        pub(crate) next_window_handle: u64,
        pub(crate) windows: std::collections::BTreeMap<u64, WinapiWindowState>,
        pub(crate) next_frame_handle: u64,
        pub(crate) frames: std::collections::BTreeMap<u64, WinapiFrameState>,
        pub(crate) next_wake_handle: u64,
        pub(crate) wakes: std::collections::BTreeMap<u64, WinapiWakeState>,
    }

    pub(crate) struct DesktopWindowProcState {
        pub(crate) desktop_state_handle: u64,
        pub(crate) handle: u64,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct NativeMonitorInfo {
        pub(crate) handle: crate::raw::types::HMONITOR,
        pub(crate) name: String,
        pub(crate) position: (i64, i64),
        pub(crate) size: (i64, i64),
        pub(crate) scale_factor_milli: i64,
        pub(crate) primary: bool,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        pub(crate) fn ShowWindow(hwnd: crate::raw::types::HWND, cmd: i32) -> i32;
        pub(crate) fn SetWindowPos(
            hwnd: crate::raw::types::HWND,
            insert_after: crate::raw::types::HWND,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        pub(crate) fn SetWindowTextW(hwnd: crate::raw::types::HWND, text: crate::raw::types::LPCWSTR) -> i32;
        pub(crate) fn IsWindow(hwnd: crate::raw::types::HWND) -> i32;
        pub(crate) fn GetWindowRect(hwnd: crate::raw::types::HWND, rect: *mut crate::raw::types::RECT) -> i32;
        pub(crate) fn InvalidateRect(
            hwnd: crate::raw::types::HWND,
            rect: *const crate::raw::types::RECT,
            erase: i32,
        ) -> i32;
        pub(crate) fn BeginPaint(hwnd: crate::raw::types::HWND, paint: *mut PAINTSTRUCT) -> crate::raw::types::HDC;
        pub(crate) fn EndPaint(hwnd: crate::raw::types::HWND, paint: *const PAINTSTRUCT) -> i32;
        pub(crate) fn EnumDisplayMonitors(
            hdc: crate::raw::types::HDC,
            rect: *const crate::raw::types::RECT,
            callback: Option<
                unsafe extern "system" fn(
                    crate::raw::types::HMONITOR,
                    crate::raw::types::HDC,
                    *mut crate::raw::types::RECT,
                    crate::raw::types::LPARAM,
                ) -> i32,
            >,
            data: crate::raw::types::LPARAM,
        ) -> i32;
        pub(crate) fn GetMonitorInfoW(
            monitor: crate::raw::types::HMONITOR,
            info: *mut crate::raw::types::MONITORINFOEXW,
        ) -> i32;
        pub(crate) fn SetLayeredWindowAttributes(
            hwnd: crate::raw::types::HWND,
            key: u32,
            alpha: u8,
            flags: u32,
        ) -> i32;
        pub(crate) fn FlashWindowEx(info: *mut FLASHWINFO) -> i32;
        pub(crate) fn LoadCursorW(
            instance: crate::raw::types::HMODULE,
            name: crate::raw::types::LPCWSTR,
        ) -> crate::raw::types::HCURSOR;
        pub(crate) fn SetCursor(cursor: crate::raw::types::HCURSOR) -> crate::raw::types::HCURSOR;
        pub(crate) fn ClipCursor(rect: *const crate::raw::types::RECT) -> i32;
        pub(crate) fn SetCursorPos(x: i32, y: i32) -> i32;
        pub(crate) fn ClientToScreen(hwnd: crate::raw::types::HWND, point: *mut crate::raw::types::POINT) -> i32;
        pub(crate) fn ScreenToClient(hwnd: crate::raw::types::HWND, point: *mut crate::raw::types::POINT) -> i32;
    }

    pub(crate) fn frame_style_mask() -> u32 {
        WS_CAPTION_NATIVE
            | WS_SYSMENU_NATIVE
            | WS_THICKFRAME_NATIVE
            | WS_MINIMIZEBOX_NATIVE
            | WS_MAXIMIZEBOX_NATIVE
    }

    pub(crate) fn frame_ex_style_mask() -> u32 {
        WS_EX_TOPMOST_NATIVE | WS_EX_LAYERED_NATIVE
    }

    pub(crate) fn desired_frame_style(
        fullscreen: bool,
        decorated: bool,
        resizable: bool,
    ) -> u32 {
        if fullscreen {
            return 0;
        }
        let mut style = 0u32;
        if decorated {
            style |= WS_CAPTION_NATIVE | WS_SYSMENU_NATIVE | WS_MINIMIZEBOX_NATIVE;
        }
        if resizable {
            style |= WS_THICKFRAME_NATIVE | WS_MAXIMIZEBOX_NATIVE;
        }
        style
    }

    pub(crate) fn desired_frame_ex_style(topmost: bool, transparent: bool) -> u32 {
        let mut ex_style = 0u32;
        if topmost {
            ex_style |= WS_EX_TOPMOST_NATIVE;
        }
        if transparent {
            ex_style |= WS_EX_LAYERED_NATIVE;
        }
        ex_style
    }

    pub(crate) fn apply_window_frame_style(
        hwnd: crate::raw::types::HWND,
        fullscreen: bool,
        decorated: bool,
        resizable: bool,
        topmost: bool,
        transparent: bool,
    ) -> Result<(), String> {
        let current_style =
            unsafe { crate::raw::user32::GetWindowLongPtrW(hwnd, GWL_STYLE_NATIVE) as usize as u32 };
        let current_ex_style =
            unsafe { crate::raw::user32::GetWindowLongPtrW(hwnd, GWL_EXSTYLE_NATIVE) as usize as u32 };
        let style =
            (current_style & !frame_style_mask()) | desired_frame_style(fullscreen, decorated, resizable);
        let ex_style =
            (current_ex_style & !frame_ex_style_mask()) | desired_frame_ex_style(topmost, transparent);
        unsafe {
            crate::raw::user32::SetWindowLongPtrW(hwnd, GWL_STYLE_NATIVE, style as isize);
            crate::raw::user32::SetWindowLongPtrW(hwnd, GWL_EXSTYLE_NATIVE, ex_style as isize);
            if ex_style & WS_EX_LAYERED_NATIVE != 0 {
                let _ = SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA_NATIVE);
            }
            let z_order = if topmost {
                HWND_TOPMOST_NATIVE as crate::raw::types::HWND
            } else {
                HWND_NOTOPMOST_NATIVE as crate::raw::types::HWND
            };
            if SetWindowPos(
                hwnd,
                z_order,
                0,
                0,
                0,
                0,
                SWP_NOMOVE_NATIVE
                    | SWP_NOSIZE_NATIVE
                    | SWP_NOACTIVATE_NATIVE
                    | SWP_NOOWNERZORDER_NATIVE
                    | SWP_FRAMECHANGED_NATIVE,
            ) == 0
            {
                return Err("failed to update native window frame style".to_string());
            }
        }
        Ok(())
    }

    pub(crate) fn cursor_resource(code: i64) -> crate::raw::types::LPCWSTR {
        let id = match code {
            1 => IDC_IBEAM_NATIVE,
            2 => IDC_CROSS_NATIVE,
            3 => IDC_HAND_NATIVE,
            4 => IDC_SIZEALL_NATIVE,
            5 => IDC_WAIT_NATIVE,
            6 => IDC_HELP_NATIVE,
            7 => IDC_NO_NATIVE,
            8 => IDC_SIZEWE_NATIVE,
            9 => IDC_SIZENS_NATIVE,
            10 => IDC_SIZENWSE_NATIVE,
            11 => IDC_SIZENESW_NATIVE,
            _ => IDC_ARROW_NATIVE,
        };
        id as crate::raw::types::LPCWSTR
    }

    pub(crate) fn apply_window_cursor(window: &WinapiWindowState) -> Result<(), String> {
        let cursor = if window.cursor_visible {
            unsafe { LoadCursorW(std::ptr::null_mut(), cursor_resource(window.cursor_icon_code)) }
        } else {
            std::ptr::null_mut()
        };
        unsafe {
            let _ = SetCursor(cursor);
        }
        Ok(())
    }

    pub(crate) fn apply_cursor_grab(window: &WinapiWindowState) -> Result<(), String> {
        if window.cursor_grab_mode == 0 {
            if unsafe { ClipCursor(std::ptr::null()) } == 0 {
                return Err("failed to release native cursor grab".to_string());
            }
            return Ok(());
        }
        let mut rect = unsafe { std::mem::zeroed::<crate::raw::types::RECT>() };
        if unsafe { GetWindowRect(window.hwnd, &mut rect) } == 0 {
            return Err("failed to query native window rect for cursor grab".to_string());
        }
        if unsafe { ClipCursor(&rect) } == 0 {
            return Err("failed to apply native cursor grab".to_string());
        }
        Ok(())
    }

    pub(crate) fn client_to_screen_point(
        hwnd: crate::raw::types::HWND,
        x: i32,
        y: i32,
    ) -> Result<crate::raw::types::POINT, String> {
        let mut point = crate::raw::types::POINT { x, y };
        if unsafe { ClientToScreen(hwnd, &mut point) } == 0 {
            return Err("failed to translate client point to screen coordinates".to_string());
        }
        Ok(point)
    }

    pub(crate) fn window_frame_insets(
        hwnd: crate::raw::types::HWND,
    ) -> Result<(i32, i32, i32, i32), String> {
        let mut window_rect = unsafe { std::mem::zeroed::<crate::raw::types::RECT>() };
        let mut client_rect = unsafe { std::mem::zeroed::<crate::raw::types::RECT>() };
        if unsafe { GetWindowRect(hwnd, &mut window_rect) } == 0 {
            return Err("failed to query native window frame rect".to_string());
        }
        if unsafe { crate::raw::user32::GetClientRect(hwnd, &mut client_rect) } == 0 {
            return Err("failed to query native client rect".to_string());
        }
        let top_left = client_to_screen_point(hwnd, client_rect.left, client_rect.top)?;
        let bottom_right = client_to_screen_point(hwnd, client_rect.right, client_rect.bottom)?;
        Ok((
            top_left.x - window_rect.left,
            top_left.y - window_rect.top,
            window_rect.right - bottom_right.x,
            window_rect.bottom - bottom_right.y,
        ))
    }

    pub(crate) fn clamp_client_size(window: &WinapiWindowState, width: i64, height: i64) -> (i64, i64) {
        clamp_client_size_bounds(window.min_size, window.max_size, width, height)
    }

    pub(crate) fn clamp_client_size_bounds(
        min_size: (i64, i64),
        max_size: (i64, i64),
        width: i64,
        height: i64,
    ) -> (i64, i64) {
        let mut clamped_width = width.max(0);
        let mut clamped_height = height.max(0);
        if min_size.0 > 0 {
            clamped_width = clamped_width.max(min_size.0);
        }
        if min_size.1 > 0 {
            clamped_height = clamped_height.max(min_size.1);
        }
        if max_size.0 > 0 {
            clamped_width = clamped_width.min(max_size.0);
        }
        if max_size.1 > 0 {
            clamped_height = clamped_height.min(max_size.1);
        }
        (clamped_width, clamped_height)
    }

    pub(crate) fn outer_size_for_client_size(
        window: &WinapiWindowState,
        width: i64,
        height: i64,
    ) -> Result<(i32, i32), String> {
        outer_size_for_client_size_bounds(
            window.hwnd,
            window.min_size,
            window.max_size,
            width,
            height,
        )
    }

    pub(crate) fn outer_size_for_client_size_bounds(
        hwnd: crate::raw::types::HWND,
        min_size: (i64, i64),
        max_size: (i64, i64),
        width: i64,
        height: i64,
    ) -> Result<(i32, i32), String> {
        let (clamped_width, clamped_height) =
            clamp_client_size_bounds(min_size, max_size, width, height);
        let (left, top, right, bottom) = window_frame_insets(hwnd)?;
        let outer_width = i32::try_from(clamped_width)
            .map_err(|_| format!("client width `{clamped_width}` does not fit in i32"))?
            .saturating_add(left)
            .saturating_add(right);
        let outer_height = i32::try_from(clamped_height)
            .map_err(|_| format!("client height `{clamped_height}` does not fit in i32"))?
            .saturating_add(top)
            .saturating_add(bottom);
        Ok((outer_width.max(0), outer_height.max(0)))
    }

    pub(crate) fn set_window_client_bounds(
        window: &mut WinapiWindowState,
        position: (i64, i64),
        size: (i64, i64),
    ) -> Result<(), String> {
        let (clamped_width, clamped_height) = clamp_client_size(window, size.0, size.1);
        set_window_client_bounds_raw(
            window.hwnd,
            window.position,
            window.min_size,
            window.max_size,
            position,
            size,
        )?;
        window.position = position;
        window.width = clamped_width;
        window.height = clamped_height;
        window.resized = true;
        Ok(())
    }

    pub(crate) fn set_window_client_bounds_raw(
        hwnd: crate::raw::types::HWND,
        _current_position: (i64, i64),
        min_size: (i64, i64),
        max_size: (i64, i64),
        position: (i64, i64),
        size: (i64, i64),
    ) -> Result<(i64, i64), String> {
        let (clamped_width, clamped_height) =
            clamp_client_size_bounds(min_size, max_size, size.0, size.1);
        let (outer_width, outer_height) =
            outer_size_for_client_size_bounds(hwnd, min_size, max_size, clamped_width, clamped_height)?;
        let x = i32::try_from(position.0)
            .map_err(|_| format!("window x `{}` does not fit in i32", position.0))?;
        let y = i32::try_from(position.1)
            .map_err(|_| format!("window y `{}` does not fit in i32", position.1))?;
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                outer_width,
                outer_height,
                SWP_NOACTIVATE_NATIVE | SWP_NOOWNERZORDER_NATIVE,
            )
        } == 0
        {
            return Err("failed to update native window bounds".to_string());
        }
        Ok((clamped_width, clamped_height))
    }

    pub(crate) fn apply_window_size_constraints(window: &mut WinapiWindowState) -> Result<(), String> {
        let _ = apply_window_size_constraints_raw(
            window.hwnd,
            window.position,
            window.min_size,
            window.max_size,
            window.fullscreen,
            (window.width, window.height),
        )?;
        Ok(())
    }

    pub(crate) fn apply_window_size_constraints_raw(
        hwnd: crate::raw::types::HWND,
        position: (i64, i64),
        min_size: (i64, i64),
        max_size: (i64, i64),
        fullscreen: bool,
        current_size: (i64, i64),
    ) -> Result<(i64, i64), String> {
        if fullscreen {
            return Ok(current_size);
        }
        let (clamped_width, clamped_height) =
            clamp_client_size_bounds(min_size, max_size, current_size.0, current_size.1);
        if clamped_width != current_size.0 || clamped_height != current_size.1 {
            return set_window_client_bounds_raw(
                hwnd,
                position,
                min_size,
                max_size,
                position,
                (clamped_width, clamped_height),
            );
        }
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE_NATIVE
                    | SWP_NOSIZE_NATIVE
                    | SWP_NOACTIVATE_NATIVE
                    | SWP_NOOWNERZORDER_NATIVE
                    | SWP_FRAMECHANGED_NATIVE,
            )
        } == 0
        {
            return Err("failed to refresh native min/max size constraints".to_string());
        }
        Ok((clamped_width, clamped_height))
    }

    pub(crate) fn current_monitor_rect(
        hwnd: crate::raw::types::HWND,
    ) -> Result<crate::raw::types::RECT, String> {
        let monitor = unsafe {
            crate::raw::user32::MonitorFromWindow(
                hwnd,
                MONITOR_DEFAULTTONEAREST_NATIVE,
            )
        };
        if monitor.is_null() {
            return Err("failed to resolve current monitor".to_string());
        }
        let mut info = unsafe { std::mem::zeroed::<crate::raw::types::MONITORINFOEXW>() };
        info.cbSize = std::mem::size_of::<crate::raw::types::MONITORINFOEXW>() as u32;
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            return Err("failed to query current monitor info".to_string());
        }
        Ok(info.rcMonitor)
    }

    pub(crate) fn window_theme_code(
        hwnd: crate::raw::types::HWND,
        theme_override_code: i64,
    ) -> i64 {
        let mut dark = 0i32;
        let hr = unsafe {
            crate::raw::dwmapi::DwmGetWindowAttribute(
                hwnd,
                crate::raw::constants::DWMWA_USE_IMMERSIVE_DARK_MODE,
                &mut dark as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<crate::raw::types::BOOL>() as u32,
            )
        };
        if crate::helpers_impl::hresult_succeeded_native(hr) {
            if dark != 0 {
                return 2;
            }
            return 1;
        }
        match theme_override_code {
            1 | 2 => theme_override_code,
            _ => 0,
        }
    }

    pub(crate) fn ime_string(
        hwnd: crate::raw::types::HWND,
        index: u32,
    ) -> Result<String, String> {
        let context = unsafe { crate::raw::imm32::ImmGetContext(hwnd) };
        if context.is_null() {
            return Ok(String::new());
        }
        let result = (|| -> Result<String, String> {
            let size = unsafe {
                crate::raw::imm32::ImmGetCompositionStringW(
                    context,
                    index,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if size <= 0 {
                return Ok(String::new());
            }
            let mut bytes = vec![0u8; size as usize];
            let actual = unsafe {
                crate::raw::imm32::ImmGetCompositionStringW(
                    context,
                    index,
                    bytes.as_mut_ptr() as *mut std::ffi::c_void,
                    size as u32,
                )
            };
            if actual < 0 {
                return Err(format!(
                    "ImmGetCompositionStringW failed for index {index} with code {actual}"
                ));
            }
            let units = actual as usize / std::mem::size_of::<u16>();
            let slice = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const u16, units) };
            Ok(String::from_utf16_lossy(slice))
        })();
        unsafe {
            let _ = crate::raw::imm32::ImmReleaseContext(hwnd, context);
        }
        result
    }

    pub(crate) fn ime_cursor(hwnd: crate::raw::types::HWND) -> i64 {
        let context = unsafe { crate::raw::imm32::ImmGetContext(hwnd) };
        if context.is_null() {
            return 0;
        }
        let value = unsafe {
            crate::raw::imm32::ImmGetCompositionStringW(
                context,
                GCS_CURSORPOS_NATIVE,
                std::ptr::null_mut(),
                0,
            )
        };
        unsafe {
            let _ = crate::raw::imm32::ImmReleaseContext(hwnd, context);
        }
        if value < 0 { 0 } else { i64::from(value) }
    }

    pub(crate) fn apply_composition_area(window: &WinapiWindowState) -> Result<(), String> {
        let context = unsafe { crate::raw::imm32::ImmGetContext(window.hwnd) };
        if context.is_null() {
            return Ok(());
        }
        let result = (|| -> Result<(), String> {
            let mut form = crate::raw::types::COMPOSITIONFORM {
                dwStyle: CFS_DEFAULT_NATIVE,
                ptCurrentPos: crate::raw::types::POINT { x: 0, y: 0 },
                rcArea: crate::raw::types::RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
            };
            if window.composition_area_active {
                let position = crate::raw::types::POINT {
                    x: i32::try_from(window.composition_area_position.0)
                        .map_err(|_| format!(
                            "composition x `{}` does not fit in i32",
                            window.composition_area_position.0
                        ))?,
                    y: i32::try_from(window.composition_area_position.1)
                        .map_err(|_| format!(
                            "composition y `{}` does not fit in i32",
                            window.composition_area_position.1
                        ))?,
                };
                let width = i32::try_from(window.composition_area_size.0.max(0))
                    .map_err(|_| format!(
                        "composition width `{}` does not fit in i32",
                        window.composition_area_size.0
                    ))?;
                let height = i32::try_from(window.composition_area_size.1.max(0))
                    .map_err(|_| format!(
                        "composition height `{}` does not fit in i32",
                        window.composition_area_size.1
                    ))?;
                form.dwStyle = if width > 0 || height > 0 {
                    CFS_RECT_NATIVE
                } else {
                    CFS_FORCE_POSITION_NATIVE
                };
                form.ptCurrentPos = position;
                form.rcArea = crate::raw::types::RECT {
                    left: position.x,
                    top: position.y,
                    right: position.x.saturating_add(width),
                    bottom: position.y.saturating_add(height),
                };
            }
            if unsafe { crate::raw::imm32::ImmSetCompositionWindow(context, &form) } == 0 {
                return Err("failed to update native IME composition area".to_string());
            }
            Ok(())
        })();
        unsafe {
            let _ = crate::raw::imm32::ImmReleaseContext(window.hwnd, context);
        }
        result
    }

    pub(crate) fn apply_text_input(window: &WinapiWindowState) -> Result<(), String> {
        let flags = if window.text_input_enabled {
            IACE_DEFAULT_NATIVE | IACE_IGNORENOCONTEXT_NATIVE
        } else {
            0
        };
        let associated = unsafe {
            crate::raw::imm32::ImmAssociateContextEx(
                window.hwnd,
                std::ptr::null_mut(),
                flags,
            )
        };
        if associated == 0 && window.text_input_enabled {
            return Err("failed to update native IME enabled state".to_string());
        }
        if window.text_input_enabled {
            apply_composition_area(window)?;
        }
        Ok(())
    }

    pub(crate) fn set_window_fullscreen(
        window: &mut WinapiWindowState,
        enabled: bool,
    ) -> Result<(), String> {
        let hwnd = window.hwnd;
        let position = window.position;
        let size = (window.width, window.height);
        let restore_position = window.fullscreen_restore_position;
        let restore_size = window.fullscreen_restore_size;
        let restore_maximized = window.fullscreen_restore_maximized;
        let min_size = window.min_size;
        let max_size = window.max_size;
        let decorated = window.decorated;
        let resizable = window.resizable;
        let topmost = window.topmost;
        let transparent = window.transparent;
        if window.fullscreen == enabled {
            return Ok(());
        }
        if enabled {
            apply_window_frame_style(hwnd, true, decorated, resizable, topmost, transparent)?;
            let monitor = current_monitor_rect(hwnd)?;
            let z_order = if topmost {
                HWND_TOPMOST_NATIVE as crate::raw::types::HWND
            } else {
                HWND_NOTOPMOST_NATIVE as crate::raw::types::HWND
            };
            if unsafe {
                SetWindowPos(
                    hwnd,
                    z_order,
                    monitor.left,
                    monitor.top,
                    monitor.right.saturating_sub(monitor.left),
                    monitor.bottom.saturating_sub(monitor.top),
                    SWP_NOACTIVATE_NATIVE | SWP_NOOWNERZORDER_NATIVE,
                )
            } == 0
            {
                let _ = apply_window_frame_style(
                    hwnd,
                    false,
                    decorated,
                    resizable,
                    topmost,
                    transparent,
                );
                return Err("failed to enter native fullscreen".to_string());
            }
            window.fullscreen_restore_position = position;
            window.fullscreen_restore_size = size;
            window.fullscreen_restore_maximized = window.maximized;
            window.fullscreen = true;
            window.minimized = false;
            window.maximized = false;
            window.position = (i64::from(monitor.left), i64::from(monitor.top));
            window.width = i64::from(monitor.right.saturating_sub(monitor.left));
            window.height = i64::from(monitor.bottom.saturating_sub(monitor.top));
            window.resized = true;
            return Ok(());
        }
        apply_window_frame_style(hwnd, false, decorated, resizable, topmost, transparent)?;
        if restore_maximized {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE_NATIVE);
                let _ = ShowWindow(hwnd, SW_MAXIMIZE_NATIVE);
            }
            window.fullscreen = false;
            window.maximized = true;
            window.minimized = false;
            window.resized = true;
            return Ok(());
        }
        let restored = set_window_client_bounds_raw(
            hwnd,
            position,
            min_size,
            max_size,
            restore_position,
            restore_size,
        )?;
        window.fullscreen = false;
        window.maximized = false;
        window.minimized = false;
        window.position = restore_position;
        window.width = restored.0;
        window.height = restored.1;
        window.resized = true;
        Ok(())
    }

    pub(crate) fn new_desktop_state_handle() -> u64 {
        let state = Box::new(WinapiDesktopState {
            next_window_handle: 1,
            windows: std::collections::BTreeMap::new(),
            next_frame_handle: 1,
            frames: std::collections::BTreeMap::new(),
            next_wake_handle: 1,
            wakes: std::collections::BTreeMap::new(),
        });
        Box::into_raw(state) as usize as u64
    }

    pub(crate) fn destroy_desktop_state_handle(handle: u64) {
        let ptr = handle as usize as *mut WinapiDesktopState;
        if !ptr.is_null() {
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }

    impl Drop for WinapiDesktopState {
        fn drop(&mut self) {
            let mut hwnds = Vec::new();
            for window in self.windows.values() {
                if !window.hwnd.is_null() {
                    hwnds.push(window.hwnd);
                }
            }
            for hwnd in hwnds {
                unsafe {
                    let _ = crate::raw::user32::DestroyWindow(hwnd);
                }
            }
            for wake in self.wakes.values() {
                if !wake.event.is_null() {
                    unsafe {
                        let _ = crate::raw::kernel32::CloseHandle(wake.event);
                    }
                }
            }
        }
    }

    pub(crate) fn desktop_state_ref(
        instance: &crate::BindingInstance,
    ) -> Result<&WinapiDesktopState, String> {
        let handle = crate::shackle::package_state_data_ref(instance)?.desktop_state_handle;
        let ptr = handle as usize as *const WinapiDesktopState;
        if ptr.is_null() {
            return Err("desktop state handle must not be null".to_string());
        }
        Ok(unsafe { &*ptr })
    }

    pub(crate) fn desktop_state_mut(
        instance: &mut crate::BindingInstance,
    ) -> Result<&mut WinapiDesktopState, String> {
        let handle = crate::shackle::package_state_data_ref(instance)?.desktop_state_handle;
        desktop_state_mut_from_handle(handle)
    }

    pub(crate) fn desktop_state_ref_from_handle(
        handle: u64,
    ) -> Result<&'static WinapiDesktopState, String> {
        let ptr = handle as usize as *mut WinapiDesktopState;
        if ptr.is_null() {
            return Err("desktop state handle must not be null".to_string());
        }
        Ok(unsafe { &*(ptr as *const WinapiDesktopState) })
    }

    pub(crate) fn desktop_state_mut_from_handle(
        handle: u64,
    ) -> Result<&'static mut WinapiDesktopState, String> {
        let ptr = handle as usize as *mut WinapiDesktopState;
        if ptr.is_null() {
            return Err("desktop state handle must not be null".to_string());
        }
        Ok(unsafe { &mut *ptr })
    }

    pub(crate) fn wide_nul(text: &str) -> Vec<u16> {
        text.encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    }

    pub(crate) fn desktop_window_class_name() -> Vec<u16> {
        "ArcanaDesktopWindow\0".encode_utf16().collect::<Vec<u16>>()
    }

    pub(crate) fn window_id_value(handle: u64) -> Result<i64, String> {
        i64::try_from(handle).map_err(|_| format!("window handle `{handle}` does not fit in Int"))
    }

    pub(crate) fn window_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiWindowState, String> {
        desktop_state_ref(instance)?
            .windows
            .get(&handle)
            .ok_or_else(|| format!("invalid Window handle `{handle}`"))
    }

    pub(crate) fn window_mut(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<&mut WinapiWindowState, String> {
        desktop_state_mut(instance)?
            .windows
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Window handle `{handle}`"))
    }

    pub(crate) fn window_ref_from_state(
        desktop_state_handle: u64,
        handle: u64,
    ) -> Result<&'static WinapiWindowState, String> {
        desktop_state_ref_from_handle(desktop_state_handle)?
            .windows
            .get(&handle)
            .ok_or_else(|| format!("invalid Window handle `{handle}`"))
    }

    pub(crate) fn window_mut_from_state(
        desktop_state_handle: u64,
        handle: u64,
    ) -> Result<&'static mut WinapiWindowState, String> {
        desktop_state_mut_from_handle(desktop_state_handle)?
            .windows
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Window handle `{handle}`"))
    }

    pub(crate) fn frame_mut(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<&mut WinapiFrameState, String> {
        desktop_state_mut(instance)?
            .frames
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid FrameInput handle `{handle}`"))
    }

    pub(crate) fn frame_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiFrameState, String> {
        desktop_state_ref(instance)?
            .frames
            .get(&handle)
            .ok_or_else(|| format!("invalid FrameInput handle `{handle}`"))
    }

    pub(crate) fn wake_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiWakeState, String> {
        desktop_state_ref(instance)?
            .wakes
            .get(&handle)
            .ok_or_else(|| format!("invalid WakeHandle `{handle}`"))
    }

    pub(crate) fn wake_mut(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<&mut WinapiWakeState, String> {
        desktop_state_mut(instance)?
            .wakes
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid WakeHandle `{handle}`"))
    }

    pub(crate) fn push_window_event(
        instance: &mut crate::BindingInstance,
        handle: u64,
        event: PendingDesktopEvent,
    ) -> Result<(), String> {
        window_mut(instance, handle)?.events.push_back(event);
        Ok(())
    }

    pub(crate) fn make_window_event(
        handle: u64,
        kind: i64,
        a: i64,
        b: i64,
    ) -> Result<PendingDesktopEvent, String> {
        Ok(PendingDesktopEvent {
            kind,
            window_id: window_id_value(handle)?,
            a,
            b,
            ..PendingDesktopEvent::default()
        })
    }

    pub(crate) fn push_unique(out: &mut Vec<i64>, value: i64) {
        if !out.contains(&value) {
            out.push(value);
        }
    }

    pub(crate) fn retain_without(out: &mut Vec<i64>, value: i64) {
        out.retain(|item| *item != value);
    }

    pub(crate) fn frame_input_from_events(
        events: &std::collections::VecDeque<PendingDesktopEvent>,
    ) -> WinapiFrameInputState {
        let mut input = WinapiFrameInputState::default();
        for event in events {
            match event.kind {
                EVENT_KEY_DOWN => {
                    push_unique(&mut input.key_down, event.key_code);
                    push_unique(&mut input.key_pressed, event.key_code);
                }
                EVENT_KEY_UP => {
                    retain_without(&mut input.key_down, event.key_code);
                    push_unique(&mut input.key_released, event.key_code);
                }
                EVENT_MOUSE_DOWN => {
                    push_unique(&mut input.mouse_down, event.a);
                    push_unique(&mut input.mouse_pressed, event.a);
                    input.mouse_pos = (event.pointer_x, event.pointer_y);
                    input.mouse_in_window = true;
                }
                EVENT_MOUSE_UP => {
                    retain_without(&mut input.mouse_down, event.a);
                    push_unique(&mut input.mouse_released, event.a);
                    input.mouse_pos = (event.pointer_x, event.pointer_y);
                    input.mouse_in_window = true;
                }
                EVENT_MOUSE_MOVE => {
                    input.mouse_pos = (event.a, event.b);
                    input.mouse_in_window = true;
                }
                EVENT_MOUSE_WHEEL => {
                    input.mouse_wheel_y += event.a;
                    if event.pointer_x != 0 || event.pointer_y != 0 {
                        input.mouse_pos = (event.pointer_x, event.pointer_y);
                    }
                    input.mouse_in_window = true;
                }
                EVENT_MOUSE_ENTERED => {
                    input.mouse_in_window = true;
                }
                EVENT_MOUSE_LEFT => {
                    input.mouse_in_window = false;
                }
                EVENT_RAW_MOUSE_BUTTON => {
                    if event.b != 0 {
                        push_unique(&mut input.mouse_down, event.a);
                        push_unique(&mut input.mouse_pressed, event.a);
                    } else {
                        retain_without(&mut input.mouse_down, event.a);
                        push_unique(&mut input.mouse_released, event.a);
                    }
                }
                EVENT_RAW_MOUSE_WHEEL => {
                    input.mouse_wheel_y += event.b;
                }
                EVENT_RAW_KEY => {
                    if event.b != 0 {
                        push_unique(&mut input.key_down, event.key_code);
                        push_unique(&mut input.key_pressed, event.key_code);
                    } else {
                        retain_without(&mut input.key_down, event.key_code);
                        push_unique(&mut input.key_released, event.key_code);
                    }
                }
                _ => {}
            }
        }
        input
    }

    pub(crate) fn named_key_code(name: &str) -> i64 {
        match name {
            "Backspace" | "backspace" => 8,
            "Tab" | "tab" => 9,
            "Enter" | "enter" => 13,
            "Shift" | "shift" => 16,
            "ShiftLeft" | "shiftleft" | "LShift" | "lshift" => 160,
            "ShiftRight" | "shiftright" | "RShift" | "rshift" => 161,
            "Control" | "control" | "Ctrl" | "ctrl" => 17,
            "ControlLeft" | "controlleft" | "CtrlLeft" | "ctrlleft" | "LControl" | "lcontrol"
            | "LCtrl" | "lctrl" => 162,
            "ControlRight" | "controlright" | "CtrlRight" | "ctrlright" | "RControl"
            | "rcontrol" | "RCtrl" | "rctrl" => 163,
            "Alt" | "alt" => 18,
            "AltLeft" | "altleft" | "LAlt" | "lalt" => 164,
            "AltRight" | "altright" | "RAlt" | "ralt" => 165,
            "Pause" | "pause" => 19,
            "CapsLock" | "capslock" => 20,
            "Escape" | "escape" => 27,
            "Space" | "space" => 32,
            "PageUp" | "pageup" => 33,
            "PageDown" | "pagedown" => 34,
            "End" | "end" => 35,
            "Home" | "home" => 36,
            "Left" | "left" => 37,
            "Up" | "up" => 38,
            "Right" | "right" => 39,
            "Down" | "down" => 40,
            "Insert" | "insert" => 45,
            "Delete" | "delete" => 46,
            "Meta" | "meta" | "Super" | "super" | "Command" | "command" => 91,
            "MetaLeft" | "metaleft" | "SuperLeft" | "superleft" | "CommandLeft"
            | "commandleft" => 91,
            "MetaRight" | "metaright" | "SuperRight" | "superright" | "CommandRight"
            | "commandright" => 92,
            "Select" | "select" => 93,
            "NumLock" | "numlock" => 144,
            "ScrollLock" | "scrolllock" => 145,
            "Numpad0" | "numpad0" => 96,
            "Numpad1" | "numpad1" => 97,
            "Numpad2" | "numpad2" => 98,
            "Numpad3" | "numpad3" => 99,
            "Numpad4" | "numpad4" => 100,
            "Numpad5" | "numpad5" => 101,
            "Numpad6" | "numpad6" => 102,
            "Numpad7" | "numpad7" => 103,
            "Numpad8" | "numpad8" => 104,
            "Numpad9" | "numpad9" => 105,
            "NumpadMultiply" | "numpadmultiply" => 106,
            "NumpadAdd" | "numpadadd" => 107,
            "NumpadSubtract" | "numpadsubtract" => 109,
            "NumpadDecimal" | "numpaddecimal" => 110,
            "NumpadDivide" | "numpaddivide" => 111,
            "F1" | "f1" => 112,
            "F2" | "f2" => 113,
            "F3" | "f3" => 114,
            "F4" | "f4" => 115,
            "F5" | "f5" => 116,
            "F6" | "f6" => 117,
            "F7" | "f7" => 118,
            "F8" | "f8" => 119,
            "F9" | "f9" => 120,
            "F10" | "f10" => 121,
            "F11" | "f11" => 122,
            "F12" | "f12" => 123,
            "Semicolon" | "semicolon" => 186,
            "Equal" | "equal" | "Equals" | "equals" => 187,
            "Comma" | "comma" => 188,
            "Minus" | "minus" => 189,
            "Period" | "period" => 190,
            "Slash" | "slash" => 191,
            "Backquote" | "backquote" | "Grave" | "grave" => 192,
            "BracketLeft" | "bracketleft" => 219,
            "Backslash" | "backslash" => 220,
            "BracketRight" | "bracketright" => 221,
            "Quote" | "quote" | "Apostrophe" | "apostrophe" => 222,
            _ if name.len() == 1 => name.chars().next().unwrap().to_ascii_uppercase() as i64,
            _ => -1,
        }
    }

    pub(crate) fn named_mouse_button_code(name: &str) -> i64 {
        match name {
            "Left" | "left" => 1,
            "Right" | "right" => 2,
            "Middle" | "middle" => 3,
            "Back" | "back" | "X1" | "x1" => 4,
            "Forward" | "forward" | "X2" | "x2" => 5,
            _ => -1,
        }
    }

    pub(crate) fn register_desktop_window_class() -> Result<(), String> {
        DESKTOP_WINDOW_CLASS
            .get_or_init(|| {
                let module = crate::shackle::current_module_handle_for_address(
                    desktop_window_proc as usize as crate::shackle::LPCVOID,
                )?;
                let class_name = desktop_window_class_name();
                let class = crate::raw::types::WNDCLASSW {
                    style: 0,
                    lpfnWndProc: Some(desktop_window_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: module,
                    hIcon: std::ptr::null_mut(),
                    hCursor: std::ptr::null_mut(),
                    hbrBackground: std::ptr::null_mut(),
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                };
                let atom = unsafe { crate::raw::user32::RegisterClassW(&class) };
                if atom == 0 {
                    let err = unsafe { crate::raw::kernel32::GetLastError() };
                    if err != crate::raw::constants::ERROR_CLASS_ALREADY_EXISTS {
                        return Err(format!("RegisterClassW failed with Win32 error {err}"));
                    }
                }
                Ok(())
            })
            .clone()
    }

    pub(crate) fn process_pending_messages() {
        unsafe {
            let mut message = std::mem::zeroed::<crate::raw::types::MSG>();
            while crate::raw::user32::PeekMessageW(
                &mut message as *mut _,
                std::ptr::null_mut(),
                0,
                0,
                crate::raw::constants::PM_REMOVE,
            ) != 0
            {
                crate::raw::user32::TranslateMessage(&message as *const _);
                crate::raw::user32::DispatchMessageW(&message as *const _);
            }
        }
    }

    pub(crate) fn collect_monitor_infos() -> Result<Vec<NativeMonitorInfo>, String> {
        unsafe extern "system" fn collect_monitor_handle_proc(
            monitor: crate::raw::types::HMONITOR,
            _hdc: crate::raw::types::HDC,
            _rect: *mut crate::raw::types::RECT,
            data: crate::raw::types::LPARAM,
        ) -> i32 {
            let handles = unsafe { &mut *(data as *mut Vec<crate::raw::types::HMONITOR>) };
            handles.push(monitor);
            1
        }

        let mut handles = Vec::new();
        let ok = unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(collect_monitor_handle_proc),
                (&mut handles as *mut Vec<crate::raw::types::HMONITOR>) as isize,
            )
        };
        if ok == 0 {
            return Err("failed to enumerate native monitors".to_string());
        }
        let mut monitors = Vec::with_capacity(handles.len());
        for handle in handles {
            let mut info = unsafe { std::mem::zeroed::<crate::raw::types::MONITORINFOEXW>() };
            info.cbSize = std::mem::size_of::<crate::raw::types::MONITORINFOEXW>() as u32;
            if unsafe { GetMonitorInfoW(handle, &mut info) } == 0 {
                return Err("failed to query native monitor info".to_string());
            }
            let mut dpi_x = 96u32;
            let mut dpi_y = 96u32;
            let status = unsafe {
                crate::raw::shcore::GetDpiForMonitor(
                    handle,
                    crate::raw::constants::MDT_EFFECTIVE_DPI,
                    &mut dpi_x,
                    &mut dpi_y,
                )
            };
            let scale_factor_milli = if crate::helpers_impl::hresult_succeeded_native(status) {
                i64::from(dpi_x.max(dpi_y)) * 1000 / 96
            } else {
                1000
            };
            let name_len = info
                .szDevice
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(info.szDevice.len());
            let name = String::from_utf16_lossy(&info.szDevice[..name_len]);
            monitors.push(NativeMonitorInfo {
                handle,
                name,
                position: (
                    info.rcMonitor.left as i64,
                    info.rcMonitor.top as i64,
                ),
                size: (
                    (info.rcMonitor.right - info.rcMonitor.left) as i64,
                    (info.rcMonitor.bottom - info.rcMonitor.top) as i64,
                ),
                scale_factor_milli,
                primary: info.dwFlags & 1 != 0,
            });
        }
        Ok(monitors)
    }

    pub(crate) fn current_monitor_index_for_window(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<i64, String> {
        let monitor = unsafe {
            crate::raw::user32::MonitorFromWindow(
                window_ref(instance, handle)?.hwnd,
                MONITOR_DEFAULTTONEAREST_NATIVE,
            )
        };
        for (index, info) in collect_monitor_infos()?.iter().enumerate() {
            if info.handle == monitor {
                return i64::try_from(index)
                    .map_err(|_| "native monitor index does not fit in Int".to_string());
            }
        }
        Ok(0)
    }

    pub(crate) fn create_frame(
        instance: &mut crate::BindingInstance,
        events: std::collections::VecDeque<PendingDesktopEvent>,
    ) -> u64 {
        let state = desktop_state_mut(instance).expect("desktop state must exist");
        let handle = state.next_frame_handle;
        state.next_frame_handle += 1;
        let input = frame_input_from_events(&events);
        state.frames.insert(
            handle,
            WinapiFrameState {
                events,
                input,
                last_polled: None,
            },
        );
        handle
    }

    pub(crate) fn drain_window_events(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<std::collections::VecDeque<PendingDesktopEvent>, String> {
        let window = window_mut(instance, handle)?;
        Ok(std::mem::take(&mut window.events))
    }

    pub(crate) fn collect_window_events(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<std::collections::VecDeque<PendingDesktopEvent>, String> {
        process_pending_messages();
        drain_window_events(instance, handle)
    }

    pub(crate) fn wait_for_wake_or_messages(
        instance: &mut crate::BindingInstance,
        handle: u64,
        timeout_ms: i64,
    ) -> Result<bool, String> {
        process_pending_messages();
        let event = wake_ref(instance, handle)?.event;
        let timeout = if timeout_ms < 0 {
            crate::raw::constants::INFINITE
        } else {
            u32::try_from(timeout_ms)
                .map_err(|_| format!("wait timeout `{timeout_ms}` does not fit in DWORD"))?
        };
        let mut handles = [event];
        let result = unsafe {
            crate::raw::user32::MsgWaitForMultipleObjectsEx(
                1,
                handles.as_mut_ptr(),
                timeout,
                crate::raw::constants::QS_ALLINPUT,
                crate::raw::constants::MWMO_INPUTAVAILABLE,
            )
        };
        if result == crate::raw::constants::WAIT_FAILED {
            return Err(format!(
                "MsgWaitForMultipleObjectsEx failed with Win32 error {}",
                unsafe { crate::raw::kernel32::GetLastError() }
            ));
        }
        if result == crate::raw::constants::WAIT_TIMEOUT {
            return Ok(false);
        }
        Ok(true)
    }

    pub(crate) unsafe extern "system" fn desktop_window_proc(
        hwnd: crate::raw::types::HWND,
        message: crate::raw::types::UINT,
        wparam: crate::raw::types::WPARAM,
        lparam: crate::raw::types::LPARAM,
    ) -> crate::raw::types::LRESULT {
        if message == crate::raw::constants::WM_NCCREATE {
            let create = lparam as crate::raw::types::PCREATESTRUCTW;
            if !create.is_null() {
                let params = unsafe { (*create).lpCreateParams as crate::raw::types::LONG_PTR };
                unsafe {
                    crate::raw::user32::SetWindowLongPtrW(
                        hwnd,
                        crate::raw::constants::GWLP_USERDATA,
                        params,
                    );
                }
            }
            return 1;
        }
        let proc_state = unsafe {
            crate::raw::user32::GetWindowLongPtrW(
                hwnd,
                crate::raw::constants::GWLP_USERDATA,
            ) as *mut DesktopWindowProcState
        };
        if proc_state.is_null() {
            return unsafe { crate::raw::user32::DefWindowProcW(hwnd, message, wparam, lparam) };
        }
        let desktop_state_handle = unsafe { (*proc_state).desktop_state_handle };
        let handle = unsafe { (*proc_state).handle };
        let _ = (|| -> Result<(), String> {
            match message {
                WM_SIZE_NATIVE => {
                    let width = (lparam as u32 & 0xFFFF) as i64;
                    let height = ((lparam as u32 >> 16) & 0xFFFF) as i64;
                    let window = window_mut_from_state(desktop_state_handle, handle)?;
                    window.width = width.max(0);
                    window.height = height.max(0);
                    window.resized = true;
                    window.minimized = wparam as usize == 1;
                    window.maximized = wparam as usize == 2;
                    window.events
                        .push_back(make_window_event(handle, EVENT_WINDOW_RESIZED, window.width, window.height)?);
                }
                WM_MOVE_NATIVE => {
                    let x = (lparam as u32 & 0xFFFF) as u16 as i16 as i64;
                    let y = ((lparam as u32 >> 16) & 0xFFFF) as u16 as i16 as i64;
                    let window = window_mut_from_state(desktop_state_handle, handle)?;
                    window.position = (x, y);
                    window
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_MOVED, x, y)?);
                }
                WM_SETFOCUS_NATIVE => {
                    let window = window_mut_from_state(desktop_state_handle, handle)?;
                    window.focused = true;
                    window
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_FOCUSED, 1, 0)?);
                }
                WM_KILLFOCUS_NATIVE => {
                    let window = window_mut_from_state(desktop_state_handle, handle)?;
                    window.focused = false;
                    window
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_FOCUSED, 0, 0)?);
                }
                WM_GETMINMAXINFO_NATIVE => {
                    let info = lparam as *mut crate::raw::types::MINMAXINFO;
                    if !info.is_null() {
                        let window = window_ref_from_state(desktop_state_handle, handle)?;
                        let (left, top, right, bottom) = window_frame_insets(window.hwnd)?;
                        let minmax = unsafe { &mut *info };
                        if window.min_size.0 > 0 {
                            minmax.ptMinTrackSize.x = i32::try_from(window.min_size.0)
                                .map_err(|_| format!(
                                    "min width `{}` does not fit in i32",
                                    window.min_size.0
                                ))?
                                .saturating_add(left)
                                .saturating_add(right);
                        }
                        if window.min_size.1 > 0 {
                            minmax.ptMinTrackSize.y = i32::try_from(window.min_size.1)
                                .map_err(|_| format!(
                                    "min height `{}` does not fit in i32",
                                    window.min_size.1
                                ))?
                                .saturating_add(top)
                                .saturating_add(bottom);
                        }
                        if window.max_size.0 > 0 {
                            minmax.ptMaxTrackSize.x = i32::try_from(window.max_size.0)
                                .map_err(|_| format!(
                                    "max width `{}` does not fit in i32",
                                    window.max_size.0
                                ))?
                                .saturating_add(left)
                                .saturating_add(right);
                        }
                        if window.max_size.1 > 0 {
                            minmax.ptMaxTrackSize.y = i32::try_from(window.max_size.1)
                                .map_err(|_| format!(
                                    "max height `{}` does not fit in i32",
                                    window.max_size.1
                                ))?
                                .saturating_add(top)
                                .saturating_add(bottom);
                        }
                    }
                    return Ok(());
                }
                WM_CHAR_NATIVE => {
                    if let Some(ch) = char::from_u32(wparam as u32) {
                        let mut event = make_window_event(handle, EVENT_TEXT_INPUT, 0, 0)?;
                        event.text = ch.to_string();
                        window_mut_from_state(desktop_state_handle, handle)?
                            .events
                            .push_back(event);
                    }
                }
                WM_DROPFILES_NATIVE => {
                    let drop = wparam as crate::raw::types::HDROP;
                    let count = unsafe {
                        crate::raw::shell32::DragQueryFileW(
                            drop,
                            u32::MAX,
                            std::ptr::null_mut(),
                            0,
                        )
                    };
                    for index in 0..count {
                        let len = unsafe {
                            crate::raw::shell32::DragQueryFileW(
                                drop,
                                index,
                                std::ptr::null_mut(),
                                0,
                            )
                        };
                        let mut units = vec![0u16; len as usize + 1];
                        unsafe {
                            let _ = crate::raw::shell32::DragQueryFileW(
                                drop,
                                index,
                                units.as_mut_ptr(),
                                len + 1,
                            );
                        }
                        let text = String::from_utf16_lossy(&units[..len as usize]);
                        let mut event = make_window_event(handle, EVENT_FILE_DROPPED, 0, 0)?;
                        event.text = text;
                        window_mut_from_state(desktop_state_handle, handle)?
                            .events
                            .push_back(event);
                    }
                    unsafe {
                        crate::raw::shell32::DragFinish(drop);
                    }
                    return Ok(());
                }
                WM_IME_STARTCOMPOSITION_NATIVE => {
                    let window = window_mut_from_state(desktop_state_handle, handle)?;
                    window.ime_composing = true;
                    window.events
                        .push_back(make_window_event(handle, EVENT_TEXT_COMPOSITION_STARTED, 0, 0)?);
                    return Ok(());
                }
                WM_IME_COMPOSITION_NATIVE => {
                    let flags = lparam as usize as u32;
                    if flags & GCS_COMPSTR_NATIVE != 0 {
                        let text = ime_string(hwnd, GCS_COMPSTR_NATIVE)?;
                        let caret = ime_cursor(hwnd);
                        let window = window_mut_from_state(desktop_state_handle, handle)?;
                        window.ime_composing = true;
                        let mut event = make_window_event(handle, EVENT_TEXT_COMPOSITION_UPDATED, caret, 0)?;
                        event.text = text;
                        window.events.push_back(event);
                    }
                    if flags & GCS_RESULTSTR_NATIVE != 0 {
                        let text = ime_string(hwnd, GCS_RESULTSTR_NATIVE)?;
                        let caret = ime_cursor(hwnd);
                        let window = window_mut_from_state(desktop_state_handle, handle)?;
                        window.ime_composing = false;
                        let mut event = make_window_event(handle, EVENT_TEXT_COMPOSITION_COMMITTED, caret, 0)?;
                        event.text = text;
                        window.events.push_back(event);
                    }
                    return Ok(());
                }
                WM_IME_ENDCOMPOSITION_NATIVE => {
                    let window = window_mut_from_state(desktop_state_handle, handle)?;
                    if window.ime_composing {
                        window.events.push_back(make_window_event(
                            handle,
                            EVENT_TEXT_COMPOSITION_CANCELLED,
                            0,
                            0,
                        )?);
                    }
                    window.ime_composing = false;
                    return Ok(());
                }
                WM_PAINT_NATIVE => {
                    let mut paint = unsafe { std::mem::zeroed::<PAINTSTRUCT>() };
                    unsafe {
                        let _ = BeginPaint(hwnd, &mut paint);
                        let _ = EndPaint(hwnd, &paint);
                    }
                    window_mut_from_state(desktop_state_handle, handle)?
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_REDRAW_REQUESTED, 0, 0)?);
                    return Ok(());
                }
                WM_SETCURSOR_NATIVE => {
                    let window = window_ref_from_state(desktop_state_handle, handle)?;
                    apply_window_cursor(window)?;
                    return Ok(());
                }
                WM_THEMECHANGED_NATIVE => {
                    let window = window_ref_from_state(desktop_state_handle, handle)?;
                    let theme_code = window_theme_code(window.hwnd, window.theme_override_code);
                    window_mut_from_state(desktop_state_handle, handle)?
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_THEME_CHANGED, theme_code, 0)?);
                }
                crate::raw::constants::WM_CLOSE => {
                    window_mut_from_state(desktop_state_handle, handle)?
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_CLOSE_REQUESTED, 0, 0)?);
                    return Ok(());
                }
                WM_DESTROY_NATIVE | WM_NCDESTROY_NATIVE => {
                    if let Ok(window) = window_mut_from_state(desktop_state_handle, handle) {
                        if window.cursor_grab_mode != 0 {
                            unsafe {
                                let _ = ClipCursor(std::ptr::null());
                            }
                        }
                        window.ime_composing = false;
                        window.closed = true;
                        window.hwnd = std::ptr::null_mut();
                    }
                    if message == WM_NCDESTROY_NATIVE {
                        unsafe {
                            crate::raw::user32::SetWindowLongPtrW(
                                hwnd,
                                crate::raw::constants::GWLP_USERDATA,
                                0,
                            );
                            drop(Box::from_raw(proc_state));
                        }
                    }
                }
                WM_DPICHANGED_NATIVE => {
                    let dpi = unsafe { crate::raw::user32::GetDpiForWindow(hwnd) };
                    let scale = if dpi == 0 { 1000 } else { i64::from(dpi) * 1000 / 96 };
                    window_mut_from_state(desktop_state_handle, handle)?
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_SCALE_FACTOR_CHANGED, scale, 0)?);
                }
                _ => {}
            }
            Ok(())
        })();
        match message {
            WM_PAINT_NATIVE
            | WM_GETMINMAXINFO_NATIVE
            | WM_SETCURSOR_NATIVE
            | WM_DROPFILES_NATIVE
            | WM_IME_STARTCOMPOSITION_NATIVE
            | WM_IME_ENDCOMPOSITION_NATIVE
            | WM_IME_COMPOSITION_NATIVE
            | crate::raw::constants::WM_CLOSE => 0,
            _ => unsafe { crate::raw::user32::DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

shackle fn window_open_impl(read title: Str, read width: Int, read height: Int) -> arcana_winapi.desktop_handles.Window = helpers.window.window_open:
    crate::shackle::clear_helper_error(instance);
    if let Err(err) = register_desktop_window_class() {
        crate::shackle::set_helper_error(instance, err);
        return Ok(binding_opaque(0));
    }
    let handle = {
        let state = desktop_state_mut(instance)?;
        let value = state.next_window_handle;
        state.next_window_handle += 1;
        value
    };
    let proc_state = Box::new(DesktopWindowProcState {
        desktop_state_handle: crate::shackle::package_state_data_ref(instance)?.desktop_state_handle,
        handle,
    });
    let proc_ptr = Box::into_raw(proc_state);
    let class_name = desktop_window_class_name();
    let title_wide = wide_nul(&title);
    let module = crate::shackle::current_module_handle_for_address(
        desktop_window_proc as usize as crate::shackle::LPCVOID,
    )?;
    let hwnd = unsafe {
        crate::raw::user32::CreateWindowExW(
            0,
            class_name.as_ptr(),
            title_wide.as_ptr(),
            WS_OVERLAPPEDWINDOW_NATIVE,
            crate::raw::constants::CW_USEDEFAULT,
            crate::raw::constants::CW_USEDEFAULT,
            width as i32,
            height as i32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            module,
            proc_ptr as *mut std::ffi::c_void,
        )
    };
    if hwnd.is_null() {
        unsafe {
            drop(Box::from_raw(proc_ptr));
        }
        crate::shackle::set_helper_error(
            instance,
            format!(
                "CreateWindowExW failed with Win32 error {}",
                unsafe { crate::raw::kernel32::GetLastError() }
            ),
        );
        return Ok(binding_opaque(0));
    }
    let mut rect = unsafe { std::mem::zeroed::<crate::raw::types::RECT>() };
    if unsafe { crate::raw::user32::GetClientRect(hwnd, &mut rect) } == 0 {
        unsafe {
            let _ = crate::raw::user32::DestroyWindow(hwnd);
        }
        crate::shackle::set_helper_error(
            instance,
            "failed to query window client rect".to_string(),
        );
        return Ok(binding_opaque(0));
    }
    let mut pos_rect = unsafe { std::mem::zeroed::<crate::raw::types::RECT>() };
    let _ = unsafe { GetWindowRect(hwnd, &mut pos_rect) };
    desktop_state_mut(instance)?.windows.insert(
        handle,
        WinapiWindowState {
            hwnd,
            title,
            width: (rect.right - rect.left) as i64,
            height: (rect.bottom - rect.top) as i64,
            position: (pos_rect.left as i64, pos_rect.top as i64),
            min_size: (0, 0),
            max_size: (0, 0),
            resized: false,
            fullscreen: false,
            minimized: false,
            maximized: false,
            focused: false,
            visible: false,
            decorated: true,
            resizable: true,
            topmost: false,
            transparent: false,
            theme_override_code: 0,
            cursor_visible: true,
            cursor_icon_code: 0,
            cursor_grab_mode: 0,
            cursor_position: (-1, -1),
            text_input_enabled: false,
            ime_composing: false,
            composition_area_active: false,
            composition_area_position: (0, 0),
            composition_area_size: (0, 0),
            fullscreen_restore_position: (pos_rect.left as i64, pos_rect.top as i64),
            fullscreen_restore_size: ((rect.right - rect.left) as i64, (rect.bottom - rect.top) as i64),
            fullscreen_restore_maximized: false,
            closed: false,
            events: std::collections::VecDeque::new(),
        },
    );
    unsafe {
        crate::raw::user32::DragAcceptFiles(hwnd, 1);
    }
    Ok(binding_opaque(handle))

shackle fn window_take_last_error_impl() -> Str = helpers.window.take_last_error:
    Ok(binding_owned_str(crate::shackle::take_helper_error(instance)))

shackle fn window_alive_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_alive:
    let Ok(window) = window_ref(instance, win) else {
        return Ok(binding_bool(false));
    };
    Ok(binding_bool(!window.closed && !window.hwnd.is_null() && unsafe { IsWindow(window.hwnd) != 0 }))

shackle fn window_native_handle_impl(read win: arcana_winapi.desktop_handles.Window) -> arcana_winapi.raw.types.HWND = helpers.window.window_native_handle:
    Ok(binding_output_layout(window_ref(instance, win)?.hwnd))

shackle fn window_width_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_width:
    Ok(binding_int(window_ref(instance, win)?.width))

shackle fn window_height_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_height:
    Ok(binding_int(window_ref(instance, win)?.height))

shackle fn window_resized_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_resized:
    Ok(binding_bool(window_ref(instance, win)?.resized))

shackle fn window_fullscreen_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_fullscreen:
    Ok(binding_bool(window_ref(instance, win)?.fullscreen))

shackle fn window_minimized_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_minimized:
    Ok(binding_bool(window_ref(instance, win)?.minimized))

shackle fn window_maximized_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_maximized:
    Ok(binding_bool(window_ref(instance, win)?.maximized))

shackle fn window_focused_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_focused:
    Ok(binding_bool(window_ref(instance, win)?.focused))

shackle fn window_id_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_id:
    Ok(binding_int(window_id_value(win)?))

shackle fn window_x_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_x:
    Ok(binding_int(window_ref(instance, win)?.position.0))

shackle fn window_y_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_y:
    Ok(binding_int(window_ref(instance, win)?.position.1))

shackle fn window_title_impl(read win: arcana_winapi.desktop_handles.Window) -> Str = helpers.window.window_title:
    Ok(binding_owned_str(window_ref(instance, win)?.title.clone()))

shackle fn window_visible_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_visible:
    Ok(binding_bool(window_ref(instance, win)?.visible))

shackle fn window_decorated_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_decorated:
    Ok(binding_bool(window_ref(instance, win)?.decorated))

shackle fn window_resizable_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_resizable:
    Ok(binding_bool(window_ref(instance, win)?.resizable))

shackle fn window_topmost_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_topmost:
    Ok(binding_bool(window_ref(instance, win)?.topmost))

shackle fn window_cursor_visible_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_cursor_visible:
    Ok(binding_bool(window_ref(instance, win)?.cursor_visible))

shackle fn window_min_width_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_min_width:
    Ok(binding_int(window_ref(instance, win)?.min_size.0))

shackle fn window_min_height_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_min_height:
    Ok(binding_int(window_ref(instance, win)?.min_size.1))

shackle fn window_max_width_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_max_width:
    Ok(binding_int(window_ref(instance, win)?.max_size.0))

shackle fn window_max_height_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_max_height:
    Ok(binding_int(window_ref(instance, win)?.max_size.1))

shackle fn window_scale_factor_milli_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_scale_factor_milli:
    let window = window_ref(instance, win)?;
    let dpi = unsafe { crate::raw::user32::GetDpiForWindow(window.hwnd) };
    Ok(binding_int(if dpi == 0 { 1000 } else { i64::from(dpi) * 1000 / 96 }))

shackle fn window_theme_code_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_theme_code:
    let window = window_ref(instance, win)?;
    Ok(binding_int(window_theme_code(window.hwnd, window.theme_override_code)))

shackle fn window_transparent_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_transparent:
    Ok(binding_bool(window_ref(instance, win)?.transparent))

shackle fn window_theme_override_code_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_theme_override_code:
    Ok(binding_int(window_ref(instance, win)?.theme_override_code))

shackle fn window_cursor_icon_code_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_cursor_icon_code:
    Ok(binding_int(window_ref(instance, win)?.cursor_icon_code))

shackle fn window_cursor_grab_mode_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_cursor_grab_mode:
    Ok(binding_int(window_ref(instance, win)?.cursor_grab_mode))

shackle fn window_cursor_x_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_cursor_x:
    Ok(binding_int(window_ref(instance, win)?.cursor_position.0))

shackle fn window_cursor_y_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_cursor_y:
    Ok(binding_int(window_ref(instance, win)?.cursor_position.1))

shackle fn window_text_input_enabled_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_text_input_enabled:
    Ok(binding_bool(window_ref(instance, win)?.text_input_enabled))

shackle fn window_current_monitor_index_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_current_monitor_index:
    Ok(binding_int(current_monitor_index_for_window(instance, win)?))

shackle fn window_primary_monitor_index_impl() -> Int = helpers.window.window_primary_monitor_index:
    for (index, monitor) in collect_monitor_infos()?.iter().enumerate() {
        if monitor.primary {
            return Ok(binding_int(
                i64::try_from(index)
                    .map_err(|_| "native monitor index does not fit in Int".to_string())?
            ));
        }
    }
    Ok(binding_int(0))

shackle fn window_monitor_count_impl() -> Int = helpers.window.window_monitor_count:
    Ok(binding_int(
        i64::try_from(collect_monitor_infos()?.len())
            .map_err(|_| "native monitor count does not fit in Int".to_string())?
    ))

shackle fn window_monitor_name_impl(index: Int) -> Str = helpers.window.window_monitor_name:
    if index < 0 {
        return Err(format!("invalid monitor index `{index}`"));
    }
    Ok(binding_owned_str(collect_monitor_infos()?
        .get(index as usize)
        .ok_or_else(|| format!("invalid monitor index `{index}`"))?
        .name
        .clone()))

shackle fn window_monitor_x_impl(index: Int) -> Int = helpers.window.window_monitor_x:
    if index < 0 {
        return Err(format!("invalid monitor index `{index}`"));
    }
    Ok(binding_int(collect_monitor_infos()?
        .get(index as usize)
        .ok_or_else(|| format!("invalid monitor index `{index}`"))?
        .position.0))

shackle fn window_monitor_y_impl(index: Int) -> Int = helpers.window.window_monitor_y:
    if index < 0 {
        return Err(format!("invalid monitor index `{index}`"));
    }
    Ok(binding_int(collect_monitor_infos()?
        .get(index as usize)
        .ok_or_else(|| format!("invalid monitor index `{index}`"))?
        .position.1))

shackle fn window_monitor_width_impl(index: Int) -> Int = helpers.window.window_monitor_width:
    if index < 0 {
        return Err(format!("invalid monitor index `{index}`"));
    }
    Ok(binding_int(collect_monitor_infos()?
        .get(index as usize)
        .ok_or_else(|| format!("invalid monitor index `{index}`"))?
        .size.0))

shackle fn window_monitor_height_impl(index: Int) -> Int = helpers.window.window_monitor_height:
    if index < 0 {
        return Err(format!("invalid monitor index `{index}`"));
    }
    Ok(binding_int(collect_monitor_infos()?
        .get(index as usize)
        .ok_or_else(|| format!("invalid monitor index `{index}`"))?
        .size.1))

shackle fn window_monitor_scale_factor_milli_impl(index: Int) -> Int = helpers.window.window_monitor_scale_factor_milli:
    if index < 0 {
        return Err(format!("invalid monitor index `{index}`"));
    }
    Ok(binding_int(collect_monitor_infos()?
        .get(index as usize)
        .ok_or_else(|| format!("invalid monitor index `{index}`"))?
        .scale_factor_milli))

shackle fn window_monitor_is_primary_impl(index: Int) -> Bool = helpers.window.window_monitor_is_primary:
    if index < 0 {
        return Err(format!("invalid monitor index `{index}`"));
    }
    Ok(binding_bool(collect_monitor_infos()?
        .get(index as usize)
        .ok_or_else(|| format!("invalid monitor index `{index}`"))?
        .primary))

shackle fn window_set_title_impl(edit win: arcana_winapi.desktop_handles.Window, read title: Str) = helpers.window.window_set_title:
    let hwnd = window_ref(instance, win)?.hwnd;
    let title_wide = wide_nul(&title);
    if unsafe { SetWindowTextW(hwnd, title_wide.as_ptr()) } == 0 {
        return Err("failed to set native window title".to_string());
    }
    window_mut(instance, win)?.title = title;
    Ok(binding_unit())

shackle fn window_set_position_impl(edit win: arcana_winapi.desktop_handles.Window, read x: Int, read y: Int) = helpers.window.window_set_position:
    let hwnd = window_ref(instance, win)?.hwnd;
    if unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            x as i32,
            y as i32,
            0,
            0,
            SWP_NOSIZE_NATIVE | SWP_NOACTIVATE_NATIVE | SWP_NOOWNERZORDER_NATIVE,
        )
    } == 0 {
        return Err("failed to move native window".to_string());
    }
    window_mut(instance, win)?.position = (x, y);
    Ok(binding_unit())

shackle fn window_set_size_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.window.window_set_size:
    let (hwnd, position, min_size, max_size) = {
        let window = window_ref(instance, win)?;
        (window.hwnd, window.position, window.min_size, window.max_size)
    };
    let size = set_window_client_bounds_raw(
        hwnd,
        position,
        min_size,
        max_size,
        position,
        (width, height),
    )?;
    let window = window_mut(instance, win)?;
    window.width = size.0;
    window.height = size.1;
    window.resized = true;
    Ok(binding_unit())

shackle fn window_set_visible_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_visible:
    let hwnd = window_ref(instance, win)?.hwnd;
    unsafe {
        let _ = ShowWindow(
            hwnd,
            if enabled { SW_SHOW_NATIVE } else { SW_HIDE_NATIVE },
        );
    }
    window_mut(instance, win)?.visible = enabled;
    Ok(binding_unit())

shackle fn window_set_decorated_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_decorated:
    let (hwnd, fullscreen, resizable, topmost, transparent) = {
        let window = window_ref(instance, win)?;
        (
            window.hwnd,
            window.fullscreen,
            window.resizable,
            window.topmost,
            window.transparent,
        )
    };
    apply_window_frame_style(
        hwnd,
        fullscreen,
        enabled,
        resizable,
        topmost,
        transparent,
    )?;
    window_mut(instance, win)?.decorated = enabled;
    Ok(binding_unit())

shackle fn window_set_resizable_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_resizable:
    let (hwnd, fullscreen, decorated, topmost, transparent) = {
        let window = window_ref(instance, win)?;
        (
            window.hwnd,
            window.fullscreen,
            window.decorated,
            window.topmost,
            window.transparent,
        )
    };
    apply_window_frame_style(
        hwnd,
        fullscreen,
        decorated,
        enabled,
        topmost,
        transparent,
    )?;
    window_mut(instance, win)?.resizable = enabled;
    Ok(binding_unit())

shackle fn window_set_min_size_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.window.window_set_min_size:
    let min_size = (width.max(0), height.max(0));
    let (hwnd, position, max_size, fullscreen, current_size) = {
        let window = window_ref(instance, win)?;
        (
            window.hwnd,
            window.position,
            window.max_size,
            window.fullscreen,
            (window.width, window.height),
        )
    };
    let size = apply_window_size_constraints_raw(
        hwnd,
        position,
        min_size,
        max_size,
        fullscreen,
        current_size,
    )?;
    let window = window_mut(instance, win)?;
    window.min_size = min_size;
    window.width = size.0;
    window.height = size.1;
    window.resized = true;
    Ok(binding_unit())

shackle fn window_set_max_size_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.window.window_set_max_size:
    let max_size = (width.max(0), height.max(0));
    let (hwnd, position, min_size, fullscreen, current_size) = {
        let window = window_ref(instance, win)?;
        (
            window.hwnd,
            window.position,
            window.min_size,
            window.fullscreen,
            (window.width, window.height),
        )
    };
    let size = apply_window_size_constraints_raw(
        hwnd,
        position,
        min_size,
        max_size,
        fullscreen,
        current_size,
    )?;
    let window = window_mut(instance, win)?;
    window.max_size = max_size;
    window.width = size.0;
    window.height = size.1;
    window.resized = true;
    Ok(binding_unit())

shackle fn window_set_fullscreen_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_fullscreen:
    let (
        hwnd,
        fullscreen,
        decorated,
        resizable,
        topmost,
        transparent,
        position,
        min_size,
        max_size,
        current_size,
        maximized,
        restore_position,
        restore_size,
        restore_maximized,
    ) = {
        let window = window_ref(instance, win)?;
        (
            window.hwnd,
            window.fullscreen,
            window.decorated,
            window.resizable,
            window.topmost,
            window.transparent,
            window.position,
            window.min_size,
            window.max_size,
            (window.width, window.height),
            window.maximized,
            window.fullscreen_restore_position,
            window.fullscreen_restore_size,
            window.fullscreen_restore_maximized,
        )
    };
    if fullscreen == enabled {
        return Ok(binding_unit());
    }
    if enabled {
        apply_window_frame_style(
            hwnd,
            true,
            decorated,
            resizable,
            topmost,
            transparent,
        )?;
        let monitor = current_monitor_rect(hwnd)?;
        let z_order = if topmost {
            HWND_TOPMOST_NATIVE as crate::raw::types::HWND
        } else {
            HWND_NOTOPMOST_NATIVE as crate::raw::types::HWND
        };
        if unsafe {
            SetWindowPos(
                hwnd,
                z_order,
                monitor.left,
                monitor.top,
                monitor.right.saturating_sub(monitor.left),
                monitor.bottom.saturating_sub(monitor.top),
                SWP_NOACTIVATE_NATIVE | SWP_NOOWNERZORDER_NATIVE,
            )
        } == 0
        {
            let _ = apply_window_frame_style(
                hwnd,
                false,
                decorated,
                resizable,
                topmost,
                transparent,
            );
            return Err("failed to enter native fullscreen".to_string());
        }
        let window = window_mut(instance, win)?;
        window.fullscreen_restore_position = position;
        window.fullscreen_restore_size = current_size;
        window.fullscreen_restore_maximized = maximized;
        window.fullscreen = true;
        window.minimized = false;
        window.maximized = false;
        window.position = (i64::from(monitor.left), i64::from(monitor.top));
        window.width = i64::from(monitor.right.saturating_sub(monitor.left));
        window.height = i64::from(monitor.bottom.saturating_sub(monitor.top));
        window.resized = true;
        return Ok(binding_unit());
    }
    apply_window_frame_style(
        hwnd,
        false,
        decorated,
        resizable,
        topmost,
        transparent,
    )?;
    if restore_maximized {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE_NATIVE);
            let _ = ShowWindow(hwnd, SW_MAXIMIZE_NATIVE);
        }
        let window = window_mut(instance, win)?;
        window.fullscreen = false;
        window.maximized = true;
        window.minimized = false;
        window.resized = true;
        return Ok(binding_unit());
    }
    let size = set_window_client_bounds_raw(
        hwnd,
        position,
        min_size,
        max_size,
        restore_position,
        restore_size,
    )?;
    let window = window_mut(instance, win)?;
    window.fullscreen = false;
    window.maximized = false;
    window.minimized = false;
    window.position = restore_position;
    window.width = size.0;
    window.height = size.1;
    window.resized = true;
    Ok(binding_unit())

shackle fn window_set_minimized_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_minimized:
    let hwnd = window_ref(instance, win)?.hwnd;
    unsafe {
        let _ = ShowWindow(hwnd, if enabled { SW_MINIMIZE_NATIVE } else { SW_RESTORE_NATIVE });
    }
    window_mut(instance, win)?.minimized = enabled;
    Ok(binding_unit())

shackle fn window_set_maximized_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_maximized:
    let hwnd = window_ref(instance, win)?.hwnd;
    unsafe {
        let _ = ShowWindow(hwnd, if enabled { SW_MAXIMIZE_NATIVE } else { SW_RESTORE_NATIVE });
    }
    window_mut(instance, win)?.maximized = enabled;
    Ok(binding_unit())

shackle fn window_set_topmost_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_topmost:
    let (hwnd, fullscreen, decorated, resizable, transparent) = {
        let window = window_ref(instance, win)?;
        (
            window.hwnd,
            window.fullscreen,
            window.decorated,
            window.resizable,
            window.transparent,
        )
    };
    apply_window_frame_style(
        hwnd,
        fullscreen,
        decorated,
        resizable,
        enabled,
        transparent,
    )?;
    window_mut(instance, win)?.topmost = enabled;
    Ok(binding_unit())

shackle fn window_set_cursor_visible_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_cursor_visible:
    let window = window_mut(instance, win)?;
    window.cursor_visible = enabled;
    apply_window_cursor(window)?;
    Ok(binding_unit())

shackle fn window_set_transparent_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_transparent:
    let (hwnd, fullscreen, decorated, resizable, topmost) = {
        let window = window_ref(instance, win)?;
        (
            window.hwnd,
            window.fullscreen,
            window.decorated,
            window.resizable,
            window.topmost,
        )
    };
    apply_window_frame_style(
        hwnd,
        fullscreen,
        decorated,
        resizable,
        topmost,
        enabled,
    )?;
    window_mut(instance, win)?.transparent = enabled;
    Ok(binding_unit())

shackle fn window_set_theme_override_code_impl(edit win: arcana_winapi.desktop_handles.Window, read code: Int) = helpers.window.window_set_theme_override_code:
    let hwnd = window_ref(instance, win)?.hwnd;
    let enabled = if code == 2 { 1i32 } else { 0i32 };
    let hr = unsafe {
        crate::raw::dwmapi::DwmSetWindowAttribute(
            hwnd,
            crate::raw::constants::DWMWA_USE_IMMERSIVE_DARK_MODE,
            &enabled as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<crate::raw::types::BOOL>() as u32,
        )
    };
    if crate::helpers_impl::hresult_failed_native(hr) {
        return Err(format!("failed to update native theme override (HRESULT {hr})"));
    }
    window_mut(instance, win)?.theme_override_code = code;
    Ok(binding_unit())

shackle fn window_set_cursor_icon_code_impl(edit win: arcana_winapi.desktop_handles.Window, read code: Int) = helpers.window.window_set_cursor_icon_code:
    let window = window_mut(instance, win)?;
    window.cursor_icon_code = code;
    apply_window_cursor(window)?;
    Ok(binding_unit())

shackle fn window_set_cursor_grab_mode_impl(edit win: arcana_winapi.desktop_handles.Window, read mode: Int) = helpers.window.window_set_cursor_grab_mode:
    let window = window_mut(instance, win)?;
    window.cursor_grab_mode = mode;
    apply_cursor_grab(window)?;
    Ok(binding_unit())

shackle fn window_set_cursor_position_impl(edit win: arcana_winapi.desktop_handles.Window, read x: Int, read y: Int) = helpers.window.window_set_cursor_position:
    let hwnd = window_ref(instance, win)?.hwnd;
    if x >= 0 && y >= 0 {
        let point = client_to_screen_point(
            hwnd,
            i32::try_from(x).map_err(|_| format!("cursor x `{x}` does not fit in i32"))?,
            i32::try_from(y).map_err(|_| format!("cursor y `{y}` does not fit in i32"))?,
        )?;
        unsafe {
            let _ = SetCursorPos(point.x, point.y);
        }
    }
    window_mut(instance, win)?.cursor_position = (x, y);
    Ok(binding_unit())

shackle fn window_text_input_set_enabled_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_text_input_enabled:
    let window = window_mut(instance, win)?;
    window.text_input_enabled = enabled;
    apply_text_input(window)?;
    Ok(binding_unit())

shackle fn window_request_redraw_impl(edit win: arcana_winapi.desktop_handles.Window) = helpers.window.window_request_redraw:
    let hwnd = window_ref(instance, win)?.hwnd;
    unsafe {
        let _ = InvalidateRect(hwnd, std::ptr::null(), 0);
    }
    Ok(binding_unit())

shackle fn window_request_attention_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_request_attention:
    let hwnd = window_ref(instance, win)?.hwnd;
    let mut info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd,
        dwFlags: if enabled { FLASHW_ALL_NATIVE } else { FLASHW_STOP_NATIVE },
        uCount: if enabled { 3 } else { 0 },
        dwTimeout: 0,
    };
    if unsafe { FlashWindowEx(&mut info) } == 0 {
        return Err("failed to update native window attention request".to_string());
    }
    Ok(binding_unit())

shackle fn window_close_impl(take win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_close:
    crate::shackle::clear_helper_error(instance);
    let hwnd = match window_ref(instance, win) {
        Ok(window) => window.hwnd,
        Err(err) => {
            crate::shackle::set_helper_error(instance, err);
            return Ok(binding_bool(false));
        }
    };
    if hwnd.is_null() {
        desktop_state_mut(instance)?.windows.remove(&win);
        return Ok(binding_bool(true));
    }
    if unsafe { crate::raw::user32::DestroyWindow(hwnd) } == 0 {
        crate::shackle::set_helper_error(
            instance,
            "failed to close native window".to_string(),
        );
        return Ok(binding_bool(false));
    }
    desktop_state_mut(instance)?.windows.remove(&win);
    Ok(binding_bool(true))

shackle fn events_pump_impl(edit win: arcana_winapi.desktop_handles.Window) -> arcana_winapi.desktop_handles.FrameInput = helpers.events.pump:
    let events = collect_window_events(instance, win)?;
    Ok(binding_opaque(create_frame(instance, events)))

shackle fn events_poll_kind_impl(edit frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_kind:
    let state = frame_mut(instance, frame)?;
    let next = state.events.pop_front();
    let kind = next.as_ref().map(|event| event.kind).unwrap_or(0);
    state.last_polled = next;
    let should_remove = kind == 0;
    let _ = state;
    if should_remove {
        let _ = desktop_state_mut(instance)?.frames.remove(&frame);
    }
    Ok(binding_int(kind))

shackle fn events_poll_window_id_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_window_id:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.window_id).unwrap_or(0)))

shackle fn events_poll_a_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_a:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.a).unwrap_or(0)))

shackle fn events_poll_b_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_b:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.b).unwrap_or(0)))

shackle fn events_poll_flags_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_flags:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.flags).unwrap_or(0)))

shackle fn events_poll_text_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Str = helpers.events.poll_text:
    Ok(binding_owned_str(frame_ref(instance, frame)?
        .last_polled
        .as_ref()
        .map(|event| event.text.clone())
        .unwrap_or_default()))

shackle fn events_poll_key_code_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_key_code:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.key_code).unwrap_or(0)))

shackle fn events_poll_physical_key_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_physical_key:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.physical_key).unwrap_or(0)))

shackle fn events_poll_logical_key_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_logical_key:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.logical_key).unwrap_or(0)))

shackle fn events_poll_key_location_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_key_location:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.key_location).unwrap_or(0)))

shackle fn events_poll_pointer_x_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_pointer_x:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.pointer_x).unwrap_or(0)))

shackle fn events_poll_pointer_y_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.events.poll_pointer_y:
    Ok(binding_int(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.pointer_y).unwrap_or(0)))

shackle fn events_poll_repeated_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Bool = helpers.events.poll_repeated:
    Ok(binding_bool(frame_ref(instance, frame)?.last_polled.as_ref().map(|event| event.repeated).unwrap_or(false)))

shackle fn events_wake_create_impl() -> arcana_winapi.desktop_handles.WakeHandle = helpers.events.wake_create:
    let event = unsafe {
        crate::raw::kernel32::CreateEventW(
            std::ptr::null_mut(),
            1,
            0,
            std::ptr::null(),
        )
    };
    if event.is_null() {
        return Err(format!(
            "CreateEventW failed with Win32 error {}",
            unsafe { crate::raw::kernel32::GetLastError() }
        ));
    }
    let state = desktop_state_mut(instance)?;
    let handle = state.next_wake_handle;
    state.next_wake_handle += 1;
    state.wakes.insert(handle, WinapiWakeState { event, pending: 0 });
    Ok(binding_opaque(handle))

shackle fn events_wake_close_impl(edit handle: arcana_winapi.desktop_handles.WakeHandle) = helpers.events.wake_close:
    let wake = desktop_state_mut(instance)?
        .wakes
        .remove(&handle)
        .ok_or_else(|| format!("invalid WakeHandle `{handle}`"))?;
    if !wake.event.is_null() {
        unsafe {
            let _ = crate::raw::kernel32::CloseHandle(wake.event);
        }
    }
    Ok(binding_unit())

shackle fn events_wake_signal_impl(read handle: arcana_winapi.desktop_handles.WakeHandle) = helpers.events.wake_signal:
    let wake_state = wake_ref(instance, handle)?;
    let event = wake_state.event;
    wake_mut(instance, handle)?.pending += 1;
    if unsafe { crate::raw::kernel32::SetEvent(event) } == 0 {
        return Err(format!(
            "SetEvent failed with Win32 error {}",
            unsafe { crate::raw::kernel32::GetLastError() }
        ));
    }
    Ok(binding_unit())

shackle fn events_wake_take_pending_impl(edit handle: arcana_winapi.desktop_handles.WakeHandle) -> Int = helpers.events.wake_take_pending:
    let wake = wake_mut(instance, handle)?;
    let pending = wake.pending;
    wake.pending = 0;
    if unsafe { crate::raw::kernel32::ResetEvent(wake.event) } == 0 {
        return Err(format!(
            "ResetEvent failed with Win32 error {}",
            unsafe { crate::raw::kernel32::GetLastError() }
        ));
    }
    Ok(binding_int(
        i64::try_from(pending)
            .map_err(|_| format!("wake pending count `{pending}` does not fit in Int"))?
    ))

shackle fn events_wait_wake_or_messages_impl(read handle: arcana_winapi.desktop_handles.WakeHandle, read timeout_ms: Int) -> Bool = helpers.events.wait_wake_or_messages:
    Ok(binding_bool(wait_for_wake_or_messages(instance, handle, timeout_ms)?))

shackle fn input_key_code_impl(read name: Str) -> Int = helpers.input.input_key_code:
    Ok(binding_int(named_key_code(&name)))

shackle fn input_key_down_impl(read frame: arcana_winapi.desktop_handles.FrameInput, read key: Int) -> Bool = helpers.input.input_key_down:
    Ok(binding_bool(frame_ref(instance, frame)?.input.key_down.contains(&key)))

shackle fn input_key_pressed_impl(read frame: arcana_winapi.desktop_handles.FrameInput, read key: Int) -> Bool = helpers.input.input_key_pressed:
    Ok(binding_bool(frame_ref(instance, frame)?.input.key_pressed.contains(&key)))

shackle fn input_key_released_impl(read frame: arcana_winapi.desktop_handles.FrameInput, read key: Int) -> Bool = helpers.input.input_key_released:
    Ok(binding_bool(frame_ref(instance, frame)?.input.key_released.contains(&key)))

shackle fn input_mouse_button_code_impl(read name: Str) -> Int = helpers.input.input_mouse_button_code:
    Ok(binding_int(named_mouse_button_code(&name)))

shackle fn input_mouse_x_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.input.input_mouse_x:
    Ok(binding_int(frame_ref(instance, frame)?.input.mouse_pos.0))

shackle fn input_mouse_y_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.input.input_mouse_y:
    Ok(binding_int(frame_ref(instance, frame)?.input.mouse_pos.1))

shackle fn input_mouse_down_impl(read frame: arcana_winapi.desktop_handles.FrameInput, read button: Int) -> Bool = helpers.input.input_mouse_down:
    Ok(binding_bool(frame_ref(instance, frame)?.input.mouse_down.contains(&button)))

shackle fn input_mouse_pressed_impl(read frame: arcana_winapi.desktop_handles.FrameInput, read button: Int) -> Bool = helpers.input.input_mouse_pressed:
    Ok(binding_bool(frame_ref(instance, frame)?.input.mouse_pressed.contains(&button)))

shackle fn input_mouse_released_impl(read frame: arcana_winapi.desktop_handles.FrameInput, read button: Int) -> Bool = helpers.input.input_mouse_released:
    Ok(binding_bool(frame_ref(instance, frame)?.input.mouse_released.contains(&button)))

shackle fn input_mouse_wheel_y_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Int = helpers.input.input_mouse_wheel_y:
    Ok(binding_int(frame_ref(instance, frame)?.input.mouse_wheel_y))

shackle fn input_mouse_in_window_impl(read frame: arcana_winapi.desktop_handles.FrameInput) -> Bool = helpers.input.input_mouse_in_window:
    Ok(binding_bool(frame_ref(instance, frame)?.input.mouse_in_window))

shackle fn text_input_enabled_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.text_input.window_text_input_enabled:
    Ok(binding_bool(window_ref(instance, win)?.text_input_enabled))

shackle fn text_input_set_enabled_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.text_input.window_set_text_input_enabled:
    let window = window_mut(instance, win)?;
    window.text_input_enabled = enabled;
    apply_text_input(window)?;
    Ok(binding_unit())

shackle fn text_input_composition_area_active_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.text_input.composition_area_active:
    Ok(binding_bool(window_ref(instance, win)?.composition_area_active))

shackle fn text_input_composition_area_x_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.text_input.composition_area_x:
    Ok(binding_int(window_ref(instance, win)?.composition_area_position.0))

shackle fn text_input_composition_area_y_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.text_input.composition_area_y:
    Ok(binding_int(window_ref(instance, win)?.composition_area_position.1))

shackle fn text_input_composition_area_width_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.text_input.composition_area_width:
    Ok(binding_int(window_ref(instance, win)?.composition_area_size.0))

shackle fn text_input_composition_area_height_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.text_input.composition_area_height:
    Ok(binding_int(window_ref(instance, win)?.composition_area_size.1))

shackle fn text_input_set_composition_area_position_raw_impl(edit win: arcana_winapi.desktop_handles.Window, read x: Int, read y: Int) = helpers.text_input.set_composition_area_position:
    let window = window_mut(instance, win)?;
    window.composition_area_active = true;
    window.composition_area_position = (x, y);
    apply_composition_area(window)?;
    Ok(binding_unit())

shackle fn text_input_set_composition_area_size_raw_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.text_input.set_composition_area_size:
    let window = window_mut(instance, win)?;
    window.composition_area_active = true;
    window.composition_area_size = (width.max(0), height.max(0));
    apply_composition_area(window)?;
    Ok(binding_unit())

shackle fn text_input_clear_composition_area_impl(edit win: arcana_winapi.desktop_handles.Window) = helpers.text_input.clear_composition_area:
    let window = window_mut(instance, win)?;
    window.composition_area_active = false;
    window.composition_area_position = (0, 0);
    window.composition_area_size = (0, 0);
    apply_composition_area(window)?;
    Ok(binding_unit())

