#![allow(clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::c_void;
use std::io::{self, BufRead, Write};
use std::mem::{size_of, zeroed};
use std::path::Path;
use std::ptr::null_mut;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows_sys::Win32::Devices::HumanInterfaceDevice::{
    HID_USAGE_GENERIC_MOUSE, HID_USAGE_PAGE_GENERIC,
};
use windows_sys::Win32::Foundation::{
    GlobalFree, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, ClientToScreen, DIB_RGB_COLORS, EndPaint,
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MONITORINFOEXW, MonitorFromWindow, PAINTSTRUCT, ReleaseDC, SRCCOPY, StretchDIBits,
};
use windows_sys::Win32::Media::Audio::{
    HWAVEOUT, WAVE_FORMAT_PCM, WAVE_FORMAT_QUERY, WAVE_MAPPER, WAVEFORMATEX, WAVEHDR,
    WHDR_BEGINLOOP, WHDR_DONE, WHDR_ENDLOOP, waveOutClose, waveOutGetErrorTextW,
    waveOutGetPosition, waveOutOpen, waveOutPause, waveOutPrepareHeader, waveOutReset,
    waveOutRestart, waveOutSetVolume, waveOutUnprepareHeader, waveOutWrite,
};
use windows_sys::Win32::Media::{MMSYSERR_NOERROR, MMTIME, TIME_SAMPLES};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleHandleExW,
};
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::Input::Ime::{
    CANDIDATEFORM, CFS_CANDIDATEPOS, CFS_FORCE_POSITION, COMPOSITIONFORM, GCS_COMPSTR,
    GCS_CURSORPOS, GCS_RESULTSTR, HIMC, ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext,
    ImmSetCandidateWindow, ImmSetCompositionWindow,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MAPVK_VK_TO_CHAR, MAPVK_VSC_TO_VK_EX, MapVirtualKeyW, TME_LEAVE, TRACKMOUSEEVENT,
    TrackMouseEvent,
};
use windows_sys::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, MOUSE_MOVE_RELATIVE, RAWINPUT, RAWINPUTDEVICE, RID_INPUT,
    RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE, RegisterRawInputDevices,
};
use windows_sys::Win32::UI::Shell::{DragAcceptFiles, DragFinish, DragQueryFileW, HDROP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, ClipCursor,
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FLASHW_CAPTION, FLASHW_STOP,
    FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GWL_EXSTYLE, GWL_STYLE,
    GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, GetWindowRect, HCURSOR, HTCLIENT,
    HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM,
    IDC_NO, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT, IsWindow,
    LWA_ALPHA, LoadCursorW, MINMAXINFO, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx,
    PM_NOREMOVE, PM_REMOVE, PeekMessageW, PostMessageW, QS_ALLINPUT, RegisterClassW,
    SIZE_MAXIMIZED, SIZE_MINIMIZED, SW_HIDE, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SetCursor,
    SetCursorPos, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, SetWindowTextW,
    ShowWindow, TranslateMessage, WM_CHAR, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_DROPFILES,
    WM_GETMINMAXINFO, WM_IME_COMPOSITION, WM_IME_ENDCOMPOSITION, WM_IME_STARTCOMPOSITION, WM_INPUT,
    WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOVE, WM_NCCREATE, WM_NCDESTROY, WM_NULL, WM_PAINT,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETFOCUS, WM_SIZE, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_THEMECHANGED, WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW, WS_EX_APPWINDOW, WS_EX_LAYERED,
    WS_MAXIMIZEBOX, WS_OVERLAPPEDWINDOW, WS_SIZEBOX, WaitMessage,
};

use crate::{
    BufferedAppFrame, BufferedEvent, BufferedFrameInput, BufferedHost, RuntimeAppFrameHandle,
    RuntimeAppSessionHandle, RuntimeAudioBufferHandle, RuntimeAudioDeviceHandle,
    RuntimeAudioPlaybackHandle, RuntimeEventRecord, RuntimeHost, RuntimeImageHandle,
    RuntimeWakeHandle, RuntimeWindowHandle, common_named_key_code, common_named_mouse_button_code,
    ensure_audio_buffer_matches_device, text_engine,
};

const WINDOW_CLASS_NAME: &str = "ArcanaNativeRuntimeWindow";
const EVENT_WINDOW_RESIZED: i64 = 1;
const EVENT_WINDOW_CLOSE_REQUESTED: i64 = 2;
const EVENT_WINDOW_FOCUSED: i64 = 3;
const EVENT_KEY_DOWN: i64 = 4;
const EVENT_KEY_UP: i64 = 5;
const EVENT_MOUSE_DOWN: i64 = 6;
const EVENT_MOUSE_UP: i64 = 7;
const EVENT_MOUSE_MOVE: i64 = 8;
const EVENT_MOUSE_WHEEL_Y: i64 = 9;
const EVENT_WINDOW_MOVED: i64 = 10;
const EVENT_MOUSE_ENTERED: i64 = 11;
const EVENT_MOUSE_LEFT: i64 = 12;
const EVENT_WINDOW_REDRAW_REQUESTED: i64 = 13;
const EVENT_TEXT_INPUT: i64 = 14;
const EVENT_FILE_DROPPED: i64 = 15;
const EVENT_WINDOW_SCALE_FACTOR_CHANGED: i64 = 16;
const EVENT_WINDOW_THEME_CHANGED: i64 = 17;
const EVENT_RAW_MOUSE_MOTION: i64 = 18;
const EVENT_RAW_MOUSE_BUTTON: i64 = 19;
const EVENT_APP_RESUMED: i64 = 20;
const EVENT_WAKE: i64 = 21;
const EVENT_APP_SUSPENDED: i64 = 22;
const EVENT_ABOUT_TO_WAIT: i64 = 23;
const EVENT_TEXT_COMPOSITION_STARTED: i64 = 24;
const EVENT_TEXT_COMPOSITION_UPDATED: i64 = 25;
const EVENT_TEXT_COMPOSITION_COMMITTED: i64 = 26;
const EVENT_TEXT_COMPOSITION_CANCELLED: i64 = 27;
const EVENT_RAW_MOUSE_WHEEL: i64 = 28;
const EVENT_RAW_KEY: i64 = 29;
const DEVICE_EVENTS_NEVER: i64 = 0;
const DEVICE_EVENTS_WHEN_FOCUSED: i64 = 1;
const DEVICE_EVENTS_ALWAYS: i64 = 2;
const RAW_MOUSE_LEFT_BUTTON_DOWN: u16 = 0x0001;
const RAW_MOUSE_LEFT_BUTTON_UP: u16 = 0x0002;
const RAW_MOUSE_RIGHT_BUTTON_DOWN: u16 = 0x0004;
const RAW_MOUSE_RIGHT_BUTTON_UP: u16 = 0x0008;
const RAW_MOUSE_MIDDLE_BUTTON_DOWN: u16 = 0x0010;
const RAW_MOUSE_MIDDLE_BUTTON_UP: u16 = 0x0020;
const RAW_MOUSE_BUTTON_4_DOWN: u16 = 0x0040;
const RAW_MOUSE_BUTTON_4_UP: u16 = 0x0080;
const RAW_MOUSE_BUTTON_5_DOWN: u16 = 0x0100;
const RAW_MOUSE_BUTTON_5_UP: u16 = 0x0200;
const RAW_MOUSE_WHEEL: u16 = 0x0400;
const RAW_MOUSE_HWHEEL: u16 = 0x0800;
const RAW_KEY_BREAK: u16 = 0x0001;
const RAW_KEY_E0: u16 = 0x0002;
const WM_MOUSELEAVE_MESSAGE: u32 = 0x02A3;
const ARCANA_BYTES_CLIPBOARD_FORMAT_NAME: &str = "ArcanaRuntimeBytes";

static REGISTERED_WINDOW_CLASS: OnceLock<Result<NativeWindowClass, String>> = OnceLock::new();
static REGISTERED_BYTES_CLIPBOARD_FORMAT: OnceLock<Result<u32, String>> = OnceLock::new();
static PENDING_RAW_INPUT_EVENTS: OnceLock<Mutex<Vec<PendingRawInputEvent>>> = OnceLock::new();

struct NativeWindowClass {
    module_handle: usize,
    name: Vec<u16>,
}

struct ClipboardGuard;

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingRawInputEvent {
    MouseMotion {
        device_id: i64,
        dx: i64,
        dy: i64,
    },
    MouseButton {
        device_id: i64,
        button: i64,
        pressed: bool,
    },
    MouseWheel {
        device_id: i64,
        dx: i64,
        dy: i64,
    },
    Key {
        device_id: i64,
        key_code: i64,
        physical_key: i64,
        logical_key: i64,
        key_location: i64,
        pressed: bool,
        text: String,
    },
}

