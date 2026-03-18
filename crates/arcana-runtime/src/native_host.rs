use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::c_void;
use std::io::{self, BufRead, Write};
use std::mem::{size_of, zeroed};
use std::path::Path;
use std::ptr::null_mut;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use font8x8::{BASIC_FONTS, UnicodeFonts};
use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, DIB_RGB_COLORS, EndPaint, GetMonitorInfoW,
    HDC, InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, PAINTSTRUCT,
    ReleaseDC, SRCCOPY, StretchDIBits, UpdateWindow,
};
use windows_sys::Win32::Media::Audio::{
    HWAVEOUT, WAVE_FORMAT_PCM, WAVE_FORMAT_QUERY, WAVE_MAPPER, WAVEFORMATEX, WAVEHDR,
    WHDR_BEGINLOOP, WHDR_DONE, WHDR_ENDLOOP, waveOutClose, waveOutGetErrorTextW,
    waveOutGetPosition, waveOutOpen, waveOutPause, waveOutPrepareHeader, waveOutReset,
    waveOutRestart, waveOutSetVolume, waveOutUnprepareHeader, waveOutWrite,
};
use windows_sys::Win32::Media::{MMSYSERR_NOERROR, MMTIME, TIME_SAMPLES};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleHandleExW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW,
    DefWindowProcW, DestroyWindow, DispatchMessageW, GWL_EXSTYLE, GWL_STYLE, GWLP_USERDATA,
    GetClientRect, GetWindowLongPtrW, GetWindowRect, HCURSOR, HTCLIENT, HWND_NOTOPMOST, HWND_TOP,
    HWND_TOPMOST, IDC_ARROW, IsWindow, LoadCursorW, MSG, PM_REMOVE, PeekMessageW, RegisterClassW,
    SIZE_MAXIMIZED, SIZE_MINIMIZED, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SetCursor,
    SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage, WM_CLOSE,
    WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOVE, WM_NCCREATE, WM_NCDESTROY, WM_PAINT,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETFOCUS, WM_SIZE, WM_XBUTTONDOWN, WM_XBUTTONUP,
    WNDCLASSW, WS_EX_APPWINDOW, WS_MAXIMIZEBOX, WS_OVERLAPPEDWINDOW, WS_SIZEBOX, WS_VISIBLE,
};

