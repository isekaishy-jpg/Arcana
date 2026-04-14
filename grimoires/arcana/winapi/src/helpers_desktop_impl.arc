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
    pub(crate) const EVENT_WINDOW_SCALE_FACTOR_CHANGED: i64 = 16;
    pub(crate) const EVENT_WINDOW_THEME_CHANGED: i64 = 17;
    pub(crate) const EVENT_RAW_MOUSE_MOTION: i64 = 18;
    pub(crate) const EVENT_RAW_MOUSE_BUTTON: i64 = 19;
    pub(crate) const EVENT_APP_RESUMED: i64 = 20;
    pub(crate) const EVENT_WAKE: i64 = 21;
    pub(crate) const EVENT_RAW_MOUSE_WHEEL: i64 = 28;
    pub(crate) const EVENT_RAW_KEY: i64 = 29;

    pub(crate) const DEVICE_EVENTS_WHEN_FOCUSED: i64 = 1;

    pub(crate) const WM_DESTROY_NATIVE: u32 = 2;
    pub(crate) const WM_MOVE_NATIVE: u32 = 3;
    pub(crate) const WM_SIZE_NATIVE: u32 = 5;
    pub(crate) const WM_SETFOCUS_NATIVE: u32 = 7;
    pub(crate) const WM_KILLFOCUS_NATIVE: u32 = 8;
    pub(crate) const WM_PAINT_NATIVE: u32 = 15;
    pub(crate) const WM_CHAR_NATIVE: u32 = 258;
    pub(crate) const WM_THEMECHANGED_NATIVE: u32 = 794;
    pub(crate) const WM_DPICHANGED_NATIVE: u32 = 736;
    pub(crate) const WM_NCDESTROY_NATIVE: u32 = 130;

    pub(crate) const WS_OVERLAPPEDWINDOW_NATIVE: u32 = 0x00CF0000;
    pub(crate) const SW_HIDE_NATIVE: i32 = 0;
    pub(crate) const SW_SHOW_NATIVE: i32 = 5;
    pub(crate) const SW_MINIMIZE_NATIVE: i32 = 6;
    pub(crate) const SW_MAXIMIZE_NATIVE: i32 = 3;
    pub(crate) const SW_RESTORE_NATIVE: i32 = 9;
    pub(crate) const SWP_NOSIZE_NATIVE: u32 = 0x0001;
    pub(crate) const SWP_NOMOVE_NATIVE: u32 = 0x0002;
    pub(crate) const SWP_NOACTIVATE_NATIVE: u32 = 0x0010;
    pub(crate) const SWP_NOOWNERZORDER_NATIVE: u32 = 0x0200;
    pub(crate) const MONITOR_DEFAULTTONEAREST_NATIVE: u32 = 2;

    #[repr(C)]
    pub(crate) struct PAINTSTRUCT {
        pub(crate) hdc: crate::raw::types::HDC,
        pub(crate) fErase: crate::raw::types::BOOL,
        pub(crate) rcPaint: crate::raw::types::RECT,
        pub(crate) fRestore: crate::raw::types::BOOL,
        pub(crate) fIncUpdate: crate::raw::types::BOOL,
        pub(crate) rgbReserved: [u8; 32],
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
        pub(crate) composition_area_active: bool,
        pub(crate) composition_area_position: (i64, i64),
        pub(crate) composition_area_size: (i64, i64),
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

    pub(crate) struct WinapiSessionState {
        pub(crate) windows: Vec<u64>,
        pub(crate) pending_wakes: usize,
        pub(crate) resumed_sent: bool,
        pub(crate) device_events_policy: i64,
    }

    pub(crate) struct WinapiWakeState {
        pub(crate) session: u64,
    }

    pub(crate) struct WinapiDesktopState {
        pub(crate) next_window_handle: u64,
        pub(crate) windows: std::collections::BTreeMap<u64, WinapiWindowState>,
        pub(crate) next_frame_handle: u64,
        pub(crate) frames: std::collections::BTreeMap<u64, WinapiFrameState>,
        pub(crate) next_session_handle: u64,
        pub(crate) sessions: std::collections::BTreeMap<u64, WinapiSessionState>,
        pub(crate) next_wake_handle: u64,
        pub(crate) wakes: std::collections::BTreeMap<u64, WinapiWakeState>,
    }

    pub(crate) struct DesktopWindowProcState {
        pub(crate) instance: *mut crate::BindingInstance,
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
        pub(crate) fn SetCursorPos(x: i32, y: i32) -> i32;
    }

    pub(crate) fn new_desktop_state_handle() -> u64 {
        let state = Box::new(WinapiDesktopState {
            next_window_handle: 1,
            windows: std::collections::BTreeMap::new(),
            next_frame_handle: 1,
            frames: std::collections::BTreeMap::new(),
            next_session_handle: 1,
            sessions: std::collections::BTreeMap::new(),
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

    pub(crate) fn session_ref(
        instance: &crate::BindingInstance,
        handle: u64,
    ) -> Result<&WinapiSessionState, String> {
        desktop_state_ref(instance)?
            .sessions
            .get(&handle)
            .ok_or_else(|| format!("invalid Session handle `{handle}`"))
    }

    pub(crate) fn session_mut(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<&mut WinapiSessionState, String> {
        desktop_state_mut(instance)?
            .sessions
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Session handle `{handle}`"))
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

    pub(crate) fn make_unit_event(kind: i64) -> PendingDesktopEvent {
        PendingDesktopEvent {
            kind,
            ..PendingDesktopEvent::default()
        }
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

    pub(crate) fn prune_dead_session_windows(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<(), String> {
        let live = {
            let state = desktop_state_ref(instance)?;
            let session = state
                .sessions
                .get(&handle)
                .ok_or_else(|| format!("invalid Session handle `{handle}`"))?;
            session
                .windows
                .iter()
                .copied()
                .filter(|window| {
                    state.windows
                        .get(window)
                        .map(|entry| !entry.closed && !entry.hwnd.is_null())
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>()
        };
        session_mut(instance, handle)?.windows = live;
        Ok(())
    }

    pub(crate) fn collect_session_events(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<std::collections::VecDeque<PendingDesktopEvent>, String> {
        process_pending_messages();
        prune_dead_session_windows(instance, handle)?;
        let session_windows = session_ref(instance, handle)?.windows.clone();
        let mut events = std::collections::VecDeque::new();
        {
            let session = session_mut(instance, handle)?;
            if !session.resumed_sent {
                events.push_back(make_unit_event(EVENT_APP_RESUMED));
                session.resumed_sent = true;
            }
            while session.pending_wakes > 0 {
                session.pending_wakes -= 1;
                events.push_back(make_unit_event(EVENT_WAKE));
            }
        }
        for window in session_windows {
            let drained = drain_window_events(instance, window)?;
            for event in drained {
                events.push_back(event);
            }
        }
        Ok(events)
    }

    pub(crate) fn collect_window_events(
        instance: &mut crate::BindingInstance,
        handle: u64,
    ) -> Result<std::collections::VecDeque<PendingDesktopEvent>, String> {
        process_pending_messages();
        drain_window_events(instance, handle)
    }

    pub(crate) fn wait_for_session_events(
        instance: &mut crate::BindingInstance,
        handle: u64,
        timeout_ms: i64,
    ) -> Result<std::collections::VecDeque<PendingDesktopEvent>, String> {
        let deadline = if timeout_ms < 0 {
            None
        } else {
            Some(std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms as u64))
        };
        loop {
            let events = collect_session_events(instance, handle)?;
            if !events.is_empty() {
                return Ok(events);
            }
            match deadline {
                Some(value) if std::time::Instant::now() >= value => {
                    return Ok(std::collections::VecDeque::new());
                }
                _ => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
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
        let instance = unsafe { &mut *(*proc_state).instance };
        let handle = unsafe { (*proc_state).handle };
        let _ = (|| -> Result<(), String> {
            match message {
                WM_SIZE_NATIVE => {
                    let width = (lparam as u32 & 0xFFFF) as i64;
                    let height = ((lparam as u32 >> 16) & 0xFFFF) as i64;
                    let window = window_mut(instance, handle)?;
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
                    let window = window_mut(instance, handle)?;
                    window.position = (x, y);
                    window
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_MOVED, x, y)?);
                }
                WM_SETFOCUS_NATIVE => {
                    let window = window_mut(instance, handle)?;
                    window.focused = true;
                    window
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_FOCUSED, 1, 0)?);
                }
                WM_KILLFOCUS_NATIVE => {
                    let window = window_mut(instance, handle)?;
                    window.focused = false;
                    window
                        .events
                        .push_back(make_window_event(handle, EVENT_WINDOW_FOCUSED, 0, 0)?);
                }
                WM_CHAR_NATIVE => {
                    if let Some(ch) = char::from_u32(wparam as u32) {
                        let mut event = make_window_event(handle, EVENT_TEXT_INPUT, 0, 0)?;
                        event.text = ch.to_string();
                        push_window_event(instance, handle, event)?;
                    }
                }
                WM_PAINT_NATIVE => {
                    let mut paint = unsafe { std::mem::zeroed::<PAINTSTRUCT>() };
                    unsafe {
                        let _ = BeginPaint(hwnd, &mut paint);
                        let _ = EndPaint(hwnd, &paint);
                    }
                    push_window_event(
                        instance,
                        handle,
                        make_window_event(handle, EVENT_WINDOW_REDRAW_REQUESTED, 0, 0)?,
                    )?;
                    return Ok(());
                }
                WM_THEMECHANGED_NATIVE => {
                    push_window_event(
                        instance,
                        handle,
                        make_window_event(handle, EVENT_WINDOW_THEME_CHANGED, 0, 0)?,
                    )?;
                }
                crate::raw::constants::WM_CLOSE => {
                    push_window_event(
                        instance,
                        handle,
                        make_window_event(handle, EVENT_WINDOW_CLOSE_REQUESTED, 0, 0)?,
                    )?;
                    return Ok(());
                }
                WM_DESTROY_NATIVE | WM_NCDESTROY_NATIVE => {
                    if let Ok(window) = window_mut(instance, handle) {
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
                    push_window_event(
                        instance,
                        handle,
                        make_window_event(handle, EVENT_WINDOW_SCALE_FACTOR_CHANGED, scale, 0)?,
                    )?;
                }
                _ => {}
            }
            Ok(())
        })();
        match message {
            WM_PAINT_NATIVE | crate::raw::constants::WM_CLOSE => 0,
            _ => unsafe { crate::raw::user32::DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

shackle fn window_open_impl(read title: Str, read width: Int, read height: Int) -> arcana_winapi.desktop_handles.Window = helpers.window.window_open:
    crate::shackle::clear_helper_error(instance);
    if let Err(err) = register_desktop_window_class() {
        crate::shackle::set_helper_error(instance, err);
        return Ok(binding_int(0));
    }
    let handle = {
        let state = desktop_state_mut(instance)?;
        let value = state.next_window_handle;
        state.next_window_handle += 1;
        value
    };
    let proc_state = Box::new(DesktopWindowProcState {
        instance: instance as *mut crate::BindingInstance,
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
        return Ok(binding_int(0));
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
        return Ok(binding_int(0));
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
            visible: true,
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
            composition_area_active: false,
            composition_area_position: (0, 0),
            composition_area_size: (0, 0),
            closed: false,
            events: std::collections::VecDeque::new(),
        },
    );
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW_NATIVE);
    }
    Ok(binding_int(handle as i64))

shackle fn window_take_last_error_impl() -> Str = helpers.window.take_last_error:
    Ok(binding_owned_str(crate::shackle::take_helper_error(instance)))

shackle fn window_alive_impl(read win: arcana_winapi.desktop_handles.Window) -> Bool = helpers.window.window_alive:
    let Ok(window) = window_ref(instance, win) else {
        return Ok(binding_bool(false));
    };
    Ok(binding_bool(!window.closed && !window.hwnd.is_null() && unsafe { IsWindow(window.hwnd) != 0 }))

shackle fn window_native_handle_impl(read win: arcana_winapi.desktop_handles.Window) -> Int = helpers.window.window_native_handle:
    Ok(binding_int(window_ref(instance, win)?.hwnd as isize as i64))

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
    let _ = win;
    Ok(binding_int(0))

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
    let window = window_mut(instance, win)?;
    if unsafe {
        SetWindowPos(
            window.hwnd,
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
    window.position = (x, y);
    window
        .events
        .push_back(make_window_event(win, EVENT_WINDOW_MOVED, x, y)?);
    Ok(binding_unit())

shackle fn window_set_size_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.window.window_set_size:
    let window = window_mut(instance, win)?;
    if unsafe {
        SetWindowPos(
            window.hwnd,
            std::ptr::null_mut(),
            0,
            0,
            width as i32,
            height as i32,
            SWP_NOMOVE_NATIVE | SWP_NOACTIVATE_NATIVE | SWP_NOOWNERZORDER_NATIVE,
        )
    } == 0 {
        return Err("failed to resize native window".to_string());
    }
    window.width = width.max(0);
    window.height = height.max(0);
    window.resized = true;
    window
        .events
        .push_back(make_window_event(win, EVENT_WINDOW_RESIZED, window.width, window.height)?);
    Ok(binding_unit())

shackle fn window_set_visible_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_visible:
    let window = window_mut(instance, win)?;
    unsafe {
        let _ = ShowWindow(
            window.hwnd,
            if enabled { SW_SHOW_NATIVE } else { SW_HIDE_NATIVE },
        );
    }
    window.visible = enabled;
    Ok(binding_unit())

shackle fn window_set_decorated_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_decorated:
    window_mut(instance, win)?.decorated = enabled;
    Ok(binding_unit())

shackle fn window_set_resizable_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_resizable:
    window_mut(instance, win)?.resizable = enabled;
    Ok(binding_unit())

shackle fn window_set_min_size_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.window.window_set_min_size:
    window_mut(instance, win)?.min_size = (width.max(0), height.max(0));
    Ok(binding_unit())

shackle fn window_set_max_size_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.window.window_set_max_size:
    window_mut(instance, win)?.max_size = (width.max(0), height.max(0));
    Ok(binding_unit())

shackle fn window_set_fullscreen_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_fullscreen:
    let window = window_mut(instance, win)?;
    window.fullscreen = enabled;
    if enabled {
        window.maximized = true;
        unsafe {
            let _ = ShowWindow(window.hwnd, SW_MAXIMIZE_NATIVE);
        }
    } else {
        unsafe {
            let _ = ShowWindow(window.hwnd, SW_RESTORE_NATIVE);
        }
    }
    Ok(binding_unit())

shackle fn window_set_minimized_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_minimized:
    let window = window_mut(instance, win)?;
    window.minimized = enabled;
    unsafe {
        let _ = ShowWindow(window.hwnd, if enabled { SW_MINIMIZE_NATIVE } else { SW_RESTORE_NATIVE });
    }
    Ok(binding_unit())

shackle fn window_set_maximized_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_maximized:
    let window = window_mut(instance, win)?;
    window.maximized = enabled;
    unsafe {
        let _ = ShowWindow(window.hwnd, if enabled { SW_MAXIMIZE_NATIVE } else { SW_RESTORE_NATIVE });
    }
    Ok(binding_unit())

shackle fn window_set_topmost_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_topmost:
    window_mut(instance, win)?.topmost = enabled;
    Ok(binding_unit())

shackle fn window_set_cursor_visible_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_cursor_visible:
    window_mut(instance, win)?.cursor_visible = enabled;
    Ok(binding_unit())

shackle fn window_set_transparent_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_transparent:
    window_mut(instance, win)?.transparent = enabled;
    Ok(binding_unit())

shackle fn window_set_theme_override_code_impl(edit win: arcana_winapi.desktop_handles.Window, read code: Int) = helpers.window.window_set_theme_override_code:
    window_mut(instance, win)?.theme_override_code = code;
    Ok(binding_unit())

shackle fn window_set_cursor_icon_code_impl(edit win: arcana_winapi.desktop_handles.Window, read code: Int) = helpers.window.window_set_cursor_icon_code:
    window_mut(instance, win)?.cursor_icon_code = code;
    Ok(binding_unit())

shackle fn window_set_cursor_grab_mode_impl(edit win: arcana_winapi.desktop_handles.Window, read mode: Int) = helpers.window.window_set_cursor_grab_mode:
    window_mut(instance, win)?.cursor_grab_mode = mode;
    Ok(binding_unit())

shackle fn window_set_cursor_position_impl(edit win: arcana_winapi.desktop_handles.Window, read x: Int, read y: Int) = helpers.window.window_set_cursor_position:
    let window = window_mut(instance, win)?;
    window.cursor_position = (x, y);
    unsafe {
        let _ = SetCursorPos(x as i32, y as i32);
    }
    Ok(binding_unit())

shackle fn window_text_input_set_enabled_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_set_text_input_enabled:
    window_mut(instance, win)?.text_input_enabled = enabled;
    Ok(binding_unit())

shackle fn window_request_redraw_impl(edit win: arcana_winapi.desktop_handles.Window) = helpers.window.window_request_redraw:
    let hwnd = window_ref(instance, win)?.hwnd;
    push_window_event(
        instance,
        win,
        make_window_event(win, EVENT_WINDOW_REDRAW_REQUESTED, 0, 0)?,
    )?;
    unsafe {
        let _ = InvalidateRect(hwnd, std::ptr::null(), 0);
    }
    Ok(binding_unit())

shackle fn window_request_attention_impl(edit win: arcana_winapi.desktop_handles.Window, read enabled: Bool) = helpers.window.window_request_attention:
    let _ = enabled;
    let _ = window_ref(instance, win)?;
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
        return Ok(binding_bool(true));
    }
    if unsafe { crate::raw::user32::DestroyWindow(hwnd) } == 0 {
        crate::shackle::set_helper_error(
            instance,
            "failed to close native window".to_string(),
        );
        return Ok(binding_bool(false));
    }
    if let Ok(window) = window_mut(instance, win) {
        window.closed = true;
        window.hwnd = std::ptr::null_mut();
    }
    Ok(binding_bool(true))

shackle fn events_pump_impl(edit win: arcana_winapi.desktop_handles.Window) -> arcana_winapi.desktop_handles.FrameInput = helpers.events.pump:
    let events = collect_window_events(instance, win)?;
    Ok(binding_int(create_frame(instance, events) as i64))

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

shackle fn events_session_open_impl() -> arcana_winapi.desktop_handles.Session = helpers.events.session_open:
    let state = desktop_state_mut(instance)?;
    let handle = state.next_session_handle;
    state.next_session_handle += 1;
    state.sessions.insert(
        handle,
        WinapiSessionState {
            windows: Vec::new(),
            pending_wakes: 0,
            resumed_sent: false,
            device_events_policy: DEVICE_EVENTS_WHEN_FOCUSED,
        },
    );
    Ok(binding_int(handle as i64))

shackle fn events_session_close_impl(edit session: arcana_winapi.desktop_handles.Session) = helpers.events.session_close:
    let state = desktop_state_mut(instance)?;
    state.sessions.remove(&session);
    state.wakes.retain(|_, wake| wake.session != session);
    Ok(binding_unit())

shackle fn events_session_attach_window_impl(edit session: arcana_winapi.desktop_handles.Session, read win: arcana_winapi.desktop_handles.Window) = helpers.events.session_attach_window:
    let session = session_mut(instance, session)?;
    if !session.windows.contains(&win) {
        session.windows.push(win);
    }
    Ok(binding_unit())

shackle fn events_session_detach_window_impl(edit session: arcana_winapi.desktop_handles.Session, read win: arcana_winapi.desktop_handles.Window) = helpers.events.session_detach_window:
    session_mut(instance, session)?.windows.retain(|value| *value != win);
    Ok(binding_unit())

shackle fn events_session_window_for_id_impl(read session: arcana_winapi.desktop_handles.Session, read window_id: Int) -> arcana_winapi.desktop_handles.Window = helpers.events.session_window_for_id:
    if window_id <= 0 {
        return Ok(binding_int(0));
    }
    prune_dead_session_windows(instance, session)?;
    let wanted = u64::try_from(window_id).map_err(|_| format!("invalid window id `{window_id}`"))?;
    let session_state = session_ref(instance, session)?;
    if session_state.windows.contains(&wanted) && window_ref(instance, wanted).is_ok() {
        return Ok(binding_int(wanted as i64));
    }
    Ok(binding_int(0))

shackle fn events_session_window_count_impl(read session: arcana_winapi.desktop_handles.Session) -> Int = helpers.events.session_window_count:
    prune_dead_session_windows(instance, session)?;
    Ok(binding_int(
        i64::try_from(session_ref(instance, session)?.windows.len())
            .map_err(|_| "session window count does not fit in Int".to_string())?
    ))

shackle fn events_session_window_id_at_impl(read session: arcana_winapi.desktop_handles.Session, read index: Int) -> Int = helpers.events.session_window_id_at:
    if index < 0 {
        return Err(format!("invalid session window index `{index}`"));
    }
    prune_dead_session_windows(instance, session)?;
    let handle = *session_ref(instance, session)?
        .windows
        .get(index as usize)
        .ok_or_else(|| format!("invalid session window index `{index}`"))?;
    Ok(binding_int(window_id_value(handle)?))

shackle fn events_session_device_events_impl(edit session: arcana_winapi.desktop_handles.Session) -> Int = helpers.events.session_device_events:
    Ok(binding_int(session_ref(instance, session)?.device_events_policy))

shackle fn events_session_set_device_events_impl(edit session: arcana_winapi.desktop_handles.Session, read policy: Int) = helpers.events.session_set_device_events:
    session_mut(instance, session)?.device_events_policy = policy;
    Ok(binding_unit())

shackle fn events_session_pump_impl(edit session: arcana_winapi.desktop_handles.Session) -> arcana_winapi.desktop_handles.FrameInput = helpers.events.session_pump:
    let events = collect_session_events(instance, session)?;
    Ok(binding_int(create_frame(instance, events) as i64))

shackle fn events_session_wait_impl(edit session: arcana_winapi.desktop_handles.Session, read timeout_ms: Int) -> arcana_winapi.desktop_handles.FrameInput = helpers.events.session_wait:
    let events = wait_for_session_events(instance, session, timeout_ms)?;
    Ok(binding_int(create_frame(instance, events) as i64))

shackle fn events_session_create_wake_impl(edit session: arcana_winapi.desktop_handles.Session) -> arcana_winapi.desktop_handles.WakeHandle = helpers.events.session_create_wake:
    let state = desktop_state_mut(instance)?;
    let handle = state.next_wake_handle;
    state.next_wake_handle += 1;
    state.wakes.insert(handle, WinapiWakeState { session });
    Ok(binding_int(handle as i64))

shackle fn events_wake_signal_impl(read handle: arcana_winapi.desktop_handles.WakeHandle) = helpers.events.wake_signal:
    let session = wake_ref(instance, handle)?.session;
    session_mut(instance, session)?.pending_wakes += 1;
    Ok(binding_unit())

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
    window_mut(instance, win)?.text_input_enabled = enabled;
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
    Ok(binding_unit())

shackle fn text_input_set_composition_area_size_raw_impl(edit win: arcana_winapi.desktop_handles.Window, read width: Int, read height: Int) = helpers.text_input.set_composition_area_size:
    let window = window_mut(instance, win)?;
    window.composition_area_active = true;
    window.composition_area_size = (width, height);
    Ok(binding_unit())

shackle fn text_input_clear_composition_area_impl(edit win: arcana_winapi.desktop_handles.Window) = helpers.text_input.clear_composition_area:
    let window = window_mut(instance, win)?;
    window.composition_area_active = false;
    window.composition_area_position = (0, 0);
    window.composition_area_size = (0, 0);
    Ok(binding_unit())