impl ClipboardGuard {
    fn open() -> Result<Self, String> {
        if unsafe { OpenClipboard(null_mut()) } == 0 {
            return Err("failed to open Windows clipboard".to_string());
        }
        Ok(Self)
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

fn pending_raw_input_events() -> &'static Mutex<Vec<PendingRawInputEvent>> {
    PENDING_RAW_INPUT_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn desktop_temp_probe_enabled() -> bool {
    std::env::var_os("ARCANA_DESKTOP_TEMP_PROBES").is_some()
}

fn desktop_temp_probe(label: &str, detail: String) {
    if desktop_temp_probe_enabled() {
        eprintln!("[arcana-desktop-probe] {label}: {detail}");
    }
}

fn push_pending_raw_input_event(
    pending: &mut Vec<PendingRawInputEvent>,
    event: PendingRawInputEvent,
) {
    match event {
        PendingRawInputEvent::MouseMotion { device_id, dx, dy } => {
            if let Some(PendingRawInputEvent::MouseMotion {
                device_id: last_device_id,
                dx: last_dx,
                dy: last_dy,
            }) = pending.last_mut()
                && *last_device_id == device_id
            {
                *last_dx = last_dx.saturating_add(dx);
                *last_dy = last_dy.saturating_add(dy);
                return;
            }
            pending.push(PendingRawInputEvent::MouseMotion { device_id, dx, dy });
        }
        other => pending.push(other),
    }
}

fn attention_flash_flags(enabled: bool) -> u32 {
    if enabled {
        FLASHW_CAPTION | FLASHW_TRAY | FLASHW_TIMERNOFG
    } else {
        FLASHW_STOP
    }
}

pub struct NativeProcessHost {
    base: BufferedHost,
    started: Instant,
    next_window_handle: u64,
    windows: BTreeMap<RuntimeWindowHandle, Box<NativeWindowState>>,
    next_image_handle: u64,
    images: BTreeMap<RuntimeImageHandle, NativeImage>,
    next_frame_handle: u64,
    frames: BTreeMap<RuntimeAppFrameHandle, BufferedAppFrame>,
    next_session_handle: u64,
    sessions: BTreeMap<RuntimeAppSessionHandle, NativeSessionState>,
    next_wake_handle: u64,
    wakes: BTreeMap<RuntimeWakeHandle, NativeWakeState>,
    next_audio_device_handle: u64,
    audio_devices: BTreeMap<RuntimeAudioDeviceHandle, NativeAudioDevice>,
    next_audio_buffer_handle: u64,
    audio_buffers: BTreeMap<RuntimeAudioBufferHandle, NativeAudioBuffer>,
    next_audio_playback_handle: u64,
    audio_playbacks: BTreeMap<RuntimeAudioPlaybackHandle, NativeAudioPlayback>,
}

struct NativeWindowState {
    hwnd: HWND,
    title: String,
    width: i64,
    height: i64,
    min_size: (i64, i64),
    max_size: (i64, i64),
    resized: bool,
    fullscreen: bool,
    minimized: bool,
    maximized: bool,
    focused: bool,
    visible: bool,
    decorated: bool,
    resizable: bool,
    topmost: bool,
    transparent: bool,
    theme_override_code: i64,
    cursor_visible: bool,
    cursor_icon_code: i64,
    cursor_grab_mode: i64,
    cursor_position: (i64, i64),
    suppress_cursor_move: bool,
    message_loop_signaled: bool,
    attention_requested: bool,
    redraw_pending: bool,
    text_input_enabled: bool,
    composition_area_active: bool,
    composition_area_position: (i64, i64),
    composition_area_size: (i64, i64),
    composition_active: bool,
    composition_committed: bool,
    applying_dpi_suggestion: bool,
    applying_theme_override: bool,
    closed: bool,
    restore_style: isize,
    restore_ex_style: isize,
    restore_rect: RECT,
    surface: CanvasSurface,
    events: VecDeque<BufferedEvent>,
    key_down: BTreeSet<i64>,
    key_pressed: BTreeSet<i64>,
    key_released: BTreeSet<i64>,
    mouse_pos: (i64, i64),
    mouse_down: BTreeSet<i64>,
    mouse_pressed: BTreeSet<i64>,
    mouse_released: BTreeSet<i64>,
    mouse_wheel_y: i64,
    mouse_in_window: bool,
    pending_high_surrogate: Option<u16>,
}

struct NativeSessionState {
    windows: Vec<RuntimeWindowHandle>,
    resumed: bool,
    suspended: bool,
    pending_wakes: usize,
    device_events_policy: i64,
}

struct NativeWakeState {
    session: RuntimeAppSessionHandle,
}

#[derive(Clone, Debug)]
struct NativeMonitorInfo {
    handle: HMONITOR,
    name: String,
    position: (i64, i64),
    size: (i64, i64),
    scale_factor_milli: i64,
    primary: bool,
}

#[derive(Clone, Debug)]
struct NativeImage {
    width: i64,
    height: i64,
    pixels: Vec<u32>,
}

struct CanvasSurface {
    width: i64,
    height: i64,
    pixels: Vec<u32>,
}

struct NativeAudioDevice {
    sample_rate_hz: i64,
    channels: i64,
    gain_milli: i64,
}

struct NativeAudioBuffer {
    frames: i64,
    channels: i64,
    sample_rate_hz: i64,
    pcm_bytes: Vec<u8>,
}

struct NativeAudioPlayback {
    device: RuntimeAudioDeviceHandle,
    wave_out: HWAVEOUT,
    header: Box<WAVEHDR>,
    pcm_bytes: Vec<u8>,
    frames: i64,
    paused: bool,
    finished: bool,
    gain_milli: i64,
    looping: bool,
    header_prepared: bool,
}

impl NativeProcessHost {
    pub fn current() -> Result<Self, String> {
        let base = BufferedHost {
            args: std::env::args().skip(1).collect(),
            env: std::env::vars().collect(),
            allow_process: true,
            cwd: std::env::current_dir()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
            ..Default::default()
        };
        Ok(Self {
            base,
            started: Instant::now(),
            next_window_handle: 0,
            windows: BTreeMap::new(),
            next_image_handle: 0,
            images: BTreeMap::new(),
            next_frame_handle: 0,
            frames: BTreeMap::new(),
            next_session_handle: 0,
            sessions: BTreeMap::new(),
            next_wake_handle: 0,
            wakes: BTreeMap::new(),
            next_audio_device_handle: 0,
            audio_devices: BTreeMap::new(),
            next_audio_buffer_handle: 0,
            audio_buffers: BTreeMap::new(),
            next_audio_playback_handle: 0,
            audio_playbacks: BTreeMap::new(),
        })
    }

    fn insert_window(&mut self, window: Box<NativeWindowState>) -> RuntimeWindowHandle {
        let handle = RuntimeWindowHandle(self.next_window_handle);
        self.next_window_handle += 1;
        self.windows.insert(handle, window);
        handle
    }

    fn window_state_live(window: &NativeWindowState) -> bool {
        !window.closed && !window.hwnd.is_null() && unsafe { IsWindow(window.hwnd) != 0 }
    }

    fn window_state_ref(&self, handle: RuntimeWindowHandle) -> Result<&NativeWindowState, String> {
        self.windows
            .get(&handle)
            .map(Box::as_ref)
            .ok_or_else(|| format!("invalid Window handle `{}`", handle.0))
    }

    fn window_ref(&self, handle: RuntimeWindowHandle) -> Result<&NativeWindowState, String> {
        let window = self.window_state_ref(handle)?;
        if Self::window_state_live(window) {
            Ok(window)
        } else {
            Err(format!("invalid Window handle `{}`", handle.0))
        }
    }

    fn window_state_mut(
        &mut self,
        handle: RuntimeWindowHandle,
    ) -> Result<&mut NativeWindowState, String> {
        self.windows
            .get_mut(&handle)
            .map(Box::as_mut)
            .ok_or_else(|| format!("invalid Window handle `{}`", handle.0))
    }

    fn window_mut(
        &mut self,
        handle: RuntimeWindowHandle,
    ) -> Result<&mut NativeWindowState, String> {
        let window = self.window_state_mut(handle)?;
        if Self::window_state_live(window) {
            Ok(window)
        } else {
            Err(format!("invalid Window handle `{}`", handle.0))
        }
    }

    fn insert_image(&mut self, image: NativeImage) -> RuntimeImageHandle {
        let handle = RuntimeImageHandle(self.next_image_handle);
        self.next_image_handle += 1;
        self.images.insert(handle, image);
        handle
    }

    fn image_ref(&self, handle: RuntimeImageHandle) -> Result<&NativeImage, String> {
        self.images
            .get(&handle)
            .ok_or_else(|| format!("invalid Image handle `{}`", handle.0))
    }

    fn image_mut(&mut self, handle: RuntimeImageHandle) -> Result<&mut NativeImage, String> {
        self.images
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Image handle `{}`", handle.0))
    }

    fn insert_frame(&mut self, frame: BufferedAppFrame) -> RuntimeAppFrameHandle {
        let handle = RuntimeAppFrameHandle(self.next_frame_handle);
        self.next_frame_handle += 1;
        self.frames.insert(handle, frame);
        handle
    }

    fn frame_ref(&self, handle: RuntimeAppFrameHandle) -> Result<&BufferedAppFrame, String> {
        self.frames
            .get(&handle)
            .ok_or_else(|| format!("invalid AppFrame handle `{}`", handle.0))
    }

    fn frame_mut(
        &mut self,
        handle: RuntimeAppFrameHandle,
    ) -> Result<&mut BufferedAppFrame, String> {
        self.frames
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AppFrame handle `{}`", handle.0))
    }

    fn insert_session(&mut self) -> RuntimeAppSessionHandle {
        let handle = RuntimeAppSessionHandle(self.next_session_handle);
        self.next_session_handle += 1;
        self.sessions.insert(
            handle,
            NativeSessionState {
                windows: Vec::new(),
                resumed: false,
                suspended: false,
                pending_wakes: 0,
                device_events_policy: DEVICE_EVENTS_WHEN_FOCUSED,
            },
        );
        handle
    }

    fn session_ref(&self, handle: RuntimeAppSessionHandle) -> Result<&NativeSessionState, String> {
        self.sessions
            .get(&handle)
            .ok_or_else(|| format!("invalid AppSession handle `{}`", handle.0))
    }

    fn session_mut(
        &mut self,
        handle: RuntimeAppSessionHandle,
    ) -> Result<&mut NativeSessionState, String> {
        self.sessions
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AppSession handle `{}`", handle.0))
    }

    fn detach_window_from_sessions(&mut self, window: RuntimeWindowHandle) {
        let mut affected = Vec::new();
        for (session, state) in &mut self.sessions {
            let before = state.windows.len();
            state.windows.retain(|candidate| *candidate != window);
            if state.windows.len() != before {
                affected.push(*session);
            }
        }
        for session in affected {
            self.notify_session_queue(session);
        }
    }

    fn remove_session_wakes(&mut self, session: RuntimeAppSessionHandle) {
        self.wakes.retain(|_, wake| wake.session != session);
    }

    fn prune_closed_windows(&mut self) {
        let closed = self
            .windows
            .iter()
            .filter_map(|(handle, state)| {
                if (state.closed || state.hwnd.is_null()) && state.events.is_empty() {
                    Some(*handle)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        if closed.is_empty() {
            return;
        }
        for session in self.sessions.values_mut() {
            session
                .windows
                .retain(|candidate| !closed.contains(candidate));
        }
        self.windows.retain(|handle, _| !closed.contains(handle));
    }

    fn close_native_window(
        &mut self,
        window: RuntimeWindowHandle,
        pump_after_destroy: bool,
    ) -> Result<(), String> {
        if !self.windows.contains_key(&window) {
            return Err(format!("invalid Window handle `{}`", window.0));
        }
        let (hwnd, keep_attached) = {
            let window_state = self.window_state_mut(window)?;
            if window_state.closed || window_state.hwnd.is_null() {
                self.detach_window_from_sessions(window);
                self.prune_closed_windows();
                return Ok(());
            }
            window_state.closed = true;
            (window_state.hwnd, !window_state.events.is_empty())
        };
        if !keep_attached {
            self.detach_window_from_sessions(window);
        }
        if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
            unsafe {
                DestroyWindow(hwnd);
            }
            if pump_after_destroy {
                self.pump_messages()?;
            }
        }
        self.prune_closed_windows();
        self.sync_process_raw_input_registration()?;
        Ok(())
    }

    fn insert_wake(&mut self, session: RuntimeAppSessionHandle) -> RuntimeWakeHandle {
        let handle = RuntimeWakeHandle(self.next_wake_handle);
        self.next_wake_handle += 1;
        self.wakes.insert(handle, NativeWakeState { session });
        handle
    }

    fn wake_ref(&self, handle: RuntimeWakeHandle) -> Result<&NativeWakeState, String> {
        self.wakes
            .get(&handle)
            .ok_or_else(|| format!("invalid Wake handle `{}`", handle.0))
    }

    fn insert_audio_device(
        &mut self,
        sample_rate_hz: i64,
        channels: i64,
    ) -> RuntimeAudioDeviceHandle {
        let handle = RuntimeAudioDeviceHandle(self.next_audio_device_handle);
        self.next_audio_device_handle += 1;
        self.audio_devices.insert(
            handle,
            NativeAudioDevice {
                sample_rate_hz,
                channels,
                gain_milli: 1000,
            },
        );
        handle
    }

    fn audio_device_ref(
        &self,
        handle: RuntimeAudioDeviceHandle,
    ) -> Result<&NativeAudioDevice, String> {
        self.audio_devices
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioDevice handle `{}`", handle.0))
    }

    fn audio_device_mut(
        &mut self,
        handle: RuntimeAudioDeviceHandle,
    ) -> Result<&mut NativeAudioDevice, String> {
        self.audio_devices
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AudioDevice handle `{}`", handle.0))
    }

    fn insert_audio_buffer(&mut self, buffer: NativeAudioBuffer) -> RuntimeAudioBufferHandle {
        let handle = RuntimeAudioBufferHandle(self.next_audio_buffer_handle);
        self.next_audio_buffer_handle += 1;
        self.audio_buffers.insert(handle, buffer);
        handle
    }

    fn audio_buffer_ref(
        &self,
        handle: RuntimeAudioBufferHandle,
    ) -> Result<&NativeAudioBuffer, String> {
        self.audio_buffers
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioBuffer handle `{}`", handle.0))
    }

    fn insert_audio_playback(
        &mut self,
        playback: NativeAudioPlayback,
    ) -> RuntimeAudioPlaybackHandle {
        let handle = RuntimeAudioPlaybackHandle(self.next_audio_playback_handle);
        self.next_audio_playback_handle += 1;
        self.audio_playbacks.insert(handle, playback);
        handle
    }

    fn audio_playback_ref(
        &self,
        handle: RuntimeAudioPlaybackHandle,
    ) -> Result<&NativeAudioPlayback, String> {
        self.audio_playbacks
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioPlayback handle `{}`", handle.0))
    }

    fn audio_playback_mut(
        &mut self,
        handle: RuntimeAudioPlaybackHandle,
    ) -> Result<&mut NativeAudioPlayback, String> {
        self.audio_playbacks
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AudioPlayback handle `{}`", handle.0))
    }

    fn ensure_window_class() -> Result<&'static NativeWindowClass, String> {
        match REGISTERED_WINDOW_CLASS.get_or_init(|| {
            let module_handle = current_module_handle()?;
            let name = native_window_class_name(module_handle);
            let cursor = unsafe { LoadCursorW(null_mut(), IDC_ARROW) };
            let class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(arcana_window_proc),
                hInstance: module_handle as HINSTANCE,
                hCursor: cursor as HCURSOR,
                lpszClassName: name.as_ptr(),
                ..unsafe { zeroed() }
            };
            let atom = unsafe { RegisterClassW(&class) };
            if atom == 0 {
                return Err(format!(
                    "failed to register Arcana native runtime window class `{}`",
                    native_window_class_name_text(module_handle)
                ));
            }
            Ok(NativeWindowClass {
                module_handle,
                name,
            })
        }) {
            Ok(class) => Ok(class),
            Err(err) => Err(err.clone()),
        }
    }

    fn open_native_window(
        &mut self,
        title: &str,
        width: i64,
        height: i64,
    ) -> Result<RuntimeWindowHandle, String> {
        let window_class = Self::ensure_window_class()?;
        let client_width = sanitize_dimension(width);
        let client_height = sanitize_dimension(height);
        let style = WS_OVERLAPPEDWINDOW;
        let ex_style = WS_EX_APPWINDOW;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: client_width as i32,
            bottom: client_height as i32,
        };
        unsafe {
            AdjustWindowRectEx(&mut rect, style, 0, ex_style);
        }
        let mut window = Box::new(NativeWindowState::new(title, client_width, client_height));
        let window_ptr = (&mut *window) as *mut NativeWindowState;
        let title_wide = wide_null(title);
        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                window_class.name.as_ptr(),
                title_wide.as_ptr(),
                style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                null_mut(),
                null_mut(),
                window_class.module_handle as HINSTANCE,
                window_ptr as *mut c_void,
            )
        };
        if hwnd.is_null() {
            return Err("failed to create native window".to_string());
        }
        if unsafe { SetWindowTextW(hwnd, title_wide.as_ptr()) } == 0 {
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err("failed to publish native window title".to_string());
        }
        unsafe {
            DragAcceptFiles(hwnd, 1);
        }
        let handle = self.insert_window(window);
        self.pump_messages()?;
        Ok(handle)
    }

    fn pump_messages(&mut self) -> Result<(), String> {
        let mut msg = unsafe { zeroed::<MSG>() };
        loop {
            let has_message = unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) };
            if has_message == 0 {
                break;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        self.dispatch_pending_raw_input_events()?;
        self.sync_process_raw_input_registration()?;
        Ok(())
    }

    fn collect_monitor_infos(&self) -> Result<Vec<NativeMonitorInfo>, String> {
        let mut handles = Vec::new();
        let ok = unsafe {
            EnumDisplayMonitors(
                null_mut(),
                std::ptr::null(),
                Some(collect_monitor_handle_proc),
                &mut handles as *mut Vec<HMONITOR> as LPARAM,
            )
        };
        if ok == 0 {
            return Err("failed to enumerate native monitors".to_string());
        }
        let mut monitors = Vec::with_capacity(handles.len());
        for handle in handles {
            let mut info = unsafe { zeroed::<MONITORINFOEXW>() };
            info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            if unsafe {
                GetMonitorInfoW(handle, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO)
            } == 0
            {
                return Err("failed to query native monitor info".to_string());
            }
            let scale_factor_milli = query_monitor_scale_factor_milli(handle);
            let position = (
                info.monitorInfo.rcMonitor.left as i64,
                info.monitorInfo.rcMonitor.top as i64,
            );
            let size = (
                (info.monitorInfo.rcMonitor.right - info.monitorInfo.rcMonitor.left) as i64,
                (info.monitorInfo.rcMonitor.bottom - info.monitorInfo.rcMonitor.top) as i64,
            );
            monitors.push(NativeMonitorInfo {
                handle,
                name: wide_units_to_string(&info.szDevice),
                position,
                size,
                scale_factor_milli,
                primary: info.monitorInfo.dwFlags & 1 != 0,
            });
        }
        Ok(monitors)
    }

    fn monitor_info_at(&self, index: i64) -> Result<NativeMonitorInfo, String> {
        if index < 0 {
            return Err(format!("invalid monitor index `{index}`"));
        }
        let monitors = self.collect_monitor_infos()?;
        monitors
            .get(index as usize)
            .cloned()
            .ok_or_else(|| format!("invalid monitor index `{index}`"))
    }

    fn current_monitor_index_for_window(&self, window: &NativeWindowState) -> Result<i64, String> {
        let monitor = unsafe { MonitorFromWindow(window.hwnd, MONITOR_DEFAULTTONEAREST) };
        for (index, info) in self.collect_monitor_infos()?.iter().enumerate() {
            if info.handle == monitor {
                return i64::try_from(index)
                    .map_err(|_| "native monitor index does not fit in Int".to_string());
            }
        }
        Ok(0)
    }

    fn session_has_ready_events(&self, session: RuntimeAppSessionHandle) -> Result<bool, String> {
        let session_state = self.session_ref(session)?;
        if !session_state.resumed || session_state.pending_wakes > 0 {
            return Ok(true);
        }
        let mut live_windows = 0usize;
        for window in &session_state.windows {
            if let Some(state) = self.windows.get(window) {
                if !Self::window_state_live(state) {
                    continue;
                }
                if !state.events.is_empty() {
                    return Ok(true);
                }
                live_windows += 1;
            }
        }
        if live_windows == 0 && session_state.resumed && !session_state.suspended {
            return Ok(true);
        }
        Ok(false)
    }

    fn wait_for_session_activity(
        &mut self,
        session: RuntimeAppSessionHandle,
        timeout_ms: i64,
    ) -> Result<(), String> {
        let started_wait = Instant::now();
        let mut blocked = false;
        let deadline = (timeout_ms >= 0).then(|| {
            Instant::now()
                .checked_add(Duration::from_millis(
                    u64::try_from(timeout_ms).unwrap_or(u64::MAX),
                ))
                .unwrap_or_else(Instant::now)
        });
        loop {
            if self.session_has_ready_events(session)? {
                let waited_ms = started_wait.elapsed().as_millis();
                if blocked && waited_ms >= 16 {
                    desktop_temp_probe(
                        "wait-session",
                        format!("session {:?} ready after {} ms", session, waited_ms),
                    );
                }
                return Ok(());
            }
            let live_window_count = self
                .session_ref(session)?
                .windows
                .iter()
                .filter_map(|handle| self.windows.get(handle))
                .filter(|window| Self::window_state_live(window))
                .count();
            if live_window_count == 0 {
                return Ok(());
            }
            let mut queued = unsafe { zeroed::<MSG>() };
            if unsafe { PeekMessageW(&mut queued, null_mut(), 0, 0, PM_NOREMOVE) } != 0 {
                self.pump_messages()?;
                continue;
            }
            if let Some(deadline) = deadline {
                let now = Instant::now();
                if now >= deadline {
                    desktop_temp_probe(
                        "wait-session",
                        format!("session {:?} timed out before readiness", session),
                    );
                    return Ok(());
                }
                let remaining = deadline.saturating_duration_since(now);
                let timeout = u32::try_from(remaining.as_millis()).unwrap_or(u32::MAX);
                let wait_result = unsafe {
                    MsgWaitForMultipleObjectsEx(
                        0,
                        std::ptr::null(),
                        timeout,
                        QS_ALLINPUT,
                        MWMO_INPUTAVAILABLE,
                    )
                };
                blocked = true;
                if wait_result == u32::MAX {
                    return Err("failed to wait for native session message".to_string());
                }
                if wait_result == windows_sys::Win32::Foundation::WAIT_TIMEOUT {
                    desktop_temp_probe(
                        "wait-session",
                        format!("session {:?} wait timeout expired", session),
                    );
                    return Ok(());
                }
            } else if unsafe { WaitMessage() } == 0 {
                return Err("failed to wait for native session message".to_string());
            } else {
                blocked = true;
            }
            self.pump_messages()?;
        }
    }

    fn notify_session_queue(&mut self, session: RuntimeAppSessionHandle) {
        let session_windows = self
            .sessions
            .get(&session)
            .map(|state| state.windows.clone())
            .unwrap_or_default();
        for window in session_windows {
            let Some(state) = self.windows.get_mut(&window).map(Box::as_mut) else {
                continue;
            };
            if !Self::window_state_live(state) {
                continue;
            }
            state.signal_message_loop();
            break;
        }
    }

    fn desired_raw_input_registration(&self) -> Option<(RuntimeWindowHandle, u32)> {
        let mut always_target = None;
        let mut focused_target = None;
        for session in self.sessions.values() {
            let wants_always = session.device_events_policy == DEVICE_EVENTS_ALWAYS;
            let wants_focused = session.device_events_policy == DEVICE_EVENTS_WHEN_FOCUSED;
            if !wants_always && !wants_focused {
                continue;
            }
            for window in &session.windows {
                let Some(state) = self.windows.get(window) else {
                    continue;
                };
                if !Self::window_state_live(state) {
                    continue;
                }
                if wants_always {
                    if state.focused {
                        return Some((*window, RIDEV_INPUTSINK));
                    }
                    if always_target.is_none() {
                        always_target = Some(*window);
                    }
                } else if state.focused && focused_target.is_none() {
                    focused_target = Some(*window);
                }
            }
        }
        if let Some(window) = always_target {
            return Some((window, RIDEV_INPUTSINK));
        }
        if let Some(window) = focused_target {
            return Some((window, 0));
        }
        None
    }

    fn session_raw_input_target(
        &self,
        session: &NativeSessionState,
    ) -> Option<RuntimeWindowHandle> {
        let mut first_live = None;
        for window in &session.windows {
            let Some(state) = self.windows.get(window) else {
                continue;
            };
            if !Self::window_state_live(state) {
                continue;
            }
            if state.focused {
                return Some(*window);
            }
            if first_live.is_none() {
                first_live = Some(*window);
            }
        }
        match session.device_events_policy {
            DEVICE_EVENTS_ALWAYS => first_live,
            DEVICE_EVENTS_WHEN_FOCUSED => None,
            _ => None,
        }
    }

    fn dispatch_pending_raw_input_events(&mut self) -> Result<(), String> {
        let pending = {
            let mut pending = pending_raw_input_events()
                .lock()
                .map_err(|_| "failed to lock pending raw input queue".to_string())?;
            std::mem::take(&mut *pending)
        };
        if pending.is_empty() {
            return Ok(());
        }
        let targets = self
            .sessions
            .values()
            .filter_map(|session| self.session_raw_input_target(session))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if targets.is_empty() {
            return Ok(());
        }
        if pending.len() >= 64 || targets.len() > 1 {
            let mut motion = 0usize;
            let mut button = 0usize;
            let mut wheel = 0usize;
            let mut key = 0usize;
            for event in &pending {
                match event {
                    PendingRawInputEvent::MouseMotion { .. } => motion += 1,
                    PendingRawInputEvent::MouseButton { .. } => button += 1,
                    PendingRawInputEvent::MouseWheel { .. } => wheel += 1,
                    PendingRawInputEvent::Key { .. } => key += 1,
                }
            }
            desktop_temp_probe(
                "raw-input",
                format!(
                    "dispatching {} event(s) to {:?} [motion={}, button={}, wheel={}, key={}]",
                    pending.len(),
                    targets,
                    motion,
                    button,
                    wheel,
                    key
                ),
            );
        }
        for event in pending {
            for window in &targets {
                let Ok(state) = self.window_mut(*window) else {
                    continue;
                };
                match &event {
                    PendingRawInputEvent::MouseMotion { device_id, dx, dy } => {
                        state.push_device_event(EVENT_RAW_MOUSE_MOTION, *device_id, *dx, *dy);
                    }
                    PendingRawInputEvent::MouseButton {
                        device_id,
                        button,
                        pressed,
                    } => {
                        state.push_device_event(
                            EVENT_RAW_MOUSE_BUTTON,
                            *device_id,
                            *button,
                            if *pressed { 1 } else { 0 },
                        );
                    }
                    PendingRawInputEvent::MouseWheel { device_id, dx, dy } => {
                        state.push_device_event(EVENT_RAW_MOUSE_WHEEL, *device_id, *dx, *dy);
                    }
                    PendingRawInputEvent::Key {
                        device_id,
                        key_code,
                        physical_key,
                        logical_key,
                        key_location,
                        pressed,
                        text,
                    } => {
                        state.push_raw_key_event(
                            *device_id,
                            *key_code,
                            *physical_key,
                            *logical_key,
                            *key_location,
                            *pressed,
                            text.clone(),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn sync_process_raw_input_registration(&mut self) -> Result<(), String> {
        let registration = self.desired_raw_input_registration();
        match registration {
            Some((window, flags)) => {
                let hwnd = self.window_ref(window)?.hwnd;
                Self::register_raw_input_target(hwnd, flags)
            }
            None => Self::register_raw_input_target(null_mut(), RIDEV_REMOVE),
        }
    }

    fn register_raw_input_target(hwnd: HWND, flags: u32) -> Result<(), String> {
        let devices = [
            RAWINPUTDEVICE {
                usUsagePage: HID_USAGE_PAGE_GENERIC,
                usUsage: HID_USAGE_GENERIC_MOUSE,
                dwFlags: flags,
                hwndTarget: if flags == RIDEV_REMOVE {
                    null_mut()
                } else {
                    hwnd
                },
            },
            RAWINPUTDEVICE {
                usUsagePage: HID_USAGE_PAGE_GENERIC,
                usUsage: 0x06,
                dwFlags: flags,
                hwndTarget: if flags == RIDEV_REMOVE {
                    null_mut()
                } else {
                    hwnd
                },
            },
        ];
        if unsafe {
            RegisterRawInputDevices(
                devices.as_ptr(),
                devices.len() as u32,
                size_of::<RAWINPUTDEVICE>() as u32,
            )
        } == 0
        {
            return Err("failed to register native raw input".to_string());
        }
        Ok(())
    }

    fn snapshot_frame_input(window: &mut NativeWindowState) -> BufferedFrameInput {
        let input = BufferedFrameInput {
            key_down: window.key_down.iter().copied().collect(),
            key_pressed: window.key_pressed.iter().copied().collect(),
            key_released: window.key_released.iter().copied().collect(),
            modifiers: frame_modifier_flags(&window.key_down),
            mouse_pos: window.mouse_pos,
            mouse_down: window.mouse_down.iter().copied().collect(),
            mouse_pressed: window.mouse_pressed.iter().copied().collect(),
            mouse_released: window.mouse_released.iter().copied().collect(),
            mouse_wheel_y: window.mouse_wheel_y,
            mouse_in_window: window.mouse_in_window,
        };
        window.key_pressed.clear();
        window.key_released.clear();
        window.mouse_pressed.clear();
        window.mouse_released.clear();
        window.mouse_wheel_y = 0;
        window.resized = false;
        input
    }

    fn event_with_window_id(window: RuntimeWindowHandle, event: BufferedEvent) -> BufferedEvent {
        if crate::buffered_device_event_kind(event.kind) {
            return event;
        }
        BufferedEvent {
            window_id: i64::try_from(window.0).unwrap_or(-1),
            ..event
        }
    }

    fn merge_frame_input(target: &mut BufferedFrameInput, input: BufferedFrameInput) {
        append_unique(&mut target.key_down, input.key_down);
        append_unique(&mut target.key_pressed, input.key_pressed);
        append_unique(&mut target.key_released, input.key_released);
        append_unique(&mut target.mouse_down, input.mouse_down);
        append_unique(&mut target.mouse_pressed, input.mouse_pressed);
        append_unique(&mut target.mouse_released, input.mouse_released);
        target.modifiers |= input.modifiers;
        target.mouse_pos = input.mouse_pos;
        target.mouse_wheel_y += input.mouse_wheel_y;
        target.mouse_in_window |= input.mouse_in_window;
    }

    fn present_window(&mut self, handle: RuntimeWindowHandle) -> Result<(), String> {
        let window = self.window_ref(handle)?;
        if window.closed || window.hwnd.is_null() {
            return Ok(());
        }
        let dc = unsafe { windows_sys::Win32::Graphics::Gdi::GetDC(window.hwnd) };
        if dc.is_null() {
            return Err(format!(
                "failed to acquire device context for `{}`",
                window.title
            ));
        }
        let result = blit_surface_to_dc(&window.surface, window.hwnd, dc);
        unsafe {
            ReleaseDC(window.hwnd, dc);
        }
        result
    }

    fn select_default_audio_config() -> Result<(i64, i64), String> {
        const CANDIDATES: &[(i64, i64)] = &[(48_000, 2), (44_100, 2), (48_000, 1), (44_100, 1)];
        for &(sample_rate_hz, channels) in CANDIDATES {
            let format = pcm_wave_format(sample_rate_hz, channels)?;
            let status =
                unsafe { waveOutOpen(null_mut(), WAVE_MAPPER, &format, 0, 0, WAVE_FORMAT_QUERY) };
            if status == MMSYSERR_NOERROR {
                return Ok((sample_rate_hz, channels));
            }
        }
        Err("failed to locate a compatible default audio output".to_string())
    }

    fn open_audio_playback(
        &mut self,
        device: RuntimeAudioDeviceHandle,
        buffer: RuntimeAudioBufferHandle,
    ) -> Result<RuntimeAudioPlaybackHandle, String> {
        let device_ref = self.audio_device_ref(device)?;
        let buffer_ref = self.audio_buffer_ref(buffer)?;
        ensure_audio_buffer_matches_device(
            device_ref.sample_rate_hz,
            device_ref.channels,
            buffer_ref.sample_rate_hz,
            buffer_ref.channels,
        )?;
        let format = pcm_wave_format(device_ref.sample_rate_hz, device_ref.channels)?;
        let mut wave_out: HWAVEOUT = null_mut();
        let open_status = unsafe { waveOutOpen(&mut wave_out, WAVE_MAPPER, &format, 0, 0, 0) };
        if open_status != MMSYSERR_NOERROR {
            return Err(format!(
                "failed to open audio playback device: {}",
                mmresult_text(open_status)
            ));
        }
        let mut playback = NativeAudioPlayback::new(
            device,
            wave_out,
            buffer_ref.frames,
            buffer_ref.pcm_bytes.clone(),
        );
        playback.gain_milli = self.audio_device_ref(device)?.gain_milli;
        if let Err(err) = prepare_and_queue_playback(&mut playback) {
            return Err(playback_error_with_cleanup(&mut playback, err));
        }
        let device_gain = self.audio_device_ref(device)?.gain_milli;
        if let Err(err) = Self::apply_playback_gain(&mut playback, device_gain) {
            return Err(playback_error_with_cleanup(&mut playback, err));
        }
        Ok(self.insert_audio_playback(playback))
    }

    fn sync_playback_state(&mut self, playback: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        let playback = self.audio_playback_mut(playback)?;
        if playback.wave_out.is_null() {
            return Ok(());
        }
        if playback.header.dwFlags & WHDR_DONE != 0 && !playback.looping {
            release_playback_resources(playback)?;
        }
        Ok(())
    }

    fn apply_playback_gain(
        playback: &mut NativeAudioPlayback,
        device_gain: i64,
    ) -> Result<(), String> {
        if playback.wave_out.is_null() {
            return Ok(());
        }
        let combined = (device_gain.max(0) * playback.gain_milli.max(0)) / 1000;
        let clamped = combined.clamp(0, 2000);
        let channel = ((clamped * 0xFFFF) / 1000).clamp(0, 0xFFFF) as u32;
        let packed = channel | (channel << 16);
        let status = unsafe { waveOutSetVolume(playback.wave_out, packed) };
        if status != MMSYSERR_NOERROR {
            return Err(format!(
                "failed to set playback volume: {}",
                mmresult_text(status)
            ));
        }
        Ok(())
    }

    fn restart_playback(&mut self, handle: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        let device = self.audio_playback_ref(handle)?.device;
        let device_gain = self.audio_device_ref(device)?.gain_milli;
        let playback = self.audio_playback_mut(handle)?;
        if playback.wave_out.is_null() {
            return Ok(());
        }
        unsafe {
            waveOutReset(playback.wave_out);
        }
        cleanup_prepared_playback(playback)?;
        prepare_and_queue_playback(playback)?;
        if playback.paused {
            let status = unsafe { waveOutPause(playback.wave_out) };
            if status != MMSYSERR_NOERROR {
                return Err(format!(
                    "failed to pause restarted playback: {}",
                    mmresult_text(status)
                ));
            }
        }
        Self::apply_playback_gain(playback, device_gain)?;
        Ok(())
    }

    fn close_playback(&mut self, handle: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        let playback = self.audio_playback_mut(handle)?;
        release_playback_resources(playback)
    }
}

impl Drop for NativeProcessHost {
    fn drop(&mut self) {
        let playback_handles = self.audio_playbacks.keys().copied().collect::<Vec<_>>();
        for handle in playback_handles {
            let _ = self.close_playback(handle);
        }
        for window in self.windows.values() {
            if !window.hwnd.is_null() && !window.closed {
                unsafe {
                    DestroyWindow(window.hwnd);
                }
            }
        }
    }
}

impl RuntimeHost for NativeProcessHost {
    fn print(&mut self, text: &str) -> Result<(), String> {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(text.as_bytes())
            .map_err(|err| format!("failed to write stdout: {err}"))
    }

    fn eprint(&mut self, text: &str) -> Result<(), String> {
        let mut stderr = io::stderr().lock();
        stderr
            .write_all(text.as_bytes())
            .map_err(|err| format!("failed to write stderr: {err}"))
    }

    fn flush_stdout(&mut self) -> Result<(), String> {
        io::stdout()
            .lock()
            .flush()
            .map_err(|err| format!("failed to flush stdout: {err}"))
    }

    fn flush_stderr(&mut self) -> Result<(), String> {
        io::stderr()
            .lock()
            .flush()
            .map_err(|err| format!("failed to flush stderr: {err}"))
    }

    fn stdin_read_line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|err| format!("failed to read stdin line: {err}"))?;
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
        Ok(line)
    }

    fn arg_count(&mut self) -> Result<usize, String> {
        RuntimeHost::arg_count(&mut self.base)
    }

    fn arg_get(&mut self, index: usize) -> Result<String, String> {
        RuntimeHost::arg_get(&mut self.base, index)
    }

    fn env_has(&mut self, name: &str) -> Result<bool, String> {
        RuntimeHost::env_has(&mut self.base, name)
    }

    fn env_get(&mut self, name: &str) -> Result<String, String> {
        RuntimeHost::env_get(&mut self.base, name)
    }

    fn cwd(&mut self) -> Result<String, String> {
        RuntimeHost::cwd(&mut self.base)
    }

    fn path_join(&mut self, a: &str, b: &str) -> Result<String, String> {
        RuntimeHost::path_join(&mut self.base, a, b)
    }

    fn path_normalize(&mut self, path: &str) -> Result<String, String> {
        RuntimeHost::path_normalize(&mut self.base, path)
    }

    fn path_parent(&mut self, path: &str) -> Result<String, String> {
        RuntimeHost::path_parent(&mut self.base, path)
    }

    fn path_file_name(&mut self, path: &str) -> Result<String, String> {
        RuntimeHost::path_file_name(&mut self.base, path)
    }

    fn path_ext(&mut self, path: &str) -> Result<String, String> {
        RuntimeHost::path_ext(&mut self.base, path)
    }

    fn path_is_absolute(&mut self, path: &str) -> Result<bool, String> {
        RuntimeHost::path_is_absolute(&mut self.base, path)
    }

    fn path_stem(&mut self, path: &str) -> Result<String, String> {
        RuntimeHost::path_stem(&mut self.base, path)
    }

    fn path_with_ext(&mut self, path: &str, ext: &str) -> Result<String, String> {
        RuntimeHost::path_with_ext(&mut self.base, path, ext)
    }

    fn path_relative_to(&mut self, path: &str, base: &str) -> Result<String, String> {
        RuntimeHost::path_relative_to(&mut self.base, path, base)
    }

    fn path_canonicalize(&mut self, path: &str) -> Result<String, String> {
        RuntimeHost::path_canonicalize(&mut self.base, path)
    }

    fn path_strip_prefix(&mut self, path: &str, prefix: &str) -> Result<String, String> {
        RuntimeHost::path_strip_prefix(&mut self.base, path, prefix)
    }

    fn fs_exists(&mut self, path: &str) -> Result<bool, String> {
        RuntimeHost::fs_exists(&mut self.base, path)
    }

    fn fs_is_file(&mut self, path: &str) -> Result<bool, String> {
        RuntimeHost::fs_is_file(&mut self.base, path)
    }

    fn fs_is_dir(&mut self, path: &str) -> Result<bool, String> {
        RuntimeHost::fs_is_dir(&mut self.base, path)
    }

    fn fs_read_text(&mut self, path: &str) -> Result<String, String> {
        RuntimeHost::fs_read_text(&mut self.base, path)
    }

    fn fs_read_bytes(&mut self, path: &str) -> Result<Vec<u8>, String> {
        RuntimeHost::fs_read_bytes(&mut self.base, path)
    }

    fn fs_stream_open_read(
        &mut self,
        path: &str,
    ) -> Result<crate::RuntimeFileStreamHandle, String> {
        RuntimeHost::fs_stream_open_read(&mut self.base, path)
    }

    fn fs_stream_open_write(
        &mut self,
        path: &str,
        append: bool,
    ) -> Result<crate::RuntimeFileStreamHandle, String> {
        RuntimeHost::fs_stream_open_write(&mut self.base, path, append)
    }

    fn fs_stream_read(
        &mut self,
        stream: crate::RuntimeFileStreamHandle,
        max_bytes: usize,
    ) -> Result<Vec<u8>, String> {
        RuntimeHost::fs_stream_read(&mut self.base, stream, max_bytes)
    }

    fn fs_stream_write(
        &mut self,
        stream: crate::RuntimeFileStreamHandle,
        bytes: &[u8],
    ) -> Result<usize, String> {
        RuntimeHost::fs_stream_write(&mut self.base, stream, bytes)
    }

    fn fs_stream_eof(&mut self, stream: crate::RuntimeFileStreamHandle) -> Result<bool, String> {
        RuntimeHost::fs_stream_eof(&mut self.base, stream)
    }

    fn fs_stream_close(&mut self, stream: crate::RuntimeFileStreamHandle) -> Result<(), String> {
        RuntimeHost::fs_stream_close(&mut self.base, stream)
    }

    fn fs_write_text(&mut self, path: &str, text: &str) -> Result<(), String> {
        RuntimeHost::fs_write_text(&mut self.base, path, text)
    }

    fn fs_write_bytes(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        RuntimeHost::fs_write_bytes(&mut self.base, path, bytes)
    }

    fn fs_list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        RuntimeHost::fs_list_dir(&mut self.base, path)
    }

    fn fs_mkdir_all(&mut self, path: &str) -> Result<(), String> {
        RuntimeHost::fs_mkdir_all(&mut self.base, path)
    }

    fn fs_create_dir(&mut self, path: &str) -> Result<(), String> {
        RuntimeHost::fs_create_dir(&mut self.base, path)
    }

    fn fs_remove_file(&mut self, path: &str) -> Result<(), String> {
        RuntimeHost::fs_remove_file(&mut self.base, path)
    }

    fn fs_remove_dir(&mut self, path: &str) -> Result<(), String> {
        RuntimeHost::fs_remove_dir(&mut self.base, path)
    }

    fn fs_remove_dir_all(&mut self, path: &str) -> Result<(), String> {
        RuntimeHost::fs_remove_dir_all(&mut self.base, path)
    }

    fn fs_copy_file(&mut self, from: &str, to: &str) -> Result<(), String> {
        RuntimeHost::fs_copy_file(&mut self.base, from, to)
    }

    fn fs_rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        RuntimeHost::fs_rename(&mut self.base, from, to)
    }

    fn fs_file_size(&mut self, path: &str) -> Result<i64, String> {
        RuntimeHost::fs_file_size(&mut self.base, path)
    }

    fn fs_modified_unix_ms(&mut self, path: &str) -> Result<i64, String> {
        RuntimeHost::fs_modified_unix_ms(&mut self.base, path)
    }

    fn window_open(
        &mut self,
        title: &str,
        width: i64,
        height: i64,
    ) -> Result<RuntimeWindowHandle, String> {
        self.open_native_window(title, width, height)
    }

    fn window_alive(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        let Some(window) = self.windows.get(&window).map(Box::as_ref) else {
            return Ok(false);
        };
        Ok(!window.closed && unsafe { IsWindow(window.hwnd) != 0 })
    }

    fn window_size(&mut self, window: RuntimeWindowHandle) -> Result<(i64, i64), String> {
        let window = self.window_ref(window)?;
        Ok((window.width, window.height))
    }

    fn window_resized(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.resized)
    }

    fn window_fullscreen(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.fullscreen)
    }

    fn window_minimized(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.minimized)
    }

    fn window_maximized(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.maximized)
    }

    fn window_focused(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.focused)
    }

    fn window_id(&mut self, window: RuntimeWindowHandle) -> Result<i64, String> {
        self.window_ref(window)?;
        i64::try_from(window.0)
            .map_err(|_| format!("Window handle `{}` does not fit in Int", window.0))
    }

    fn window_position(&mut self, window: RuntimeWindowHandle) -> Result<(i64, i64), String> {
        let window = self.window_ref(window)?;
        let mut rect = unsafe { zeroed::<RECT>() };
        if unsafe { GetWindowRect(window.hwnd, &mut rect) } == 0 {
            return Err("failed to query window position".to_string());
        }
        Ok((rect.left as i64, rect.top as i64))
    }

    fn window_title(&mut self, window: RuntimeWindowHandle) -> Result<String, String> {
        Ok(self.window_ref(window)?.title.clone())
    }

    fn window_visible(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.visible)
    }

    fn window_decorated(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.decorated)
    }

    fn window_resizable(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.resizable)
    }

    fn window_topmost(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.topmost)
    }

    fn window_cursor_visible(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.cursor_visible)
    }

    fn window_min_size(&mut self, window: RuntimeWindowHandle) -> Result<(i64, i64), String> {
        Ok(self.window_ref(window)?.min_size)
    }

    fn window_max_size(&mut self, window: RuntimeWindowHandle) -> Result<(i64, i64), String> {
        Ok(self.window_ref(window)?.max_size)
    }

    fn window_scale_factor_milli(&mut self, window: RuntimeWindowHandle) -> Result<i64, String> {
        let window = self.window_ref(window)?;
        let dpi = unsafe { GetDpiForWindow(window.hwnd) };
        if dpi == 0 {
            return Ok(query_monitor_scale_factor_milli(unsafe {
                MonitorFromWindow(window.hwnd, MONITOR_DEFAULTTONEAREST)
            }));
        }
        Ok((i64::from(dpi) * 1000) / 96)
    }

    fn window_theme_code(&mut self, window: RuntimeWindowHandle) -> Result<i64, String> {
        self.window_ref(window)?;
        Ok(current_windows_theme_code())
    }

    fn window_transparent(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.transparent)
    }

    fn window_theme_override_code(&mut self, window: RuntimeWindowHandle) -> Result<i64, String> {
        Ok(self.window_ref(window)?.theme_override_code)
    }

    fn window_cursor_icon_code(&mut self, window: RuntimeWindowHandle) -> Result<i64, String> {
        Ok(self.window_ref(window)?.cursor_icon_code)
    }

    fn window_cursor_grab_mode(&mut self, window: RuntimeWindowHandle) -> Result<i64, String> {
        Ok(self.window_ref(window)?.cursor_grab_mode)
    }

    fn window_cursor_position(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<(i64, i64), String> {
        Ok(self.window_ref(window)?.cursor_position)
    }

    fn window_text_input_enabled(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.window_ref(window)?.text_input_enabled)
    }

    fn window_current_monitor_index(&mut self, window: RuntimeWindowHandle) -> Result<i64, String> {
        let window = self.window_ref(window)?;
        self.current_monitor_index_for_window(window)
    }

    fn window_primary_monitor_index(&mut self) -> Result<i64, String> {
        for (index, monitor) in self.collect_monitor_infos()?.iter().enumerate() {
            if monitor.primary {
                return i64::try_from(index)
                    .map_err(|_| "native monitor index does not fit in Int".to_string());
            }
        }
        Ok(0)
    }

    fn window_monitor_count(&mut self) -> Result<i64, String> {
        i64::try_from(self.collect_monitor_infos()?.len())
            .map_err(|_| "native monitor count does not fit in Int".to_string())
    }

    fn window_monitor_name(&mut self, index: i64) -> Result<String, String> {
        Ok(self.monitor_info_at(index)?.name)
    }

    fn window_monitor_position(&mut self, index: i64) -> Result<(i64, i64), String> {
        Ok(self.monitor_info_at(index)?.position)
    }

    fn window_monitor_size(&mut self, index: i64) -> Result<(i64, i64), String> {
        Ok(self.monitor_info_at(index)?.size)
    }

    fn window_monitor_scale_factor_milli(&mut self, index: i64) -> Result<i64, String> {
        Ok(self.monitor_info_at(index)?.scale_factor_milli)
    }

    fn window_monitor_is_primary(&mut self, index: i64) -> Result<bool, String> {
        Ok(self.monitor_info_at(index)?.primary)
    }

    fn window_set_title(&mut self, window: RuntimeWindowHandle, title: &str) -> Result<(), String> {
        let hwnd = self.window_ref(window)?.hwnd;
        if self.window_ref(window)?.title == title {
            return Ok(());
        }
        let title_wide = wide_null(title);
        if unsafe { SetWindowTextW(hwnd, title_wide.as_ptr()) } == 0 {
            return Err("failed to update window title".to_string());
        }
        self.window_mut(window)?.title = title.to_string();
        Ok(())
    }

    fn window_set_position(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
    ) -> Result<(), String> {
        let hwnd = self.window_ref(window)?.hwnd;
        let x =
            i32::try_from(x).map_err(|_| "window x position does not fit in i32".to_string())?;
        let y =
            i32::try_from(y).map_err(|_| "window y position does not fit in i32".to_string())?;
        let ok = unsafe {
            SetWindowPos(
                hwnd,
                null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
        };
        if ok == 0 {
            return Err("failed to move native window".to_string());
        }
        Ok(())
    }

    fn window_set_size(
        &mut self,
        window: RuntimeWindowHandle,
        width: i64,
        height: i64,
    ) -> Result<(), String> {
        let hwnd = self.window_ref(window)?.hwnd;
        let (width, height) = (sanitize_dimension(width), sanitize_dimension(height));
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: i32::try_from(width).unwrap_or(i32::MAX),
            bottom: i32::try_from(height).unwrap_or(i32::MAX),
        };
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
        let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 };
        unsafe {
            AdjustWindowRectEx(&mut rect, style, 0, ex_style);
        }
        let ok = unsafe {
            SetWindowPos(
                hwnd,
                null_mut(),
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
        };
        if ok == 0 {
            return Err("failed to resize native window".to_string());
        }
        Ok(())
    }

    fn window_set_visible(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        if self.window_ref(window)?.visible == enabled {
            return Ok(());
        }
        let hwnd = self.window_ref(window)?.hwnd;
        unsafe {
            ShowWindow(hwnd, if enabled { SW_SHOW } else { SW_HIDE });
        }
        self.window_mut(window)?.visible = enabled;
        Ok(())
    }

    fn window_set_decorated(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        if self.window_ref(window)?.decorated == enabled {
            return Ok(());
        }
        let hwnd = self.window_ref(window)?.hwnd;
        let mut style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        if enabled {
            style |= WS_OVERLAPPEDWINDOW as isize;
        } else {
            style &= !(WS_OVERLAPPEDWINDOW as isize);
        }
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style);
            SetWindowPos(
                hwnd,
                null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            );
        }
        self.window_mut(window)?.decorated = enabled;
        Ok(())
    }

    fn window_set_resizable(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        if self.window_ref(window)?.resizable == enabled {
            return Ok(());
        }
        let hwnd = self.window_ref(window)?.hwnd;
        let mut style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        if enabled {
            style |= (WS_SIZEBOX | WS_MAXIMIZEBOX) as isize;
        } else {
            style &= !((WS_SIZEBOX | WS_MAXIMIZEBOX) as isize);
        }
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_STYLE, style);
            SetWindowPos(
                hwnd,
                null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            );
        }
        self.window_mut(window)?.resizable = enabled;
        Ok(())
    }

    fn window_set_min_size(
        &mut self,
        window: RuntimeWindowHandle,
        width: i64,
        height: i64,
    ) -> Result<(), String> {
        let size = (width.max(0), height.max(0));
        let window = self.window_mut(window)?;
        if window.min_size == size {
            return Ok(());
        }
        window.min_size = size;
        Ok(())
    }

    fn window_set_max_size(
        &mut self,
        window: RuntimeWindowHandle,
        width: i64,
        height: i64,
    ) -> Result<(), String> {
        let size = (width.max(0), height.max(0));
        let window = self.window_mut(window)?;
        if window.max_size == size {
            return Ok(());
        }
        window.max_size = size;
        Ok(())
    }

    fn window_set_fullscreen(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let (hwnd, restore_style, restore_ex_style, restore_rect) = {
            let window_state = self.window_mut(window)?;
            if window_state.fullscreen == enabled {
                return Ok(());
            }
            (
                window_state.hwnd,
                window_state.restore_style,
                window_state.restore_ex_style,
                window_state.restore_rect,
            )
        };
        if enabled {
            let next_restore_style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
            let next_restore_ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
            let mut next_restore_rect = unsafe { zeroed::<RECT>() };
            unsafe {
                GetWindowRect(hwnd, &mut next_restore_rect);
            }
            let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
            let mut monitor_info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..unsafe { zeroed() }
            };
            if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) } == 0 {
                return Err("failed to resolve monitor bounds for fullscreen window".to_string());
            }
            {
                let window_state = self.window_mut(window)?;
                window_state.restore_style = next_restore_style;
                window_state.restore_ex_style = next_restore_ex_style;
                window_state.restore_rect = next_restore_rect;
            }
            unsafe {
                SetWindowLongPtrW(
                    hwnd,
                    GWL_STYLE,
                    next_restore_style & !(WS_OVERLAPPEDWINDOW as isize),
                );
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    monitor_info.rcMonitor.left,
                    monitor_info.rcMonitor.top,
                    monitor_info.rcMonitor.right - monitor_info.rcMonitor.left,
                    monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top,
                    SWP_FRAMECHANGED | SWP_NOOWNERZORDER,
                );
            }
        } else {
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, restore_style);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, restore_ex_style);
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    restore_rect.left,
                    restore_rect.top,
                    restore_rect.right - restore_rect.left,
                    restore_rect.bottom - restore_rect.top,
                    SWP_FRAMECHANGED | SWP_NOOWNERZORDER,
                );
            }
        }
        self.window_mut(window)?.fullscreen = enabled;
        Ok(())
    }

    fn window_set_minimized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let hwnd = self.window_ref(window)?.hwnd;
        unsafe {
            ShowWindow(hwnd, if enabled { SW_MINIMIZE } else { SW_RESTORE });
        }
        self.window_mut(window)?.minimized = enabled;
        Ok(())
    }

    fn window_set_maximized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        if self.window_ref(window)?.maximized == enabled {
            return Ok(());
        }
        let hwnd = self.window_ref(window)?.hwnd;
        unsafe {
            ShowWindow(hwnd, if enabled { SW_MAXIMIZE } else { SW_RESTORE });
        }
        self.window_mut(window)?.maximized = enabled;
        Ok(())
    }

    fn window_set_topmost(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        if self.window_ref(window)?.topmost == enabled {
            return Ok(());
        }
        let hwnd = self.window_ref(window)?.hwnd;
        unsafe {
            SetWindowPos(
                hwnd,
                if enabled {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                },
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        self.window_mut(window)?.topmost = enabled;
        Ok(())
    }

    fn window_set_cursor_visible(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let (hwnd, mouse_in_window, cursor_icon_code) = {
            let window = self.window_mut(window)?;
            if window.cursor_visible == enabled {
                return Ok(());
            }
            window.cursor_visible = enabled;
            (window.hwnd, window.mouse_in_window, window.cursor_icon_code)
        };
        if mouse_in_window {
            desktop_temp_probe(
                "cursor",
                format!("hwnd={hwnd:?} visible={enabled} icon={cursor_icon_code}"),
            );
            apply_window_cursor_state(hwnd, enabled, cursor_icon_code);
        }
        Ok(())
    }

    fn window_set_transparent(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        if self.window_ref(window)?.transparent == enabled {
            return Ok(());
        }
        let hwnd = self.window_ref(window)?.hwnd;
        let mut ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        if enabled {
            ex_style |= WS_EX_LAYERED as isize;
        } else {
            ex_style &= !(WS_EX_LAYERED as isize);
        }
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        }
        if enabled {
            unsafe {
                SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            }
        }
        self.window_mut(window)?.transparent = enabled;
        Ok(())
    }

    fn window_set_theme_override_code(
        &mut self,
        window: RuntimeWindowHandle,
        code: i64,
    ) -> Result<(), String> {
        if self.window_ref(window)?.theme_override_code == code {
            return Ok(());
        }
        let hwnd = {
            let window = self.window_mut(window)?;
            window.theme_override_code = code;
            window.applying_theme_override = true;
            window.hwnd
        };
        apply_theme_override(hwnd, code);
        self.window_mut(window)?.applying_theme_override = false;
        Ok(())
    }

    fn window_set_cursor_icon_code(
        &mut self,
        window: RuntimeWindowHandle,
        code: i64,
    ) -> Result<(), String> {
        let (hwnd, mouse_in_window, cursor_visible) = {
            let window = self.window_mut(window)?;
            if window.cursor_icon_code == code {
                return Ok(());
            }
            window.cursor_icon_code = code;
            (window.hwnd, window.mouse_in_window, window.cursor_visible)
        };
        if mouse_in_window {
            desktop_temp_probe(
                "cursor",
                format!("hwnd={hwnd:?} visible={cursor_visible} icon={code}"),
            );
            apply_window_cursor_state(hwnd, cursor_visible, code);
        }
        Ok(())
    }

    fn window_set_cursor_grab_mode(
        &mut self,
        window: RuntimeWindowHandle,
        mode: i64,
    ) -> Result<(), String> {
        if self.window_ref(window)?.cursor_grab_mode == mode {
            return Ok(());
        }
        let (hwnd, center) = {
            let window_state = self.window_ref(window)?;
            (
                window_state.hwnd,
                (
                    window_state.width.saturating_div(2),
                    window_state.height.saturating_div(2),
                ),
            )
        };
        apply_cursor_grab(hwnd, mode)?;
        if mode == 2 {
            let mut point = POINT {
                x: i32::try_from(center.0)
                    .map_err(|_| "cursor center x does not fit in i32".to_string())?,
                y: i32::try_from(center.1)
                    .map_err(|_| "cursor center y does not fit in i32".to_string())?,
            };
            if unsafe { ClientToScreen(hwnd, &mut point) } == 0 {
                return Err("failed to translate locked cursor position".to_string());
            }
            self.window_mut(window)?.suppress_cursor_move = true;
            unsafe {
                SetCursorPos(point.x, point.y);
            }
        }
        let window_state = self.window_mut(window)?;
        window_state.cursor_grab_mode = mode;
        if mode == 2 {
            window_state.cursor_position = center;
            window_state.mouse_pos = center;
        }
        Ok(())
    }

    fn window_set_cursor_position(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
    ) -> Result<(), String> {
        if self.window_ref(window)?.cursor_position == (x, y) {
            return Ok(());
        }
        let hwnd = self.window_ref(window)?.hwnd;
        let mut point = POINT {
            x: i32::try_from(x).map_err(|_| "cursor x does not fit in i32".to_string())?,
            y: i32::try_from(y).map_err(|_| "cursor y does not fit in i32".to_string())?,
        };
        if unsafe { ClientToScreen(hwnd, &mut point) } == 0 {
            return Err("failed to translate cursor position".to_string());
        }
        self.window_mut(window)?.suppress_cursor_move = true;
        unsafe {
            SetCursorPos(point.x, point.y);
        }
        let window = self.window_mut(window)?;
        window.cursor_position = (x, y);
        window.mouse_pos = (x, y);
        Ok(())
    }

    fn window_set_text_input_enabled(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        if window.text_input_enabled == enabled {
            return Ok(());
        }
        window.text_input_enabled = enabled;
        if !enabled {
            window.composition_active = false;
            window.pending_high_surrogate = None;
        }
        Ok(())
    }

    fn window_request_redraw(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.push_redraw_event();
        window.signal_message_loop();
        Ok(())
    }

    fn window_request_attention(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let hwnd = {
            let window = self.window_mut(window)?;
            if enabled && window.focused {
                window.attention_requested = false;
                return Ok(());
            }
            if window.attention_requested == enabled {
                return Ok(());
            }
            window.attention_requested = enabled;
            if window.hwnd.is_null() || window.closed || unsafe { IsWindow(window.hwnd) } == 0 {
                return Ok(());
            }
            window.hwnd
        };
        let info = FLASHWINFO {
            cbSize: size_of::<FLASHWINFO>() as u32,
            hwnd,
            dwFlags: attention_flash_flags(enabled),
            uCount: 0,
            dwTimeout: 0,
        };
        unsafe {
            FlashWindowEx(&info);
        }
        Ok(())
    }

    fn window_close(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        self.close_native_window(window, true)
    }

    fn canvas_fill(&mut self, window: RuntimeWindowHandle, color: i64) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.surface.fill(pack_color(color));
        Ok(())
    }

    fn canvas_rect(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
        w: i64,
        h: i64,
        color: i64,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.surface.fill_rect(x, y, w, h, pack_color(color));
        Ok(())
    }

    fn canvas_line(
        &mut self,
        window: RuntimeWindowHandle,
        x1: i64,
        y1: i64,
        x2: i64,
        y2: i64,
        color: i64,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.surface.draw_line(x1, y1, x2, y2, pack_color(color));
        Ok(())
    }

    fn canvas_circle_fill(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
        radius: i64,
        color: i64,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.surface.fill_circle(x, y, radius, pack_color(color));
        Ok(())
    }

    fn canvas_label(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
        text: &str,
        color: i64,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.surface.draw_label(x, y, text, pack_color(color));
        Ok(())
    }

    fn canvas_label_size(&mut self, text: &str) -> Result<(i64, i64), String> {
        Ok(text_engine::measure_plain_text(
            text,
            text_engine::DEFAULT_FONT_SIZE,
        ))
    }

    fn canvas_present(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        self.present_window(window)
    }

    fn canvas_rgb(&mut self, r: i64, g: i64, b: i64) -> Result<i64, String> {
        let clamp = |value: i64| value.clamp(0, 255);
        Ok((clamp(r) << 16) | (clamp(g) << 8) | clamp(b))
    }

    fn image_load(&mut self, path: &str) -> Result<RuntimeImageHandle, String> {
        let resolved = self.base.resolve_fs_path(path)?;
        let image = decode_bmp_image(&resolved)?;
        Ok(self.insert_image(image))
    }

    fn canvas_image_create(
        &mut self,
        width: i64,
        height: i64,
    ) -> Result<RuntimeImageHandle, String> {
        if width < 0 || height < 0 {
            return Err("canvas_image_create expects non-negative dimensions".to_string());
        }
        Ok(self.insert_image(NativeImage {
            width,
            height,
            pixels: vec![0; usize::try_from(width.saturating_mul(height)).unwrap_or(0)],
        }))
    }

    fn canvas_image_replace_rgba(
        &mut self,
        image: RuntimeImageHandle,
        rgba: &[u8],
    ) -> Result<(), String> {
        let image = self.image_mut(image)?;
        let expected = usize::try_from(image.width.saturating_mul(image.height))
            .unwrap_or(0)
            .saturating_mul(4);
        if rgba.len() != expected {
            return Err(format!(
                "canvas_image_replace_rgba expected {expected} bytes for {}x{} image, got {}",
                image.width,
                image.height,
                rgba.len()
            ));
        }
        image.pixels.clear();
        image.pixels.reserve(expected / 4);
        for chunk in rgba.chunks_exact(4) {
            image.pixels.push(
                (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]),
            );
        }
        Ok(())
    }

    fn canvas_image_size(&mut self, image: RuntimeImageHandle) -> Result<(i64, i64), String> {
        let image = self.image_ref(image)?;
        Ok((image.width, image.height))
    }

    fn canvas_blit(
        &mut self,
        window: RuntimeWindowHandle,
        image: RuntimeImageHandle,
        x: i64,
        y: i64,
    ) -> Result<(), String> {
        let source = self.image_ref(image)?.clone();
        let window = self.window_mut(window)?;
        window.surface.blit(
            &source,
            0,
            0,
            source.width,
            source.height,
            x,
            y,
            source.width,
            source.height,
        );
        Ok(())
    }

    fn canvas_blit_scaled(
        &mut self,
        window: RuntimeWindowHandle,
        image: RuntimeImageHandle,
        x: i64,
        y: i64,
        w: i64,
        h: i64,
    ) -> Result<(), String> {
        let source = self.image_ref(image)?.clone();
        let window = self.window_mut(window)?;
        window
            .surface
            .blit(&source, 0, 0, source.width, source.height, x, y, w, h);
        Ok(())
    }

    fn canvas_blit_region(
        &mut self,
        window: RuntimeWindowHandle,
        image: RuntimeImageHandle,
        sx: i64,
        sy: i64,
        sw: i64,
        sh: i64,
        dx: i64,
        dy: i64,
        dw: i64,
        dh: i64,
    ) -> Result<(), String> {
        let source = self.image_ref(image)?.clone();
        let window = self.window_mut(window)?;
        window.surface.blit(&source, sx, sy, sw, sh, dx, dy, dw, dh);
        Ok(())
    }

    fn events_pump(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<RuntimeAppFrameHandle, String> {
        self.pump_messages()?;
        let window_state = self.window_mut(window)?;
        window_state.redraw_pending = false;
        let events = std::mem::take(&mut window_state.events)
            .into_iter()
            .map(|event| Self::event_with_window_id(window, event))
            .collect::<VecDeque<_>>();
        let input = Self::snapshot_frame_input(window_state);
        Ok(self.insert_frame(BufferedAppFrame { events, input }))
    }

    fn events_poll(
        &mut self,
        frame: RuntimeAppFrameHandle,
    ) -> Result<Option<RuntimeEventRecord>, String> {
        let frame = self.frame_mut(frame)?;
        let Some(event) = frame.events.pop_front() else {
            return Ok(None);
        };
        Ok(Some(RuntimeEventRecord {
            kind: event.kind,
            window_id: event.window_id,
            a: event.a,
            b: event.b,
            flags: event.flags,
            text: event.text,
            key_code: event.key_code,
            physical_key: event.physical_key,
            logical_key: event.logical_key,
            key_location: event.key_location,
            pointer_x: event.pointer_x,
            pointer_y: event.pointer_y,
            repeated: event.repeated,
        }))
    }

    fn events_session_open(&mut self) -> Result<RuntimeAppSessionHandle, String> {
        Ok(self.insert_session())
    }

    fn events_session_close(&mut self, session: RuntimeAppSessionHandle) -> Result<(), String> {
        let mut windows = self.session_ref(session)?.windows.clone();
        windows.reverse();
        self.remove_session_wakes(session);
        self.sessions.remove(&session);
        self.sync_process_raw_input_registration()?;
        for window in windows {
            let _ = self.close_native_window(window, false);
        }
        self.pump_messages()?;
        self.prune_closed_windows();
        Ok(())
    }

    fn events_session_attach_window(
        &mut self,
        session: RuntimeAppSessionHandle,
        window: RuntimeWindowHandle,
    ) -> Result<(), String> {
        self.window_ref(window)?;
        let session_state = self.session_mut(session)?;
        if !session_state.windows.contains(&window) {
            session_state.windows.push(window);
            if session_state.suspended {
                session_state.resumed = false;
                session_state.suspended = false;
            }
        }
        self.sync_process_raw_input_registration()?;
        self.notify_session_queue(session);
        Ok(())
    }

    fn events_session_detach_window(
        &mut self,
        session: RuntimeAppSessionHandle,
        window: RuntimeWindowHandle,
    ) -> Result<(), String> {
        let session_state = self.session_mut(session)?;
        let before = session_state.windows.len();
        session_state
            .windows
            .retain(|candidate| *candidate != window);
        if session_state.windows.len() != before {
            self.sync_process_raw_input_registration()?;
            self.notify_session_queue(session);
        }
        Ok(())
    }

    fn events_session_window_by_id(
        &mut self,
        session: RuntimeAppSessionHandle,
        window_id: i64,
    ) -> Result<Option<RuntimeWindowHandle>, String> {
        self.prune_closed_windows();
        let session_windows = self.session_ref(session)?.windows.clone();
        for window in session_windows {
            let Ok(id) = self.window_id(window) else {
                continue;
            };
            if id == window_id {
                return Ok(Some(window));
            }
        }
        Ok(None)
    }

    fn events_session_window_ids(
        &mut self,
        session: RuntimeAppSessionHandle,
    ) -> Result<Vec<i64>, String> {
        self.prune_closed_windows();
        let session_windows = self.session_ref(session)?.windows.clone();
        let mut ids = Vec::new();
        for window in session_windows {
            let Ok(id) = self.window_id(window) else {
                continue;
            };
            ids.push(id);
        }
        Ok(ids)
    }

    fn events_session_pump(
        &mut self,
        session: RuntimeAppSessionHandle,
    ) -> Result<RuntimeAppFrameHandle, String> {
        let started = Instant::now();
        self.pump_messages()?;
        self.prune_closed_windows();
        let session_windows = self.session_ref(session)?.windows.clone();
        let mut window_events = Vec::new();
        let mut input = BufferedFrameInput::default();
        let mut live_windows = 0usize;
        let mut any_window_focused = false;
        for window in session_windows {
            let Ok(state) = self.window_state_mut(window) else {
                continue;
            };
            let is_live = Self::window_state_live(state);
            if !is_live {
                state.events.clear();
                continue;
            }
            live_windows += 1;
            state.redraw_pending = false;
            any_window_focused = any_window_focused || state.focused;
            let mut state_events = std::mem::take(&mut state.events)
                .into_iter()
                .map(|event| Self::event_with_window_id(window, event))
                .collect::<Vec<_>>();
            prioritize_close_requested_events(&mut state_events);
            let state_input = Self::snapshot_frame_input(state);
            Self::merge_frame_input(&mut input, state_input);
            window_events.extend(state_events);
        }
        let device_events_policy = self.session_ref(session)?.device_events_policy;
        window_events.retain(|event| {
            !crate::buffered_device_event_kind(event.kind)
                || crate::buffered_device_events_allowed(device_events_policy, any_window_focused)
        });
        let mut events = Vec::new();
        {
            let session_state = self.session_mut(session)?;
            if !session_state.resumed && live_windows > 0 {
                events.push(BufferedEvent {
                    kind: EVENT_APP_RESUMED,
                    window_id: 0,
                    a: 0,
                    b: 0,
                    flags: 0,
                    text: String::new(),
                    ..BufferedEvent::default()
                });
                session_state.resumed = true;
                session_state.suspended = false;
            }
            while session_state.pending_wakes > 0 {
                session_state.pending_wakes -= 1;
                events.push(BufferedEvent {
                    kind: EVENT_WAKE,
                    window_id: 0,
                    a: 0,
                    b: 0,
                    flags: 0,
                    text: String::new(),
                    ..BufferedEvent::default()
                });
            }
        }
        events.extend(window_events);
        {
            let session_state = self.session_mut(session)?;
            if live_windows == 0 && session_state.resumed && !session_state.suspended {
                events.push(BufferedEvent {
                    kind: EVENT_APP_SUSPENDED,
                    window_id: 0,
                    a: 0,
                    b: 0,
                    flags: 0,
                    text: String::new(),
                    ..BufferedEvent::default()
                });
                session_state.suspended = true;
            }
        }
        self.prune_closed_windows();
        self.sync_process_raw_input_registration()?;
        events.push(BufferedEvent {
            kind: EVENT_ABOUT_TO_WAIT,
            window_id: 0,
            a: 0,
            b: 0,
            flags: 0,
            text: String::new(),
            ..BufferedEvent::default()
        });
        let elapsed = started.elapsed();
        if events.len() >= 128 || elapsed >= Duration::from_millis(16) {
            desktop_temp_probe(
                "frame",
                format!(
                    "session {:?} produced {} event(s) across {} live window(s) in {} ms",
                    session,
                    events.len(),
                    live_windows,
                    elapsed.as_millis()
                ),
            );
        }
        let frame = self.insert_frame(BufferedAppFrame {
            events: VecDeque::from(events),
            input,
        });
        Ok(frame)
    }

    fn events_session_device_events(
        &mut self,
        session: RuntimeAppSessionHandle,
    ) -> Result<i64, String> {
        Ok(self.session_ref(session)?.device_events_policy)
    }

    fn events_session_set_device_events(
        &mut self,
        session: RuntimeAppSessionHandle,
        policy: i64,
    ) -> Result<(), String> {
        if !(DEVICE_EVENTS_NEVER..=DEVICE_EVENTS_ALWAYS).contains(&policy) {
            return Err(format!("invalid device events policy `{policy}`"));
        }
        self.session_mut(session)?.device_events_policy = policy;
        self.sync_process_raw_input_registration()?;
        Ok(())
    }

    fn events_session_wait(
        &mut self,
        session: RuntimeAppSessionHandle,
        timeout_ms: i64,
    ) -> Result<RuntimeAppFrameHandle, String> {
        self.wait_for_session_activity(session, timeout_ms)?;
        let frame = self.events_session_pump(session)?;
        Ok(frame)
    }

    fn events_session_create_wake(
        &mut self,
        session: RuntimeAppSessionHandle,
    ) -> Result<RuntimeWakeHandle, String> {
        self.session_ref(session)?;
        Ok(self.insert_wake(session))
    }

    fn events_wake_signal(&mut self, wake: RuntimeWakeHandle) -> Result<(), String> {
        let session = self.wake_ref(wake)?.session;
        self.session_mut(session)?.pending_wakes += 1;
        self.notify_session_queue(session);
        Ok(())
    }

    fn input_key_code(&mut self, name: &str) -> Result<i64, String> {
        Ok(common_named_key_code(name))
    }

    fn input_key_down(&mut self, frame: RuntimeAppFrameHandle, key: i64) -> Result<bool, String> {
        Ok(self.frame_ref(frame)?.input.key_down.contains(&key))
    }

    fn input_key_pressed(
        &mut self,
        frame: RuntimeAppFrameHandle,
        key: i64,
    ) -> Result<bool, String> {
        Ok(self.frame_ref(frame)?.input.key_pressed.contains(&key))
    }

    fn input_key_released(
        &mut self,
        frame: RuntimeAppFrameHandle,
        key: i64,
    ) -> Result<bool, String> {
        Ok(self.frame_ref(frame)?.input.key_released.contains(&key))
    }

    fn input_mouse_button_code(&mut self, name: &str) -> Result<i64, String> {
        Ok(common_named_mouse_button_code(name))
    }

    fn input_mouse_pos(&mut self, frame: RuntimeAppFrameHandle) -> Result<(i64, i64), String> {
        Ok(self.frame_ref(frame)?.input.mouse_pos)
    }

    fn input_mouse_down(
        &mut self,
        frame: RuntimeAppFrameHandle,
        button: i64,
    ) -> Result<bool, String> {
        Ok(self.frame_ref(frame)?.input.mouse_down.contains(&button))
    }

    fn input_mouse_pressed(
        &mut self,
        frame: RuntimeAppFrameHandle,
        button: i64,
    ) -> Result<bool, String> {
        Ok(self.frame_ref(frame)?.input.mouse_pressed.contains(&button))
    }

    fn input_mouse_released(
        &mut self,
        frame: RuntimeAppFrameHandle,
        button: i64,
    ) -> Result<bool, String> {
        Ok(self
            .frame_ref(frame)?
            .input
            .mouse_released
            .contains(&button))
    }

    fn input_mouse_wheel_y(&mut self, frame: RuntimeAppFrameHandle) -> Result<i64, String> {
        Ok(self.frame_ref(frame)?.input.mouse_wheel_y)
    }

    fn input_mouse_in_window(&mut self, frame: RuntimeAppFrameHandle) -> Result<bool, String> {
        Ok(self.frame_ref(frame)?.input.mouse_in_window)
    }

    fn clipboard_read_text(&mut self) -> Result<String, String> {
        clipboard_read_text_impl()
    }

    fn clipboard_write_text(&mut self, text: &str) -> Result<(), String> {
        clipboard_write_text_impl(text)
    }

    fn clipboard_read_bytes(&mut self) -> Result<Vec<u8>, String> {
        clipboard_read_bytes_impl()
    }

    fn clipboard_write_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        clipboard_write_bytes_impl(bytes)
    }

    fn text_input_composition_area_active(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<bool, String> {
        Ok(self.window_ref(window)?.composition_area_active)
    }

    fn text_input_composition_area_position(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<(i64, i64), String> {
        Ok(self.window_ref(window)?.composition_area_position)
    }

    fn text_input_composition_area_size(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<(i64, i64), String> {
        Ok(self.window_ref(window)?.composition_area_size)
    }

    fn text_input_set_composition_area(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.composition_area_active = true;
        window.composition_area_position = (x, y);
        window.composition_area_size = (width.max(0), height.max(0));
        apply_composition_area(
            window.hwnd,
            true,
            window.composition_area_position,
            window.composition_area_size,
        )
    }

    fn text_input_clear_composition_area(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.composition_area_active = false;
        window.composition_area_position = (0, 0);
        window.composition_area_size = (0, 0);
        apply_composition_area(window.hwnd, false, (0, 0), (0, 0))
    }

    fn audio_default_output(&mut self) -> Result<RuntimeAudioDeviceHandle, String> {
        let (sample_rate_hz, channels) = Self::select_default_audio_config()?;
        Ok(self.insert_audio_device(sample_rate_hz, channels))
    }

    fn audio_output_close(&mut self, device: RuntimeAudioDeviceHandle) -> Result<(), String> {
        if !self.audio_devices.contains_key(&device) {
            return Err(format!("invalid AudioDevice handle `{}`", device.0));
        }
        let playback_handles = self
            .audio_playbacks
            .iter()
            .filter_map(|(handle, playback)| (playback.device == device).then_some(*handle))
            .collect::<Vec<_>>();
        for handle in playback_handles {
            self.close_playback(handle)?;
            self.audio_playbacks.remove(&handle);
        }
        self.audio_devices.remove(&device);
        Ok(())
    }

    fn audio_output_sample_rate_hz(
        &mut self,
        device: RuntimeAudioDeviceHandle,
    ) -> Result<i64, String> {
        Ok(self.audio_device_ref(device)?.sample_rate_hz)
    }

    fn audio_output_channels(&mut self, device: RuntimeAudioDeviceHandle) -> Result<i64, String> {
        Ok(self.audio_device_ref(device)?.channels)
    }

    fn audio_buffer_load_wav(&mut self, path: &str) -> Result<RuntimeAudioBufferHandle, String> {
        let resolved = self.base.resolve_fs_path(path)?;
        let buffer = decode_wav_buffer(&resolved)?;
        Ok(self.insert_audio_buffer(buffer))
    }

    fn audio_buffer_frames(&mut self, buffer: RuntimeAudioBufferHandle) -> Result<i64, String> {
        Ok(self.audio_buffer_ref(buffer)?.frames)
    }

    fn audio_buffer_channels(&mut self, buffer: RuntimeAudioBufferHandle) -> Result<i64, String> {
        Ok(self.audio_buffer_ref(buffer)?.channels)
    }

    fn audio_buffer_sample_rate_hz(
        &mut self,
        buffer: RuntimeAudioBufferHandle,
    ) -> Result<i64, String> {
        Ok(self.audio_buffer_ref(buffer)?.sample_rate_hz)
    }

    fn audio_play_buffer(
        &mut self,
        device: RuntimeAudioDeviceHandle,
        buffer: RuntimeAudioBufferHandle,
    ) -> Result<RuntimeAudioPlaybackHandle, String> {
        self.open_audio_playback(device, buffer)
    }

    fn audio_output_set_gain_milli(
        &mut self,
        device: RuntimeAudioDeviceHandle,
        milli: i64,
    ) -> Result<(), String> {
        self.audio_device_mut(device)?.gain_milli = milli;
        let playback_handles = self
            .audio_playbacks
            .iter()
            .filter_map(|(handle, playback)| (playback.device == device).then_some(*handle))
            .collect::<Vec<_>>();
        for handle in playback_handles {
            let device_gain = self.audio_device_ref(device)?.gain_milli;
            let playback = self.audio_playback_mut(handle)?;
            Self::apply_playback_gain(playback, device_gain)?;
        }
        Ok(())
    }

    fn audio_playback_stop(&mut self, playback: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        self.close_playback(playback)?;
        self.audio_playbacks.remove(&playback);
        Ok(())
    }

    fn audio_playback_pause(&mut self, playback: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        self.sync_playback_state(playback)?;
        let playback = self.audio_playback_mut(playback)?;
        if playback.wave_out.is_null() || playback.finished {
            return Ok(());
        }
        let status = unsafe { waveOutPause(playback.wave_out) };
        if status != MMSYSERR_NOERROR {
            return Err(format!(
                "failed to pause playback: {}",
                mmresult_text(status)
            ));
        }
        playback.paused = true;
        Ok(())
    }

    fn audio_playback_resume(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<(), String> {
        self.sync_playback_state(playback)?;
        let playback = self.audio_playback_mut(playback)?;
        if playback.wave_out.is_null() || playback.finished {
            return Ok(());
        }
        let status = unsafe { waveOutRestart(playback.wave_out) };
        if status != MMSYSERR_NOERROR {
            return Err(format!(
                "failed to resume playback: {}",
                mmresult_text(status)
            ));
        }
        playback.paused = false;
        Ok(())
    }

    fn audio_playback_playing(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        self.sync_playback_state(playback)?;
        let playback = self.audio_playback_ref(playback)?;
        Ok(!playback.wave_out.is_null() && !playback.paused && !playback.finished)
    }

    fn audio_playback_paused(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        self.sync_playback_state(playback)?;
        Ok(self.audio_playback_ref(playback)?.paused)
    }

    fn audio_playback_finished(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        self.sync_playback_state(playback)?;
        Ok(self.audio_playback_ref(playback)?.finished)
    }

    fn audio_playback_set_gain_milli(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
        milli: i64,
    ) -> Result<(), String> {
        let device = self.audio_playback_ref(playback)?.device;
        let device_gain = self.audio_device_ref(device)?.gain_milli;
        self.audio_playback_mut(playback)?.gain_milli = milli;
        let playback = self.audio_playback_mut(playback)?;
        Self::apply_playback_gain(playback, device_gain)
    }

    fn audio_playback_set_looping(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
        looping: bool,
    ) -> Result<(), String> {
        self.audio_playback_mut(playback)?.looping = looping;
        self.restart_playback(playback)
    }

    fn audio_playback_looping(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        Ok(self.audio_playback_ref(playback)?.looping)
    }

    fn audio_playback_position_frames(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<i64, String> {
        self.sync_playback_state(playback)?;
        let playback = self.audio_playback_ref(playback)?;
        if playback.wave_out.is_null() {
            return Ok(playback.frames);
        }
        let mut position = MMTIME {
            wType: TIME_SAMPLES,
            ..unsafe { zeroed() }
        };
        let status = unsafe {
            waveOutGetPosition(playback.wave_out, &mut position, size_of::<MMTIME>() as u32)
        };
        if status != MMSYSERR_NOERROR {
            return Err(format!(
                "failed to query playback position: {}",
                mmresult_text(status)
            ));
        }
        Ok(unsafe { position.u.sample } as i64)
    }

    fn monotonic_now_ms(&mut self) -> Result<i64, String> {
        i64::try_from(self.started.elapsed().as_millis())
            .map_err(|_| "monotonic millisecond timestamp overflow".to_string())
    }

    fn monotonic_now_ns(&mut self) -> Result<i64, String> {
        i64::try_from(self.started.elapsed().as_nanos())
            .map_err(|_| "monotonic nanosecond timestamp overflow".to_string())
    }

    fn sleep_ms(&mut self, ms: i64) -> Result<(), String> {
        if ms < 0 {
            return Err("sleep_ms expects a non-negative duration".to_string());
        }
        std::thread::sleep(Duration::from_millis(ms as u64));
        Ok(())
    }

    fn process_exec_status(&mut self, program: &str, args: &[String]) -> Result<i64, String> {
        RuntimeHost::process_exec_status(&mut self.base, program, args)
    }

    fn process_exec_capture(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<(i64, Vec<u8>, Vec<u8>, bool, bool), String> {
        RuntimeHost::process_exec_capture(&mut self.base, program, args)
    }
}

impl NativeWindowState {
    fn new(title: &str, width: i64, height: i64) -> Self {
        Self {
            hwnd: null_mut(),
            title: title.to_string(),
            width,
            height,
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
            cursor_position: (0, 0),
            suppress_cursor_move: false,
            message_loop_signaled: false,
            attention_requested: false,
            redraw_pending: false,
            text_input_enabled: false,
            composition_area_active: false,
            composition_area_position: (0, 0),
            composition_area_size: (0, 0),
            composition_active: false,
            composition_committed: false,
            applying_dpi_suggestion: false,
            applying_theme_override: false,
            closed: false,
            restore_style: 0,
            restore_ex_style: 0,
            restore_rect: unsafe { zeroed() },
            surface: CanvasSurface::new(width, height),
            events: VecDeque::new(),
            key_down: BTreeSet::new(),
            key_pressed: BTreeSet::new(),
            key_released: BTreeSet::new(),
            mouse_pos: (0, 0),
            mouse_down: BTreeSet::new(),
            mouse_pressed: BTreeSet::new(),
            mouse_released: BTreeSet::new(),
            mouse_wheel_y: 0,
            mouse_in_window: false,
            pending_high_surrogate: None,
        }
    }

    fn push_event(&mut self, kind: i64, a: i64, b: i64) {
        if self.closed {
            return;
        }
        self.events.push_back(BufferedEvent {
            kind,
            window_id: 0,
            a,
            b,
            flags: frame_modifier_flags(&self.key_down),
            text: String::new(),
            ..BufferedEvent::default()
        });
    }

    fn push_device_event(&mut self, kind: i64, device_id: i64, a: i64, b: i64) {
        if self.closed {
            return;
        }
        self.events.push_back(BufferedEvent {
            kind,
            window_id: device_id,
            a,
            b,
            flags: frame_modifier_flags(&self.key_down),
            text: String::new(),
            ..BufferedEvent::default()
        });
    }

    fn push_redraw_event(&mut self) {
        if self.closed {
            return;
        }
        if self.redraw_pending {
            return;
        }
        self.redraw_pending = true;
        self.push_event(EVENT_WINDOW_REDRAW_REQUESTED, 0, 0);
    }

    fn push_mouse_button_event(&mut self, kind: i64, button: i64, x: i64, y: i64) {
        if self.closed {
            return;
        }
        self.events.push_back(BufferedEvent {
            kind,
            window_id: 0,
            a: button,
            b: 0,
            flags: frame_modifier_flags(&self.key_down),
            text: String::new(),
            pointer_x: x,
            pointer_y: y,
            ..BufferedEvent::default()
        });
    }

    fn push_key_event(
        &mut self,
        kind: i64,
        key_code: i64,
        physical_key: i64,
        logical_key: i64,
        key_location: i64,
        repeated: bool,
        text: String,
    ) {
        if self.closed {
            return;
        }
        self.events.push_back(BufferedEvent {
            kind,
            window_id: 0,
            a: key_code,
            b: 0,
            flags: frame_modifier_flags(&self.key_down),
            text,
            key_code,
            physical_key,
            logical_key,
            key_location,
            pointer_x: self.mouse_pos.0,
            pointer_y: self.mouse_pos.1,
            repeated,
        });
    }

    fn push_text_event(&mut self, kind: i64, text: String) {
        if self.closed {
            return;
        }
        self.events.push_back(BufferedEvent {
            kind,
            window_id: 0,
            a: 0,
            b: 0,
            flags: frame_modifier_flags(&self.key_down),
            text,
            pointer_x: self.mouse_pos.0,
            pointer_y: self.mouse_pos.1,
            ..BufferedEvent::default()
        });
    }

    fn push_raw_key_event(
        &mut self,
        device_id: i64,
        key_code: i64,
        physical_key: i64,
        logical_key: i64,
        key_location: i64,
        pressed: bool,
        text: String,
    ) {
        if self.closed {
            return;
        }
        self.events.push_back(BufferedEvent {
            kind: EVENT_RAW_KEY,
            window_id: device_id,
            a: 0,
            b: if pressed { 1 } else { 0 },
            flags: frame_modifier_flags(&self.key_down),
            text,
            key_code,
            physical_key,
            logical_key,
            key_location,
            pointer_x: self.mouse_pos.0,
            pointer_y: self.mouse_pos.1,
            repeated: false,
        });
    }

    fn push_composition_event(&mut self, kind: i64, text: String, caret: i64) {
        if self.closed {
            return;
        }
        self.events.push_back(BufferedEvent {
            kind,
            window_id: 0,
            a: caret,
            b: 0,
            flags: frame_modifier_flags(&self.key_down),
            text,
            pointer_x: self.mouse_pos.0,
            pointer_y: self.mouse_pos.1,
            ..BufferedEvent::default()
        });
    }

    fn signal_message_loop(&mut self) {
        if self.closed || self.hwnd.is_null() {
            return;
        }
        if self.message_loop_signaled {
            return;
        }
        self.message_loop_signaled = true;
        unsafe {
            PostMessageW(self.hwnd, WM_NULL, 0, 0);
        }
    }

    fn push_text_input_code_unit(&mut self, code_unit: u16) {
        if (0xD800..=0xDBFF).contains(&code_unit) {
            self.pending_high_surrogate = Some(code_unit);
            return;
        }
        let mut units = Vec::new();
        if let Some(high) = self.pending_high_surrogate.take() {
            if (0xDC00..=0xDFFF).contains(&code_unit) {
                units.push(high);
                units.push(code_unit);
            } else {
                units.push(high);
                if is_text_input_code_unit(code_unit) {
                    units.push(code_unit);
                }
            }
        } else if is_text_input_code_unit(code_unit) {
            units.push(code_unit);
        }
        if units.is_empty() {
            return;
        }
        let text = String::from_utf16_lossy(&units);
        if !text.is_empty() {
            self.push_text_event(EVENT_TEXT_INPUT, text);
        }
    }
}

fn frame_modifier_flags(keys: &BTreeSet<i64>) -> i64 {
    let mut flags = 0;
    if keys.contains(&common_named_key_code("Shift")) {
        flags |= 1;
    }
    if keys.contains(&common_named_key_code("Control")) {
        flags |= 2;
    }
    if keys.contains(&common_named_key_code("Alt")) {
        flags |= 4;
    }
    if keys.contains(&common_named_key_code("Meta")) {
        flags |= 8;
    }
    flags
}

fn prioritize_close_requested_events(events: &mut Vec<BufferedEvent>) {
    let Some(index) = events
        .iter()
        .position(|event| event.kind == EVENT_WINDOW_CLOSE_REQUESTED)
    else {
        return;
    };
    if index == 0 {
        return;
    }
    let event = events.remove(index);
    events.insert(0, event);
}

fn append_unique(target: &mut Vec<i64>, values: Vec<i64>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

fn key_physical_code(lparam: LPARAM) -> i64 {
    let scancode = ((lparam >> 16) & 0xff) as i64;
    if (lparam & 0x0100_0000) != 0 {
        scancode | 0xE000
    } else {
        scancode
    }
}

fn raw_key_lparam(make_code: u16, flags: u16) -> LPARAM {
    let mut lparam = (i64::from(make_code) & 0xff) << 16;
    if flags & RAW_KEY_E0 != 0 {
        lparam |= 0x0100_0000;
    }
    lparam as LPARAM
}

fn raw_key_physical_code(make_code: u16, flags: u16) -> i64 {
    key_physical_code(raw_key_lparam(make_code, flags))
}

fn raw_key_location_code(vkey: u32, make_code: u16, flags: u16) -> i64 {
    key_location_code(vkey, raw_key_lparam(make_code, flags))
}

fn raw_input_device_id(handle: *mut c_void) -> i64 {
    if handle.is_null() {
        0
    } else {
        handle as usize as i64
    }
}

fn raw_mouse_button_events(flags: u16) -> Vec<(i64, bool)> {
    let mut out = Vec::new();
    if flags & RAW_MOUSE_LEFT_BUTTON_DOWN != 0 {
        out.push((1, true));
    }
    if flags & RAW_MOUSE_LEFT_BUTTON_UP != 0 {
        out.push((1, false));
    }
    if flags & RAW_MOUSE_RIGHT_BUTTON_DOWN != 0 {
        out.push((2, true));
    }
    if flags & RAW_MOUSE_RIGHT_BUTTON_UP != 0 {
        out.push((2, false));
    }
    if flags & RAW_MOUSE_MIDDLE_BUTTON_DOWN != 0 {
        out.push((3, true));
    }
    if flags & RAW_MOUSE_MIDDLE_BUTTON_UP != 0 {
        out.push((3, false));
    }
    if flags & RAW_MOUSE_BUTTON_4_DOWN != 0 {
        out.push((4, true));
    }
    if flags & RAW_MOUSE_BUTTON_4_UP != 0 {
        out.push((4, false));
    }
    if flags & RAW_MOUSE_BUTTON_5_DOWN != 0 {
        out.push((5, true));
    }
    if flags & RAW_MOUSE_BUTTON_5_UP != 0 {
        out.push((5, false));
    }
    out
}

fn raw_mouse_wheel_delta(flags: u16, data: u16) -> Option<(i64, i64)> {
    let value = i64::from(i16::from_ne_bytes(data.to_ne_bytes()));
    if flags & RAW_MOUSE_WHEEL != 0 {
        return Some((0, value));
    }
    if flags & RAW_MOUSE_HWHEEL != 0 {
        return Some((value, 0));
    }
    None
}

fn key_location_code(vkey: u32, lparam: LPARAM) -> i64 {
    const KEY_LOCATION_STANDARD: i64 = 0;
    const KEY_LOCATION_LEFT: i64 = 1;
    const KEY_LOCATION_RIGHT: i64 = 2;
    const KEY_LOCATION_NUMPAD: i64 = 3;

    let extended = (lparam & 0x0100_0000) != 0;
    match vkey {
        0x10 => {
            let scancode = ((lparam >> 16) & 0xff) as u32;
            let resolved = unsafe { MapVirtualKeyW(scancode, MAPVK_VSC_TO_VK_EX) };
            match resolved {
                0xA0 => KEY_LOCATION_LEFT,
                0xA1 => KEY_LOCATION_RIGHT,
                _ => KEY_LOCATION_STANDARD,
            }
        }
        0x11 | 0x12 => {
            if extended {
                KEY_LOCATION_RIGHT
            } else {
                KEY_LOCATION_LEFT
            }
        }
        0x0D if extended => KEY_LOCATION_NUMPAD,
        0x60..=0x6F => KEY_LOCATION_NUMPAD,
        _ => KEY_LOCATION_STANDARD,
    }
}

fn key_logical_value_and_text(vkey: u32) -> (i64, String) {
    let mapped = unsafe { MapVirtualKeyW(vkey, MAPVK_VK_TO_CHAR) };
    let mapped = mapped & 0x7fff_ffff;
    let Some(ch) = char::from_u32(mapped) else {
        return (i64::from(vkey), String::new());
    };
    if ch.is_control() {
        return (i64::from(vkey), String::new());
    }
    let text = ch.to_string();
    (i64::from(mapped), text)
}

fn is_text_input_code_unit(code_unit: u16) -> bool {
    let Some(ch) = char::from_u32(u32::from(code_unit)) else {
        return false;
    };
    !ch.is_control() || matches!(ch, '\n' | '\r' | '\t')
}

fn native_cursor_handle(code: i64) -> HCURSOR {
    let name = match code {
        1 => IDC_IBEAM,
        2 => IDC_CROSS,
        3 => IDC_HAND,
        4 => IDC_SIZEALL,
        5 => IDC_WAIT,
        6 => IDC_HELP,
        7 => IDC_NO,
        8 => IDC_SIZEWE,
        9 => IDC_SIZENS,
        10 => IDC_SIZENWSE,
        11 => IDC_SIZENESW,
        _ => IDC_ARROW,
    };
    unsafe { LoadCursorW(null_mut(), name) as HCURSOR }
}

fn apply_window_cursor_state(hwnd: HWND, cursor_visible: bool, cursor_icon_code: i64) {
    if hwnd.is_null() {
        return;
    }
    if !cursor_visible {
        unsafe { SetCursor(null_mut()) };
        return;
    }
    unsafe { SetCursor(native_cursor_handle(cursor_icon_code)) };
}

fn client_rect_screen(hwnd: HWND) -> Result<RECT, String> {
    let mut rect = unsafe { zeroed::<RECT>() };
    if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
        return Err("failed to query native client rect".to_string());
    }
    let mut origin = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut corner = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    if unsafe { ClientToScreen(hwnd, &mut origin) } == 0
        || unsafe { ClientToScreen(hwnd, &mut corner) } == 0
    {
        return Err("failed to translate native client rect to screen coordinates".to_string());
    }
    Ok(RECT {
        left: origin.x,
        top: origin.y,
        right: corner.x,
        bottom: corner.y,
    })
}

fn apply_cursor_grab(hwnd: HWND, mode: i64) -> Result<(), String> {
    match mode {
        1 | 2 => {
            let rect = client_rect_screen(hwnd)?;
            if unsafe { ClipCursor(&rect) } == 0 {
                return Err("failed to confine native cursor".to_string());
            }
        }
        _ => {
            if unsafe { ClipCursor(std::ptr::null()) } == 0 {
                return Err("failed to release native cursor confinement".to_string());
            }
        }
    }
    Ok(())
}

fn apply_theme_override(hwnd: HWND, code: i64) {
    let value: i32 = if code == 2 { 1 } else { 0 };
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &value as *const i32 as *const c_void,
            size_of::<i32>() as u32,
        );
    }
}

fn apply_composition_area(
    hwnd: HWND,
    _active: bool,
    position: (i64, i64),
    size: (i64, i64),
) -> Result<(), String> {
    let himc: HIMC = unsafe { ImmGetContext(hwnd) };
    if himc.is_null() {
        return Ok(());
    }
    let result = (|| {
        let comp = COMPOSITIONFORM {
            dwStyle: CFS_FORCE_POSITION,
            ptCurrentPos: POINT {
                x: i32::try_from(position.0)
                    .map_err(|_| "composition x does not fit in i32".to_string())?,
                y: i32::try_from(position.1)
                    .map_err(|_| "composition y does not fit in i32".to_string())?,
            },
            rcArea: RECT {
                left: i32::try_from(position.0)
                    .map_err(|_| "composition left does not fit in i32".to_string())?,
                top: i32::try_from(position.1)
                    .map_err(|_| "composition top does not fit in i32".to_string())?,
                right: i32::try_from(position.0.saturating_add(size.0))
                    .map_err(|_| "composition right does not fit in i32".to_string())?,
                bottom: i32::try_from(position.1.saturating_add(size.1))
                    .map_err(|_| "composition bottom does not fit in i32".to_string())?,
            },
        };
        let candidate = CANDIDATEFORM {
            dwIndex: 0,
            dwStyle: CFS_CANDIDATEPOS,
            ptCurrentPos: comp.ptCurrentPos,
            rcArea: comp.rcArea,
        };
        if unsafe { ImmSetCompositionWindow(himc, &comp) } == 0 {
            return Err("failed to update IME composition window".to_string());
        }
        if unsafe { ImmSetCandidateWindow(himc, &candidate) } == 0 {
            return Err("failed to update IME candidate window".to_string());
        }
        Ok(())
    })();
    unsafe {
        ImmReleaseContext(hwnd, himc);
    }
    result
}

fn read_ime_composition_string(himc: HIMC, which: u32) -> Result<Option<String>, String> {
    let len = unsafe { ImmGetCompositionStringW(himc, which, null_mut(), 0) };
    if len < 0 {
        return Err("failed to read IME composition string".to_string());
    }
    if len == 0 {
        return Ok(None);
    }
    let units_len =
        usize::try_from(len / 2).map_err(|_| "IME composition length overflow".to_string())?;
    let mut buffer = vec![0u16; units_len];
    let read = unsafe {
        ImmGetCompositionStringW(himc, which, buffer.as_mut_ptr() as *mut c_void, len as u32)
    };
    if read < 0 {
        return Err("failed to load IME composition payload".to_string());
    }
    Ok(Some(String::from_utf16_lossy(&buffer)))
}

fn read_ime_cursor_position(himc: HIMC) -> Result<i64, String> {
    let cursor = unsafe { ImmGetCompositionStringW(himc, GCS_CURSORPOS, null_mut(), 0) };
    if cursor < 0 {
        return Err("failed to read IME composition cursor position".to_string());
    }
    Ok(i64::from(cursor))
}

fn theme_code_from_registry_value(light_theme: u32) -> i64 {
    if light_theme == 0 { 2 } else { 1 }
}

fn current_windows_theme_code() -> i64 {
    let subkey = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = wide_null("AppsUseLightTheme");
    let mut raw = 0u32;
    let mut size = size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut raw as *mut u32 as *mut c_void,
            &mut size,
        )
    };
    if status != 0 {
        return 0;
    }
    theme_code_from_registry_value(raw)
}

unsafe extern "system" fn collect_monitor_handle_proc(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> i32 {
    let handles = unsafe { &mut *(lparam as *mut Vec<HMONITOR>) };
    handles.push(monitor);
    1
}

fn query_monitor_scale_factor_milli(monitor: HMONITOR) -> i64 {
    let mut dpi_x = 0u32;
    let mut dpi_y = 0u32;
    let status = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    if status >= 0 && dpi_x > 0 {
        (i64::from(dpi_x) * 1000) / 96
    } else {
        1000
    }
}

fn wide_units_to_string(units: &[u16]) -> String {
    let end = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

impl NativeAudioPlayback {
    fn new(
        device: RuntimeAudioDeviceHandle,
        wave_out: HWAVEOUT,
        frames: i64,
        pcm_bytes: Vec<u8>,
    ) -> Self {
        let mut header = Box::new(unsafe { zeroed::<WAVEHDR>() });
        header.lpData = pcm_bytes.as_ptr() as *mut u8;
        header.dwBufferLength = pcm_bytes.len() as u32;
        Self {
            device,
            wave_out,
            header,
            pcm_bytes,
            frames,
            paused: false,
            finished: false,
            gain_milli: 1000,
            looping: false,
            header_prepared: false,
        }
    }
}

impl CanvasSurface {
    fn new(width: i64, height: i64) -> Self {
        let width = sanitize_dimension(width);
        let height = sanitize_dimension(height);
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }

    fn resize(&mut self, width: i64, height: i64) {
        let width = sanitize_dimension(width);
        let height = sanitize_dimension(height);
        self.width = width;
        self.height = height;
        self.pixels.resize((width * height) as usize, 0);
    }

    fn fill(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    fn fill_rect(&mut self, x: i64, y: i64, w: i64, h: i64, color: u32) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width).max(x0);
        let y1 = (y + h).min(self.height).max(y0);
        if x0 == x1 || y0 == y1 {
            return;
        }
        let row_width = self.width as usize;
        let x0 = x0 as usize;
        let x1 = x1 as usize;
        for py in y0..y1 {
            let row_start = py as usize * row_width;
            self.pixels[row_start + x0..row_start + x1].fill(color);
        }
    }

    fn draw_line(&mut self, mut x0: i64, mut y0: i64, x1: i64, y1: i64, color: u32) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn fill_circle(&mut self, cx: i64, cy: i64, radius: i64, color: u32) {
        let radius = radius.max(0);
        let radius_sq = radius * radius;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius_sq {
                    self.set_pixel(cx + dx, cy + dy, color);
                }
            }
        }
    }

    fn draw_label(&mut self, x: i64, y: i64, text: &str, color: u32) {
        text_engine::paint_plain_text(
            self,
            x,
            y,
            text,
            i64::from(color),
            text_engine::DEFAULT_FONT_SIZE,
        );
    }

    fn blit(
        &mut self,
        source: &NativeImage,
        sx: i64,
        sy: i64,
        sw: i64,
        sh: i64,
        dx: i64,
        dy: i64,
        dw: i64,
        dh: i64,
    ) {
        if sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0 {
            return;
        }
        for out_y in 0..dh {
            for out_x in 0..dw {
                let sample_x = sx + (out_x * sw) / dw;
                let sample_y = sy + (out_y * sh) / dh;
                if sample_x < 0
                    || sample_y < 0
                    || sample_x >= source.width
                    || sample_y >= source.height
                {
                    continue;
                }
                let source_index = (sample_y * source.width + sample_x) as usize;
                if let Some(&pixel) = source.pixels.get(source_index) {
                    self.set_pixel(dx + out_x, dy + out_y, pixel);
                }
            }
        }
    }

    fn set_pixel(&mut self, x: i64, y: i64, color: u32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let index = (y * self.width + x) as usize;
        if let Some(slot) = self.pixels.get_mut(index) {
            *slot = color;
        }
    }
}