use crate::{
    BufferedAppFrame, BufferedEvent, BufferedFrameInput, BufferedHost, RuntimeAppFrameHandle,
    RuntimeAudioBufferHandle, RuntimeAudioDeviceHandle, RuntimeAudioPlaybackHandle, RuntimeHost,
    RuntimeImageHandle, RuntimeWindowHandle, common_named_key_code, common_named_mouse_button_code,
    ensure_audio_buffer_matches_device,
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
const WM_MOUSELEAVE_MESSAGE: u32 = 0x02A3;

static REGISTERED_WINDOW_CLASS: OnceLock<Result<NativeWindowClass, String>> = OnceLock::new();

struct NativeWindowClass {
    module_handle: usize,
    name: Vec<u16>,
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
    resized: bool,
    fullscreen: bool,
    minimized: bool,
    maximized: bool,
    focused: bool,
    resizable: bool,
    topmost: bool,
    cursor_visible: bool,
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
        let mut base = BufferedHost::default();
        base.args = std::env::args().skip(1).collect();
        base.env = std::env::vars().collect();
        base.allow_process = true;
        base.cwd = std::env::current_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Self {
            base,
            started: Instant::now(),
            next_window_handle: 0,
            windows: BTreeMap::new(),
            next_image_handle: 0,
            images: BTreeMap::new(),
            next_frame_handle: 0,
            frames: BTreeMap::new(),
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

    fn window_ref(&self, handle: RuntimeWindowHandle) -> Result<&NativeWindowState, String> {
        self.windows
            .get(&handle)
            .map(Box::as_ref)
            .ok_or_else(|| format!("invalid Window handle `{}`", handle.0))
    }

    fn window_mut(
        &mut self,
        handle: RuntimeWindowHandle,
    ) -> Result<&mut NativeWindowState, String> {
        self.windows
            .get_mut(&handle)
            .map(Box::as_mut)
            .ok_or_else(|| format!("invalid Window handle `{}`", handle.0))
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
        let style = WS_OVERLAPPEDWINDOW | WS_VISIBLE;
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
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
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
        Ok(())
    }

    fn snapshot_frame_input(window: &mut NativeWindowState) -> BufferedFrameInput {
        let input = BufferedFrameInput {
            key_down: window.key_down.iter().copied().collect(),
            key_pressed: window.key_pressed.iter().copied().collect(),
            key_released: window.key_released.iter().copied().collect(),
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
        let window = self.window_ref(window)?;
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

    fn window_set_title(&mut self, window: RuntimeWindowHandle, title: &str) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.title = title.to_string();
        let title_wide = wide_null(title);
        if unsafe { SetWindowTextW(window.hwnd, title_wide.as_ptr()) } == 0 {
            return Err("failed to update window title".to_string());
        }
        Ok(())
    }

    fn window_set_resizable(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        let mut style = unsafe { GetWindowLongPtrW(window.hwnd, GWL_STYLE) };
        if enabled {
            style |= (WS_SIZEBOX | WS_MAXIMIZEBOX) as isize;
        } else {
            style &= !((WS_SIZEBOX | WS_MAXIMIZEBOX) as isize);
        }
        unsafe {
            SetWindowLongPtrW(window.hwnd, GWL_STYLE, style);
            SetWindowPos(
                window.hwnd,
                null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            );
        }
        window.resizable = enabled;
        Ok(())
    }

    fn window_set_fullscreen(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        if window.fullscreen == enabled {
            return Ok(());
        }
        if enabled {
            window.restore_style = unsafe { GetWindowLongPtrW(window.hwnd, GWL_STYLE) };
            window.restore_ex_style = unsafe { GetWindowLongPtrW(window.hwnd, GWL_EXSTYLE) };
            unsafe {
                GetWindowRect(window.hwnd, &mut window.restore_rect);
            }
            let monitor = unsafe { MonitorFromWindow(window.hwnd, MONITOR_DEFAULTTONEAREST) };
            let mut monitor_info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..unsafe { zeroed() }
            };
            if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) } == 0 {
                return Err("failed to resolve monitor bounds for fullscreen window".to_string());
            }
            unsafe {
                SetWindowLongPtrW(
                    window.hwnd,
                    GWL_STYLE,
                    window.restore_style & !(WS_OVERLAPPEDWINDOW as isize),
                );
                SetWindowPos(
                    window.hwnd,
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
                SetWindowLongPtrW(window.hwnd, GWL_STYLE, window.restore_style);
                SetWindowLongPtrW(window.hwnd, GWL_EXSTYLE, window.restore_ex_style);
                SetWindowPos(
                    window.hwnd,
                    HWND_TOP,
                    window.restore_rect.left,
                    window.restore_rect.top,
                    window.restore_rect.right - window.restore_rect.left,
                    window.restore_rect.bottom - window.restore_rect.top,
                    SWP_FRAMECHANGED | SWP_NOOWNERZORDER,
                );
            }
        }
        window.fullscreen = enabled;
        Ok(())
    }

    fn window_set_minimized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        unsafe {
            ShowWindow(window.hwnd, if enabled { SW_MINIMIZE } else { SW_RESTORE });
        }
        window.minimized = enabled;
        Ok(())
    }

    fn window_set_maximized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        unsafe {
            ShowWindow(window.hwnd, if enabled { SW_MAXIMIZE } else { SW_RESTORE });
        }
        window.maximized = enabled;
        Ok(())
    }

    fn window_set_topmost(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        unsafe {
            SetWindowPos(
                window.hwnd,
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
        window.topmost = enabled;
        Ok(())
    }

    fn window_set_cursor_visible(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let window = self.window_mut(window)?;
        window.cursor_visible = enabled;
        unsafe {
            InvalidateRect(window.hwnd, null_mut(), 0);
        }
        Ok(())
    }

    fn window_close(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        if !self.windows.contains_key(&window) {
            return Err(format!("invalid Window handle `{}`", window.0));
        }
        let hwnd = self.window_ref(window)?.hwnd;
        if !hwnd.is_null() && unsafe { IsWindow(hwnd) != 0 } {
            unsafe {
                DestroyWindow(hwnd);
            }
            self.pump_messages()?;
        }
        if let Some(window) = self.windows.get_mut(&window) {
            window.closed = true;
        }
        Ok(())
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
        Ok((
            i64::try_from(text.chars().count()).map_err(|_| "label width overflow".to_string())?
                * 8,
            16,
        ))
    }

    fn canvas_present(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        self.present_window(window)?;
        self.pump_messages()
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
        let window = self.window_mut(window)?;
        let events = std::mem::take(&mut window.events)
            .into_iter()
            .collect::<Vec<_>>();
        let input = Self::snapshot_frame_input(window);
        Ok(self.insert_frame(BufferedAppFrame { events, input }))
    }

    fn events_poll(&mut self, frame: RuntimeAppFrameHandle) -> Result<(i64, i64, i64), String> {
        let frame = self.frame_mut(frame)?;
        let Some(event) = frame.events.first().cloned() else {
            return Ok((0, 0, 0));
        };
        frame.events.remove(0);
        Ok((event.kind, event.a, event.b))
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
            resized: false,
            fullscreen: false,
            minimized: false,
            maximized: false,
            focused: true,
            resizable: true,
            topmost: false,
            cursor_visible: true,
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
        }
    }

    fn push_event(&mut self, kind: i64, a: i64, b: i64) {
        self.events.push_back(BufferedEvent { kind, a, b });
    }
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
        for py in y0..y1 {
            for px in x0..x1 {
                self.set_pixel(px, py, color);
            }
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
        for (index, ch) in text.chars().enumerate() {
            let x_offset = x + i64::try_from(index).unwrap_or(0) * 8;
            self.draw_char(x_offset, y, ch, color);
        }
    }

    fn draw_char(&mut self, x: i64, y: i64, ch: char, color: u32) {
        let glyph = BASIC_FONTS.get(ch).unwrap_or([0; 8]);
        for (row, bits) in glyph.into_iter().enumerate() {
            for col in 0..8 {
                if bits & (1 << col) == 0 {
                    continue;
                }
                let px = x + col;
                let py = y + row as i64 * 2;
                self.set_pixel(px, py, color);
                self.set_pixel(px, py + 1, color);
            }
        }
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

    match message {
        WM_PAINT => {
            let mut paint = unsafe { zeroed::<PAINTSTRUCT>() };
            let dc = unsafe { BeginPaint(hwnd, &mut paint) };
            let _ = blit_surface_to_dc(&state.surface, hwnd, dc);
            unsafe { EndPaint(hwnd, &paint) };
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
            state.push_event(EVENT_WINDOW_FOCUSED, 1, 0);
            0
        }
        WM_KILLFOCUS => {
            state.focused = false;
            state.push_event(EVENT_WINDOW_FOCUSED, 0, 0);
            0
        }
        WM_KEYDOWN => {
            let key = wparam as i64;
            if state.key_down.insert(key) {
                state.key_pressed.insert(key);
                state.push_event(EVENT_KEY_DOWN, key, 0);
            }
            0
        }
        WM_KEYUP => {
            let key = wparam as i64;
            state.key_down.remove(&key);
            state.key_released.insert(key);
            state.push_event(EVENT_KEY_UP, key, 0);
            0
        }
        WM_MOUSEMOVE => {
            let x = signed_loword(lparam as u32) as i64;
            let y = signed_hiword(lparam as u32) as i64;
            state.mouse_pos = (x, y);
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
            0
        }
        WM_MOUSELEAVE_MESSAGE => {
            state.mouse_in_window = false;
            state.push_event(EVENT_MOUSE_LEFT, 0, 0);
            0
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
            let button = mouse_button_from_message(message, wparam as u32);
            state.mouse_down.insert(button);
            state.mouse_pressed.insert(button);
            state.push_event(EVENT_MOUSE_DOWN, button, 0);
            0
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
            let button = mouse_button_from_message(message, wparam as u32);
            state.mouse_down.remove(&button);
            state.mouse_released.insert(button);
            state.push_event(EVENT_MOUSE_UP, button, 0);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = signed_hiword(wparam as u32) as i64 / 120;
            state.mouse_wheel_y += delta;
            state.push_event(EVENT_MOUSE_WHEEL_Y, delta, 0);
            0
        }
        WM_CLOSE => {
            state.push_event(EVENT_WINDOW_CLOSE_REQUESTED, 0, 0);
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            state.closed = true;
            0
        }
        WM_SETCURSOR => {
            if !state.cursor_visible && loword(lparam as u32) as u32 == HTCLIENT {
                unsafe { SetCursor(null_mut()) };
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
    if data.len() % sample_bytes != 0 {
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

    use super::{
        NativeAudioPlayback, RuntimeAudioDeviceHandle, RuntimeAudioPlaybackHandle, RuntimeHost,
        decode_bmp_bytes, decode_wav_bytes, native_window_class_name_text,
        release_playback_resources,
    };
    use crate::NativeProcessHost;
    use windows_sys::Win32::Media::Audio::{HWAVEOUT, WHDR_DONE};

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
}
