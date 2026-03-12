use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Component, Path, PathBuf};

use arcana_aot::{
    AotEntrypointArtifact, AotPackageArtifact, AotRoutineArtifact, parse_package_artifact,
};
use pathdiff::diff_paths;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeParamPlan {
    pub mode: Option<String>,
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRoutinePlan {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub behavior_attrs: BTreeMap<String, String>,
    pub params: Vec<RuntimeParamPlan>,
    pub signature_row: String,
    pub intrinsic_impl: Option<String>,
    pub foreword_rows: Vec<String>,
    pub rollup_rows: Vec<String>,
    statements: Vec<ParsedStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEntrypointPlan {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
    pub routine_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePackagePlan {
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub runtime_requirements: Vec<String>,
    pub module_aliases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub entrypoints: Vec<RuntimeEntrypointPlan>,
    pub routines: Vec<RuntimeRoutinePlan>,
}

impl RuntimePackagePlan {
    pub fn main_entrypoint(&self) -> Option<&RuntimeEntrypointPlan> {
        self.entrypoints
            .iter()
            .find(|entry| entry.symbol_kind == "fn" && entry.symbol_name == "main")
    }
}

fn runtime_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    let mut saw_root = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => {
                normalized.push(component.as_os_str());
                saw_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() && !saw_root {
                    normalized.push("..");
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    if normalized.as_os_str().is_empty() && saw_root {
        normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR));
    }
    normalized
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeFileStreamHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeWindowHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeImageHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAppFrameHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAudioDeviceHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAudioBufferHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAudioPlaybackHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeOpaqueValue {
    FileStream(RuntimeFileStreamHandle),
    Window(RuntimeWindowHandle),
    Image(RuntimeImageHandle),
    AppFrame(RuntimeAppFrameHandle),
    AudioDevice(RuntimeAudioDeviceHandle),
    AudioBuffer(RuntimeAudioBufferHandle),
    AudioPlayback(RuntimeAudioPlaybackHandle),
}

pub trait RuntimeHost {
    fn print(&mut self, text: &str) -> Result<(), String>;
    fn eprint(&mut self, text: &str) -> Result<(), String>;
    fn flush_stdout(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn flush_stderr(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn stdin_read_line(&mut self) -> Result<String, String> {
        Err("runtime host stdin_read_line is not implemented".to_string())
    }
    fn arg_count(&mut self) -> Result<usize, String> {
        Ok(0)
    }
    fn arg_get(&mut self, index: usize) -> Result<String, String> {
        Ok(self
            .arg_count()?
            .checked_sub(index + 1)
            .map(|_| String::new())
            .unwrap_or_default())
    }
    fn env_has(&mut self, name: &str) -> Result<bool, String> {
        let _ = name;
        Ok(false)
    }
    fn env_get(&mut self, name: &str) -> Result<String, String> {
        let _ = name;
        Ok(String::new())
    }
    fn cwd(&mut self) -> Result<String, String> {
        Err("runtime host cwd is not implemented".to_string())
    }
    fn path_join(&mut self, a: &str, b: &str) -> Result<String, String> {
        let _ = (a, b);
        Err("runtime host path_join is not implemented".to_string())
    }
    fn path_normalize(&mut self, path: &str) -> Result<String, String> {
        let _ = path;
        Err("runtime host path_normalize is not implemented".to_string())
    }
    fn path_parent(&mut self, path: &str) -> Result<String, String> {
        let _ = path;
        Err("runtime host path_parent is not implemented".to_string())
    }
    fn path_file_name(&mut self, path: &str) -> Result<String, String> {
        let _ = path;
        Err("runtime host path_file_name is not implemented".to_string())
    }
    fn path_ext(&mut self, path: &str) -> Result<String, String> {
        let _ = path;
        Err("runtime host path_ext is not implemented".to_string())
    }
    fn path_is_absolute(&mut self, path: &str) -> Result<bool, String> {
        let _ = path;
        Err("runtime host path_is_absolute is not implemented".to_string())
    }
    fn path_stem(&mut self, path: &str) -> Result<String, String> {
        let _ = path;
        Err("runtime host path_stem is not implemented".to_string())
    }
    fn path_with_ext(&mut self, path: &str, ext: &str) -> Result<String, String> {
        let _ = (path, ext);
        Err("runtime host path_with_ext is not implemented".to_string())
    }
    fn path_relative_to(&mut self, path: &str, base: &str) -> Result<String, String> {
        let _ = (path, base);
        Err("runtime host path_relative_to is not implemented".to_string())
    }
    fn path_canonicalize(&mut self, path: &str) -> Result<String, String> {
        let _ = path;
        Err("runtime host path_canonicalize is not implemented".to_string())
    }
    fn path_strip_prefix(&mut self, path: &str, prefix: &str) -> Result<String, String> {
        let _ = (path, prefix);
        Err("runtime host path_strip_prefix is not implemented".to_string())
    }
    fn fs_exists(&mut self, path: &str) -> Result<bool, String> {
        let _ = path;
        Err("runtime host fs_exists is not implemented".to_string())
    }
    fn fs_is_file(&mut self, path: &str) -> Result<bool, String> {
        let _ = path;
        Err("runtime host fs_is_file is not implemented".to_string())
    }
    fn fs_is_dir(&mut self, path: &str) -> Result<bool, String> {
        let _ = path;
        Err("runtime host fs_is_dir is not implemented".to_string())
    }
    fn fs_read_text(&mut self, path: &str) -> Result<String, String> {
        let _ = path;
        Err("runtime host fs_read_text is not implemented".to_string())
    }
    fn fs_read_bytes(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let _ = path;
        Err("runtime host fs_read_bytes is not implemented".to_string())
    }
    fn fs_stream_open_read(&mut self, path: &str) -> Result<RuntimeFileStreamHandle, String> {
        let _ = path;
        Err("runtime host fs_stream_open_read is not implemented".to_string())
    }
    fn fs_stream_open_write(
        &mut self,
        path: &str,
        append: bool,
    ) -> Result<RuntimeFileStreamHandle, String> {
        let _ = (path, append);
        Err("runtime host fs_stream_open_write is not implemented".to_string())
    }
    fn fs_stream_read(
        &mut self,
        stream: RuntimeFileStreamHandle,
        max_bytes: usize,
    ) -> Result<Vec<u8>, String> {
        let _ = (stream, max_bytes);
        Err("runtime host fs_stream_read is not implemented".to_string())
    }
    fn fs_stream_write(
        &mut self,
        stream: RuntimeFileStreamHandle,
        bytes: &[u8],
    ) -> Result<usize, String> {
        let _ = (stream, bytes);
        Err("runtime host fs_stream_write is not implemented".to_string())
    }
    fn fs_stream_eof(&mut self, stream: RuntimeFileStreamHandle) -> Result<bool, String> {
        let _ = stream;
        Err("runtime host fs_stream_eof is not implemented".to_string())
    }
    fn fs_stream_close(&mut self, stream: RuntimeFileStreamHandle) -> Result<(), String> {
        let _ = stream;
        Err("runtime host fs_stream_close is not implemented".to_string())
    }
    fn fs_write_text(&mut self, path: &str, text: &str) -> Result<(), String> {
        let _ = (path, text);
        Err("runtime host fs_write_text is not implemented".to_string())
    }
    fn fs_write_bytes(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let _ = (path, bytes);
        Err("runtime host fs_write_bytes is not implemented".to_string())
    }
    fn fs_list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        let _ = path;
        Err("runtime host fs_list_dir is not implemented".to_string())
    }
    fn fs_mkdir_all(&mut self, path: &str) -> Result<(), String> {
        let _ = path;
        Err("runtime host fs_mkdir_all is not implemented".to_string())
    }
    fn fs_create_dir(&mut self, path: &str) -> Result<(), String> {
        let _ = path;
        Err("runtime host fs_create_dir is not implemented".to_string())
    }
    fn fs_remove_file(&mut self, path: &str) -> Result<(), String> {
        let _ = path;
        Err("runtime host fs_remove_file is not implemented".to_string())
    }
    fn fs_remove_dir(&mut self, path: &str) -> Result<(), String> {
        let _ = path;
        Err("runtime host fs_remove_dir is not implemented".to_string())
    }
    fn fs_remove_dir_all(&mut self, path: &str) -> Result<(), String> {
        let _ = path;
        Err("runtime host fs_remove_dir_all is not implemented".to_string())
    }
    fn fs_copy_file(&mut self, from: &str, to: &str) -> Result<(), String> {
        let _ = (from, to);
        Err("runtime host fs_copy_file is not implemented".to_string())
    }
    fn fs_rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        let _ = (from, to);
        Err("runtime host fs_rename is not implemented".to_string())
    }
    fn fs_file_size(&mut self, path: &str) -> Result<i64, String> {
        let _ = path;
        Err("runtime host fs_file_size is not implemented".to_string())
    }
    fn fs_modified_unix_ms(&mut self, path: &str) -> Result<i64, String> {
        let _ = path;
        Err("runtime host fs_modified_unix_ms is not implemented".to_string())
    }
    fn window_open(
        &mut self,
        title: &str,
        width: i64,
        height: i64,
    ) -> Result<RuntimeWindowHandle, String> {
        let _ = (title, width, height);
        Err("runtime host window_open is not implemented".to_string())
    }
    fn window_alive(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        let _ = window;
        Err("runtime host window_alive is not implemented".to_string())
    }
    fn window_size(&mut self, window: RuntimeWindowHandle) -> Result<(i64, i64), String> {
        let _ = window;
        Err("runtime host window_size is not implemented".to_string())
    }
    fn window_resized(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        let _ = window;
        Err("runtime host window_resized is not implemented".to_string())
    }
    fn window_fullscreen(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        let _ = window;
        Err("runtime host window_fullscreen is not implemented".to_string())
    }
    fn window_minimized(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        let _ = window;
        Err("runtime host window_minimized is not implemented".to_string())
    }
    fn window_maximized(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        let _ = window;
        Err("runtime host window_maximized is not implemented".to_string())
    }
    fn window_focused(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        let _ = window;
        Err("runtime host window_focused is not implemented".to_string())
    }
    fn window_set_title(&mut self, window: RuntimeWindowHandle, title: &str) -> Result<(), String> {
        let _ = (window, title);
        Err("runtime host window_set_title is not implemented".to_string())
    }
    fn window_set_resizable(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let _ = (window, enabled);
        Err("runtime host window_set_resizable is not implemented".to_string())
    }
    fn window_set_fullscreen(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let _ = (window, enabled);
        Err("runtime host window_set_fullscreen is not implemented".to_string())
    }
    fn window_set_minimized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let _ = (window, enabled);
        Err("runtime host window_set_minimized is not implemented".to_string())
    }
    fn window_set_maximized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let _ = (window, enabled);
        Err("runtime host window_set_maximized is not implemented".to_string())
    }
    fn window_set_topmost(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let _ = (window, enabled);
        Err("runtime host window_set_topmost is not implemented".to_string())
    }
    fn window_set_cursor_visible(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        let _ = (window, enabled);
        Err("runtime host window_set_cursor_visible is not implemented".to_string())
    }
    fn window_close(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        let _ = window;
        Err("runtime host window_close is not implemented".to_string())
    }
    fn canvas_fill(&mut self, window: RuntimeWindowHandle, color: i64) -> Result<(), String> {
        let _ = (window, color);
        Err("runtime host canvas_fill is not implemented".to_string())
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
        let _ = (window, x, y, w, h, color);
        Err("runtime host canvas_rect is not implemented".to_string())
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
        let _ = (window, x1, y1, x2, y2, color);
        Err("runtime host canvas_line is not implemented".to_string())
    }
    fn canvas_circle_fill(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
        radius: i64,
        color: i64,
    ) -> Result<(), String> {
        let _ = (window, x, y, radius, color);
        Err("runtime host canvas_circle_fill is not implemented".to_string())
    }
    fn canvas_label(
        &mut self,
        window: RuntimeWindowHandle,
        x: i64,
        y: i64,
        text: &str,
        color: i64,
    ) -> Result<(), String> {
        let _ = (window, x, y, text, color);
        Err("runtime host canvas_label is not implemented".to_string())
    }
    fn canvas_label_size(&mut self, text: &str) -> Result<(i64, i64), String> {
        let _ = text;
        Err("runtime host canvas_label_size is not implemented".to_string())
    }
    fn canvas_present(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        let _ = window;
        Err("runtime host canvas_present is not implemented".to_string())
    }
    fn canvas_rgb(&mut self, r: i64, g: i64, b: i64) -> Result<i64, String> {
        let _ = (r, g, b);
        Err("runtime host canvas_rgb is not implemented".to_string())
    }
    fn image_load(&mut self, path: &str) -> Result<RuntimeImageHandle, String> {
        let _ = path;
        Err("runtime host image_load is not implemented".to_string())
    }
    fn canvas_image_size(&mut self, image: RuntimeImageHandle) -> Result<(i64, i64), String> {
        let _ = image;
        Err("runtime host canvas_image_size is not implemented".to_string())
    }
    fn canvas_blit(
        &mut self,
        window: RuntimeWindowHandle,
        image: RuntimeImageHandle,
        x: i64,
        y: i64,
    ) -> Result<(), String> {
        let _ = (window, image, x, y);
        Err("runtime host canvas_blit is not implemented".to_string())
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
        let _ = (window, image, x, y, w, h);
        Err("runtime host canvas_blit_scaled is not implemented".to_string())
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
        let _ = (window, image, sx, sy, sw, sh, dx, dy, dw, dh);
        Err("runtime host canvas_blit_region is not implemented".to_string())
    }
    fn events_pump(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<RuntimeAppFrameHandle, String> {
        let _ = window;
        Err("runtime host events_pump is not implemented".to_string())
    }
    fn events_poll(&mut self, frame: RuntimeAppFrameHandle) -> Result<(i64, i64, i64), String> {
        let _ = frame;
        Err("runtime host events_poll is not implemented".to_string())
    }
    fn input_key_code(&mut self, name: &str) -> Result<i64, String> {
        let _ = name;
        Err("runtime host input_key_code is not implemented".to_string())
    }
    fn input_key_down(&mut self, frame: RuntimeAppFrameHandle, key: i64) -> Result<bool, String> {
        let _ = (frame, key);
        Err("runtime host input_key_down is not implemented".to_string())
    }
    fn input_key_pressed(
        &mut self,
        frame: RuntimeAppFrameHandle,
        key: i64,
    ) -> Result<bool, String> {
        let _ = (frame, key);
        Err("runtime host input_key_pressed is not implemented".to_string())
    }
    fn input_key_released(
        &mut self,
        frame: RuntimeAppFrameHandle,
        key: i64,
    ) -> Result<bool, String> {
        let _ = (frame, key);
        Err("runtime host input_key_released is not implemented".to_string())
    }
    fn input_mouse_button_code(&mut self, name: &str) -> Result<i64, String> {
        let _ = name;
        Err("runtime host input_mouse_button_code is not implemented".to_string())
    }
    fn input_mouse_pos(&mut self, frame: RuntimeAppFrameHandle) -> Result<(i64, i64), String> {
        let _ = frame;
        Err("runtime host input_mouse_pos is not implemented".to_string())
    }
    fn input_mouse_down(
        &mut self,
        frame: RuntimeAppFrameHandle,
        button: i64,
    ) -> Result<bool, String> {
        let _ = (frame, button);
        Err("runtime host input_mouse_down is not implemented".to_string())
    }
    fn input_mouse_pressed(
        &mut self,
        frame: RuntimeAppFrameHandle,
        button: i64,
    ) -> Result<bool, String> {
        let _ = (frame, button);
        Err("runtime host input_mouse_pressed is not implemented".to_string())
    }
    fn input_mouse_released(
        &mut self,
        frame: RuntimeAppFrameHandle,
        button: i64,
    ) -> Result<bool, String> {
        let _ = (frame, button);
        Err("runtime host input_mouse_released is not implemented".to_string())
    }
    fn input_mouse_wheel_y(&mut self, frame: RuntimeAppFrameHandle) -> Result<i64, String> {
        let _ = frame;
        Err("runtime host input_mouse_wheel_y is not implemented".to_string())
    }
    fn input_mouse_in_window(&mut self, frame: RuntimeAppFrameHandle) -> Result<bool, String> {
        let _ = frame;
        Err("runtime host input_mouse_in_window is not implemented".to_string())
    }
    fn audio_default_output(&mut self) -> Result<RuntimeAudioDeviceHandle, String> {
        Err("runtime host audio_default_output is not implemented".to_string())
    }
    fn audio_output_close(&mut self, device: RuntimeAudioDeviceHandle) -> Result<(), String> {
        let _ = device;
        Err("runtime host audio_output_close is not implemented".to_string())
    }
    fn audio_output_sample_rate_hz(
        &mut self,
        device: RuntimeAudioDeviceHandle,
    ) -> Result<i64, String> {
        let _ = device;
        Err("runtime host audio_output_sample_rate_hz is not implemented".to_string())
    }
    fn audio_output_channels(&mut self, device: RuntimeAudioDeviceHandle) -> Result<i64, String> {
        let _ = device;
        Err("runtime host audio_output_channels is not implemented".to_string())
    }
    fn audio_buffer_load_wav(&mut self, path: &str) -> Result<RuntimeAudioBufferHandle, String> {
        let _ = path;
        Err("runtime host audio_buffer_load_wav is not implemented".to_string())
    }
    fn audio_buffer_frames(&mut self, buffer: RuntimeAudioBufferHandle) -> Result<i64, String> {
        let _ = buffer;
        Err("runtime host audio_buffer_frames is not implemented".to_string())
    }
    fn audio_buffer_channels(&mut self, buffer: RuntimeAudioBufferHandle) -> Result<i64, String> {
        let _ = buffer;
        Err("runtime host audio_buffer_channels is not implemented".to_string())
    }
    fn audio_buffer_sample_rate_hz(
        &mut self,
        buffer: RuntimeAudioBufferHandle,
    ) -> Result<i64, String> {
        let _ = buffer;
        Err("runtime host audio_buffer_sample_rate_hz is not implemented".to_string())
    }
    fn audio_play_buffer(
        &mut self,
        device: RuntimeAudioDeviceHandle,
        buffer: RuntimeAudioBufferHandle,
    ) -> Result<RuntimeAudioPlaybackHandle, String> {
        let _ = (device, buffer);
        Err("runtime host audio_play_buffer is not implemented".to_string())
    }
    fn audio_output_set_gain_milli(
        &mut self,
        device: RuntimeAudioDeviceHandle,
        milli: i64,
    ) -> Result<(), String> {
        let _ = (device, milli);
        Err("runtime host audio_output_set_gain_milli is not implemented".to_string())
    }
    fn audio_playback_stop(&mut self, playback: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        let _ = playback;
        Err("runtime host audio_playback_stop is not implemented".to_string())
    }
    fn audio_playback_pause(&mut self, playback: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        let _ = playback;
        Err("runtime host audio_playback_pause is not implemented".to_string())
    }
    fn audio_playback_resume(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<(), String> {
        let _ = playback;
        Err("runtime host audio_playback_resume is not implemented".to_string())
    }
    fn audio_playback_playing(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        let _ = playback;
        Err("runtime host audio_playback_playing is not implemented".to_string())
    }
    fn audio_playback_paused(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        let _ = playback;
        Err("runtime host audio_playback_paused is not implemented".to_string())
    }
    fn audio_playback_finished(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        let _ = playback;
        Err("runtime host audio_playback_finished is not implemented".to_string())
    }
    fn audio_playback_set_gain_milli(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
        milli: i64,
    ) -> Result<(), String> {
        let _ = (playback, milli);
        Err("runtime host audio_playback_set_gain_milli is not implemented".to_string())
    }
    fn audio_playback_set_looping(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
        looping: bool,
    ) -> Result<(), String> {
        let _ = (playback, looping);
        Err("runtime host audio_playback_set_looping is not implemented".to_string())
    }
    fn audio_playback_looping(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        let _ = playback;
        Err("runtime host audio_playback_looping is not implemented".to_string())
    }
    fn audio_playback_position_frames(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<i64, String> {
        let _ = playback;
        Err("runtime host audio_playback_position_frames is not implemented".to_string())
    }
    fn monotonic_now_ms(&mut self) -> Result<i64, String> {
        Err("runtime host monotonic_now_ms is not implemented".to_string())
    }
    fn monotonic_now_ns(&mut self) -> Result<i64, String> {
        Err("runtime host monotonic_now_ns is not implemented".to_string())
    }
    fn sleep_ms(&mut self, ms: i64) -> Result<(), String> {
        let _ = ms;
        Err("runtime host sleep_ms is not implemented".to_string())
    }
    fn process_exec_status(&mut self, program: &str, args: &[String]) -> Result<i64, String> {
        let _ = (program, args);
        Err("process execution is disabled on this runtime host".to_string())
    }
    fn process_exec_capture(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<(i64, Vec<u8>, Vec<u8>, bool, bool), String> {
        let _ = (program, args);
        Err("process execution is disabled on this runtime host".to_string())
    }
    fn text_len_bytes(&mut self, text: &str) -> Result<i64, String> {
        Ok(i64::try_from(text.len()).map_err(|_| "text length does not fit in i64".to_string())?)
    }
    fn text_byte_at(&mut self, text: &str, index: usize) -> Result<i64, String> {
        let bytes = text.as_bytes();
        let byte = bytes
            .get(index)
            .copied()
            .ok_or_else(|| format!("text byte index `{index}` is out of bounds"))?;
        Ok(i64::from(byte))
    }
    fn text_slice_bytes(&mut self, text: &str, start: usize, end: usize) -> Result<String, String> {
        let bytes = text.as_bytes();
        let slice = bytes
            .get(start..end)
            .ok_or_else(|| format!("text byte slice `{start}..{end}` is out of bounds"))?;
        std::str::from_utf8(slice)
            .map(|slice| slice.to_string())
            .map_err(|_| format!("text byte slice `{start}..{end}` is not valid UTF-8"))
    }
    fn text_starts_with(&mut self, text: &str, prefix: &str) -> Result<bool, String> {
        Ok(text.starts_with(prefix))
    }
    fn text_ends_with(&mut self, text: &str, suffix: &str) -> Result<bool, String> {
        Ok(text.ends_with(suffix))
    }
    fn text_split_lines(&mut self, text: &str) -> Result<Vec<String>, String> {
        Ok(text.lines().map(ToString::to_string).collect())
    }
    fn text_from_int(&mut self, value: i64) -> Result<String, String> {
        Ok(value.to_string())
    }
    fn bytes_from_str_utf8(&mut self, text: &str) -> Result<Vec<u8>, String> {
        Ok(text.as_bytes().to_vec())
    }
    fn bytes_to_str_utf8(&mut self, bytes: &[u8]) -> Result<String, String> {
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
    fn bytes_sha256_hex(&mut self, bytes: &[u8]) -> Result<String, String> {
        let digest = Sha256::digest(bytes);
        Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }
}

#[derive(Debug)]
struct BufferedFileStream {
    path: String,
    file: fs::File,
    readable: bool,
    writable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferedEvent {
    kind: i64,
    a: i64,
    b: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BufferedFrameInput {
    key_down: Vec<i64>,
    key_pressed: Vec<i64>,
    key_released: Vec<i64>,
    mouse_pos: (i64, i64),
    mouse_down: Vec<i64>,
    mouse_pressed: Vec<i64>,
    mouse_released: Vec<i64>,
    mouse_wheel_y: i64,
    mouse_in_window: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BufferedWindow {
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
    draw_log: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferedImage {
    path: String,
    width: i64,
    height: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferedAppFrame {
    events: Vec<BufferedEvent>,
    input: BufferedFrameInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferedAudioDevice {
    sample_rate_hz: i64,
    channels: i64,
    gain_milli: i64,
    closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferedAudioBuffer {
    path: String,
    frames: i64,
    channels: i64,
    sample_rate_hz: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BufferedAudioPlayback {
    device: RuntimeAudioDeviceHandle,
    buffer: RuntimeAudioBufferHandle,
    paused: bool,
    finished: bool,
    gain_milli: i64,
    looping: bool,
    position_frames: i64,
}

#[derive(Debug, Default)]
pub struct BufferedHost {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub stdout_flushes: usize,
    pub stderr_flushes: usize,
    pub stdin: Vec<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub allow_process: bool,
    pub cwd: String,
    pub sandbox_root: String,
    next_stream_handle: u64,
    streams: BTreeMap<RuntimeFileStreamHandle, BufferedFileStream>,
    next_window_handle: u64,
    windows: BTreeMap<RuntimeWindowHandle, BufferedWindow>,
    next_image_handle: u64,
    images: BTreeMap<RuntimeImageHandle, BufferedImage>,
    next_frame_handle: u64,
    frames: BTreeMap<RuntimeAppFrameHandle, BufferedAppFrame>,
    next_frame_events: Vec<BufferedEvent>,
    next_frame_input: BufferedFrameInput,
    next_audio_device_handle: u64,
    audio_devices: BTreeMap<RuntimeAudioDeviceHandle, BufferedAudioDevice>,
    next_audio_buffer_handle: u64,
    audio_buffers: BTreeMap<RuntimeAudioBufferHandle, BufferedAudioBuffer>,
    next_audio_playback_handle: u64,
    audio_playbacks: BTreeMap<RuntimeAudioPlaybackHandle, BufferedAudioPlayback>,
    pub monotonic_now_ms: i64,
    pub monotonic_now_ns: i64,
    pub monotonic_step_ms: i64,
    pub monotonic_step_ns: i64,
    pub sleep_log_ms: Vec<i64>,
    pub canvas_log: Vec<String>,
    pub audio_log: Vec<String>,
}

impl BufferedHost {
    fn current_working_dir(&self) -> Result<PathBuf, String> {
        if !self.cwd.is_empty() {
            return Ok(normalize_lexical_path(Path::new(&self.cwd)));
        }
        std::env::current_dir()
            .map(|path| normalize_lexical_path(&path))
            .map_err(|err| format!("failed to resolve current directory: {err}"))
    }

    fn sandbox_root_path(&self) -> Result<Option<PathBuf>, String> {
        if self.sandbox_root.is_empty() {
            return Ok(None);
        }
        Ok(Some(normalize_lexical_path(Path::new(&self.sandbox_root))))
    }

    fn resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        let requested = PathBuf::from(path);
        let candidate = if requested.is_absolute() {
            normalize_lexical_path(&requested)
        } else {
            normalize_lexical_path(&self.current_working_dir()?.join(requested))
        };
        if let Some(root) = self.sandbox_root_path()? {
            if !candidate.starts_with(&root) {
                return Err(format!(
                    "path `{}` escapes sandbox root `{}`",
                    runtime_path_string(&candidate),
                    runtime_path_string(&root)
                ));
            }
        }
        Ok(candidate)
    }

    fn insert_stream(
        &mut self,
        path: &Path,
        file: fs::File,
        readable: bool,
        writable: bool,
    ) -> RuntimeFileStreamHandle {
        let handle = RuntimeFileStreamHandle(self.next_stream_handle);
        self.next_stream_handle += 1;
        self.streams.insert(
            handle,
            BufferedFileStream {
                path: runtime_path_string(path),
                file,
                readable,
                writable,
            },
        );
        handle
    }

    fn stream_mut(
        &mut self,
        handle: RuntimeFileStreamHandle,
    ) -> Result<&mut BufferedFileStream, String> {
        self.streams
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid FileStream handle `{}`", handle.0))
    }

    fn insert_window(&mut self, title: &str, width: i64, height: i64) -> RuntimeWindowHandle {
        let handle = RuntimeWindowHandle(self.next_window_handle);
        self.next_window_handle += 1;
        self.windows.insert(
            handle,
            BufferedWindow {
                title: title.to_string(),
                width,
                height,
                focused: true,
                resizable: true,
                cursor_visible: true,
                ..BufferedWindow::default()
            },
        );
        handle
    }

    fn window_mut(&mut self, handle: RuntimeWindowHandle) -> Result<&mut BufferedWindow, String> {
        self.windows
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Window handle `{}`", handle.0))
    }

    fn window_ref(&self, handle: RuntimeWindowHandle) -> Result<&BufferedWindow, String> {
        self.windows
            .get(&handle)
            .ok_or_else(|| format!("invalid Window handle `{}`", handle.0))
    }

    fn insert_image(&mut self, path: &str, width: i64, height: i64) -> RuntimeImageHandle {
        let handle = RuntimeImageHandle(self.next_image_handle);
        self.next_image_handle += 1;
        self.images.insert(
            handle,
            BufferedImage {
                path: path.to_string(),
                width,
                height,
            },
        );
        handle
    }

    fn image_ref(&self, handle: RuntimeImageHandle) -> Result<&BufferedImage, String> {
        self.images
            .get(&handle)
            .ok_or_else(|| format!("invalid Image handle `{}`", handle.0))
    }

    fn insert_frame(
        &mut self,
        events: Vec<BufferedEvent>,
        input: BufferedFrameInput,
    ) -> RuntimeAppFrameHandle {
        let handle = RuntimeAppFrameHandle(self.next_frame_handle);
        self.next_frame_handle += 1;
        self.frames
            .insert(handle, BufferedAppFrame { events, input });
        handle
    }

    fn frame_mut(
        &mut self,
        handle: RuntimeAppFrameHandle,
    ) -> Result<&mut BufferedAppFrame, String> {
        self.frames
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AppFrame handle `{}`", handle.0))
    }

    fn frame_ref(&self, handle: RuntimeAppFrameHandle) -> Result<&BufferedAppFrame, String> {
        self.frames
            .get(&handle)
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
            BufferedAudioDevice {
                sample_rate_hz,
                channels,
                gain_milli: 1000,
                closed: false,
            },
        );
        handle
    }

    fn audio_device_mut(
        &mut self,
        handle: RuntimeAudioDeviceHandle,
    ) -> Result<&mut BufferedAudioDevice, String> {
        self.audio_devices
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AudioDevice handle `{}`", handle.0))
    }

    fn audio_device_ref(
        &self,
        handle: RuntimeAudioDeviceHandle,
    ) -> Result<&BufferedAudioDevice, String> {
        self.audio_devices
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioDevice handle `{}`", handle.0))
    }

    fn insert_audio_buffer(
        &mut self,
        path: &str,
        frames: i64,
        channels: i64,
        sample_rate_hz: i64,
    ) -> RuntimeAudioBufferHandle {
        let handle = RuntimeAudioBufferHandle(self.next_audio_buffer_handle);
        self.next_audio_buffer_handle += 1;
        self.audio_buffers.insert(
            handle,
            BufferedAudioBuffer {
                path: path.to_string(),
                frames,
                channels,
                sample_rate_hz,
            },
        );
        handle
    }

    fn audio_buffer_ref(
        &self,
        handle: RuntimeAudioBufferHandle,
    ) -> Result<&BufferedAudioBuffer, String> {
        self.audio_buffers
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioBuffer handle `{}`", handle.0))
    }

    fn insert_audio_playback(
        &mut self,
        device: RuntimeAudioDeviceHandle,
        buffer: RuntimeAudioBufferHandle,
    ) -> Result<RuntimeAudioPlaybackHandle, String> {
        let gain_milli = self.audio_device_ref(device)?.gain_milli;
        let handle = RuntimeAudioPlaybackHandle(self.next_audio_playback_handle);
        self.next_audio_playback_handle += 1;
        self.audio_playbacks.insert(
            handle,
            BufferedAudioPlayback {
                device,
                buffer,
                paused: false,
                finished: false,
                gain_milli,
                looping: false,
                position_frames: 0,
            },
        );
        Ok(handle)
    }

    fn audio_playback_mut(
        &mut self,
        handle: RuntimeAudioPlaybackHandle,
    ) -> Result<&mut BufferedAudioPlayback, String> {
        self.audio_playbacks
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid AudioPlayback handle `{}`", handle.0))
    }

    fn audio_playback_ref(
        &self,
        handle: RuntimeAudioPlaybackHandle,
    ) -> Result<&BufferedAudioPlayback, String> {
        self.audio_playbacks
            .get(&handle)
            .ok_or_else(|| format!("invalid AudioPlayback handle `{}`", handle.0))
    }
}

impl RuntimeHost for BufferedHost {
    fn print(&mut self, text: &str) -> Result<(), String> {
        self.stdout.push(text.to_string());
        Ok(())
    }

    fn eprint(&mut self, text: &str) -> Result<(), String> {
        self.stderr.push(text.to_string());
        Ok(())
    }

    fn flush_stdout(&mut self) -> Result<(), String> {
        self.stdout_flushes += 1;
        Ok(())
    }

    fn flush_stderr(&mut self) -> Result<(), String> {
        self.stderr_flushes += 1;
        Ok(())
    }

    fn stdin_read_line(&mut self) -> Result<String, String> {
        if self.stdin.is_empty() {
            return Err("stdin has no queued line".to_string());
        }
        Ok(self.stdin.remove(0))
    }

    fn arg_count(&mut self) -> Result<usize, String> {
        Ok(self.args.len())
    }

    fn arg_get(&mut self, index: usize) -> Result<String, String> {
        Ok(self.args.get(index).cloned().unwrap_or_default())
    }

    fn env_has(&mut self, name: &str) -> Result<bool, String> {
        Ok(self.env.contains_key(name))
    }

    fn env_get(&mut self, name: &str) -> Result<String, String> {
        Ok(self.env.get(name).cloned().unwrap_or_default())
    }

    fn cwd(&mut self) -> Result<String, String> {
        Ok(runtime_path_string(&self.current_working_dir()?))
    }

    fn path_join(&mut self, a: &str, b: &str) -> Result<String, String> {
        Ok(runtime_path_string(&normalize_lexical_path(
            &Path::new(a).join(b),
        )))
    }

    fn path_normalize(&mut self, path: &str) -> Result<String, String> {
        Ok(runtime_path_string(&normalize_lexical_path(Path::new(
            path,
        ))))
    }

    fn path_parent(&mut self, path: &str) -> Result<String, String> {
        Ok(Path::new(path)
            .parent()
            .map(runtime_path_string)
            .unwrap_or_default())
    }

    fn path_file_name(&mut self, path: &str) -> Result<String, String> {
        Ok(Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default())
    }

    fn path_ext(&mut self, path: &str) -> Result<String, String> {
        Ok(Path::new(path)
            .extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or_default())
    }

    fn path_is_absolute(&mut self, path: &str) -> Result<bool, String> {
        Ok(Path::new(path).is_absolute())
    }

    fn path_stem(&mut self, path: &str) -> Result<String, String> {
        Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .ok_or_else(|| format!("path `{path}` has no stem"))
    }

    fn path_with_ext(&mut self, path: &str, ext: &str) -> Result<String, String> {
        let mut updated = PathBuf::from(path);
        updated.set_extension(ext);
        Ok(runtime_path_string(&updated))
    }

    fn path_relative_to(&mut self, path: &str, base: &str) -> Result<String, String> {
        let path = normalize_lexical_path(Path::new(path));
        let base = normalize_lexical_path(Path::new(base));
        diff_paths(&path, &base)
            .map(|relative| runtime_path_string(&relative))
            .ok_or_else(|| {
                format!(
                    "failed to make `{}` relative to `{}`",
                    runtime_path_string(&path),
                    runtime_path_string(&base)
                )
            })
    }

    fn path_canonicalize(&mut self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::canonicalize(&resolved)
            .map(|path| runtime_path_string(&normalize_lexical_path(&path)))
            .map_err(|err| format!("failed to canonicalize `{path}`: {err}"))
    }

    fn path_strip_prefix(&mut self, path: &str, prefix: &str) -> Result<String, String> {
        let path = normalize_lexical_path(Path::new(path));
        let prefix = normalize_lexical_path(Path::new(prefix));
        path.strip_prefix(&prefix)
            .map(runtime_path_string)
            .map_err(|_| {
                format!(
                    "path `{}` does not start with `{}`",
                    runtime_path_string(&path),
                    runtime_path_string(&prefix)
                )
            })
    }

    fn fs_exists(&mut self, path: &str) -> Result<bool, String> {
        Ok(self.resolve_fs_path(path)?.exists())
    }

    fn fs_is_file(&mut self, path: &str) -> Result<bool, String> {
        Ok(self.resolve_fs_path(path)?.is_file())
    }

    fn fs_is_dir(&mut self, path: &str) -> Result<bool, String> {
        Ok(self.resolve_fs_path(path)?.is_dir())
    }

    fn fs_read_text(&mut self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::read_to_string(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    }

    fn fs_read_bytes(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::read(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    }

    fn fs_stream_open_read(&mut self, path: &str) -> Result<RuntimeFileStreamHandle, String> {
        let resolved = self.resolve_fs_path(path)?;
        let file = fs::File::open(&resolved).map_err(|err| {
            format!(
                "failed to open `{}` for reading: {err}",
                runtime_path_string(&resolved)
            )
        })?;
        Ok(self.insert_stream(&resolved, file, true, false))
    }

    fn fs_stream_open_write(
        &mut self,
        path: &str,
        append: bool,
    ) -> Result<RuntimeFileStreamHandle, String> {
        let resolved = self.resolve_fs_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", runtime_path_string(parent))
            })?;
        }
        let mut options = fs::OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let file = options.open(&resolved).map_err(|err| {
            format!(
                "failed to open `{}` for writing: {err}",
                runtime_path_string(&resolved)
            )
        })?;
        Ok(self.insert_stream(&resolved, file, false, true))
    }

    fn fs_stream_read(
        &mut self,
        stream: RuntimeFileStreamHandle,
        max_bytes: usize,
    ) -> Result<Vec<u8>, String> {
        let stream = self.stream_mut(stream)?;
        if !stream.readable {
            return Err(format!(
                "FileStream `{}` is not opened for reading",
                stream.path
            ));
        }
        let mut buffer = vec![0_u8; max_bytes];
        let read = stream
            .file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read from FileStream `{}`: {err}", stream.path))?;
        buffer.truncate(read);
        Ok(buffer)
    }

    fn fs_stream_write(
        &mut self,
        stream: RuntimeFileStreamHandle,
        bytes: &[u8],
    ) -> Result<usize, String> {
        let stream = self.stream_mut(stream)?;
        if !stream.writable {
            return Err(format!(
                "FileStream `{}` is not opened for writing",
                stream.path
            ));
        }
        stream
            .file
            .write_all(bytes)
            .map_err(|err| format!("failed to write to FileStream `{}`: {err}", stream.path))?;
        Ok(bytes.len())
    }

    fn fs_stream_eof(&mut self, stream: RuntimeFileStreamHandle) -> Result<bool, String> {
        let stream = self.stream_mut(stream)?;
        if !stream.readable {
            return Err(format!(
                "FileStream `{}` is not opened for reading",
                stream.path
            ));
        }
        let position = stream
            .file
            .stream_position()
            .map_err(|err| format!("failed to inspect FileStream `{}`: {err}", stream.path))?;
        let len = stream
            .file
            .metadata()
            .map_err(|err| format!("failed to stat FileStream `{}`: {err}", stream.path))?
            .len();
        Ok(position >= len)
    }

    fn fs_stream_close(&mut self, stream: RuntimeFileStreamHandle) -> Result<(), String> {
        let Some(mut stream) = self.streams.remove(&stream) else {
            return Err(format!("invalid FileStream handle `{}`", stream.0));
        };
        if stream.writable {
            stream.file.flush().map_err(|err| {
                format!(
                    "failed to flush FileStream `{}` during close: {err}",
                    stream.path
                )
            })?;
        }
        Ok(())
    }

    fn fs_write_text(&mut self, path: &str, text: &str) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", runtime_path_string(parent))
            })?;
        }
        fs::write(&resolved, text).map_err(|err| {
            format!(
                "failed to write `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }

    fn fs_write_bytes(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", runtime_path_string(parent))
            })?;
        }
        fs::write(&resolved, bytes).map_err(|err| {
            format!(
                "failed to write `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }

    fn fs_list_dir(&mut self, path: &str) -> Result<Vec<String>, String> {
        let resolved = self.resolve_fs_path(path)?;
        let mut entries = fs::read_dir(&resolved)
            .map_err(|err| format!("failed to list `{}`: {err}", runtime_path_string(&resolved)))?
            .map(|entry| {
                entry
                    .map(|entry| runtime_path_string(&normalize_lexical_path(&entry.path())))
                    .map_err(|err| {
                        format!(
                            "failed to read directory entry in `{}`: {err}",
                            runtime_path_string(&resolved)
                        )
                    })
            })
            .collect::<Result<Vec<_>, String>>()?;
        entries.sort();
        Ok(entries)
    }

    fn fs_mkdir_all(&mut self, path: &str) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::create_dir_all(&resolved).map_err(|err| {
            format!(
                "failed to create `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }

    fn fs_create_dir(&mut self, path: &str) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::create_dir(&resolved).map_err(|err| {
            format!(
                "failed to create directory `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }

    fn fs_remove_file(&mut self, path: &str) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::remove_file(&resolved).map_err(|err| {
            format!(
                "failed to remove file `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }

    fn fs_remove_dir(&mut self, path: &str) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::remove_dir(&resolved).map_err(|err| {
            format!(
                "failed to remove directory `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }

    fn fs_remove_dir_all(&mut self, path: &str) -> Result<(), String> {
        let resolved = self.resolve_fs_path(path)?;
        fs::remove_dir_all(&resolved).map_err(|err| {
            format!(
                "failed to remove directory tree `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }

    fn fs_copy_file(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from_resolved = self.resolve_fs_path(from)?;
        let to_resolved = self.resolve_fs_path(to)?;
        if let Some(parent) = to_resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", runtime_path_string(parent))
            })?;
        }
        fs::copy(&from_resolved, &to_resolved).map_err(|err| {
            format!(
                "failed to copy `{}` to `{}`: {err}",
                runtime_path_string(&from_resolved),
                runtime_path_string(&to_resolved)
            )
        })?;
        Ok(())
    }

    fn fs_rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        let from_resolved = self.resolve_fs_path(from)?;
        let to_resolved = self.resolve_fs_path(to)?;
        if let Some(parent) = to_resolved.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to prepare `{}`: {err}", runtime_path_string(parent))
            })?;
        }
        fs::rename(&from_resolved, &to_resolved).map_err(|err| {
            format!(
                "failed to rename `{}` to `{}`: {err}",
                runtime_path_string(&from_resolved),
                runtime_path_string(&to_resolved)
            )
        })
    }

    fn fs_file_size(&mut self, path: &str) -> Result<i64, String> {
        let resolved = self.resolve_fs_path(path)?;
        let len = fs::metadata(&resolved)
            .map_err(|err| format!("failed to stat `{}`: {err}", runtime_path_string(&resolved)))?
            .len();
        i64::try_from(len).map_err(|_| format!("file size for `{}` does not fit in i64", path))
    }

    fn fs_modified_unix_ms(&mut self, path: &str) -> Result<i64, String> {
        let resolved = self.resolve_fs_path(path)?;
        let modified = fs::metadata(&resolved)
            .map_err(|err| format!("failed to stat `{}`: {err}", runtime_path_string(&resolved)))?
            .modified()
            .map_err(|err| {
                format!(
                    "failed to read modified time for `{}`: {err}",
                    runtime_path_string(&resolved)
                )
            })?;
        let duration = modified
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| {
                format!(
                    "modified time for `{}` predates unix epoch: {err}",
                    runtime_path_string(&resolved)
                )
            })?;
        i64::try_from(duration.as_millis()).map_err(|_| {
            format!(
                "modified time for `{}` does not fit in i64 milliseconds",
                runtime_path_string(&resolved)
            )
        })
    }

    fn window_open(
        &mut self,
        title: &str,
        width: i64,
        height: i64,
    ) -> Result<RuntimeWindowHandle, String> {
        Ok(self.insert_window(title, width, height))
    }

    fn window_alive(&mut self, window: RuntimeWindowHandle) -> Result<bool, String> {
        Ok(self.windows.contains_key(&window))
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
        self.window_mut(window)?.title = title.to_string();
        Ok(())
    }

    fn window_set_resizable(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        self.window_mut(window)?.resizable = enabled;
        Ok(())
    }

    fn window_set_fullscreen(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        self.window_mut(window)?.fullscreen = enabled;
        Ok(())
    }

    fn window_set_minimized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        self.window_mut(window)?.minimized = enabled;
        Ok(())
    }

    fn window_set_maximized(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        self.window_mut(window)?.maximized = enabled;
        Ok(())
    }

    fn window_set_topmost(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        self.window_mut(window)?.topmost = enabled;
        Ok(())
    }

    fn window_set_cursor_visible(
        &mut self,
        window: RuntimeWindowHandle,
        enabled: bool,
    ) -> Result<(), String> {
        self.window_mut(window)?.cursor_visible = enabled;
        Ok(())
    }

    fn window_close(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        self.windows
            .remove(&window)
            .map(|_| ())
            .ok_or_else(|| format!("invalid Window handle `{}`", window.0))
    }

    fn canvas_fill(&mut self, window: RuntimeWindowHandle, color: i64) -> Result<(), String> {
        let entry = format!("fill:{color}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
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
        let entry = format!("rect:{x},{y},{w},{h},{color}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
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
        let entry = format!("line:{x1},{y1},{x2},{y2},{color}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
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
        let entry = format!("circle:{x},{y},{radius},{color}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
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
        let entry = format!("label:{x},{y},{text},{color}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
        Ok(())
    }

    fn canvas_label_size(&mut self, text: &str) -> Result<(i64, i64), String> {
        Ok((
            i64::try_from(text.len()).map_err(|_| "label width overflow".to_string())? * 8,
            16,
        ))
    }

    fn canvas_present(&mut self, window: RuntimeWindowHandle) -> Result<(), String> {
        let entry = "present".to_string();
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
        Ok(())
    }

    fn canvas_rgb(&mut self, r: i64, g: i64, b: i64) -> Result<i64, String> {
        let clamp = |value: i64| value.clamp(0, 255);
        Ok((clamp(r) << 16) | (clamp(g) << 8) | clamp(b))
    }

    fn image_load(&mut self, path: &str) -> Result<RuntimeImageHandle, String> {
        let resolved = self.resolve_fs_path(path)?;
        if !resolved.is_file() {
            return Err(format!(
                "image `{}` does not exist",
                runtime_path_string(&resolved)
            ));
        }
        Ok(self.insert_image(&runtime_path_string(&resolved), 16, 16))
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
        let image_path = self.image_ref(image)?.path.clone();
        let entry = format!("blit:{image_path},{x},{y}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
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
        let image_path = self.image_ref(image)?.path.clone();
        let entry = format!("blit_scaled:{image_path},{x},{y},{w},{h}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
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
        let image_path = self.image_ref(image)?.path.clone();
        let entry = format!("blit_region:{image_path},{sx},{sy},{sw},{sh},{dx},{dy},{dw},{dh}");
        self.window_mut(window)?.draw_log.push(entry.clone());
        self.canvas_log.push(entry);
        Ok(())
    }

    fn events_pump(
        &mut self,
        window: RuntimeWindowHandle,
    ) -> Result<RuntimeAppFrameHandle, String> {
        if !self.windows.contains_key(&window) {
            return Err(format!("invalid Window handle `{}`", window.0));
        }
        self.window_mut(window)?.resized = false;
        let events = std::mem::take(&mut self.next_frame_events);
        let input = std::mem::take(&mut self.next_frame_input);
        Ok(self.insert_frame(events, input))
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
        Ok(match name {
            "A" | "a" => 65,
            "Space" | "space" => 32,
            "Escape" | "escape" => 27,
            _ => -1,
        })
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
        Ok(match name {
            "Left" | "left" => 1,
            "Right" | "right" => 2,
            "Middle" | "middle" => 3,
            _ => -1,
        })
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
        let handle = self.insert_audio_device(48_000, 2);
        self.audio_log.push(format!("default_output:{}", handle.0));
        Ok(handle)
    }

    fn audio_output_close(&mut self, device: RuntimeAudioDeviceHandle) -> Result<(), String> {
        self.audio_device_mut(device)?.closed = true;
        self.audio_log.push(format!("output_close:{}", device.0));
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
        let resolved = self.resolve_fs_path(path)?;
        if !resolved.exists() {
            return Err(format!(
                "audio buffer path `{}` does not exist",
                runtime_path_string(&resolved)
            ));
        }
        let runtime_path = runtime_path_string(&resolved);
        let handle = self.insert_audio_buffer(&runtime_path, 64, 2, 48_000);
        self.audio_log
            .push(format!("buffer_load_wav:{runtime_path}"));
        Ok(handle)
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
        if self.audio_device_ref(device)?.closed {
            return Err(format!("AudioDevice `{}` is closed", device.0));
        }
        let buffer_path = self.audio_buffer_ref(buffer)?.path.clone();
        let handle = self.insert_audio_playback(device, buffer)?;
        self.audio_log.push(format!(
            "play_buffer:{},{},{}",
            device.0, handle.0, buffer_path
        ));
        Ok(handle)
    }

    fn audio_output_set_gain_milli(
        &mut self,
        device: RuntimeAudioDeviceHandle,
        milli: i64,
    ) -> Result<(), String> {
        self.audio_device_mut(device)?.gain_milli = milli;
        self.audio_log
            .push(format!("output_set_gain_milli:{},{}", device.0, milli));
        Ok(())
    }

    fn audio_playback_stop(&mut self, playback: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        let frames = {
            let playback_state = self.audio_playback_ref(playback)?;
            self.audio_buffer_ref(playback_state.buffer)?.frames
        };
        let playback_state = self.audio_playback_mut(playback)?;
        playback_state.finished = true;
        playback_state.paused = false;
        playback_state.position_frames = frames;
        self.audio_log.push(format!("playback_stop:{}", playback.0));
        Ok(())
    }

    fn audio_playback_pause(&mut self, playback: RuntimeAudioPlaybackHandle) -> Result<(), String> {
        self.audio_playback_mut(playback)?.paused = true;
        self.audio_log
            .push(format!("playback_pause:{}", playback.0));
        Ok(())
    }

    fn audio_playback_resume(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<(), String> {
        self.audio_playback_mut(playback)?.paused = false;
        self.audio_log
            .push(format!("playback_resume:{}", playback.0));
        Ok(())
    }

    fn audio_playback_playing(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        let playback = self.audio_playback_ref(playback)?;
        Ok(!playback.paused && !playback.finished)
    }

    fn audio_playback_paused(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        Ok(self.audio_playback_ref(playback)?.paused)
    }

    fn audio_playback_finished(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
    ) -> Result<bool, String> {
        Ok(self.audio_playback_ref(playback)?.finished)
    }

    fn audio_playback_set_gain_milli(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
        milli: i64,
    ) -> Result<(), String> {
        self.audio_playback_mut(playback)?.gain_milli = milli;
        self.audio_log
            .push(format!("playback_set_gain_milli:{},{}", playback.0, milli));
        Ok(())
    }

    fn audio_playback_set_looping(
        &mut self,
        playback: RuntimeAudioPlaybackHandle,
        looping: bool,
    ) -> Result<(), String> {
        self.audio_playback_mut(playback)?.looping = looping;
        self.audio_log
            .push(format!("playback_set_looping:{},{}", playback.0, looping));
        Ok(())
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
        Ok(self.audio_playback_ref(playback)?.position_frames)
    }

    fn monotonic_now_ms(&mut self) -> Result<i64, String> {
        let now = self.monotonic_now_ms;
        self.monotonic_now_ms += self.monotonic_step_ms;
        Ok(now)
    }

    fn monotonic_now_ns(&mut self) -> Result<i64, String> {
        let now = self.monotonic_now_ns;
        self.monotonic_now_ns += self.monotonic_step_ns;
        Ok(now)
    }

    fn sleep_ms(&mut self, ms: i64) -> Result<(), String> {
        self.sleep_log_ms.push(ms);
        Ok(())
    }

    fn process_exec_status(&mut self, program: &str, args: &[String]) -> Result<i64, String> {
        if !self.allow_process {
            return Err("process execution is disabled on this runtime host".to_string());
        }
        let status = std::process::Command::new(program)
            .args(args)
            .status()
            .map_err(|err| format!("failed to run process `{program}`: {err}"))?;
        Ok(i64::from(status.code().unwrap_or(-1)))
    }

    fn process_exec_capture(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<(i64, Vec<u8>, Vec<u8>, bool, bool), String> {
        if !self.allow_process {
            return Err("process execution is disabled on this runtime host".to_string());
        }
        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .map_err(|err| format!("failed to run process `{program}`: {err}"))?;
        let status = i64::from(output.status.code().unwrap_or(-1));
        let stdout_utf8 = std::str::from_utf8(&output.stdout).is_ok();
        let stderr_utf8 = std::str::from_utf8(&output.stderr).is_ok();
        Ok((
            status,
            output.stdout,
            output.stderr,
            stdout_utf8,
            stderr_utf8,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeValue {
    Int(i64),
    Bool(bool),
    Str(String),
    Pair(Box<RuntimeValue>, Box<RuntimeValue>),
    Array(Vec<RuntimeValue>),
    List(Vec<RuntimeValue>),
    Opaque(RuntimeOpaqueValue),
    Record {
        name: String,
        fields: BTreeMap<String, RuntimeValue>,
    },
    Variant {
        name: String,
        payload: Vec<RuntimeValue>,
    },
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedExpr {
    Int(i64),
    Bool(bool),
    Str(String),
    Path(Vec<String>),
    Pair {
        left: Box<ParsedExpr>,
        right: Box<ParsedExpr>,
    },
    Collection {
        items: Vec<ParsedExpr>,
    },
    Match {
        subject: Box<ParsedExpr>,
        arms: Vec<ParsedMatchArm>,
    },
    Member {
        expr: Box<ParsedExpr>,
        member: String,
    },
    Generic {
        expr: Box<ParsedExpr>,
        type_args: Vec<String>,
    },
    Phrase {
        subject: Box<ParsedExpr>,
        args: Vec<ParsedPhraseArg>,
        qualifier: String,
        attached: Vec<ParsedHeaderAttachment>,
    },
    Unary {
        op: ParsedUnaryOp,
        expr: Box<ParsedExpr>,
    },
    Binary {
        left: Box<ParsedExpr>,
        op: ParsedBinaryOp,
        right: Box<ParsedExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedMatchArm {
    patterns: Vec<ParsedMatchPattern>,
    value: ParsedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedPhraseArg {
    name: Option<String>,
    value: ParsedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeCallArg {
    name: Option<String>,
    value: RuntimeValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedHeaderAttachment {
    Named { name: String, value: ParsedExpr },
    Chain { expr: ParsedExpr },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedMatchPattern {
    Wildcard,
    Name(String),
    Literal(String),
    Variant {
        path: String,
        args: Vec<ParsedMatchPattern>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedStmt {
    Let {
        mutable: bool,
        name: String,
        value: ParsedExpr,
    },
    Expr(ParsedExpr),
    Return(Option<ParsedExpr>),
    If {
        condition: ParsedExpr,
        then_branch: Vec<ParsedStmt>,
        else_branch: Vec<ParsedStmt>,
    },
    While {
        condition: ParsedExpr,
        body: Vec<ParsedStmt>,
    },
    For {
        binding: String,
        iterable: ParsedExpr,
        body: Vec<ParsedStmt>,
    },
    Defer(ParsedExpr),
    Break,
    Continue,
    Assign {
        target: ParsedAssignTarget,
        op: ParsedAssignOp,
        value: ParsedExpr,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParsedUnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParsedBinaryOp {
    Or,
    And,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    BitOr,
    BitXor,
    BitAnd,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ParsedAssignTarget {
    Name(String),
    Member {
        target: Box<ParsedAssignTarget>,
        member: String,
    },
    Index {
        target: Box<ParsedAssignTarget>,
        index: ParsedExpr,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParsedAssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeLocal {
    mutable: bool,
    value: RuntimeValue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeScope {
    locals: BTreeMap<String, RuntimeLocal>,
    deferred: Vec<ParsedExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FlowSignal {
    Next,
    Return(RuntimeValue),
    Break,
    Continue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeExecutionState {
    next_entity_id: i64,
    live_entities: BTreeSet<i64>,
    component_slots: BTreeMap<Vec<String>, BTreeMap<i64, RuntimeValue>>,
}

type RuntimeTypeBindings = BTreeMap<String, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeIntrinsic {
    IoPrint,
    IoPrintLine,
    IoEprint,
    IoEprintLine,
    IoFlushStdout,
    IoFlushStderr,
    IoStdinReadLineTry,
    ArgCount,
    ArgGet,
    EnvHas,
    EnvGet,
    PathCwd,
    PathJoin,
    PathNormalize,
    PathParent,
    PathFileName,
    PathExt,
    PathIsAbsolute,
    PathStemTry,
    PathWithExt,
    PathRelativeToTry,
    PathCanonicalizeTry,
    PathStripPrefixTry,
    FsExists,
    FsIsFile,
    FsIsDir,
    FsReadTextTry,
    FsReadBytesTry,
    FsStreamOpenReadTry,
    FsStreamOpenWriteTry,
    FsStreamReadTry,
    FsStreamWriteTry,
    FsStreamEofTry,
    FsStreamCloseTry,
    FsWriteTextTry,
    FsWriteBytesTry,
    FsListDirTry,
    FsMkdirAllTry,
    FsCreateDirTry,
    FsRemoveFileTry,
    FsRemoveDirTry,
    FsRemoveDirAllTry,
    FsCopyFileTry,
    FsRenameTry,
    FsFileSizeTry,
    FsModifiedUnixMsTry,
    WindowOpenTry,
    CanvasAlive,
    CanvasFill,
    CanvasRect,
    CanvasLine,
    CanvasCircleFill,
    CanvasLabel,
    CanvasLabelSize,
    CanvasPresent,
    CanvasRgb,
    ImageLoadTry,
    CanvasImageSize,
    CanvasBlit,
    CanvasBlitScaled,
    CanvasBlitRegion,
    WindowSize,
    WindowResized,
    WindowFullscreen,
    WindowMinimized,
    WindowMaximized,
    WindowFocused,
    WindowSetTitle,
    WindowSetResizable,
    WindowSetFullscreen,
    WindowSetMinimized,
    WindowSetMaximized,
    WindowSetTopmost,
    WindowSetCursorVisible,
    WindowClose,
    EventsPump,
    EventsPoll,
    InputKeyCode,
    InputKeyDown,
    InputKeyPressed,
    InputKeyReleased,
    InputMouseButtonCode,
    InputMousePos,
    InputMouseDown,
    InputMousePressed,
    InputMouseReleased,
    InputMouseWheelY,
    InputMouseInWindow,
    TimeMonotonicNowMs,
    TimeMonotonicNowNs,
    ConcurrentSleep,
    ConcurrentBehaviorStep,
    AudioDefaultOutputTry,
    AudioOutputClose,
    AudioOutputSampleRateHz,
    AudioOutputChannels,
    AudioBufferLoadWavTry,
    AudioBufferFrames,
    AudioBufferChannels,
    AudioBufferSampleRateHz,
    AudioPlayBufferTry,
    AudioOutputSetGainMilli,
    AudioPlaybackStop,
    AudioPlaybackPause,
    AudioPlaybackResume,
    AudioPlaybackPlaying,
    AudioPlaybackPaused,
    AudioPlaybackFinished,
    AudioPlaybackSetGainMilli,
    AudioPlaybackSetLooping,
    AudioPlaybackLooping,
    AudioPlaybackPositionFrames,
    ProcessExecStatusTry,
    ProcessExecCaptureTry,
    TextLenBytes,
    TextByteAt,
    TextSliceBytes,
    TextStartsWith,
    TextEndsWith,
    TextSplitLines,
    TextFromInt,
    BytesFromStrUtf8,
    BytesToStrUtf8,
    BytesLen,
    BytesAt,
    BytesSlice,
    BytesSha256Hex,
    ResultOk,
    ResultErr,
    ListNew,
    ArrayNew,
    ArrayLen,
    ArrayFromList,
    ArrayToList,
    EcsSetSingleton,
    EcsHasSingleton,
    EcsGetSingleton,
    EcsSpawn,
    EcsDespawn,
    EcsSetComponentAt,
    EcsHasComponentAt,
    EcsGetComponentAt,
    EcsRemoveComponentAt,
}

fn lower_routine(routine: &AotRoutineArtifact) -> Result<RuntimeRoutinePlan, String> {
    let params = routine
        .param_rows
        .iter()
        .map(|row| parse_param_row(row))
        .collect::<Result<Vec<_>, String>>()?;
    let type_params = routine
        .type_param_rows
        .iter()
        .map(|row| parse_type_param_row(row))
        .collect::<Result<Vec<_>, String>>()?;
    let behavior_attrs = routine
        .behavior_attr_rows
        .iter()
        .map(|row| parse_behavior_attr_row(row))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let statements = routine
        .statement_rows
        .iter()
        .map(|row| parse_stmt(row))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RuntimeRoutinePlan {
        module_id: routine.module_id.clone(),
        symbol_name: routine.symbol_name.clone(),
        symbol_kind: routine.symbol_kind.clone(),
        exported: routine.exported,
        is_async: routine.is_async,
        type_params,
        behavior_attrs,
        params,
        signature_row: routine.signature_row.clone(),
        intrinsic_impl: routine.intrinsic_impl.clone(),
        foreword_rows: routine.foreword_rows.clone(),
        rollup_rows: routine.rollup_rows.clone(),
        statements,
    })
}

fn lower_entrypoint(
    entrypoint: &AotEntrypointArtifact,
    routines: &[RuntimeRoutinePlan],
) -> Result<RuntimeEntrypointPlan, String> {
    let routine_index = routines
        .iter()
        .position(|routine| {
            routine.module_id == entrypoint.module_id
                && routine.symbol_name == entrypoint.symbol_name
        })
        .ok_or_else(|| {
            format!(
                "entrypoint `{}` in module `{}` has no lowered runtime routine",
                entrypoint.symbol_name, entrypoint.module_id
            )
        })?;
    Ok(RuntimeEntrypointPlan {
        module_id: entrypoint.module_id.clone(),
        symbol_name: entrypoint.symbol_name.clone(),
        symbol_kind: entrypoint.symbol_kind.clone(),
        is_async: entrypoint.is_async,
        exported: entrypoint.exported,
        routine_index,
    })
}

fn parse_module_directive_row(
    row: &str,
) -> Result<(String, String, Vec<String>, Option<String>), String> {
    let payload = row
        .strip_prefix("module=")
        .ok_or_else(|| format!("malformed module directive row `{row}`"))?;
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(format!("malformed module directive row `{row}`"));
    }
    Ok((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].split('.').map(ToString::to_string).collect(),
        if parts[3].is_empty() {
            None
        } else {
            Some(parts[3].to_string())
        },
    ))
}

fn build_module_aliases(
    artifact: &AotPackageArtifact,
) -> Result<BTreeMap<String, BTreeMap<String, Vec<String>>>, String> {
    let mut aliases = BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
    for module in &artifact.modules {
        let module_aliases = aliases.entry(module.module_id.clone()).or_default();
        for row in &module.directive_rows {
            let (module_id, kind, path, alias) = parse_module_directive_row(row)?;
            if module_id != module.module_id {
                return Err(format!(
                    "module directive row `{row}` does not match containing module `{}`",
                    module.module_id
                ));
            }
            if kind != "import" && kind != "use" {
                continue;
            }
            let local_name = alias.unwrap_or_else(|| path.last().cloned().unwrap_or_default());
            if !local_name.is_empty() {
                module_aliases.insert(local_name, path);
            }
        }
    }
    Ok(aliases)
}

pub fn plan_from_artifact(artifact: &AotPackageArtifact) -> Result<RuntimePackagePlan, String> {
    let routines = artifact
        .routines
        .iter()
        .map(lower_routine)
        .collect::<Result<Vec<_>, String>>()?;
    let entrypoints = artifact
        .entrypoints
        .iter()
        .map(|entrypoint| lower_entrypoint(entrypoint, &routines))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RuntimePackagePlan {
        package_name: artifact.package_name.clone(),
        root_module_id: artifact.root_module_id.clone(),
        direct_deps: artifact.direct_deps.clone(),
        runtime_requirements: artifact.runtime_requirements.clone(),
        module_aliases: build_module_aliases(artifact)?,
        entrypoints,
        routines,
    })
}

pub fn load_package_plan(path: &Path) -> Result<RuntimePackagePlan, String> {
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read backend artifact `{}`: {err}",
            path.display()
        )
    })?;
    let artifact = parse_package_artifact(&text)?;
    plan_from_artifact(&artifact)
}

fn strip_prefix_suffix<'a>(text: &'a str, prefix: &str, suffix: &str) -> Result<&'a str, String> {
    let inner = text
        .strip_prefix(prefix)
        .and_then(|value| value.strip_suffix(suffix))
        .ok_or_else(|| format!("malformed runtime row `{text}`"))?;
    Ok(inner)
}

fn split_top_level(text: &str) -> Vec<String> {
    split_top_level_with_delim(text, ',')
}

fn split_top_level_with_delim(text: &str, delimiter: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in text.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            ch if ch == delimiter && paren_depth == 0 && bracket_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn parse_named_fields(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut fields = BTreeMap::new();
    for part in split_top_level(text) {
        let Some((name, value)) = part.split_once('=') else {
            return Err(format!("expected named field in `{part}`"));
        };
        fields.insert(name.trim().to_string(), value.trim().to_string());
    }
    Ok(fields)
}

fn parse_list(text: &str) -> Result<Vec<String>, String> {
    let inner = strip_prefix_suffix(text, "[", "]")?;
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(split_top_level(inner))
}

fn parse_param_row(text: &str) -> Result<RuntimeParamPlan, String> {
    let parts = text.splitn(3, ':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!("malformed runtime param row `{text}`"));
    }
    let mode = parts[0]
        .strip_prefix("mode=")
        .ok_or_else(|| format!("runtime param row missing mode in `{text}`"))?;
    let name = parts[1]
        .strip_prefix("name=")
        .ok_or_else(|| format!("runtime param row missing name in `{text}`"))?;
    let ty = parts[2]
        .strip_prefix("ty=")
        .ok_or_else(|| format!("runtime param row missing ty in `{text}`"))?;
    Ok(RuntimeParamPlan {
        mode: if mode.is_empty() {
            None
        } else {
            Some(mode.to_string())
        },
        name: name.to_string(),
        ty: ty.to_string(),
    })
}

fn parse_type_param_row(text: &str) -> Result<String, String> {
    text.strip_prefix("name=")
        .map(ToString::to_string)
        .ok_or_else(|| format!("runtime type param row missing name in `{text}`"))
}

fn parse_behavior_attr_row(text: &str) -> Result<(String, String), String> {
    let parts = text.split(':').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(format!("malformed runtime behavior attr row `{text}`"));
    }
    let name = parts[0]
        .strip_prefix("name=")
        .ok_or_else(|| format!("runtime behavior attr row missing name in `{text}`"))?;
    let value = parts[1]
        .strip_prefix("value=")
        .ok_or_else(|| format!("runtime behavior attr row missing value in `{text}`"))?;
    Ok((name.to_string(), value.to_string()))
}

fn parse_match_pattern(text: &str) -> Result<ParsedMatchPattern, String> {
    if text == "_" {
        return Ok(ParsedMatchPattern::Wildcard);
    }
    if text.starts_with("name(") && text.ends_with(')') {
        let name = strip_prefix_suffix(text, "name(", ")")?;
        if name.contains('.') {
            return Ok(ParsedMatchPattern::Variant {
                path: name.to_string(),
                args: Vec::new(),
            });
        }
        return Ok(ParsedMatchPattern::Name(name.to_string()));
    }
    if text.starts_with("lit(\"") && text.ends_with("\")") {
        return Ok(ParsedMatchPattern::Literal(decode_row_string(&format!(
            "\"{}\"",
            strip_prefix_suffix(text, "lit(\"", "\")")?
        ))?));
    }
    if text.starts_with("variant(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "variant(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed runtime match variant `{text}`"));
        }
        let args = parse_list(&parts[1])?
            .into_iter()
            .map(|item| parse_match_pattern(&item))
            .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedMatchPattern::Variant {
            path: parts[0].to_string(),
            args,
        });
    }
    Err(format!("unsupported runtime match pattern `{text}`"))
}

fn parse_match_arm(text: &str) -> Result<ParsedMatchArm, String> {
    let fields = parse_named_fields(strip_prefix_suffix(text, "arm(", ")")?)?;
    let patterns_src = fields
        .get("patterns")
        .ok_or_else(|| format!("runtime arm missing patterns in `{text}`"))?;
    let patterns_inner = strip_prefix_suffix(patterns_src, "[", "]")?;
    let patterns = if patterns_inner.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level_with_delim(patterns_inner, '|')
            .into_iter()
            .map(|item| parse_match_pattern(&item))
            .collect::<Result<Vec<_>, String>>()?
    };
    let value = parse_expr(
        fields
            .get("value")
            .ok_or_else(|| format!("runtime arm missing value in `{text}`"))?,
    )?;
    Ok(ParsedMatchArm { patterns, value })
}

fn decode_row_string(text: &str) -> Result<String, String> {
    let inner = strip_prefix_suffix(text, "\"", "\"")?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                return Err("unterminated escape in runtime string".to_string());
            };
            match next {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn decode_source_string_literal(text: &str) -> Result<String, String> {
    let source = decode_row_string(text)?;
    if source.starts_with('"') && source.ends_with('"') && source.len() >= 2 {
        decode_row_string(&source)
    } else {
        Ok(source)
    }
}

fn parse_unary_op(text: &str) -> Result<ParsedUnaryOp, String> {
    match text {
        "-" => Ok(ParsedUnaryOp::Neg),
        "not" => Ok(ParsedUnaryOp::Not),
        "~" => Ok(ParsedUnaryOp::BitNot),
        _ => Err(format!("unsupported runtime unary op `{text}`")),
    }
}

fn parse_binary_op(text: &str) -> Result<ParsedBinaryOp, String> {
    match text {
        "or" => Ok(ParsedBinaryOp::Or),
        "and" => Ok(ParsedBinaryOp::And),
        "==" => Ok(ParsedBinaryOp::EqEq),
        "!=" => Ok(ParsedBinaryOp::NotEq),
        "<" => Ok(ParsedBinaryOp::Lt),
        "<=" => Ok(ParsedBinaryOp::LtEq),
        ">" => Ok(ParsedBinaryOp::Gt),
        ">=" => Ok(ParsedBinaryOp::GtEq),
        "|" => Ok(ParsedBinaryOp::BitOr),
        "^" => Ok(ParsedBinaryOp::BitXor),
        "&" => Ok(ParsedBinaryOp::BitAnd),
        "<<" => Ok(ParsedBinaryOp::Shl),
        "shr" => Ok(ParsedBinaryOp::Shr),
        "+" => Ok(ParsedBinaryOp::Add),
        "-" => Ok(ParsedBinaryOp::Sub),
        "*" => Ok(ParsedBinaryOp::Mul),
        "/" => Ok(ParsedBinaryOp::Div),
        "%" => Ok(ParsedBinaryOp::Mod),
        _ => Err(format!("unsupported runtime binary op `{text}`")),
    }
}

fn parse_assign_op(text: &str) -> Result<ParsedAssignOp, String> {
    match text {
        "=" => Ok(ParsedAssignOp::Assign),
        "+=" => Ok(ParsedAssignOp::AddAssign),
        "-=" => Ok(ParsedAssignOp::SubAssign),
        "*=" => Ok(ParsedAssignOp::MulAssign),
        "/=" => Ok(ParsedAssignOp::DivAssign),
        "%=" => Ok(ParsedAssignOp::ModAssign),
        "&=" => Ok(ParsedAssignOp::BitAndAssign),
        "|=" => Ok(ParsedAssignOp::BitOrAssign),
        "^=" => Ok(ParsedAssignOp::BitXorAssign),
        "<<=" => Ok(ParsedAssignOp::ShlAssign),
        "shr=" => Ok(ParsedAssignOp::ShrAssign),
        _ => Err(format!("unsupported runtime assign op `{text}`")),
    }
}

fn parse_expr(text: &str) -> Result<ParsedExpr, String> {
    if let Some(inner) = text
        .strip_prefix("int(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return inner
            .parse::<i64>()
            .map(ParsedExpr::Int)
            .map_err(|err| format!("invalid runtime int `{inner}`: {err}"));
    }
    if let Some(inner) = text
        .strip_prefix("bool(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return match inner {
            "true" => Ok(ParsedExpr::Bool(true)),
            "false" => Ok(ParsedExpr::Bool(false)),
            _ => Err(format!("invalid runtime bool `{inner}`")),
        };
    }
    if text.starts_with("str(") && text.ends_with(')') {
        return Ok(ParsedExpr::Str(decode_source_string_literal(
            strip_prefix_suffix(text, "str(", ")")?,
        )?));
    }
    if text.starts_with("pair(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "pair(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed pair expression `{text}`"));
        }
        return Ok(ParsedExpr::Pair {
            left: Box::new(parse_expr(&parts[0])?),
            right: Box::new(parse_expr(&parts[1])?),
        });
    }
    if text.starts_with("collection(") && text.ends_with(')') {
        let items = parse_list(strip_prefix_suffix(text, "collection(", ")")?)?
            .into_iter()
            .map(|item| parse_expr(&item))
            .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedExpr::Collection { items });
    }
    if text.starts_with("match(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "match(", ")")?)?;
        let arms = parse_list(
            fields
                .get("arms")
                .ok_or_else(|| format!("match expression missing arms in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_match_arm(&item))
        .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedExpr::Match {
            subject: Box::new(parse_expr(fields.get("subject").ok_or_else(|| {
                format!("match expression missing subject in `{text}`")
            })?)?),
            arms,
        });
    }
    if text.starts_with("path(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "path(", ")")?;
        if inner.is_empty() {
            return Err("empty path in runtime row".to_string());
        }
        return Ok(ParsedExpr::Path(
            inner.split('.').map(ToString::to_string).collect(),
        ));
    }
    if text.starts_with("member(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "member(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed member expression `{text}`"));
        }
        return Ok(ParsedExpr::Member {
            expr: Box::new(parse_expr(&parts[0])?),
            member: parts[1].to_string(),
        });
    }
    if text.starts_with("unary(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "unary(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed unary expression `{text}`"));
        }
        return Ok(ParsedExpr::Unary {
            op: parse_unary_op(&parts[0])?,
            expr: Box::new(parse_expr(&parts[1])?),
        });
    }
    if text.starts_with("binary(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "binary(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 3 {
            return Err(format!("malformed binary expression `{text}`"));
        }
        return Ok(ParsedExpr::Binary {
            left: Box::new(parse_expr(&parts[0])?),
            op: parse_binary_op(&parts[1])?,
            right: Box::new(parse_expr(&parts[2])?),
        });
    }
    if text.starts_with("generic(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "generic(", ")")?)?;
        let type_args = parse_list(
            fields
                .get("types")
                .ok_or_else(|| format!("generic expression missing types in `{text}`"))?,
        )?;
        return Ok(ParsedExpr::Generic {
            expr: Box::new(parse_expr(fields.get("expr").ok_or_else(|| {
                format!("generic expression missing expr in `{text}`")
            })?)?),
            type_args,
        });
    }
    if text.starts_with("phrase(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "phrase(", ")")?)?;
        let args = parse_list(
            fields
                .get("args")
                .ok_or_else(|| format!("phrase missing args in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_phrase_arg(&item))
        .collect::<Result<Vec<_>, String>>()?;
        let attached = parse_list(
            fields
                .get("attached")
                .ok_or_else(|| format!("phrase missing attached in `{text}`"))?,
        )?
        .into_iter()
        .map(|item| parse_header_attachment(&item))
        .collect::<Result<Vec<_>, String>>()?;
        return Ok(ParsedExpr::Phrase {
            subject: Box::new(parse_expr(
                fields
                    .get("subject")
                    .ok_or_else(|| format!("phrase missing subject in `{text}`"))?,
            )?),
            args,
            qualifier: fields
                .get("qualifier")
                .ok_or_else(|| format!("phrase missing qualifier in `{text}`"))?
                .to_string(),
            attached,
        });
    }
    Err(format!("unsupported runtime expression `{text}`"))
}

fn parse_phrase_arg(text: &str) -> Result<ParsedPhraseArg, String> {
    if let Some((name, value)) = split_top_level_assignment(text) {
        return Ok(ParsedPhraseArg {
            name: Some(name.to_string()),
            value: parse_expr(value)?,
        });
    }
    Ok(ParsedPhraseArg {
        name: None,
        value: parse_expr(text)?,
    })
}

fn parse_header_attachment(text: &str) -> Result<ParsedHeaderAttachment, String> {
    if text.starts_with("named(") && text.ends_with(')') {
        let fields = parse_named_fields(strip_prefix_suffix(text, "named(", ")")?)?;
        let forewords = parse_list(
            fields
                .get("forewords")
                .ok_or_else(|| format!("named attachment missing forewords in `{text}`"))?,
        )?;
        if !forewords.is_empty() {
            return Err("attached phrase forewords are not implemented yet".to_string());
        }
        let named_fields = fields
            .into_iter()
            .filter(|(name, _)| name != "forewords")
            .collect::<Vec<_>>();
        if named_fields.len() != 1 {
            return Err(format!(
                "named attachment must contain exactly one value in `{text}`"
            ));
        }
        let (name, value) = named_fields
            .into_iter()
            .next()
            .ok_or_else(|| format!("named attachment is empty in `{text}`"))?;
        return Ok(ParsedHeaderAttachment::Named {
            name,
            value: parse_expr(&value)?,
        });
    }
    if text.starts_with("chain(") && text.ends_with(')') {
        let parts = split_top_level(strip_prefix_suffix(text, "chain(", ")")?);
        if parts.len() != 2 {
            return Err(format!("malformed chain attachment `{text}`"));
        }
        let (_, forewords_text) = split_top_level_assignment(&parts[1])
            .ok_or_else(|| format!("chain attachment missing forewords in `{text}`"))?;
        let forewords = parse_list(forewords_text)?;
        if !forewords.is_empty() {
            return Err("attached phrase forewords are not implemented yet".to_string());
        }
        return Ok(ParsedHeaderAttachment::Chain {
            expr: parse_expr(&parts[0])?,
        });
    }
    Err(format!("unsupported runtime attachment `{text}`"))
}

fn split_top_level_assignment(text: &str) -> Option<(&str, &str)> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    for (index, ch) in text.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '=' if depth == 0 => return Some((&text[..index], &text[index + 1..])),
            _ => {}
        }
    }
    None
}

fn parse_assign_target(text: &str) -> Result<ParsedAssignTarget, String> {
    if text.starts_with("name(") && text.ends_with(')') {
        return Ok(ParsedAssignTarget::Name(
            strip_prefix_suffix(text, "name(", ")")?.to_string(),
        ));
    }
    if text.starts_with("member(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "member(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed runtime assign target `{text}`"));
        }
        return Ok(ParsedAssignTarget::Member {
            target: Box::new(parse_assign_target(&parts[0])?),
            member: parts[1].to_string(),
        });
    }
    if text.starts_with("index(") && text.ends_with(')') {
        let inner = strip_prefix_suffix(text, "index(", ")")?;
        let parts = split_top_level(inner);
        if parts.len() != 2 {
            return Err(format!("malformed runtime assign target `{text}`"));
        }
        return Ok(ParsedAssignTarget::Index {
            target: Box::new(parse_assign_target(&parts[0])?),
            index: parse_expr(&parts[1])?,
        });
    }
    Err(format!("unsupported runtime assign target `{text}`"))
}

fn parse_stmt(text: &str) -> Result<ParsedStmt, String> {
    let fields = parse_named_fields(strip_prefix_suffix(text, "stmt(", ")")?)?;
    let forewords = parse_list(
        fields
            .get("forewords")
            .ok_or_else(|| format!("runtime stmt missing forewords in `{text}`"))?,
    )?;
    let rollups = parse_list(
        fields
            .get("rollups")
            .ok_or_else(|| format!("runtime stmt missing rollups in `{text}`"))?,
    )?;
    if !forewords.is_empty() || !rollups.is_empty() {
        return Err("runtime stmt forewords/rollups are not implemented yet".to_string());
    }
    let core = fields
        .get("core")
        .ok_or_else(|| format!("runtime stmt missing core in `{text}`"))?;
    if core.starts_with("let(") && core.ends_with(')') {
        let let_fields = parse_named_fields(strip_prefix_suffix(core, "let(", ")")?)?;
        let mutable = match let_fields
            .get("mutable")
            .map(String::as_str)
            .ok_or_else(|| format!("runtime let missing mutable in `{text}`"))?
        {
            "true" => true,
            "false" => false,
            other => return Err(format!("invalid runtime let mutable `{other}`")),
        };
        return Ok(ParsedStmt::Let {
            mutable,
            name: let_fields
                .get("name")
                .ok_or_else(|| format!("runtime let missing name in `{text}`"))?
                .to_string(),
            value: parse_expr(
                let_fields
                    .get("value")
                    .ok_or_else(|| format!("runtime let missing value in `{text}`"))?,
            )?,
        });
    }
    if core.starts_with("expr(") && core.ends_with(')') {
        return Ok(ParsedStmt::Expr(parse_expr(strip_prefix_suffix(
            core, "expr(", ")",
        )?)?));
    }
    if core.starts_with("return(") && core.ends_with(')') {
        let inner = strip_prefix_suffix(core, "return(", ")")?;
        return Ok(ParsedStmt::Return(if inner == "none" {
            None
        } else {
            Some(parse_expr(inner)?)
        }));
    }
    if core.starts_with("if(") && core.ends_with(')') {
        let if_fields = parse_named_fields(strip_prefix_suffix(core, "if(", ")")?)?;
        return Ok(ParsedStmt::If {
            condition: parse_expr(
                if_fields
                    .get("cond")
                    .ok_or_else(|| format!("runtime if missing cond in `{text}`"))?,
            )?,
            then_branch: parse_list(
                if_fields
                    .get("then")
                    .ok_or_else(|| format!("runtime if missing then in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
            else_branch: parse_list(
                if_fields
                    .get("else")
                    .ok_or_else(|| format!("runtime if missing else in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
        });
    }
    if core.starts_with("while(") && core.ends_with(')') {
        let while_fields = parse_named_fields(strip_prefix_suffix(core, "while(", ")")?)?;
        return Ok(ParsedStmt::While {
            condition: parse_expr(
                while_fields
                    .get("cond")
                    .ok_or_else(|| format!("runtime while missing cond in `{text}`"))?,
            )?,
            body: parse_list(
                while_fields
                    .get("body")
                    .ok_or_else(|| format!("runtime while missing body in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
        });
    }
    if core.starts_with("for(") && core.ends_with(')') {
        let for_fields = parse_named_fields(strip_prefix_suffix(core, "for(", ")")?)?;
        return Ok(ParsedStmt::For {
            binding: for_fields
                .get("binding")
                .ok_or_else(|| format!("runtime for missing binding in `{text}`"))?
                .to_string(),
            iterable: parse_expr(
                for_fields
                    .get("iterable")
                    .ok_or_else(|| format!("runtime for missing iterable in `{text}`"))?,
            )?,
            body: parse_list(
                for_fields
                    .get("body")
                    .ok_or_else(|| format!("runtime for missing body in `{text}`"))?,
            )?
            .into_iter()
            .map(|item| parse_stmt(&item))
            .collect::<Result<Vec<_>, String>>()?,
        });
    }
    if core.starts_with("defer(") && core.ends_with(')') {
        return Ok(ParsedStmt::Defer(parse_expr(strip_prefix_suffix(
            core, "defer(", ")",
        )?)?));
    }
    if core == "break" {
        return Ok(ParsedStmt::Break);
    }
    if core == "continue" {
        return Ok(ParsedStmt::Continue);
    }
    if core.starts_with("assign(") && core.ends_with(')') {
        let assign_fields = parse_named_fields(strip_prefix_suffix(core, "assign(", ")")?)?;
        return Ok(ParsedStmt::Assign {
            target: parse_assign_target(
                assign_fields
                    .get("target")
                    .ok_or_else(|| format!("runtime assign missing target in `{text}`"))?,
            )?,
            op: parse_assign_op(
                assign_fields
                    .get("op")
                    .ok_or_else(|| format!("runtime assign missing op in `{text}`"))?,
            )?,
            value: parse_expr(
                assign_fields
                    .get("value")
                    .ok_or_else(|| format!("runtime assign missing value in `{text}`"))?,
            )?,
        });
    }
    Err(format!("unsupported runtime statement `{core}`"))
}

fn resolve_callable_path(
    expr: &ParsedExpr,
    aliases: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    match expr {
        ParsedExpr::Path(segments) if segments.len() == 1 => aliases
            .get(&segments[0])
            .cloned()
            .or_else(|| Some(segments.clone())),
        ParsedExpr::Path(segments) => Some(segments.clone()),
        ParsedExpr::Member { expr, member } => {
            let mut path = resolve_callable_path(expr, aliases)?;
            path.push(member.clone());
            Some(path)
        }
        ParsedExpr::Generic { expr, .. } => resolve_callable_path(expr, aliases),
        _ => None,
    }
}

fn extract_generic_type_args(expr: &ParsedExpr) -> Vec<String> {
    match expr {
        ParsedExpr::Generic { expr, type_args } => {
            let inner = extract_generic_type_args(expr);
            if inner.is_empty() {
                type_args.clone()
            } else {
                inner
            }
        }
        _ => Vec::new(),
    }
}

fn resolve_runtime_type_args(
    type_args: &[String],
    type_bindings: &RuntimeTypeBindings,
) -> Vec<String> {
    type_args
        .iter()
        .map(|arg| {
            type_bindings
                .get(arg)
                .cloned()
                .unwrap_or_else(|| arg.clone())
        })
        .collect()
}

fn resolve_routine_index(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    callable_path: &[String],
) -> Option<usize> {
    let (module_id, symbol_name) = match callable_path {
        [] => return None,
        [symbol_name] => (current_module_id.to_string(), symbol_name.clone()),
        _ => (
            callable_path[..callable_path.len() - 1].join("."),
            callable_path.last()?.clone(),
        ),
    };
    plan.routines
        .iter()
        .position(|routine| routine.module_id == module_id && routine.symbol_name == symbol_name)
        .or_else(|| {
            let prefixed_module = if module_id == plan.root_module_id
                || module_id.starts_with(&(plan.root_module_id.clone() + "."))
            {
                module_id.clone()
            } else {
                format!("{}.{}", plan.root_module_id, module_id)
            };
            plan.routines.iter().position(|routine| {
                routine.module_id == prefixed_module && routine.symbol_name == symbol_name
            })
        })
}

fn resolve_runtime_intrinsic_path(callable_path: &[String]) -> Option<RuntimeIntrinsic> {
    let parts = callable_path.iter().map(String::as_str).collect::<Vec<_>>();
    match parts.as_slice() {
        ["std", "io", "print"] | ["std", "kernel", "io", "print"] => {
            Some(RuntimeIntrinsic::IoPrint)
        }
        ["std", "io", "print_line"] => Some(RuntimeIntrinsic::IoPrintLine),
        ["std", "io", "eprint"] | ["std", "kernel", "io", "eprint"] => {
            Some(RuntimeIntrinsic::IoEprint)
        }
        ["std", "io", "eprint_line"] => Some(RuntimeIntrinsic::IoEprintLine),
        ["std", "args", "count"] | ["std", "kernel", "args", "arg_count"] => {
            Some(RuntimeIntrinsic::ArgCount)
        }
        ["std", "args", "get"] | ["std", "kernel", "args", "arg_get"] => {
            Some(RuntimeIntrinsic::ArgGet)
        }
        ["std", "env", "has"] | ["std", "kernel", "env", "env_has"] => {
            Some(RuntimeIntrinsic::EnvHas)
        }
        ["std", "env", "get"] | ["std", "kernel", "env", "env_get"] => {
            Some(RuntimeIntrinsic::EnvGet)
        }
        ["std", "io", "flush_stdout"] | ["std", "kernel", "io", "flush_stdout"] => {
            Some(RuntimeIntrinsic::IoFlushStdout)
        }
        ["std", "io", "flush_stderr"] | ["std", "kernel", "io", "flush_stderr"] => {
            Some(RuntimeIntrinsic::IoFlushStderr)
        }
        ["std", "io", "read_line"] | ["std", "kernel", "io", "stdin_read_line"] => {
            Some(RuntimeIntrinsic::IoStdinReadLineTry)
        }
        ["std", "path", "cwd"] | ["std", "kernel", "path", "path_cwd"] => {
            Some(RuntimeIntrinsic::PathCwd)
        }
        ["std", "path", "join"] | ["std", "kernel", "path", "path_join"] => {
            Some(RuntimeIntrinsic::PathJoin)
        }
        ["std", "path", "normalize"] | ["std", "kernel", "path", "path_normalize"] => {
            Some(RuntimeIntrinsic::PathNormalize)
        }
        ["std", "path", "parent"] | ["std", "kernel", "path", "path_parent"] => {
            Some(RuntimeIntrinsic::PathParent)
        }
        ["std", "path", "file_name"] | ["std", "kernel", "path", "path_file_name"] => {
            Some(RuntimeIntrinsic::PathFileName)
        }
        ["std", "path", "ext"] | ["std", "kernel", "path", "path_ext"] => {
            Some(RuntimeIntrinsic::PathExt)
        }
        ["std", "path", "is_absolute"] | ["std", "kernel", "path", "path_is_absolute"] => {
            Some(RuntimeIntrinsic::PathIsAbsolute)
        }
        ["std", "path", "stem"] | ["std", "kernel", "path", "path_stem"] => {
            Some(RuntimeIntrinsic::PathStemTry)
        }
        ["std", "path", "with_ext"] | ["std", "kernel", "path", "path_with_ext"] => {
            Some(RuntimeIntrinsic::PathWithExt)
        }
        ["std", "path", "relative_to"] | ["std", "kernel", "path", "path_relative_to"] => {
            Some(RuntimeIntrinsic::PathRelativeToTry)
        }
        ["std", "path", "canonicalize"] | ["std", "kernel", "path", "path_canonicalize"] => {
            Some(RuntimeIntrinsic::PathCanonicalizeTry)
        }
        ["std", "path", "strip_prefix"] | ["std", "kernel", "path", "path_strip_prefix"] => {
            Some(RuntimeIntrinsic::PathStripPrefixTry)
        }
        ["std", "fs", "exists"] | ["std", "kernel", "fs", "fs_exists"] => {
            Some(RuntimeIntrinsic::FsExists)
        }
        ["std", "fs", "is_file"] | ["std", "kernel", "fs", "fs_is_file"] => {
            Some(RuntimeIntrinsic::FsIsFile)
        }
        ["std", "fs", "is_dir"] | ["std", "kernel", "fs", "fs_is_dir"] => {
            Some(RuntimeIntrinsic::FsIsDir)
        }
        ["std", "fs", "read_text"] | ["std", "kernel", "fs", "fs_read_text"] => {
            Some(RuntimeIntrinsic::FsReadTextTry)
        }
        ["std", "fs", "read_bytes"] | ["std", "kernel", "fs", "fs_read_bytes"] => {
            Some(RuntimeIntrinsic::FsReadBytesTry)
        }
        ["std", "fs", "stream_open_read"] | ["std", "kernel", "fs", "fs_stream_open_read"] => {
            Some(RuntimeIntrinsic::FsStreamOpenReadTry)
        }
        ["std", "fs", "stream_open_write"] | ["std", "kernel", "fs", "fs_stream_open_write"] => {
            Some(RuntimeIntrinsic::FsStreamOpenWriteTry)
        }
        ["std", "fs", "stream_read"] | ["std", "kernel", "fs", "fs_stream_read"] => {
            Some(RuntimeIntrinsic::FsStreamReadTry)
        }
        ["std", "fs", "stream_write"] | ["std", "kernel", "fs", "fs_stream_write"] => {
            Some(RuntimeIntrinsic::FsStreamWriteTry)
        }
        ["std", "fs", "stream_eof"] | ["std", "kernel", "fs", "fs_stream_eof"] => {
            Some(RuntimeIntrinsic::FsStreamEofTry)
        }
        ["std", "fs", "stream_close"] | ["std", "kernel", "fs", "fs_stream_close"] => {
            Some(RuntimeIntrinsic::FsStreamCloseTry)
        }
        ["std", "fs", "write_text"] | ["std", "kernel", "fs", "fs_write_text"] => {
            Some(RuntimeIntrinsic::FsWriteTextTry)
        }
        ["std", "fs", "write_bytes"] | ["std", "kernel", "fs", "fs_write_bytes"] => {
            Some(RuntimeIntrinsic::FsWriteBytesTry)
        }
        ["std", "fs", "list_dir"] | ["std", "kernel", "fs", "fs_list_dir"] => {
            Some(RuntimeIntrinsic::FsListDirTry)
        }
        ["std", "fs", "mkdir_all"] | ["std", "kernel", "fs", "fs_mkdir_all"] => {
            Some(RuntimeIntrinsic::FsMkdirAllTry)
        }
        ["std", "fs", "create_dir"] | ["std", "kernel", "fs", "fs_create_dir"] => {
            Some(RuntimeIntrinsic::FsCreateDirTry)
        }
        ["std", "fs", "remove_file"] | ["std", "kernel", "fs", "fs_remove_file"] => {
            Some(RuntimeIntrinsic::FsRemoveFileTry)
        }
        ["std", "fs", "remove_dir"] | ["std", "kernel", "fs", "fs_remove_dir"] => {
            Some(RuntimeIntrinsic::FsRemoveDirTry)
        }
        ["std", "fs", "remove_dir_all"] | ["std", "kernel", "fs", "fs_remove_dir_all"] => {
            Some(RuntimeIntrinsic::FsRemoveDirAllTry)
        }
        ["std", "fs", "copy_file"] | ["std", "kernel", "fs", "fs_copy_file"] => {
            Some(RuntimeIntrinsic::FsCopyFileTry)
        }
        ["std", "fs", "rename"] | ["std", "kernel", "fs", "fs_rename"] => {
            Some(RuntimeIntrinsic::FsRenameTry)
        }
        ["std", "fs", "file_size"] | ["std", "kernel", "fs", "fs_file_size"] => {
            Some(RuntimeIntrinsic::FsFileSizeTry)
        }
        ["std", "fs", "modified_unix_ms"] | ["std", "kernel", "fs", "fs_modified_unix_ms"] => {
            Some(RuntimeIntrinsic::FsModifiedUnixMsTry)
        }
        ["std", "process", "exec_status"] | ["std", "kernel", "process", "process_exec_status"] => {
            Some(RuntimeIntrinsic::ProcessExecStatusTry)
        }
        ["std", "process", "exec_capture"]
        | ["std", "kernel", "process", "process_exec_capture"] => {
            Some(RuntimeIntrinsic::ProcessExecCaptureTry)
        }
        ["std", "text", "len_bytes"] | ["std", "kernel", "text", "text_len_bytes"] => {
            Some(RuntimeIntrinsic::TextLenBytes)
        }
        ["std", "text", "byte_at"] | ["std", "kernel", "text", "text_byte_at"] => {
            Some(RuntimeIntrinsic::TextByteAt)
        }
        ["std", "text", "slice_bytes"] | ["std", "kernel", "text", "text_slice_bytes"] => {
            Some(RuntimeIntrinsic::TextSliceBytes)
        }
        ["std", "text", "starts_with"] | ["std", "kernel", "text", "text_starts_with"] => {
            Some(RuntimeIntrinsic::TextStartsWith)
        }
        ["std", "text", "ends_with"] | ["std", "kernel", "text", "text_ends_with"] => {
            Some(RuntimeIntrinsic::TextEndsWith)
        }
        ["std", "text", "split_lines"] | ["std", "kernel", "text", "text_split_lines"] => {
            Some(RuntimeIntrinsic::TextSplitLines)
        }
        ["std", "text", "from_int"] | ["std", "kernel", "text", "text_from_int"] => {
            Some(RuntimeIntrinsic::TextFromInt)
        }
        ["std", "bytes", "from_str_utf8"] | ["std", "kernel", "text", "bytes_from_str_utf8"] => {
            Some(RuntimeIntrinsic::BytesFromStrUtf8)
        }
        ["std", "bytes", "to_str_utf8"] | ["std", "kernel", "text", "bytes_to_str_utf8"] => {
            Some(RuntimeIntrinsic::BytesToStrUtf8)
        }
        ["std", "bytes", "len"] | ["std", "kernel", "text", "bytes_len"] => {
            Some(RuntimeIntrinsic::BytesLen)
        }
        ["std", "bytes", "at"] | ["std", "kernel", "text", "bytes_at"] => {
            Some(RuntimeIntrinsic::BytesAt)
        }
        ["std", "bytes", "slice"] | ["std", "kernel", "text", "bytes_slice"] => {
            Some(RuntimeIntrinsic::BytesSlice)
        }
        ["std", "bytes", "sha256_hex"] | ["std", "kernel", "text", "bytes_sha256_hex"] => {
            Some(RuntimeIntrinsic::BytesSha256Hex)
        }
        ["std", "collections", "list", "new"] | ["std", "kernel", "collections", "list_new"] => {
            Some(RuntimeIntrinsic::ListNew)
        }
        ["Result", "Ok"] | ["std", "result", "Result", "Ok"] => Some(RuntimeIntrinsic::ResultOk),
        ["Result", "Err"] | ["std", "result", "Result", "Err"] => Some(RuntimeIntrinsic::ResultErr),
        _ => None,
    }
}

fn resolve_runtime_intrinsic_impl(intrinsic_impl: &str) -> Option<RuntimeIntrinsic> {
    match intrinsic_impl {
        "IoPrint" => Some(RuntimeIntrinsic::IoPrint),
        "IoEprint" => Some(RuntimeIntrinsic::IoEprint),
        "IoFlushStdout" => Some(RuntimeIntrinsic::IoFlushStdout),
        "IoFlushStderr" => Some(RuntimeIntrinsic::IoFlushStderr),
        "IoStdinReadLineTry" => Some(RuntimeIntrinsic::IoStdinReadLineTry),
        "HostArgCount" => Some(RuntimeIntrinsic::ArgCount),
        "HostArgGet" => Some(RuntimeIntrinsic::ArgGet),
        "HostEnvHas" => Some(RuntimeIntrinsic::EnvHas),
        "HostEnvGet" => Some(RuntimeIntrinsic::EnvGet),
        "HostPathCwd" => Some(RuntimeIntrinsic::PathCwd),
        "HostPathJoin" => Some(RuntimeIntrinsic::PathJoin),
        "HostPathNormalize" => Some(RuntimeIntrinsic::PathNormalize),
        "HostPathParent" => Some(RuntimeIntrinsic::PathParent),
        "HostPathFileName" => Some(RuntimeIntrinsic::PathFileName),
        "HostPathExt" => Some(RuntimeIntrinsic::PathExt),
        "HostPathIsAbsolute" => Some(RuntimeIntrinsic::PathIsAbsolute),
        "HostPathStemTry" => Some(RuntimeIntrinsic::PathStemTry),
        "HostPathWithExt" => Some(RuntimeIntrinsic::PathWithExt),
        "HostPathRelativeToTry" => Some(RuntimeIntrinsic::PathRelativeToTry),
        "HostPathCanonicalizeTry" => Some(RuntimeIntrinsic::PathCanonicalizeTry),
        "HostPathStripPrefixTry" => Some(RuntimeIntrinsic::PathStripPrefixTry),
        "HostFsExists" => Some(RuntimeIntrinsic::FsExists),
        "HostFsIsFile" => Some(RuntimeIntrinsic::FsIsFile),
        "HostFsIsDir" => Some(RuntimeIntrinsic::FsIsDir),
        "HostFsReadTextTry" => Some(RuntimeIntrinsic::FsReadTextTry),
        "HostFsReadBytesTry" => Some(RuntimeIntrinsic::FsReadBytesTry),
        "HostFsStreamOpenReadTry" => Some(RuntimeIntrinsic::FsStreamOpenReadTry),
        "HostFsStreamOpenWriteTry" => Some(RuntimeIntrinsic::FsStreamOpenWriteTry),
        "HostFsStreamReadTry" => Some(RuntimeIntrinsic::FsStreamReadTry),
        "HostFsStreamWriteTry" => Some(RuntimeIntrinsic::FsStreamWriteTry),
        "HostFsStreamEofTry" => Some(RuntimeIntrinsic::FsStreamEofTry),
        "HostFsStreamCloseTry" => Some(RuntimeIntrinsic::FsStreamCloseTry),
        "HostFsWriteTextTry" => Some(RuntimeIntrinsic::FsWriteTextTry),
        "HostFsWriteBytesTry" => Some(RuntimeIntrinsic::FsWriteBytesTry),
        "HostFsListDirTry" => Some(RuntimeIntrinsic::FsListDirTry),
        "HostFsMkdirAllTry" => Some(RuntimeIntrinsic::FsMkdirAllTry),
        "HostFsCreateDirTry" => Some(RuntimeIntrinsic::FsCreateDirTry),
        "HostFsRemoveFileTry" => Some(RuntimeIntrinsic::FsRemoveFileTry),
        "HostFsRemoveDirTry" => Some(RuntimeIntrinsic::FsRemoveDirTry),
        "HostFsRemoveDirAllTry" => Some(RuntimeIntrinsic::FsRemoveDirAllTry),
        "HostFsCopyFileTry" => Some(RuntimeIntrinsic::FsCopyFileTry),
        "HostFsRenameTry" => Some(RuntimeIntrinsic::FsRenameTry),
        "HostFsFileSizeTry" => Some(RuntimeIntrinsic::FsFileSizeTry),
        "HostFsModifiedUnixMsTry" => Some(RuntimeIntrinsic::FsModifiedUnixMsTry),
        "WindowOpenTry" => Some(RuntimeIntrinsic::WindowOpenTry),
        "CanvasAlive" => Some(RuntimeIntrinsic::CanvasAlive),
        "CanvasFill" => Some(RuntimeIntrinsic::CanvasFill),
        "CanvasRect" => Some(RuntimeIntrinsic::CanvasRect),
        "CanvasLine" => Some(RuntimeIntrinsic::CanvasLine),
        "CanvasCircleFill" => Some(RuntimeIntrinsic::CanvasCircleFill),
        "CanvasLabel" => Some(RuntimeIntrinsic::CanvasLabel),
        "CanvasLabelSize" => Some(RuntimeIntrinsic::CanvasLabelSize),
        "CanvasPresent" => Some(RuntimeIntrinsic::CanvasPresent),
        "CanvasRgb" => Some(RuntimeIntrinsic::CanvasRgb),
        "ImageLoadTry" => Some(RuntimeIntrinsic::ImageLoadTry),
        "CanvasImageSize" => Some(RuntimeIntrinsic::CanvasImageSize),
        "CanvasBlit" => Some(RuntimeIntrinsic::CanvasBlit),
        "CanvasBlitScaled" => Some(RuntimeIntrinsic::CanvasBlitScaled),
        "CanvasBlitRegion" => Some(RuntimeIntrinsic::CanvasBlitRegion),
        "WindowSize" => Some(RuntimeIntrinsic::WindowSize),
        "WindowResized" => Some(RuntimeIntrinsic::WindowResized),
        "WindowFullscreen" => Some(RuntimeIntrinsic::WindowFullscreen),
        "WindowMinimized" => Some(RuntimeIntrinsic::WindowMinimized),
        "WindowMaximized" => Some(RuntimeIntrinsic::WindowMaximized),
        "WindowFocused" => Some(RuntimeIntrinsic::WindowFocused),
        "WindowSetTitle" => Some(RuntimeIntrinsic::WindowSetTitle),
        "WindowSetResizable" => Some(RuntimeIntrinsic::WindowSetResizable),
        "WindowSetFullscreen" => Some(RuntimeIntrinsic::WindowSetFullscreen),
        "WindowSetMinimized" => Some(RuntimeIntrinsic::WindowSetMinimized),
        "WindowSetMaximized" => Some(RuntimeIntrinsic::WindowSetMaximized),
        "WindowSetTopmost" => Some(RuntimeIntrinsic::WindowSetTopmost),
        "WindowSetCursorVisible" => Some(RuntimeIntrinsic::WindowSetCursorVisible),
        "WindowClose" => Some(RuntimeIntrinsic::WindowClose),
        "EventsPump" => Some(RuntimeIntrinsic::EventsPump),
        "EventsPoll" => Some(RuntimeIntrinsic::EventsPoll),
        "InputKeyCode" => Some(RuntimeIntrinsic::InputKeyCode),
        "InputKeyDown" => Some(RuntimeIntrinsic::InputKeyDown),
        "InputKeyPressed" => Some(RuntimeIntrinsic::InputKeyPressed),
        "InputKeyReleased" => Some(RuntimeIntrinsic::InputKeyReleased),
        "InputMouseButtonCode" => Some(RuntimeIntrinsic::InputMouseButtonCode),
        "InputMousePos" => Some(RuntimeIntrinsic::InputMousePos),
        "InputMouseDown" => Some(RuntimeIntrinsic::InputMouseDown),
        "InputMousePressed" => Some(RuntimeIntrinsic::InputMousePressed),
        "InputMouseReleased" => Some(RuntimeIntrinsic::InputMouseReleased),
        "InputMouseWheelY" => Some(RuntimeIntrinsic::InputMouseWheelY),
        "InputMouseInWindow" => Some(RuntimeIntrinsic::InputMouseInWindow),
        "HostTimeMonotonicNowMs" => Some(RuntimeIntrinsic::TimeMonotonicNowMs),
        "HostTimeMonotonicNowNs" => Some(RuntimeIntrinsic::TimeMonotonicNowNs),
        "ConcurrentSleep" => Some(RuntimeIntrinsic::ConcurrentSleep),
        "ConcurrentBehaviorStep" => Some(RuntimeIntrinsic::ConcurrentBehaviorStep),
        "AudioDefaultOutputTry" => Some(RuntimeIntrinsic::AudioDefaultOutputTry),
        "AudioOutputClose" => Some(RuntimeIntrinsic::AudioOutputClose),
        "AudioOutputSampleRateHz" => Some(RuntimeIntrinsic::AudioOutputSampleRateHz),
        "AudioOutputChannels" => Some(RuntimeIntrinsic::AudioOutputChannels),
        "AudioBufferLoadWavTry" => Some(RuntimeIntrinsic::AudioBufferLoadWavTry),
        "AudioBufferFrames" => Some(RuntimeIntrinsic::AudioBufferFrames),
        "AudioBufferChannels" => Some(RuntimeIntrinsic::AudioBufferChannels),
        "AudioBufferSampleRateHz" => Some(RuntimeIntrinsic::AudioBufferSampleRateHz),
        "AudioPlayBufferTry" => Some(RuntimeIntrinsic::AudioPlayBufferTry),
        "AudioOutputSetGainMilli" => Some(RuntimeIntrinsic::AudioOutputSetGainMilli),
        "AudioPlaybackStop" => Some(RuntimeIntrinsic::AudioPlaybackStop),
        "AudioPlaybackPause" => Some(RuntimeIntrinsic::AudioPlaybackPause),
        "AudioPlaybackResume" => Some(RuntimeIntrinsic::AudioPlaybackResume),
        "AudioPlaybackPlaying" => Some(RuntimeIntrinsic::AudioPlaybackPlaying),
        "AudioPlaybackPaused" => Some(RuntimeIntrinsic::AudioPlaybackPaused),
        "AudioPlaybackFinished" => Some(RuntimeIntrinsic::AudioPlaybackFinished),
        "AudioPlaybackSetGainMilli" => Some(RuntimeIntrinsic::AudioPlaybackSetGainMilli),
        "AudioPlaybackSetLooping" => Some(RuntimeIntrinsic::AudioPlaybackSetLooping),
        "AudioPlaybackLooping" => Some(RuntimeIntrinsic::AudioPlaybackLooping),
        "AudioPlaybackPositionFrames" => Some(RuntimeIntrinsic::AudioPlaybackPositionFrames),
        "HostProcessExecStatusTry" => Some(RuntimeIntrinsic::ProcessExecStatusTry),
        "HostProcessExecCaptureTry" => Some(RuntimeIntrinsic::ProcessExecCaptureTry),
        "HostTextLenBytes" => Some(RuntimeIntrinsic::TextLenBytes),
        "HostTextByteAt" => Some(RuntimeIntrinsic::TextByteAt),
        "HostTextSliceBytes" => Some(RuntimeIntrinsic::TextSliceBytes),
        "HostTextStartsWith" => Some(RuntimeIntrinsic::TextStartsWith),
        "HostTextEndsWith" => Some(RuntimeIntrinsic::TextEndsWith),
        "HostTextSplitLines" => Some(RuntimeIntrinsic::TextSplitLines),
        "HostTextFromInt" => Some(RuntimeIntrinsic::TextFromInt),
        "HostBytesFromStrUtf8" => Some(RuntimeIntrinsic::BytesFromStrUtf8),
        "HostBytesToStrUtf8" => Some(RuntimeIntrinsic::BytesToStrUtf8),
        "HostBytesLen" => Some(RuntimeIntrinsic::BytesLen),
        "HostBytesAt" => Some(RuntimeIntrinsic::BytesAt),
        "HostBytesSlice" => Some(RuntimeIntrinsic::BytesSlice),
        "HostBytesSha256Hex" => Some(RuntimeIntrinsic::BytesSha256Hex),
        "ListNew" => Some(RuntimeIntrinsic::ListNew),
        "ArrayNew" => Some(RuntimeIntrinsic::ArrayNew),
        "ArrayLen" => Some(RuntimeIntrinsic::ArrayLen),
        "ArrayFromList" => Some(RuntimeIntrinsic::ArrayFromList),
        "ArrayToList" => Some(RuntimeIntrinsic::ArrayToList),
        "EcsSetSingleton" => Some(RuntimeIntrinsic::EcsSetSingleton),
        "EcsHasSingleton" => Some(RuntimeIntrinsic::EcsHasSingleton),
        "EcsGetSingleton" => Some(RuntimeIntrinsic::EcsGetSingleton),
        "EcsSpawn" => Some(RuntimeIntrinsic::EcsSpawn),
        "EcsDespawn" => Some(RuntimeIntrinsic::EcsDespawn),
        "EcsSetComponentAt" => Some(RuntimeIntrinsic::EcsSetComponentAt),
        "EcsHasComponentAt" => Some(RuntimeIntrinsic::EcsHasComponentAt),
        "EcsGetComponentAt" => Some(RuntimeIntrinsic::EcsGetComponentAt),
        "EcsRemoveComponentAt" => Some(RuntimeIntrinsic::EcsRemoveComponentAt),
        _ => None,
    }
}

fn runtime_value_to_string(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Int(value) => value.to_string(),
        RuntimeValue::Bool(value) => value.to_string(),
        RuntimeValue::Str(value) => value.clone(),
        RuntimeValue::Pair(left, right) => format!(
            "({}, {})",
            runtime_value_to_string(left),
            runtime_value_to_string(right)
        ),
        RuntimeValue::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(runtime_value_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(runtime_value_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::Opaque(RuntimeOpaqueValue::FileStream(handle)) => {
            format!("<FileStream:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Window(handle)) => {
            format!("<Window:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Image(handle)) => {
            format!("<Image:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AppFrame(handle)) => {
            format!("<AppFrame:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AudioDevice(handle)) => {
            format!("<AudioDevice:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AudioBuffer(handle)) => {
            format!("<AudioBuffer:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AudioPlayback(handle)) => {
            format!("<AudioPlayback:{}>", handle.0)
        }
        RuntimeValue::Record { name, fields } => format!(
            "{}{{{}}}",
            name,
            fields
                .iter()
                .map(|(field, value)| format!("{field}: {}", runtime_value_to_string(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::Variant { name, payload } => {
            if payload.is_empty() {
                name.clone()
            } else {
                format!(
                    "{}({})",
                    name,
                    payload
                        .iter()
                        .map(runtime_value_to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        RuntimeValue::Unit => String::new(),
    }
}

fn lookup_local<'a>(scopes: &'a [RuntimeScope], name: &str) -> Option<&'a RuntimeLocal> {
    scopes.iter().rev().find_map(|scope| scope.locals.get(name))
}

fn lookup_local_mut<'a>(
    scopes: &'a mut [RuntimeScope],
    name: &str,
) -> Option<&'a mut RuntimeLocal> {
    scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.locals.get_mut(name))
}

fn expect_int(value: RuntimeValue, context: &str) -> Result<i64, String> {
    match value {
        RuntimeValue::Int(value) => Ok(value),
        other => Err(format!("{context} expected Int, got `{other:?}`")),
    }
}

fn expect_entity_id(value: RuntimeValue, context: &str) -> Result<i64, String> {
    let entity = expect_int(value, context)?;
    if entity < 0 {
        return Err(format!("{context} entity id must be non-negative"));
    }
    Ok(entity)
}

fn expect_bool(value: RuntimeValue, context: &str) -> Result<bool, String> {
    match value {
        RuntimeValue::Bool(value) => Ok(value),
        other => Err(format!("{context} expected Bool, got `{other:?}`")),
    }
}

fn expect_str(value: RuntimeValue, context: &str) -> Result<String, String> {
    match value {
        RuntimeValue::Str(value) => Ok(value),
        other => Err(format!("{context} expected Str, got `{other:?}`")),
    }
}

fn expect_byte_array(value: RuntimeValue, context: &str) -> Result<Vec<u8>, String> {
    let RuntimeValue::Array(values) = value else {
        return Err(format!("{context} expected Array[Int]"));
    };
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let byte = expect_int(value, context)?;
            u8::try_from(byte).map_err(|_| {
                format!("{context} byte index `{index}` is out of range 0..255: `{byte}`")
            })
        })
        .collect()
}

fn expect_string_list(value: RuntimeValue, context: &str) -> Result<Vec<String>, String> {
    let RuntimeValue::List(values) = value else {
        return Err(format!("{context} expected List[Str]"));
    };
    values
        .into_iter()
        .map(|value| expect_str(value, context))
        .collect()
}

fn expect_file_stream(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeFileStreamHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::FileStream(handle)) = value else {
        return Err(format!("{context} expected FileStream"));
    };
    Ok(handle)
}

fn expect_window(value: RuntimeValue, context: &str) -> Result<RuntimeWindowHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Window(handle)) = value else {
        return Err(format!("{context} expected Window"));
    };
    Ok(handle)
}

fn expect_image(value: RuntimeValue, context: &str) -> Result<RuntimeImageHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Image(handle)) = value else {
        return Err(format!("{context} expected Image"));
    };
    Ok(handle)
}

fn expect_app_frame(value: RuntimeValue, context: &str) -> Result<RuntimeAppFrameHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::AppFrame(handle)) = value else {
        return Err(format!("{context} expected AppFrame"));
    };
    Ok(handle)
}

fn expect_audio_device(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeAudioDeviceHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::AudioDevice(handle)) = value else {
        return Err(format!("{context} expected AudioDevice"));
    };
    Ok(handle)
}

fn expect_audio_buffer(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeAudioBufferHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::AudioBuffer(handle)) = value else {
        return Err(format!("{context} expected AudioBuffer"));
    };
    Ok(handle)
}

fn expect_audio_playback(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeAudioPlaybackHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::AudioPlayback(handle)) = value else {
        return Err(format!("{context} expected AudioPlayback"));
    };
    Ok(handle)
}

fn bytes_to_runtime_array(bytes: impl IntoIterator<Item = u8>) -> RuntimeValue {
    RuntimeValue::Array(
        bytes
            .into_iter()
            .map(|byte| RuntimeValue::Int(i64::from(byte)))
            .collect(),
    )
}

fn require_runtime_type_key(type_args: &[String], context: &str) -> Result<Vec<String>, String> {
    if type_args.is_empty() {
        return Err(format!("{context} requires at least one type argument"));
    }
    Ok(type_args.to_vec())
}

fn ecs_slot<'a>(
    state: &'a RuntimeExecutionState,
    key: &[String],
    entity: i64,
) -> Option<&'a RuntimeValue> {
    state
        .component_slots
        .get(key)
        .and_then(|slots| slots.get(&entity))
}

fn ecs_slot_mut<'a>(
    state: &'a mut RuntimeExecutionState,
    key: &[String],
) -> &'a mut BTreeMap<i64, RuntimeValue> {
    state.component_slots.entry(key.to_vec()).or_default()
}

fn ecs_entity_exists(state: &RuntimeExecutionState, entity: i64) -> bool {
    entity == 0 || state.live_entities.contains(&entity)
}

fn runtime_behavior_step(
    plan: &RuntimePackagePlan,
    phase: &str,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<i64, String> {
    for (routine_index, routine) in plan.routines.iter().enumerate() {
        if routine.symbol_kind != "behavior" && routine.symbol_kind != "system" {
            continue;
        }
        if routine.behavior_attrs.get("phase").map(String::as_str) != Some(phase) {
            continue;
        }
        let result =
            execute_routine_with_state(plan, routine_index, Vec::new(), Vec::new(), state, host)?;
        let code = match result {
            RuntimeValue::Int(value) => value,
            RuntimeValue::Unit => 0,
            other => {
                return Err(format!(
                    "behavior/system `{}` in phase `{phase}` returned unsupported value `{other:?}`",
                    routine.symbol_name
                ));
            }
        };
        if code != 0 {
            return Ok(code);
        }
    }
    Ok(0)
}

fn expect_single_arg(mut args: Vec<RuntimeValue>, name: &str) -> Result<RuntimeValue, String> {
    if args.len() != 1 {
        return Err(format!("{name} expects one argument"));
    }
    Ok(args.remove(0))
}

fn collect_call_args(
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<Vec<RuntimeCallArg>, String> {
    let mut values = args
        .iter()
        .map(|arg| {
            Ok(RuntimeCallArg {
                name: arg.name.clone(),
                value: eval_expr(
                    &arg.value,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    for attachment in attached {
        let (name, value) = match attachment {
            ParsedHeaderAttachment::Named { name, value } => (
                Some(name.clone()),
                eval_expr(
                    value,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
            ),
            ParsedHeaderAttachment::Chain { expr } => (
                None,
                eval_expr(
                    expr,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
            ),
        };
        values.push(RuntimeCallArg { name, value });
    }
    Ok(values)
}

fn bind_call_args_for_routine(
    routine: &RuntimeRoutinePlan,
    args: Vec<RuntimeCallArg>,
) -> Result<Vec<RuntimeValue>, String> {
    let mut bound = vec![None; routine.params.len()];
    let mut next_positional = 0usize;
    for arg in args {
        let index = if let Some(name) = &arg.name {
            routine
                .params
                .iter()
                .position(|param| param.name == *name)
                .ok_or_else(|| {
                    format!(
                        "runtime routine `{}` has no parameter `{name}`",
                        routine.symbol_name
                    )
                })?
        } else {
            let Some(index) =
                (next_positional..routine.params.len()).find(|index| bound[*index].is_none())
            else {
                return Err(format!(
                    "runtime routine `{}` received too many arguments",
                    routine.symbol_name
                ));
            };
            next_positional = index + 1;
            index
        };
        if bound[index].is_some() {
            return Err(format!(
                "runtime routine `{}` received duplicate argument for `{}`",
                routine.symbol_name, routine.params[index].name
            ));
        }
        bound[index] = Some(arg.value);
    }
    let missing = routine
        .params
        .iter()
        .zip(bound.iter())
        .filter_map(|(param, value)| value.is_none().then_some(param.name.clone()))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "runtime routine `{}` is missing arguments for {}",
            routine.symbol_name,
            missing.join(", ")
        ));
    }
    Ok(bound
        .into_iter()
        .map(|value| value.expect("missing args should have returned"))
        .collect())
}

fn ok_variant(value: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Variant {
        name: "Result.Ok".to_string(),
        payload: vec![value],
    }
}

fn make_pair(left: RuntimeValue, right: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Pair(Box::new(left), Box::new(right))
}

fn variant_name_matches(name: &str, expected: &str) -> bool {
    name == expected
        || name.ends_with(&format!(".{expected}"))
        || expected.ends_with(&format!(".{name}"))
}

fn opaque_type_name(value: &RuntimeOpaqueValue) -> &'static str {
    match value {
        RuntimeOpaqueValue::FileStream(_) => "std.fs.FileStream",
        RuntimeOpaqueValue::Window(_) => "std.window.Window",
        RuntimeOpaqueValue::Image(_) => "std.canvas.Image",
        RuntimeOpaqueValue::AppFrame(_) => "std.events.AppFrame",
        RuntimeOpaqueValue::AudioDevice(_) => "std.audio.AudioDevice",
        RuntimeOpaqueValue::AudioBuffer(_) => "std.audio.AudioBuffer",
        RuntimeOpaqueValue::AudioPlayback(_) => "std.audio.AudioPlayback",
    }
}

fn runtime_method_callable_path(receiver: &RuntimeValue, qualifier: &str) -> Option<Vec<String>> {
    let type_name = match receiver {
        RuntimeValue::Record { name, .. } => name.as_str(),
        RuntimeValue::Opaque(value) => opaque_type_name(value),
        _ => return None,
    };
    let mut segments = type_name
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if segments.len() > 1 {
        segments.pop();
        segments.push(qualifier.to_string());
        Some(segments)
    } else {
        Some(vec![qualifier.to_string()])
    }
}

fn eval_member_value(base: RuntimeValue, member: &str) -> Result<RuntimeValue, String> {
    match base {
        RuntimeValue::Pair(left, right) => match member {
            "0" => Ok((*left).clone()),
            "1" => Ok((*right).clone()),
            _ => Err(format!("pair has no member `.{member}`")),
        },
        RuntimeValue::Record { name, fields } => fields
            .get(member)
            .cloned()
            .ok_or_else(|| format!("record `{name}` has no field `.{member}`")),
        RuntimeValue::Variant { name, payload } => {
            match (name.as_str(), member, payload.as_slice()) {
                (name, "is_ok", [_]) if variant_name_matches(name, "Result.Ok") => {
                    Ok(RuntimeValue::Bool(true))
                }
                (name, "is_err", [_]) if variant_name_matches(name, "Result.Ok") => {
                    Ok(RuntimeValue::Bool(false))
                }
                (name, "is_ok", [_]) if variant_name_matches(name, "Result.Err") => {
                    Ok(RuntimeValue::Bool(false))
                }
                (name, "is_err", [_]) if variant_name_matches(name, "Result.Err") => {
                    Ok(RuntimeValue::Bool(true))
                }
                _ => Err(format!(
                    "unsupported runtime member `.{member}` on `{name}`"
                )),
            }
        }
        other => Err(format!(
            "unsupported runtime member access `.{member}` on `{other:?}`"
        )),
    }
}

fn eval_match_expr(
    subject: &ParsedExpr,
    arms: &[ParsedMatchArm],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let subject = eval_expr(
        subject,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    for arm in arms {
        for pattern in &arm.patterns {
            let mut bindings = BTreeMap::new();
            if !match_pattern(pattern, &subject, &mut bindings) {
                continue;
            }
            let mut scope = RuntimeScope::default();
            for (name, value) in bindings {
                scope.locals.insert(
                    name,
                    RuntimeLocal {
                        mutable: false,
                        value,
                    },
                );
            }
            scopes.push(scope);
            let result = eval_expr(
                &arm.value,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            );
            scopes.pop();
            return result;
        }
    }
    Err("runtime match expression had no matching arm".to_string())
}

fn err_variant(message: String) -> RuntimeValue {
    RuntimeValue::Variant {
        name: "Result.Err".to_string(),
        payload: vec![RuntimeValue::Str(message)],
    }
}

fn try_construct_record_value(
    callable: &[String],
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<Option<RuntimeValue>, String> {
    if !attached.is_empty() {
        return Ok(None);
    }
    if args.is_empty() {
        return Ok(None);
    }
    let mut fields = BTreeMap::new();
    for arg in args {
        let Some(name) = &arg.name else {
            return Ok(None);
        };
        if fields.contains_key(name) {
            return Err(format!(
                "record constructor `{}` provided duplicate field `{name}`",
                callable.join(".")
            ));
        }
        fields.insert(
            name.clone(),
            eval_expr(
                &arg.value,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?,
        );
    }
    Ok(Some(RuntimeValue::Record {
        name: callable.join("."),
        fields,
    }))
}

fn looks_like_variant_constructor(callable: &[String]) -> bool {
    if callable.len() < 2 {
        return false;
    }
    let Some(enum_name) = callable.get(callable.len().saturating_sub(2)) else {
        return false;
    };
    let Some(variant_name) = callable.last() else {
        return false;
    };
    enum_name.chars().next().is_some_and(char::is_uppercase)
        && variant_name.chars().next().is_some_and(char::is_uppercase)
}

fn try_construct_variant_value(
    callable: &[String],
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<Option<RuntimeValue>, String> {
    if !looks_like_variant_constructor(callable) || !attached.is_empty() {
        return Ok(None);
    }
    let mut payload = Vec::with_capacity(args.len());
    for arg in args {
        if arg.name.is_some() {
            return Ok(None);
        }
        payload.push(eval_expr(
            &arg.value,
            plan,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?);
    }
    Ok(Some(RuntimeValue::Variant {
        name: callable.join("."),
        payload,
    }))
}

fn into_iterable_values(value: RuntimeValue) -> Result<Vec<RuntimeValue>, String> {
    match value {
        RuntimeValue::List(values) => Ok(values),
        RuntimeValue::Array(values) => Ok(values),
        other => Err(format!(
            "runtime for-loop expects List or Array, got `{other:?}`"
        )),
    }
}

fn eval_qualifier(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    qualifier: &str,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match qualifier {
        "len" => {
            if !args.is_empty() {
                return Err("collection len expects zero arguments".to_string());
            }
            let base = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let len = match base {
                RuntimeValue::List(values) => values.len(),
                RuntimeValue::Array(values) => values.len(),
                other => {
                    return Err(format!(
                        "collection len expects List or Array subject, got `{other:?}`"
                    ));
                }
            };
            Ok(RuntimeValue::Int(i64::try_from(len).map_err(|_| {
                "collection length does not fit in i64".to_string()
            })?))
        }
        "is_ok" => {
            if !args.is_empty() {
                return Err("result is_ok expects zero arguments".to_string());
            }
            let base = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let RuntimeValue::Variant { name, payload } = base else {
                return Err("result is_ok expects Result subject".to_string());
            };
            match payload.as_slice() {
                [_] if variant_name_matches(&name, "Result.Ok") => Ok(RuntimeValue::Bool(true)),
                [_] if variant_name_matches(&name, "Result.Err") => Ok(RuntimeValue::Bool(false)),
                _ => Err("result is_ok expects Result subject".to_string()),
            }
        }
        "is_err" => {
            if !args.is_empty() {
                return Err("result is_err expects zero arguments".to_string());
            }
            let base = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let RuntimeValue::Variant { name, payload } = base else {
                return Err("result is_err expects Result subject".to_string());
            };
            match payload.as_slice() {
                [_] if variant_name_matches(&name, "Result.Ok") => Ok(RuntimeValue::Bool(false)),
                [_] if variant_name_matches(&name, "Result.Err") => Ok(RuntimeValue::Bool(true)),
                _ => Err("result is_err expects Result subject".to_string()),
            }
        }
        "unwrap_or" => {
            if args.len() != 1 {
                return Err("variant unwrap_or expects one argument".to_string());
            }
            let fallback = eval_expr(
                &args[0].value,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let base = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let RuntimeValue::Variant { name, payload } = base else {
                return Err("variant unwrap_or expects Result or Option subject".to_string());
            };
            match payload.as_slice() {
                [value] if variant_name_matches(&name, "Result.Ok") => Ok(value.clone()),
                [_] if variant_name_matches(&name, "Result.Err") => Ok(fallback),
                [value] if variant_name_matches(&name, "Option.Some") => Ok(value.clone()),
                [] if variant_name_matches(&name, "Option.None") => Ok(fallback),
                _ => Err("variant unwrap_or expects Result or Option subject".to_string()),
            }
        }
        "is_some" => {
            if !args.is_empty() {
                return Err("option is_some expects zero arguments".to_string());
            }
            let base = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let RuntimeValue::Variant { name, payload } = base else {
                return Err("option is_some expects Option subject".to_string());
            };
            match payload.as_slice() {
                [_] if variant_name_matches(&name, "Option.Some") => Ok(RuntimeValue::Bool(true)),
                [] if variant_name_matches(&name, "Option.None") => Ok(RuntimeValue::Bool(false)),
                _ => Err("option is_some expects Option subject".to_string()),
            }
        }
        "is_none" => {
            if !args.is_empty() {
                return Err("option is_none expects zero arguments".to_string());
            }
            let base = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let RuntimeValue::Variant { name, payload } = base else {
                return Err("option is_none expects Option subject".to_string());
            };
            match payload.as_slice() {
                [_] if variant_name_matches(&name, "Option.Some") => Ok(RuntimeValue::Bool(false)),
                [] if variant_name_matches(&name, "Option.None") => Ok(RuntimeValue::Bool(true)),
                _ => Err("option is_none expects Option subject".to_string()),
            }
        }
        "to_list" => {
            if !args.is_empty() {
                return Err("array to_list expects zero arguments".to_string());
            }
            let base = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let RuntimeValue::Array(values) = base else {
                return Err("array to_list expects Array subject".to_string());
            };
            Ok(RuntimeValue::List(values))
        }
        "push" => {
            if args.len() != 1 {
                return Err("list push expects one argument".to_string());
            }
            let ParsedExpr::Path(segments) = subject else {
                return Err("list push requires a local List subject".to_string());
            };
            if segments.len() != 1 {
                return Err("list push requires a local List subject".to_string());
            }
            let value = eval_expr(
                &args[0].value,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let local = lookup_local_mut(scopes, &segments[0])
                .ok_or_else(|| format!("runtime local `{}` is unresolved", segments[0]))?;
            if !local.mutable {
                return Err(format!("runtime local `{}` is not mutable", segments[0]));
            }
            let RuntimeValue::List(values) = &mut local.value else {
                return Err(format!("runtime local `{}` is not a List", segments[0]));
            };
            values.push(value);
            Ok(RuntimeValue::Unit)
        }
        "pop" => {
            if !args.is_empty() {
                return Err("list pop expects zero arguments".to_string());
            }
            let ParsedExpr::Path(segments) = subject else {
                return Err("list pop requires a local List subject".to_string());
            };
            if segments.len() != 1 {
                return Err("list pop requires a local List subject".to_string());
            }
            let local = lookup_local_mut(scopes, &segments[0])
                .ok_or_else(|| format!("runtime local `{}` is unresolved", segments[0]))?;
            if !local.mutable {
                return Err(format!("runtime local `{}` is not mutable", segments[0]));
            }
            let RuntimeValue::List(values) = &mut local.value else {
                return Err(format!("runtime local `{}` is not a List", segments[0]));
            };
            values
                .pop()
                .ok_or_else(|| format!("list pop on `{}` was empty", segments[0]))
        }
        "try_pop_or" => {
            if args.len() != 1 {
                return Err("list try_pop_or expects one argument".to_string());
            }
            let ParsedExpr::Path(segments) = subject else {
                return Err("list try_pop_or requires a local List subject".to_string());
            };
            if segments.len() != 1 {
                return Err("list try_pop_or requires a local List subject".to_string());
            }
            let fallback = eval_expr(
                &args[0].value,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let local = lookup_local_mut(scopes, &segments[0])
                .ok_or_else(|| format!("runtime local `{}` is unresolved", segments[0]))?;
            if !local.mutable {
                return Err(format!("runtime local `{}` is not mutable", segments[0]));
            }
            let RuntimeValue::List(values) = &mut local.value else {
                return Err(format!("runtime local `{}` is not a List", segments[0]));
            };
            Ok(match values.pop() {
                Some(value) => {
                    RuntimeValue::Pair(Box::new(RuntimeValue::Bool(true)), Box::new(value))
                }
                None => RuntimeValue::Pair(Box::new(RuntimeValue::Bool(false)), Box::new(fallback)),
            })
        }
        _ => {
            let receiver = eval_expr(
                subject,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let Some(callable) = runtime_method_callable_path(&receiver, qualifier) else {
                return Err(format!("unsupported runtime qualifier `{qualifier}`"));
            };
            let Some(routine_index) = resolve_routine_index(plan, current_module_id, &callable)
            else {
                return Err(format!(
                    "unsupported runtime qualifier `{qualifier}` for receiver `{receiver:?}`"
                ));
            };
            let mut values = vec![receiver];
            values.extend(
                args.iter()
                    .map(|arg| {
                        eval_expr(
                            &arg.value,
                            plan,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            );
            execute_routine_with_state(plan, routine_index, Vec::new(), values, state, host)
        }
    }
}

fn execute_runtime_intrinsic(
    intrinsic: RuntimeIntrinsic,
    type_args: &[String],
    args: Vec<RuntimeValue>,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match intrinsic {
        RuntimeIntrinsic::IoPrint => {
            let value = expect_single_arg(args, "print")?;
            host.print(&runtime_value_to_string(&value))?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoPrintLine => {
            let value = expect_single_arg(args, "print_line")?;
            host.print(&runtime_value_to_string(&value))?;
            host.print("\n")?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoEprint => {
            let value = expect_single_arg(args, "eprint")?;
            host.eprint(&runtime_value_to_string(&value))?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoEprintLine => {
            let value = expect_single_arg(args, "eprint_line")?;
            host.eprint(&runtime_value_to_string(&value))?;
            host.eprint("\n")?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoFlushStdout => {
            if !args.is_empty() {
                return Err("flush_stdout expects zero arguments".to_string());
            }
            host.flush_stdout()?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoFlushStderr => {
            if !args.is_empty() {
                return Err("flush_stderr expects zero arguments".to_string());
            }
            host.flush_stderr()?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::IoStdinReadLineTry => {
            if !args.is_empty() {
                return Err("stdin_read_line expects zero arguments".to_string());
            }
            Ok(match host.stdin_read_line() {
                Ok(line) => ok_variant(RuntimeValue::Str(line)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::ArgCount => {
            if !args.is_empty() {
                return Err("arg_count expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Int(
                i64::try_from(host.arg_count()?)
                    .map_err(|_| "runtime arg count does not fit in i64".to_string())?,
            ))
        }
        RuntimeIntrinsic::ArgGet => {
            let index = expect_int(expect_single_arg(args, "arg_get")?, "arg_get")?;
            if index < 0 {
                return Err("arg_get index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Str(host.arg_get(index as usize)?))
        }
        RuntimeIntrinsic::EnvHas => {
            let name = expect_str(expect_single_arg(args, "env_has")?, "env_has")?;
            Ok(RuntimeValue::Bool(host.env_has(&name)?))
        }
        RuntimeIntrinsic::EnvGet => {
            let name = expect_str(expect_single_arg(args, "env_get")?, "env_get")?;
            Ok(RuntimeValue::Str(host.env_get(&name)?))
        }
        RuntimeIntrinsic::PathCwd => {
            if !args.is_empty() {
                return Err("path_cwd expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Str(host.cwd()?))
        }
        RuntimeIntrinsic::PathJoin => {
            if args.len() != 2 {
                return Err("path_join expects two arguments".to_string());
            }
            Ok(RuntimeValue::Str(host.path_join(
                &expect_str(args[0].clone(), "path_join")?,
                &expect_str(args[1].clone(), "path_join")?,
            )?))
        }
        RuntimeIntrinsic::PathNormalize => {
            let path = expect_str(expect_single_arg(args, "path_normalize")?, "path_normalize")?;
            Ok(RuntimeValue::Str(host.path_normalize(&path)?))
        }
        RuntimeIntrinsic::PathParent => {
            let path = expect_str(expect_single_arg(args, "path_parent")?, "path_parent")?;
            Ok(RuntimeValue::Str(host.path_parent(&path)?))
        }
        RuntimeIntrinsic::PathFileName => {
            let path = expect_str(expect_single_arg(args, "path_file_name")?, "path_file_name")?;
            Ok(RuntimeValue::Str(host.path_file_name(&path)?))
        }
        RuntimeIntrinsic::PathExt => {
            let path = expect_str(expect_single_arg(args, "path_ext")?, "path_ext")?;
            Ok(RuntimeValue::Str(host.path_ext(&path)?))
        }
        RuntimeIntrinsic::PathIsAbsolute => {
            let path = expect_str(
                expect_single_arg(args, "path_is_absolute")?,
                "path_is_absolute",
            )?;
            Ok(RuntimeValue::Bool(host.path_is_absolute(&path)?))
        }
        RuntimeIntrinsic::PathStemTry => {
            let path = expect_str(expect_single_arg(args, "path_stem")?, "path_stem")?;
            Ok(match host.path_stem(&path) {
                Ok(stem) => ok_variant(RuntimeValue::Str(stem)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::PathWithExt => {
            if args.len() != 2 {
                return Err("path_with_ext expects two arguments".to_string());
            }
            Ok(RuntimeValue::Str(host.path_with_ext(
                &expect_str(args[0].clone(), "path_with_ext")?,
                &expect_str(args[1].clone(), "path_with_ext")?,
            )?))
        }
        RuntimeIntrinsic::PathRelativeToTry => {
            if args.len() != 2 {
                return Err("path_relative_to expects two arguments".to_string());
            }
            Ok(
                match host.path_relative_to(
                    &expect_str(args[0].clone(), "path_relative_to")?,
                    &expect_str(args[1].clone(), "path_relative_to")?,
                ) {
                    Ok(path) => ok_variant(RuntimeValue::Str(path)),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::PathCanonicalizeTry => {
            let path = expect_str(
                expect_single_arg(args, "path_canonicalize")?,
                "path_canonicalize",
            )?;
            Ok(match host.path_canonicalize(&path) {
                Ok(path) => ok_variant(RuntimeValue::Str(path)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::PathStripPrefixTry => {
            if args.len() != 2 {
                return Err("path_strip_prefix expects two arguments".to_string());
            }
            Ok(
                match host.path_strip_prefix(
                    &expect_str(args[0].clone(), "path_strip_prefix")?,
                    &expect_str(args[1].clone(), "path_strip_prefix")?,
                ) {
                    Ok(path) => ok_variant(RuntimeValue::Str(path)),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::FsExists => {
            let path = expect_str(expect_single_arg(args, "fs_exists")?, "fs_exists")?;
            Ok(RuntimeValue::Bool(host.fs_exists(&path)?))
        }
        RuntimeIntrinsic::FsIsFile => {
            let path = expect_str(expect_single_arg(args, "fs_is_file")?, "fs_is_file")?;
            Ok(RuntimeValue::Bool(host.fs_is_file(&path)?))
        }
        RuntimeIntrinsic::FsIsDir => {
            let path = expect_str(expect_single_arg(args, "fs_is_dir")?, "fs_is_dir")?;
            Ok(RuntimeValue::Bool(host.fs_is_dir(&path)?))
        }
        RuntimeIntrinsic::FsReadTextTry => {
            let path = expect_str(expect_single_arg(args, "fs_read_text")?, "fs_read_text")?;
            Ok(match host.fs_read_text(&path) {
                Ok(text) => ok_variant(RuntimeValue::Str(text)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsReadBytesTry => {
            let path = expect_str(expect_single_arg(args, "fs_read_bytes")?, "fs_read_bytes")?;
            Ok(match host.fs_read_bytes(&path) {
                Ok(bytes) => ok_variant(bytes_to_runtime_array(bytes)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsStreamOpenReadTry => {
            let path = expect_str(
                expect_single_arg(args, "fs_stream_open_read")?,
                "fs_stream_open_read",
            )?;
            Ok(match host.fs_stream_open_read(&path) {
                Ok(handle) => {
                    ok_variant(RuntimeValue::Opaque(RuntimeOpaqueValue::FileStream(handle)))
                }
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsStreamOpenWriteTry => {
            if args.len() != 2 {
                return Err("fs_stream_open_write expects two arguments".to_string());
            }
            let path = expect_str(args[0].clone(), "fs_stream_open_write")?;
            let append = expect_bool(args[1].clone(), "fs_stream_open_write")?;
            Ok(match host.fs_stream_open_write(&path, append) {
                Ok(handle) => {
                    ok_variant(RuntimeValue::Opaque(RuntimeOpaqueValue::FileStream(handle)))
                }
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsStreamReadTry => {
            if args.len() != 2 {
                return Err("fs_stream_read expects two arguments".to_string());
            }
            let stream = expect_file_stream(args[0].clone(), "fs_stream_read")?;
            let max_bytes = expect_int(args[1].clone(), "fs_stream_read")?;
            if max_bytes < 0 {
                return Err("fs_stream_read max_bytes must be non-negative".to_string());
            }
            Ok(match host.fs_stream_read(stream, max_bytes as usize) {
                Ok(bytes) => ok_variant(bytes_to_runtime_array(bytes)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsStreamWriteTry => {
            if args.len() != 2 {
                return Err("fs_stream_write expects two arguments".to_string());
            }
            let stream = expect_file_stream(args[0].clone(), "fs_stream_write")?;
            let bytes = expect_byte_array(args[1].clone(), "fs_stream_write")?;
            Ok(match host.fs_stream_write(stream, &bytes) {
                Ok(written) => {
                    ok_variant(RuntimeValue::Int(i64::try_from(written).map_err(|_| {
                        "fs_stream_write byte count does not fit in i64".to_string()
                    })?))
                }
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsStreamEofTry => {
            let stream =
                expect_file_stream(expect_single_arg(args, "fs_stream_eof")?, "fs_stream_eof")?;
            Ok(match host.fs_stream_eof(stream) {
                Ok(done) => ok_variant(RuntimeValue::Bool(done)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsStreamCloseTry => {
            let stream = expect_file_stream(
                expect_single_arg(args, "fs_stream_close")?,
                "fs_stream_close",
            )?;
            Ok(match host.fs_stream_close(stream) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsWriteTextTry => {
            if args.len() != 2 {
                return Err("fs_write_text expects two arguments".to_string());
            }
            Ok(
                match host.fs_write_text(
                    &expect_str(args[0].clone(), "fs_write_text")?,
                    &expect_str(args[1].clone(), "fs_write_text")?,
                ) {
                    Ok(()) => ok_variant(RuntimeValue::Unit),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::FsWriteBytesTry => {
            if args.len() != 2 {
                return Err("fs_write_bytes expects two arguments".to_string());
            }
            Ok(
                match host.fs_write_bytes(
                    &expect_str(args[0].clone(), "fs_write_bytes")?,
                    &expect_byte_array(args[1].clone(), "fs_write_bytes")?,
                ) {
                    Ok(()) => ok_variant(RuntimeValue::Unit),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::FsListDirTry => {
            let path = expect_str(expect_single_arg(args, "fs_list_dir")?, "fs_list_dir")?;
            Ok(match host.fs_list_dir(&path) {
                Ok(entries) => ok_variant(RuntimeValue::List(
                    entries.into_iter().map(RuntimeValue::Str).collect(),
                )),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsMkdirAllTry => {
            let path = expect_str(expect_single_arg(args, "fs_mkdir_all")?, "fs_mkdir_all")?;
            Ok(match host.fs_mkdir_all(&path) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsCreateDirTry => {
            let path = expect_str(expect_single_arg(args, "fs_create_dir")?, "fs_create_dir")?;
            Ok(match host.fs_create_dir(&path) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsRemoveFileTry => {
            let path = expect_str(expect_single_arg(args, "fs_remove_file")?, "fs_remove_file")?;
            Ok(match host.fs_remove_file(&path) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsRemoveDirTry => {
            let path = expect_str(expect_single_arg(args, "fs_remove_dir")?, "fs_remove_dir")?;
            Ok(match host.fs_remove_dir(&path) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsRemoveDirAllTry => {
            let path = expect_str(
                expect_single_arg(args, "fs_remove_dir_all")?,
                "fs_remove_dir_all",
            )?;
            Ok(match host.fs_remove_dir_all(&path) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsCopyFileTry => {
            if args.len() != 2 {
                return Err("fs_copy_file expects two arguments".to_string());
            }
            Ok(
                match host.fs_copy_file(
                    &expect_str(args[0].clone(), "fs_copy_file")?,
                    &expect_str(args[1].clone(), "fs_copy_file")?,
                ) {
                    Ok(()) => ok_variant(RuntimeValue::Unit),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::FsRenameTry => {
            if args.len() != 2 {
                return Err("fs_rename expects two arguments".to_string());
            }
            Ok(
                match host.fs_rename(
                    &expect_str(args[0].clone(), "fs_rename")?,
                    &expect_str(args[1].clone(), "fs_rename")?,
                ) {
                    Ok(()) => ok_variant(RuntimeValue::Unit),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::FsFileSizeTry => {
            let path = expect_str(expect_single_arg(args, "fs_file_size")?, "fs_file_size")?;
            Ok(match host.fs_file_size(&path) {
                Ok(size) => ok_variant(RuntimeValue::Int(size)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::FsModifiedUnixMsTry => {
            let path = expect_str(
                expect_single_arg(args, "fs_modified_unix_ms")?,
                "fs_modified_unix_ms",
            )?;
            Ok(match host.fs_modified_unix_ms(&path) {
                Ok(value) => ok_variant(RuntimeValue::Int(value)),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::WindowOpenTry => {
            if args.len() != 3 {
                return Err("window_open expects three arguments".to_string());
            }
            let title = expect_str(args[0].clone(), "window_open")?;
            let width = expect_int(args[1].clone(), "window_open")?;
            let height = expect_int(args[2].clone(), "window_open")?;
            Ok(match host.window_open(&title, width, height) {
                Ok(handle) => ok_variant(RuntimeValue::Opaque(RuntimeOpaqueValue::Window(handle))),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::CanvasAlive => {
            let window = expect_window(expect_single_arg(args, "canvas_alive")?, "canvas_alive")?;
            Ok(RuntimeValue::Bool(host.window_alive(window)?))
        }
        RuntimeIntrinsic::CanvasFill => {
            if args.len() != 2 {
                return Err("canvas_fill expects two arguments".to_string());
            }
            host.canvas_fill(
                expect_window(args[0].clone(), "canvas_fill")?,
                expect_int(args[1].clone(), "canvas_fill")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasRect => {
            if args.len() != 6 {
                return Err("canvas_rect expects six arguments".to_string());
            }
            host.canvas_rect(
                expect_window(args[0].clone(), "canvas_rect")?,
                expect_int(args[1].clone(), "canvas_rect")?,
                expect_int(args[2].clone(), "canvas_rect")?,
                expect_int(args[3].clone(), "canvas_rect")?,
                expect_int(args[4].clone(), "canvas_rect")?,
                expect_int(args[5].clone(), "canvas_rect")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasLine => {
            if args.len() != 6 {
                return Err("canvas_line expects six arguments".to_string());
            }
            host.canvas_line(
                expect_window(args[0].clone(), "canvas_line")?,
                expect_int(args[1].clone(), "canvas_line")?,
                expect_int(args[2].clone(), "canvas_line")?,
                expect_int(args[3].clone(), "canvas_line")?,
                expect_int(args[4].clone(), "canvas_line")?,
                expect_int(args[5].clone(), "canvas_line")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasCircleFill => {
            if args.len() != 5 {
                return Err("canvas_circle_fill expects five arguments".to_string());
            }
            host.canvas_circle_fill(
                expect_window(args[0].clone(), "canvas_circle_fill")?,
                expect_int(args[1].clone(), "canvas_circle_fill")?,
                expect_int(args[2].clone(), "canvas_circle_fill")?,
                expect_int(args[3].clone(), "canvas_circle_fill")?,
                expect_int(args[4].clone(), "canvas_circle_fill")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasLabel => {
            if args.len() != 5 {
                return Err("canvas_label expects five arguments".to_string());
            }
            host.canvas_label(
                expect_window(args[0].clone(), "canvas_label")?,
                expect_int(args[1].clone(), "canvas_label")?,
                expect_int(args[2].clone(), "canvas_label")?,
                &expect_str(args[3].clone(), "canvas_label")?,
                expect_int(args[4].clone(), "canvas_label")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasLabelSize => {
            let text = expect_str(
                expect_single_arg(args, "canvas_label_size")?,
                "canvas_label_size",
            )?;
            let (w, h) = host.canvas_label_size(&text)?;
            Ok(make_pair(RuntimeValue::Int(w), RuntimeValue::Int(h)))
        }
        RuntimeIntrinsic::CanvasPresent => {
            let window =
                expect_window(expect_single_arg(args, "canvas_present")?, "canvas_present")?;
            host.canvas_present(window)?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasRgb => {
            if args.len() != 3 {
                return Err("canvas_rgb expects three arguments".to_string());
            }
            Ok(RuntimeValue::Int(host.canvas_rgb(
                expect_int(args[0].clone(), "canvas_rgb")?,
                expect_int(args[1].clone(), "canvas_rgb")?,
                expect_int(args[2].clone(), "canvas_rgb")?,
            )?))
        }
        RuntimeIntrinsic::ImageLoadTry => {
            let path = expect_str(expect_single_arg(args, "image_load")?, "image_load")?;
            Ok(match host.image_load(&path) {
                Ok(handle) => ok_variant(RuntimeValue::Opaque(RuntimeOpaqueValue::Image(handle))),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::CanvasImageSize => {
            let image = expect_image(
                expect_single_arg(args, "canvas_image_size")?,
                "canvas_image_size",
            )?;
            let (w, h) = host.canvas_image_size(image)?;
            Ok(make_pair(RuntimeValue::Int(w), RuntimeValue::Int(h)))
        }
        RuntimeIntrinsic::CanvasBlit => {
            if args.len() != 4 {
                return Err("canvas_blit expects four arguments".to_string());
            }
            host.canvas_blit(
                expect_window(args[0].clone(), "canvas_blit")?,
                expect_image(args[1].clone(), "canvas_blit")?,
                expect_int(args[2].clone(), "canvas_blit")?,
                expect_int(args[3].clone(), "canvas_blit")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasBlitScaled => {
            if args.len() != 6 {
                return Err("canvas_blit_scaled expects six arguments".to_string());
            }
            host.canvas_blit_scaled(
                expect_window(args[0].clone(), "canvas_blit_scaled")?,
                expect_image(args[1].clone(), "canvas_blit_scaled")?,
                expect_int(args[2].clone(), "canvas_blit_scaled")?,
                expect_int(args[3].clone(), "canvas_blit_scaled")?,
                expect_int(args[4].clone(), "canvas_blit_scaled")?,
                expect_int(args[5].clone(), "canvas_blit_scaled")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::CanvasBlitRegion => {
            if args.len() != 10 {
                return Err("canvas_blit_region expects ten arguments".to_string());
            }
            host.canvas_blit_region(
                expect_window(args[0].clone(), "canvas_blit_region")?,
                expect_image(args[1].clone(), "canvas_blit_region")?,
                expect_int(args[2].clone(), "canvas_blit_region")?,
                expect_int(args[3].clone(), "canvas_blit_region")?,
                expect_int(args[4].clone(), "canvas_blit_region")?,
                expect_int(args[5].clone(), "canvas_blit_region")?,
                expect_int(args[6].clone(), "canvas_blit_region")?,
                expect_int(args[7].clone(), "canvas_blit_region")?,
                expect_int(args[8].clone(), "canvas_blit_region")?,
                expect_int(args[9].clone(), "canvas_blit_region")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowSize => {
            let window = expect_window(expect_single_arg(args, "window_size")?, "window_size")?;
            let (w, h) = host.window_size(window)?;
            Ok(make_pair(RuntimeValue::Int(w), RuntimeValue::Int(h)))
        }
        RuntimeIntrinsic::WindowResized => {
            let window =
                expect_window(expect_single_arg(args, "window_resized")?, "window_resized")?;
            Ok(RuntimeValue::Bool(host.window_resized(window)?))
        }
        RuntimeIntrinsic::WindowFullscreen => {
            let window = expect_window(
                expect_single_arg(args, "window_fullscreen")?,
                "window_fullscreen",
            )?;
            Ok(RuntimeValue::Bool(host.window_fullscreen(window)?))
        }
        RuntimeIntrinsic::WindowMinimized => {
            let window = expect_window(
                expect_single_arg(args, "window_minimized")?,
                "window_minimized",
            )?;
            Ok(RuntimeValue::Bool(host.window_minimized(window)?))
        }
        RuntimeIntrinsic::WindowMaximized => {
            let window = expect_window(
                expect_single_arg(args, "window_maximized")?,
                "window_maximized",
            )?;
            Ok(RuntimeValue::Bool(host.window_maximized(window)?))
        }
        RuntimeIntrinsic::WindowFocused => {
            let window =
                expect_window(expect_single_arg(args, "window_focused")?, "window_focused")?;
            Ok(RuntimeValue::Bool(host.window_focused(window)?))
        }
        RuntimeIntrinsic::WindowSetTitle => {
            if args.len() != 2 {
                return Err("window_set_title expects two arguments".to_string());
            }
            host.window_set_title(
                expect_window(args[0].clone(), "window_set_title")?,
                &expect_str(args[1].clone(), "window_set_title")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowSetResizable => {
            if args.len() != 2 {
                return Err("window_set_resizable expects two arguments".to_string());
            }
            host.window_set_resizable(
                expect_window(args[0].clone(), "window_set_resizable")?,
                expect_bool(args[1].clone(), "window_set_resizable")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowSetFullscreen => {
            if args.len() != 2 {
                return Err("window_set_fullscreen expects two arguments".to_string());
            }
            host.window_set_fullscreen(
                expect_window(args[0].clone(), "window_set_fullscreen")?,
                expect_bool(args[1].clone(), "window_set_fullscreen")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowSetMinimized => {
            if args.len() != 2 {
                return Err("window_set_minimized expects two arguments".to_string());
            }
            host.window_set_minimized(
                expect_window(args[0].clone(), "window_set_minimized")?,
                expect_bool(args[1].clone(), "window_set_minimized")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowSetMaximized => {
            if args.len() != 2 {
                return Err("window_set_maximized expects two arguments".to_string());
            }
            host.window_set_maximized(
                expect_window(args[0].clone(), "window_set_maximized")?,
                expect_bool(args[1].clone(), "window_set_maximized")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowSetTopmost => {
            if args.len() != 2 {
                return Err("window_set_topmost expects two arguments".to_string());
            }
            host.window_set_topmost(
                expect_window(args[0].clone(), "window_set_topmost")?,
                expect_bool(args[1].clone(), "window_set_topmost")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowSetCursorVisible => {
            if args.len() != 2 {
                return Err("window_set_cursor_visible expects two arguments".to_string());
            }
            host.window_set_cursor_visible(
                expect_window(args[0].clone(), "window_set_cursor_visible")?,
                expect_bool(args[1].clone(), "window_set_cursor_visible")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::WindowClose => {
            let window = expect_window(expect_single_arg(args, "window_close")?, "window_close")?;
            Ok(match host.window_close(window) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::EventsPump => {
            let window = expect_window(expect_single_arg(args, "events_pump")?, "events_pump")?;
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::AppFrame(
                host.events_pump(window)?,
            )))
        }
        RuntimeIntrinsic::EventsPoll => {
            let frame = expect_app_frame(expect_single_arg(args, "events_poll")?, "events_poll")?;
            let (kind, a, b) = host.events_poll(frame)?;
            Ok(make_pair(
                RuntimeValue::Int(kind),
                make_pair(RuntimeValue::Int(a), RuntimeValue::Int(b)),
            ))
        }
        RuntimeIntrinsic::InputKeyCode => {
            let name = expect_str(expect_single_arg(args, "input_key_code")?, "input_key_code")?;
            Ok(RuntimeValue::Int(host.input_key_code(&name)?))
        }
        RuntimeIntrinsic::InputKeyDown => {
            if args.len() != 2 {
                return Err("input_key_down expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.input_key_down(
                expect_app_frame(args[0].clone(), "input_key_down")?,
                expect_int(args[1].clone(), "input_key_down")?,
            )?))
        }
        RuntimeIntrinsic::InputKeyPressed => {
            if args.len() != 2 {
                return Err("input_key_pressed expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.input_key_pressed(
                expect_app_frame(args[0].clone(), "input_key_pressed")?,
                expect_int(args[1].clone(), "input_key_pressed")?,
            )?))
        }
        RuntimeIntrinsic::InputKeyReleased => {
            if args.len() != 2 {
                return Err("input_key_released expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.input_key_released(
                expect_app_frame(args[0].clone(), "input_key_released")?,
                expect_int(args[1].clone(), "input_key_released")?,
            )?))
        }
        RuntimeIntrinsic::InputMouseButtonCode => {
            let name = expect_str(
                expect_single_arg(args, "input_mouse_button_code")?,
                "input_mouse_button_code",
            )?;
            Ok(RuntimeValue::Int(host.input_mouse_button_code(&name)?))
        }
        RuntimeIntrinsic::InputMousePos => {
            let frame = expect_app_frame(
                expect_single_arg(args, "input_mouse_pos")?,
                "input_mouse_pos",
            )?;
            let (x, y) = host.input_mouse_pos(frame)?;
            Ok(make_pair(RuntimeValue::Int(x), RuntimeValue::Int(y)))
        }
        RuntimeIntrinsic::InputMouseDown => {
            if args.len() != 2 {
                return Err("input_mouse_down expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.input_mouse_down(
                expect_app_frame(args[0].clone(), "input_mouse_down")?,
                expect_int(args[1].clone(), "input_mouse_down")?,
            )?))
        }
        RuntimeIntrinsic::InputMousePressed => {
            if args.len() != 2 {
                return Err("input_mouse_pressed expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.input_mouse_pressed(
                expect_app_frame(args[0].clone(), "input_mouse_pressed")?,
                expect_int(args[1].clone(), "input_mouse_pressed")?,
            )?))
        }
        RuntimeIntrinsic::InputMouseReleased => {
            if args.len() != 2 {
                return Err("input_mouse_released expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.input_mouse_released(
                expect_app_frame(args[0].clone(), "input_mouse_released")?,
                expect_int(args[1].clone(), "input_mouse_released")?,
            )?))
        }
        RuntimeIntrinsic::InputMouseWheelY => {
            let frame = expect_app_frame(
                expect_single_arg(args, "input_mouse_wheel_y")?,
                "input_mouse_wheel_y",
            )?;
            Ok(RuntimeValue::Int(host.input_mouse_wheel_y(frame)?))
        }
        RuntimeIntrinsic::InputMouseInWindow => {
            let frame = expect_app_frame(
                expect_single_arg(args, "input_mouse_in_window")?,
                "input_mouse_in_window",
            )?;
            Ok(RuntimeValue::Bool(host.input_mouse_in_window(frame)?))
        }
        RuntimeIntrinsic::TimeMonotonicNowMs => {
            if !args.is_empty() {
                return Err("monotonic_now_ms expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Int(host.monotonic_now_ms()?))
        }
        RuntimeIntrinsic::TimeMonotonicNowNs => {
            if !args.is_empty() {
                return Err("monotonic_now_ns expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Int(host.monotonic_now_ns()?))
        }
        RuntimeIntrinsic::ConcurrentSleep => {
            let ms = expect_int(expect_single_arg(args, "sleep")?, "sleep")?;
            host.sleep_ms(ms)?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentBehaviorStep => {
            let phase = expect_str(expect_single_arg(args, "behavior_step")?, "behavior_step")?;
            Ok(RuntimeValue::Int(runtime_behavior_step(
                plan, &phase, state, host,
            )?))
        }
        RuntimeIntrinsic::AudioDefaultOutputTry => {
            if !args.is_empty() {
                return Err("audio_default_output expects zero arguments".to_string());
            }
            Ok(match host.audio_default_output() {
                Ok(handle) => ok_variant(RuntimeValue::Opaque(RuntimeOpaqueValue::AudioDevice(
                    handle,
                ))),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::AudioOutputClose => {
            let device = expect_audio_device(
                expect_single_arg(args, "audio_output_close")?,
                "audio_output_close",
            )?;
            Ok(match host.audio_output_close(device) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::AudioOutputSampleRateHz => {
            let device = expect_audio_device(
                expect_single_arg(args, "audio_output_sample_rate_hz")?,
                "audio_output_sample_rate_hz",
            )?;
            Ok(RuntimeValue::Int(host.audio_output_sample_rate_hz(device)?))
        }
        RuntimeIntrinsic::AudioOutputChannels => {
            let device = expect_audio_device(
                expect_single_arg(args, "audio_output_channels")?,
                "audio_output_channels",
            )?;
            Ok(RuntimeValue::Int(host.audio_output_channels(device)?))
        }
        RuntimeIntrinsic::AudioBufferLoadWavTry => {
            let path = expect_str(
                expect_single_arg(args, "audio_buffer_load_wav")?,
                "audio_buffer_load_wav",
            )?;
            Ok(match host.audio_buffer_load_wav(&path) {
                Ok(handle) => ok_variant(RuntimeValue::Opaque(RuntimeOpaqueValue::AudioBuffer(
                    handle,
                ))),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::AudioBufferFrames => {
            let buffer = expect_audio_buffer(
                expect_single_arg(args, "audio_buffer_frames")?,
                "audio_buffer_frames",
            )?;
            Ok(RuntimeValue::Int(host.audio_buffer_frames(buffer)?))
        }
        RuntimeIntrinsic::AudioBufferChannels => {
            let buffer = expect_audio_buffer(
                expect_single_arg(args, "audio_buffer_channels")?,
                "audio_buffer_channels",
            )?;
            Ok(RuntimeValue::Int(host.audio_buffer_channels(buffer)?))
        }
        RuntimeIntrinsic::AudioBufferSampleRateHz => {
            let buffer = expect_audio_buffer(
                expect_single_arg(args, "audio_buffer_sample_rate_hz")?,
                "audio_buffer_sample_rate_hz",
            )?;
            Ok(RuntimeValue::Int(host.audio_buffer_sample_rate_hz(buffer)?))
        }
        RuntimeIntrinsic::AudioPlayBufferTry => {
            if args.len() != 2 {
                return Err("audio_play_buffer expects two arguments".to_string());
            }
            Ok(
                match host.audio_play_buffer(
                    expect_audio_device(args[0].clone(), "audio_play_buffer")?,
                    expect_audio_buffer(args[1].clone(), "audio_play_buffer")?,
                ) {
                    Ok(handle) => ok_variant(RuntimeValue::Opaque(
                        RuntimeOpaqueValue::AudioPlayback(handle),
                    )),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::AudioOutputSetGainMilli => {
            if args.len() != 2 {
                return Err("audio_output_set_gain_milli expects two arguments".to_string());
            }
            host.audio_output_set_gain_milli(
                expect_audio_device(args[0].clone(), "audio_output_set_gain_milli")?,
                expect_int(args[1].clone(), "audio_output_set_gain_milli")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::AudioPlaybackStop => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_stop")?,
                "audio_playback_stop",
            )?;
            Ok(match host.audio_playback_stop(playback) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            })
        }
        RuntimeIntrinsic::AudioPlaybackPause => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_pause")?,
                "audio_playback_pause",
            )?;
            host.audio_playback_pause(playback)?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::AudioPlaybackResume => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_resume")?,
                "audio_playback_resume",
            )?;
            host.audio_playback_resume(playback)?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::AudioPlaybackPlaying => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_playing")?,
                "audio_playback_playing",
            )?;
            Ok(RuntimeValue::Bool(host.audio_playback_playing(playback)?))
        }
        RuntimeIntrinsic::AudioPlaybackPaused => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_paused")?,
                "audio_playback_paused",
            )?;
            Ok(RuntimeValue::Bool(host.audio_playback_paused(playback)?))
        }
        RuntimeIntrinsic::AudioPlaybackFinished => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_finished")?,
                "audio_playback_finished",
            )?;
            Ok(RuntimeValue::Bool(host.audio_playback_finished(playback)?))
        }
        RuntimeIntrinsic::AudioPlaybackSetGainMilli => {
            if args.len() != 2 {
                return Err("audio_playback_set_gain_milli expects two arguments".to_string());
            }
            host.audio_playback_set_gain_milli(
                expect_audio_playback(args[0].clone(), "audio_playback_set_gain_milli")?,
                expect_int(args[1].clone(), "audio_playback_set_gain_milli")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::AudioPlaybackSetLooping => {
            if args.len() != 2 {
                return Err("audio_playback_set_looping expects two arguments".to_string());
            }
            host.audio_playback_set_looping(
                expect_audio_playback(args[0].clone(), "audio_playback_set_looping")?,
                expect_bool(args[1].clone(), "audio_playback_set_looping")?,
            )?;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::AudioPlaybackLooping => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_looping")?,
                "audio_playback_looping",
            )?;
            Ok(RuntimeValue::Bool(host.audio_playback_looping(playback)?))
        }
        RuntimeIntrinsic::AudioPlaybackPositionFrames => {
            let playback = expect_audio_playback(
                expect_single_arg(args, "audio_playback_position_frames")?,
                "audio_playback_position_frames",
            )?;
            Ok(RuntimeValue::Int(
                host.audio_playback_position_frames(playback)?,
            ))
        }
        RuntimeIntrinsic::ProcessExecStatusTry => {
            if args.len() != 2 {
                return Err("process_exec_status expects two arguments".to_string());
            }
            Ok(
                match host.process_exec_status(
                    &expect_str(args[0].clone(), "process_exec_status")?,
                    &expect_string_list(args[1].clone(), "process_exec_status")?,
                ) {
                    Ok(status) => ok_variant(RuntimeValue::Int(status)),
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::ProcessExecCaptureTry => {
            if args.len() != 2 {
                return Err("process_exec_capture expects two arguments".to_string());
            }
            Ok(
                match host.process_exec_capture(
                    &expect_str(args[0].clone(), "process_exec_capture")?,
                    &expect_string_list(args[1].clone(), "process_exec_capture")?,
                ) {
                    Ok((status, stdout, stderr, stdout_utf8, stderr_utf8)) => {
                        ok_variant(make_pair(
                            RuntimeValue::Int(status),
                            make_pair(
                                bytes_to_runtime_array(stdout),
                                make_pair(
                                    bytes_to_runtime_array(stderr),
                                    make_pair(
                                        RuntimeValue::Bool(stdout_utf8),
                                        RuntimeValue::Bool(stderr_utf8),
                                    ),
                                ),
                            ),
                        ))
                    }
                    Err(err) => err_variant(err),
                },
            )
        }
        RuntimeIntrinsic::TextLenBytes => {
            let text = expect_str(expect_single_arg(args, "text_len_bytes")?, "text_len_bytes")?;
            Ok(RuntimeValue::Int(host.text_len_bytes(&text)?))
        }
        RuntimeIntrinsic::TextByteAt => {
            if args.len() != 2 {
                return Err("text_byte_at expects two arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_byte_at")?;
            let index = expect_int(args[1].clone(), "text_byte_at")?;
            if index < 0 {
                return Err("text_byte_at index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Int(host.text_byte_at(&text, index as usize)?))
        }
        RuntimeIntrinsic::TextSliceBytes => {
            if args.len() != 3 {
                return Err("text_slice_bytes expects three arguments".to_string());
            }
            let text = expect_str(args[0].clone(), "text_slice_bytes")?;
            let start = expect_int(args[1].clone(), "text_slice_bytes")?;
            let end = expect_int(args[2].clone(), "text_slice_bytes")?;
            if start < 0 || end < 0 {
                return Err("text_slice_bytes bounds must be non-negative".to_string());
            }
            if end < start {
                return Err(
                    "text_slice_bytes end must be greater than or equal to start".to_string(),
                );
            }
            Ok(RuntimeValue::Str(host.text_slice_bytes(
                &text,
                start as usize,
                end as usize,
            )?))
        }
        RuntimeIntrinsic::TextStartsWith => {
            if args.len() != 2 {
                return Err("text_starts_with expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.text_starts_with(
                &expect_str(args[0].clone(), "text_starts_with")?,
                &expect_str(args[1].clone(), "text_starts_with")?,
            )?))
        }
        RuntimeIntrinsic::TextEndsWith => {
            if args.len() != 2 {
                return Err("text_ends_with expects two arguments".to_string());
            }
            Ok(RuntimeValue::Bool(host.text_ends_with(
                &expect_str(args[0].clone(), "text_ends_with")?,
                &expect_str(args[1].clone(), "text_ends_with")?,
            )?))
        }
        RuntimeIntrinsic::TextSplitLines => {
            let text = expect_str(
                expect_single_arg(args, "text_split_lines")?,
                "text_split_lines",
            )?;
            Ok(RuntimeValue::List(
                host.text_split_lines(&text)?
                    .into_iter()
                    .map(RuntimeValue::Str)
                    .collect(),
            ))
        }
        RuntimeIntrinsic::TextFromInt => {
            let value = expect_int(expect_single_arg(args, "text_from_int")?, "text_from_int")?;
            Ok(RuntimeValue::Str(host.text_from_int(value)?))
        }
        RuntimeIntrinsic::BytesFromStrUtf8 => {
            let text = expect_str(
                expect_single_arg(args, "bytes_from_str_utf8")?,
                "bytes_from_str_utf8",
            )?;
            Ok(bytes_to_runtime_array(host.bytes_from_str_utf8(&text)?))
        }
        RuntimeIntrinsic::BytesToStrUtf8 => {
            let bytes = expect_byte_array(
                expect_single_arg(args, "bytes_to_str_utf8")?,
                "bytes_to_str_utf8",
            )?;
            Ok(RuntimeValue::Str(host.bytes_to_str_utf8(&bytes)?))
        }
        RuntimeIntrinsic::BytesLen => {
            let bytes = expect_byte_array(expect_single_arg(args, "bytes_len")?, "bytes_len")?;
            Ok(RuntimeValue::Int(i64::try_from(bytes.len()).map_err(
                |_| "bytes length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::BytesAt => {
            if args.len() != 2 {
                return Err("bytes_at expects two arguments".to_string());
            }
            let bytes = expect_byte_array(args[0].clone(), "bytes_at")?;
            let index = expect_int(args[1].clone(), "bytes_at")?;
            if index < 0 {
                return Err("bytes_at index must be non-negative".to_string());
            }
            Ok(RuntimeValue::Int(i64::from(
                *bytes
                    .get(index as usize)
                    .ok_or_else(|| format!("bytes_at index `{index}` is out of bounds"))?,
            )))
        }
        RuntimeIntrinsic::BytesSlice => {
            if args.len() != 3 {
                return Err("bytes_slice expects three arguments".to_string());
            }
            let bytes = expect_byte_array(args[0].clone(), "bytes_slice")?;
            let start = expect_int(args[1].clone(), "bytes_slice")?;
            let end = expect_int(args[2].clone(), "bytes_slice")?;
            if start < 0 || end < 0 {
                return Err("bytes_slice bounds must be non-negative".to_string());
            }
            if end < start {
                return Err("bytes_slice end must be greater than or equal to start".to_string());
            }
            let slice = bytes
                .get(start as usize..end as usize)
                .ok_or_else(|| format!("bytes_slice `{start}..{end}` is out of bounds"))?;
            Ok(bytes_to_runtime_array(slice.iter().copied()))
        }
        RuntimeIntrinsic::BytesSha256Hex => {
            let bytes = expect_byte_array(
                expect_single_arg(args, "bytes_sha256_hex")?,
                "bytes_sha256_hex",
            )?;
            Ok(RuntimeValue::Str(host.bytes_sha256_hex(&bytes)?))
        }
        RuntimeIntrinsic::ResultOk => match args.len() {
            0 => Ok(ok_variant(RuntimeValue::Unit)),
            1 => Ok(ok_variant(
                args.into_iter().next().unwrap_or(RuntimeValue::Unit),
            )),
            _ => Err("Result.Ok expects zero or one argument".to_string()),
        },
        RuntimeIntrinsic::ResultErr => {
            let value = expect_str(expect_single_arg(args, "Result.Err")?, "Result.Err")?;
            Ok(err_variant(value))
        }
        RuntimeIntrinsic::ListNew => {
            if !args.is_empty() {
                return Err("list_new expects zero arguments".to_string());
            }
            Ok(RuntimeValue::List(Vec::new()))
        }
        RuntimeIntrinsic::ArrayNew => {
            if args.len() != 2 {
                return Err("array_new expects two arguments".to_string());
            }
            let len = expect_int(args[0].clone(), "array_new")?;
            if len < 0 {
                return Err("array_new length must be non-negative".to_string());
            }
            Ok(RuntimeValue::Array(vec![args[1].clone(); len as usize]))
        }
        RuntimeIntrinsic::ArrayLen => {
            let value = expect_single_arg(args, "array_len")?;
            let RuntimeValue::Array(values) = value else {
                return Err("array_len expects Array".to_string());
            };
            Ok(RuntimeValue::Int(i64::try_from(values.len()).map_err(
                |_| "array length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::ArrayFromList => {
            let value = expect_single_arg(args, "array_from_list")?;
            let RuntimeValue::List(values) = value else {
                return Err("array_from_list expects List".to_string());
            };
            Ok(RuntimeValue::Array(values))
        }
        RuntimeIntrinsic::ArrayToList => {
            let value = expect_single_arg(args, "array_to_list")?;
            let RuntimeValue::Array(values) = value else {
                return Err("array_to_list expects Array".to_string());
            };
            Ok(RuntimeValue::List(values))
        }
        RuntimeIntrinsic::EcsSetSingleton => {
            let key = require_runtime_type_key(type_args, "ecs_set_singleton")?;
            let value = expect_single_arg(args, "ecs_set_singleton")?;
            ecs_slot_mut(state, &key).insert(0, value);
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::EcsHasSingleton => {
            let key = require_runtime_type_key(type_args, "ecs_has_singleton")?;
            if !args.is_empty() {
                return Err("ecs_has_singleton expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Bool(ecs_slot(state, &key, 0).is_some()))
        }
        RuntimeIntrinsic::EcsGetSingleton => {
            let key = require_runtime_type_key(type_args, "ecs_get_singleton")?;
            if !args.is_empty() {
                return Err("ecs_get_singleton expects zero arguments".to_string());
            }
            ecs_slot(state, &key, 0)
                .cloned()
                .ok_or_else(|| format!("missing singleton component for `{}`", key.join(", ")))
        }
        RuntimeIntrinsic::EcsSpawn => {
            if !args.is_empty() {
                return Err("ecs_spawn expects zero arguments".to_string());
            }
            let entity = if state.next_entity_id <= 0 {
                1
            } else {
                state.next_entity_id
            };
            state.next_entity_id = entity + 1;
            state.live_entities.insert(entity);
            Ok(RuntimeValue::Int(entity))
        }
        RuntimeIntrinsic::EcsDespawn => {
            let entity = expect_entity_id(expect_single_arg(args, "ecs_despawn")?, "ecs_despawn")?;
            if entity == 0 {
                return Err("ecs_despawn cannot target singleton entity 0".to_string());
            }
            if !state.live_entities.remove(&entity) {
                return Err(format!("ecs_despawn unknown entity `{entity}`"));
            }
            for slots in state.component_slots.values_mut() {
                slots.remove(&entity);
            }
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::EcsSetComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_set_component_at")?;
            if args.len() != 2 {
                return Err("ecs_set_component_at expects two arguments".to_string());
            }
            let entity = expect_entity_id(args[0].clone(), "ecs_set_component_at")?;
            if !ecs_entity_exists(state, entity) {
                return Err(format!("ecs_set_component_at unknown entity `{entity}`"));
            }
            ecs_slot_mut(state, &key).insert(entity, args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::EcsHasComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_has_component_at")?;
            let entity = expect_entity_id(
                expect_single_arg(args, "ecs_has_component_at")?,
                "ecs_has_component_at",
            )?;
            if !ecs_entity_exists(state, entity) {
                return Ok(RuntimeValue::Bool(false));
            }
            Ok(RuntimeValue::Bool(ecs_slot(state, &key, entity).is_some()))
        }
        RuntimeIntrinsic::EcsGetComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_get_component_at")?;
            let entity = expect_entity_id(
                expect_single_arg(args, "ecs_get_component_at")?,
                "ecs_get_component_at",
            )?;
            if !ecs_entity_exists(state, entity) {
                return Err(format!("ecs_get_component_at unknown entity `{entity}`"));
            }
            ecs_slot(state, &key, entity).cloned().ok_or_else(|| {
                format!(
                    "missing component `{}` at entity `{entity}`",
                    key.join(", ")
                )
            })
        }
        RuntimeIntrinsic::EcsRemoveComponentAt => {
            let key = require_runtime_type_key(type_args, "ecs_remove_component_at")?;
            let entity = expect_entity_id(
                expect_single_arg(args, "ecs_remove_component_at")?,
                "ecs_remove_component_at",
            )?;
            if !ecs_entity_exists(state, entity) {
                return Err(format!("ecs_remove_component_at unknown entity `{entity}`"));
            }
            ecs_slot_mut(state, &key).remove(&entity);
            Ok(RuntimeValue::Unit)
        }
    }
}

fn match_pattern(
    pattern: &ParsedMatchPattern,
    value: &RuntimeValue,
    bindings: &mut BTreeMap<String, RuntimeValue>,
) -> bool {
    match pattern {
        ParsedMatchPattern::Wildcard => true,
        ParsedMatchPattern::Name(name) => {
            bindings.insert(name.clone(), value.clone());
            true
        }
        ParsedMatchPattern::Literal(text) => match value {
            RuntimeValue::Str(value) => value == text,
            _ => false,
        },
        ParsedMatchPattern::Variant { path, args } => match value {
            RuntimeValue::Variant { name, payload } => {
                if !variant_name_matches(name, path) || payload.len() != args.len() {
                    return false;
                }
                let mut nested = bindings.clone();
                for (pattern, payload) in args.iter().zip(payload.iter()) {
                    if !match_pattern(pattern, payload, &mut nested) {
                        return false;
                    }
                }
                *bindings = nested;
                true
            }
            _ => false,
        },
    }
}

fn apply_binary_op(
    op: ParsedBinaryOp,
    left: RuntimeValue,
    right: RuntimeValue,
) -> Result<RuntimeValue, String> {
    match op {
        ParsedBinaryOp::EqEq => Ok(RuntimeValue::Bool(left == right)),
        ParsedBinaryOp::NotEq => Ok(RuntimeValue::Bool(left != right)),
        ParsedBinaryOp::Lt => Ok(RuntimeValue::Bool(
            expect_int(left, "<")? < expect_int(right, "<")?,
        )),
        ParsedBinaryOp::LtEq => Ok(RuntimeValue::Bool(
            expect_int(left, "<=")? <= expect_int(right, "<=")?,
        )),
        ParsedBinaryOp::Gt => Ok(RuntimeValue::Bool(
            expect_int(left, ">")? > expect_int(right, ">")?,
        )),
        ParsedBinaryOp::GtEq => Ok(RuntimeValue::Bool(
            expect_int(left, ">=")? >= expect_int(right, ">=")?,
        )),
        ParsedBinaryOp::BitOr => Ok(RuntimeValue::Int(
            expect_int(left, "|")? | expect_int(right, "|")?,
        )),
        ParsedBinaryOp::BitXor => Ok(RuntimeValue::Int(
            expect_int(left, "^")? ^ expect_int(right, "^")?,
        )),
        ParsedBinaryOp::BitAnd => Ok(RuntimeValue::Int(
            expect_int(left, "&")? & expect_int(right, "&")?,
        )),
        ParsedBinaryOp::Shl => Ok(RuntimeValue::Int(
            expect_int(left, "<<")? << expect_int(right, "<<")?,
        )),
        ParsedBinaryOp::Shr => Ok(RuntimeValue::Int(
            expect_int(left, "shr")? >> expect_int(right, "shr")?,
        )),
        ParsedBinaryOp::Add => Ok(RuntimeValue::Int(
            expect_int(left, "+")? + expect_int(right, "+")?,
        )),
        ParsedBinaryOp::Sub => Ok(RuntimeValue::Int(
            expect_int(left, "-")? - expect_int(right, "-")?,
        )),
        ParsedBinaryOp::Mul => Ok(RuntimeValue::Int(
            expect_int(left, "*")? * expect_int(right, "*")?,
        )),
        ParsedBinaryOp::Div => Ok(RuntimeValue::Int(
            expect_int(left, "/")? / expect_int(right, "/")?,
        )),
        ParsedBinaryOp::Mod => Ok(RuntimeValue::Int(
            expect_int(left, "%")? % expect_int(right, "%")?,
        )),
        ParsedBinaryOp::Or | ParsedBinaryOp::And => {
            Err("logical ops must short-circuit before apply".to_string())
        }
    }
}

fn eval_expr(
    expr: &ParsedExpr,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match expr {
        ParsedExpr::Int(value) => Ok(RuntimeValue::Int(*value)),
        ParsedExpr::Bool(value) => Ok(RuntimeValue::Bool(*value)),
        ParsedExpr::Str(value) => Ok(RuntimeValue::Str(value.clone())),
        ParsedExpr::Pair { left, right } => Ok(RuntimeValue::Pair(
            Box::new(eval_expr(
                left,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?),
            Box::new(eval_expr(
                right,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?),
        )),
        ParsedExpr::Collection { items } => Ok(RuntimeValue::List(
            items
                .iter()
                .map(|item| {
                    eval_expr(
                        item,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )
                })
                .collect::<Result<Vec<_>, String>>()?,
        )),
        ParsedExpr::Match { subject, arms } => eval_match_expr(
            subject,
            arms,
            plan,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::Path(segments) if segments.len() == 1 => lookup_local(scopes, &segments[0])
            .map(|local| local.value.clone())
            .ok_or_else(|| format!("unsupported runtime value path `{}`", segments[0])),
        ParsedExpr::Path(segments) => Err(format!(
            "unsupported runtime value path `{}`",
            segments.join(".")
        )),
        ParsedExpr::Member { expr, member } => {
            let base = eval_expr(
                expr,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            eval_member_value(base, member)
        }
        ParsedExpr::Generic { expr, .. } => eval_expr(
            expr,
            plan,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::Unary { op, expr } => {
            let value = eval_expr(
                expr,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            match op {
                ParsedUnaryOp::Neg => Ok(RuntimeValue::Int(-expect_int(value, "unary -")?)),
                ParsedUnaryOp::Not => Ok(RuntimeValue::Bool(!expect_bool(value, "not")?)),
                ParsedUnaryOp::BitNot => Ok(RuntimeValue::Int(!expect_int(value, "~")?)),
            }
        }
        ParsedExpr::Binary { left, op, right } => match op {
            ParsedBinaryOp::Or => {
                let left = expect_bool(
                    eval_expr(
                        left,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    "or",
                )?;
                if left {
                    Ok(RuntimeValue::Bool(true))
                } else {
                    Ok(RuntimeValue::Bool(expect_bool(
                        eval_expr(
                            right,
                            plan,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        "or",
                    )?))
                }
            }
            ParsedBinaryOp::And => {
                let left = expect_bool(
                    eval_expr(
                        left,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    "and",
                )?;
                if !left {
                    Ok(RuntimeValue::Bool(false))
                } else {
                    Ok(RuntimeValue::Bool(expect_bool(
                        eval_expr(
                            right,
                            plan,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        "and",
                    )?))
                }
            }
            other => {
                let left = eval_expr(
                    left,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let right = eval_expr(
                    right,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                apply_binary_op(*other, left, right)
            }
        },
        ParsedExpr::Phrase {
            subject,
            args,
            qualifier,
            attached,
        } => {
            if qualifier == "std.io.print"
                || qualifier == "std.io.print_line"
                || qualifier == "std.io.eprint"
                || qualifier == "std.io.eprint_line"
            {
                if !args.is_empty() {
                    return Err(format!("{qualifier} expects zero explicit arguments"));
                }
                if !attached.is_empty() {
                    return Err(format!("{qualifier} does not support attached arguments"));
                }
                let intrinsic = match qualifier.as_str() {
                    "std.io.print" => RuntimeIntrinsic::IoPrint,
                    "std.io.print_line" => RuntimeIntrinsic::IoPrintLine,
                    "std.io.eprint" => RuntimeIntrinsic::IoEprint,
                    "std.io.eprint_line" => RuntimeIntrinsic::IoEprintLine,
                    _ => unreachable!(),
                };
                let value = eval_expr(
                    subject,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                return execute_runtime_intrinsic(intrinsic, &[], vec![value], plan, state, host);
            }
            if qualifier != "call" {
                return eval_qualifier(
                    subject,
                    args,
                    qualifier,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                );
            }
            let callable = resolve_callable_path(subject, aliases)
                .ok_or_else(|| format!("unsupported runtime callable `{subject:?}`"))?;
            let type_args =
                resolve_runtime_type_args(&extract_generic_type_args(subject), type_bindings);
            let routine_index = resolve_routine_index(plan, current_module_id, &callable);
            let intrinsic = if routine_index.is_none() {
                resolve_runtime_intrinsic_path(&callable)
            } else {
                None
            };
            if routine_index.is_none() && intrinsic.is_none() {
                if let Some(record) = try_construct_record_value(
                    &callable,
                    args,
                    attached,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )? {
                    return Ok(record);
                }
                if let Some(variant) = try_construct_variant_value(
                    &callable,
                    args,
                    attached,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )? {
                    return Ok(variant);
                }
                return Err(format!(
                    "unsupported runtime callable `{}`",
                    callable.join(".")
                ));
            }
            let call_args = collect_call_args(
                args,
                attached,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            if let Some(intrinsic) = intrinsic {
                if call_args.iter().any(|arg| arg.name.is_some()) {
                    return Err(format!(
                        "runtime intrinsic `{}` does not yet support named-only fallback binding",
                        callable.join(".")
                    ));
                }
                let values = call_args.into_iter().map(|arg| arg.value).collect();
                return execute_runtime_intrinsic(intrinsic, &type_args, values, plan, state, host);
            }
            let routine_index = routine_index.expect("routine index should exist");
            let routine = plan
                .routines
                .get(routine_index)
                .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
            let values = bind_call_args_for_routine(routine, call_args)?;
            execute_routine_with_state(plan, routine_index, type_args, values, state, host)
        }
    }
}

fn apply_assign(
    scopes: &mut Vec<RuntimeScope>,
    target: &ParsedAssignTarget,
    op: ParsedAssignOp,
    value: RuntimeValue,
) -> Result<(), String> {
    let ParsedAssignTarget::Name(name) = target else {
        return Err(format!(
            "unsupported runtime assignment target `{target:?}`"
        ));
    };
    let local = lookup_local_mut(scopes, name)
        .ok_or_else(|| format!("runtime assignment target `{name}` is unresolved"))?;
    if !local.mutable {
        return Err(format!("runtime assignment target `{name}` is not mutable"));
    }
    local.value = match op {
        ParsedAssignOp::Assign => value,
        ParsedAssignOp::AddAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "+=")? + expect_int(value, "+=")?)
        }
        ParsedAssignOp::SubAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "-=")? - expect_int(value, "-=")?)
        }
        ParsedAssignOp::MulAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "*=")? * expect_int(value, "*=")?)
        }
        ParsedAssignOp::DivAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "/=")? / expect_int(value, "/=")?)
        }
        ParsedAssignOp::ModAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "%=")? % expect_int(value, "%=")?)
        }
        ParsedAssignOp::BitAndAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "&=")? & expect_int(value, "&=")?)
        }
        ParsedAssignOp::BitOrAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "|=")? | expect_int(value, "|=")?)
        }
        ParsedAssignOp::BitXorAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "^=")? ^ expect_int(value, "^=")?)
        }
        ParsedAssignOp::ShlAssign => {
            RuntimeValue::Int(expect_int(local.value.clone(), "<<=")? << expect_int(value, "<<=")?)
        }
        ParsedAssignOp::ShrAssign => RuntimeValue::Int(
            expect_int(local.value.clone(), "shr=")? >> expect_int(value, "shr=")?,
        ),
    };
    Ok(())
}

fn run_scope_defers(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    let deferred = scopes
        .last_mut()
        .ok_or_else(|| "runtime scope stack is empty".to_string())?
        .deferred
        .drain(..)
        .rev()
        .collect::<Vec<_>>();
    for expr in deferred {
        let _ = eval_expr(
            &expr,
            plan,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
    }
    Ok(())
}

fn execute_scoped_block(
    statements: &[ParsedStmt],
    scopes: &mut Vec<RuntimeScope>,
    scope: RuntimeScope,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<FlowSignal, String> {
    scopes.push(scope);
    let result = execute_statements(
        statements,
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    );
    let defer_result = run_scope_defers(
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    );
    scopes.pop();
    defer_result?;
    result
}

fn execute_statements(
    statements: &[ParsedStmt],
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<FlowSignal, String> {
    for statement in statements {
        let signal = match statement {
            ParsedStmt::Let {
                mutable,
                name,
                value,
            } => {
                let value = eval_expr(
                    value,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let current_scope = scopes
                    .last_mut()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                current_scope.locals.insert(
                    name.clone(),
                    RuntimeLocal {
                        mutable: *mutable,
                        value,
                    },
                );
                FlowSignal::Next
            }
            ParsedStmt::Expr(expr) => {
                let _ = eval_expr(
                    expr,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                FlowSignal::Next
            }
            ParsedStmt::Return(expr) => FlowSignal::Return(match expr {
                Some(expr) => eval_expr(
                    expr,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
                None => RuntimeValue::Unit,
            }),
            ParsedStmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if expect_bool(
                    eval_expr(
                        condition,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    "if condition",
                )? {
                    execute_scoped_block(
                        then_branch,
                        scopes,
                        RuntimeScope::default(),
                        plan,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?
                } else if else_branch.is_empty() {
                    FlowSignal::Next
                } else {
                    execute_scoped_block(
                        else_branch,
                        scopes,
                        RuntimeScope::default(),
                        plan,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?
                }
            }
            ParsedStmt::While { condition, body } => loop {
                if !expect_bool(
                    eval_expr(
                        condition,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                    "while condition",
                )? {
                    break FlowSignal::Next;
                }
                match execute_scoped_block(
                    body,
                    scopes,
                    RuntimeScope::default(),
                    plan,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )? {
                    FlowSignal::Next | FlowSignal::Continue => {}
                    FlowSignal::Break => break FlowSignal::Next,
                    FlowSignal::Return(value) => break FlowSignal::Return(value),
                }
            },
            ParsedStmt::For {
                binding,
                iterable,
                body,
            } => {
                let values = into_iterable_values(eval_expr(
                    iterable,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?)?;
                let mut loop_signal = FlowSignal::Next;
                for value in values {
                    let mut scope = RuntimeScope::default();
                    scope.locals.insert(
                        binding.clone(),
                        RuntimeLocal {
                            mutable: false,
                            value,
                        },
                    );
                    match execute_scoped_block(
                        body,
                        scopes,
                        scope,
                        plan,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )? {
                        FlowSignal::Next | FlowSignal::Continue => {}
                        FlowSignal::Break => {
                            loop_signal = FlowSignal::Next;
                            break;
                        }
                        FlowSignal::Return(value) => {
                            loop_signal = FlowSignal::Return(value);
                            break;
                        }
                    }
                }
                loop_signal
            }
            ParsedStmt::Defer(expr) => {
                scopes
                    .last_mut()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?
                    .deferred
                    .push(expr.clone());
                FlowSignal::Next
            }
            ParsedStmt::Break => FlowSignal::Break,
            ParsedStmt::Continue => FlowSignal::Continue,
            ParsedStmt::Assign { target, op, value } => {
                let value = eval_expr(
                    value,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                apply_assign(scopes, target, *op, value)?;
                FlowSignal::Next
            }
        };
        if signal != FlowSignal::Next {
            return Ok(signal);
        }
    }
    Ok(FlowSignal::Next)
}

fn execute_routine_with_state(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    type_args: Vec<String>,
    args: Vec<RuntimeValue>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let routine = plan
        .routines
        .get(routine_index)
        .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
    if let Some(intrinsic_impl) = &routine.intrinsic_impl {
        let intrinsic = resolve_runtime_intrinsic_impl(intrinsic_impl).ok_or_else(|| {
            format!(
                "unsupported runtime intrinsic `{intrinsic_impl}` for `{}`",
                routine.symbol_name
            )
        })?;
        return execute_runtime_intrinsic(intrinsic, &type_args, args, plan, state, host);
    }
    if routine.is_async {
        return Err(format!(
            "async routine `{}` is not executable in the current runtime lane",
            routine.symbol_name
        ));
    }
    if args.len() != routine.params.len() {
        return Err(format!(
            "routine `{}` expected {} arguments, got {}",
            routine.symbol_name,
            routine.params.len(),
            args.len()
        ));
    }
    if !routine.type_params.is_empty() && type_args.len() != routine.type_params.len() {
        return Err(format!(
            "routine `{}` expected {} type arguments, got {}",
            routine.symbol_name,
            routine.type_params.len(),
            type_args.len()
        ));
    }
    let aliases = plan
        .module_aliases
        .get(&routine.module_id)
        .cloned()
        .unwrap_or_default();
    let type_bindings = routine
        .type_params
        .iter()
        .cloned()
        .zip(type_args)
        .collect::<RuntimeTypeBindings>();
    let mut initial_scope = RuntimeScope::default();
    for (param, value) in routine.params.iter().zip(args) {
        initial_scope.locals.insert(
            param.name.clone(),
            RuntimeLocal {
                mutable: param.mode.as_deref() == Some("edit"),
                value,
            },
        );
    }
    let mut scopes = Vec::new();
    match execute_scoped_block(
        &routine.statements,
        &mut scopes,
        initial_scope,
        plan,
        &routine.module_id,
        &aliases,
        &type_bindings,
        state,
        host,
    )? {
        FlowSignal::Next => Ok(RuntimeValue::Unit),
        FlowSignal::Return(value) => Ok(value),
        FlowSignal::Break => Err("break escaped the top-level routine".to_string()),
        FlowSignal::Continue => Err("continue escaped the top-level routine".to_string()),
    }
}

#[cfg(test)]
fn execute_routine(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    args: Vec<RuntimeValue>,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let mut state = RuntimeExecutionState::default();
    execute_routine_with_state(plan, routine_index, Vec::new(), args, &mut state, host)
}

pub fn execute_main(plan: &RuntimePackagePlan, host: &mut dyn RuntimeHost) -> Result<i32, String> {
    let entry = plan
        .main_entrypoint()
        .ok_or_else(|| format!("package `{}` has no main entrypoint", plan.package_name))?;
    let mut state = RuntimeExecutionState::default();
    let value = execute_routine_with_state(
        plan,
        entry.routine_index,
        Vec::new(),
        Vec::new(),
        &mut state,
        host,
    )?;
    match value {
        RuntimeValue::Int(value) => i32::try_from(value)
            .map_err(|_| format!("main return value `{value}` does not fit in i32")),
        RuntimeValue::Unit => Ok(0),
        RuntimeValue::Bool(_)
        | RuntimeValue::Str(_)
        | RuntimeValue::Pair(_, _)
        | RuntimeValue::Array(_)
        | RuntimeValue::List(_)
        | RuntimeValue::Opaque(_)
        | RuntimeValue::Record { .. }
        | RuntimeValue::Variant { .. } => {
            Err("main must return Int or Unit in the current runtime lane".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BufferedEvent, BufferedFrameInput, BufferedHost, RuntimeHost, RuntimeOpaqueValue,
        RuntimeValue, execute_main, execute_routine, load_package_plan, plan_from_artifact,
        resolve_routine_index,
    };
    use arcana_aot::{
        AOT_INTERNAL_FORMAT, AotEntrypointArtifact, AotPackageArtifact, AotPackageModuleArtifact,
        AotRoutineArtifact, render_package_artifact,
    };
    use arcana_frontend::{
        check_workspace_graph, compute_member_fingerprints_for_checked_workspace,
    };
    use arcana_package::{execute_build, load_workspace_graph, plan_build, plan_workspace};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_artifact_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        std::env::temp_dir().join(format!("arcana_runtime_{label}_{nanos}.toml"))
    }

    fn temp_workspace_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        repo_root()
            .join("target")
            .join(format!("arcana_runtime_{label}_{nanos}"))
    }

    fn repo_root() -> PathBuf {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir
            .parent()
            .and_then(Path::parent)
            .expect("workspace root should exist")
            .to_path_buf()
    }

    fn write_file(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directories should be created");
        }
        fs::write(path, text).expect("file should write");
    }

    fn synthetic_window_canvas_host(fixture_root: &Path) -> BufferedHost {
        let cwd = fixture_root.to_string_lossy().replace('\\', "/");
        BufferedHost {
            cwd: cwd.clone(),
            sandbox_root: cwd,
            monotonic_now_ms: 100,
            monotonic_step_ms: 5,
            next_frame_events: vec![
                BufferedEvent {
                    kind: 3,
                    a: 1,
                    b: 0,
                },
                BufferedEvent {
                    kind: 4,
                    a: 65,
                    b: 0,
                },
            ],
            next_frame_input: BufferedFrameInput {
                key_down: vec![65],
                key_pressed: vec![65],
                mouse_pos: (40, 50),
                mouse_in_window: true,
                ..BufferedFrameInput::default()
            },
            ..BufferedHost::default()
        }
    }

    fn synthetic_audio_host(fixture_root: &Path) -> BufferedHost {
        let cwd = fixture_root.to_string_lossy().replace('\\', "/");
        BufferedHost {
            cwd: cwd.clone(),
            sandbox_root: cwd,
            ..BufferedHost::default()
        }
    }

    fn write_host_core_workspace(destination: &Path) {
        write_file(
            &destination.join("book.toml"),
            "name = \"runtime_host_core\"\nkind = \"app\"\n",
        );
        write_file(
            &destination.join("src").join("shelf.arc"),
            concat!(
                "import std.collections.list\n",
                "import std.fs\n",
                "import std.io\n",
                "import std.path\n",
                "import std.text\n",
                "use std.result.Result\n",
                "\n",
                "fn list_arc_files(root: Str) -> List[Str]:\n",
                "    let mut pending = std.collections.list.new[Str] :: :: call\n",
                "    let mut files = std.collections.list.new[Str] :: :: call\n",
                "    pending :: root :: push\n",
                "    while (pending :: :: len) > 0:\n",
                "        let path = pending :: :: pop\n",
                "        if std.fs.is_dir :: path :: call:\n",
                "            let mut entries = match (std.fs.list_dir :: path :: call):\n",
                "                Result.Ok(found) => found\n",
                "                Result.Err(_) => std.collections.list.new[Str] :: :: call\n",
                "            while (entries :: :: len) > 0:\n",
                "                pending :: (entries :: :: pop) :: push\n",
                "            continue\n",
                "        if (std.path.ext :: path :: call) != \"arc\":\n",
                "            continue\n",
                "        files :: path :: push\n",
                "    return files\n",
                "\n",
                "fn read_text_or_empty(path: Str) -> Str:\n",
                "    return match (std.fs.read_text :: path :: call):\n",
                "        Result.Ok(text) => text\n",
                "        Result.Err(_) => \"\"\n",
                "\n",
                "fn main() -> Int:\n",
                "    let root = std.path.cwd :: :: call\n",
                "    let mut files = list_arc_files :: root :: call\n",
                "    let mut count = 0\n",
                "    let mut checksum = 0\n",
                "    while (files :: :: len) > 0:\n",
                "        let file = files :: :: pop\n",
                "        let text = read_text_or_empty :: file :: call\n",
                "        let size = std.text.len_bytes :: text :: call\n",
                "        std.io.print[Str] :: file :: call\n",
                "        count += 1\n",
                "        checksum = ((checksum * 131) + size + 7) % 2147483647\n",
                "    let report_dir = std.path.join :: root, \".arcana\" :: call\n",
                "    let logs_dir = std.path.join :: report_dir, \"logs\" :: call\n",
                "    let report_path = std.path.join :: logs_dir, \"host_core_report.txt\" :: call\n",
                "    std.fs.mkdir_all :: logs_dir :: call\n",
                "    std.fs.write_text :: report_path, \"Arcana Runtime Host Core v1\\n\" :: call\n",
                "    std.io.print[Int] :: count :: call\n",
                "    std.io.print[Int] :: checksum :: call\n",
                "    return 0\n",
            ),
        );
        write_file(
            &destination.join("src").join("types.arc"),
            "// test types\n",
        );
    }

    fn sample_return_artifact() -> AotPackageArtifact {
        AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT,
            package_name: "hello".to_string(),
            root_module_id: "hello".to_string(),
            direct_deps: vec!["std".to_string()],
            module_count: 1,
            dependency_edge_count: 1,
            dependency_rows: vec!["source=hello:import:std.io:".to_string()],
            exported_surface_rows: vec!["module=hello:export:fn:fn main() -> Int:".to_string()],
            runtime_requirements: vec!["std.io".to_string()],
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "hello".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: true,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "hello".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: true,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main() -> Int:".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                statement_rows: vec![
                    "stmt(core=return(int(7)),forewords=[],rollups=[])".to_string(),
                ],
            }],
            modules: vec![AotPackageModuleArtifact {
                module_id: "hello".to_string(),
                symbol_count: 1,
                item_count: 2,
                line_count: 2,
                non_empty_line_count: 2,
                directive_rows: vec!["module=hello:import:std.io:".to_string()],
                lang_item_rows: Vec::new(),
                exported_surface_rows: vec!["module=hello:export:fn:fn main() -> Int:".to_string()],
            }],
        }
    }

    fn sample_print_artifact() -> AotPackageArtifact {
        AotPackageArtifact {
            format: AOT_INTERNAL_FORMAT,
            package_name: "hello".to_string(),
            root_module_id: "hello".to_string(),
            direct_deps: vec!["std".to_string()],
            module_count: 1,
            dependency_edge_count: 2,
            dependency_rows: vec![
                "source=hello:import:std.io:".to_string(),
                "source=hello:use:std.io:io".to_string(),
            ],
            exported_surface_rows: vec![],
            runtime_requirements: vec!["std.io".to_string()],
            entrypoints: vec![AotEntrypointArtifact {
                module_id: "hello".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                is_async: false,
                exported: false,
            }],
            routines: vec![AotRoutineArtifact {
                module_id: "hello".to_string(),
                symbol_name: "main".to_string(),
                symbol_kind: "fn".to_string(),
                exported: false,
                is_async: false,
                type_param_rows: Vec::new(),
                behavior_attr_rows: Vec::new(),
                param_rows: Vec::new(),
                signature_row: "fn main():".to_string(),
                intrinsic_impl: None,
                foreword_rows: Vec::new(),
                rollup_rows: Vec::new(),
                statement_rows: vec![
                    "stmt(core=expr(phrase(subject=generic(expr=member(path(io), print),types=[Str]),args=[str(\"\\\"hello, arcana\\\"\")],qualifier=call,attached=[])),forewords=[],rollups=[])".to_string(),
                ],
            }],
            modules: vec![AotPackageModuleArtifact {
                module_id: "hello".to_string(),
                symbol_count: 1,
                item_count: 4,
                line_count: 4,
                non_empty_line_count: 4,
                directive_rows: vec![
                    "module=hello:import:std.io:".to_string(),
                    "module=hello:use:std.io:io".to_string(),
                ],
                lang_item_rows: Vec::new(),
                exported_surface_rows: Vec::new(),
            }],
        }
    }

    #[test]
    fn plan_from_artifact_links_entrypoints_to_routines() {
        let plan =
            plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
        assert_eq!(plan.entrypoints.len(), 1);
        assert_eq!(plan.routines.len(), 1);
        assert_eq!(plan.entrypoints[0].routine_index, 0);
        assert_eq!(
            plan.main_entrypoint()
                .map(|entry| entry.symbol_name.as_str()),
            Some("main")
        );
    }

    #[test]
    fn load_package_plan_reads_rendered_backend_artifact() {
        let path = temp_artifact_path("load");
        let rendered = format!(
            "member = \"hello\"\nkind = \"app\"\nfingerprint = \"fp\"\napi_fingerprint = \"api\"\n{}",
            render_package_artifact(&sample_return_artifact())
        );
        fs::write(&path, rendered).expect("artifact should write");
        let plan = load_package_plan(&path).expect("runtime plan should load");
        assert_eq!(plan.package_name, "hello");
        assert_eq!(plan.runtime_requirements, vec!["std.io".to_string()]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn execute_main_returns_exit_code() {
        let plan =
            plan_from_artifact(&sample_return_artifact()).expect("runtime plan should build");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");
        assert_eq!(code, 7);
        assert!(host.stdout.is_empty());
    }

    #[test]
    fn execute_main_prints_hello() {
        let plan = plan_from_artifact(&sample_print_artifact()).expect("runtime plan should build");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");
        assert_eq!(code, 0);
        assert_eq!(host.stdout, vec!["hello, arcana".to_string()]);
    }

    #[test]
    fn execute_main_runs_counter_style_workspace_artifact() {
        let dir = temp_workspace_dir("counter");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_counter\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.io\n",
                "use std.io as io\n",
                "fn main() -> Int:\n",
                "    let mut i = 0\n",
                "    while i < 3:\n",
                "        io.print[Int] :: i :: call\n",
                "        i += 1\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_counter")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            host.stdout,
            vec!["0".to_string(), "1".to_string(), "2".to_string()]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_routine_calls_with_std_args() {
        let dir = temp_workspace_dir("args");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_args\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.args\n",
                "import std.io\n",
                "fn add_one(value: Int) -> Int:\n",
                "    return value + 1\n",
                "fn main() -> Int:\n",
                "    let argc = std.args.count :: :: call\n",
                "    let total = add_one :: argc :: call\n",
                "    std.io.print[Int] :: total :: call\n",
                "    if argc > 0:\n",
                "        let first = std.args.get :: 0 :: call\n",
                "        std.io.print[Str] :: first :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_args")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost {
            args: vec!["alpha.arc".to_string(), "beta.arc".to_string()],
            ..BufferedHost::default()
        };
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(host.stdout, vec!["3".to_string(), "alpha.arc".to_string()]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_text_routine() {
        let dir = temp_workspace_dir("std_text");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_text\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.io\n",
                "import std.text\n",
                "fn main() -> Int:\n",
                "    std.io.print[Int] :: (std.text.find :: \"abc\", 0, \"b\" :: call) :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_text")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(host.stdout, vec!["1".to_string()]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_array_routines() {
        let dir = temp_workspace_dir("std_array");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_array\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.collections.array\n",
                "import std.collections.list\n",
                "import std.io\n",
                "fn main() -> Int:\n",
                "    let mut values = std.collections.list.new[Int] :: :: call\n",
                "    values :: 4 :: push\n",
                "    values :: 9 :: push\n",
                "    let arr = std.collections.array.from_list[Int] :: values :: call\n",
                "    let mut sum = 0\n",
                "    for value in arr:\n",
                "        sum += value\n",
                "    std.io.print[Int] :: (arr :: :: len) :: call\n",
                "    std.io.print[Int] :: sum :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_array")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(host.stdout, vec!["2".to_string(), "13".to_string()]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_host_text_bytes_io_env_routines() {
        let dir = temp_workspace_dir("std_host_misc");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_host_misc\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.bytes\n",
                "import std.env\n",
                "import std.io\n",
                "import std.text\n",
                "use std.result.Result\n",
                "fn main() -> Int:\n",
                "    let label = std.env.get_or :: \"ARCANA_LABEL\", \"unset\" :: call\n",
                "    let input = match (std.io.read_line :: :: call):\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(err) => err\n",
                "    let lines = std.text.split_lines :: \"alpha\\r\\nbeta\\n\" :: call\n",
                "    let bytes = std.bytes.from_str_utf8 :: input :: call\n",
                "    let mid = std.bytes.slice :: bytes, 1, 4 :: call\n",
                "    std.io.flush_stdout :: :: call\n",
                "    std.io.flush_stderr :: :: call\n",
                "    std.io.print[Str] :: label :: call\n",
                "    std.io.print[Bool] :: (std.text.starts_with :: input, \"he\" :: call) :: call\n",
                "    std.io.print[Bool] :: (std.text.ends_with :: input, \"lo\" :: call) :: call\n",
                "    std.io.print[Int] :: (lines :: :: len) :: call\n",
                "    std.io.print[Str] :: (std.text.from_int :: (std.bytes.len :: bytes :: call) :: call) :: call\n",
                "    std.io.print[Int] :: (std.bytes.at :: bytes, 1 :: call) :: call\n",
                "    std.io.print[Str] :: (std.bytes.to_str_utf8 :: mid :: call) :: call\n",
                "    std.io.print[Str] :: (std.bytes.sha256_hex :: bytes :: call) :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_host_misc")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost {
            stdin: vec!["hello".to_string()],
            env: std::collections::BTreeMap::from([(
                "ARCANA_LABEL".to_string(),
                "runtime".to_string(),
            )]),
            ..BufferedHost::default()
        };
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(host.stdout_flushes, 1);
        assert_eq!(host.stderr_flushes, 1);
        assert_eq!(
            host.stdout,
            vec![
                "runtime".to_string(),
                "true".to_string(),
                "true".to_string(),
                "2".to_string(),
                "5".to_string(),
                "101".to_string(),
                "ell".to_string(),
                "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_fs_bytes_routines() {
        let dir = temp_workspace_dir("std_fs_bytes");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_fs_bytes\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.bytes\n",
                "import std.fs\n",
                "import std.io\n",
                "import std.path\n",
                "use std.result.Result\n",
                "fn unwrap_unit(result: Result[Unit, Str]) -> Bool:\n",
                "    return match result:\n",
                "        Result.Ok(_) => true\n",
                "        Result.Err(_) => false\n",
                "fn unwrap_bytes(result: Result[Array[Int], Str]) -> Array[Int]:\n",
                "    return match result:\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => std.bytes.from_str_utf8 :: \"\" :: call\n",
                "fn unwrap_int(result: Result[Int, Str]) -> Int:\n",
                "    return match result:\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => -1\n",
                "fn main() -> Int:\n",
                "    let root = std.path.cwd :: :: call\n",
                "    let data_dir = std.path.join :: root, \"data\" :: call\n",
                "    let nested_dir = std.path.join :: data_dir, \"nested\" :: call\n",
                "    let empty_dir = std.path.join :: root, \"empty\" :: call\n",
                "    let source = std.path.join :: data_dir, \"payload.bin\" :: call\n",
                "    let copied = std.path.join :: nested_dir, \"copied.bin\" :: call\n",
                "    let moved = std.path.join :: root, \"moved.bin\" :: call\n",
                "    if not (unwrap_unit :: (std.fs.create_dir :: empty_dir :: call) :: call):\n",
                "        return 1\n",
                "    if not (unwrap_unit :: (std.fs.remove_dir :: empty_dir :: call) :: call):\n",
                "        return 2\n",
                "    if not (unwrap_unit :: (std.fs.create_dir :: data_dir :: call) :: call):\n",
                "        return 3\n",
                "    if not (unwrap_unit :: (std.fs.mkdir_all :: nested_dir :: call) :: call):\n",
                "        return 4\n",
                "    let payload = std.bytes.from_str_utf8 :: \"arc\" :: call\n",
                "    if not (unwrap_unit :: (std.fs.write_bytes :: source, payload :: call) :: call):\n",
                "        return 5\n",
                "    if not (unwrap_unit :: (std.fs.copy_file :: source, copied :: call) :: call):\n",
                "        return 6\n",
                "    if not (unwrap_unit :: (std.fs.rename :: copied, moved :: call) :: call):\n",
                "        return 7\n",
                "    let read_back = unwrap_bytes :: (std.fs.read_bytes :: moved :: call) :: call\n",
                "    let size = unwrap_int :: (std.fs.file_size :: moved :: call) :: call\n",
                "    let modified = unwrap_int :: (std.fs.modified_unix_ms :: moved :: call) :: call\n",
                "    std.io.print[Bool] :: (std.fs.exists :: source :: call) :: call\n",
                "    std.io.print[Str] :: (std.bytes.to_str_utf8 :: read_back :: call) :: call\n",
                "    std.io.print[Int] :: size :: call\n",
                "    std.io.print[Bool] :: (modified > 0) :: call\n",
                "    if not (unwrap_unit :: (std.fs.remove_file :: source :: call) :: call):\n",
                "        return 8\n",
                "    if not (unwrap_unit :: (std.fs.remove_file :: moved :: call) :: call):\n",
                "        return 9\n",
                "    if not (unwrap_unit :: (std.fs.remove_dir_all :: data_dir :: call) :: call):\n",
                "        return 10\n",
                "    std.io.print[Bool] :: (std.fs.exists :: data_dir :: call) :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_fs_bytes")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let cwd = dir.join("fixture").to_string_lossy().replace('\\', "/");
        fs::create_dir_all(dir.join("fixture")).expect("fixture root should exist");
        let mut host = BufferedHost {
            cwd: cwd.clone(),
            sandbox_root: cwd,
            ..BufferedHost::default()
        };
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            host.stdout,
            vec![
                "true".to_string(),
                "arc".to_string(),
                "3".to_string(),
                "true".to_string(),
                "false".to_string(),
            ]
        );

        let fixture_root = dir.join("fixture");
        assert!(!fixture_root.join("data").exists());
        assert!(!fixture_root.join("moved.bin").exists());
        assert!(!fixture_root.join("empty").exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_fs_stream_routines() {
        let dir = temp_workspace_dir("std_fs_streams");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_fs_streams\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.bytes\n",
                "import std.fs\n",
                "use std.result.Result\n",
                "fn write_and_close(take stream: std.fs.FileStream, read bytes: Array[Int]) -> Int:\n",
                "    let mut stream = stream\n",
                "    let wrote = match (std.fs.stream_write :: stream, bytes :: call):\n",
                "        Result.Ok(count) => count\n",
                "        Result.Err(_) => -1\n",
                "    if wrote < 0:\n",
                "        return 1\n",
                "    if wrote != (std.bytes.len :: bytes :: call):\n",
                "        return 2\n",
                "    let close_result = std.fs.stream_close :: stream :: call\n",
                "    if close_result :: :: is_err:\n",
                "        return 3\n",
                "    return 0\n",
                "fn verify_read(take stream: std.fs.FileStream) -> Int:\n",
                "    let mut stream = stream\n",
                "    let empty = std.bytes.from_str_utf8 :: \"\" :: call\n",
                "    let first_result = std.fs.stream_read :: stream, 5 :: call\n",
                "    if first_result :: :: is_err:\n",
                "        return 4\n",
                "    let first = match first_result:\n",
                "        Result.Ok(bytes) => bytes\n",
                "        Result.Err(_) => empty\n",
                "    if (std.bytes.to_str_utf8 :: first :: call) != \"hello\":\n",
                "        return 5\n",
                "    let before_eof_result = std.fs.stream_eof :: stream :: call\n",
                "    if before_eof_result :: :: is_err:\n",
                "        return 6\n",
                "    let before_eof = match before_eof_result:\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => false\n",
                "    if before_eof:\n",
                "        return 7\n",
                "    let second_result = std.fs.stream_read :: stream, 5 :: call\n",
                "    if second_result :: :: is_err:\n",
                "        return 8\n",
                "    let second = match second_result:\n",
                "        Result.Ok(bytes) => bytes\n",
                "        Result.Err(_) => empty\n",
                "    if (std.bytes.to_str_utf8 :: second :: call) != \"!\":\n",
                "        return 9\n",
                "    let after_eof_result = std.fs.stream_eof :: stream :: call\n",
                "    if after_eof_result :: :: is_err:\n",
                "        return 10\n",
                "    let after_eof = match after_eof_result:\n",
                "        Result.Ok(value) => value\n",
                "        Result.Err(_) => false\n",
                "    if not after_eof:\n",
                "        return 11\n",
                "    let close_result = std.fs.stream_close :: stream :: call\n",
                "    if close_result :: :: is_err:\n",
                "        return 12\n",
                "    return 0\n",
                "fn main() -> Int:\n",
                "    let hello = std.bytes.from_str_utf8 :: \"hello\" :: call\n",
                "    let bang = std.bytes.from_str_utf8 :: \"!\" :: call\n",
                "    let write_status = match (std.fs.stream_open_write :: \"notes.bin\", false :: call):\n",
                "        Result.Ok(stream) => write_and_close :: stream, hello :: call\n",
                "        Result.Err(_) => 20\n",
                "    if write_status != 0:\n",
                "        return 21\n",
                "    let append_status = match (std.fs.stream_open_write :: \"notes.bin\", true :: call):\n",
                "        Result.Ok(stream) => write_and_close :: stream, bang :: call\n",
                "        Result.Err(_) => 22\n",
                "    if append_status != 0:\n",
                "        return 23\n",
                "    let read_status = match (std.fs.stream_open_read :: \"notes.bin\" :: call):\n",
                "        Result.Ok(stream) => verify_read :: stream :: call\n",
                "        Result.Err(_) => 24\n",
                "    if read_status != 0:\n",
                "        return 25\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_fs_streams")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let fixture_root = dir.join("fixture");
        fs::create_dir_all(&fixture_root).expect("fixture root should exist");
        let cwd = fixture_root.to_string_lossy().replace('\\', "/");
        let mut host = BufferedHost {
            cwd: cwd.clone(),
            sandbox_root: cwd,
            ..BufferedHost::default()
        };
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            fs::read_to_string(fixture_root.join("notes.bin")).expect("streamed file should exist"),
            "hello!"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_local_record_constructor_and_impl_method() {
        let dir = temp_workspace_dir("record_method");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_record_method\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.io\n",
                "record Counter:\n",
                "    value: Int\n",
                "impl Counter:\n",
                "    fn double(read self: Counter) -> Int:\n",
                "        return self.value * 2\n",
                "fn main() -> Int:\n",
                "    let counter = Counter :: value = 7 :: call\n",
                "    std.io.print[Int] :: counter.value :: call\n",
                "    std.io.print[Int] :: (counter :: :: double) :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_record_method")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(host.stdout, vec!["7".to_string(), "14".to_string()]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_process_routines() {
        let dir = temp_workspace_dir("std_process");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_process\"\nkind = \"app\"\n",
        );
        let (program, status_a, status_b, capture_a, capture_b) = if cfg!(windows) {
            ("cmd", "/C", "exit 7", "/C", "echo hello")
        } else {
            ("sh", "-c", "exit 7", "-c", "printf hello")
        };
        write_file(
            &dir.join("src").join("shelf.arc"),
            &format!(
                concat!(
                    "import std.bytes\n",
                    "import std.collections.list\n",
                    "import std.io\n",
                    "import std.process\n",
                    "import std.text\n",
                    "use std.result.Result\n",
                    "fn status_args() -> List[Str]:\n",
                    "    let mut args = std.collections.list.new[Str] :: :: call\n",
                    "    args :: {status_a:?} :: push\n",
                    "    args :: {status_b:?} :: push\n",
                    "    return args\n",
                    "fn capture_args() -> List[Str]:\n",
                    "    let mut args = std.collections.list.new[Str] :: :: call\n",
                    "    args :: {capture_a:?} :: push\n",
                    "    args :: {capture_b:?} :: push\n",
                    "    return args\n",
                    "fn main() -> Int:\n",
                    "    let status = match (std.process.exec_status :: {program:?}, (status_args :: :: call) :: call):\n",
                    "        Result.Ok(value) => value\n",
                    "        Result.Err(_) => -1\n",
                    "    let capture_result = std.process.exec_capture :: {program:?}, (capture_args :: :: call) :: call\n",
                    "    if capture_result :: :: is_err:\n",
                    "        return 99\n",
                    "    let empty = std.bytes.from_str_utf8 :: \"\" :: call\n",
                    "    let capture = capture_result :: (std.process.ExecCapture :: status = 0, output = (empty, empty), utf8 = (true, true) :: call) :: unwrap_or\n",
                    "    let text = match (capture :: :: stdout_text):\n",
                    "        Result.Ok(value) => value\n",
                    "        Result.Err(_) => \"\"\n",
                    "    std.io.print[Int] :: status :: call\n",
                    "    std.io.print[Bool] :: (capture :: :: success) :: call\n",
                    "    std.io.print[Bool] :: (std.text.starts_with :: text, \"hello\" :: call) :: call\n",
                    "    return 0\n",
                ),
                program = program,
                status_a = status_a,
                status_b = status_b,
                capture_a = capture_a,
                capture_b = capture_b,
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_process")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost {
            allow_process: true,
            ..BufferedHost::default()
        };
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            host.stdout,
            vec!["7".to_string(), "true".to_string(), "true".to_string()]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_option_routines() {
        let dir = temp_workspace_dir("std_option");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_option\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.option\n",
                "import std.io\n",
                "use std.option.Option\n",
                "fn main() -> Int:\n",
                "    let some = Option.Some[Int] :: 5 :: call\n",
                "    let none = Option.None[Int] :: :: call\n",
                "    std.io.print[Bool] :: (some :: :: is_some) :: call\n",
                "    std.io.print[Bool] :: (none :: :: is_none) :: call\n",
                "    std.io.print[Int] :: (some :: 0 :: unwrap_or) :: call\n",
                "    std.io.print[Int] :: (none :: 9 :: unwrap_or) :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_option")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            host.stdout,
            vec![
                "true".to_string(),
                "true".to_string(),
                "5".to_string(),
                "9".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_linked_std_ecs_behavior_routines() {
        let dir = temp_workspace_dir("std_ecs");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_ecs\"\nkind = \"app\"\n",
        );
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.behaviors\n",
                "import std.ecs\n",
                "import std.io\n",
                "record Position:\n",
                "    x: Int\n",
                "    y: Int\n",
                "behavior[phase=startup] fn boot() -> Int:\n",
                "    std.ecs.set_component[Int] :: 7 :: call\n",
                "    let entity = std.ecs.spawn :: :: call\n",
                "    std.ecs.set_component_at[Position] :: entity, (Position :: x = 4, y = 5 :: call) :: call\n",
                "    return 0\n",
                "behavior[phase=update] fn tick() -> Int:\n",
                "    if not (std.ecs.has_component[Int] :: :: call):\n",
                "        return 10\n",
                "    let current = std.ecs.get_component[Int] :: :: call\n",
                "    std.ecs.set_component[Int] :: current + 1 :: call\n",
                "    return 0\n",
                "system[phase=update] fn cleanup() -> Int:\n",
                "    if not (std.ecs.has_component_at[Position] :: 1 :: call):\n",
                "        return 20\n",
                "    let pos = std.ecs.get_component_at[Position] :: 1 :: call\n",
                "    if pos.x != 4:\n",
                "        return 21\n",
                "    if pos.y != 5:\n",
                "        return 22\n",
                "    let current = std.ecs.get_component[Int] :: :: call\n",
                "    std.ecs.set_component[Int] :: current + 10 :: call\n",
                "    std.ecs.remove_component_at[Position] :: 1 :: call\n",
                "    std.ecs.despawn :: 1 :: call\n",
                "    return 0\n",
                "behavior[phase=render] fn render_only() -> Int:\n",
                "    std.ecs.set_component[Int] :: 999 :: call\n",
                "    return 0\n",
                "fn main() -> Int:\n",
                "    if (std.ecs.step_startup :: :: call) != 0:\n",
                "        return 1\n",
                "    if (std.behaviors.step :: \"update\" :: call) != 0:\n",
                "        return 2\n",
                "    if (std.ecs.get_component[Int] :: :: call) != 18:\n",
                "        return 3\n",
                "    if std.ecs.has_component_at[Position] :: 1 :: call:\n",
                "        return 4\n",
                "    std.io.print[Int] :: (std.ecs.get_component[Int] :: :: call) :: call\n",
                "    return 0\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_ecs")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let mut host = BufferedHost::default();
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(host.stdout, vec!["18".to_string()]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_owned_app_facade_workspace() {
        let dir = temp_workspace_dir("owned_app_facade");
        let desktop_dep = repo_root()
            .join("grimoires")
            .join("owned")
            .join("app")
            .join("arcana-desktop")
            .to_string_lossy()
            .replace('\\', "/");
        let audio_dep = repo_root()
            .join("grimoires")
            .join("owned")
            .join("app")
            .join("arcana-audio")
            .to_string_lossy()
            .replace('\\', "/");
        write_file(
            &dir.join("book.toml"),
            &format!(
                concat!(
                    "name = \"runtime_owned_app_facade\"\n",
                    "kind = \"app\"\n",
                    "[deps]\n",
                    "arcana_desktop = {desktop_dep:?}\n",
                    "arcana_audio = {audio_dep:?}\n",
                ),
                desktop_dep = desktop_dep,
                audio_dep = audio_dep,
            ),
        );
        write_file(&dir.join("fixture").join("clip.wav"), "wave");
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import arcana_audio.clip\n",
                "import arcana_audio.output\n",
                "import arcana_audio.playback\n",
                "import arcana_desktop.events\n",
                "import arcana_desktop.input\n",
                "import arcana_desktop.window\n",
                "import std.io\n",
                "use std.result.Result\n",
                "fn with_playback(take win: std.window.Window, take device: std.audio.AudioDevice, take playback: std.audio.AudioPlayback) -> Int:\n",
                "    std.io.print[Bool] :: (arcana_audio.playback.playing :: playback :: call) :: call\n",
                "    let stop = arcana_audio.playback.stop :: playback :: call\n",
                "    if stop :: :: is_err:\n",
                "        return 7\n",
                "    let close_audio = arcana_audio.output.close :: device :: call\n",
                "    if close_audio :: :: is_err:\n",
                "        return 8\n",
                "    let close_window = arcana_desktop.window.close :: win :: call\n",
                "    if close_window :: :: is_err:\n",
                "        return 9\n",
                "    return 0\n",
                "fn with_clip(take win: std.window.Window, take device: std.audio.AudioDevice, read clip: std.audio.AudioBuffer) -> Int:\n",
                "    let mut device = device\n",
                "    let info = arcana_audio.clip.info :: clip :: call\n",
                "    if info.sample_rate_hz != 48000:\n",
                "        return 5\n",
                "    let playback_result = arcana_audio.playback.play :: device, clip :: call\n",
                "    return match playback_result:\n",
                "        Result.Ok(value) => with_playback :: win, device, value :: call\n",
                "        Result.Err(_) => 6\n",
                "fn with_device(take win: std.window.Window, take device: std.audio.AudioDevice) -> Int:\n",
                "    let mut device = device\n",
                "    let cfg = arcana_audio.output.default_output_config :: :: call\n",
                "    arcana_audio.output.configure :: device, cfg :: call\n",
                "    std.io.print[Int] :: (arcana_audio.output.sample_rate_hz :: device :: call) :: call\n",
                "    return match (arcana_audio.clip.load_wav :: \"clip.wav\" :: call):\n",
                "        Result.Ok(value) => with_clip :: win, device, value :: call\n",
                "        Result.Err(_) => 4\n",
                "fn with_window(take win: std.window.Window) -> Int:\n",
                "    let mut win = win\n",
                "    if not (arcana_desktop.window.alive :: win :: call):\n",
                "        return 2\n",
                "    let frame = arcana_desktop.events.pump :: win :: call\n",
                "    let key = arcana_desktop.input.key_code :: \"A\" :: call\n",
                "    std.io.print[Bool] :: (arcana_desktop.input.key_down :: frame, key :: call) :: call\n",
                "    return match (arcana_audio.output.default_output :: :: call):\n",
                "        Result.Ok(value) => with_device :: win, value :: call\n",
                "        Result.Err(_) => 3\n",
                "fn main() -> Int:\n",
                "    return match (arcana_desktop.window.open :: \"Arcana\", 320, 200 :: call):\n",
                "        Result.Ok(value) => with_window :: value :: call\n",
                "        Result.Err(_) => 1\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_owned_app_facade")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let fixture_root = dir.join("fixture");
        let mut host = synthetic_window_canvas_host(&fixture_root);
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            host.stdout,
            vec!["true".to_string(), "48000".to_string(), "true".to_string()]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_synthetic_audio_runtime() {
        let dir = temp_workspace_dir("std_audio");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_std_audio\"\nkind = \"app\"\n",
        );
        write_file(&dir.join("fixture").join("clip.wav"), "wave");
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.audio\n",
                "use std.result.Result\n",
                "fn use_playback(take device: std.audio.AudioDevice, take playback: std.audio.AudioPlayback) -> Int:\n",
                "    let mut device = device\n",
                "    let mut playback = playback\n",
                "    if not (playback :: :: playing):\n",
                "        return 9\n",
                "    if playback :: :: paused:\n",
                "        return 10\n",
                "    if playback :: :: finished:\n",
                "        return 11\n",
                "    playback :: :: pause\n",
                "    if not (playback :: :: paused):\n",
                "        return 12\n",
                "    playback :: :: resume\n",
                "    playback :: 500 :: set_gain_milli\n",
                "    playback :: true :: set_looping\n",
                "    if not (playback :: :: looping):\n",
                "        return 13\n",
                "    if (playback :: :: position_frames) != 0:\n",
                "        return 14\n",
                "    let stop = playback :: :: stop\n",
                "    if stop :: :: is_err:\n",
                "        return 15\n",
                "    let close = std.audio.output_close :: device :: call\n",
                "    if close :: :: is_err:\n",
                "        return 16\n",
                "    return 0\n",
                "fn use_clip(take device: std.audio.AudioDevice, read clip: std.audio.AudioBuffer) -> Int:\n",
                "    let mut device = device\n",
                "    if (std.audio.buffer_frames :: clip :: call) != 64:\n",
                "        return 5\n",
                "    if (std.audio.buffer_channels :: clip :: call) != 2:\n",
                "        return 6\n",
                "    if (std.audio.buffer_sample_rate_hz :: clip :: call) != 48000:\n",
                "        return 7\n",
                "    let playback_result = std.audio.play_buffer :: device, clip :: call\n",
                "    return match playback_result:\n",
                "        Result.Ok(value) => use_playback :: device, value :: call\n",
                "        Result.Err(_) => 8\n",
                "fn use_device(take device: std.audio.AudioDevice) -> Int:\n",
                "    let mut device = device\n",
                "    if (std.audio.output_sample_rate_hz :: device :: call) != 48000:\n",
                "        return 2\n",
                "    if (std.audio.output_channels :: device :: call) != 2:\n",
                "        return 3\n",
                "    std.audio.output_set_gain_milli :: device, 750 :: call\n",
                "    return match (std.audio.buffer_load_wav :: \"clip.wav\" :: call):\n",
                "        Result.Ok(value) => use_clip :: device, value :: call\n",
                "        Result.Err(_) => 4\n",
                "fn main() -> Int:\n",
                "    return match (std.audio.default_output :: :: call):\n",
                "        Result.Ok(value) => use_device :: value :: call\n",
                "        Result.Err(_) => 1\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_std_audio")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let fixture_root = dir.join("fixture");
        let mut host = synthetic_audio_host(&fixture_root);
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            host.audio_log,
            vec![
                "default_output:0".to_string(),
                "output_set_gain_milli:0,750".to_string(),
                format!(
                    "buffer_load_wav:{}/clip.wav",
                    fixture_root.to_string_lossy().replace('\\', "/")
                ),
                format!(
                    "play_buffer:0,0,{}/clip.wav",
                    fixture_root.to_string_lossy().replace('\\', "/")
                ),
                "playback_pause:0".to_string(),
                "playback_resume:0".to_string(),
                "playback_set_gain_milli:0,500".to_string(),
                "playback_set_looping:0,true".to_string(),
                "playback_stop:0".to_string(),
                "output_close:0".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_synthetic_window_canvas_events_runtime() {
        let dir = temp_workspace_dir("std_window_canvas");
        write_file(
            &dir.join("book.toml"),
            "name = \"runtime_window_canvas\"\nkind = \"app\"\n",
        );
        write_file(&dir.join("fixture").join("sprite.bin"), "sprite");
        write_file(
            &dir.join("src").join("shelf.arc"),
            concat!(
                "import std.canvas\n",
                "import std.events\n",
                "import std.input\n",
                "import std.time\n",
                "import std.window\n",
                "use std.result.Result\n",
                "fn draw_image(edit win: std.window.Window, read img: std.canvas.Image) -> Int:\n",
                "    let size = std.canvas.image_size :: img :: call\n",
                "    if size.0 != 16 or size.1 != 16:\n",
                "        return 1\n",
                "    std.canvas.blit :: win, img, 7 :: call\n",
                "        y = 8\n",
                "    std.canvas.blit_scaled :: win, img, 1 :: call\n",
                "        y = 2\n",
                "        w = 3\n",
                "        h = 4\n",
                "    std.canvas.blit_region :: win, img, 0 :: call\n",
                "        sy = 0\n",
                "        sw = 1\n",
                "        sh = 1\n",
                "        dx = 9\n",
                "        dy = 10\n",
                "        dw = 11\n",
                "        dh = 12\n",
                "    return 0\n",
                "fn run(take win: std.window.Window) -> Int:\n",
                "    let mut win = win\n",
                "    if not (std.window.alive :: win :: call):\n",
                "        return 2\n",
                "    let size = std.window.size :: win :: call\n",
                "    if size.0 != 320 or size.1 != 200:\n",
                "        return 3\n",
                "    std.window.set_title :: win, \"Renamed\" :: call\n",
                "    std.window.set_topmost :: win, true :: call\n",
                "    let color = std.canvas.rgb :: 10, 20, 30 :: call\n",
                "    let rect = std.canvas.RectSpec :: pos = (1, 2), size = (3, 4), color = color :: call\n",
                "    std.canvas.fill :: win, color :: call\n",
                "    std.canvas.rect_draw :: win, rect :: call\n",
                "    std.canvas.label :: win, 5, 6 :: call\n",
                "        text = \"Arcana\"\n",
                "        color = color\n",
                "    let label_size = std.canvas.label_size :: \"Arcana\" :: call\n",
                "    if label_size.0 <= 0:\n",
                "        return 4\n",
                "    let image_status = match (std.canvas.image_load :: \"sprite.bin\" :: call):\n",
                "        Result.Ok(img) => draw_image :: win, img :: call\n",
                "        Result.Err(_) => 5\n",
                "    if image_status != 0:\n",
                "        return 6\n",
                "    std.canvas.present :: win :: call\n",
                "    let start = std.time.monotonic_now_ms :: :: call\n",
                "    std.time.sleep_ms :: 5 :: call\n",
                "    let end = std.time.monotonic_now_ms :: :: call\n",
                "    let delta = std.time.elapsed_ms :: start, end :: call\n",
                "    if delta.value < 0:\n",
                "        return 7\n",
                "    let mut frame = std.events.pump :: win :: call\n",
                "    if not (std.input.mouse_in_window :: frame :: call):\n",
                "        return 8\n",
                "    if (std.input.mouse_pos :: frame :: call).0 != 40:\n",
                "        return 9\n",
                "    let key = std.input.key_code :: \"A\" :: call\n",
                "    if not (std.input.key_down :: frame, key :: call):\n",
                "        return 10\n",
                "    let first = std.events.poll :: frame :: call\n",
                "    if first :: :: is_none:\n",
                "        return 11\n",
                "    let second = std.events.poll :: frame :: call\n",
                "    if second :: :: is_none:\n",
                "        return 12\n",
                "    let none = std.events.poll :: frame :: call\n",
                "    if not (none :: :: is_none):\n",
                "        return 13\n",
                "    let close = std.window.close :: win :: call\n",
                "    if close :: :: is_err:\n",
                "        return 14\n",
                "    return 0\n",
                "fn main() -> Int:\n",
                "    return match (std.window.open :: \"Arcana\", 320, 200 :: call):\n",
                "        Result.Ok(win) => run :: win :: call\n",
                "        Result.Err(_) => 99\n",
            ),
        );
        write_file(&dir.join("src").join("types.arc"), "// test types\n");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_window_canvas")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");
        let fixture_root = dir.join("fixture");
        let decode_routine = resolve_routine_index(
            &plan,
            &plan.root_module_id,
            &[
                "std".to_string(),
                "kernel".to_string(),
                "events".to_string(),
                "decode".to_string(),
            ],
        )
        .expect("std.kernel.events.decode should exist");
        let kernel_poll_routine = resolve_routine_index(
            &plan,
            &plan.root_module_id,
            &[
                "std".to_string(),
                "kernel".to_string(),
                "events".to_string(),
                "poll".to_string(),
            ],
        )
        .expect("std.kernel.events.poll should exist");
        let lift_event_routine = resolve_routine_index(
            &plan,
            &plan.root_module_id,
            &[
                "std".to_string(),
                "events".to_string(),
                "lift_event".to_string(),
            ],
        )
        .expect("std.events.lift_event should exist");
        let poll_routine = resolve_routine_index(
            &plan,
            &plan.root_module_id,
            &["std".to_string(), "events".to_string(), "poll".to_string()],
        )
        .expect("std.events.poll should exist");

        let mut debug_host = synthetic_window_canvas_host(&fixture_root);
        let decoded = execute_routine(
            &plan,
            decode_routine,
            vec![
                RuntimeValue::Int(3),
                RuntimeValue::Int(1),
                RuntimeValue::Int(0),
            ],
            &mut debug_host,
        )
        .expect("std.kernel.events.decode should execute");
        assert_eq!(
            decoded,
            RuntimeValue::Variant {
                name: "std.kernel.events.Event.WindowFocused".to_string(),
                payload: vec![RuntimeValue::Bool(true)],
            }
        );

        let debug_window = debug_host
            .window_open("Arcana", 320, 200)
            .expect("debug window should open");
        let debug_frame = debug_host
            .events_pump(debug_window)
            .expect("debug frame should pump");
        let kernel_polled = execute_routine(
            &plan,
            kernel_poll_routine,
            vec![RuntimeValue::Opaque(RuntimeOpaqueValue::AppFrame(
                debug_frame,
            ))],
            &mut debug_host,
        )
        .expect("std.kernel.events.poll should execute");
        assert_eq!(
            kernel_polled,
            RuntimeValue::Variant {
                name: "std.kernel.events.Event.WindowFocused".to_string(),
                payload: vec![RuntimeValue::Bool(true)],
            }
        );
        let lifted_direct = execute_routine(
            &plan,
            lift_event_routine,
            vec![kernel_polled.clone()],
            &mut debug_host,
        )
        .expect("std.events.lift_event should execute");
        assert_eq!(
            lifted_direct,
            RuntimeValue::Variant {
                name: "std.option.Option.Some".to_string(),
                payload: vec![RuntimeValue::Variant {
                    name: "AppEvent.WindowFocused".to_string(),
                    payload: vec![RuntimeValue::Bool(true)],
                }],
            }
        );

        let mut debug_host = synthetic_window_canvas_host(&fixture_root);
        let debug_window = debug_host
            .window_open("Arcana", 320, 200)
            .expect("debug window should open");
        let debug_frame = debug_host
            .events_pump(debug_window)
            .expect("debug frame should pump");
        let lifted = execute_routine(
            &plan,
            poll_routine,
            vec![RuntimeValue::Opaque(RuntimeOpaqueValue::AppFrame(
                debug_frame,
            ))],
            &mut debug_host,
        )
        .expect("std.events.poll should execute");
        assert_eq!(
            lifted,
            RuntimeValue::Variant {
                name: "std.option.Option.Some".to_string(),
                payload: vec![RuntimeValue::Variant {
                    name: "AppEvent.WindowFocused".to_string(),
                    payload: vec![RuntimeValue::Bool(true)],
                }],
            }
        );

        let mut host = synthetic_window_canvas_host(&fixture_root);
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(host.sleep_log_ms, vec![5]);
        assert_eq!(
            host.canvas_log,
            vec![
                "fill:660510".to_string(),
                "rect:1,2,3,4,660510".to_string(),
                "label:5,6,Arcana,660510".to_string(),
                format!(
                    "blit:{}/sprite.bin,7,8",
                    fixture_root.to_string_lossy().replace('\\', "/")
                ),
                format!(
                    "blit_scaled:{}/sprite.bin,1,2,3,4",
                    fixture_root.to_string_lossy().replace('\\', "/",)
                ),
                format!(
                    "blit_region:{}/sprite.bin,0,0,1,1,9,10,11,12",
                    fixture_root.to_string_lossy().replace('\\', "/",)
                ),
                "present".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn execute_main_runs_synthetic_host_core_workspace_artifact() {
        let dir = temp_workspace_dir("host_tool");
        write_host_core_workspace(&dir);

        let fixture_root = dir.join("fixture");
        write_file(&fixture_root.join("alpha.arc"), "alpha");
        write_file(&fixture_root.join("notes.txt"), "skip me");

        let graph = load_workspace_graph(&dir).expect("workspace graph should load");
        let checked = check_workspace_graph(&graph).expect("workspace should check");
        let fingerprints = compute_member_fingerprints_for_checked_workspace(&graph, &checked)
            .expect("fingerprints should compute");
        let order = plan_workspace(&graph).expect("workspace order should plan");
        let statuses =
            plan_build(&graph, &order, &fingerprints, None).expect("build plan should compute");
        execute_build(&graph, &statuses).expect("build should execute");

        let artifact_path = graph.root_dir.join(
            &statuses
                .iter()
                .find(|status| status.member == "runtime_host_core")
                .expect("app artifact status should exist")
                .artifact_rel_path,
        );
        let plan = load_package_plan(&artifact_path).expect("runtime plan should load");

        let cwd = fixture_root.to_string_lossy().replace('\\', "/");
        let mut host = BufferedHost {
            cwd: cwd.clone(),
            sandbox_root: cwd.clone(),
            ..BufferedHost::default()
        };
        let code = execute_main(&plan, &mut host).expect("runtime should execute");

        assert_eq!(code, 0);
        assert_eq!(
            host.stdout,
            vec![
                format!("{cwd}/alpha.arc"),
                "1".to_string(),
                "12".to_string(),
            ]
        );

        let report_path = fixture_root
            .join(".arcana")
            .join("logs")
            .join("host_core_report.txt");
        assert_eq!(
            fs::read_to_string(&report_path).expect("report should write"),
            "Arcana Runtime Host Core v1\n"
        );

        let _ = fs::remove_dir_all(dir);
    }
}