impl text_engine::TextPaintSink for CanvasSurface {
    fn fill_rect(&mut self, x: i64, y: i64, w: i64, h: i64, color: i64) {
        self.fill_rect(x, y, w, h, pack_color(color));
    }
}

unsafe extern "system" fn arcana_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() {
            let state = unsafe { (*create).lpCreateParams as *mut NativeWindowState };
            if !state.is_null() {
                unsafe {
                    (*state).hwnd = hwnd;
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
                }
            }
        }
        return 1;
    }

    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeWindowState };
    if state_ptr.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    let state = unsafe { &mut *state_ptr };
    if state.closed {
        return match message {
            WM_DESTROY => {
                let _ = apply_cursor_grab(hwnd, 0);
                0
            }
            WM_NULL => {
                state.message_loop_signaled = false;
                0
            }
            WM_NCDESTROY => {
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        };
    }
    match message {
        WM_NULL => {
            state.message_loop_signaled = false;
            0
        }
        WM_PAINT => {
            let mut paint = unsafe { zeroed::<PAINTSTRUCT>() };
            unsafe {
                BeginPaint(hwnd, &mut paint);
                EndPaint(hwnd, &paint);
            }
            state.push_redraw_event();
            0
        }
        WM_GETMINMAXINFO => {
            let info = lparam as *mut MINMAXINFO;
            if info.is_null() {
                return 0;
            }
            let info = unsafe { &mut *info };
            if state.min_size.0 > 0 {
                info.ptMinTrackSize.x = i32::try_from(state.min_size.0).unwrap_or(i32::MAX);
            }
            if state.min_size.1 > 0 {
                info.ptMinTrackSize.y = i32::try_from(state.min_size.1).unwrap_or(i32::MAX);
            }
            if state.max_size.0 > 0 {
                info.ptMaxTrackSize.x = i32::try_from(state.max_size.0).unwrap_or(i32::MAX);
            }
            if state.max_size.1 > 0 {
                info.ptMaxTrackSize.y = i32::try_from(state.max_size.1).unwrap_or(i32::MAX);
            }
            0
        }
        WM_SIZE => {
            let width = loword(lparam as u32) as i64;
            let height = hiword(lparam as u32) as i64;
            state.width = width.max(1);
            state.height = height.max(1);
            state.surface.resize(state.width, state.height);
            state.minimized = wparam as u32 == SIZE_MINIMIZED;
            state.maximized = wparam as u32 == SIZE_MAXIMIZED;
            state.resized = true;
            state.push_event(EVENT_WINDOW_RESIZED, state.width, state.height);
            state.push_redraw_event();
            0
        }
        WM_MOVE => {
            let x = signed_loword(lparam as u32) as i64;
            let y = signed_hiword(lparam as u32) as i64;
            state.push_event(EVENT_WINDOW_MOVED, x, y);
            0
        }
        WM_SETFOCUS => {
            state.focused = true;
            state.attention_requested = false;
            if state.cursor_grab_mode != 0 {
                let _ = apply_cursor_grab(hwnd, state.cursor_grab_mode);
            }
            if state.composition_area_active {
                let _ = apply_composition_area(
                    hwnd,
                    true,
                    state.composition_area_position,
                    state.composition_area_size,
                );
            }
            state.push_event(EVENT_WINDOW_FOCUSED, 1, 0);
            0
        }
        WM_KILLFOCUS => {
            state.focused = false;
            if state.cursor_grab_mode != 0 {
                let _ = apply_cursor_grab(hwnd, 0);
            }
            state.push_event(EVENT_WINDOW_FOCUSED, 0, 0);
            0
        }
        WM_KEYDOWN => {
            let key = wparam as i64;
            let repeated = (lparam & 0x4000_0000) != 0;
            let physical_key = key_physical_code(lparam);
            let key_location = key_location_code(wparam as u32, lparam);
            let (logical_key, text) = key_logical_value_and_text(wparam as u32);
            if state.key_down.insert(key) {
                state.key_pressed.insert(key);
            }
            state.push_key_event(
                EVENT_KEY_DOWN,
                key,
                physical_key,
                logical_key,
                key_location,
                repeated,
                text,
            );
            0
        }
        WM_SYSKEYDOWN => {
            let key = wparam as i64;
            let repeated = (lparam & 0x4000_0000) != 0;
            let physical_key = key_physical_code(lparam);
            let key_location = key_location_code(wparam as u32, lparam);
            let (logical_key, text) = key_logical_value_and_text(wparam as u32);
            if state.key_down.insert(key) {
                state.key_pressed.insert(key);
            }
            state.push_key_event(
                EVENT_KEY_DOWN,
                key,
                physical_key,
                logical_key,
                key_location,
                repeated,
                text,
            );
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_KEYUP => {
            let key = wparam as i64;
            let physical_key = key_physical_code(lparam);
            let key_location = key_location_code(wparam as u32, lparam);
            let (logical_key, text) = key_logical_value_and_text(wparam as u32);
            state.key_down.remove(&key);
            state.key_released.insert(key);
            state.push_key_event(
                EVENT_KEY_UP,
                key,
                physical_key,
                logical_key,
                key_location,
                false,
                text,
            );
            0
        }
        WM_SYSKEYUP => {
            let key = wparam as i64;
            let physical_key = key_physical_code(lparam);
            let key_location = key_location_code(wparam as u32, lparam);
            let (logical_key, text) = key_logical_value_and_text(wparam as u32);
            state.key_down.remove(&key);
            state.key_released.insert(key);
            state.push_key_event(
                EVENT_KEY_UP,
                key,
                physical_key,
                logical_key,
                key_location,
                false,
                text,
            );
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_MOUSEMOVE => {
            let x = signed_loword(lparam as u32) as i64;
            let y = signed_hiword(lparam as u32) as i64;
            if state.suppress_cursor_move {
                state.suppress_cursor_move = false;
                return 0;
            }
            state.mouse_pos = (x, y);
            state.cursor_position = (x, y);
            let entering = !state.mouse_in_window;
            state.mouse_in_window = true;
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            unsafe { TrackMouseEvent(&mut track) };
            if entering {
                state.push_event(EVENT_MOUSE_ENTERED, 0, 0);
            }
            state.push_event(EVENT_MOUSE_MOVE, x, y);
            if state.cursor_grab_mode == 2 {
                let center = (
                    state.width.saturating_div(2),
                    state.height.saturating_div(2),
                );
                let mut point = POINT {
                    x: i32::try_from(center.0).unwrap_or(0),
                    y: i32::try_from(center.1).unwrap_or(0),
                };
                if unsafe { ClientToScreen(hwnd, &mut point) } != 0 {
                    state.suppress_cursor_move = true;
                    unsafe {
                        SetCursorPos(point.x, point.y);
                    }
                    state.cursor_position = center;
                    state.mouse_pos = center;
                }
            }
            0
        }
        WM_MOUSELEAVE_MESSAGE => {
            state.mouse_in_window = false;
            state.push_event(EVENT_MOUSE_LEFT, 0, 0);
            0
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
            let button = mouse_button_from_message(message, wparam as u32);
            let x = signed_loword(lparam as u32) as i64;
            let y = signed_hiword(lparam as u32) as i64;
            state.mouse_pos = (x, y);
            state.cursor_position = (x, y);
            state.mouse_down.insert(button);
            state.mouse_pressed.insert(button);
            state.push_mouse_button_event(EVENT_MOUSE_DOWN, button, x, y);
            0
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
            let button = mouse_button_from_message(message, wparam as u32);
            let x = signed_loword(lparam as u32) as i64;
            let y = signed_hiword(lparam as u32) as i64;
            state.mouse_pos = (x, y);
            state.cursor_position = (x, y);
            state.mouse_down.remove(&button);
            state.mouse_released.insert(button);
            state.push_mouse_button_event(EVENT_MOUSE_UP, button, x, y);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = signed_hiword(wparam as u32) as i64 / 120;
            state.mouse_wheel_y += delta;
            state.push_event(EVENT_MOUSE_WHEEL_Y, delta, 0);
            0
        }
        WM_CHAR => {
            if state.text_input_enabled {
                state.push_text_input_code_unit(wparam as u16);
            }
            0
        }
        WM_IME_STARTCOMPOSITION => {
            if state.text_input_enabled {
                state.composition_active = true;
                state.composition_committed = false;
                state.push_event(EVENT_TEXT_COMPOSITION_STARTED, 0, 0);
            }
            0
        }
        WM_IME_COMPOSITION => {
            if !state.text_input_enabled {
                return 0;
            }
            let himc = unsafe { ImmGetContext(hwnd) };
            if himc.is_null() {
                return 0;
            }
            let _ = (|| -> Result<(), String> {
                if (lparam as u32 & GCS_COMPSTR) != 0
                    && let Some(text) = read_ime_composition_string(himc, GCS_COMPSTR)?
                {
                    let caret = read_ime_cursor_position(himc).unwrap_or(0);
                    state.push_composition_event(EVENT_TEXT_COMPOSITION_UPDATED, text, caret);
                }
                if (lparam as u32 & GCS_RESULTSTR) != 0
                    && let Some(text) = read_ime_composition_string(himc, GCS_RESULTSTR)?
                {
                    state.composition_committed = true;
                    state.push_composition_event(EVENT_TEXT_COMPOSITION_COMMITTED, text, 0);
                }
                Ok(())
            })();
            unsafe {
                ImmReleaseContext(hwnd, himc);
            }
            0
        }
        WM_IME_ENDCOMPOSITION => {
            if state.text_input_enabled {
                if state.composition_active && !state.composition_committed {
                    state.push_event(EVENT_TEXT_COMPOSITION_CANCELLED, 0, 0);
                }
                state.composition_active = false;
                state.composition_committed = false;
            }
            0
        }
        WM_DPICHANGED => {
            let dpi = loword(wparam as u32) as i64;
            let suggested = lparam as *const RECT;
            if !suggested.is_null() && !state.applying_dpi_suggestion {
                let suggested = unsafe { &*suggested };
                let mut current = unsafe { zeroed::<RECT>() };
                unsafe {
                    GetWindowRect(hwnd, &mut current);
                }
                if current.left != suggested.left
                    || current.top != suggested.top
                    || current.right != suggested.right
                    || current.bottom != suggested.bottom
                {
                    state.applying_dpi_suggestion = true;
                    unsafe {
                        SetWindowPos(
                            hwnd,
                            null_mut(),
                            suggested.left,
                            suggested.top,
                            suggested.right - suggested.left,
                            suggested.bottom - suggested.top,
                            SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                        );
                    }
                    state.applying_dpi_suggestion = false;
                }
            }
            state.push_event(EVENT_WINDOW_SCALE_FACTOR_CHANGED, (dpi * 1000) / 96, 0);
            0
        }
        WM_THEMECHANGED => {
            if state.theme_override_code != 0 && !state.applying_theme_override {
                state.applying_theme_override = true;
                apply_theme_override(hwnd, state.theme_override_code);
                state.applying_theme_override = false;
            }
            state.push_event(EVENT_WINDOW_THEME_CHANGED, current_windows_theme_code(), 0);
            0
        }
        WM_DROPFILES => {
            let drop = wparam as HDROP;
            let count = unsafe { DragQueryFileW(drop, u32::MAX, null_mut(), 0) };
            for index in 0..count {
                let len = unsafe { DragQueryFileW(drop, index, null_mut(), 0) };
                if len == 0 {
                    continue;
                }
                let mut buffer = vec![0u16; len as usize + 1];
                let copied = unsafe { DragQueryFileW(drop, index, buffer.as_mut_ptr(), len + 1) };
                let text = String::from_utf16_lossy(&buffer[..copied as usize]);
                state.push_text_event(EVENT_FILE_DROPPED, text);
            }
            unsafe {
                DragFinish(drop);
            }
            0
        }
        WM_INPUT => {
            let mut size = 0u32;
            let header_size = size_of::<windows_sys::Win32::UI::Input::RAWINPUTHEADER>() as u32;
            let queried = unsafe {
                GetRawInputData(
                    lparam as HRAWINPUT,
                    RID_INPUT,
                    null_mut(),
                    &mut size,
                    header_size,
                )
            };
            if queried == u32::MAX || size == 0 {
                return 0;
            }
            let mut buffer = vec![0u8; size as usize];
            let read = unsafe {
                GetRawInputData(
                    lparam as HRAWINPUT,
                    RID_INPUT,
                    buffer.as_mut_ptr() as *mut c_void,
                    &mut size,
                    header_size,
                )
            };
            if read == u32::MAX || read == 0 {
                return 0;
            }
            let raw = unsafe { &*(buffer.as_ptr() as *const RAWINPUT) };
            let device_id = raw_input_device_id(raw.header.hDevice);
            let mut pending = pending_raw_input_events().lock().ok();
            if raw.header.dwType == RIM_TYPEMOUSE {
                let mouse = unsafe { raw.data.mouse };
                if (mouse.usFlags == MOUSE_MOVE_RELATIVE || mouse.lLastX != 0 || mouse.lLastY != 0)
                    && let Some(pending) = pending.as_mut()
                {
                    push_pending_raw_input_event(
                        pending,
                        PendingRawInputEvent::MouseMotion {
                            device_id,
                            dx: i64::from(mouse.lLastX),
                            dy: i64::from(mouse.lLastY),
                        },
                    );
                }
                let button_flags = unsafe { mouse.Anonymous.Anonymous.usButtonFlags };
                let button_data = unsafe { mouse.Anonymous.Anonymous.usButtonData };
                for (button, pressed) in raw_mouse_button_events(button_flags) {
                    if let Some(pending) = pending.as_mut() {
                        push_pending_raw_input_event(
                            pending,
                            PendingRawInputEvent::MouseButton {
                                device_id,
                                button,
                                pressed,
                            },
                        );
                    }
                }
                if let Some((dx, dy)) = raw_mouse_wheel_delta(button_flags, button_data)
                    && let Some(pending) = pending.as_mut()
                {
                    push_pending_raw_input_event(
                        pending,
                        PendingRawInputEvent::MouseWheel { device_id, dx, dy },
                    );
                }
            } else if raw.header.dwType == RIM_TYPEKEYBOARD {
                let keyboard = unsafe { raw.data.keyboard };
                let vkey = keyboard.VKey as u32;
                if vkey != 0 && vkey != 0xff {
                    let physical_key = raw_key_physical_code(keyboard.MakeCode, keyboard.Flags);
                    let key_location =
                        raw_key_location_code(vkey, keyboard.MakeCode, keyboard.Flags);
                    let (logical_key, text) = key_logical_value_and_text(vkey);
                    let pressed = keyboard.Flags & RAW_KEY_BREAK == 0;
                    if let Some(pending) = pending.as_mut() {
                        push_pending_raw_input_event(
                            pending,
                            PendingRawInputEvent::Key {
                                device_id,
                                key_code: i64::from(vkey),
                                physical_key,
                                logical_key,
                                key_location,
                                pressed,
                                text,
                            },
                        );
                    }
                }
            }
            0
        }
        WM_CLOSE => {
            state.push_event(EVENT_WINDOW_CLOSE_REQUESTED, 0, 0);
            0
        }
        WM_DESTROY => {
            let _ = apply_cursor_grab(hwnd, 0);
            state.closed = true;
            0
        }
        WM_SETCURSOR => {
            if !state.cursor_visible && loword(lparam as u32) as u32 == HTCLIENT {
                unsafe { SetCursor(null_mut()) };
                return 1;
            }
            if loword(lparam as u32) as u32 == HTCLIENT {
                unsafe { SetCursor(native_cursor_handle(state.cursor_icon_code)) };
                return 1;
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        WM_NCDESTROY => {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            state.closed = true;
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn blit_surface_to_dc(surface: &CanvasSurface, hwnd: HWND, dc: HDC) -> Result<(), String> {
    if dc.is_null() {
        return Err("failed to acquire native draw context".to_string());
    }
    let mut client = unsafe { zeroed::<RECT>() };
    unsafe { GetClientRect(hwnd, &mut client) };
    let mut info = unsafe { zeroed::<BITMAPINFO>() };
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: surface.width as i32,
        biHeight: -(surface.height as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { zeroed() }
    };
    let copied = unsafe {
        StretchDIBits(
            dc,
            0,
            0,
            client.right - client.left,
            client.bottom - client.top,
            0,
            0,
            surface.width as i32,
            surface.height as i32,
            surface.pixels.as_ptr() as *const c_void,
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
    if copied == 0 {
        return Err("failed to present native canvas surface".to_string());
    }
    Ok(())
}

fn prepare_and_queue_playback(playback: &mut NativeAudioPlayback) -> Result<(), String> {
    playback.header.lpData = playback.pcm_bytes.as_ptr() as *mut u8;
    playback.header.dwBufferLength = playback.pcm_bytes.len() as u32;
    playback.header.dwFlags = 0;
    if playback.looping {
        playback.header.dwFlags |= WHDR_BEGINLOOP | WHDR_ENDLOOP;
        playback.header.dwLoops = u32::MAX;
    } else {
        playback.header.dwLoops = 0;
    }
    let prepare_status = unsafe {
        waveOutPrepareHeader(
            playback.wave_out,
            &mut *playback.header,
            size_of::<WAVEHDR>() as u32,
        )
    };
    if prepare_status != MMSYSERR_NOERROR {
        return Err(format!(
            "failed to prepare playback buffer: {}",
            mmresult_text(prepare_status)
        ));
    }
    playback.header_prepared = true;
    let write_status = unsafe {
        waveOutWrite(
            playback.wave_out,
            &mut *playback.header,
            size_of::<WAVEHDR>() as u32,
        )
    };
    if write_status != MMSYSERR_NOERROR {
        cleanup_prepared_playback(playback)?;
        return Err(format!(
            "failed to queue playback buffer: {}",
            mmresult_text(write_status)
        ));
    }
    playback.finished = false;
    Ok(())
}

fn cleanup_prepared_playback(playback: &mut NativeAudioPlayback) -> Result<(), String> {
    if !playback.header_prepared || playback.wave_out.is_null() {
        playback.header_prepared = false;
        return Ok(());
    }
    let status = unsafe {
        waveOutUnprepareHeader(
            playback.wave_out,
            &mut *playback.header,
            size_of::<WAVEHDR>() as u32,
        )
    };
    if status != MMSYSERR_NOERROR {
        return Err(format!(
            "failed to release playback buffer: {}",
            mmresult_text(status)
        ));
    }
    playback.header_prepared = false;
    Ok(())
}

fn release_playback_resources(playback: &mut NativeAudioPlayback) -> Result<(), String> {
    if !playback.wave_out.is_null() {
        unsafe {
            waveOutReset(playback.wave_out);
        }
        cleanup_prepared_playback(playback)?;
        let status = unsafe { waveOutClose(playback.wave_out) };
        if status != MMSYSERR_NOERROR {
            return Err(format!(
                "failed to close playback device: {}",
                mmresult_text(status)
            ));
        }
        playback.wave_out = null_mut();
    }
    playback.header.lpData = null_mut();
    playback.header.dwBufferLength = 0;
    playback.header.dwFlags = 0;
    playback.header.dwLoops = 0;
    playback.pcm_bytes.clear();
    playback.pcm_bytes.shrink_to_fit();
    playback.finished = true;
    playback.paused = false;
    Ok(())
}

fn playback_error_with_cleanup(playback: &mut NativeAudioPlayback, err: String) -> String {
    match release_playback_resources(playback) {
        Ok(()) => err,
        Err(cleanup_err) => format!("{err}; cleanup failed: {cleanup_err}"),
    }
}

fn pcm_wave_format(sample_rate_hz: i64, channels: i64) -> Result<WAVEFORMATEX, String> {
    let channels = u16::try_from(channels)
        .map_err(|_| format!("audio channel count `{channels}` does not fit in u16"))?;
    let sample_rate_hz = u32::try_from(sample_rate_hz)
        .map_err(|_| format!("audio sample rate `{sample_rate_hz}` does not fit in u32"))?;
    let block_align = channels
        .checked_mul(2)
        .ok_or_else(|| "audio block alignment overflow".to_string())?;
    Ok(WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_PCM as u16,
        nChannels: channels,
        nSamplesPerSec: sample_rate_hz,
        nAvgBytesPerSec: sample_rate_hz
            .checked_mul(u32::from(block_align))
            .ok_or_else(|| "audio byte-rate overflow".to_string())?,
        nBlockAlign: block_align,
        wBitsPerSample: 16,
        cbSize: 0,
    })
}

fn decode_wav_buffer(path: &Path) -> Result<NativeAudioBuffer, String> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("failed to read wav `{}`: {err}", path.display()))?;
    decode_wav_bytes(&bytes, path)
}

fn decode_wav_bytes(bytes: &[u8], path: &Path) -> Result<NativeAudioBuffer, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(format!("`{}` is not a RIFF/WAVE file", path.display()));
    }
    let mut offset = 12usize;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let kind = &bytes[offset..offset + 4];
        let size = read_u32_le(&bytes[offset + 4..offset + 8]) as usize;
        offset += 8;
        let end = offset
            .checked_add(size)
            .ok_or_else(|| format!("wav `{}` chunk length overflow", path.display()))?;
        if end > bytes.len() {
            return Err(format!("wav `{}` has a truncated chunk", path.display()));
        }
        match kind {
            b"fmt " => format = Some(bytes[offset..end].to_vec()),
            b"data" => data = Some(bytes[offset..end].to_vec()),
            _ => {}
        }
        offset = end
            .checked_add(size % 2)
            .ok_or_else(|| format!("wav `{}` chunk alignment overflow", path.display()))?;
    }
    let format = format.ok_or_else(|| format!("wav `{}` is missing fmt chunk", path.display()))?;
    let data = data.ok_or_else(|| format!("wav `{}` is missing data chunk", path.display()))?;
    if format.len() < 16 {
        return Err(format!("wav `{}` has an invalid fmt chunk", path.display()));
    }
    let audio_format = read_u16_le(&format[0..2]);
    let channels = i64::from(read_u16_le(&format[2..4]));
    let sample_rate_hz = i64::from(read_u32_le(&format[4..8]));
    let bits_per_sample = read_u16_le(&format[14..16]);
    if channels <= 0 {
        return Err(format!(
            "wav `{}` has an invalid channel count",
            path.display()
        ));
    }
    if sample_rate_hz <= 0 {
        return Err(format!(
            "wav `{}` has an invalid sample rate",
            path.display()
        ));
    }
    let pcm_bytes = decode_wav_pcm16_samples(audio_format, channels, bits_per_sample, &data)?;
    let bytes_per_frame = usize::try_from(channels)
        .ok()
        .and_then(|channels| channels.checked_mul(2))
        .ok_or_else(|| "wav frame size overflow".to_string())?;
    let frames = i64::try_from(pcm_bytes.len() / bytes_per_frame)
        .map_err(|_| "wav frame count overflow".to_string())?;
    Ok(NativeAudioBuffer {
        frames,
        channels,
        sample_rate_hz,
        pcm_bytes,
    })
}

fn decode_wav_pcm16_samples(
    audio_format: u16,
    channels: i64,
    bits_per_sample: u16,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let channels =
        usize::try_from(channels).map_err(|_| "wav channel count overflow".to_string())?;
    if channels == 0 {
        return Err("wav channel count must be greater than zero".to_string());
    }
    let sample_bytes = usize::from(bits_per_sample / 8);
    if sample_bytes == 0 {
        return Err("wav bits-per-sample must be non-zero".to_string());
    }
    if !data.len().is_multiple_of(sample_bytes) {
        return Err("wav sample data is not aligned to the declared sample width".to_string());
    }
    let sample_count = data.len() / sample_bytes;
    let capacity = sample_count
        .checked_mul(2)
        .ok_or_else(|| "wav PCM output size overflow".to_string())?;
    let mut out = Vec::with_capacity(capacity);
    match (audio_format, bits_per_sample) {
        (1, 8) => {
            for &sample in data {
                let sample = ((i16::from(sample) - 128) << 8).to_le_bytes();
                out.extend_from_slice(&sample);
            }
        }
        (1, 16) => out.extend_from_slice(data),
        (1, 24) => {
            for chunk in data.chunks_exact(3) {
                let sample = (((chunk[2] as i32) << 24)
                    | ((chunk[1] as i32) << 16)
                    | ((chunk[0] as i32) << 8))
                    >> 16;
                out.extend_from_slice(&(sample as i16).to_le_bytes());
            }
        }
        (1, 32) => {
            for chunk in data.chunks_exact(4) {
                let sample = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) >> 16;
                out.extend_from_slice(&(sample as i16).to_le_bytes());
            }
        }
        (3, 32) => {
            for chunk in data.chunks_exact(4) {
                let sample =
                    f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).clamp(-1.0, 1.0);
                let sample = (sample * i16::MAX as f32) as i16;
                out.extend_from_slice(&sample.to_le_bytes());
            }
        }
        _ => {
            return Err(format!(
                "unsupported wav format: audio_format={audio_format}, bits_per_sample={bits_per_sample}"
            ));
        }
    }
    if out.len() % (channels * 2) != 0 {
        return Err("wav PCM data does not align to channel width".to_string());
    }
    Ok(out)
}

fn decode_bmp_image(path: &Path) -> Result<NativeImage, String> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("failed to read image `{}`: {err}", path.display()))?;
    decode_bmp_bytes(&bytes, path)
}

