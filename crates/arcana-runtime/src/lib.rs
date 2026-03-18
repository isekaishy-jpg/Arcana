use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Component, Path, PathBuf};

use arcana_aot::{
    AotEntrypointArtifact, AotOwnerArtifact, AotPackageArtifact, AotRoutineArtifact,
    parse_package_artifact, validate_package_artifact,
};
use arcana_ir::{
    ExecAssignOp as ParsedAssignOp, ExecAssignTarget as ParsedAssignTarget,
    ExecAvailabilityAttachment as ParsedAvailabilityAttachment,
    ExecAvailabilityKind as ParsedAvailabilityKind, ExecBinaryOp as ParsedBinaryOp,
    ExecChainConnector as ParsedChainConnector, ExecChainIntroducer as ParsedChainIntroducer,
    ExecChainStep as ParsedChainStep, ExecDynamicDispatch as ParsedDynamicDispatch,
    ExecExpr as ParsedExpr, ExecHeaderAttachment as ParsedHeaderAttachment,
    ExecMatchArm as ParsedMatchArm, ExecMatchPattern as ParsedMatchPattern,
    ExecPageRollup as ParsedPageRollup, ExecPhraseArg as ParsedPhraseArg,
    ExecPhraseQualifierKind as ParsedPhraseQualifierKind, ExecStmt as ParsedStmt,
    ExecUnaryOp as ParsedUnaryOp, RUNTIME_MAIN_ENTRYPOINT_NAME,
    runtime_main_return_type_from_signature, validate_runtime_main_entry_contract,
};
use pathdiff::diff_paths;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod json_abi;
mod native_abi;
#[cfg(windows)]
mod native_host;
mod package_image;
pub use json_abi::{
    RUNTIME_JSON_ABI_FORMAT, execute_exported_json_abi_routine, render_exported_json_abi_manifest,
};
pub use native_abi::{RuntimeAbiValue, execute_exported_abi_routine};
#[cfg(windows)]
pub use native_host::NativeProcessHost;
pub use package_image::{
    RUNTIME_PACKAGE_IMAGE_FORMAT, parse_runtime_package_image, render_runtime_package_image,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeParamPlan {
    pub mode: Option<String>,
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRoutinePlan {
    pub module_id: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub exported: bool,
    pub is_async: bool,
    pub type_params: Vec<String>,
    pub behavior_attrs: BTreeMap<String, String>,
    pub params: Vec<RuntimeParamPlan>,
    pub signature_row: String,
    pub intrinsic_impl: Option<String>,
    pub impl_target_type: Option<String>,
    pub impl_trait_path: Option<Vec<String>>,
    pub availability: Vec<ParsedAvailabilityAttachment>,
    pub foreword_rows: Vec<String>,
    rollups: Vec<ParsedPageRollup>,
    statements: Vec<ParsedStmt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeOwnerObjectPlan {
    pub type_path: Vec<String>,
    pub local_name: String,
    pub init_routine_key: Option<String>,
    pub init_with_context_routine_key: Option<String>,
    pub resume_routine_key: Option<String>,
    pub resume_with_context_routine_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeOwnerExitPlan {
    pub name: String,
    pub condition: ParsedExpr,
    pub holds: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeOwnerPlan {
    pub module_id: String,
    pub owner_path: Vec<String>,
    pub owner_name: String,
    pub objects: Vec<RuntimeOwnerObjectPlan>,
    pub exits: Vec<RuntimeOwnerExitPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEntrypointPlan {
    pub module_id: String,
    pub symbol_name: String,
    pub symbol_kind: String,
    pub is_async: bool,
    pub exported: bool,
    pub routine_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePackagePlan {
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub runtime_requirements: Vec<String>,
    pub module_aliases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub entrypoints: Vec<RuntimeEntrypointPlan>,
    pub routines: Vec<RuntimeRoutinePlan>,
    pub owners: Vec<RuntimeOwnerPlan>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeChannelHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMutexHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAtomicIntHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAtomicBoolHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeArenaHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeFrameArenaHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimePoolArenaHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeTaskHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeThreadHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeLocalHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeArenaIdValue {
    arena: RuntimeArenaHandle,
    slot: u64,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeFrameIdValue {
    arena: RuntimeFrameArenaHandle,
    slot: u64,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimePoolIdValue {
    arena: RuntimePoolArenaHandle,
    slot: u64,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeReferenceTarget {
    Local {
        local: RuntimeLocalHandle,
        members: Vec<String>,
    },
    OwnerObject {
        owner_key: String,
        object_name: String,
        members: Vec<String>,
    },
    ArenaSlot {
        id: RuntimeArenaIdValue,
        members: Vec<String>,
    },
    FrameSlot {
        id: RuntimeFrameIdValue,
        members: Vec<String>,
    },
    PoolSlot {
        id: RuntimePoolIdValue,
        members: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeReferenceValue {
    mutable: bool,
    target: RuntimeReferenceTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeResolvedPlace {
    mutable: bool,
    target: RuntimeReferenceTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeOpaqueValue {
    FileStream(RuntimeFileStreamHandle),
    Window(RuntimeWindowHandle),
    Image(RuntimeImageHandle),
    AppFrame(RuntimeAppFrameHandle),
    AudioDevice(RuntimeAudioDeviceHandle),
    AudioBuffer(RuntimeAudioBufferHandle),
    AudioPlayback(RuntimeAudioPlaybackHandle),
    Channel(RuntimeChannelHandle),
    Mutex(RuntimeMutexHandle),
    AtomicInt(RuntimeAtomicIntHandle),
    AtomicBool(RuntimeAtomicBoolHandle),
    Arena(RuntimeArenaHandle),
    ArenaId(RuntimeArenaIdValue),
    FrameArena(RuntimeFrameArenaHandle),
    FrameId(RuntimeFrameIdValue),
    PoolArena(RuntimePoolArenaHandle),
    PoolId(RuntimePoolIdValue),
    Task(RuntimeTaskHandle),
    Thread(RuntimeThreadHandle),
}

pub trait RuntimeHost {
    fn supports_runtime_requirement(&self, requirement: &str) -> bool {
        let _ = requirement;
        true
    }
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

pub(crate) fn common_named_key_code(name: &str) -> i64 {
    match name {
        "Backspace" | "backspace" => 8,
        "Tab" | "tab" => 9,
        "Enter" | "enter" => 13,
        "Shift" | "shift" => 16,
        "Control" | "control" | "Ctrl" | "ctrl" => 17,
        "Alt" | "alt" => 18,
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
        _ if name.len() == 1 => name.chars().next().unwrap().to_ascii_uppercase() as i64,
        _ => -1,
    }
}

pub(crate) fn common_named_mouse_button_code(name: &str) -> i64 {
    match name {
        "Left" | "left" => 1,
        "Right" | "right" => 2,
        "Middle" | "middle" => 3,
        "Back" | "back" | "X1" | "x1" => 4,
        "Forward" | "forward" | "X2" | "x2" => 5,
        _ => -1,
    }
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
    pub supported_runtime_requirements: Option<BTreeSet<String>>,
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
    fn supports_runtime_requirement(&self, requirement: &str) -> bool {
        self.supported_runtime_requirements
            .as_ref()
            .is_none_or(|supported| supported.contains(requirement))
    }

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
        let handle = self.insert_audio_device(48_000, 2);
        self.audio_log.push(format!("default_output:{}", handle.0));
        Ok(handle)
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
            self.audio_playbacks.remove(&handle);
        }
        self.audio_devices.remove(&device);
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
        let device_state = self.audio_device_ref(device)?;
        let buffer_state = self.audio_buffer_ref(buffer)?;
        ensure_audio_buffer_matches_device(
            device_state.sample_rate_hz,
            device_state.channels,
            buffer_state.sample_rate_hz,
            buffer_state.channels,
        )?;
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
        if self.audio_playbacks.remove(&playback).is_none() {
            return Err(format!("invalid AudioPlayback handle `{}`", playback.0));
        }
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
    Map(Vec<(RuntimeValue, RuntimeValue)>),
    Range {
        start: Option<i64>,
        end: Option<i64>,
        inclusive_end: bool,
    },
    OwnerHandle(String),
    Ref(RuntimeReferenceValue),
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
struct RuntimeCallArg {
    name: Option<String>,
    value: RuntimeValue,
    source_expr: ParsedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BoundRuntimeArg {
    value: RuntimeValue,
    source_expr: ParsedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RoutineExecutionOutcome {
    value: RuntimeValue,
    final_args: Vec<RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeLocal {
    handle: RuntimeLocalHandle,
    mutable: bool,
    moved: bool,
    value: RuntimeValue,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeScope {
    locals: BTreeMap<String, RuntimeLocal>,
    deferred: Vec<ParsedExpr>,
    attached_object_names: BTreeSet<String>,
    activated_owner_keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FlowSignal {
    Next,
    Return(RuntimeValue),
    Break,
    Continue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeEvalSignal {
    Message(String),
    Return(RuntimeValue),
}

type RuntimeEvalResult<T> = Result<T, RuntimeEvalSignal>;

impl From<String> for RuntimeEvalSignal {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

fn runtime_eval_message(signal: RuntimeEvalSignal) -> String {
    match signal {
        RuntimeEvalSignal::Message(message) => message,
        RuntimeEvalSignal::Return(value) => {
            format!("runtime try qualifier `?` returned from unsupported context with `{value:?}`")
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeExecutionState {
    next_local_handle: u64,
    call_stack: Vec<String>,
    rollup_frames: Vec<RuntimeRollupFrame>,
    owners: BTreeMap<String, RuntimeOwnerState>,
    next_entity_id: i64,
    live_entities: BTreeSet<i64>,
    component_slots: BTreeMap<Vec<String>, BTreeMap<i64, RuntimeValue>>,
    next_arena_handle: u64,
    arenas: BTreeMap<RuntimeArenaHandle, RuntimeArenaState>,
    next_frame_arena_handle: u64,
    frame_arenas: BTreeMap<RuntimeFrameArenaHandle, RuntimeFrameArenaState>,
    next_pool_arena_handle: u64,
    pool_arenas: BTreeMap<RuntimePoolArenaHandle, RuntimePoolArenaState>,
    next_task_handle: u64,
    tasks: BTreeMap<RuntimeTaskHandle, RuntimeTaskState>,
    next_thread_handle: u64,
    threads: BTreeMap<RuntimeThreadHandle, RuntimeThreadState>,
    async_context_depth: usize,
    next_scheduler_thread_id: i64,
    current_thread_id: i64,
    next_channel_handle: u64,
    channels: BTreeMap<RuntimeChannelHandle, RuntimeChannelState>,
    next_mutex_handle: u64,
    mutexes: BTreeMap<RuntimeMutexHandle, RuntimeMutexState>,
    next_atomic_int_handle: u64,
    atomic_ints: BTreeMap<RuntimeAtomicIntHandle, i64>,
    next_atomic_bool_handle: u64,
    atomic_bools: BTreeMap<RuntimeAtomicBoolHandle, bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeOwnerState {
    activation_context: Option<RuntimeValue>,
    objects: BTreeMap<String, RuntimeValue>,
    pending_init: BTreeSet<String>,
    pending_resume: BTreeSet<String>,
    active_bindings: usize,
}

const RUNTIME_MAX_CALL_DEPTH: usize = 256;

type RuntimeTypeBindings = BTreeMap<String, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeChannelState {
    type_args: Vec<String>,
    capacity: usize,
    queue: VecDeque<RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeMutexState {
    type_args: Vec<String>,
    value: Option<RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeArenaState {
    type_args: Vec<String>,
    next_slot: u64,
    generation: u64,
    slots: BTreeMap<u64, RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeFrameArenaState {
    type_args: Vec<String>,
    next_slot: u64,
    generation: u64,
    slots: BTreeMap<u64, RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimePoolArenaState {
    type_args: Vec<String>,
    next_slot: u64,
    free_slots: Vec<u64>,
    generations: BTreeMap<u64, u64>,
    slots: BTreeMap<u64, RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeRollupFrame {
    rollups: Vec<ParsedPageRollup>,
    owner_scope_depth: usize,
    subjects: BTreeMap<String, RuntimeTrackedRollupSubject>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeTrackedRollupSubject {
    binding: Option<RuntimeLocalHandle>,
    value: Option<RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeDeferredCall {
    callable: Vec<String>,
    resolved_routine: Option<String>,
    dynamic_dispatch: Option<ParsedDynamicDispatch>,
    current_module_id: String,
    type_args: Vec<String>,
    call_args: Vec<RuntimeCallArg>,
    scopes: Vec<RuntimeScope>,
    thread_id: i64,
    allow_async: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeDeferredExpr {
    expr: ParsedExpr,
    current_module_id: String,
    aliases: BTreeMap<String, Vec<String>>,
    type_bindings: RuntimeTypeBindings,
    scopes: Vec<RuntimeScope>,
    thread_id: i64,
    allow_async: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeDeferredWork {
    Call(RuntimeDeferredCall),
    Expr(RuntimeDeferredExpr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimePendingState {
    Pending(RuntimeDeferredWork),
    Running,
    Completed(RuntimeValue),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeTaskState {
    type_args: Vec<String>,
    state: RuntimePendingState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeThreadState {
    type_args: Vec<String>,
    state: RuntimePendingState,
}

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
    ConcurrentThreadId,
    ConcurrentTaskDone,
    ConcurrentTaskJoin,
    ConcurrentThreadDone,
    ConcurrentThreadJoin,
    ConcurrentChannelNew,
    ConcurrentChannelSend,
    ConcurrentChannelRecv,
    ConcurrentMutexNew,
    ConcurrentMutexTake,
    ConcurrentMutexPut,
    ConcurrentAtomicIntNew,
    ConcurrentAtomicIntLoad,
    ConcurrentAtomicIntStore,
    ConcurrentAtomicIntAdd,
    ConcurrentAtomicIntSub,
    ConcurrentAtomicIntSwap,
    ConcurrentAtomicBoolNew,
    ConcurrentAtomicBoolLoad,
    ConcurrentAtomicBoolStore,
    ConcurrentAtomicBoolSwap,
    MemoryArenaNew,
    MemoryArenaAlloc,
    MemoryArenaLen,
    MemoryArenaHas,
    MemoryArenaGet,
    MemoryArenaBorrowRead,
    MemoryArenaBorrowEdit,
    MemoryArenaSet,
    MemoryArenaRemove,
    MemoryArenaReset,
    MemoryFrameNew,
    MemoryFrameAlloc,
    MemoryFrameLen,
    MemoryFrameHas,
    MemoryFrameGet,
    MemoryFrameBorrowRead,
    MemoryFrameBorrowEdit,
    MemoryFrameSet,
    MemoryFrameReset,
    MemoryPoolNew,
    MemoryPoolAlloc,
    MemoryPoolLen,
    MemoryPoolHas,
    MemoryPoolGet,
    MemoryPoolBorrowRead,
    MemoryPoolBorrowEdit,
    MemoryPoolSet,
    MemoryPoolRemove,
    MemoryPoolReset,
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
    ListLen,
    ListPush,
    ListPop,
    ListTryPopOr,
    ArrayNew,
    ArrayLen,
    ArrayFromList,
    ArrayToList,
    MapNew,
    MapLen,
    MapHas,
    MapGet,
    MapSet,
    MapRemove,
    MapTryGetOr,
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
    Ok(RuntimeRoutinePlan {
        module_id: routine.module_id.clone(),
        routine_key: routine.routine_key.clone(),
        symbol_name: routine.symbol_name.clone(),
        symbol_kind: routine.symbol_kind.clone(),
        exported: routine.exported,
        is_async: routine.is_async,
        type_params,
        behavior_attrs,
        params,
        signature_row: routine.signature_row.clone(),
        intrinsic_impl: routine.intrinsic_impl.clone(),
        impl_target_type: routine.impl_target_type.clone(),
        impl_trait_path: routine.impl_trait_path.clone(),
        availability: routine.availability.clone(),
        foreword_rows: routine.foreword_rows.clone(),
        rollups: routine.rollups.clone(),
        statements: routine.statements.clone(),
    })
}

fn lower_owner(owner: &AotOwnerArtifact) -> RuntimeOwnerPlan {
    RuntimeOwnerPlan {
        module_id: owner.module_id.clone(),
        owner_path: owner.owner_path.clone(),
        owner_name: owner.owner_name.clone(),
        objects: owner
            .objects
            .iter()
            .map(|object| RuntimeOwnerObjectPlan {
                type_path: object.type_path.clone(),
                local_name: object.local_name.clone(),
                init_routine_key: object.init_routine_key.clone(),
                init_with_context_routine_key: object.init_with_context_routine_key.clone(),
                resume_routine_key: object.resume_routine_key.clone(),
                resume_with_context_routine_key: object.resume_with_context_routine_key.clone(),
            })
            .collect(),
        exits: owner
            .exits
            .iter()
            .map(|owner_exit| RuntimeOwnerExitPlan {
                name: owner_exit.name.clone(),
                condition: owner_exit.condition.clone(),
                holds: owner_exit.holds.clone(),
            })
            .collect(),
    }
}

fn lower_entrypoint(
    entrypoint: &AotEntrypointArtifact,
    routines: &[RuntimeRoutinePlan],
) -> Result<RuntimeEntrypointPlan, String> {
    let (routine_index, routine) = routines
        .iter()
        .enumerate()
        .find(|(_, routine)| {
            routine.module_id == entrypoint.module_id
                && routine.symbol_name == entrypoint.symbol_name
                && routine.symbol_kind == entrypoint.symbol_kind
        })
        .ok_or_else(|| {
            format!(
                "entrypoint `{}` in module `{}` has no lowered runtime routine",
                entrypoint.symbol_name, entrypoint.module_id
            )
        })?;
    if entrypoint.symbol_kind == "fn" && entrypoint.symbol_name == RUNTIME_MAIN_ENTRYPOINT_NAME {
        validate_runtime_main_entry_contract(
            routine.params.len(),
            runtime_main_return_type_from_signature(&routine.signature_row),
        )?;
    }
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
    validate_package_artifact(artifact)?;
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
    let plan = RuntimePackagePlan {
        package_name: artifact.package_name.clone(),
        root_module_id: artifact.root_module_id.clone(),
        direct_deps: artifact.direct_deps.clone(),
        runtime_requirements: artifact.runtime_requirements.clone(),
        module_aliases: build_module_aliases(artifact)?,
        entrypoints,
        routines,
        owners: artifact.owners.iter().map(lower_owner).collect(),
    };
    validate_runtime_rollup_handlers(&plan)?;
    Ok(plan)
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

fn validate_runtime_rollup_handlers(plan: &RuntimePackagePlan) -> Result<(), String> {
    for routine in &plan.routines {
        for rollup in &routine.rollups {
            validate_runtime_rollup_handler_callable_path(
                plan,
                &routine.module_id,
                &rollup.handler_path,
            )?;
        }
        validate_runtime_rollup_handlers_in_statements(
            plan,
            &routine.module_id,
            &routine.statements,
        )?;
    }
    Ok(())
}

fn validate_runtime_rollup_handlers_in_statements(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    statements: &[ParsedStmt],
) -> Result<(), String> {
    for statement in statements {
        match statement {
            ParsedStmt::Expr { rollups, .. } => {
                for rollup in rollups {
                    validate_runtime_rollup_handler_callable_path(
                        plan,
                        current_module_id,
                        &rollup.handler_path,
                    )?;
                }
            }
            ParsedStmt::If {
                then_branch,
                else_branch,
                rollups,
                ..
            } => {
                for rollup in rollups {
                    validate_runtime_rollup_handler_callable_path(
                        plan,
                        current_module_id,
                        &rollup.handler_path,
                    )?;
                }
                validate_runtime_rollup_handlers_in_statements(
                    plan,
                    current_module_id,
                    then_branch,
                )?;
                validate_runtime_rollup_handlers_in_statements(
                    plan,
                    current_module_id,
                    else_branch,
                )?;
            }
            ParsedStmt::While { body, rollups, .. } | ParsedStmt::For { body, rollups, .. } => {
                for rollup in rollups {
                    validate_runtime_rollup_handler_callable_path(
                        plan,
                        current_module_id,
                        &rollup.handler_path,
                    )?;
                }
                validate_runtime_rollup_handlers_in_statements(plan, current_module_id, body)?;
            }
            ParsedStmt::Let { .. }
            | ParsedStmt::ReturnVoid
            | ParsedStmt::ReturnValue { .. }
            | ParsedStmt::ActivateOwner { .. }
            | ParsedStmt::Defer(_)
            | ParsedStmt::Break
            | ParsedStmt::Continue
            | ParsedStmt::Assign { .. } => {}
        }
    }
    Ok(())
}

fn strip_prefix_suffix<'a>(text: &'a str, prefix: &str, suffix: &str) -> Result<&'a str, String> {
    let inner = text
        .strip_prefix(prefix)
        .and_then(|value| value.strip_suffix(suffix))
        .ok_or_else(|| format!("malformed runtime row `{text}`"))?;
    Ok(inner)
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
    let payload = text
        .strip_prefix("name=")
        .ok_or_else(|| format!("runtime behavior attr row missing name in `{text}`"))?;
    let Some((name, value)) = payload.split_once(":value=") else {
        return Err(format!("malformed runtime behavior attr row `{text}`"));
    };
    let decode_part = |part: &str| {
        if part.starts_with('"') {
            decode_row_string(part)
        } else {
            Ok(part.to_string())
        }
    };
    let name = decode_part(name)?;
    let value = decode_part(value)?;
    if name.is_empty() {
        return Err(format!(
            "runtime behavior attr row missing name in `{text}`"
        ));
    }
    if value.is_empty() {
        return Err(format!(
            "runtime behavior attr row missing value in `{text}`"
        ));
    }
    Ok((name, value))
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

fn resolve_named_qualifier_callable_path(
    expr: &ParsedExpr,
    aliases: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    match expr {
        ParsedExpr::Path(segments) if segments.len() == 1 => resolve_callable_path(expr, aliases),
        ParsedExpr::Path(segments) => {
            if let Some(prefix) = aliases.get(&segments[0]) {
                let mut path = prefix.clone();
                path.extend(segments[1..].iter().cloned());
                Some(path)
            } else {
                Some(segments.clone())
            }
        }
        ParsedExpr::Generic { expr, .. } => resolve_named_qualifier_callable_path(expr, aliases),
        _ => resolve_callable_path(expr, aliases),
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

fn parse_runtime_callable_expr(text: &str) -> ParsedExpr {
    let trimmed = text.trim();
    let (path_text, type_args) = if let Some(start) = trimmed.rfind('[') {
        if trimmed.ends_with(']') {
            (
                trimmed[..start].trim(),
                split_runtime_type_args(&trimmed[start + 1..trimmed.len() - 1]),
            )
        } else {
            (trimmed, Vec::new())
        }
    } else {
        (trimmed, Vec::new())
    };
    let path = ParsedExpr::Path(path_text.split('.').map(ToString::to_string).collect());
    if type_args.is_empty() {
        path
    } else {
        ParsedExpr::Generic {
            expr: Box::new(path),
            type_args,
        }
    }
}

fn split_runtime_type_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for ch in text.chars() {
        match ch {
            '[' => {
                depth += 1;
                current.push(ch);
            }
            ']' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    args.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        args.push(trimmed.to_string());
    }
    args
}

fn parse_runtime_value_type_args(type_name: &str) -> Vec<String> {
    let Some(start) = type_name.rfind('[') else {
        return Vec::new();
    };
    if !type_name.ends_with(']') {
        return Vec::new();
    }
    split_runtime_type_args(&type_name[start + 1..type_name.len() - 1])
}

fn strip_runtime_reference_prefix(text: &str) -> &str {
    let trimmed = text.trim_start();
    if let Some(rest) = trimmed.strip_prefix("&mut") {
        return rest.trim_start();
    }
    if let Some(rest) = trimmed.strip_prefix('&') {
        return rest.trim_start();
    }
    trimmed
}

fn parse_runtime_type_application(type_name: &str) -> Option<(String, Vec<String>)> {
    let trimmed = strip_runtime_reference_prefix(type_name).trim();
    let Some(start) = trimmed.find('[') else {
        return Some((trimmed.to_string(), Vec::new()));
    };
    if !trimmed.ends_with(']') {
        return None;
    }
    Some((
        trimmed[..start].trim().to_string(),
        split_runtime_type_args(&trimmed[start + 1..trimmed.len() - 1]),
    ))
}

fn runtime_type_root_name(type_name: &str) -> String {
    let base = type_name
        .split_once('[')
        .map(|(head, _)| head)
        .unwrap_or(type_name)
        .trim();
    base.rsplit('.').next().unwrap_or(base).to_string()
}

fn runtime_type_name_matches(expected: &str, actual: &str) -> bool {
    expected == actual
        || expected.ends_with(&format!(".{actual}"))
        || actual.ends_with(&format!(".{expected}"))
}

fn runtime_declared_type_matches_actual(
    declared_type: &str,
    actual_type: &str,
    type_params: &BTreeSet<String>,
) -> bool {
    let Some((declared_base, declared_args)) = parse_runtime_type_application(declared_type) else {
        return false;
    };
    let Some((actual_base, actual_args)) = parse_runtime_type_application(actual_type) else {
        return false;
    };
    if !runtime_type_name_matches(&declared_base, &actual_base) {
        return false;
    }
    if declared_args.len() != actual_args.len() {
        return declared_args.is_empty() && actual_args.is_empty();
    }
    declared_args
        .iter()
        .zip(actual_args.iter())
        .all(|(declared_arg, actual_arg)| {
            let trimmed = declared_arg.trim();
            if type_params.contains(trimmed) {
                return true;
            }
            runtime_declared_type_matches_actual(trimmed, actual_arg, type_params)
        })
}

fn runtime_variant_enum_name(variant_name: &str) -> String {
    let mut segments = variant_name
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if segments.len() >= 2 {
        segments.pop();
        segments.join(".")
    } else {
        variant_name.to_string()
    }
}

fn push_runtime_call_frame(
    state: &mut RuntimeExecutionState,
    module_id: &str,
    symbol_name: &str,
) -> Result<(), String> {
    let frame = format!("{module_id}.{symbol_name}");
    if state.call_stack.len() >= RUNTIME_MAX_CALL_DEPTH {
        let mut trace = state.call_stack.clone();
        trace.push(frame);
        return Err(format!(
            "runtime call depth exceeded {RUNTIME_MAX_CALL_DEPTH}; trace: {}",
            trace.join(" -> ")
        ));
    }
    state.call_stack.push(frame);
    Ok(())
}

fn pop_runtime_call_frame(state: &mut RuntimeExecutionState) {
    let _ = state.call_stack.pop();
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

fn resolve_routine_candidate_indices(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    callable_path: &[String],
) -> Vec<usize> {
    let (module_id, symbol_name) = match callable_path {
        [] => return Vec::new(),
        [symbol_name] => (current_module_id.to_string(), symbol_name.clone()),
        _ => (
            callable_path[..callable_path.len() - 1].join("."),
            callable_path.last().cloned().unwrap_or_default(),
        ),
    };
    plan.routines
        .iter()
        .enumerate()
        .filter_map(|(index, routine)| {
            (routine.module_id == module_id && routine.symbol_name == symbol_name).then_some(index)
        })
        .collect()
}

fn resolve_dynamic_method_candidate_indices(
    plan: &RuntimePackagePlan,
    method_name: &str,
    trait_path: &[String],
) -> Vec<usize> {
    plan.routines
        .iter()
        .enumerate()
        .filter_map(|(index, routine)| {
            (routine.symbol_name == method_name
                && routine.impl_trait_path.as_deref() == Some(trait_path))
            .then_some(index)
        })
        .collect()
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
        ["std", "concurrent", "channel"] | ["std", "kernel", "concurrency", "channel_new"] => {
            Some(RuntimeIntrinsic::ConcurrentChannelNew)
        }
        ["std", "concurrent", "mutex"] | ["std", "kernel", "concurrency", "mutex_new"] => {
            Some(RuntimeIntrinsic::ConcurrentMutexNew)
        }
        ["std", "concurrent", "atomic_int"]
        | ["std", "kernel", "concurrency", "atomic_int_new"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntNew)
        }
        ["std", "concurrent", "atomic_bool"]
        | ["std", "kernel", "concurrency", "atomic_bool_new"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolNew)
        }
        ["std", "concurrent", "sleep"] | ["std", "kernel", "concurrency", "sleep"] => {
            Some(RuntimeIntrinsic::ConcurrentSleep)
        }
        ["std", "concurrent", "thread_id"] | ["std", "kernel", "concurrency", "thread_id"] => {
            Some(RuntimeIntrinsic::ConcurrentThreadId)
        }
        ["std", "kernel", "concurrency", "task_done"] => Some(RuntimeIntrinsic::ConcurrentTaskDone),
        ["std", "kernel", "concurrency", "task_join"] => Some(RuntimeIntrinsic::ConcurrentTaskJoin),
        ["std", "kernel", "concurrency", "thread_done"] => {
            Some(RuntimeIntrinsic::ConcurrentThreadDone)
        }
        ["std", "kernel", "concurrency", "thread_join"] => {
            Some(RuntimeIntrinsic::ConcurrentThreadJoin)
        }
        ["std", "memory", "new"] | ["std", "kernel", "memory", "arena_new"] => {
            Some(RuntimeIntrinsic::MemoryArenaNew)
        }
        ["std", "memory", "frame_new"] | ["std", "kernel", "memory", "frame_new"] => {
            Some(RuntimeIntrinsic::MemoryFrameNew)
        }
        ["std", "memory", "pool_new"] | ["std", "kernel", "memory", "pool_new"] => {
            Some(RuntimeIntrinsic::MemoryPoolNew)
        }
        ["std", "kernel", "memory", "arena_alloc"] => Some(RuntimeIntrinsic::MemoryArenaAlloc),
        ["std", "kernel", "memory", "arena_len"] => Some(RuntimeIntrinsic::MemoryArenaLen),
        ["std", "kernel", "memory", "arena_has"] => Some(RuntimeIntrinsic::MemoryArenaHas),
        ["std", "kernel", "memory", "arena_get"] => Some(RuntimeIntrinsic::MemoryArenaGet),
        ["std", "kernel", "memory", "arena_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryArenaBorrowRead)
        }
        ["std", "kernel", "memory", "arena_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryArenaBorrowEdit)
        }
        ["std", "kernel", "memory", "arena_set"] => Some(RuntimeIntrinsic::MemoryArenaSet),
        ["std", "kernel", "memory", "arena_remove"] => Some(RuntimeIntrinsic::MemoryArenaRemove),
        ["std", "kernel", "memory", "arena_reset"] => Some(RuntimeIntrinsic::MemoryArenaReset),
        ["std", "kernel", "memory", "frame_alloc"] => Some(RuntimeIntrinsic::MemoryFrameAlloc),
        ["std", "kernel", "memory", "frame_len"] => Some(RuntimeIntrinsic::MemoryFrameLen),
        ["std", "kernel", "memory", "frame_has"] => Some(RuntimeIntrinsic::MemoryFrameHas),
        ["std", "kernel", "memory", "frame_get"] => Some(RuntimeIntrinsic::MemoryFrameGet),
        ["std", "kernel", "memory", "frame_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryFrameBorrowRead)
        }
        ["std", "kernel", "memory", "frame_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryFrameBorrowEdit)
        }
        ["std", "kernel", "memory", "frame_set"] => Some(RuntimeIntrinsic::MemoryFrameSet),
        ["std", "kernel", "memory", "frame_reset"] => Some(RuntimeIntrinsic::MemoryFrameReset),
        ["std", "kernel", "memory", "pool_alloc"] => Some(RuntimeIntrinsic::MemoryPoolAlloc),
        ["std", "kernel", "memory", "pool_len"] => Some(RuntimeIntrinsic::MemoryPoolLen),
        ["std", "kernel", "memory", "pool_has"] => Some(RuntimeIntrinsic::MemoryPoolHas),
        ["std", "kernel", "memory", "pool_get"] => Some(RuntimeIntrinsic::MemoryPoolGet),
        ["std", "kernel", "memory", "pool_borrow_read"] => {
            Some(RuntimeIntrinsic::MemoryPoolBorrowRead)
        }
        ["std", "kernel", "memory", "pool_borrow_edit"] => {
            Some(RuntimeIntrinsic::MemoryPoolBorrowEdit)
        }
        ["std", "kernel", "memory", "pool_set"] => Some(RuntimeIntrinsic::MemoryPoolSet),
        ["std", "kernel", "memory", "pool_remove"] => Some(RuntimeIntrinsic::MemoryPoolRemove),
        ["std", "kernel", "memory", "pool_reset"] => Some(RuntimeIntrinsic::MemoryPoolReset),
        ["std", "kernel", "concurrency", "channel_send"] => {
            Some(RuntimeIntrinsic::ConcurrentChannelSend)
        }
        ["std", "kernel", "concurrency", "channel_recv"] => {
            Some(RuntimeIntrinsic::ConcurrentChannelRecv)
        }
        ["std", "kernel", "concurrency", "mutex_take"] => {
            Some(RuntimeIntrinsic::ConcurrentMutexTake)
        }
        ["std", "kernel", "concurrency", "mutex_put"] => Some(RuntimeIntrinsic::ConcurrentMutexPut),
        ["std", "kernel", "concurrency", "atomic_int_load"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntLoad)
        }
        ["std", "kernel", "concurrency", "atomic_int_store"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntStore)
        }
        ["std", "kernel", "concurrency", "atomic_int_add"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntAdd)
        }
        ["std", "kernel", "concurrency", "atomic_int_sub"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntSub)
        }
        ["std", "kernel", "concurrency", "atomic_int_swap"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicIntSwap)
        }
        ["std", "kernel", "concurrency", "atomic_bool_load"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolLoad)
        }
        ["std", "kernel", "concurrency", "atomic_bool_store"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolStore)
        }
        ["std", "kernel", "concurrency", "atomic_bool_swap"] => {
            Some(RuntimeIntrinsic::ConcurrentAtomicBoolSwap)
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
        ["std", "kernel", "collections", "list_len"] => Some(RuntimeIntrinsic::ListLen),
        ["std", "kernel", "collections", "list_push"] => Some(RuntimeIntrinsic::ListPush),
        ["std", "kernel", "collections", "list_pop"] => Some(RuntimeIntrinsic::ListPop),
        ["std", "kernel", "collections", "list_try_pop_or"] => Some(RuntimeIntrinsic::ListTryPopOr),
        ["std", "kernel", "collections", "array_new"] => Some(RuntimeIntrinsic::ArrayNew),
        ["std", "kernel", "collections", "array_len"] => Some(RuntimeIntrinsic::ArrayLen),
        ["std", "kernel", "collections", "array_from_list"] => {
            Some(RuntimeIntrinsic::ArrayFromList)
        }
        ["std", "kernel", "collections", "array_to_list"] => Some(RuntimeIntrinsic::ArrayToList),
        ["std", "kernel", "collections", "map_new"] => Some(RuntimeIntrinsic::MapNew),
        ["std", "kernel", "collections", "map_len"] => Some(RuntimeIntrinsic::MapLen),
        ["std", "kernel", "collections", "map_has"] => Some(RuntimeIntrinsic::MapHas),
        ["std", "kernel", "collections", "map_get"] => Some(RuntimeIntrinsic::MapGet),
        ["std", "kernel", "collections", "map_set"] => Some(RuntimeIntrinsic::MapSet),
        ["std", "kernel", "collections", "map_remove"] => Some(RuntimeIntrinsic::MapRemove),
        ["std", "kernel", "collections", "map_try_get_or"] => Some(RuntimeIntrinsic::MapTryGetOr),
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
        "ConcurrentThreadId" => Some(RuntimeIntrinsic::ConcurrentThreadId),
        "ConcurrentTaskDone" => Some(RuntimeIntrinsic::ConcurrentTaskDone),
        "ConcurrentTaskJoin" => Some(RuntimeIntrinsic::ConcurrentTaskJoin),
        "ConcurrentThreadDone" => Some(RuntimeIntrinsic::ConcurrentThreadDone),
        "ConcurrentThreadJoin" => Some(RuntimeIntrinsic::ConcurrentThreadJoin),
        "ConcurrentChannelNew" => Some(RuntimeIntrinsic::ConcurrentChannelNew),
        "ConcurrentChannelSend" => Some(RuntimeIntrinsic::ConcurrentChannelSend),
        "ConcurrentChannelRecv" => Some(RuntimeIntrinsic::ConcurrentChannelRecv),
        "ConcurrentMutexNew" => Some(RuntimeIntrinsic::ConcurrentMutexNew),
        "ConcurrentMutexTake" => Some(RuntimeIntrinsic::ConcurrentMutexTake),
        "ConcurrentMutexPut" => Some(RuntimeIntrinsic::ConcurrentMutexPut),
        "ConcurrentAtomicIntNew" => Some(RuntimeIntrinsic::ConcurrentAtomicIntNew),
        "ConcurrentAtomicIntLoad" => Some(RuntimeIntrinsic::ConcurrentAtomicIntLoad),
        "ConcurrentAtomicIntStore" => Some(RuntimeIntrinsic::ConcurrentAtomicIntStore),
        "ConcurrentAtomicIntAdd" => Some(RuntimeIntrinsic::ConcurrentAtomicIntAdd),
        "ConcurrentAtomicIntSub" => Some(RuntimeIntrinsic::ConcurrentAtomicIntSub),
        "ConcurrentAtomicIntSwap" => Some(RuntimeIntrinsic::ConcurrentAtomicIntSwap),
        "ConcurrentAtomicBoolNew" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolNew),
        "ConcurrentAtomicBoolLoad" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolLoad),
        "ConcurrentAtomicBoolStore" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolStore),
        "ConcurrentAtomicBoolSwap" => Some(RuntimeIntrinsic::ConcurrentAtomicBoolSwap),
        "MemoryArenaNew" => Some(RuntimeIntrinsic::MemoryArenaNew),
        "MemoryArenaAlloc" => Some(RuntimeIntrinsic::MemoryArenaAlloc),
        "MemoryArenaLen" => Some(RuntimeIntrinsic::MemoryArenaLen),
        "MemoryArenaHas" => Some(RuntimeIntrinsic::MemoryArenaHas),
        "MemoryArenaGet" => Some(RuntimeIntrinsic::MemoryArenaGet),
        "MemoryArenaBorrowRead" => Some(RuntimeIntrinsic::MemoryArenaBorrowRead),
        "MemoryArenaBorrowEdit" => Some(RuntimeIntrinsic::MemoryArenaBorrowEdit),
        "MemoryArenaSet" => Some(RuntimeIntrinsic::MemoryArenaSet),
        "MemoryArenaRemove" => Some(RuntimeIntrinsic::MemoryArenaRemove),
        "MemoryArenaReset" => Some(RuntimeIntrinsic::MemoryArenaReset),
        "MemoryFrameNew" => Some(RuntimeIntrinsic::MemoryFrameNew),
        "MemoryFrameAlloc" => Some(RuntimeIntrinsic::MemoryFrameAlloc),
        "MemoryFrameLen" => Some(RuntimeIntrinsic::MemoryFrameLen),
        "MemoryFrameHas" => Some(RuntimeIntrinsic::MemoryFrameHas),
        "MemoryFrameGet" => Some(RuntimeIntrinsic::MemoryFrameGet),
        "MemoryFrameBorrowRead" => Some(RuntimeIntrinsic::MemoryFrameBorrowRead),
        "MemoryFrameBorrowEdit" => Some(RuntimeIntrinsic::MemoryFrameBorrowEdit),
        "MemoryFrameSet" => Some(RuntimeIntrinsic::MemoryFrameSet),
        "MemoryFrameReset" => Some(RuntimeIntrinsic::MemoryFrameReset),
        "MemoryPoolNew" => Some(RuntimeIntrinsic::MemoryPoolNew),
        "MemoryPoolAlloc" => Some(RuntimeIntrinsic::MemoryPoolAlloc),
        "MemoryPoolLen" => Some(RuntimeIntrinsic::MemoryPoolLen),
        "MemoryPoolHas" => Some(RuntimeIntrinsic::MemoryPoolHas),
        "MemoryPoolGet" => Some(RuntimeIntrinsic::MemoryPoolGet),
        "MemoryPoolBorrowRead" => Some(RuntimeIntrinsic::MemoryPoolBorrowRead),
        "MemoryPoolBorrowEdit" => Some(RuntimeIntrinsic::MemoryPoolBorrowEdit),
        "MemoryPoolSet" => Some(RuntimeIntrinsic::MemoryPoolSet),
        "MemoryPoolRemove" => Some(RuntimeIntrinsic::MemoryPoolRemove),
        "MemoryPoolReset" => Some(RuntimeIntrinsic::MemoryPoolReset),
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
        "ListLen" => Some(RuntimeIntrinsic::ListLen),
        "ListPush" => Some(RuntimeIntrinsic::ListPush),
        "ListPop" => Some(RuntimeIntrinsic::ListPop),
        "ListTryPopOr" => Some(RuntimeIntrinsic::ListTryPopOr),
        "ArrayNew" => Some(RuntimeIntrinsic::ArrayNew),
        "ArrayLen" => Some(RuntimeIntrinsic::ArrayLen),
        "ArrayFromList" => Some(RuntimeIntrinsic::ArrayFromList),
        "ArrayToList" => Some(RuntimeIntrinsic::ArrayToList),
        "MapNew" => Some(RuntimeIntrinsic::MapNew),
        "MapLen" => Some(RuntimeIntrinsic::MapLen),
        "MapHas" => Some(RuntimeIntrinsic::MapHas),
        "MapGet" => Some(RuntimeIntrinsic::MapGet),
        "MapSet" => Some(RuntimeIntrinsic::MapSet),
        "MapRemove" => Some(RuntimeIntrinsic::MapRemove),
        "MapTryGetOr" => Some(RuntimeIntrinsic::MapTryGetOr),
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
        RuntimeValue::Map(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(key, value)| format!(
                    "{}: {}",
                    runtime_value_to_string(key),
                    runtime_value_to_string(value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::Range {
            start,
            end,
            inclusive_end,
        } => {
            let mut rendered = String::new();
            if let Some(start) = start {
                rendered.push_str(&start.to_string());
            }
            rendered.push_str(if *inclusive_end { "..=" } else { ".." });
            if let Some(end) = end {
                rendered.push_str(&end.to_string());
            }
            rendered
        }
        RuntimeValue::OwnerHandle(owner_key) => format!("<Owner:{owner_key}>"),
        RuntimeValue::Ref(reference) => match &reference.target {
            RuntimeReferenceTarget::Local { local, members } => {
                if members.is_empty() {
                    format!("<ref:{}>", local.0)
                } else {
                    format!("<ref:{}:{}>", local.0, members.join("."))
                }
            }
            RuntimeReferenceTarget::OwnerObject {
                owner_key,
                object_name,
                members,
            } => {
                if members.is_empty() {
                    format!("<ref:Owner:{owner_key}:{object_name}>")
                } else {
                    format!(
                        "<ref:Owner:{owner_key}:{object_name}:{}>",
                        members.join(".")
                    )
                }
            }
            RuntimeReferenceTarget::ArenaSlot { id, members } => {
                if members.is_empty() {
                    format!("<ref:Arena:{}:{}:{}>", id.arena.0, id.slot, id.generation)
                } else {
                    format!(
                        "<ref:Arena:{}:{}:{}:{}>",
                        id.arena.0,
                        id.slot,
                        id.generation,
                        members.join(".")
                    )
                }
            }
            RuntimeReferenceTarget::FrameSlot { id, members } => {
                if members.is_empty() {
                    format!("<ref:Frame:{}:{}:{}>", id.arena.0, id.slot, id.generation)
                } else {
                    format!(
                        "<ref:Frame:{}:{}:{}:{}>",
                        id.arena.0,
                        id.slot,
                        id.generation,
                        members.join(".")
                    )
                }
            }
            RuntimeReferenceTarget::PoolSlot { id, members } => {
                if members.is_empty() {
                    format!("<ref:Pool:{}:{}:{}>", id.arena.0, id.slot, id.generation)
                } else {
                    format!(
                        "<ref:Pool:{}:{}:{}:{}>",
                        id.arena.0,
                        id.slot,
                        id.generation,
                        members.join(".")
                    )
                }
            }
        },
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
        RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(handle)) => {
            format!("<Channel:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(handle)) => {
            format!("<Mutex:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicInt(handle)) => {
            format!("<AtomicInt:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicBool(handle)) => {
            format!("<AtomicBool:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(handle)) => {
            format!("<Arena:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id)) => {
            format!("<ArenaId:{}:{}:{}>", id.arena.0, id.slot, id.generation)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle)) => {
            format!("<FrameArena:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id)) => {
            format!("<FrameId:{}:{}:{}>", id.arena.0, id.slot, id.generation)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(handle)) => {
            format!("<PoolArena:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id)) => {
            format!("<PoolId:{}:{}:{}>", id.arena.0, id.slot, id.generation)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Task(handle)) => {
            format!("<Task:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(handle)) => {
            format!("<Thread:{}>", handle.0)
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

fn read_runtime_local_value(scopes: &[RuntimeScope], name: &str) -> Result<RuntimeValue, String> {
    let local = lookup_local(scopes, name)
        .ok_or_else(|| format!("unsupported runtime value path `{name}`"))?;
    if local.moved {
        return Err(format!("use of moved local `{name}`"));
    }
    Ok(local.value.clone())
}

fn lookup_local_mut_by_handle<'a>(
    scopes: &'a mut [RuntimeScope],
    handle: RuntimeLocalHandle,
) -> Option<&'a mut RuntimeLocal> {
    scopes.iter_mut().rev().find_map(|scope| {
        scope
            .locals
            .values_mut()
            .find(|local| local.handle == handle)
    })
}

fn lookup_local_with_name_by_handle<'a>(
    scopes: &'a [RuntimeScope],
    handle: RuntimeLocalHandle,
) -> Option<(&'a str, &'a RuntimeLocal)> {
    scopes.iter().rev().find_map(|scope| {
        scope
            .locals
            .iter()
            .find_map(|(name, local)| (local.handle == handle).then_some((name.as_str(), local)))
    })
}

fn push_runtime_rollup_frame(
    state: &mut RuntimeExecutionState,
    rollups: &[ParsedPageRollup],
    scopes: &[RuntimeScope],
) {
    if rollups.is_empty() {
        return;
    }
    let subjects = rollups
        .iter()
        .map(|rollup| rollup.subject.clone())
        .map(|subject| {
            (
                subject,
                RuntimeTrackedRollupSubject {
                    binding: None,
                    value: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    state.rollup_frames.push(RuntimeRollupFrame {
        rollups: rollups.to_vec(),
        owner_scope_depth: scopes.len(),
        subjects,
    });
}

fn pop_runtime_rollup_frame(state: &mut RuntimeExecutionState) -> Option<RuntimeRollupFrame> {
    state.rollup_frames.pop()
}

fn activate_runtime_rollup_binding(
    state: &mut RuntimeExecutionState,
    scope_depth: usize,
    name: &str,
    handle: RuntimeLocalHandle,
    value: &RuntimeValue,
) {
    for frame in state.rollup_frames.iter_mut().rev() {
        if frame.owner_scope_depth != scope_depth {
            continue;
        }
        if let Some(subject) = frame.subjects.get_mut(name) {
            subject.binding = Some(handle);
            subject.value = Some(value.clone());
        }
    }
}

fn update_runtime_rollup_binding_value(
    state: &mut RuntimeExecutionState,
    handle: RuntimeLocalHandle,
    value: &RuntimeValue,
) {
    for frame in state.rollup_frames.iter_mut().rev() {
        for subject in frame.subjects.values_mut() {
            if subject.binding == Some(handle) {
                subject.value = Some(value.clone());
            }
        }
    }
}

fn insert_runtime_local(
    state: &mut RuntimeExecutionState,
    scope_depth: usize,
    scope: &mut RuntimeScope,
    name: String,
    mutable: bool,
    value: RuntimeValue,
) {
    let handle = RuntimeLocalHandle(state.next_local_handle);
    state.next_local_handle += 1;
    activate_runtime_rollup_binding(state, scope_depth, &name, handle, &value);
    scope.locals.insert(
        name,
        RuntimeLocal {
            handle,
            mutable,
            moved: false,
            value,
        },
    );
}

fn apply_runtime_availability_attachments(
    scope: &mut RuntimeScope,
    attachments: &[ParsedAvailabilityAttachment],
) {
    for attachment in attachments {
        if matches!(attachment.kind, ParsedAvailabilityKind::Object) {
            scope
                .attached_object_names
                .insert(attachment.local_name.clone());
        }
    }
}

fn owner_state_key(owner_path: &[String]) -> String {
    owner_path.join(".")
}

fn lookup_runtime_owner_plan<'a>(
    plan: &'a RuntimePackagePlan,
    owner_path: &[String],
) -> Option<&'a RuntimeOwnerPlan> {
    plan.owners
        .iter()
        .find(|owner| owner.owner_path == owner_path)
}

fn attached_object_is_visible(scopes: &[RuntimeScope], name: &str) -> bool {
    scopes
        .iter()
        .rev()
        .any(|scope| scope.attached_object_names.contains(name))
}

fn make_owner_object_reference(owner_key: &str, object_name: &str) -> RuntimeValue {
    RuntimeValue::Ref(RuntimeReferenceValue {
        mutable: true,
        target: RuntimeReferenceTarget::OwnerObject {
            owner_key: owner_key.to_string(),
            object_name: object_name.to_string(),
            members: Vec::new(),
        },
    })
}

fn inactive_owner_message(owner_path: &[String]) -> String {
    format!(
        "owner `{}` is not active; explicit re-entry is required",
        owner_path.join(".")
    )
}

fn lookup_runtime_owner_object_plan<'a>(
    owner: &'a RuntimeOwnerPlan,
    object_name: &str,
) -> Result<&'a RuntimeOwnerObjectPlan, String> {
    owner
        .objects
        .iter()
        .find(|object| object.local_name == object_name)
        .ok_or_else(|| {
            format!(
                "runtime owner `{}` does not declare owned object `{object_name}`",
                owner.owner_path.join(".")
            )
        })
}

fn realize_owner_object_value(
    plan: &RuntimePackagePlan,
    owner_path: &[String],
    object_name: &str,
) -> Result<RuntimeValue, String> {
    let owner = lookup_runtime_owner_plan(plan, owner_path).ok_or_else(|| {
        format!(
            "runtime owner `{}` is not declared in the package plan",
            owner_path.join(".")
        )
    })?;
    let object = lookup_runtime_owner_object_plan(owner, object_name)?;
    Ok(RuntimeValue::Record {
        name: object.type_path.join("."),
        fields: BTreeMap::new(),
    })
}

fn resolve_routine_index_by_key(
    plan: &RuntimePackagePlan,
    routine_key: &str,
) -> Result<usize, String> {
    plan.routines
        .iter()
        .position(|routine| routine.routine_key == routine_key)
        .ok_or_else(|| {
            format!("runtime routine `{routine_key}` is not present in the package plan")
        })
}

fn execute_owner_object_lifecycle_hook(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    owner_key: &str,
    object_name: &str,
) -> Result<(), String> {
    let owner_path = owner_key
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let owner = lookup_runtime_owner_plan(plan, &owner_path).ok_or_else(|| {
        format!(
            "runtime owner `{}` is not declared in the package plan",
            owner_key
        )
    })?;
    let object = lookup_runtime_owner_object_plan(owner, object_name)?;

    let (current_value, activation_context, needs_init, needs_resume) = {
        let owner_state = state
            .owners
            .get_mut(owner_key)
            .ok_or_else(|| inactive_owner_message(&owner_path))?;
        if owner_state.active_bindings == 0 {
            return Err(inactive_owner_message(&owner_path));
        }
        let (object_value, needs_init) = match owner_state.objects.get(object_name) {
            Some(value) => (
                value.clone(),
                owner_state.pending_init.contains(object_name),
            ),
            None => {
                let value = realize_owner_object_value(plan, &owner_path, object_name)?;
                owner_state
                    .objects
                    .insert(object_name.to_string(), value.clone());
                owner_state.pending_init.insert(object_name.to_string());
                (value, true)
            }
        };
        (
            object_value,
            owner_state.activation_context.clone(),
            needs_init,
            owner_state.pending_resume.contains(object_name),
        )
    };

    let (routine_key, args, missing_context_message) = if needs_resume {
        if let Some(context) = activation_context.clone() {
            if let Some(routine_key) = object.resume_with_context_routine_key.clone() {
                (
                    Some(routine_key),
                    vec![current_value.clone(), context],
                    None,
                )
            } else if let Some(routine_key) = object.resume_routine_key.clone() {
                (Some(routine_key), vec![current_value.clone()], None)
            } else {
                (None, Vec::new(), None)
            }
        } else if let Some(routine_key) = object.resume_routine_key.clone() {
            (Some(routine_key), vec![current_value.clone()], None)
        } else if object.resume_with_context_routine_key.is_some() {
            (
                None,
                Vec::new(),
                Some(format!(
                    "owner `{}` object `{}` requires an activation context to resume",
                    owner.owner_path.join("."),
                    object_name
                )),
            )
        } else {
            (None, Vec::new(), None)
        }
    } else if needs_init {
        if let Some(context) = activation_context.clone() {
            if let Some(routine_key) = object.init_with_context_routine_key.clone() {
                (
                    Some(routine_key),
                    vec![current_value.clone(), context],
                    None,
                )
            } else if let Some(routine_key) = object.init_routine_key.clone() {
                (Some(routine_key), vec![current_value.clone()], None)
            } else {
                (None, Vec::new(), None)
            }
        } else if let Some(routine_key) = object.init_routine_key.clone() {
            (Some(routine_key), vec![current_value.clone()], None)
        } else if object.init_with_context_routine_key.is_some() {
            (
                None,
                Vec::new(),
                Some(format!(
                    "owner `{}` object `{}` requires an activation context to initialize",
                    owner.owner_path.join("."),
                    object_name
                )),
            )
        } else {
            (None, Vec::new(), None)
        }
    } else if let Some(context) = activation_context.clone() {
        let _ = context;
        (None, Vec::new(), None)
    } else {
        (None, Vec::new(), None)
    };

    if let Some(message) = missing_context_message {
        return Err(message);
    }

    if let Some(routine_key) = routine_key {
        let routine_index = resolve_routine_index_by_key(plan, &routine_key)?;
        let outcome = execute_routine_call_with_state(
            plan,
            routine_index,
            Vec::new(),
            args,
            state,
            host,
            false,
        )?;
        if outcome.value != RuntimeValue::Unit {
            return Err(format!(
                "owner lifecycle hook for `{}` must return Unit",
                object_name
            ));
        }
        let updated_value = outcome.final_args.first().cloned().ok_or_else(|| {
            format!(
                "owner lifecycle hook for `{}` did not preserve `self`",
                object_name
            )
        })?;
        let owner_state = state
            .owners
            .entry(owner_key.to_string())
            .or_insert_with(RuntimeOwnerState::default);
        owner_state
            .objects
            .insert(object_name.to_string(), updated_value);
        owner_state.pending_init.remove(object_name);
        owner_state.pending_resume.remove(object_name);
        let owner_keys = vec![owner_key.to_string()];
        evaluate_owner_exit_checkpoints(
            &owner_keys,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
            Some(scopes),
        )?;
    }

    let owner_state = state
        .owners
        .get_mut(owner_key)
        .ok_or_else(|| inactive_owner_message(&owner_path))?;
    owner_state.pending_init.remove(object_name);
    owner_state.pending_resume.remove(object_name);
    if owner_state.active_bindings == 0 {
        return Err(inactive_owner_message(&owner_path));
    }
    Ok(())
}

fn owner_object_root_value(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    owner_key: &str,
    object_name: &str,
) -> Result<RuntimeValue, String> {
    let owner_path = owner_key
        .split('.')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let owner_state = state
        .owners
        .get(owner_key)
        .ok_or_else(|| inactive_owner_message(&owner_path))?;
    if owner_state.active_bindings == 0 {
        return Err(inactive_owner_message(&owner_path));
    }
    let _ = owner_state;
    execute_owner_object_lifecycle_hook(
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
        owner_key,
        object_name,
    )?;
    state
        .owners
        .get(owner_key)
        .ok_or_else(|| inactive_owner_message(&owner_path))?
        .objects
        .get(object_name)
        .cloned()
        .ok_or_else(|| {
            format!(
                "runtime owner `{}` failed to realize object `{object_name}`",
                owner_path.join(".")
            )
        })
}

fn owner_exit_eval_scope(
    state: &mut RuntimeExecutionState,
    owner: &RuntimeOwnerPlan,
    owner_key: &str,
) -> Result<Vec<RuntimeScope>, String> {
    let mut scope = RuntimeScope::default();
    insert_runtime_local(
        state,
        0,
        &mut scope,
        owner.owner_name.clone(),
        false,
        RuntimeValue::OwnerHandle(owner_key.to_string()),
    );
    for object in &owner.objects {
        insert_runtime_local(
            state,
            0,
            &mut scope,
            object.local_name.clone(),
            true,
            make_owner_object_reference(owner_key, &object.local_name),
        );
    }
    Ok(vec![scope])
}

fn invalidate_owner_activations_in_scopes(scopes: &mut [RuntimeScope], owner_key: &str) {
    for scope in scopes {
        scope
            .activated_owner_keys
            .retain(|active| active != owner_key);
    }
}

fn release_scope_owner_activations(state: &mut RuntimeExecutionState, owner_keys: &[String]) {
    let mut released = BTreeSet::new();
    for owner_key in owner_keys {
        if !released.insert(owner_key.clone()) {
            continue;
        }
        if let Some(owner_state) = state.owners.get_mut(owner_key) {
            owner_state.active_bindings = owner_state.active_bindings.saturating_sub(1);
            if owner_state.active_bindings == 0 {
                owner_state.activation_context = None;
                owner_state.pending_init.clear();
                owner_state.pending_resume.clear();
            }
        }
    }
}

fn evaluate_owner_exit_checkpoints(
    owner_keys: &[String],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    mut scopes: Option<&mut Vec<RuntimeScope>>,
) -> Result<(), String> {
    let mut unique_keys = BTreeSet::new();
    for owner_key in owner_keys {
        if !unique_keys.insert(owner_key.clone()) {
            continue;
        }
        let owner_path = owner_key
            .split('.')
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let Some(owner) = lookup_runtime_owner_plan(plan, &owner_path).cloned() else {
            return Err(format!(
                "runtime owner `{}` is not declared in the package plan",
                owner_key
            ));
        };
        let Some(owner_state) = state.owners.get(owner_key) else {
            continue;
        };
        if owner_state.active_bindings == 0 {
            continue;
        }
        let mut exit_scopes = owner_exit_eval_scope(state, &owner, owner_key)?;
        let mut selected_exit = None;
        for owner_exit in &owner.exits {
            let condition = eval_expr(
                &owner_exit.condition,
                plan,
                current_module_id,
                &mut exit_scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
            .map_err(runtime_eval_message)?;
            if expect_bool(condition, "owner exit condition")? {
                selected_exit = Some(owner_exit.clone());
                break;
            }
        }
        if let Some(owner_exit) = selected_exit {
            let owner_state = state
                .owners
                .entry(owner_key.clone())
                .or_insert_with(RuntimeOwnerState::default);
            owner_state
                .objects
                .retain(|name, _| owner_exit.holds.iter().any(|hold| hold == name));
            owner_state.pending_init.clear();
            owner_state.pending_resume.clear();
            owner_state.activation_context = None;
            owner_state.active_bindings = 0;
            if let Some(scopes) = scopes.as_deref_mut() {
                invalidate_owner_activations_in_scopes(scopes, owner_key);
            }
        }
    }
    Ok(())
}

fn activate_owner_scope_binding(
    scopes: &mut Vec<RuntimeScope>,
    state: &mut RuntimeExecutionState,
    owner: &RuntimeOwnerPlan,
    owner_key: &str,
    binding: Option<&str>,
) -> Result<(), String> {
    let scope_depth = scopes.len().saturating_sub(1);
    let visible_objects = owner
        .objects
        .iter()
        .filter(|object| attached_object_is_visible(scopes, &object.local_name))
        .map(|object| object.local_name.clone())
        .collect::<Vec<_>>();
    let scope = scopes
        .last_mut()
        .ok_or_else(|| "runtime scope stack is empty".to_string())?;
    let newly_activated = !scope
        .activated_owner_keys
        .iter()
        .any(|active| active == owner_key);
    if newly_activated {
        scope.activated_owner_keys.push(owner_key.to_string());
        state
            .owners
            .entry(owner_key.to_string())
            .or_default()
            .active_bindings += 1;
    }
    insert_runtime_local(
        state,
        scope_depth,
        scope,
        owner.owner_name.clone(),
        false,
        RuntimeValue::OwnerHandle(owner_key.to_string()),
    );
    if let Some(binding) = binding {
        insert_runtime_local(
            state,
            scope_depth,
            scope,
            binding.to_string(),
            false,
            RuntimeValue::OwnerHandle(owner_key.to_string()),
        );
    }
    for object_name in visible_objects {
        insert_runtime_local(
            state,
            scope_depth,
            scope,
            object_name.clone(),
            true,
            make_owner_object_reference(owner_key, &object_name),
        );
    }
    Ok(())
}

fn runtime_reference_with_member(
    target: &RuntimeReferenceTarget,
    member: String,
) -> RuntimeReferenceTarget {
    match target {
        RuntimeReferenceTarget::Local { local, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::Local {
                local: *local,
                members: next_members,
            }
        }
        RuntimeReferenceTarget::OwnerObject {
            owner_key,
            object_name,
            members,
        } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::OwnerObject {
                owner_key: owner_key.clone(),
                object_name: object_name.clone(),
                members: next_members,
            }
        }
        RuntimeReferenceTarget::ArenaSlot { id, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::ArenaSlot {
                id: *id,
                members: next_members,
            }
        }
        RuntimeReferenceTarget::FrameSlot { id, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::FrameSlot {
                id: *id,
                members: next_members,
            }
        }
        RuntimeReferenceTarget::PoolSlot { id, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::PoolSlot {
                id: *id,
                members: next_members,
            }
        }
    }
}

fn runtime_reference_members(target: &RuntimeReferenceTarget) -> &[String] {
    match target {
        RuntimeReferenceTarget::Local { members, .. }
        | RuntimeReferenceTarget::OwnerObject { members, .. }
        | RuntimeReferenceTarget::ArenaSlot { members, .. }
        | RuntimeReferenceTarget::FrameSlot { members, .. }
        | RuntimeReferenceTarget::PoolSlot { members, .. } => members,
    }
}

fn runtime_reference_root_value(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    target: &RuntimeReferenceTarget,
) -> Result<RuntimeValue, String> {
    match target {
        RuntimeReferenceTarget::Local { local, .. } => {
            let (name, runtime_local) = lookup_local_with_name_by_handle(scopes, *local)
                .ok_or_else(|| format!("runtime reference local `{}` is unresolved", local.0))?;
            if runtime_local.moved {
                return Err(format!("use of moved local `{name}`"));
            }
            Ok(runtime_local.value.clone())
        }
        RuntimeReferenceTarget::OwnerObject {
            owner_key,
            object_name,
            ..
        } => owner_object_root_value(
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
            owner_key,
            object_name,
        ),
        RuntimeReferenceTarget::ArenaSlot { id, .. } => {
            let arena = state
                .arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid Arena handle `{}`", id.arena.0))?;
            if !arena_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("Arena slot `{}` is missing", id.slot))
        }
        RuntimeReferenceTarget::FrameSlot { id, .. } => {
            let arena = state
                .frame_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", id.arena.0))?;
            if !frame_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("FrameArena slot `{}` is missing", id.slot))
        }
        RuntimeReferenceTarget::PoolSlot { id, .. } => {
            let arena = state
                .pool_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", id.arena.0))?;
            if !pool_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(*id))),
                    id.arena.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("PoolArena slot `{}` is missing", id.slot))
        }
    }
}

fn assign_member_chain(
    base: RuntimeValue,
    members: &[String],
    value: RuntimeValue,
) -> Result<RuntimeValue, String> {
    let Some((member, rest)) = members.split_first() else {
        return Ok(value);
    };
    if rest.is_empty() {
        return assign_record_member(base, member, value);
    }
    let child = eval_member_value(base.clone(), member)?;
    let updated_child = assign_member_chain(child, rest, value)?;
    assign_record_member(base, member, updated_child)
}

fn read_runtime_reference(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let mut value = runtime_reference_root_value(
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
        &reference.target,
    )?;
    for member in runtime_reference_members(&reference.target) {
        value = eval_member_value(value, member)?;
    }
    Ok(value)
}

fn write_runtime_reference(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    value: RuntimeValue,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    if !reference.mutable {
        return Err("runtime reference is not mutable".to_string());
    }
    let members = runtime_reference_members(&reference.target);
    let updated_root = if members.is_empty() {
        if matches!(reference.target, RuntimeReferenceTarget::OwnerObject { .. }) {
            let _ = runtime_reference_root_value(
                scopes,
                plan,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
                &reference.target,
            )?;
        }
        value
    } else {
        let root = runtime_reference_root_value(
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
            &reference.target,
        )?;
        assign_member_chain(root, members, value)?
    };
    match &reference.target {
        RuntimeReferenceTarget::Local { local, .. } => {
            let runtime_local = lookup_local_mut_by_handle(scopes, *local)
                .ok_or_else(|| format!("runtime reference local `{}` is unresolved", local.0))?;
            runtime_local.moved = false;
            runtime_local.value = updated_root;
            update_runtime_rollup_binding_value(state, *local, &runtime_local.value);
            Ok(())
        }
        RuntimeReferenceTarget::OwnerObject {
            owner_key,
            object_name,
            ..
        } => {
            let owner_state = state
                .owners
                .entry(owner_key.clone())
                .or_insert_with(RuntimeOwnerState::default);
            owner_state
                .objects
                .insert(object_name.clone(), updated_root);
            let active_owners = scopes
                .iter()
                .flat_map(|scope| scope.activated_owner_keys.iter().cloned())
                .collect::<Vec<_>>();
            evaluate_owner_exit_checkpoints(
                &active_owners,
                plan,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
                Some(scopes),
            )
        }
        RuntimeReferenceTarget::ArenaSlot { id, .. } => {
            let arena = state
                .arenas
                .get_mut(&id.arena)
                .ok_or_else(|| format!("invalid Arena handle `{}`", id.arena.0))?;
            if !arena_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            arena.slots.insert(id.slot, updated_root);
            Ok(())
        }
        RuntimeReferenceTarget::FrameSlot { id, .. } => {
            let arena = state
                .frame_arenas
                .get_mut(&id.arena)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", id.arena.0))?;
            if !frame_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            arena.slots.insert(id.slot, updated_root);
            Ok(())
        }
        RuntimeReferenceTarget::PoolSlot { id, .. } => {
            let arena = state
                .pool_arenas
                .get_mut(&id.arena)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", id.arena.0))?;
            if !pool_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(*id))),
                    id.arena.0
                ));
            }
            arena.slots.insert(id.slot, updated_root);
            Ok(())
        }
    }
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

fn expect_channel(value: RuntimeValue, context: &str) -> Result<RuntimeChannelHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(handle)) = value else {
        return Err(format!("{context} expected Channel"));
    };
    Ok(handle)
}

fn expect_mutex(value: RuntimeValue, context: &str) -> Result<RuntimeMutexHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(handle)) = value else {
        return Err(format!("{context} expected Mutex"));
    };
    Ok(handle)
}

fn expect_atomic_int(value: RuntimeValue, context: &str) -> Result<RuntimeAtomicIntHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicInt(handle)) = value else {
        return Err(format!("{context} expected AtomicInt"));
    };
    Ok(handle)
}

fn expect_atomic_bool(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeAtomicBoolHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicBool(handle)) = value else {
        return Err(format!("{context} expected AtomicBool"));
    };
    Ok(handle)
}

fn expect_arena(value: RuntimeValue, context: &str) -> Result<RuntimeArenaHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(handle)) = value else {
        return Err(format!("{context} expected Arena"));
    };
    Ok(handle)
}

fn expect_arena_id(value: RuntimeValue, context: &str) -> Result<RuntimeArenaIdValue, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id)) = value else {
        return Err(format!("{context} expected ArenaId"));
    };
    Ok(id)
}

fn expect_frame_arena(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeFrameArenaHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle)) = value else {
        return Err(format!("{context} expected FrameArena"));
    };
    Ok(handle)
}

fn expect_frame_id(value: RuntimeValue, context: &str) -> Result<RuntimeFrameIdValue, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id)) = value else {
        return Err(format!("{context} expected FrameId"));
    };
    Ok(id)
}

fn expect_pool_arena(value: RuntimeValue, context: &str) -> Result<RuntimePoolArenaHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(handle)) = value else {
        return Err(format!("{context} expected PoolArena"));
    };
    Ok(handle)
}

fn expect_pool_id(value: RuntimeValue, context: &str) -> Result<RuntimePoolIdValue, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id)) = value else {
        return Err(format!("{context} expected PoolId"));
    };
    Ok(id)
}

fn expect_task(value: RuntimeValue, context: &str) -> Result<RuntimeTaskHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Task(handle)) = value else {
        return Err(format!("{context} expected Task"));
    };
    Ok(handle)
}

fn expect_thread(value: RuntimeValue, context: &str) -> Result<RuntimeThreadHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(handle)) = value else {
        return Err(format!("{context} expected Thread"));
    };
    Ok(handle)
}

fn expect_reference(value: RuntimeValue, context: &str) -> Result<RuntimeReferenceValue, String> {
    let RuntimeValue::Ref(reference) = value else {
        return Err(format!("{context} expected reference"));
    };
    Ok(reference)
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

fn insert_runtime_channel(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    capacity: usize,
) -> RuntimeChannelHandle {
    let handle = RuntimeChannelHandle(state.next_channel_handle);
    state.next_channel_handle += 1;
    state.channels.insert(
        handle,
        RuntimeChannelState {
            type_args: type_args.to_vec(),
            capacity,
            queue: VecDeque::new(),
        },
    );
    handle
}

fn insert_runtime_mutex(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    value: RuntimeValue,
) -> RuntimeMutexHandle {
    let handle = RuntimeMutexHandle(state.next_mutex_handle);
    state.next_mutex_handle += 1;
    state.mutexes.insert(
        handle,
        RuntimeMutexState {
            type_args: type_args.to_vec(),
            value: Some(value),
        },
    );
    handle
}

fn insert_runtime_atomic_int(
    state: &mut RuntimeExecutionState,
    value: i64,
) -> RuntimeAtomicIntHandle {
    let handle = RuntimeAtomicIntHandle(state.next_atomic_int_handle);
    state.next_atomic_int_handle += 1;
    state.atomic_ints.insert(handle, value);
    handle
}

fn insert_runtime_atomic_bool(
    state: &mut RuntimeExecutionState,
    value: bool,
) -> RuntimeAtomicBoolHandle {
    let handle = RuntimeAtomicBoolHandle(state.next_atomic_bool_handle);
    state.next_atomic_bool_handle += 1;
    state.atomic_bools.insert(handle, value);
    handle
}

fn insert_runtime_arena(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
) -> RuntimeArenaHandle {
    let handle = RuntimeArenaHandle(state.next_arena_handle);
    state.next_arena_handle += 1;
    state.arenas.insert(
        handle,
        RuntimeArenaState {
            type_args: type_args.to_vec(),
            next_slot: 0,
            generation: 0,
            slots: BTreeMap::new(),
        },
    );
    handle
}

fn insert_runtime_frame_arena(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
) -> RuntimeFrameArenaHandle {
    let handle = RuntimeFrameArenaHandle(state.next_frame_arena_handle);
    state.next_frame_arena_handle += 1;
    state.frame_arenas.insert(
        handle,
        RuntimeFrameArenaState {
            type_args: type_args.to_vec(),
            next_slot: 0,
            generation: 0,
            slots: BTreeMap::new(),
        },
    );
    handle
}

fn insert_runtime_pool_arena(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
) -> RuntimePoolArenaHandle {
    let handle = RuntimePoolArenaHandle(state.next_pool_arena_handle);
    state.next_pool_arena_handle += 1;
    state.pool_arenas.insert(
        handle,
        RuntimePoolArenaState {
            type_args: type_args.to_vec(),
            next_slot: 0,
            free_slots: Vec::new(),
            generations: BTreeMap::new(),
            slots: BTreeMap::new(),
        },
    );
    handle
}

fn insert_runtime_task(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    pending: RuntimePendingState,
) -> RuntimeTaskHandle {
    let handle = RuntimeTaskHandle(state.next_task_handle);
    state.next_task_handle += 1;
    state.tasks.insert(
        handle,
        RuntimeTaskState {
            type_args: type_args.to_vec(),
            state: pending,
        },
    );
    handle
}

fn allocate_scheduler_thread_id(state: &mut RuntimeExecutionState) -> i64 {
    if state.next_scheduler_thread_id <= 0 {
        state.next_scheduler_thread_id = 1;
    }
    let thread_id = state.next_scheduler_thread_id;
    state.next_scheduler_thread_id += 1;
    thread_id
}

fn runtime_async_calls_allowed(state: &RuntimeExecutionState) -> bool {
    state.async_context_depth > 0
}

fn insert_runtime_thread(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    pending: RuntimePendingState,
) -> RuntimeThreadHandle {
    let handle = RuntimeThreadHandle(state.next_thread_handle);
    state.next_thread_handle += 1;
    state.threads.insert(
        handle,
        RuntimeThreadState {
            type_args: type_args.to_vec(),
            state: pending,
        },
    );
    handle
}

fn arena_id_is_live(
    handle: RuntimeArenaHandle,
    arena: &RuntimeArenaState,
    id: RuntimeArenaIdValue,
) -> bool {
    id.arena == handle && id.generation == arena.generation && arena.slots.contains_key(&id.slot)
}

fn frame_id_is_live(
    handle: RuntimeFrameArenaHandle,
    arena: &RuntimeFrameArenaState,
    id: RuntimeFrameIdValue,
) -> bool {
    id.arena == handle && id.generation == arena.generation && arena.slots.contains_key(&id.slot)
}

fn pool_slot_generation(pool: &RuntimePoolArenaState, slot: u64) -> u64 {
    pool.generations.get(&slot).copied().unwrap_or(0)
}

fn pool_id_is_live(
    handle: RuntimePoolArenaHandle,
    arena: &RuntimePoolArenaState,
    id: RuntimePoolIdValue,
) -> bool {
    id.arena == handle
        && id.generation == pool_slot_generation(arena, id.slot)
        && arena.slots.contains_key(&id.slot)
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
) -> RuntimeEvalResult<Vec<RuntimeCallArg>> {
    let mut values = args
        .iter()
        .map(|arg| {
            Ok(RuntimeCallArg {
                name: arg.name.clone(),
                source_expr: arg.value.clone(),
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
        .collect::<RuntimeEvalResult<Vec<_>>>()?;
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
        let source_expr = match attachment {
            ParsedHeaderAttachment::Named { value, .. } => value.clone(),
            ParsedHeaderAttachment::Chain { expr } => expr.clone(),
        };
        values.push(RuntimeCallArg {
            name,
            value,
            source_expr,
        });
    }
    Ok(values)
}

fn bind_call_args_for_routine(
    routine: &RuntimeRoutinePlan,
    args: Vec<RuntimeCallArg>,
) -> Result<Vec<BoundRuntimeArg>, String> {
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
        bound[index] = Some(BoundRuntimeArg {
            value: arg.value,
            source_expr: arg.source_expr,
        });
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
        RuntimeOpaqueValue::Channel(_) => "std.concurrent.Channel",
        RuntimeOpaqueValue::Mutex(_) => "std.concurrent.Mutex",
        RuntimeOpaqueValue::AtomicInt(_) => "std.concurrent.AtomicInt",
        RuntimeOpaqueValue::AtomicBool(_) => "std.concurrent.AtomicBool",
        RuntimeOpaqueValue::Arena(_) => "std.memory.Arena",
        RuntimeOpaqueValue::ArenaId(_) => "std.memory.ArenaId",
        RuntimeOpaqueValue::FrameArena(_) => "std.memory.FrameArena",
        RuntimeOpaqueValue::FrameId(_) => "std.memory.FrameId",
        RuntimeOpaqueValue::PoolArena(_) => "std.memory.PoolArena",
        RuntimeOpaqueValue::PoolId(_) => "std.memory.PoolId",
        RuntimeOpaqueValue::Task(_) => "std.concurrent.Task",
        RuntimeOpaqueValue::Thread(_) => "std.concurrent.Thread",
    }
}

fn runtime_receiver_type_args(
    receiver: &RuntimeValue,
    state: &RuntimeExecutionState,
) -> Vec<String> {
    match receiver {
        RuntimeValue::Record { name, .. } => parse_runtime_value_type_args(name),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(handle)) => state
            .channels
            .get(handle)
            .map(|channel| channel.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(handle)) => state
            .mutexes
            .get(handle)
            .map(|mutex| mutex.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(handle)) => state
            .arenas
            .get(handle)
            .map(|arena| arena.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle)) => state
            .frame_arenas
            .get(handle)
            .map(|arena| arena.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(handle)) => state
            .pool_arenas
            .get(handle)
            .map(|arena| arena.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Task(handle)) => state
            .tasks
            .get(handle)
            .map(|task| task.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(handle)) => state
            .threads
            .get(handle)
            .map(|thread| thread.type_args.clone())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn runtime_value_type_text(
    receiver: &RuntimeValue,
    state: &RuntimeExecutionState,
) -> Option<String> {
    match receiver {
        RuntimeValue::OwnerHandle(owner_key) => Some(format!("Owner<{owner_key}>")),
        RuntimeValue::Record { name, .. } => Some(name.clone()),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(_)) => {
            let type_args = runtime_receiver_type_args(receiver, state);
            Some(if type_args.is_empty() {
                opaque_type_name(match receiver {
                    RuntimeValue::Opaque(value) => value,
                    _ => unreachable!(),
                })
                .to_string()
            } else {
                format!(
                    "{}[{}]",
                    opaque_type_name(match receiver {
                        RuntimeValue::Opaque(value) => value,
                        _ => unreachable!(),
                    }),
                    type_args.join(", ")
                )
            })
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Task(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(_)) => {
            let type_args = runtime_receiver_type_args(receiver, state);
            let opaque_name = match receiver {
                RuntimeValue::Opaque(value) => opaque_type_name(value),
                _ => unreachable!(),
            };
            Some(if type_args.is_empty() {
                opaque_name.to_string()
            } else {
                format!("{opaque_name}[{}]", type_args.join(", "))
            })
        }
        RuntimeValue::Opaque(value) => Some(opaque_type_name(value).to_string()),
        RuntimeValue::Variant { name, .. } => Some(runtime_variant_enum_name(name)),
        _ => runtime_value_type_root(receiver),
    }
}

fn runtime_value_type_root(receiver: &RuntimeValue) -> Option<String> {
    match receiver {
        RuntimeValue::Int(_) => Some("Int".to_string()),
        RuntimeValue::Bool(_) => Some("Bool".to_string()),
        RuntimeValue::Str(_) => Some("Str".to_string()),
        RuntimeValue::Pair(_, _) => Some("Pair".to_string()),
        RuntimeValue::Array(_) => Some("Array".to_string()),
        RuntimeValue::List(_) => Some("List".to_string()),
        RuntimeValue::Map(_) => Some("Map".to_string()),
        RuntimeValue::Range { .. } => Some("RangeInt".to_string()),
        RuntimeValue::OwnerHandle(_) => Some("Owner".to_string()),
        RuntimeValue::Ref(_) => Some("Ref".to_string()),
        RuntimeValue::Opaque(value) => Some(runtime_type_root_name(opaque_type_name(value))),
        RuntimeValue::Record { name, .. } => Some(runtime_type_root_name(name)),
        RuntimeValue::Variant { name, .. } => {
            Some(runtime_type_root_name(&runtime_variant_enum_name(name)))
        }
        RuntimeValue::Unit => Some("Unit".to_string()),
    }
}

fn runtime_value_is_copy(value: &RuntimeValue) -> bool {
    match value {
        RuntimeValue::Int(_)
        | RuntimeValue::Bool(_)
        | RuntimeValue::Range { .. }
        | RuntimeValue::OwnerHandle(_)
        | RuntimeValue::Ref(_)
        | RuntimeValue::Unit => true,
        RuntimeValue::Pair(left, right) => {
            runtime_value_is_copy(left) && runtime_value_is_copy(right)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicInt(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicBool(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(_)) => true,
        RuntimeValue::Str(_)
        | RuntimeValue::Array(_)
        | RuntimeValue::List(_)
        | RuntimeValue::Map(_)
        | RuntimeValue::Opaque(_)
        | RuntimeValue::Record { .. }
        | RuntimeValue::Variant { .. } => false,
    }
}

fn expr_to_assign_target(expr: &ParsedExpr) -> Option<ParsedAssignTarget> {
    match expr {
        ParsedExpr::Path(segments) if segments.len() == 1 => {
            Some(ParsedAssignTarget::Name(segments[0].clone()))
        }
        ParsedExpr::Member { expr, member } => Some(ParsedAssignTarget::Member {
            target: Box::new(expr_to_assign_target(expr)?),
            member: member.clone(),
        }),
        ParsedExpr::Index { expr, index } => Some(ParsedAssignTarget::Index {
            target: Box::new(expr_to_assign_target(expr)?),
            index: (**index).clone(),
        }),
        ParsedExpr::Generic { expr, .. } => expr_to_assign_target(expr),
        ParsedExpr::Unary { op, expr }
            if matches!(
                op,
                ParsedUnaryOp::BorrowRead | ParsedUnaryOp::BorrowMut | ParsedUnaryOp::Deref
            ) =>
        {
            expr_to_assign_target(expr)
        }
        _ => None,
    }
}

fn expr_root_local_name(expr: &ParsedExpr) -> Option<&str> {
    match expr {
        ParsedExpr::Path(segments) if segments.len() == 1 => Some(segments[0].as_str()),
        ParsedExpr::Generic { expr, .. } => expr_root_local_name(expr),
        _ => None,
    }
}

fn consume_take_arg_root_local(
    scopes: &mut [RuntimeScope],
    source_expr: &ParsedExpr,
) -> Result<(), String> {
    let Some(name) = expr_root_local_name(source_expr) else {
        return Ok(());
    };
    let Some(local) = scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.locals.get_mut(name))
    else {
        return Ok(());
    };
    if local.moved {
        return Err(format!("use of moved local `{name}`"));
    }
    if runtime_value_is_copy(&local.value) {
        return Ok(());
    }
    local.moved = true;
    Ok(())
}

fn consume_take_bound_args(
    scopes: &mut [RuntimeScope],
    routine: &RuntimeRoutinePlan,
    args: &[BoundRuntimeArg],
) -> Result<(), String> {
    for (param, bound_arg) in routine.params.iter().zip(args) {
        if param.mode.as_deref() != Some("take") {
            continue;
        }
        consume_take_arg_root_local(scopes, &bound_arg.source_expr)?;
    }
    Ok(())
}

fn assign_record_member(
    base: RuntimeValue,
    member: &str,
    value: RuntimeValue,
) -> Result<RuntimeValue, String> {
    match base {
        RuntimeValue::Pair(left, right) => match member {
            "0" => Ok(RuntimeValue::Pair(Box::new(value), right)),
            "1" => Ok(RuntimeValue::Pair(left, Box::new(value))),
            _ => Err(format!("pair has no member `.{member}`")),
        },
        RuntimeValue::Record { name, mut fields } => {
            fields.insert(member.to_string(), value);
            Ok(RuntimeValue::Record { name, fields })
        }
        other => Err(format!(
            "unsupported runtime member assignment `.{member}` on `{other:?}`"
        )),
    }
}

fn resolve_assign_target_place(
    scopes: &[RuntimeScope],
    target: &ParsedAssignTarget,
) -> Result<RuntimeResolvedPlace, String> {
    match target {
        ParsedAssignTarget::Name(name) => {
            let local = lookup_local(scopes, name)
                .ok_or_else(|| format!("runtime assignment target `{name}` is unresolved"))?;
            match &local.value {
                RuntimeValue::Ref(reference) => Ok(RuntimeResolvedPlace {
                    mutable: reference.mutable,
                    target: reference.target.clone(),
                }),
                _ => Ok(RuntimeResolvedPlace {
                    mutable: local.mutable,
                    target: RuntimeReferenceTarget::Local {
                        local: local.handle,
                        members: Vec::new(),
                    },
                }),
            }
        }
        ParsedAssignTarget::Member { target, member } => {
            let place = resolve_assign_target_place(scopes, target)?;
            Ok(RuntimeResolvedPlace {
                mutable: place.mutable,
                target: runtime_reference_with_member(&place.target, member.clone()),
            })
        }
        ParsedAssignTarget::Index { .. } => Err(format!(
            "unsupported runtime assignment target `{target:?}`"
        )),
    }
}

fn read_assign_target_value(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    target: &ParsedAssignTarget,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let place = resolve_assign_target_place(scopes, target)?;
    read_runtime_reference(
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        &RuntimeReferenceValue {
            mutable: place.mutable,
            target: place.target,
        },
        host,
    )
}

fn write_assign_target_value(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    target: &ParsedAssignTarget,
    value: RuntimeValue,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    let place = resolve_assign_target_place(scopes, target)?;
    if !place.mutable {
        return Err(format!(
            "runtime assignment target `{target:?}` is not mutable"
        ));
    }
    write_runtime_reference(
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        &RuntimeReferenceValue {
            mutable: true,
            target: place.target,
        },
        value,
        host,
    )
}

fn assign_runtime_index_value(
    base: RuntimeValue,
    index: RuntimeValue,
    value: RuntimeValue,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let base = read_runtime_value_if_ref(
        base,
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let index = expect_int(index, "indexed assignment")?;
    match base {
        RuntimeValue::List(mut values) => {
            let index = runtime_index_to_usize(index, values.len(), "list index assignment")?;
            values[index] = value;
            Ok(RuntimeValue::List(values))
        }
        RuntimeValue::Array(mut values) => {
            let index = runtime_index_to_usize(index, values.len(), "array index assignment")?;
            values[index] = value;
            Ok(RuntimeValue::Array(values))
        }
        other => Err(format!(
            "runtime indexed assignment expects List or Array, got `{other:?}`"
        )),
    }
}

fn read_assign_target_value_runtime(
    target: &ParsedAssignTarget,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match target {
        ParsedAssignTarget::Index { target, index } => eval_runtime_index_value(
            read_assign_target_value_runtime(
                target,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?,
            eval_expr(
                index,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
            .map_err(runtime_eval_message)?,
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
        _ => read_assign_target_value(
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            target,
            host,
        ),
    }
}

fn write_assign_target_value_runtime(
    target: &ParsedAssignTarget,
    value: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    match target {
        ParsedAssignTarget::Index {
            target: base_target,
            index,
        } => {
            let updated = assign_runtime_index_value(
                read_assign_target_value_runtime(
                    base_target,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
                eval_expr(
                    index,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )
                .map_err(runtime_eval_message)?,
                value,
                scopes,
                plan,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            write_assign_target_value_runtime(
                base_target,
                updated,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
        }
        _ => write_assign_target_value(
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            target,
            value,
            host,
        ),
    }
}

fn apply_assignment_op(
    current: RuntimeValue,
    op: ParsedAssignOp,
    value: RuntimeValue,
) -> Result<RuntimeValue, String> {
    Ok(match op {
        ParsedAssignOp::Assign => value,
        ParsedAssignOp::AddAssign => apply_runtime_add(current, value, "+=")?,
        ParsedAssignOp::SubAssign => {
            RuntimeValue::Int(expect_int(current, "-=")? - expect_int(value, "-=")?)
        }
        ParsedAssignOp::MulAssign => {
            RuntimeValue::Int(expect_int(current, "*=")? * expect_int(value, "*=")?)
        }
        ParsedAssignOp::DivAssign => {
            RuntimeValue::Int(expect_int(current, "/=")? / expect_int(value, "/=")?)
        }
        ParsedAssignOp::ModAssign => {
            RuntimeValue::Int(expect_int(current, "%=")? % expect_int(value, "%=")?)
        }
        ParsedAssignOp::BitAndAssign => {
            RuntimeValue::Int(expect_int(current, "&=")? & expect_int(value, "&=")?)
        }
        ParsedAssignOp::BitOrAssign => {
            RuntimeValue::Int(expect_int(current, "|=")? | expect_int(value, "|=")?)
        }
        ParsedAssignOp::BitXorAssign => {
            RuntimeValue::Int(expect_int(current, "^=")? ^ expect_int(value, "^=")?)
        }
        ParsedAssignOp::ShlAssign => {
            RuntimeValue::Int(expect_int(current, "<<=")? << expect_int(value, "<<=")?)
        }
        ParsedAssignOp::ShrAssign => {
            RuntimeValue::Int(expect_int(current, "shr=")? >> expect_int(value, "shr=")?)
        }
    })
}

fn apply_runtime_add(
    left: RuntimeValue,
    right: RuntimeValue,
    context: &str,
) -> Result<RuntimeValue, String> {
    match (left, right) {
        (RuntimeValue::Int(left), RuntimeValue::Int(right)) => Ok(RuntimeValue::Int(left + right)),
        (RuntimeValue::Str(mut left), RuntimeValue::Str(right)) => {
            left.push_str(&right);
            Ok(RuntimeValue::Str(left))
        }
        (left, right) => Err(format!(
            "{context} expected Int or Str operands of the same type, got `{left:?}` and `{right:?}`"
        )),
    }
}

fn intrinsic_edit_arg_indices(intrinsic: RuntimeIntrinsic) -> &'static [usize] {
    match intrinsic {
        RuntimeIntrinsic::ListPush
        | RuntimeIntrinsic::ListPop
        | RuntimeIntrinsic::ListTryPopOr
        | RuntimeIntrinsic::MapSet
        | RuntimeIntrinsic::MapRemove => &[0],
        _ => &[],
    }
}

fn intrinsic_take_arg_indices(intrinsic: RuntimeIntrinsic) -> &'static [usize] {
    match intrinsic {
        RuntimeIntrinsic::FsStreamCloseTry
        | RuntimeIntrinsic::WindowClose
        | RuntimeIntrinsic::AudioOutputClose
        | RuntimeIntrinsic::AudioPlaybackStop
        | RuntimeIntrinsic::ArrayFromList
        | RuntimeIntrinsic::ConcurrentMutexNew
        | RuntimeIntrinsic::EcsSetSingleton => &[0],
        RuntimeIntrinsic::ListPush
        | RuntimeIntrinsic::ListTryPopOr
        | RuntimeIntrinsic::ConcurrentChannelSend
        | RuntimeIntrinsic::ConcurrentMutexPut
        | RuntimeIntrinsic::MemoryArenaAlloc
        | RuntimeIntrinsic::MemoryFrameAlloc
        | RuntimeIntrinsic::MemoryPoolAlloc
        | RuntimeIntrinsic::EcsSetComponentAt => &[1],
        RuntimeIntrinsic::MapSet
        | RuntimeIntrinsic::MapTryGetOr
        | RuntimeIntrinsic::MemoryArenaSet
        | RuntimeIntrinsic::MemoryFrameSet
        | RuntimeIntrinsic::MemoryPoolSet => &[2],
        _ => &[],
    }
}

fn write_back_call_args(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    edit_arg_indices: &[usize],
    call_args: &[RuntimeCallArg],
    final_args: &[RuntimeValue],
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    for &index in edit_arg_indices {
        let Some(call_arg) = call_args.get(index) else {
            continue;
        };
        let Some(value) = final_args.get(index) else {
            continue;
        };
        if matches!(value, RuntimeValue::Ref(_)) {
            continue;
        }
        let target = expr_to_assign_target(&call_arg.source_expr).ok_or_else(|| {
            format!(
                "runtime edit argument `{:?}` is not a writable place",
                call_arg.source_expr
            )
        })?;
        write_assign_target_value(
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            &target,
            value.clone(),
            host,
        )?;
    }
    Ok(())
}

fn consume_take_call_args(
    scopes: &mut [RuntimeScope],
    take_arg_indices: &[usize],
    call_args: &[RuntimeCallArg],
) -> Result<(), String> {
    for &index in take_arg_indices {
        let Some(call_arg) = call_args.get(index) else {
            continue;
        };
        consume_take_arg_root_local(scopes, &call_arg.source_expr)?;
    }
    Ok(())
}

fn write_back_bound_args(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    routine: &RuntimeRoutinePlan,
    args: &[BoundRuntimeArg],
    final_args: &[RuntimeValue],
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    for (index, param) in routine.params.iter().enumerate() {
        if param.mode.as_deref() != Some("edit") {
            continue;
        }
        let Some(bound_arg) = args.get(index) else {
            continue;
        };
        let Some(value) = final_args.get(index) else {
            continue;
        };
        if matches!(value, RuntimeValue::Ref(_)) {
            continue;
        }
        let target = expr_to_assign_target(&bound_arg.source_expr).ok_or_else(|| {
            format!(
                "runtime edit argument for `{}` is not a writable place",
                param.name
            )
        })?;
        write_assign_target_value(
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            &target,
            value.clone(),
            host,
        )?;
    }
    Ok(())
}

fn eval_member_value(base: RuntimeValue, member: &str) -> Result<RuntimeValue, String> {
    match base {
        RuntimeValue::OwnerHandle(owner_key) => Ok(make_owner_object_reference(&owner_key, member)),
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

fn eval_runtime_member_value(
    base: RuntimeValue,
    member: &str,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match base {
        RuntimeValue::Ref(reference) => {
            let value = read_runtime_reference(
                scopes,
                plan,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &reference,
                host,
            )?;
            eval_member_value(value, member)
        }
        other => eval_member_value(other, member),
    }
}

fn read_runtime_value_if_ref(
    value: RuntimeValue,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match value {
        RuntimeValue::Ref(reference) => read_runtime_reference(
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            &reference,
            host,
        ),
        other => Ok(other),
    }
}

fn runtime_index_to_usize(index: i64, len: usize, context: &str) -> Result<usize, String> {
    if index < 0 {
        return Err(format!("{context} index must be non-negative"));
    }
    let index = usize::try_from(index)
        .map_err(|_| format!("{context} index `{index}` does not fit in usize"))?;
    if index >= len {
        return Err(format!(
            "{context} index `{index}` is out of bounds for length `{len}`"
        ));
    }
    Ok(index)
}

fn runtime_slice_bound_to_usize(
    bound: Option<i64>,
    default: usize,
    len: usize,
    context: &str,
    bound_name: &str,
) -> Result<usize, String> {
    let Some(bound) = bound else {
        return Ok(default);
    };
    if bound < 0 {
        return Err(format!("{context} {bound_name} must be non-negative"));
    }
    let bound = usize::try_from(bound)
        .map_err(|_| format!("{context} {bound_name} `{bound}` does not fit in usize"))?;
    if bound > len {
        return Err(format!(
            "{context} {bound_name} `{bound}` is out of bounds for length `{len}`"
        ));
    }
    Ok(bound)
}

fn eval_runtime_index_value(
    base: RuntimeValue,
    index: RuntimeValue,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let base = read_runtime_value_if_ref(
        base,
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let index = expect_int(index, "index")?;
    match base {
        RuntimeValue::List(values) => {
            let index = runtime_index_to_usize(index, values.len(), "list index")?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| format!("list index `{index}` is out of bounds"))
        }
        RuntimeValue::Array(values) => {
            let index = runtime_index_to_usize(index, values.len(), "array index")?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| format!("array index `{index}` is out of bounds"))
        }
        other => Err(format!(
            "runtime index expects List or Array, got `{other:?}`"
        )),
    }
}

fn eval_runtime_slice_value(
    base: RuntimeValue,
    start: Option<RuntimeValue>,
    end: Option<RuntimeValue>,
    inclusive_end: bool,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let base = read_runtime_value_if_ref(
        base,
        scopes,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let (values, slice_kind) = match base {
        RuntimeValue::List(values) => (values, "list"),
        RuntimeValue::Array(values) => (values, "array"),
        other => {
            return Err(format!(
                "runtime slice expects List or Array, got `{other:?}`"
            ));
        }
    };
    let len = values.len();
    let start = runtime_slice_bound_to_usize(
        start
            .map(|value| expect_int(value, "slice start"))
            .transpose()?,
        0,
        len,
        "slice",
        "start",
    )?;
    let raw_end = runtime_slice_bound_to_usize(
        end.clone()
            .map(|value| expect_int(value, "slice end"))
            .transpose()?,
        len,
        len,
        "slice",
        "end",
    )?;
    let end = if inclusive_end {
        if end.is_some() {
            if raw_end >= len {
                return Err(format!(
                    "slice inclusive end `{raw_end}` is out of bounds for length `{len}`"
                ));
            }
            raw_end + 1
        } else {
            len
        }
    } else {
        raw_end
    };
    if start > end {
        return Err(format!(
            "slice start `{start}` must be less than or equal to end `{end}`"
        ));
    }
    let sliced = values[start..end].to_vec();
    Ok(match slice_kind {
        "list" => RuntimeValue::List(sliced),
        "array" => RuntimeValue::Array(sliced),
        _ => unreachable!("validated runtime slice carrier kind"),
    })
}

fn eval_optional_runtime_int_expr(
    expr: Option<&ParsedExpr>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    context: &str,
) -> RuntimeEvalResult<Option<i64>> {
    Ok(expr
        .map(|expr| {
            expect_int(
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
                context,
            )
            .map_err(RuntimeEvalSignal::from)
        })
        .transpose()?)
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
) -> RuntimeEvalResult<RuntimeValue> {
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
                insert_runtime_local(state, scopes.len(), &mut scope, name, false, value);
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
    Err("runtime match expression had no matching arm"
        .to_string()
        .into())
}

fn err_variant(message: String) -> RuntimeValue {
    RuntimeValue::Variant {
        name: "Result.Err".to_string(),
        payload: vec![RuntimeValue::Str(message)],
    }
}

fn try_construct_record_value(
    callable: &[String],
    resolved_type_args: &[String],
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
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
            )
            .into());
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
    let name = if resolved_type_args.is_empty() {
        callable.join(".")
    } else {
        format!("{}[{}]", callable.join("."), resolved_type_args.join(", "))
    };
    Ok(Some(RuntimeValue::Record { name, fields }))
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
) -> RuntimeEvalResult<Option<RuntimeValue>> {
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
        RuntimeValue::Map(entries) => Ok(entries
            .into_iter()
            .map(|(key, value)| make_pair(key, value))
            .collect()),
        RuntimeValue::Range {
            start,
            end,
            inclusive_end,
        } => {
            let Some(end) = end else {
                return Err("runtime for-loop range must have an end bound".to_string());
            };
            let start = start.unwrap_or(0);
            let iter = if inclusive_end {
                (start..=end).collect::<Vec<_>>()
            } else {
                (start..end).collect::<Vec<_>>()
            };
            Ok(iter.into_iter().map(RuntimeValue::Int).collect())
        }
        other => Err(format!(
            "runtime for-loop expects List, Array, Map, or RangeInt, got `{other:?}`"
        )),
    }
}

fn resolve_routine_index_for_call(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    callable: &[String],
    call_args: &[RuntimeCallArg],
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    allow_receiver_root_fallback: bool,
    state: Option<&RuntimeExecutionState>,
) -> Result<Option<usize>, String> {
    let candidates = match dynamic_dispatch {
        Some(ParsedDynamicDispatch::TraitMethod { trait_path }) => {
            let Some(method_name) = callable.last() else {
                return Ok(None);
            };
            resolve_dynamic_method_candidate_indices(plan, method_name, trait_path)
        }
        None => resolve_routine_candidate_indices(plan, current_module_id, callable),
    };
    if candidates.is_empty() {
        return Ok(None);
    }
    if let Some(routine_key) = resolved_routine {
        let filtered = candidates
            .into_iter()
            .filter(|index| {
                plan.routines
                    .get(*index)
                    .map(|routine| routine.routine_key == routine_key)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        return match filtered.as_slice() {
            [] => Err(format!(
                "runtime call `{}` has no overload matching lowered routine `{routine_key}`",
                callable.join(".")
            )),
            [index] => Ok(Some(*index)),
            _ => Err(format!(
                "runtime call `{}` remains ambiguous for lowered routine `{routine_key}`",
                callable.join(".")
            )),
        };
    }
    if dynamic_dispatch.is_none() && candidates.len() == 1 {
        return Ok(candidates.into_iter().next());
    }
    if !allow_receiver_root_fallback && dynamic_dispatch.is_none() {
        return Err(format!(
            "runtime call `{}` is ambiguous without lowered routine identity",
            callable.join(".")
        ));
    }
    let Some(receiver) = call_args.first().map(|arg| &arg.value) else {
        return Err(format!(
            "runtime call `{}` is ambiguous with no receiver type information",
            callable.join(".")
        ));
    };
    let receiver_type_text = state.and_then(|state| runtime_value_type_text(receiver, state));
    let Some(receiver_root) = receiver_type_text
        .as_deref()
        .map(runtime_type_root_name)
        .or_else(|| runtime_value_type_root(receiver))
    else {
        return Err(format!(
            "runtime call `{}` is ambiguous with no receiver type information",
            callable.join(".")
        ));
    };
    let filtered = candidates
        .into_iter()
        .filter(|index| {
            plan.routines
                .get(*index)
                .map(|routine| {
                    let declared = routine
                        .impl_target_type
                        .as_deref()
                        .or_else(|| routine.params.first().map(|param| param.ty.as_str()));
                    let Some(declared) = declared else {
                        return false;
                    };
                    let type_params = routine.type_params.iter().cloned().collect::<BTreeSet<_>>();
                    receiver_type_text
                        .as_deref()
                        .map(|actual| {
                            runtime_declared_type_matches_actual(declared, actual, &type_params)
                        })
                        .unwrap_or_else(|| runtime_type_root_name(declared) == receiver_root)
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    match filtered.as_slice() {
        [] => Err(format!(
            "runtime call `{}` has no overload matching receiver `{receiver_root}`",
            callable.join(".")
        )),
        [index] => Ok(Some(*index)),
        _ => Err(format!(
            "runtime call `{}` remains ambiguous for receiver `{receiver_root}`",
            callable.join(".")
        )),
    }
}

fn resolve_rollup_handler_callable_path(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    handler_path: &[String],
) -> Vec<String> {
    let Some((first, suffix)) = handler_path.split_first() else {
        return Vec::new();
    };
    if let Some(prefix) = plan
        .module_aliases
        .get(current_module_id)
        .and_then(|aliases| aliases.get(first))
    {
        let mut callable = prefix.clone();
        callable.extend(suffix.iter().cloned());
        return callable;
    }
    handler_path.to_vec()
}

fn validate_runtime_rollup_handler_callable_path(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    handler_path: &[String],
) -> Result<Vec<String>, String> {
    let callable = resolve_rollup_handler_callable_path(plan, current_module_id, handler_path);
    if let Some(routine_index) = resolve_routine_index(plan, current_module_id, &callable) {
        let routine = plan.routines.get(routine_index).ok_or_else(|| {
            format!(
                "runtime rollup handler `{}` resolved to invalid routine index `{routine_index}`",
                handler_path.join(".")
            )
        })?;
        if routine.is_async {
            return Err(format!(
                "runtime rollup handler `{}` cannot be async in v1",
                handler_path.join(".")
            ));
        }
        if routine.params.len() != 1 {
            return Err(format!(
                "runtime rollup handler `{}` must accept exactly one parameter in v1",
                handler_path.join(".")
            ));
        }
        return Ok(callable);
    }
    Err(format!(
        "runtime rollup handler `{}` does not resolve to a callable path",
        handler_path.join(".")
    ))
}

fn execute_call_by_path(
    callable: &[String],
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    current_module_id: &str,
    type_args: Vec<String>,
    call_args: Vec<RuntimeCallArg>,
    allow_receiver_root_fallback: bool,
    plan: &RuntimePackagePlan,
    scopes: &mut Vec<RuntimeScope>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    allow_async: bool,
) -> Result<RuntimeValue, String> {
    if let Some(routine_index) = resolve_routine_index_for_call(
        plan,
        current_module_id,
        callable,
        &call_args,
        resolved_routine,
        dynamic_dispatch,
        allow_receiver_root_fallback,
        Some(state),
    )? {
        let routine = plan
            .routines
            .get(routine_index)
            .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
        let bound_args = bind_call_args_for_routine(routine, call_args)?;
        consume_take_bound_args(scopes, routine, &bound_args)?;
        let values = bound_args
            .iter()
            .map(|arg| arg.value.clone())
            .collect::<Vec<_>>();
        let outcome = execute_routine_call_with_state(
            plan,
            routine_index,
            type_args,
            values,
            state,
            host,
            allow_async,
        )?;
        write_back_bound_args(
            scopes,
            plan,
            current_module_id,
            &BTreeMap::new(),
            &BTreeMap::new(),
            state,
            routine,
            &bound_args,
            &outcome.final_args,
            host,
        )?;
        return Ok(outcome.value);
    }
    let intrinsic = resolve_runtime_intrinsic_path(callable)
        .ok_or_else(|| format!("unsupported runtime callable `{}`", callable.join(".")))?;
    if call_args.iter().any(|arg| arg.name.is_some()) {
        return Err(format!(
            "runtime intrinsic `{}` does not yet support named-only fallback binding",
            callable.join(".")
        ));
    }
    consume_take_call_args(scopes, intrinsic_take_arg_indices(intrinsic), &call_args)?;
    let mut values = call_args
        .iter()
        .map(|arg| arg.value.clone())
        .collect::<Vec<_>>();
    let value = execute_runtime_intrinsic(intrinsic, &type_args, &mut values, plan, state, host)?;
    write_back_call_args(
        scopes,
        plan,
        current_module_id,
        &BTreeMap::new(),
        &BTreeMap::new(),
        state,
        intrinsic_edit_arg_indices(intrinsic),
        &call_args,
        &values,
        host,
    )?;
    Ok(value)
}

fn execute_runtime_apply_phrase(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    allow_async: bool,
) -> RuntimeEvalResult<RuntimeValue> {
    let callable = resolved_callable
        .map(|path| path.to_vec())
        .or_else(|| resolve_callable_path(subject, aliases))
        .ok_or_else(|| format!("unsupported runtime callable `{subject:?}`"))?;
    let type_args = resolve_runtime_type_args(&extract_generic_type_args(subject), type_bindings);
    if resolve_routine_index(plan, current_module_id, &callable).is_none()
        && resolve_runtime_intrinsic_path(&callable).is_none()
    {
        if let Some(record) = try_construct_record_value(
            &callable,
            &type_args,
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
        return Err(format!("unsupported runtime callable `{}`", callable.join(".")).into());
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
    Ok(execute_call_by_path(
        &callable,
        resolved_routine,
        None,
        current_module_id,
        type_args,
        call_args,
        false,
        plan,
        scopes,
        state,
        host,
        allow_async,
    )?)
}

fn execute_runtime_method_call(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier: &str,
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
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
    let callable = resolved_callable
        .ok_or_else(|| {
            format!(
                "runtime bare-method qualifier `{qualifier}` is missing lowered callable identity"
            )
        })?
        .to_vec();
    let type_args = runtime_receiver_type_args(&receiver, state);
    let mut call_args = vec![RuntimeCallArg {
        name: None,
        value: receiver,
        source_expr: subject.clone(),
    }];
    call_args.extend(collect_call_args(
        args,
        attached,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?);
    Ok(execute_call_by_path(
        &callable,
        resolved_routine,
        dynamic_dispatch,
        current_module_id,
        type_args,
        call_args,
        resolved_routine.is_none(),
        plan,
        scopes,
        state,
        host,
        runtime_async_calls_allowed(state),
    )?)
}

fn execute_runtime_named_qualifier_call(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier: &str,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
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
    let callable_expr = parse_runtime_callable_expr(qualifier);
    let callable = resolve_named_qualifier_callable_path(&callable_expr, aliases)
        .ok_or_else(|| format!("unsupported runtime named qualifier callable `{qualifier}`"))?;
    let type_args =
        resolve_runtime_type_args(&extract_generic_type_args(&callable_expr), type_bindings);
    let mut call_args = vec![RuntimeCallArg {
        name: None,
        value: receiver,
        source_expr: subject.clone(),
    }];
    call_args.extend(collect_call_args(
        args,
        attached,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?);
    Ok(execute_call_by_path(
        &callable,
        None,
        None,
        current_module_id,
        type_args,
        call_args,
        false,
        plan,
        scopes,
        state,
        host,
        runtime_async_calls_allowed(state),
    )?)
}

fn eval_qualifier(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier: &str,
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
    execute_runtime_method_call(
        subject,
        args,
        attached,
        qualifier,
        resolved_callable,
        resolved_routine,
        dynamic_dispatch,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )
}

fn try_unwrap_runtime_result(value: RuntimeValue) -> RuntimeEvalResult<RuntimeValue> {
    match value {
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Ok") => {
            match payload.as_slice() {
                [value] => Ok(value.clone()),
                _ => Err(format!(
                    "runtime try qualifier `?` expected Result.Ok with one payload value, got `{name}`"
                )
                .into()),
            }
        }
        RuntimeValue::Variant { ref name, .. } if variant_name_matches(name, "Result.Err") => {
            Err(RuntimeEvalSignal::Return(value))
        }
        other => Err(format!(
            "runtime try qualifier `?` expects Result-shape value, got `{other:?}`"
        )
        .into()),
    }
}

fn eval_try_qualifier(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
    if !args.is_empty() {
        return Err("`:: ?` does not accept arguments".to_string().into());
    }
    if !attached.is_empty() {
        return Err("`:: ?` does not support an attached block"
            .to_string()
            .into());
    }
    try_unwrap_runtime_result(eval_expr(
        subject,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?)
}

fn execute_runtime_intrinsic(
    intrinsic: RuntimeIntrinsic,
    type_args: &[String],
    final_args: &mut Vec<RuntimeValue>,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    let args = final_args.clone();
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
        RuntimeIntrinsic::ConcurrentThreadId => {
            if !args.is_empty() {
                return Err("thread_id expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Int(state.current_thread_id))
        }
        RuntimeIntrinsic::ConcurrentTaskDone => {
            let handle = expect_task(expect_single_arg(args, "task_done")?, "task_done")?;
            let task = state
                .tasks
                .get(&handle)
                .ok_or_else(|| format!("invalid Task handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(pending_state_is_done(&task.state)))
        }
        RuntimeIntrinsic::ConcurrentTaskJoin => {
            let handle = expect_task(expect_single_arg(args, "task_join")?, "task_join")?;
            drive_runtime_task(handle, plan, state, host)?;
            let task = state
                .tasks
                .get(&handle)
                .ok_or_else(|| format!("invalid Task handle `{}`", handle.0))?;
            pending_state_value(&task.state, &format!("Task `{}`", handle.0))
        }
        RuntimeIntrinsic::ConcurrentThreadDone => {
            let handle = expect_thread(expect_single_arg(args, "thread_done")?, "thread_done")?;
            let thread = state
                .threads
                .get(&handle)
                .ok_or_else(|| format!("invalid Thread handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(pending_state_is_done(&thread.state)))
        }
        RuntimeIntrinsic::ConcurrentThreadJoin => {
            let handle = expect_thread(expect_single_arg(args, "thread_join")?, "thread_join")?;
            drive_runtime_thread(handle, plan, state, host)?;
            let thread = state
                .threads
                .get(&handle)
                .ok_or_else(|| format!("invalid Thread handle `{}`", handle.0))?;
            pending_state_value(&thread.state, &format!("Thread `{}`", handle.0))
        }
        RuntimeIntrinsic::ConcurrentChannelNew => {
            let capacity = expect_int(expect_single_arg(args, "channel_new")?, "channel_new")?;
            if capacity < 0 {
                return Err("channel_new capacity must be non-negative".to_string());
            }
            let handle = insert_runtime_channel(state, type_args, capacity as usize);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(handle)))
        }
        RuntimeIntrinsic::ConcurrentChannelSend => {
            if args.len() != 2 {
                return Err("channel_send expects two arguments".to_string());
            }
            let handle = expect_channel(args[0].clone(), "channel_send")?;
            let channel = state
                .channels
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Channel handle `{}`", handle.0))?;
            if channel.queue.len() >= channel.capacity {
                return Err("channel_send would exceed channel capacity".to_string());
            }
            channel.queue.push_back(args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentChannelRecv => {
            let handle = expect_channel(expect_single_arg(args, "channel_recv")?, "channel_recv")?;
            let channel = state
                .channels
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Channel handle `{}`", handle.0))?;
            channel
                .queue
                .pop_front()
                .ok_or_else(|| "channel_recv called on empty channel".to_string())
        }
        RuntimeIntrinsic::ConcurrentMutexNew => {
            let value = expect_single_arg(args, "mutex_new")?;
            let handle = insert_runtime_mutex(state, type_args, value);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(handle)))
        }
        RuntimeIntrinsic::ConcurrentMutexTake => {
            let handle = expect_mutex(expect_single_arg(args, "mutex_take")?, "mutex_take")?;
            let mutex = state
                .mutexes
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Mutex handle `{}`", handle.0))?;
            mutex
                .value
                .take()
                .ok_or_else(|| "mutex_take called on empty mutex".to_string())
        }
        RuntimeIntrinsic::ConcurrentMutexPut => {
            if args.len() != 2 {
                return Err("mutex_put expects two arguments".to_string());
            }
            let handle = expect_mutex(args[0].clone(), "mutex_put")?;
            let mutex = state
                .mutexes
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Mutex handle `{}`", handle.0))?;
            if mutex.value.is_some() {
                return Err("mutex_put called while mutex already holds a value".to_string());
            }
            mutex.value = Some(args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentAtomicIntNew => {
            let value = expect_int(expect_single_arg(args, "atomic_int_new")?, "atomic_int_new")?;
            let handle = insert_runtime_atomic_int(state, value);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicInt(handle)))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntLoad => {
            let handle = expect_atomic_int(
                expect_single_arg(args, "atomic_int_load")?,
                "atomic_int_load",
            )?;
            let value = state
                .atomic_ints
                .get(&handle)
                .copied()
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(value))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntStore => {
            if args.len() != 2 {
                return Err("atomic_int_store expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_store")?;
            let value = expect_int(args[1].clone(), "atomic_int_store")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            *slot = value;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentAtomicIntAdd => {
            if args.len() != 2 {
                return Err("atomic_int_add expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_add")?;
            let delta = expect_int(args[1].clone(), "atomic_int_add")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            *slot += delta;
            Ok(RuntimeValue::Int(*slot))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntSub => {
            if args.len() != 2 {
                return Err("atomic_int_sub expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_sub")?;
            let delta = expect_int(args[1].clone(), "atomic_int_sub")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            *slot -= delta;
            Ok(RuntimeValue::Int(*slot))
        }
        RuntimeIntrinsic::ConcurrentAtomicIntSwap => {
            if args.len() != 2 {
                return Err("atomic_int_swap expects two arguments".to_string());
            }
            let handle = expect_atomic_int(args[0].clone(), "atomic_int_swap")?;
            let value = expect_int(args[1].clone(), "atomic_int_swap")?;
            let slot = state
                .atomic_ints
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicInt handle `{}`", handle.0))?;
            let old = *slot;
            *slot = value;
            Ok(RuntimeValue::Int(old))
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolNew => {
            let value = expect_bool(
                expect_single_arg(args, "atomic_bool_new")?,
                "atomic_bool_new",
            )?;
            let handle = insert_runtime_atomic_bool(state, value);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicBool(handle)))
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolLoad => {
            let handle = expect_atomic_bool(
                expect_single_arg(args, "atomic_bool_load")?,
                "atomic_bool_load",
            )?;
            let value = state
                .atomic_bools
                .get(&handle)
                .copied()
                .ok_or_else(|| format!("invalid AtomicBool handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(value))
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolStore => {
            if args.len() != 2 {
                return Err("atomic_bool_store expects two arguments".to_string());
            }
            let handle = expect_atomic_bool(args[0].clone(), "atomic_bool_store")?;
            let value = expect_bool(args[1].clone(), "atomic_bool_store")?;
            let slot = state
                .atomic_bools
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicBool handle `{}`", handle.0))?;
            *slot = value;
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ConcurrentAtomicBoolSwap => {
            if args.len() != 2 {
                return Err("atomic_bool_swap expects two arguments".to_string());
            }
            let handle = expect_atomic_bool(args[0].clone(), "atomic_bool_swap")?;
            let value = expect_bool(args[1].clone(), "atomic_bool_swap")?;
            let slot = state
                .atomic_bools
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid AtomicBool handle `{}`", handle.0))?;
            let old = *slot;
            *slot = value;
            Ok(RuntimeValue::Bool(old))
        }
        RuntimeIntrinsic::MemoryArenaNew => {
            let capacity = expect_int(expect_single_arg(args, "arena_new")?, "arena_new")?;
            if capacity < 0 {
                return Err("arena_new capacity must be non-negative".to_string());
            }
            let handle = insert_runtime_arena(state, type_args);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(handle)))
        }
        RuntimeIntrinsic::MemoryArenaAlloc => {
            if args.len() != 2 {
                return Err("arena_alloc expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_alloc")?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            let slot = arena.next_slot;
            arena.next_slot += 1;
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(
                RuntimeArenaIdValue {
                    arena: handle,
                    slot,
                    generation: arena.generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryArenaLen => {
            let handle = expect_arena(expect_single_arg(args, "arena_len")?, "arena_len")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemoryArenaHas => {
            if args.len() != 2 {
                return Err("arena_has expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_has")?;
            let id = expect_arena_id(args[1].clone(), "arena_has")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(arena_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryArenaGet => {
            if args.len() != 2 {
                return Err("arena access expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_access")?;
            let id = expect_arena_id(args[1].clone(), "arena_access")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("Arena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryArenaBorrowRead | RuntimeIntrinsic::MemoryArenaBorrowEdit => {
            if args.len() != 2 {
                return Err("arena borrow expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_borrow")?;
            let id = expect_arena_id(args[1].clone(), "arena_borrow")?;
            let arena = state
                .arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mutable: matches!(intrinsic, RuntimeIntrinsic::MemoryArenaBorrowEdit),
                target: RuntimeReferenceTarget::ArenaSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryArenaSet => {
            if args.len() != 3 {
                return Err("arena_set expects three arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_set")?;
            let id = expect_arena_id(args[1].clone(), "arena_set")?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryArenaRemove => {
            if args.len() != 2 {
                return Err("arena_remove expects two arguments".to_string());
            }
            let handle = expect_arena(args[0].clone(), "arena_remove")?;
            let id = expect_arena_id(args[1].clone(), "arena_remove")?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            if !arena_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid ArenaId `{}` for Arena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(id))),
                    handle.0
                ));
            }
            arena.slots.remove(&id.slot);
            Ok(RuntimeValue::Bool(true))
        }
        RuntimeIntrinsic::MemoryArenaReset => {
            let handle = expect_arena(expect_single_arg(args, "arena_reset")?, "arena_reset")?;
            let arena = state
                .arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid Arena handle `{}`", handle.0))?;
            arena.generation += 1;
            arena.next_slot = 0;
            arena.slots.clear();
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryFrameNew => {
            let capacity = expect_int(expect_single_arg(args, "frame_new")?, "frame_new")?;
            if capacity < 0 {
                return Err("frame_new capacity must be non-negative".to_string());
            }
            let handle = insert_runtime_frame_arena(state, type_args);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle)))
        }
        RuntimeIntrinsic::MemoryFrameAlloc => {
            if args.len() != 2 {
                return Err("frame_alloc expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_alloc")?;
            let arena = state
                .frame_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            let slot = arena.next_slot;
            arena.next_slot += 1;
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(
                RuntimeFrameIdValue {
                    arena: handle,
                    slot,
                    generation: arena.generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryFrameLen => {
            let handle = expect_frame_arena(expect_single_arg(args, "frame_len")?, "frame_len")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemoryFrameHas => {
            if args.len() != 2 {
                return Err("frame_has expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_has")?;
            let id = expect_frame_id(args[1].clone(), "frame_has")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(frame_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryFrameGet => {
            if args.len() != 2 {
                return Err("frame access expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_access")?;
            let id = expect_frame_id(args[1].clone(), "frame_access")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            if !frame_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("FrameArena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryFrameBorrowRead | RuntimeIntrinsic::MemoryFrameBorrowEdit => {
            if args.len() != 2 {
                return Err("frame borrow expects two arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_borrow")?;
            let id = expect_frame_id(args[1].clone(), "frame_borrow")?;
            let arena = state
                .frame_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            if !frame_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mutable: matches!(intrinsic, RuntimeIntrinsic::MemoryFrameBorrowEdit),
                target: RuntimeReferenceTarget::FrameSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryFrameSet => {
            if args.len() != 3 {
                return Err("frame_set expects three arguments".to_string());
            }
            let handle = expect_frame_arena(args[0].clone(), "frame_set")?;
            let id = expect_frame_id(args[1].clone(), "frame_set")?;
            let arena = state
                .frame_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            if !frame_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid FrameId `{}` for FrameArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryFrameReset => {
            let handle =
                expect_frame_arena(expect_single_arg(args, "frame_reset")?, "frame_reset")?;
            let arena = state
                .frame_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
            arena.generation += 1;
            arena.next_slot = 0;
            arena.slots.clear();
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryPoolNew => {
            let capacity = expect_int(expect_single_arg(args, "pool_new")?, "pool_new")?;
            if capacity < 0 {
                return Err("pool_new capacity must be non-negative".to_string());
            }
            let handle = insert_runtime_pool_arena(state, type_args);
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(handle)))
        }
        RuntimeIntrinsic::MemoryPoolAlloc => {
            if args.len() != 2 {
                return Err("pool_alloc expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_alloc")?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            let slot = arena.free_slots.pop().unwrap_or_else(|| {
                let slot = arena.next_slot;
                arena.next_slot += 1;
                arena.generations.entry(slot).or_insert(0);
                slot
            });
            let generation = pool_slot_generation(arena, slot);
            arena.slots.insert(slot, args[1].clone());
            Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(
                RuntimePoolIdValue {
                    arena: handle,
                    slot,
                    generation,
                },
            )))
        }
        RuntimeIntrinsic::MemoryPoolLen => {
            let handle = expect_pool_arena(expect_single_arg(args, "pool_len")?, "pool_len")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Int(arena.slots.len() as i64))
        }
        RuntimeIntrinsic::MemoryPoolHas => {
            if args.len() != 2 {
                return Err("pool_has expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_has")?;
            let id = expect_pool_id(args[1].clone(), "pool_has")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            Ok(RuntimeValue::Bool(pool_id_is_live(handle, arena, id)))
        }
        RuntimeIntrinsic::MemoryPoolGet => {
            if args.len() != 2 {
                return Err("pool access expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_access")?;
            let id = expect_pool_id(args[1].clone(), "pool_access")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("PoolArena slot `{}` is missing", id.slot))
        }
        RuntimeIntrinsic::MemoryPoolBorrowRead | RuntimeIntrinsic::MemoryPoolBorrowEdit => {
            if args.len() != 2 {
                return Err("pool borrow expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_borrow")?;
            let id = expect_pool_id(args[1].clone(), "pool_borrow")?;
            let arena = state
                .pool_arenas
                .get(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                mutable: matches!(intrinsic, RuntimeIntrinsic::MemoryPoolBorrowEdit),
                target: RuntimeReferenceTarget::PoolSlot {
                    id,
                    members: Vec::new(),
                },
            }))
        }
        RuntimeIntrinsic::MemoryPoolSet => {
            if args.len() != 3 {
                return Err("pool_set expects three arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_set")?;
            let id = expect_pool_id(args[1].clone(), "pool_set")?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            arena.slots.insert(id.slot, args[2].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MemoryPoolRemove => {
            if args.len() != 2 {
                return Err("pool_remove expects two arguments".to_string());
            }
            let handle = expect_pool_arena(args[0].clone(), "pool_remove")?;
            let id = expect_pool_id(args[1].clone(), "pool_remove")?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            if !pool_id_is_live(handle, arena, id) {
                return Err(format!(
                    "stale or invalid PoolId `{}` for PoolArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(id))),
                    handle.0
                ));
            }
            arena.slots.remove(&id.slot);
            *arena.generations.entry(id.slot).or_insert(0) += 1;
            arena.free_slots.push(id.slot);
            Ok(RuntimeValue::Bool(true))
        }
        RuntimeIntrinsic::MemoryPoolReset => {
            let handle = expect_pool_arena(expect_single_arg(args, "pool_reset")?, "pool_reset")?;
            let arena = state
                .pool_arenas
                .get_mut(&handle)
                .ok_or_else(|| format!("invalid PoolArena handle `{}`", handle.0))?;
            arena.slots.clear();
            for generation in arena.generations.values_mut() {
                *generation += 1;
            }
            arena.free_slots = arena.generations.keys().copied().rev().collect();
            Ok(RuntimeValue::Unit)
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
        RuntimeIntrinsic::ListLen => {
            let value = expect_single_arg(args, "list_len")?;
            let RuntimeValue::List(values) = value else {
                return Err("list_len expects List".to_string());
            };
            Ok(RuntimeValue::Int(i64::try_from(values.len()).map_err(
                |_| "list length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::ListPush => {
            if args.len() != 2 {
                return Err("list_push expects two arguments".to_string());
            }
            let Some(RuntimeValue::List(values)) = final_args.get_mut(0) else {
                return Err("list_push expects List".to_string());
            };
            values.push(args[1].clone());
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::ListPop => {
            if args.len() != 1 {
                return Err("list_pop expects one argument".to_string());
            }
            let Some(RuntimeValue::List(values)) = final_args.get_mut(0) else {
                return Err("list_pop expects List".to_string());
            };
            values
                .pop()
                .ok_or_else(|| "list_pop called on empty list".to_string())
        }
        RuntimeIntrinsic::ListTryPopOr => {
            if args.len() != 2 {
                return Err("list_try_pop_or expects two arguments".to_string());
            }
            let Some(RuntimeValue::List(values)) = final_args.get_mut(0) else {
                return Err("list_try_pop_or expects List".to_string());
            };
            Ok(match values.pop() {
                Some(value) => make_pair(RuntimeValue::Bool(true), value),
                None => make_pair(RuntimeValue::Bool(false), args[1].clone()),
            })
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
        RuntimeIntrinsic::MapNew => {
            if !args.is_empty() {
                return Err("map_new expects zero arguments".to_string());
            }
            Ok(RuntimeValue::Map(Vec::new()))
        }
        RuntimeIntrinsic::MapLen => {
            let value = expect_single_arg(args, "map_len")?;
            let RuntimeValue::Map(entries) = value else {
                return Err("map_len expects Map".to_string());
            };
            Ok(RuntimeValue::Int(i64::try_from(entries.len()).map_err(
                |_| "map length does not fit in i64".to_string(),
            )?))
        }
        RuntimeIntrinsic::MapHas => {
            if args.len() != 2 {
                return Err("map_has expects two arguments".to_string());
            }
            let RuntimeValue::Map(entries) = args[0].clone() else {
                return Err("map_has expects Map".to_string());
            };
            Ok(RuntimeValue::Bool(
                entries.iter().any(|(entry_key, _)| *entry_key == args[1]),
            ))
        }
        RuntimeIntrinsic::MapGet => {
            if args.len() != 2 {
                return Err("map_get expects two arguments".to_string());
            }
            let RuntimeValue::Map(entries) = args[0].clone() else {
                return Err("map_get expects Map".to_string());
            };
            entries
                .into_iter()
                .find_map(|(entry_key, entry_value)| (entry_key == args[1]).then_some(entry_value))
                .ok_or_else(|| "map_get key was not present".to_string())
        }
        RuntimeIntrinsic::MapSet => {
            if args.len() != 3 {
                return Err("map_set expects three arguments".to_string());
            }
            let Some(RuntimeValue::Map(entries)) = final_args.get_mut(0) else {
                return Err("map_set expects Map".to_string());
            };
            if let Some((_, entry_value)) = entries
                .iter_mut()
                .find(|(entry_key, _)| *entry_key == args[1])
            {
                *entry_value = args[2].clone();
            } else {
                entries.push((args[1].clone(), args[2].clone()));
            }
            Ok(RuntimeValue::Unit)
        }
        RuntimeIntrinsic::MapRemove => {
            if args.len() != 2 {
                return Err("map_remove expects two arguments".to_string());
            }
            let Some(RuntimeValue::Map(entries)) = final_args.get_mut(0) else {
                return Err("map_remove expects Map".to_string());
            };
            let original_len = entries.len();
            entries.retain(|(entry_key, _)| *entry_key != args[1]);
            Ok(RuntimeValue::Bool(entries.len() != original_len))
        }
        RuntimeIntrinsic::MapTryGetOr => {
            if args.len() != 3 {
                return Err("map_try_get_or expects three arguments".to_string());
            }
            let RuntimeValue::Map(entries) = args[0].clone() else {
                return Err("map_try_get_or expects Map".to_string());
            };
            Ok(
                match entries.into_iter().find_map(|(entry_key, entry_value)| {
                    (entry_key == args[1]).then_some(entry_value)
                }) {
                    Some(value) => make_pair(RuntimeValue::Bool(true), value),
                    None => make_pair(RuntimeValue::Bool(false), args[2].clone()),
                },
            )
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
            RuntimeValue::Int(value) => text.parse::<i64>().is_ok_and(|literal| *value == literal),
            RuntimeValue::Bool(value) => match text.as_str() {
                "true" => *value,
                "false" => !*value,
                _ => false,
            },
            RuntimeValue::Str(value) => text
                .starts_with('"')
                .then(|| decode_source_string_literal(text).ok())
                .flatten()
                .is_some_and(|literal| literal == *value),
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
        ParsedBinaryOp::Add => apply_runtime_add(left, right, "+"),
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

fn normalized_chain_indices(
    introducer: ParsedChainIntroducer,
    steps: &[ParsedChainStep],
) -> Vec<usize> {
    if steps.is_empty() {
        return Vec::new();
    }
    match introducer {
        ParsedChainIntroducer::Reverse => (0..steps.len()).rev().collect(),
        ParsedChainIntroducer::Forward => {
            let reverse_start = steps.iter().enumerate().skip(1).find_map(|(index, step)| {
                (step.incoming == Some(ParsedChainConnector::Reverse)).then_some(index)
            });
            match reverse_start {
                Some(start) => {
                    let mut out = (0..start).collect::<Vec<_>>();
                    for index in (start..steps.len()).rev() {
                        out.push(index);
                    }
                    out
                }
                None => (0..steps.len()).collect(),
            }
        }
    }
}

fn build_runtime_call_args_from_chain_stage(
    stage: &ParsedChainStep,
    input: Option<RuntimeValue>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<(Vec<String>, Vec<String>, Vec<RuntimeCallArg>)> {
    let callable = resolve_callable_path(&stage.stage, aliases)
        .ok_or_else(|| format!("unsupported runtime chain stage `{}`", stage.text))?;
    let type_args =
        resolve_runtime_type_args(&extract_generic_type_args(&stage.stage), type_bindings);
    let mut call_args = Vec::new();
    if let Some(input_value) = input {
        call_args.push(RuntimeCallArg {
            name: None,
            value: input_value,
            source_expr: ParsedExpr::Path(vec!["$chain_input".to_string()]),
        });
    }
    for bound in &stage.bind_args {
        call_args.push(RuntimeCallArg {
            name: None,
            value: eval_expr(
                bound,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?,
            source_expr: bound.clone(),
        });
    }
    Ok((callable, type_args, call_args))
}

fn reject_edit_chain_stage_call(
    callable: &[String],
    current_module_id: &str,
    call_args: &[RuntimeCallArg],
    plan: &RuntimePackagePlan,
) -> RuntimeEvalResult<()> {
    if let Some(routine_index) = resolve_routine_index_for_call(
        plan,
        current_module_id,
        callable,
        call_args,
        None,
        None,
        false,
        None,
    )? {
        let routine = plan
            .routines
            .get(routine_index)
            .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
        if routine
            .params
            .iter()
            .any(|param| param.mode.as_deref() == Some("edit"))
        {
            return Err(format!(
                "chain stage `{}` does not yet support `edit` parameters",
                callable.join(".")
            )
            .into());
        }
        return Ok(());
    }
    let intrinsic = resolve_runtime_intrinsic_path(callable)
        .ok_or_else(|| format!("unsupported runtime callable `{}`", callable.join(".")))?;
    if !intrinsic_edit_arg_indices(intrinsic).is_empty() {
        return Err(format!(
            "chain stage `{}` does not yet support `edit` intrinsic arguments",
            callable.join(".")
        )
        .into());
    }
    Ok(())
}

fn execute_runtime_chain_stage(
    stage: &ParsedChainStep,
    input: Option<RuntimeValue>,
    allow_async: bool,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let (callable, type_args, call_args) = build_runtime_call_args_from_chain_stage(
        stage,
        input,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    reject_edit_chain_stage_call(&callable, current_module_id, &call_args, plan)?;
    Ok(execute_call_by_path(
        &callable,
        None,
        None,
        current_module_id,
        type_args,
        call_args,
        false,
        plan,
        scopes,
        state,
        host,
        allow_async,
    )?)
}

fn spawn_runtime_chain_stage(
    op: ParsedUnaryOp,
    stage: &ParsedChainStep,
    input: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let (callable, type_args, call_args) = build_runtime_call_args_from_chain_stage(
        stage,
        Some(input),
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    reject_edit_chain_stage_call(&callable, current_module_id, &call_args, plan)?;
    if let Some(routine_index) = resolve_routine_index_for_call(
        plan,
        current_module_id,
        &callable,
        &call_args,
        None,
        None,
        false,
        None,
    )? {
        let routine = plan
            .routines
            .get(routine_index)
            .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
        let bound_args = bind_call_args_for_routine(routine, call_args.clone())?;
        consume_take_bound_args(scopes, routine, &bound_args)?;
    } else {
        let intrinsic = resolve_runtime_intrinsic_path(&callable)
            .ok_or_else(|| format!("unsupported runtime callable `{}`", callable.join(".")))?;
        consume_take_call_args(scopes, intrinsic_take_arg_indices(intrinsic), &call_args)?;
    }
    let thread_id = match op {
        ParsedUnaryOp::Weave => state.current_thread_id,
        ParsedUnaryOp::Split => allocate_scheduler_thread_id(state),
        _ => unreachable!(),
    };
    let pending = RuntimePendingState::Pending(RuntimeDeferredWork::Call(RuntimeDeferredCall {
        callable,
        resolved_routine: None,
        dynamic_dispatch: None,
        current_module_id: current_module_id.to_string(),
        type_args: type_args.clone(),
        call_args,
        scopes: scopes.clone(),
        thread_id,
        allow_async: true,
    }));
    let value = match op {
        ParsedUnaryOp::Weave => RuntimeValue::Opaque(RuntimeOpaqueValue::Task(
            insert_runtime_task(state, &type_args, pending),
        )),
        ParsedUnaryOp::Split => RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(
            insert_runtime_thread(state, &type_args, pending),
        )),
        _ => unreachable!(),
    };
    Ok(value)
}

fn eval_runtime_chain_expr(
    style: &str,
    introducer: ParsedChainIntroducer,
    steps: &[ParsedChainStep],
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
    if steps.is_empty() {
        return Err("runtime chain expression must contain at least one stage"
            .to_string()
            .into());
    }
    let ordered = normalized_chain_indices(introducer, steps)
        .into_iter()
        .map(|index| &steps[index])
        .collect::<Vec<_>>();
    let seed = execute_runtime_chain_stage(
        ordered[0],
        None,
        true,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    if ordered.len() == 1 {
        return Ok(seed);
    }
    match style {
        "forward" | "lazy" | "async" | "plan" => {
            let mut current = seed;
            for stage in ordered.iter().skip(1) {
                current = execute_runtime_chain_stage(
                    stage,
                    Some(current),
                    true,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                if style == "async" {
                    match current {
                        RuntimeValue::Opaque(RuntimeOpaqueValue::Task(_))
                        | RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(_)) => {
                            current = await_runtime_value(current, plan, state, host)?
                        }
                        _ => {}
                    }
                }
            }
            Ok(current)
        }
        "collect" | "broadcast" => {
            let mut values = Vec::new();
            for stage in ordered.iter().skip(1) {
                values.push(execute_runtime_chain_stage(
                    stage,
                    Some(seed.clone()),
                    true,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?);
            }
            Ok(RuntimeValue::List(values))
        }
        "parallel" => {
            let mut values = Vec::new();
            for stage in ordered.iter().skip(1) {
                let is_async_stage = build_runtime_call_args_from_chain_stage(
                    stage,
                    Some(seed.clone()),
                    plan,
                    current_module_id,
                    &mut scopes.clone(),
                    aliases,
                    type_bindings,
                    state,
                    host,
                )
                .ok()
                .and_then(|(callable, _, call_args)| {
                    resolve_routine_index_for_call(
                        plan,
                        current_module_id,
                        &callable,
                        &call_args,
                        None,
                        None,
                        false,
                        None,
                    )
                    .ok()
                    .flatten()
                })
                .and_then(|routine_index| plan.routines.get(routine_index))
                .map(|routine| routine.is_async)
                .unwrap_or(false);
                let spawned = spawn_runtime_chain_stage(
                    if is_async_stage {
                        ParsedUnaryOp::Weave
                    } else {
                        ParsedUnaryOp::Split
                    },
                    stage,
                    seed.clone(),
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                values.push(await_runtime_value(spawned, plan, state, host)?);
            }
            Ok(RuntimeValue::List(values))
        }
        other => Err(format!("unsupported runtime chain style `{other}`").into()),
    }
}

fn pending_state_is_done(state: &RuntimePendingState) -> bool {
    matches!(
        state,
        RuntimePendingState::Completed(_) | RuntimePendingState::Failed(_)
    )
}

fn pending_state_value(state: &RuntimePendingState, context: &str) -> Result<RuntimeValue, String> {
    match state {
        RuntimePendingState::Completed(value) => Ok(value.clone()),
        RuntimePendingState::Failed(message) => Err(message.clone()),
        RuntimePendingState::Pending(_) => Err(format!("{context} is not completed")),
        RuntimePendingState::Running => Err(format!("{context} is already running")),
    }
}

fn execute_deferred_work(
    pending: RuntimeDeferredWork,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match pending {
        RuntimeDeferredWork::Call(pending) => {
            let RuntimeDeferredCall {
                callable,
                resolved_routine,
                dynamic_dispatch,
                current_module_id,
                type_args,
                call_args,
                mut scopes,
                thread_id,
                allow_async,
            } = pending;
            let previous_thread_id = state.current_thread_id;
            state.current_thread_id = thread_id;
            let result = execute_call_by_path(
                &callable,
                resolved_routine.as_deref(),
                dynamic_dispatch.as_ref(),
                &current_module_id,
                type_args,
                call_args,
                false,
                plan,
                &mut scopes,
                state,
                host,
                allow_async,
            );
            state.current_thread_id = previous_thread_id;
            result
        }
        RuntimeDeferredWork::Expr(pending) => {
            let RuntimeDeferredExpr {
                expr,
                current_module_id,
                aliases,
                type_bindings,
                mut scopes,
                thread_id,
                allow_async,
            } = pending;
            let previous_thread_id = state.current_thread_id;
            let previous_async_depth = state.async_context_depth;
            state.current_thread_id = thread_id;
            if allow_async {
                state.async_context_depth += 1;
            }
            let result = eval_expr(
                &expr,
                plan,
                &current_module_id,
                &mut scopes,
                &aliases,
                &type_bindings,
                state,
                host,
            )
            .map_err(runtime_eval_message);
            state.current_thread_id = previous_thread_id;
            state.async_context_depth = previous_async_depth;
            result
        }
    }
}

fn drive_runtime_task(
    handle: RuntimeTaskHandle,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    let pending = {
        let task = state
            .tasks
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Task handle `{}`", handle.0))?;
        match std::mem::replace(&mut task.state, RuntimePendingState::Running) {
            RuntimePendingState::Pending(pending) => pending,
            RuntimePendingState::Completed(value) => {
                task.state = RuntimePendingState::Completed(value);
                return Ok(());
            }
            RuntimePendingState::Failed(message) => {
                task.state = RuntimePendingState::Failed(message);
                return Ok(());
            }
            RuntimePendingState::Running => {
                return Err(format!("Task `{}` is already running", handle.0));
            }
        }
    };
    let next_state = match execute_deferred_work(pending, plan, state, host) {
        Ok(value) => RuntimePendingState::Completed(value),
        Err(message) => RuntimePendingState::Failed(message),
    };
    state
        .tasks
        .get_mut(&handle)
        .ok_or_else(|| format!("invalid Task handle `{}`", handle.0))?
        .state = next_state;
    Ok(())
}

fn drive_runtime_thread(
    handle: RuntimeThreadHandle,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    let pending = {
        let thread = state
            .threads
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Thread handle `{}`", handle.0))?;
        match std::mem::replace(&mut thread.state, RuntimePendingState::Running) {
            RuntimePendingState::Pending(pending) => pending,
            RuntimePendingState::Completed(value) => {
                thread.state = RuntimePendingState::Completed(value);
                return Ok(());
            }
            RuntimePendingState::Failed(message) => {
                thread.state = RuntimePendingState::Failed(message);
                return Ok(());
            }
            RuntimePendingState::Running => {
                return Err(format!("Thread `{}` is already running", handle.0));
            }
        }
    };
    let next_state = match execute_deferred_work(pending, plan, state, host) {
        Ok(value) => RuntimePendingState::Completed(value),
        Err(message) => RuntimePendingState::Failed(message),
    };
    state
        .threads
        .get_mut(&handle)
        .ok_or_else(|| format!("invalid Thread handle `{}`", handle.0))?
        .state = next_state;
    Ok(())
}

fn capture_spawned_phrase_call(
    op: ParsedUnaryOp,
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier_kind: ParsedPhraseQualifierKind,
    qualifier: &str,
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
    let (callable, type_args, call_args, call_routine, call_dynamic_dispatch) = match qualifier_kind
    {
        ParsedPhraseQualifierKind::Call | ParsedPhraseQualifierKind::Apply => {
            if qualifier != "call" && qualifier_kind == ParsedPhraseQualifierKind::Call {
                return Ok(None);
            }
            let callable = resolved_callable
                .map(|path| path.to_vec())
                .or_else(|| resolve_callable_path(subject, aliases))
                .ok_or_else(|| format!("unsupported runtime callable `{subject:?}`"))?;
            let type_args =
                resolve_runtime_type_args(&extract_generic_type_args(subject), type_bindings);
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
            (
                callable,
                type_args,
                call_args,
                resolved_routine.map(ToString::to_string),
                None,
            )
        }
        ParsedPhraseQualifierKind::NamedPath => {
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
            let callable_expr = parse_runtime_callable_expr(qualifier);
            let callable = resolve_named_qualifier_callable_path(&callable_expr, aliases)
                .ok_or_else(|| {
                    format!("unsupported runtime named qualifier callable `{qualifier}`")
                })?;
            let type_args = resolve_runtime_type_args(
                &extract_generic_type_args(&callable_expr),
                type_bindings,
            );
            let mut call_args = vec![RuntimeCallArg {
                name: None,
                value: receiver,
                source_expr: subject.clone(),
            }];
            call_args.extend(collect_call_args(
                args,
                attached,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?);
            (callable, type_args, call_args, None, None)
        }
        ParsedPhraseQualifierKind::BareMethod => {
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
            let callable = resolved_callable
                .ok_or_else(|| {
                    format!(
                        "runtime bare-method qualifier `{qualifier}` is missing lowered callable identity"
                    )
                })?
                .to_vec();
            let type_args = runtime_receiver_type_args(&receiver, state);
            let mut call_args = vec![RuntimeCallArg {
                name: None,
                value: receiver,
                source_expr: subject.clone(),
            }];
            call_args.extend(collect_call_args(
                args,
                attached,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?);
            (
                callable,
                type_args,
                call_args,
                resolved_routine.map(ToString::to_string),
                dynamic_dispatch.cloned(),
            )
        }
        ParsedPhraseQualifierKind::Try | ParsedPhraseQualifierKind::AwaitApply => return Ok(None),
    };

    if let Some(routine_index) = resolve_routine_index_for_call(
        plan,
        current_module_id,
        &callable,
        &call_args,
        call_routine.as_deref(),
        call_dynamic_dispatch.as_ref(),
        call_routine.is_none() && qualifier_kind == ParsedPhraseQualifierKind::BareMethod,
        Some(state),
    )? {
        let routine = plan
            .routines
            .get(routine_index)
            .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
        let bound_args = bind_call_args_for_routine(routine, call_args.clone())?;
        if routine
            .params
            .iter()
            .any(|param| param.mode.as_deref() == Some("edit"))
        {
            return Err("spawned runtime calls do not yet support `edit` parameters"
                .to_string()
                .into());
        }
        consume_take_bound_args(scopes, routine, &bound_args)?;
    } else {
        let intrinsic = resolve_runtime_intrinsic_path(&callable)
            .ok_or_else(|| format!("unsupported runtime callable `{}`", callable.join(".")))?;
        if !intrinsic_edit_arg_indices(intrinsic).is_empty() {
            return Err(
                "spawned runtime intrinsic calls do not yet support `edit` arguments"
                    .to_string()
                    .into(),
            );
        }
        consume_take_call_args(scopes, intrinsic_take_arg_indices(intrinsic), &call_args)?;
    }

    let thread_id = match op {
        ParsedUnaryOp::Weave => state.current_thread_id,
        ParsedUnaryOp::Split => allocate_scheduler_thread_id(state),
        _ => unreachable!(),
    };
    let pending = RuntimePendingState::Pending(RuntimeDeferredWork::Call(RuntimeDeferredCall {
        callable,
        resolved_routine: call_routine,
        dynamic_dispatch: call_dynamic_dispatch,
        current_module_id: current_module_id.to_string(),
        type_args: type_args.clone(),
        call_args,
        scopes: scopes.clone(),
        thread_id,
        allow_async: true,
    }));
    let value = match op {
        ParsedUnaryOp::Weave => RuntimeValue::Opaque(RuntimeOpaqueValue::Task(
            insert_runtime_task(state, &type_args, pending),
        )),
        ParsedUnaryOp::Split => RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(
            insert_runtime_thread(state, &type_args, pending),
        )),
        _ => unreachable!(),
    };
    Ok(Some(value))
}

fn spawn_runtime_expr(
    op: ParsedUnaryOp,
    expr: &ParsedExpr,
    current_module_id: &str,
    scopes: &[RuntimeScope],
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
) -> RuntimeValue {
    let thread_id = match op {
        ParsedUnaryOp::Weave => state.current_thread_id,
        ParsedUnaryOp::Split => allocate_scheduler_thread_id(state),
        _ => unreachable!(),
    };
    let pending = RuntimePendingState::Pending(RuntimeDeferredWork::Expr(RuntimeDeferredExpr {
        expr: expr.clone(),
        current_module_id: current_module_id.to_string(),
        aliases: aliases.clone(),
        type_bindings: type_bindings.clone(),
        scopes: scopes.to_vec(),
        thread_id,
        allow_async: true,
    }));
    match op {
        ParsedUnaryOp::Weave => RuntimeValue::Opaque(RuntimeOpaqueValue::Task(
            insert_runtime_task(state, &[], pending),
        )),
        ParsedUnaryOp::Split => RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(
            insert_runtime_thread(state, &[], pending),
        )),
        _ => unreachable!(),
    }
}

fn await_runtime_value(
    value: RuntimeValue,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    match value {
        RuntimeValue::Opaque(RuntimeOpaqueValue::Task(handle)) => {
            drive_runtime_task(handle, plan, state, host)?;
            let task = state
                .tasks
                .get(&handle)
                .ok_or_else(|| format!("invalid Task handle `{}`", handle.0))?;
            pending_state_value(&task.state, &format!("Task `{}`", handle.0))
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(handle)) => {
            drive_runtime_thread(handle, plan, state, host)?;
            let thread = state
                .threads
                .get(&handle)
                .ok_or_else(|| format!("invalid Thread handle `{}`", handle.0))?;
            pending_state_value(&thread.state, &format!("Thread `{}`", handle.0))
        }
        other => Err(format!("await expects Task or Thread, got `{other:?}`")),
    }
}

fn eval_spawn_expr(
    op: ParsedUnaryOp,
    expr: &ParsedExpr,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<RuntimeValue> {
    if let ParsedExpr::Phrase {
        subject,
        args,
        qualifier_kind,
        qualifier,
        resolved_callable,
        resolved_routine,
        dynamic_dispatch,
        attached,
    } = expr
    {
        if let Some(spawned) = capture_spawned_phrase_call(
            op,
            subject,
            args,
            attached,
            *qualifier_kind,
            qualifier,
            resolved_callable.as_deref(),
            resolved_routine.as_deref(),
            dynamic_dispatch.as_ref(),
            plan,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )? {
            return Ok(spawned);
        }
    }
    Ok(spawn_runtime_expr(
        op,
        expr,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
    ))
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
) -> RuntimeEvalResult<RuntimeValue> {
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
                .collect::<RuntimeEvalResult<Vec<_>>>()?,
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
        ParsedExpr::Chain {
            style,
            introducer,
            steps,
        } => eval_runtime_chain_expr(
            style,
            *introducer,
            steps,
            plan,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        ParsedExpr::Path(segments) if segments.len() == 1 => {
            Ok(read_runtime_local_value(scopes, &segments[0])?)
        }
        ParsedExpr::Path(segments) => {
            Err(format!("unsupported runtime value path `{}`", segments.join(".")).into())
        }
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
            Ok(eval_runtime_member_value(
                base,
                member,
                scopes,
                plan,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )?)
        }
        ParsedExpr::Index { expr, index } => Ok(eval_runtime_index_value(
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
            eval_expr(
                index,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?,
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        )?),
        ParsedExpr::Slice {
            expr,
            start,
            end,
            inclusive_end,
        } => Ok(eval_runtime_slice_value(
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
            start
                .as_deref()
                .map(|expr| {
                    eval_expr(
                        expr,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )
                })
                .transpose()?,
            end.as_deref()
                .map(|expr| {
                    eval_expr(
                        expr,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )
                })
                .transpose()?,
            *inclusive_end,
            scopes,
            plan,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        )?),
        ParsedExpr::Range {
            start,
            end,
            inclusive_end,
        } => Ok(RuntimeValue::Range {
            start: eval_optional_runtime_int_expr(
                start.as_deref(),
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                "range start",
            )?,
            end: eval_optional_runtime_int_expr(
                end.as_deref(),
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                "range end",
            )?,
            inclusive_end: *inclusive_end,
        }),
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
        ParsedExpr::Await { expr } => Ok(await_runtime_value(
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
            plan,
            state,
            host,
        )?),
        ParsedExpr::Unary { op, expr } => match op {
            ParsedUnaryOp::Weave | ParsedUnaryOp::Split => eval_spawn_expr(
                *op,
                expr,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedUnaryOp::BorrowRead | ParsedUnaryOp::BorrowMut => {
                let target = expr_to_assign_target(expr).ok_or_else(|| {
                    format!(
                        "runtime borrow operand `{:?}` is not a writable place",
                        expr
                    )
                })?;
                let place = resolve_assign_target_place(scopes, &target)?;
                if matches!(op, ParsedUnaryOp::BorrowMut) && !place.mutable {
                    return Err(format!(
                        "runtime mutable borrow operand `{:?}` is not mutable",
                        expr
                    )
                    .into());
                }
                Ok(RuntimeValue::Ref(RuntimeReferenceValue {
                    mutable: matches!(op, ParsedUnaryOp::BorrowMut),
                    target: place.target,
                }))
            }
            ParsedUnaryOp::Deref => {
                let reference = expect_reference(
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
                    "deref",
                )?;
                Ok(read_runtime_reference(
                    scopes,
                    plan,
                    current_module_id,
                    aliases,
                    type_bindings,
                    state,
                    &reference,
                    host,
                )?)
            }
            ParsedUnaryOp::Neg => Ok(RuntimeValue::Int(-expect_int(
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
                "unary -",
            )?)),
            ParsedUnaryOp::Not => Ok(RuntimeValue::Bool(!expect_bool(
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
                "not",
            )?)),
            ParsedUnaryOp::BitNot => Ok(RuntimeValue::Int(!expect_int(
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
                "~",
            )?)),
        },
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
                Ok(apply_binary_op(*other, left, right)?)
            }
        },
        ParsedExpr::Phrase {
            subject,
            args,
            qualifier_kind,
            qualifier,
            resolved_callable,
            resolved_routine,
            dynamic_dispatch,
            attached,
        } => match qualifier_kind {
            ParsedPhraseQualifierKind::Call => execute_runtime_apply_phrase(
                subject,
                args,
                attached,
                resolved_callable.as_deref(),
                resolved_routine.as_deref(),
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                runtime_async_calls_allowed(state),
            ),
            ParsedPhraseQualifierKind::NamedPath => execute_runtime_named_qualifier_call(
                subject,
                args,
                attached,
                qualifier,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedPhraseQualifierKind::BareMethod => eval_qualifier(
                subject,
                args,
                attached,
                qualifier,
                resolved_callable.as_deref(),
                resolved_routine.as_deref(),
                dynamic_dispatch.as_ref(),
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedPhraseQualifierKind::Try => eval_try_qualifier(
                subject,
                args,
                attached,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            ),
            ParsedPhraseQualifierKind::Apply => execute_runtime_apply_phrase(
                subject,
                args,
                attached,
                resolved_callable.as_deref(),
                None,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                runtime_async_calls_allowed(state),
            ),
            ParsedPhraseQualifierKind::AwaitApply => {
                if args.is_empty() && attached.is_empty() {
                    return Ok(await_runtime_value(
                        eval_expr(
                            subject,
                            plan,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?,
                        plan,
                        state,
                        host,
                    )?);
                }
                let value = execute_runtime_apply_phrase(
                    subject,
                    args,
                    attached,
                    None,
                    None,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                    true,
                )?;
                match value {
                    RuntimeValue::Opaque(RuntimeOpaqueValue::Task(_))
                    | RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(_)) => {
                        Ok(await_runtime_value(value, plan, state, host)?)
                    }
                    other => Ok(other),
                }
            }
        },
        ParsedExpr::MemoryPhrase {
            family,
            arena,
            init_args,
            constructor,
            attached,
        } => {
            let arena_value = eval_expr(
                arena,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            let constructed = execute_runtime_apply_phrase(
                constructor,
                init_args,
                attached,
                None,
                None,
                plan,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
                false,
            )?;
            let intrinsic = match family.as_str() {
                "arena" => RuntimeIntrinsic::MemoryArenaAlloc,
                "frame" => RuntimeIntrinsic::MemoryFrameAlloc,
                "pool" => RuntimeIntrinsic::MemoryPoolAlloc,
                other => return Err(format!("unsupported runtime memory family `{other}`").into()),
            };
            let type_args = runtime_receiver_type_args(&arena_value, state);
            let mut values = vec![arena_value, constructed];
            Ok(execute_runtime_intrinsic(
                intrinsic,
                &type_args,
                &mut values,
                plan,
                state,
                host,
            )?)
        }
    }
}

fn apply_assign(
    target: &ParsedAssignTarget,
    op: ParsedAssignOp,
    value: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<()> {
    let updated = if matches!(op, ParsedAssignOp::Assign) {
        value
    } else {
        let current = read_assign_target_value_runtime(
            target,
            plan,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        apply_assignment_op(current, op, value)?
    };
    Ok(write_assign_target_value_runtime(
        target,
        updated,
        plan,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?)
}

fn run_scope_defers(
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<()> {
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

fn execute_page_rollups(
    frame: RuntimeRollupFrame,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<()> {
    for rollup in frame.rollups.into_iter().rev() {
        if rollup.kind != "cleanup" {
            return Err(format!("unsupported runtime rollup kind `{}`", rollup.kind).into());
        }
        let Some(subject_value) = frame
            .subjects
            .get(&rollup.subject)
            .and_then(|subject| subject.value.clone())
        else {
            continue;
        };
        let callable = validate_runtime_rollup_handler_callable_path(
            plan,
            current_module_id,
            &rollup.handler_path,
        )?;
        let _ = execute_call_by_path(
            &callable,
            None,
            None,
            current_module_id,
            Vec::new(),
            vec![RuntimeCallArg {
                name: None,
                value: subject_value,
                source_expr: ParsedExpr::Path(vec![rollup.subject.clone()]),
            }],
            false,
            plan,
            scopes,
            state,
            host,
            false,
        )?;
    }
    Ok(())
}

fn finish_runtime_rollups<T>(
    result: RuntimeEvalResult<T>,
    frame: Option<RuntimeRollupFrame>,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<T> {
    if let Some(frame) = frame {
        if !matches!(result, Err(RuntimeEvalSignal::Message(_))) {
            execute_page_rollups(frame, plan, current_module_id, scopes, state, host)?;
        }
    }
    result
}

fn execute_scoped_block(
    statements: &[ParsedStmt],
    rollups: &[ParsedPageRollup],
    scopes: &mut Vec<RuntimeScope>,
    scope: RuntimeScope,
    plan: &RuntimePackagePlan,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> RuntimeEvalResult<FlowSignal> {
    push_runtime_rollup_frame(state, rollups, scopes);
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
    let exited_scope = scopes
        .pop()
        .ok_or_else(|| "runtime scope stack is empty".to_string())?;
    defer_result?;
    let frame = if rollups.is_empty() {
        None
    } else {
        pop_runtime_rollup_frame(state)
    };
    let result =
        finish_runtime_rollups(result, frame, plan, current_module_id, scopes, state, host);
    if let Err(err) = evaluate_owner_exit_checkpoints(
        &exited_scope.activated_owner_keys,
        plan,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
        Some(scopes),
    ) {
        return Err(err.into());
    }
    release_scope_owner_activations(state, &exited_scope.activated_owner_keys);
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
) -> RuntimeEvalResult<FlowSignal> {
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
                let current_scope_depth = scopes.len().saturating_sub(1);
                let current_scope = scopes
                    .last_mut()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?;
                insert_runtime_local(
                    state,
                    current_scope_depth,
                    current_scope,
                    name.clone(),
                    *mutable,
                    value,
                );
                FlowSignal::Next
            }
            ParsedStmt::Expr { expr, rollups } => {
                push_runtime_rollup_frame(state, rollups, scopes);
                finish_runtime_rollups(
                    eval_expr(
                        expr,
                        plan,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )
                    .map(|_| FlowSignal::Next),
                    if rollups.is_empty() {
                        None
                    } else {
                        pop_runtime_rollup_frame(state)
                    },
                    plan,
                    current_module_id,
                    scopes,
                    state,
                    host,
                )?;
                FlowSignal::Next
            }
            ParsedStmt::ReturnVoid => FlowSignal::Return(RuntimeValue::Unit),
            ParsedStmt::ReturnValue { value } => FlowSignal::Return(match value {
                expr => eval_expr(
                    expr,
                    plan,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?,
            }),
            ParsedStmt::If {
                condition,
                then_branch,
                else_branch,
                rollups,
                availability,
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
                    let mut scope = RuntimeScope::default();
                    apply_runtime_availability_attachments(&mut scope, availability);
                    execute_scoped_block(
                        then_branch,
                        rollups,
                        scopes,
                        scope,
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
                    let mut scope = RuntimeScope::default();
                    apply_runtime_availability_attachments(&mut scope, availability);
                    execute_scoped_block(
                        else_branch,
                        rollups,
                        scopes,
                        scope,
                        plan,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?
                }
            }
            ParsedStmt::While {
                condition,
                body,
                rollups,
                availability,
            } => {
                push_runtime_rollup_frame(state, rollups, scopes);
                let result = (|| -> RuntimeEvalResult<FlowSignal> {
                    loop {
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
                            break Ok(FlowSignal::Next);
                        }
                        let mut scope = RuntimeScope::default();
                        apply_runtime_availability_attachments(&mut scope, availability);
                        match execute_scoped_block(
                            body,
                            &[],
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
                            FlowSignal::Break => break Ok(FlowSignal::Next),
                            FlowSignal::Return(value) => break Ok(FlowSignal::Return(value)),
                        }
                    }
                })();
                finish_runtime_rollups(
                    result,
                    if rollups.is_empty() {
                        None
                    } else {
                        pop_runtime_rollup_frame(state)
                    },
                    plan,
                    current_module_id,
                    scopes,
                    state,
                    host,
                )?
            }
            ParsedStmt::For {
                binding,
                iterable,
                body,
                rollups,
                availability,
            } => {
                push_runtime_rollup_frame(state, rollups, scopes);
                let result = (|| -> RuntimeEvalResult<FlowSignal> {
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
                        apply_runtime_availability_attachments(&mut scope, availability);
                        insert_runtime_local(
                            state,
                            scopes.len(),
                            &mut scope,
                            binding.clone(),
                            false,
                            value,
                        );
                        match execute_scoped_block(
                            body,
                            &[],
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
                    Ok(loop_signal)
                })();
                finish_runtime_rollups(
                    result,
                    if rollups.is_empty() {
                        None
                    } else {
                        pop_runtime_rollup_frame(state)
                    },
                    plan,
                    current_module_id,
                    scopes,
                    state,
                    host,
                )?
            }
            ParsedStmt::Defer(expr) => {
                scopes
                    .last_mut()
                    .ok_or_else(|| "runtime scope stack is empty".to_string())?
                    .deferred
                    .push(expr.clone());
                FlowSignal::Next
            }
            ParsedStmt::ActivateOwner { .. } => {
                let ParsedStmt::ActivateOwner {
                    owner_path,
                    owner_local_name: _,
                    binding,
                    context,
                } = statement
                else {
                    unreachable!();
                };
                let owner = lookup_runtime_owner_plan(plan, owner_path).ok_or_else(|| {
                    format!(
                        "runtime owner activation `{}` resolves to an unknown owner",
                        owner_path.join(".")
                    )
                })?;
                let owner_key = owner_state_key(owner_path);
                let had_prior_active_state = state
                    .owners
                    .get(&owner_key)
                    .map(|owner_state| owner_state.active_bindings > 0)
                    .unwrap_or(false);
                let context_value = context
                    .as_ref()
                    .map(|expr| {
                        eval_expr(
                            expr,
                            plan,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )
                    })
                    .transpose()?;
                let owner_state = state
                    .owners
                    .entry(owner_key.clone())
                    .or_insert_with(RuntimeOwnerState::default);
                if owner_state.active_bindings == 0 {
                    owner_state.activation_context = context_value;
                    owner_state.pending_init.clear();
                    owner_state.pending_resume = owner_state.objects.keys().cloned().collect();
                }
                activate_owner_scope_binding(scopes, state, owner, &owner_key, binding.as_deref())
                    .map_err(RuntimeEvalSignal::from)?;
                if had_prior_active_state {
                    evaluate_owner_exit_checkpoints(
                        std::slice::from_ref(&owner_key),
                        plan,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                        Some(scopes),
                    )
                    .map_err(RuntimeEvalSignal::from)?;
                }
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
                apply_assign(
                    target,
                    *op,
                    value,
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
        };
        if signal != FlowSignal::Next {
            return Ok(signal);
        }
    }
    Ok(FlowSignal::Next)
}

fn execute_routine_call_with_state(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    type_args: Vec<String>,
    mut args: Vec<RuntimeValue>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
    allow_async: bool,
) -> Result<RoutineExecutionOutcome, String> {
    let routine = plan
        .routines
        .get(routine_index)
        .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
    push_runtime_call_frame(state, &routine.module_id, &routine.symbol_name)?;
    let execution_result = (|| -> Result<RoutineExecutionOutcome, String> {
        if let Some(intrinsic_impl) = &routine.intrinsic_impl {
            let intrinsic = resolve_runtime_intrinsic_impl(intrinsic_impl).ok_or_else(|| {
                format!(
                    "unsupported runtime intrinsic `{intrinsic_impl}` for `{}`",
                    routine.symbol_name
                )
            })?;
            let value =
                execute_runtime_intrinsic(intrinsic, &type_args, &mut args, plan, state, host)?;
            return Ok(RoutineExecutionOutcome {
                value,
                final_args: args,
            });
        }
        if routine.is_async && !allow_async {
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
        if !routine.type_params.is_empty()
            && !type_args.is_empty()
            && type_args.len() != routine.type_params.len()
        {
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
        let resolved_type_args = if type_args.is_empty() {
            routine.type_params.clone()
        } else {
            type_args
        };
        let type_bindings = routine
            .type_params
            .iter()
            .cloned()
            .zip(resolved_type_args)
            .collect::<RuntimeTypeBindings>();
        let entered_async_context = routine.is_async;
        if entered_async_context {
            state.async_context_depth += 1;
        }
        let outcome = (|| -> Result<RoutineExecutionOutcome, String> {
            let mut initial_scope = RuntimeScope::default();
            apply_runtime_availability_attachments(&mut initial_scope, &routine.availability);
            push_runtime_rollup_frame(state, &routine.rollups, &[]);
            for (param, value) in routine.params.iter().zip(args) {
                insert_runtime_local(
                    state,
                    0,
                    &mut initial_scope,
                    param.name.clone(),
                    param.mode.as_deref() == Some("edit"),
                    value,
                );
            }
            let mut scopes = Vec::new();
            scopes.push(initial_scope);
            let result = execute_statements(
                &routine.statements,
                &mut scopes,
                plan,
                &routine.module_id,
                &aliases,
                &type_bindings,
                state,
                host,
            );
            let defer_result = run_scope_defers(
                plan,
                &routine.module_id,
                &mut scopes,
                &aliases,
                &type_bindings,
                state,
                host,
            );
            let routine_rollup_frame = pop_runtime_rollup_frame(state);
            let final_scope = scopes
                .pop()
                .ok_or_else(|| "runtime scope stack is empty".to_string())?;
            match defer_result {
                Ok(()) => {}
                Err(RuntimeEvalSignal::Message(message)) => return Err(message),
                Err(RuntimeEvalSignal::Return(value)) => {
                    if let Some(frame) = routine_rollup_frame {
                        execute_page_rollups(
                            frame,
                            plan,
                            &routine.module_id,
                            &mut scopes,
                            state,
                            host,
                        )
                        .map_err(runtime_eval_message)?;
                    }
                    evaluate_owner_exit_checkpoints(
                        &final_scope.activated_owner_keys,
                        plan,
                        &routine.module_id,
                        &aliases,
                        &type_bindings,
                        state,
                        host,
                        None,
                    )?;
                    release_scope_owner_activations(state, &final_scope.activated_owner_keys);
                    let final_args = routine
                        .params
                        .iter()
                        .map(|param| {
                            final_scope
                                .locals
                                .get(&param.name)
                                .map(|local| local.value.clone())
                                .ok_or_else(|| {
                                    format!(
                                        "runtime routine `{}` lost bound parameter `{}`",
                                        routine.symbol_name, param.name
                                    )
                                })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    return Ok(RoutineExecutionOutcome { value, final_args });
                }
            }
            if let Some(frame) = routine_rollup_frame {
                if !matches!(result, Err(RuntimeEvalSignal::Message(_))) {
                    execute_page_rollups(frame, plan, &routine.module_id, &mut scopes, state, host)
                        .map_err(runtime_eval_message)?;
                }
            }
            evaluate_owner_exit_checkpoints(
                &final_scope.activated_owner_keys,
                plan,
                &routine.module_id,
                &aliases,
                &type_bindings,
                state,
                host,
                None,
            )?;
            release_scope_owner_activations(state, &final_scope.activated_owner_keys);
            let result = match result {
                Ok(signal) => signal,
                Err(RuntimeEvalSignal::Message(message)) => return Err(message),
                Err(RuntimeEvalSignal::Return(value)) => FlowSignal::Return(value),
            };
            let value = match result {
                FlowSignal::Next => RuntimeValue::Unit,
                FlowSignal::Return(value) => value,
                FlowSignal::Break => return Err("break escaped the top-level routine".to_string()),
                FlowSignal::Continue => {
                    return Err("continue escaped the top-level routine".to_string());
                }
            };
            let final_args = routine
                .params
                .iter()
                .map(|param| {
                    final_scope
                        .locals
                        .get(&param.name)
                        .map(|local| local.value.clone())
                        .ok_or_else(|| {
                            format!(
                                "runtime routine `{}` lost bound parameter `{}`",
                                routine.symbol_name, param.name
                            )
                        })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(RoutineExecutionOutcome { value, final_args })
        })();
        if entered_async_context {
            state.async_context_depth = state.async_context_depth.saturating_sub(1);
        }
        outcome
    })();
    pop_runtime_call_frame(state);
    execution_result
}

fn execute_routine_with_state(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    type_args: Vec<String>,
    args: Vec<RuntimeValue>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    Ok(
        execute_routine_call_with_state(plan, routine_index, type_args, args, state, host, false)?
            .value,
    )
}

pub fn validate_runtime_requirements_supported(
    plan: &RuntimePackagePlan,
    host: &mut dyn RuntimeHost,
) -> Result<(), String> {
    for requirement in &plan.runtime_requirements {
        if !host.supports_runtime_requirement(requirement) {
            return Err(format!(
                "runtime host does not support required capability `{requirement}`"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn execute_routine(
    plan: &RuntimePackagePlan,
    routine_index: usize,
    args: Vec<RuntimeValue>,
    host: &mut dyn RuntimeHost,
) -> Result<RuntimeValue, String> {
    validate_runtime_requirements_supported(plan, host)?;
    let mut state = RuntimeExecutionState::default();
    execute_routine_with_state(plan, routine_index, Vec::new(), args, &mut state, host)
}

pub fn execute_main(plan: &RuntimePackagePlan, host: &mut dyn RuntimeHost) -> Result<i32, String> {
    validate_runtime_requirements_supported(plan, host)?;
    let entry = plan
        .main_entrypoint()
        .ok_or_else(|| format!("package `{}` has no main entrypoint", plan.package_name))?;
    let routine_key = plan
        .routines
        .get(entry.routine_index)
        .map(|routine| routine.routine_key.clone())
        .ok_or_else(|| format!("invalid routine index `{}`", entry.routine_index))?;
    execute_entrypoint_routine(plan, &routine_key, host)
}

pub fn execute_entrypoint_routine(
    plan: &RuntimePackagePlan,
    routine_key: &str,
    host: &mut dyn RuntimeHost,
) -> Result<i32, String> {
    validate_runtime_requirements_supported(plan, host)?;
    let entry = plan
        .entrypoints
        .iter()
        .find(|entry| {
            plan.routines
                .get(entry.routine_index)
                .is_some_and(|routine| routine.routine_key == routine_key)
        })
        .ok_or_else(|| {
            format!(
                "entrypoint routine `{routine_key}` is not present in package `{}`",
                plan.package_name
            )
        })?;
    let routine = plan
        .routines
        .get(entry.routine_index)
        .ok_or_else(|| format!("invalid routine index `{}`", entry.routine_index))?;
    validate_runtime_main_entry_contract(
        routine.params.len(),
        runtime_main_return_type_from_signature(&routine.signature_row),
    )?;
    let mut state = RuntimeExecutionState::default();
    if state.next_scheduler_thread_id <= 0 {
        state.next_scheduler_thread_id = 1;
    }
    state.current_thread_id = 0;
    let value = execute_routine_call_with_state(
        plan,
        entry.routine_index,
        Vec::new(),
        Vec::new(),
        &mut state,
        host,
        true,
    )?
    .value;
    match value {
        RuntimeValue::Int(value) => i32::try_from(value)
            .map_err(|_| format!("main return value `{value}` does not fit in i32")),
        RuntimeValue::Unit => Ok(0),
        RuntimeValue::Bool(_)
        | RuntimeValue::Str(_)
        | RuntimeValue::Pair(_, _)
        | RuntimeValue::Array(_)
        | RuntimeValue::List(_)
        | RuntimeValue::Map(_)
        | RuntimeValue::Range { .. }
        | RuntimeValue::OwnerHandle(_)
        | RuntimeValue::Ref(_)
        | RuntimeValue::Opaque(_)
        | RuntimeValue::Record { .. }
        | RuntimeValue::Variant { .. } => {
            Err("main must return Int or Unit in the current runtime lane".to_string())
        }
    }
}

pub fn current_process_runtime_host() -> Result<Box<dyn RuntimeHost>, String> {
    #[cfg(windows)]
    {
        return Ok(Box::new(NativeProcessHost::current()?));
    }

    #[cfg(not(windows))]
    {
        let mut host = BufferedHost::default();
        host.args = std::env::args().skip(1).collect();
        host.env = std::env::vars().collect();
        host.allow_process = true;
        host.cwd = std::env::current_dir()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(Box::new(host))
    }
}

pub(crate) fn ensure_audio_buffer_matches_device(
    device_sample_rate_hz: i64,
    device_channels: i64,
    buffer_sample_rate_hz: i64,
    buffer_channels: i64,
) -> Result<(), String> {
    if device_sample_rate_hz == buffer_sample_rate_hz && device_channels == buffer_channels {
        return Ok(());
    }
    Err(format!(
        "AudioBuffer format {buffer_sample_rate_hz} Hz / {buffer_channels} channel(s) does not match AudioDevice format {device_sample_rate_hz} Hz / {device_channels} channel(s)"
    ))
}

#[cfg(test)]
mod test_parse;
#[cfg(test)]
pub(crate) use test_parse::{parse_rollup_row, parse_stmt};
#[cfg(test)]
mod tests;