fn decode_bmp_bytes(bytes: &[u8], path: &Path) -> Result<NativeImage, String> {
    if bytes.len() < 54 || &bytes[0..2] != b"BM" {
        return Err(format!("`{}` is not a supported BMP image", path.display()));
    }
    let pixel_offset = read_u32_le(&bytes[10..14]) as usize;
    let dib_size = read_u32_le(&bytes[14..18]) as usize;
    if dib_size < 40 {
        return Err(format!(
            "BMP `{}` uses an unsupported DIB header",
            path.display()
        ));
    }
    let width = read_i32_le(&bytes[18..22]);
    let height = read_i32_le(&bytes[22..26]);
    let planes = read_u16_le(&bytes[26..28]);
    let bits_per_pixel = read_u16_le(&bytes[28..30]);
    let compression = read_u32_le(&bytes[30..34]);
    if planes != 1 || compression != 0 {
        return Err(format!(
            "BMP `{}` uses unsupported compression or plane count",
            path.display()
        ));
    }
    if !(bits_per_pixel == 24 || bits_per_pixel == 32) {
        return Err(format!("BMP `{}` must be 24-bit or 32-bit", path.display()));
    }
    if width == 0 || height == 0 {
        return Err(format!(
            "BMP `{}` must have non-zero dimensions",
            path.display()
        ));
    }
    let width_abs = width.unsigned_abs() as usize;
    let height_abs = height.unsigned_abs() as usize;
    let bytes_per_pixel = usize::from(bits_per_pixel / 8);
    let pixel_count = width_abs
        .checked_mul(height_abs)
        .ok_or_else(|| format!("BMP `{}` pixel count overflow", path.display()))?;
    let row_stride = width_abs
        .checked_mul(bytes_per_pixel)
        .and_then(|row_bytes| row_bytes.checked_add(3))
        .map(|row_bytes| row_bytes & !3)
        .ok_or_else(|| format!("BMP `{}` row size overflow", path.display()))?;
    let pixel_end = pixel_offset
        .checked_add(
            row_stride
                .checked_mul(height_abs)
                .ok_or_else(|| format!("BMP `{}` pixel data size overflow", path.display()))?,
        )
        .ok_or_else(|| format!("BMP `{}` pixel offset overflow", path.display()))?;
    if pixel_end > bytes.len() {
        return Err(format!("BMP `{}` is truncated", path.display()));
    }
    let mut pixels = vec![0; pixel_count];
    let top_down = height < 0;
    for row in 0..height_abs {
        let source_row = if top_down { row } else { height_abs - 1 - row };
        let row_start = pixel_offset + source_row * row_stride;
        for col in 0..width_abs {
            let pixel_start = row_start + col * bytes_per_pixel;
            let b = bytes[pixel_start];
            let g = bytes[pixel_start + 1];
            let r = bytes[pixel_start + 2];
            pixels[row * width_abs + col] = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
        }
    }
    Ok(NativeImage {
        width: width_abs as i64,
        height: height_abs as i64,
        pixels,
    })
}

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn bytes_clipboard_format() -> Result<u32, String> {
    REGISTERED_BYTES_CLIPBOARD_FORMAT
        .get_or_init(|| {
            let name = wide_null(ARCANA_BYTES_CLIPBOARD_FORMAT_NAME);
            let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
            if format == 0 {
                Err("failed to register Arcana bytes clipboard format".to_string())
            } else {
                Ok(format)
            }
        })
        .clone()
}

fn clipboard_payload_from_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8 + bytes.len());
    payload.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    payload.extend_from_slice(bytes);
    payload
}

fn decode_clipboard_bytes_payload(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 8 {
        return Err("Windows clipboard bytes payload is truncated".to_string());
    }
    let declared_len = u64::from_le_bytes(bytes[..8].try_into().expect("prefix should fit"));
    let declared_len = usize::try_from(declared_len)
        .map_err(|_| "Windows clipboard bytes payload length overflow".to_string())?;
    if bytes.len() < 8 + declared_len {
        return Err("Windows clipboard bytes payload is truncated".to_string());
    }
    Ok(bytes[8..8 + declared_len].to_vec())
}

fn clipboard_write_block(format: u32, bytes: &[u8]) -> Result<(), String> {
    let _guard = ClipboardGuard::open()?;
    if unsafe { EmptyClipboard() } == 0 {
        return Err("failed to clear Windows clipboard".to_string());
    }
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
    if handle.is_null() {
        return Err("failed to allocate Windows clipboard block".to_string());
    }
    let locked = unsafe { GlobalLock(handle) } as *mut u8;
    if locked.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err("failed to lock Windows clipboard block".to_string());
    }
    if !bytes.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), locked, bytes.len());
        }
    }
    unsafe {
        GlobalUnlock(handle);
    }
    let stored = unsafe { SetClipboardData(format, handle) };
    if stored.is_null() {
        unsafe {
            GlobalFree(handle);
        }
        return Err("failed to publish Windows clipboard data".to_string());
    }
    Ok(())
}

fn clipboard_read_block(format: u32) -> Result<Vec<u8>, String> {
    let _guard = ClipboardGuard::open()?;
    if unsafe { IsClipboardFormatAvailable(format) } == 0 {
        return Err("requested Windows clipboard format is not available".to_string());
    }
    let handle = unsafe { GetClipboardData(format) };
    if handle.is_null() {
        return Err("failed to access Windows clipboard data".to_string());
    }
    let locked = unsafe { GlobalLock(handle) } as *const u8;
    if locked.is_null() {
        return Err("failed to lock Windows clipboard data".to_string());
    }
    let size = unsafe { GlobalSize(handle) };
    let bytes = unsafe { std::slice::from_raw_parts(locked, size) }.to_vec();
    unsafe {
        GlobalUnlock(handle);
    }
    Ok(bytes)
}

fn clipboard_write_text_impl(text: &str) -> Result<(), String> {
    let utf16 = wide_null(text);
    let bytes = unsafe {
        std::slice::from_raw_parts(utf16.as_ptr() as *const u8, utf16.len() * size_of::<u16>())
    };
    clipboard_write_block(CF_UNICODETEXT.into(), bytes)
}

fn clipboard_read_text_impl() -> Result<String, String> {
    let bytes = clipboard_read_block(CF_UNICODETEXT.into())?;
    if bytes.len() % size_of::<u16>() != 0 {
        return Err("Windows clipboard text payload is not valid UTF-16".to_string());
    }
    let units = unsafe {
        std::slice::from_raw_parts(bytes.as_ptr() as *const u16, bytes.len() / size_of::<u16>())
    };
    let end = units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len());
    String::from_utf16(&units[..end])
        .map_err(|_| "Windows clipboard text is not valid UTF-16".to_string())
}

fn clipboard_write_bytes_impl(bytes: &[u8]) -> Result<(), String> {
    let format = bytes_clipboard_format()?;
    let payload = clipboard_payload_from_bytes(bytes);
    clipboard_write_block(format, &payload)
}

fn clipboard_read_bytes_impl() -> Result<Vec<u8>, String> {
    let format = bytes_clipboard_format()?;
    let payload = clipboard_read_block(format)?;
    decode_clipboard_bytes_payload(&payload)
}

fn current_module_handle() -> Result<usize, String> {
    let mut module: HINSTANCE = null_mut();
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            current_module_handle as *const () as *const u16,
            &mut module,
        )
    };
    if ok == 0 || module.is_null() {
        return Err("failed to resolve current Arcana runtime module handle".to_string());
    }
    Ok(module as usize)
}

fn native_window_class_name(module_handle: usize) -> Vec<u16> {
    wide_null(&native_window_class_name_text(module_handle))
}

fn native_window_class_name_text(module_handle: usize) -> String {
    format!("{WINDOW_CLASS_NAME}_{module_handle:x}")
}

fn pack_color(color: i64) -> u32 {
    (color.clamp(0, 0xFFFFFF)) as u32
}

fn sanitize_dimension(value: i64) -> i64 {
    value.max(1)
}

fn mouse_button_from_message(message: u32, wparam: u32) -> i64 {
    match message {
        WM_LBUTTONDOWN | WM_LBUTTONUP => 1,
        WM_RBUTTONDOWN | WM_RBUTTONUP => 2,
        WM_MBUTTONDOWN | WM_MBUTTONUP => 3,
        WM_XBUTTONDOWN | WM_XBUTTONUP => match hiword(wparam) as u32 {
            1 => 4,
            2 => 5,
            _ => -1,
        },
        _ => -1,
    }
}

fn loword(value: u32) -> u16 {
    (value & 0xFFFF) as u16
}

fn hiword(value: u32) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

fn signed_loword(value: u32) -> i16 {
    loword(value) as i16
}

fn signed_hiword(value: u32) -> i16 {
    hiword(value) as i16
}

fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_i32_le(bytes: &[u8]) -> i32 {
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn mmresult_text(code: u32) -> String {
    let mut buffer = [0u16; 256];
    let status = unsafe { waveOutGetErrorTextW(code, buffer.as_mut_ptr(), buffer.len() as u32) };
    if status == MMSYSERR_NOERROR {
        let len = buffer
            .iter()
            .position(|&ch| ch == 0)
            .unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..len])
    } else {
        format!("MMRESULT({code})")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::ptr::null_mut;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{
        DEVICE_EVENTS_ALWAYS, DEVICE_EVENTS_NEVER, DEVICE_EVENTS_WHEN_FOCUSED, EVENT_ABOUT_TO_WAIT,
        EVENT_APP_RESUMED, EVENT_APP_SUSPENDED, EVENT_RAW_MOUSE_MOTION,
        EVENT_TEXT_COMPOSITION_CANCELLED, EVENT_TEXT_COMPOSITION_COMMITTED,
        EVENT_TEXT_COMPOSITION_STARTED, EVENT_TEXT_INPUT, EVENT_WINDOW_CLOSE_REQUESTED,
        EVENT_WINDOW_FOCUSED, EVENT_WINDOW_MOVED, EVENT_WINDOW_REDRAW_REQUESTED,
        EVENT_WINDOW_RESIZED, NativeAudioPlayback, PendingRawInputEvent, RIDEV_INPUTSINK,
        RuntimeAudioDeviceHandle, RuntimeAudioPlaybackHandle, RuntimeHost, attention_flash_flags,
        clipboard_payload_from_bytes, decode_bmp_bytes, decode_clipboard_bytes_payload,
        decode_wav_bytes, native_cursor_handle, native_window_class_name_text,
        pending_raw_input_events, push_pending_raw_input_event, release_playback_resources,
    };
    use crate::{BufferedEvent, NativeProcessHost};
    use windows_sys::Win32::Media::Audio::{HWAVEOUT, WHDR_DONE};
    use windows_sys::Win32::UI::Input::Ime::{
        CPS_COMPLETE, GCS_COMPSTR, GCS_RESULTSTR, HIMC, ImmAssociateContext, ImmCreateContext,
        ImmDestroyContext, ImmGetContext, ImmNotifyIME, ImmReleaseContext,
        ImmSetCompositionStringW, NI_COMPOSITIONSTR, SCS_SETSTR,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FLASHW_CAPTION, FLASHW_STOP, FLASHW_TIMERNOFG, FLASHW_TRAY, GetCursor,
        GetWindowTextLengthW, GetWindowTextW, PostMessageW, SendMessageW, WM_CHAR, WM_CLOSE,
        WM_IME_COMPOSITION, WM_IME_ENDCOMPOSITION, WM_IME_STARTCOMPOSITION, WM_NULL,
    };

    fn read_window_title(hwnd: super::HWND) -> String {
        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; usize::try_from(len).unwrap_or(0) + 1];
        let read = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), len + 1) };
        if read <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..usize::try_from(read).unwrap_or(0)])
    }

    #[test]
    fn decode_wav_bytes_preserves_pcm16_frames() {
        let pcm = vec![1, 0, 2, 0, 0xFF, 0xFF, 0xFE, 0xFF];
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36u32 + pcm.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&48_000u32.to_le_bytes());
        wav.extend_from_slice(&192_000u32.to_le_bytes());
        wav.extend_from_slice(&4u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
        wav.extend_from_slice(&pcm);

        let decoded =
            decode_wav_bytes(&wav, Path::new("fixture.wav")).expect("wav fixture should decode");
        assert_eq!(decoded.frames, 2);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.sample_rate_hz, 48_000);
        assert_eq!(decoded.pcm_bytes, pcm);
    }

    #[test]
    fn decode_wav_bytes_normalizes_pcm8_to_pcm16() {
        let samples = [0u8, 128u8, 255u8];
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36u32 + samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&22_050u32.to_le_bytes());
        wav.extend_from_slice(&22_050u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&8u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(samples.len() as u32).to_le_bytes());
        wav.extend_from_slice(&samples);

        let decoded =
            decode_wav_bytes(&wav, Path::new("fixture.wav")).expect("wav fixture should decode");
        assert_eq!(decoded.pcm_bytes, vec![0x00, 0x80, 0x00, 0x00, 0x00, 0x7F]);
    }

    #[test]
    fn decode_bmp_bytes_reads_bottom_up_rgb24() {
        let mut bmp = Vec::new();
        bmp.extend_from_slice(b"BM");
        bmp.extend_from_slice(&70u32.to_le_bytes());
        bmp.extend_from_slice(&0u16.to_le_bytes());
        bmp.extend_from_slice(&0u16.to_le_bytes());
        bmp.extend_from_slice(&54u32.to_le_bytes());
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&2i32.to_le_bytes());
        bmp.extend_from_slice(&2i32.to_le_bytes());
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&16u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&[0xFF, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00]);
        bmp.extend_from_slice(&[0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x00, 0x00]);

        let decoded =
            decode_bmp_bytes(&bmp, Path::new("fixture.bmp")).expect("bmp fixture should decode");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.pixels, vec![0xFF0000, 0x00FF00, 0x0000FF, 0xFFFFFF]);
    }

    #[test]
    fn decode_bmp_bytes_reads_top_down_rgba32() {
        let mut bmp = Vec::new();
        bmp.extend_from_slice(b"BM");
        bmp.extend_from_slice(&62u32.to_le_bytes());
        bmp.extend_from_slice(&0u16.to_le_bytes());
        bmp.extend_from_slice(&0u16.to_le_bytes());
        bmp.extend_from_slice(&54u32.to_le_bytes());
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&1i32.to_le_bytes());
        bmp.extend_from_slice(&(-2i32).to_le_bytes());
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&32u16.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&8u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&[0x00, 0x00, 0xFF, 0x80]);
        bmp.extend_from_slice(&[0xFF, 0x00, 0x00, 0x40]);

        let decoded =
            decode_bmp_bytes(&bmp, Path::new("fixture.bmp")).expect("bmp fixture should decode");
        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 2);
        assert_eq!(decoded.pixels, vec![0xFF0000, 0x0000FF]);
    }

    #[test]
    fn native_window_class_name_is_module_specific() {
        assert_ne!(
            native_window_class_name_text(0x1000usize),
            native_window_class_name_text(0x2000usize)
        );
    }

    #[test]
    fn clipboard_bytes_payload_roundtrips() {
        let payload = clipboard_payload_from_bytes(&[7, 8, 9]);
        let decoded =
            decode_clipboard_bytes_payload(&payload).expect("clipboard payload should decode");
        assert_eq!(decoded, vec![7, 8, 9]);
    }

    #[test]
    fn clipboard_bytes_payload_rejects_truncated_data() {
        let err = decode_clipboard_bytes_payload(&[1, 2, 3, 4])
            .expect_err("truncated clipboard payload should fail");
        assert!(err.contains("truncated"));
    }

    #[test]
    fn release_playback_resources_drops_cached_pcm_bytes() {
        let mut playback = NativeAudioPlayback::new(
            RuntimeAudioDeviceHandle(3),
            null_mut::<core::ffi::c_void>() as HWAVEOUT,
            8,
            vec![1, 2, 3, 4],
        );
        playback.paused = true;
        playback.header.dwFlags = WHDR_DONE;

        release_playback_resources(&mut playback).expect("null playback cleanup should succeed");

        let header_data = unsafe { std::ptr::addr_of!(playback.header.lpData).read_unaligned() };
        let header_len =
            unsafe { std::ptr::addr_of!(playback.header.dwBufferLength).read_unaligned() };
        assert!(playback.wave_out.is_null());
        assert!(playback.pcm_bytes.is_empty());
        assert_eq!(header_data, null_mut());
        assert_eq!(header_len, 0);
        assert!(playback.finished);
        assert!(!playback.paused);
    }

    #[test]
    fn stop_and_output_close_consume_native_audio_handles() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let device = host.insert_audio_device(48_000, 2);
        let playback_handle = RuntimeAudioPlaybackHandle(7);
        host.audio_playbacks.insert(
            playback_handle,
            NativeAudioPlayback::new(
                device,
                null_mut::<core::ffi::c_void>() as HWAVEOUT,
                4,
                vec![0, 1, 2, 3],
            ),
        );

        RuntimeHost::audio_playback_stop(&mut host, playback_handle)
            .expect("playback stop should succeed");
        assert!(host.audio_playback_ref(playback_handle).is_err());

        let second_playback = RuntimeAudioPlaybackHandle(8);
        host.audio_playbacks.insert(
            second_playback,
            NativeAudioPlayback::new(
                device,
                null_mut::<core::ffi::c_void>() as HWAVEOUT,
                4,
                vec![4, 5, 6, 7],
            ),
        );

        RuntimeHost::audio_output_close(&mut host, device).expect("device close should succeed");
        assert!(host.audio_device_ref(device).is_err());
        assert!(host.audio_playback_ref(second_playback).is_err());
    }

    #[test]
    fn native_audio_playback_rejects_buffer_format_mismatch() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let device = host.insert_audio_device(48_000, 2);
        let clip = host.insert_audio_buffer(super::NativeAudioBuffer {
            frames: 32,
            channels: 1,
            sample_rate_hz: 44_100,
            pcm_bytes: vec![0; 64],
        });

        let err = RuntimeHost::audio_play_buffer(&mut host, device, clip)
            .expect_err("mismatched clip should be rejected");
        assert!(err.contains("does not match AudioDevice format"));
    }

    #[test]
    fn native_close_request_waits_for_explicit_window_close() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        let hwnd = host
            .window_ref(window)
            .expect("window state should exist")
            .hwnd;
        unsafe {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
        }

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }

        assert_eq!(kinds.first().copied(), Some(EVENT_APP_RESUMED));
        assert_eq!(kinds.last().copied(), Some(EVENT_ABOUT_TO_WAIT));
        assert!(kinds.contains(&EVENT_WINDOW_CLOSE_REQUESTED));
        let close_index = kinds
            .iter()
            .position(|kind| *kind == EVENT_WINDOW_CLOSE_REQUESTED)
            .expect("close request should be present");
        for later_kind in [
            EVENT_WINDOW_MOVED,
            EVENT_WINDOW_RESIZED,
            EVENT_WINDOW_FOCUSED,
            EVENT_WINDOW_REDRAW_REQUESTED,
        ] {
            if let Some(index) = kinds.iter().position(|kind| *kind == later_kind) {
                assert!(
                    close_index < index,
                    "close request should outrank window event kind {later_kind}, got {kinds:?}"
                );
            }
        }
        assert!(!kinds.contains(&EVENT_APP_SUSPENDED));
        assert!(RuntimeHost::window_alive(&mut host, window).expect("window should remain alive"));

        RuntimeHost::window_close(&mut host, window).expect("explicit close should succeed");
        assert!(host.window_ref(window).is_err());
        assert!(
            !RuntimeHost::window_alive(&mut host, window)
                .expect("closed window should report not alive")
        );
        assert!(
            host.session_ref(session)
                .expect("session should still exist")
                .windows
                .is_empty()
        );

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }

        assert_eq!(kinds, vec![EVENT_APP_SUSPENDED, EVENT_ABOUT_TO_WAIT]);
    }

    #[test]
    fn native_close_request_backlog_is_dropped_after_explicit_close_before_pump() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        let hwnd = host
            .window_ref(window)
            .expect("window state should exist")
            .hwnd;
        unsafe {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
        }

        RuntimeHost::window_close(&mut host, window).expect("explicit close should succeed");

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }

        assert_eq!(kinds, vec![EVENT_ABOUT_TO_WAIT]);
        assert!(
            host.session_ref(session)
                .expect("session should still exist")
                .windows
                .is_empty()
        );
        assert!(host.window_ref(window).is_err());
    }

    #[test]
    fn native_session_close_after_close_request_destroys_window_cleanly() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        let hwnd = host
            .window_ref(window)
            .expect("window state should exist")
            .hwnd;
        unsafe {
            SendMessageW(hwnd, WM_CLOSE, 0, 0);
        }

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll")
            .is_some()
        {}

        RuntimeHost::events_session_close(&mut host, session)
            .expect("session close after close request should succeed");
        assert!(host.window_ref(window).is_err());
        assert!(host.session_ref(session).is_err());
    }

    #[test]
    fn native_session_window_lookup_finds_attached_window_by_id() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        let window_id = RuntimeHost::window_id(&mut host, window).expect("window id");
        assert_eq!(
            RuntimeHost::events_session_window_by_id(&mut host, session, window_id)
                .expect("lookup should succeed"),
            Some(window)
        );
        assert_eq!(
            RuntimeHost::events_session_window_by_id(&mut host, session, 999_999)
                .expect("missing lookup should succeed"),
            None
        );

        RuntimeHost::window_close(&mut host, window).expect("window close should succeed");
        assert_eq!(
            RuntimeHost::events_session_window_by_id(&mut host, session, window_id)
                .expect("closed lookup should succeed"),
            None
        );
    }

    #[test]
    fn native_session_window_lookup_ignores_closed_windows_with_pending_events() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        RuntimeHost::window_request_redraw(&mut host, window)
            .expect("redraw should queue before close");
        let window_id = RuntimeHost::window_id(&mut host, window).expect("window id");

        RuntimeHost::window_close(&mut host, window).expect("window close should succeed");
        assert_eq!(
            RuntimeHost::events_session_window_by_id(&mut host, session, window_id)
                .expect("closed lookup should succeed"),
            None
        );
        assert!(
            RuntimeHost::events_session_window_ids(&mut host, session)
                .expect("window ids should succeed")
                .is_empty(),
            "closed windows with queued events must not stay externally visible"
        );

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }
        assert_eq!(kinds, vec![EVENT_ABOUT_TO_WAIT]);
    }

    #[test]
    fn native_session_ready_events_report_suspend_after_close() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");
        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}

        RuntimeHost::window_request_redraw(&mut host, window)
            .expect("redraw should queue before close");
        RuntimeHost::window_close(&mut host, window).expect("window close should succeed");

        assert!(
            host.session_has_ready_events(session)
                .expect("ready-event probe should succeed"),
            "closing the final live window must still make the session ready for suspension"
        );
    }

    #[test]
    fn native_session_notifications_skip_closed_attached_windows() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let first = RuntimeHost::window_open(&mut host, "First", 320, 200).expect("first window");
        let second =
            RuntimeHost::window_open(&mut host, "Second", 320, 200).expect("second window");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, first)
            .expect("first window should attach");
        RuntimeHost::events_session_attach_window(&mut host, session, second)
            .expect("second window should attach");
        host.pump_messages()
            .expect("native message loop should settle initial WM_NULL signals");

        RuntimeHost::window_request_redraw(&mut host, first)
            .expect("first redraw should queue before close");
        RuntimeHost::window_close(&mut host, first).expect("first window should close");

        host.notify_session_queue(session);

        assert!(
            host.window_state_ref(second)
                .expect("second window state should exist")
                .message_loop_signaled,
            "session notifications must wake a live sibling window even if the first attached window is already closed"
        );
    }

    #[test]
    fn native_session_reattach_emits_resumed_again() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let first =
            RuntimeHost::window_open(&mut host, "First", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, first)
            .expect("window should attach");

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }
        assert_eq!(kinds, vec![EVENT_APP_RESUMED, EVENT_ABOUT_TO_WAIT]);

        RuntimeHost::window_close(&mut host, first).expect("first window should close");
        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }
        assert_eq!(kinds, vec![EVENT_APP_SUSPENDED, EVENT_ABOUT_TO_WAIT]);

        let second =
            RuntimeHost::window_open(&mut host, "Second", 320, 200).expect("window should open");
        RuntimeHost::events_session_attach_window(&mut host, session, second)
            .expect("window should attach");
        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }
        assert_eq!(kinds, vec![EVENT_APP_RESUMED, EVENT_ABOUT_TO_WAIT]);

        RuntimeHost::window_close(&mut host, second).expect("second window should close");
    }

    #[test]
    fn native_session_device_events_when_focused_filters_unfocused_raw_input() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        assert_eq!(
            RuntimeHost::events_session_device_events(&mut host, session)
                .expect("default device events policy"),
            DEVICE_EVENTS_WHEN_FOCUSED
        );

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}

        let state = host.window_mut(window).expect("window state should exist");
        state.focused = false;
        state.events.push_back(BufferedEvent {
            kind: EVENT_RAW_MOUSE_MOTION,
            window_id: 7,
            a: 3,
            b: 4,
            ..BufferedEvent::default()
        });

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
        }

        assert_eq!(kinds, vec![EVENT_ABOUT_TO_WAIT]);
    }

    #[test]
    fn native_raw_input_registration_uses_focused_window_for_when_focused_policy() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let handle =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, handle)
            .expect("window should attach");
        host.window_mut(handle).expect("window state").focused = true;

        assert_eq!(host.desired_raw_input_registration(), Some((handle, 0)));

        RuntimeHost::window_close(&mut host, handle).expect("window should close");
    }

    #[test]
    fn native_raw_input_registration_prefers_always_policy_with_input_sink() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let focused_handle =
            RuntimeHost::window_open(&mut host, "Focused", 320, 200).expect("focused window");
        let always_handle =
            RuntimeHost::window_open(&mut host, "Always", 320, 200).expect("always window");
        let focused_session = RuntimeHost::events_session_open(&mut host).expect("session");
        let always_session = RuntimeHost::events_session_open(&mut host).expect("session");
        RuntimeHost::events_session_attach_window(&mut host, focused_session, focused_handle)
            .expect("focused attach");
        RuntimeHost::events_session_attach_window(&mut host, always_session, always_handle)
            .expect("always attach");
        RuntimeHost::events_session_set_device_events(
            &mut host,
            always_session,
            DEVICE_EVENTS_ALWAYS,
        )
        .expect("always policy should update");
        host.window_mut(focused_handle)
            .expect("focused state should exist")
            .focused = true;

        assert_eq!(
            host.desired_raw_input_registration(),
            Some((always_handle, RIDEV_INPUTSINK))
        );

        RuntimeHost::window_close(&mut host, focused_handle).expect("focused close");
        RuntimeHost::window_close(&mut host, always_handle).expect("always close");
    }

    #[test]
    fn native_raw_input_registration_returns_none_when_device_events_are_disabled() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let handle =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, handle)
            .expect("window should attach");
        RuntimeHost::events_session_set_device_events(&mut host, session, DEVICE_EVENTS_NEVER)
            .expect("device events policy should update");
        host.window_mut(handle).expect("window state").focused = true;

        assert_eq!(host.desired_raw_input_registration(), None);

        RuntimeHost::window_close(&mut host, handle).expect("window should close");
    }

    #[test]
    fn native_session_device_events_always_delivers_unfocused_raw_input() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        RuntimeHost::events_session_set_device_events(&mut host, session, DEVICE_EVENTS_ALWAYS)
            .expect("device events policy should update");

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}

        let state = host.window_mut(window).expect("window state should exist");
        state.focused = false;
        state.events.push_back(BufferedEvent {
            kind: EVENT_RAW_MOUSE_MOTION,
            window_id: 9,
            a: 5,
            b: 6,
            ..BufferedEvent::default()
        });

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut seen = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            seen.push((event.kind, event.window_id, event.a, event.b));
        }

        assert_eq!(
            seen,
            vec![
                (EVENT_RAW_MOUSE_MOTION, 9, 5, 6),
                (EVENT_ABOUT_TO_WAIT, 0, 0, 0),
            ]
        );
    }

    #[test]
    fn native_pending_raw_input_events_fan_out_per_session_policy() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let focused_window =
            RuntimeHost::window_open(&mut host, "Focused", 320, 200).expect("focused window");
        let always_window =
            RuntimeHost::window_open(&mut host, "Always", 320, 200).expect("always window");
        let focused_session = RuntimeHost::events_session_open(&mut host).expect("session");
        let always_session = RuntimeHost::events_session_open(&mut host).expect("session");
        RuntimeHost::events_session_attach_window(&mut host, focused_session, focused_window)
            .expect("focused attach");
        RuntimeHost::events_session_attach_window(&mut host, always_session, always_window)
            .expect("always attach");
        RuntimeHost::events_session_set_device_events(
            &mut host,
            always_session,
            DEVICE_EVENTS_ALWAYS,
        )
        .expect("always policy should update");

        let frame = RuntimeHost::events_session_pump(&mut host, focused_session).expect("pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}
        let frame = RuntimeHost::events_session_pump(&mut host, always_session).expect("pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}

        host.window_mut(focused_window)
            .expect("focused state should exist")
            .focused = true;
        host.window_mut(always_window)
            .expect("always state should exist")
            .focused = false;

        let mut pending = pending_raw_input_events()
            .lock()
            .expect("pending raw input queue should lock");
        pending.clear();
        pending.push(PendingRawInputEvent::MouseMotion {
            device_id: 41,
            dx: 7,
            dy: 8,
        });
        drop(pending);

        host.dispatch_pending_raw_input_events()
            .expect("raw input dispatch should succeed");

        let frame = RuntimeHost::events_session_pump(&mut host, focused_session).expect("pump");
        let mut focused_seen = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            focused_seen.push((event.kind, event.window_id, event.a, event.b));
        }
        assert_eq!(
            focused_seen,
            vec![
                (EVENT_RAW_MOUSE_MOTION, 41, 7, 8),
                (EVENT_ABOUT_TO_WAIT, 0, 0, 0),
            ]
        );

        let frame = RuntimeHost::events_session_pump(&mut host, always_session).expect("pump");
        let mut always_seen = Vec::new();
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            always_seen.push((event.kind, event.window_id, event.a, event.b));
        }
        assert_eq!(
            always_seen,
            vec![
                (EVENT_RAW_MOUSE_MOTION, 41, 7, 8),
                (EVENT_ABOUT_TO_WAIT, 0, 0, 0),
            ]
        );

        pending_raw_input_events()
            .lock()
            .expect("pending raw input queue should lock")
            .clear();
        RuntimeHost::window_close(&mut host, focused_window).expect("focused close");
        RuntimeHost::window_close(&mut host, always_window).expect("always close");
    }

    #[test]
    fn native_pending_raw_input_events_coalesce_consecutive_motion() {
        let mut pending = Vec::new();
        push_pending_raw_input_event(
            &mut pending,
            PendingRawInputEvent::MouseMotion {
                device_id: 7,
                dx: 5,
                dy: -3,
            },
        );
        push_pending_raw_input_event(
            &mut pending,
            PendingRawInputEvent::MouseMotion {
                device_id: 7,
                dx: 4,
                dy: 6,
            },
        );
        push_pending_raw_input_event(
            &mut pending,
            PendingRawInputEvent::MouseButton {
                device_id: 7,
                button: 1,
                pressed: true,
            },
        );
        push_pending_raw_input_event(
            &mut pending,
            PendingRawInputEvent::MouseMotion {
                device_id: 7,
                dx: -2,
                dy: 1,
            },
        );

        assert_eq!(
            pending,
            vec![
                PendingRawInputEvent::MouseMotion {
                    device_id: 7,
                    dx: 9,
                    dy: 3,
                },
                PendingRawInputEvent::MouseButton {
                    device_id: 7,
                    button: 1,
                    pressed: true,
                },
                PendingRawInputEvent::MouseMotion {
                    device_id: 7,
                    dx: -2,
                    dy: 1,
                },
            ]
        );
    }

    #[test]
    fn native_attention_flash_flags_match_header_and_taskbar_until_focus() {
        assert_eq!(
            attention_flash_flags(true),
            FLASHW_CAPTION | FLASHW_TRAY | FLASHW_TIMERNOFG
        );
        assert_eq!(attention_flash_flags(false), FLASHW_STOP);
    }

    #[test]
    fn native_repeated_attention_requests_do_not_fail() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Attention", 320, 200).expect("window should open");

        RuntimeHost::window_request_attention(&mut host, window, true)
            .expect("first attention request should succeed");
        RuntimeHost::window_request_attention(&mut host, window, true)
            .expect("repeated attention request should succeed");
        RuntimeHost::window_request_attention(&mut host, window, false)
            .expect("attention reset should succeed");
        RuntimeHost::window_request_attention(&mut host, window, false)
            .expect("repeated attention reset should succeed");

        RuntimeHost::window_close(&mut host, window).expect("window should close");
    }

    #[test]
    fn native_wait_for_session_activity_ignores_other_session_messages() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let first = RuntimeHost::window_open(&mut host, "First", 320, 200).expect("first window");
        let second =
            RuntimeHost::window_open(&mut host, "Second", 320, 200).expect("second window");
        let first_session = RuntimeHost::events_session_open(&mut host).expect("session");
        let second_session = RuntimeHost::events_session_open(&mut host).expect("session");
        RuntimeHost::events_session_attach_window(&mut host, first_session, first)
            .expect("first attach");
        RuntimeHost::events_session_attach_window(&mut host, second_session, second)
            .expect("second attach");

        let frame = RuntimeHost::events_session_pump(&mut host, first_session).expect("pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}
        let frame = RuntimeHost::events_session_pump(&mut host, second_session).expect("pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}

        let first_hwnd = host
            .window_ref(first)
            .expect("first state should exist")
            .hwnd as usize;
        let second_hwnd = host
            .window_ref(second)
            .expect("second state should exist")
            .hwnd;
        let closer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            unsafe {
                PostMessageW(first_hwnd as super::HWND, WM_CLOSE, 0, 0);
            }
        });

        unsafe {
            PostMessageW(second_hwnd, WM_NULL, 0, 0);
        }
        let start = Instant::now();
        host.wait_for_session_activity(first_session, 200)
            .expect("session wait should succeed");
        closer.join().expect("closer thread should finish");

        assert!(
            start.elapsed() >= Duration::from_millis(25),
            "waiting session should ignore unrelated window messages"
        );
        assert!(
            host.session_has_ready_events(first_session)
                .expect("ready probe should succeed"),
            "target session should become ready only after its own close request arrives"
        );

        RuntimeHost::window_close(&mut host, first).expect("first window should close");
        RuntimeHost::window_close(&mut host, second).expect("second window should close");
    }

    #[test]
    fn native_cursor_setters_apply_immediately_while_pointer_is_inside() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        host.window_mut(window)
            .expect("window state should exist")
            .mouse_in_window = true;

        RuntimeHost::window_set_cursor_icon_code(&mut host, window, 3)
            .expect("cursor icon should update");
        assert_eq!(unsafe { GetCursor() }, native_cursor_handle(3));

        RuntimeHost::window_set_cursor_visible(&mut host, window, false)
            .expect("cursor visibility should update");
        assert!(unsafe { GetCursor() }.is_null());

        RuntimeHost::window_set_cursor_visible(&mut host, window, true)
            .expect("cursor visibility should restore");
        assert_eq!(unsafe { GetCursor() }, native_cursor_handle(3));

        RuntimeHost::window_close(&mut host, window).expect("window should close");
    }

    #[test]
    fn native_window_set_size_emits_single_resize_and_redraw() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        while RuntimeHost::events_poll(&mut host, frame)
            .expect("event poll should succeed")
            .is_some()
        {}

        RuntimeHost::window_set_size(&mut host, window, 400, 250)
            .expect("window size should update");

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut resize_events = Vec::new();
        let mut redraw_count = 0usize;
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            if event.kind == EVENT_WINDOW_RESIZED {
                resize_events.push((event.a, event.b));
            }
            if event.kind == EVENT_WINDOW_REDRAW_REQUESTED {
                redraw_count += 1;
            }
        }

        assert_eq!(resize_events, vec![(400, 250)]);
        assert_eq!(redraw_count, 1);

        RuntimeHost::window_close(&mut host, window).expect("window should close");
    }

    #[test]
    fn native_window_open_publishes_os_title() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana Desktop Proof :: Overview", 320, 200)
                .expect("window should open");
        let hwnd = host
            .window_ref(window)
            .expect("window state should exist")
            .hwnd;
        assert_eq!(read_window_title(hwnd), "Arcana Desktop Proof :: Overview");
        RuntimeHost::window_close(&mut host, window).expect("window should close");
    }

    #[test]
    fn native_window_text_input_is_disabled_by_default() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana Desktop Proof :: Overview", 320, 200)
                .expect("window should open");
        assert!(
            !RuntimeHost::window_text_input_enabled(&mut host, window)
                .expect("text input state should be readable")
        );
        RuntimeHost::window_close(&mut host, window).expect("window should close");
    }

    #[test]
    fn native_session_close_removes_windows_and_wakes() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let first = RuntimeHost::window_open(&mut host, "First", 320, 200)
            .expect("first window should open");
        let second = RuntimeHost::window_open(&mut host, "Second", 320, 200)
            .expect("second window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, first)
            .expect("first window should attach");
        RuntimeHost::events_session_attach_window(&mut host, session, second)
            .expect("second window should attach");
        let wake = RuntimeHost::events_session_create_wake(&mut host, session)
            .expect("wake handle should create");
        RuntimeHost::events_wake_signal(&mut host, wake).expect("wake should signal");

        RuntimeHost::events_session_close(&mut host, session)
            .expect("session close should succeed");

        assert!(host.session_ref(session).is_err());
        assert!(host.window_ref(first).is_err());
        assert!(host.window_ref(second).is_err());
        assert!(host.wake_ref(wake).is_err());
    }

    #[test]
    fn native_window_settings_and_text_input_events_roundtrip() {
        let mut host = NativeProcessHost::current().expect("native host should construct");
        let window =
            RuntimeHost::window_open(&mut host, "Arcana", 320, 200).expect("window should open");
        let session = RuntimeHost::events_session_open(&mut host).expect("session should open");
        RuntimeHost::events_session_attach_window(&mut host, session, window)
            .expect("window should attach");

        RuntimeHost::window_set_min_size(&mut host, window, 111, 112).expect("min size should set");
        RuntimeHost::window_set_max_size(&mut host, window, 333, 334).expect("max size should set");
        RuntimeHost::window_set_transparent(&mut host, window, true)
            .expect("transparent should set");
        RuntimeHost::window_set_theme_override_code(&mut host, window, 2)
            .expect("theme override should set");
        RuntimeHost::window_set_cursor_icon_code(&mut host, window, 3)
            .expect("cursor icon should set");
        RuntimeHost::window_set_text_input_enabled(&mut host, window, true)
            .expect("text input should set");
        RuntimeHost::text_input_set_composition_area(&mut host, window, 9, 10, 20, 21)
            .expect("composition area should set");

        assert_eq!(
            RuntimeHost::window_min_size(&mut host, window).expect("min size"),
            (111, 112)
        );
        assert_eq!(
            RuntimeHost::window_max_size(&mut host, window).expect("max size"),
            (333, 334)
        );
        assert!(RuntimeHost::window_transparent(&mut host, window).expect("transparent state"));
        assert_eq!(
            RuntimeHost::window_theme_override_code(&mut host, window).expect("theme override"),
            2
        );
        assert_eq!(
            RuntimeHost::window_cursor_icon_code(&mut host, window).expect("cursor icon"),
            3
        );
        assert!(
            RuntimeHost::window_text_input_enabled(&mut host, window).expect("text input enabled")
        );
        assert!(
            RuntimeHost::text_input_composition_area_active(&mut host, window)
                .expect("composition area active")
        );
        assert_eq!(
            RuntimeHost::text_input_composition_area_position(&mut host, window)
                .expect("composition area position"),
            (9, 10)
        );
        assert_eq!(
            RuntimeHost::text_input_composition_area_size(&mut host, window)
                .expect("composition area size"),
            (20, 21)
        );

        let hwnd = host
            .window_ref(window)
            .expect("window state should exist")
            .hwnd;
        unsafe {
            SendMessageW(hwnd, WM_CHAR, 'x' as usize, 0);
            let mut himc = ImmGetContext(hwnd);
            let mut created_himc: HIMC = null_mut();
            let mut previous_himc: HIMC = null_mut();
            if himc.is_null() {
                created_himc = ImmCreateContext();
                assert!(!created_himc.is_null(), "IME context should create");
                previous_himc = ImmAssociateContext(hwnd, created_himc);
                himc = created_himc;
            }
            SendMessageW(hwnd, WM_IME_STARTCOMPOSITION, 0, 0);
            let composition = "compose".encode_utf16().collect::<Vec<_>>();
            assert_ne!(
                ImmSetCompositionStringW(
                    himc,
                    SCS_SETSTR,
                    composition.as_ptr().cast(),
                    u32::try_from(composition.len() * 2).expect("composition length should fit"),
                    null_mut(),
                    0,
                ),
                0,
                "composition string should set"
            );
            SendMessageW(hwnd, WM_IME_COMPOSITION, 0, GCS_COMPSTR as isize);
            assert_ne!(
                ImmNotifyIME(himc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0),
                0,
                "composition should complete"
            );
            SendMessageW(hwnd, WM_IME_COMPOSITION, 0, GCS_RESULTSTR as isize);
            if created_himc.is_null() {
                ImmReleaseContext(hwnd, himc);
            } else {
                ImmAssociateContext(hwnd, previous_himc);
                ImmDestroyContext(created_himc);
            }
            SendMessageW(hwnd, WM_IME_ENDCOMPOSITION, 0, 0);
        }

        let frame = RuntimeHost::events_session_pump(&mut host, session).expect("session pump");
        let mut kinds = Vec::new();
        let mut committed_text = None;
        while let Some(event) = RuntimeHost::events_poll(&mut host, frame).expect("event poll") {
            kinds.push(event.kind);
            if event.kind == EVENT_TEXT_COMPOSITION_COMMITTED {
                committed_text = Some(event.text.clone());
            }
        }

        assert_eq!(kinds.first().copied(), Some(EVENT_APP_RESUMED));
        assert_eq!(kinds.last().copied(), Some(EVENT_ABOUT_TO_WAIT));
        assert!(kinds.contains(&EVENT_TEXT_INPUT));
        assert!(kinds.contains(&EVENT_TEXT_COMPOSITION_STARTED));
        assert!(kinds.contains(&EVENT_TEXT_COMPOSITION_COMMITTED));
        assert!(!kinds.contains(&EVENT_TEXT_COMPOSITION_CANCELLED));
        assert_eq!(committed_text.as_deref(), Some("compose"));

        RuntimeHost::window_close(&mut host, window).expect("window close should succeed");
    }
}
