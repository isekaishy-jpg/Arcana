#![allow(clippy::large_enum_variant)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::c_void;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use arcana_aot::{
    AotOwnerArtifact, AotPackageArtifact, parse_package_artifact, validate_package_artifact,
};
use arcana_cabi::{
    ARCANA_CABI_VIEW_FLAG_UTF8, ArcanaCabiBindingLayout, ArcanaCabiBindingLayoutKind,
    ArcanaCabiBindingParam, ArcanaCabiBindingPayloadV1, ArcanaCabiBindingRawType,
    ArcanaCabiBindingScalarType, ArcanaCabiBindingSignature, ArcanaCabiBindingType,
    ArcanaCabiBindingValueTag, ArcanaCabiBindingValueV1, ArcanaCabiBindingViewType,
    ArcanaCabiOwnedBytesFreeFn, ArcanaCabiOwnedStrFreeFn, ArcanaCabiParamSourceMode,
    ArcanaCabiViewFamily, ArcanaViewV1, binding_write_back_slots, contiguous_u8_view,
    into_owned_bytes, into_owned_str, raw_view, read_binding_input_bytes_arg,
    read_binding_input_layout_bytes_arg, read_binding_input_utf8_arg, read_binding_input_utf16_arg,
    read_binding_input_view_bytes_arg, read_binding_output_bytes_arg,
    read_binding_output_layout_bytes_arg, read_binding_output_utf8_arg,
    read_binding_output_utf16_arg, read_binding_output_view_arg, release_binding_output_value,
    validate_binding_transport_type,
};

use arcana_ir::{
    ExecArrayRegion as ParsedArrayRegion, ExecAssignOp as ParsedAssignOp,
    ExecAssignTarget as ParsedAssignTarget,
    ExecAvailabilityAttachment as ParsedAvailabilityAttachment,
    ExecAvailabilityKind as ParsedAvailabilityKind, ExecBinaryOp as ParsedBinaryOp,
    ExecBindLineKind as ParsedBindLineKind, ExecChainConnector as ParsedChainConnector,
    ExecChainIntroducer as ParsedChainIntroducer, ExecChainStep as ParsedChainStep,
    ExecCleanupFooter as ParsedCleanupFooter,
    ExecConstructContributionMode as ParsedConstructContributionMode,
    ExecConstructDestination as ParsedConstructDestination,
    ExecConstructLine as ParsedConstructLine, ExecDeferAction as ParsedDeferAction,
    ExecDynamicDispatch as ParsedDynamicDispatch, ExecExpr as ParsedExpr,
    ExecFloatKind as ParsedFloatKind, ExecHeadedModifier as ParsedHeadedModifier,
    ExecHeaderAttachment as ParsedHeaderAttachment, ExecMatchArm as ParsedMatchArm,
    ExecMatchPattern as ParsedMatchPattern, ExecMemorySpecDecl as ParsedMemorySpecDecl,
    ExecNamedBindingId as ParsedNamedBindingId, ExecPhraseArg as ParsedPhraseArg,
    ExecPhraseQualifierKind as ParsedPhraseQualifierKind,
    ExecProjectionFamily as ParsedProjectionFamily, ExecRecordRegion as ParsedRecordRegion,
    ExecRecycleLineKind as ParsedRecycleLineKind, ExecStmt as ParsedStmt,
    ExecStructBitfieldFieldLayout, ExecStructBitfieldLayout, ExecUnaryOp as ParsedUnaryOp,
    IrRoutineType, IrRoutineTypeKind, parse_memory_spec_surface_row, parse_routine_type_text,
    parse_struct_bitfield_layout_row, validate_runtime_main_entry_contract,
};
use arcana_syntax::{
    MemoryDetailKey, MemoryDetailValueKind, MemoryFamily, memory_detail_descriptor,
    memory_family_descriptor,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use stacker::grow;

#[cfg(test)]
mod app_input;
mod binding_transport;
mod core_intrinsics;
mod evaluator;
mod host_core_policy;
mod intrinsic_resolution;
mod json_abi;
mod native_abi;
mod native_product_loader;
mod package_image;
mod process_runtime_host;
mod routine_plan;
mod runtime_intrinsics;
mod runtime_types;
mod view_runtime;
#[cfg(test)]
pub use app_input::BufferedHost;
pub use arcana_ir::{
    IrForewordArg, IrForewordEntryKind, IrForewordGeneratedBy, IrForewordMetadata,
    IrForewordRegistrationRow, IrForewordRetention,
};
use binding_transport::*;
use core_intrinsics::*;
use evaluator::*;
pub use evaluator::{execute_entrypoint_routine, execute_main};
pub(crate) use host_core_policy::{HostCoreFsPolicy, HostCoreStreamState};
use intrinsic_resolution::*;
pub use json_abi::{
    RUNTIME_JSON_ABI_FORMAT, execute_exported_json_abi_routine, render_exported_json_abi_manifest,
};
pub use native_abi::{
    RuntimeAbiExportOutcome, RuntimeAbiValue, RuntimeAbiWriteBack,
    execute_cleanup_runtime_abi_routine, execute_exported_abi_routine,
};
use native_product_loader::{RuntimeBindingCallbackRegistrationSpec, RuntimeBindingImportOutcome};
pub use native_product_loader::{
    RuntimeChildBindingInfo, RuntimeNativePluginHandle, RuntimeNativeProductCatalog,
    RuntimeNativeProductInfo, activate_current_bundle_native_products, load_bundle_native_products,
    load_bundle_native_products_from_manifest_path, load_current_bundle_native_products,
};
pub use package_image::{
    RUNTIME_PACKAGE_IMAGE_FORMAT, parse_runtime_package_image, render_runtime_package_image,
};
use process_runtime_host::ProcessRuntimeHost;
pub use process_runtime_host::ProcessRuntimeHostConfig;
pub use routine_plan::{
    RuntimeEntrypointPlan, RuntimeNativeCallbackPlan, RuntimeParamPlan, RuntimeRoutinePlan,
};
use routine_plan::{lower_entrypoint, lower_native_callback, lower_routine};
use runtime_intrinsics::*;
use runtime_types::*;
use view_runtime::*;

const MODULE_MEMORY_SPEC_ALIAS_PREFIX: &str = "@memory_spec:";
const MODULE_BITFIELD_LAYOUT_SCOPE: &str = "@meta.bitfield";
const RUNTIME_STRUCT_BITFIELD_STORAGE_PREFIX: &str = "__arcana_bitfield_storage_";
pub const ARCANA_NATIVE_BUNDLE_DIR_ENV: &str = "ARCANA_NATIVE_BUNDLE_DIR";
pub const ARCANA_NATIVE_BUNDLE_MANIFEST_ENV: &str = "ARCANA_NATIVE_BUNDLE_MANIFEST";

thread_local! {
    static ACTIVE_RUNTIME_NATIVE_PRODUCTS: RefCell<Option<RuntimeNativeProductCatalog>> = const { RefCell::new(None) };
    static ACTIVE_RUNTIME_BINDING_CALLBACK_CONTEXT: RefCell<Option<RuntimeBindingCallbackContext>> = const { RefCell::new(None) };
}

pub fn current_process_core_host() -> Result<Box<dyn RuntimeCoreHost>, String> {
    current_process_core_host_with_config(ProcessRuntimeHostConfig::from_current_process()?)
}

pub fn current_process_core_host_with_config(
    config: ProcessRuntimeHostConfig,
) -> Result<Box<dyn RuntimeCoreHost>, String> {
    Ok(Box::new(ProcessRuntimeHost::from_config(config)))
}

pub fn execute_current_bundle_entrypoint(
    package_image_text: &str,
    routine_key: &str,
) -> Result<i32, String> {
    let mut native_products = activate_current_bundle_native_products()?;
    if let Some(code) = native_products.run_child_entrypoint(package_image_text, routine_key)? {
        return Ok(code);
    }
    let plan = parse_runtime_package_image(package_image_text)?;
    let mut host = current_process_core_host()?;
    execute_entrypoint_routine(&plan, routine_key, host.as_mut())
}

#[derive(Clone, Copy)]
struct RuntimeBindingCallbackContext {
    plan: *const RuntimePackagePlan,
    host_data: *mut (),
    host_vtable: *mut (),
}

#[derive(Clone)]
struct RuntimeBindingCallbackThunkData {
    callback: RuntimeNativeCallbackPlan,
}

#[derive(Default)]
struct RuntimeBindingArgStorage {
    strings: Vec<String>,
    bytes: Vec<Vec<u8>>,
    in_place_edits: BTreeMap<usize, RuntimeBindingInPlaceEdit>,
}

#[derive(Clone, Debug)]
enum RuntimeBindingInPlaceEdit {
    ByteBuffer {
        bytes_index: usize,
    },
    Utf16Buffer {
        bytes_index: usize,
    },
    View {
        original: RuntimeValue,
        bytes_index: usize,
        expected_type: String,
    },
}

struct RuntimeBindingCallbackOutcome {
    result: ArcanaCabiBindingValueV1,
    write_backs: Vec<ArcanaCabiBindingValueV1>,
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
    pub retains: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeOwnerPlan {
    pub package_id: String,
    pub module_id: String,
    pub owner_path: Vec<String>,
    pub owner_name: String,
    pub context_type: Option<String>,
    pub objects: Vec<RuntimeOwnerObjectPlan>,
    pub exits: Vec<RuntimeOwnerExitPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePackagePlan {
    pub package_id: String,
    pub package_name: String,
    pub root_module_id: String,
    pub direct_deps: Vec<String>,
    pub direct_dep_ids: Vec<String>,
    pub package_display_names: BTreeMap<String, String>,
    pub package_direct_dep_ids: BTreeMap<String, BTreeMap<String, String>>,
    pub runtime_requirements: Vec<String>,
    #[serde(default)]
    pub foreword_index: Vec<IrForewordMetadata>,
    #[serde(default)]
    pub foreword_registrations: Vec<IrForewordRegistrationRow>,
    pub module_aliases: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub opaque_family_types: BTreeMap<String, Vec<String>>,
    pub entrypoints: Vec<RuntimeEntrypointPlan>,
    pub routines: Vec<RuntimeRoutinePlan>,
    #[serde(default)]
    pub native_callbacks: Vec<RuntimeNativeCallbackPlan>,
    #[serde(default)]
    pub shackle_decls: Vec<String>,
    #[serde(default)]
    pub binding_layouts: Vec<ArcanaCabiBindingLayout>,
    pub owners: Vec<RuntimeOwnerPlan>,
}

const RETIRED_BINDING_OPAQUE_LANG_ITEMS: &[&str] = &[
    "file_stream_handle",
    "window_handle",
    "app_frame_handle",
    "wake_handle",
    "audio_device_handle",
    "audio_buffer_handle",
    "audio_playback_handle",
];

impl RuntimePackagePlan {
    pub fn main_entrypoint(&self) -> Option<&RuntimeEntrypointPlan> {
        self.entrypoints
            .iter()
            .find(|entry| entry.symbol_kind == "fn" && entry.symbol_name == "main")
    }

    pub fn forewords(&self) -> &[IrForewordMetadata] {
        &self.foreword_index
    }

    pub fn foreword_registrations(&self) -> &[IrForewordRegistrationRow] {
        &self.foreword_registrations
    }

    pub fn public_foreword_registrations(&self) -> Vec<&IrForewordRegistrationRow> {
        self.foreword_registrations
            .iter()
            .filter(|row| row.public)
            .collect()
    }

    pub fn foreword_registrations_for_target(
        &self,
        target_kind: &str,
        target_path: &str,
    ) -> Vec<&IrForewordRegistrationRow> {
        self.foreword_registrations
            .iter()
            .filter(|row| row.target_kind == target_kind && row.target_path == target_path)
            .collect()
    }

    pub fn public_foreword_registrations_for_target(
        &self,
        target_kind: &str,
        target_path: &str,
    ) -> Vec<&IrForewordRegistrationRow> {
        self.foreword_registrations
            .iter()
            .filter(|row| {
                row.public && row.target_kind == target_kind && row.target_path == target_path
            })
            .collect()
    }

    pub fn runtime_retained_forewords(&self) -> Vec<&IrForewordMetadata> {
        self.foreword_index
            .iter()
            .filter(|entry| entry.retention == IrForewordRetention::Runtime)
            .collect()
    }

    pub fn public_runtime_retained_forewords(&self) -> Vec<&IrForewordMetadata> {
        self.foreword_index
            .iter()
            .filter(|entry| entry.public && entry.retention == IrForewordRetention::Runtime)
            .collect()
    }

    pub fn runtime_retained_forewords_for_package(
        &self,
        package_id: &str,
    ) -> Vec<&IrForewordMetadata> {
        self.foreword_index
            .iter()
            .filter(|entry| {
                entry.package_id == package_id && entry.retention == IrForewordRetention::Runtime
            })
            .collect()
    }

    pub fn public_runtime_retained_forewords_for_package(
        &self,
        package_id: &str,
    ) -> Vec<&IrForewordMetadata> {
        self.foreword_index
            .iter()
            .filter(|entry| {
                entry.public
                    && entry.package_id == package_id
                    && entry.retention == IrForewordRetention::Runtime
            })
            .collect()
    }

    pub fn runtime_retained_forewords_for_target(
        &self,
        target_kind: &str,
        target_path: &str,
    ) -> Vec<&IrForewordMetadata> {
        self.foreword_index
            .iter()
            .filter(|entry| {
                entry.retention == IrForewordRetention::Runtime
                    && entry.target_kind == target_kind
                    && entry.target_path == target_path
            })
            .collect()
    }

    pub fn public_runtime_retained_forewords_for_target(
        &self,
        target_kind: &str,
        target_path: &str,
    ) -> Vec<&IrForewordMetadata> {
        self.foreword_index
            .iter()
            .filter(|entry| {
                entry.public
                    && entry.retention == IrForewordRetention::Runtime
                    && entry.target_kind == target_kind
                    && entry.target_path == target_path
            })
            .collect()
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

fn runtime_relative_path(path: &Path, base: &Path) -> Result<PathBuf, String> {
    let path = normalize_lexical_path(path);
    let base = normalize_lexical_path(base);
    let path_parts = path.components().collect::<Vec<_>>();
    let base_parts = base.components().collect::<Vec<_>>();
    let mut shared = 0usize;
    while shared < path_parts.len()
        && shared < base_parts.len()
        && path_parts[shared] == base_parts[shared]
    {
        shared += 1;
    }
    if shared == 0
        && path_parts
            .first()
            .is_some_and(|part| matches!(part, Component::Prefix(_)))
        && base_parts
            .first()
            .is_some_and(|part| matches!(part, Component::Prefix(_)))
    {
        return Err(format!(
            "failed to make `{}` relative to `{}`",
            runtime_path_string(&path),
            runtime_path_string(&base)
        ));
    }
    let mut relative = PathBuf::new();
    for _ in shared..base_parts.len() {
        relative.push("..");
    }
    for component in path_parts.iter().skip(shared) {
        relative.push(component.as_os_str());
    }
    Ok(relative)
}

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
pub struct RuntimeTempArenaHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeSessionArenaHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeRingBufferHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeSlabHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeReadViewHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeEditViewHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeByteViewHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeByteEditViewHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeStrViewHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeElementViewBufferHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeByteViewBufferHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeStrViewBufferHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeTaskHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeThreadHandle(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeLazyHandle(u64);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeTempIdValue {
    arena: RuntimeTempArenaHandle,
    slot: u64,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSessionIdValue {
    arena: RuntimeSessionArenaHandle,
    slot: u64,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeRingIdValue {
    arena: RuntimeRingBufferHandle,
    slot: u64,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSlabIdValue {
    arena: RuntimeSlabHandle,
    slot: u64,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeReferenceMode {
    Read,
    Edit,
    Take,
    Hold,
}

impl RuntimeReferenceMode {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Take => "take",
            Self::Hold => "hold",
        }
    }

    const fn allows_write(&self) -> bool {
        matches!(self, Self::Edit | Self::Hold)
    }
}

const fn runtime_reference_mode_for_place(mutable: bool) -> RuntimeReferenceMode {
    if mutable {
        RuntimeReferenceMode::Edit
    } else {
        RuntimeReferenceMode::Read
    }
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
    TempSlot {
        id: RuntimeTempIdValue,
        members: Vec<String>,
    },
    SessionSlot {
        id: RuntimeSessionIdValue,
        members: Vec<String>,
    },
    RingSlot {
        id: RuntimeRingIdValue,
        members: Vec<String>,
    },
    SlabSlot {
        id: RuntimeSlabIdValue,
        members: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeReferenceValue {
    mode: RuntimeReferenceMode,
    target: RuntimeReferenceTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeResolvedPlace {
    mode: RuntimeReferenceMode,
    target: RuntimeReferenceTarget,
}

fn runtime_reference_mode_from_unary(op: ParsedUnaryOp) -> Option<RuntimeReferenceMode> {
    match op {
        ParsedUnaryOp::CapabilityRead => Some(RuntimeReferenceMode::Read),
        ParsedUnaryOp::CapabilityEdit => Some(RuntimeReferenceMode::Edit),
        ParsedUnaryOp::CapabilityTake => Some(RuntimeReferenceMode::Take),
        ParsedUnaryOp::CapabilityHold => Some(RuntimeReferenceMode::Hold),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeBindingOpaqueValue {
    package_id: &'static str,
    type_name: &'static str,
    handle: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeOpaqueValue {
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
    TempArena(RuntimeTempArenaHandle),
    TempId(RuntimeTempIdValue),
    SessionArena(RuntimeSessionArenaHandle),
    SessionId(RuntimeSessionIdValue),
    RingBuffer(RuntimeRingBufferHandle),
    RingId(RuntimeRingIdValue),
    Slab(RuntimeSlabHandle),
    SlabId(RuntimeSlabIdValue),
    ReadView(RuntimeReadViewHandle),
    EditView(RuntimeEditViewHandle),
    ByteView(RuntimeByteViewHandle),
    ByteEditView(RuntimeByteEditViewHandle),
    StrView(RuntimeStrViewHandle),
    Task(RuntimeTaskHandle),
    Thread(RuntimeThreadHandle),
    Binding(RuntimeBindingOpaqueValue),
    Lazy(RuntimeLazyHandle),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeOpaqueFamily {
    Channel,
    Mutex,
    AtomicInt,
    AtomicBool,
    Arena,
    ArenaId,
    FrameArena,
    FrameId,
    PoolArena,
    PoolId,
    TempArena,
    TempId,
    SessionArena,
    SessionId,
    RingBuffer,
    RingId,
    Slab,
    SlabId,
    ReadView,
    EditView,
    ByteView,
    ByteEditView,
    StrView,
    Task,
    Thread,
}

fn retired_binding_opaque_lang_item(name: &str) -> bool {
    RETIRED_BINDING_OPAQUE_LANG_ITEMS.contains(&name)
}

fn tracked_opaque_lang_item(name: &str) -> bool {
    RuntimeOpaqueFamily::from_lang_item_name(name).is_some()
        || retired_binding_opaque_lang_item(name)
}

pub(crate) fn retired_binding_opaque_type_matches(
    plan: &RuntimePackagePlan,
    rendered: &str,
) -> bool {
    RETIRED_BINDING_OPAQUE_LANG_ITEMS.iter().any(|name| {
        plan.opaque_family_types
            .get(*name)
            .is_some_and(|entries| entries.iter().any(|entry| entry == rendered))
    })
}

impl RuntimeOpaqueFamily {
    const ALL: [Self; 25] = [
        Self::Channel,
        Self::Mutex,
        Self::AtomicInt,
        Self::AtomicBool,
        Self::Arena,
        Self::ArenaId,
        Self::FrameArena,
        Self::FrameId,
        Self::PoolArena,
        Self::PoolId,
        Self::TempArena,
        Self::TempId,
        Self::SessionArena,
        Self::SessionId,
        Self::RingBuffer,
        Self::RingId,
        Self::Slab,
        Self::SlabId,
        Self::ReadView,
        Self::EditView,
        Self::ByteView,
        Self::ByteEditView,
        Self::StrView,
        Self::Task,
        Self::Thread,
    ];

    const fn lang_item_name(self) -> &'static str {
        match self {
            Self::Channel => "channel_handle",
            Self::Mutex => "mutex_handle",
            Self::AtomicInt => "atomic_int_handle",
            Self::AtomicBool => "atomic_bool_handle",
            Self::Arena => "arena_handle",
            Self::ArenaId => "arena_id_handle",
            Self::FrameArena => "frame_arena_handle",
            Self::FrameId => "frame_id_handle",
            Self::PoolArena => "pool_arena_handle",
            Self::PoolId => "pool_id_handle",
            Self::TempArena => "temp_arena_handle",
            Self::TempId => "temp_id_handle",
            Self::SessionArena => "session_arena_handle",
            Self::SessionId => "session_id_handle",
            Self::RingBuffer => "ring_buffer_handle",
            Self::RingId => "ring_id_handle",
            Self::Slab => "slab_handle",
            Self::SlabId => "slab_id_handle",
            Self::ReadView => "contiguous_view_handle",
            Self::EditView => "contiguous_view_edit_handle",
            Self::ByteView => "u8_view_handle",
            Self::ByteEditView => "u8_view_edit_handle",
            Self::StrView => "text_bytes_view_handle",
            Self::Task => "task_handle",
            Self::Thread => "thread_handle",
        }
    }

    const fn canonical_type_name(self) -> &'static str {
        match self {
            Self::Channel => "std.concurrent.Channel",
            Self::Mutex => "std.concurrent.Mutex",
            Self::AtomicInt => "std.concurrent.AtomicInt",
            Self::AtomicBool => "std.concurrent.AtomicBool",
            Self::Arena => "std.memory.Arena",
            Self::ArenaId => "std.memory.ArenaId",
            Self::FrameArena => "std.memory.FrameArena",
            Self::FrameId => "std.memory.FrameId",
            Self::PoolArena => "std.memory.PoolArena",
            Self::PoolId => "std.memory.PoolId",
            Self::TempArena => "std.memory.TempArena",
            Self::TempId => "std.memory.TempId",
            Self::SessionArena => "std.memory.SessionArena",
            Self::SessionId => "std.memory.SessionId",
            Self::RingBuffer => "std.memory.RingBuffer",
            Self::RingId => "std.memory.RingId",
            Self::Slab => "std.memory.Slab",
            Self::SlabId => "std.memory.SlabId",
            Self::ReadView => "View",
            Self::EditView => "View",
            Self::ByteView => "View",
            Self::ByteEditView => "View",
            Self::StrView => "View",
            Self::Task => "std.concurrent.Task",
            Self::Thread => "std.concurrent.Thread",
        }
    }

    fn from_lang_item_name(name: &str) -> Option<Self> {
        match name {
            "channel_handle" => Some(Self::Channel),
            "mutex_handle" => Some(Self::Mutex),
            "atomic_int_handle" => Some(Self::AtomicInt),
            "atomic_bool_handle" => Some(Self::AtomicBool),
            "arena_handle" => Some(Self::Arena),
            "arena_id_handle" => Some(Self::ArenaId),
            "frame_arena_handle" => Some(Self::FrameArena),
            "frame_id_handle" => Some(Self::FrameId),
            "pool_arena_handle" => Some(Self::PoolArena),
            "pool_id_handle" => Some(Self::PoolId),
            "temp_arena_handle" => Some(Self::TempArena),
            "temp_id_handle" => Some(Self::TempId),
            "session_arena_handle" => Some(Self::SessionArena),
            "session_id_handle" => Some(Self::SessionId),
            "ring_buffer_handle" => Some(Self::RingBuffer),
            "ring_id_handle" => Some(Self::RingId),
            "slab_handle" => Some(Self::Slab),
            "slab_id_handle" => Some(Self::SlabId),
            "contiguous_view_handle" => Some(Self::ReadView),
            "contiguous_view_edit_handle" => Some(Self::EditView),
            "u8_view_handle" => Some(Self::ByteView),
            "u8_view_edit_handle" => Some(Self::ByteEditView),
            "text_bytes_view_handle" => Some(Self::StrView),
            "task_handle" => Some(Self::Task),
            "thread_handle" => Some(Self::Thread),
            _ => None,
        }
    }

    const fn from_opaque_value(value: &RuntimeOpaqueValue) -> Self {
        match value {
            RuntimeOpaqueValue::Channel(_) => Self::Channel,
            RuntimeOpaqueValue::Mutex(_) => Self::Mutex,
            RuntimeOpaqueValue::AtomicInt(_) => Self::AtomicInt,
            RuntimeOpaqueValue::AtomicBool(_) => Self::AtomicBool,
            RuntimeOpaqueValue::Arena(_) => Self::Arena,
            RuntimeOpaqueValue::ArenaId(_) => Self::ArenaId,
            RuntimeOpaqueValue::FrameArena(_) => Self::FrameArena,
            RuntimeOpaqueValue::FrameId(_) => Self::FrameId,
            RuntimeOpaqueValue::PoolArena(_) => Self::PoolArena,
            RuntimeOpaqueValue::PoolId(_) => Self::PoolId,
            RuntimeOpaqueValue::TempArena(_) => Self::TempArena,
            RuntimeOpaqueValue::TempId(_) => Self::TempId,
            RuntimeOpaqueValue::SessionArena(_) => Self::SessionArena,
            RuntimeOpaqueValue::SessionId(_) => Self::SessionId,
            RuntimeOpaqueValue::RingBuffer(_) => Self::RingBuffer,
            RuntimeOpaqueValue::RingId(_) => Self::RingId,
            RuntimeOpaqueValue::Slab(_) => Self::Slab,
            RuntimeOpaqueValue::SlabId(_) => Self::SlabId,
            RuntimeOpaqueValue::ReadView(_) => Self::ReadView,
            RuntimeOpaqueValue::EditView(_) => Self::EditView,
            RuntimeOpaqueValue::ByteView(_) => Self::ByteView,
            RuntimeOpaqueValue::ByteEditView(_) => Self::ByteEditView,
            RuntimeOpaqueValue::StrView(_) => Self::StrView,
            RuntimeOpaqueValue::Task(_) => Self::Task,
            RuntimeOpaqueValue::Thread(_) => Self::Thread,
            RuntimeOpaqueValue::Binding(_) => panic!("binding opaques are package-defined"),
            RuntimeOpaqueValue::Lazy(_) => panic!("lazy opaques are internal"),
        }
    }
}

pub trait RuntimeCoreHost {
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
        Err("runtime core host stdin_read_line is not implemented".to_string())
    }
    fn monotonic_now_ms(&mut self) -> Result<i64, String> {
        Err("runtime core host monotonic_now_ms is not implemented".to_string())
    }
    fn monotonic_now_ns(&mut self) -> Result<i64, String> {
        Err("runtime core host monotonic_now_ns is not implemented".to_string())
    }
    fn sleep_ms(&mut self, ms: i64) -> Result<(), String> {
        let _ = ms;
        Err("runtime core host sleep_ms is not implemented".to_string())
    }
    fn allows_process_execution(&self) -> bool {
        false
    }
    fn runtime_arg_count(&self) -> Result<i64, String> {
        Ok(std::env::args().skip(1).count() as i64)
    }
    fn runtime_arg_get(&self, index: i64) -> Result<String, String> {
        if index < 0 {
            return Err("arg_get index must be non-negative".to_string());
        }
        Ok(std::env::args()
            .skip(1)
            .nth(index as usize)
            .unwrap_or_default())
    }
    fn runtime_env_has(&self, name: &str) -> Result<bool, String> {
        Ok(std::env::var_os(name).is_some())
    }
    fn runtime_env_get(&self, name: &str) -> Result<String, String> {
        Ok(std::env::var(name).unwrap_or_default())
    }
    fn runtime_current_working_dir(&self) -> Result<PathBuf, String> {
        std::env::current_dir()
            .map(|path| normalize_lexical_path(&path))
            .map_err(|err| format!("failed to resolve current directory: {err}"))
    }
    fn runtime_resolve_fs_path(&self, path: &str) -> Result<PathBuf, String> {
        let _ = path;
        Err("runtime core host fs path resolution is not implemented".to_string())
    }
    fn runtime_path_canonicalize(&self, path: &str) -> Result<String, String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::canonicalize(&resolved)
            .map(|real| runtime_path_string(&normalize_lexical_path(&real)))
            .map_err(|err| format!("failed to canonicalize `{path}`: {err}"))
    }
    fn runtime_fs_exists(&self, path: &str) -> Result<bool, String> {
        Ok(self.runtime_resolve_fs_path(path)?.exists())
    }
    fn runtime_fs_is_file(&self, path: &str) -> Result<bool, String> {
        Ok(self.runtime_resolve_fs_path(path)?.is_file())
    }
    fn runtime_fs_is_dir(&self, path: &str) -> Result<bool, String> {
        Ok(self.runtime_resolve_fs_path(path)?.is_dir())
    }
    fn runtime_fs_read_text(&self, path: &str) -> Result<String, String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::read_to_string(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    }
    fn runtime_fs_read_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::read(&resolved)
            .map_err(|err| format!("failed to read `{}`: {err}", runtime_path_string(&resolved)))
    }
    fn runtime_fs_write_text(&self, path: &str, text: &str) -> Result<(), String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
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
    fn runtime_fs_write_bytes(&self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
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
    fn runtime_fs_list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
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
    fn runtime_fs_mkdir_all(&self, path: &str) -> Result<(), String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::create_dir_all(&resolved).map_err(|err| {
            format!(
                "failed to create `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }
    fn runtime_fs_create_dir(&self, path: &str) -> Result<(), String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::create_dir(&resolved).map_err(|err| {
            format!(
                "failed to create directory `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }
    fn runtime_fs_remove_file(&self, path: &str) -> Result<(), String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::remove_file(&resolved).map_err(|err| {
            format!(
                "failed to remove file `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }
    fn runtime_fs_remove_dir(&self, path: &str) -> Result<(), String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::remove_dir(&resolved).map_err(|err| {
            format!(
                "failed to remove directory `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }
    fn runtime_fs_remove_dir_all(&self, path: &str) -> Result<(), String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        fs::remove_dir_all(&resolved).map_err(|err| {
            format!(
                "failed to remove directory tree `{}`: {err}",
                runtime_path_string(&resolved)
            )
        })
    }
    fn runtime_fs_copy_file(&self, from: &str, to: &str) -> Result<(), String> {
        let from_resolved = self.runtime_resolve_fs_path(from)?;
        let to_resolved = self.runtime_resolve_fs_path(to)?;
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
    fn runtime_fs_rename(&self, from: &str, to: &str) -> Result<(), String> {
        let from_resolved = self.runtime_resolve_fs_path(from)?;
        let to_resolved = self.runtime_resolve_fs_path(to)?;
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
    fn runtime_fs_file_size(&self, path: &str) -> Result<i64, String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        let metadata = fs::metadata(&resolved)
            .map_err(|err| format!("failed to stat `{}`: {err}", runtime_path_string(&resolved)))?;
        i64::try_from(metadata.len())
            .map_err(|_| format!("file size for `{path}` does not fit in i64"))
    }
    fn runtime_fs_modified_unix_ms(&self, path: &str) -> Result<i64, String> {
        let resolved = self.runtime_resolve_fs_path(path)?;
        let metadata = fs::metadata(&resolved)
            .map_err(|err| format!("failed to stat `{}`: {err}", runtime_path_string(&resolved)))?;
        let modified = metadata.modified().map_err(|err| {
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
    fn runtime_fs_stream_open_read(&mut self, path: &str) -> Result<u64, String> {
        let _ = path;
        Err("runtime core host fs stream_open_read is not implemented".to_string())
    }
    fn runtime_fs_stream_open_write(&mut self, path: &str, append: bool) -> Result<u64, String> {
        let _ = (path, append);
        Err("runtime core host fs stream_open_write is not implemented".to_string())
    }
    fn runtime_fs_stream_read(&mut self, handle: u64, max_bytes: usize) -> Result<Vec<u8>, String> {
        let _ = (handle, max_bytes);
        Err("runtime core host fs stream_read is not implemented".to_string())
    }
    fn runtime_fs_stream_write(&mut self, handle: u64, bytes: &[u8]) -> Result<usize, String> {
        let _ = (handle, bytes);
        Err("runtime core host fs stream_write is not implemented".to_string())
    }
    fn runtime_fs_stream_eof(&mut self, handle: u64) -> Result<bool, String> {
        let _ = handle;
        Err("runtime core host fs stream_eof is not implemented".to_string())
    }
    fn runtime_fs_stream_close(&mut self, handle: u64) -> Result<(), String> {
        let _ = handle;
        Err("runtime core host fs stream_close is not implemented".to_string())
    }
    fn runtime_process_exec_status(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<i64, String> {
        let _ = (program, args);
        Err("runtime core host process execution is not implemented".to_string())
    }
    fn runtime_process_exec_capture(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<RuntimeProcessCapture, String> {
        let _ = (program, args);
        Err("runtime core host process execution is not implemented".to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeProcessCapture {
    pub status: i64,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_utf8: bool,
    pub stderr_utf8: bool,
}

fn runtime_memory_strategy_from_name(name: &str) -> Result<RuntimeMemoryStrategy, String> {
    match name {
        "alloc" => Ok(RuntimeMemoryStrategy::Alloc),
        "grow" => Ok(RuntimeMemoryStrategy::Grow),
        "fixed" => Ok(RuntimeMemoryStrategy::Fixed),
        "recycle" => Ok(RuntimeMemoryStrategy::Recycle),
        other => Err(format!("unsupported runtime memory modifier `-{other}`")),
    }
}

fn runtime_memory_strategy_from_modifier(
    modifier: &ParsedHeadedModifier,
) -> Result<RuntimeMemoryStrategy, String> {
    if modifier.payload.is_some() {
        return Err("runtime memory modifiers do not take payload expressions in v1".to_string());
    }
    runtime_memory_strategy_from_name(&modifier.kind)
}

fn runtime_memory_pressure_from_atom(atom: &str) -> Result<RuntimeMemoryPressurePolicy, String> {
    match atom {
        "bounded" => Ok(RuntimeMemoryPressurePolicy::Bounded),
        "elastic" => Ok(RuntimeMemoryPressurePolicy::Elastic),
        other => Err(format!("unsupported memory pressure atom `{other}`")),
    }
}

fn runtime_memory_handle_policy_from_atom(atom: &str) -> Result<RuntimeMemoryHandlePolicy, String> {
    match atom {
        "stable" => Ok(RuntimeMemoryHandlePolicy::Stable),
        "unstable" => Ok(RuntimeMemoryHandlePolicy::Unstable),
        other => Err(format!("unsupported memory handle atom `{other}`")),
    }
}

fn runtime_frame_recycle_policy_from_atom(atom: &str) -> Result<RuntimeFrameRecyclePolicy, String> {
    match atom {
        "manual" => Ok(RuntimeFrameRecyclePolicy::Manual),
        "frame" => Ok(RuntimeFrameRecyclePolicy::Frame),
        other => Err(format!("unsupported frame recycle atom `{other}`")),
    }
}

fn runtime_pool_recycle_policy_from_atom(atom: &str) -> Result<RuntimePoolRecyclePolicy, String> {
    match atom {
        "free_list" => Ok(RuntimePoolRecyclePolicy::FreeList),
        "strict" => Ok(RuntimePoolRecyclePolicy::Strict),
        other => Err(format!("unsupported pool recycle atom `{other}`")),
    }
}

fn runtime_reset_on_policy_from_atom(atom: &str) -> Result<RuntimeResetOnPolicy, String> {
    match atom {
        "manual" => Ok(RuntimeResetOnPolicy::Manual),
        "frame" => Ok(RuntimeResetOnPolicy::Frame),
        "owner_exit" => Ok(RuntimeResetOnPolicy::OwnerExit),
        other => Err(format!("unsupported reset_on atom `{other}`")),
    }
}

fn runtime_ring_overwrite_policy_from_atom(
    atom: &str,
) -> Result<RuntimeRingOverwritePolicy, String> {
    match atom {
        "oldest" => Ok(RuntimeRingOverwritePolicy::Oldest),
        other => Err(format!("unsupported ring overwrite atom `{other}`")),
    }
}

fn runtime_default_memory_pressure(strategy: RuntimeMemoryStrategy) -> RuntimeMemoryPressurePolicy {
    match strategy {
        RuntimeMemoryStrategy::Grow => RuntimeMemoryPressurePolicy::Elastic,
        RuntimeMemoryStrategy::Alloc
        | RuntimeMemoryStrategy::Fixed
        | RuntimeMemoryStrategy::Recycle => RuntimeMemoryPressurePolicy::Bounded,
    }
}

fn runtime_default_memory_handle_policy(
    strategy: RuntimeMemoryStrategy,
) -> RuntimeMemoryHandlePolicy {
    match strategy {
        RuntimeMemoryStrategy::Grow => RuntimeMemoryHandlePolicy::Unstable,
        RuntimeMemoryStrategy::Alloc
        | RuntimeMemoryStrategy::Fixed
        | RuntimeMemoryStrategy::Recycle => RuntimeMemoryHandlePolicy::Stable,
    }
}

fn runtime_default_frame_recycle_policy(
    strategy: RuntimeMemoryStrategy,
) -> RuntimeFrameRecyclePolicy {
    match strategy {
        RuntimeMemoryStrategy::Recycle => RuntimeFrameRecyclePolicy::Frame,
        RuntimeMemoryStrategy::Alloc
        | RuntimeMemoryStrategy::Grow
        | RuntimeMemoryStrategy::Fixed => RuntimeFrameRecyclePolicy::Manual,
    }
}

fn runtime_default_pool_recycle_policy(
    strategy: RuntimeMemoryStrategy,
) -> RuntimePoolRecyclePolicy {
    match strategy {
        RuntimeMemoryStrategy::Recycle => RuntimePoolRecyclePolicy::FreeList,
        RuntimeMemoryStrategy::Alloc
        | RuntimeMemoryStrategy::Grow
        | RuntimeMemoryStrategy::Fixed => RuntimePoolRecyclePolicy::Strict,
    }
}

fn runtime_default_reset_on_policy(strategy: RuntimeMemoryStrategy) -> RuntimeResetOnPolicy {
    match strategy {
        RuntimeMemoryStrategy::Recycle => RuntimeResetOnPolicy::Frame,
        RuntimeMemoryStrategy::Alloc
        | RuntimeMemoryStrategy::Grow
        | RuntimeMemoryStrategy::Fixed => RuntimeResetOnPolicy::Manual,
    }
}

fn runtime_default_growth_step(strategy: RuntimeMemoryStrategy, base_capacity: usize) -> usize {
    match strategy {
        RuntimeMemoryStrategy::Grow => base_capacity.max(1),
        RuntimeMemoryStrategy::Alloc
        | RuntimeMemoryStrategy::Fixed
        | RuntimeMemoryStrategy::Recycle => 0,
    }
}

fn runtime_non_negative_usize(value: i64, context: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("{context} must be non-negative"))
}

fn runtime_try_grow_limit(limit: &mut usize, growth_step: usize) -> bool {
    if growth_step == 0 {
        return false;
    }
    let next = limit.saturating_add(growth_step);
    if next == *limit {
        return false;
    }
    *limit = next;
    true
}

fn runtime_try_grow_limit_by(limit: &mut usize, growth_step: usize) -> bool {
    runtime_try_grow_limit(limit, growth_step.max(1))
}

fn memory_spec_alias_name(name: &str) -> String {
    format!("{MODULE_MEMORY_SPEC_ALIAS_PREFIX}{name}")
}

fn memory_spec_state_key(package_id: &str, module_id: &str, name: &str) -> String {
    format!("{package_id}|{module_id}|{name}")
}

fn runtime_eval_message(signal: RuntimeEvalSignal) -> String {
    match signal {
        RuntimeEvalSignal::Message(message) => message,
        RuntimeEvalSignal::Return(value) => {
            format!("runtime try qualifier `?` returned from unsupported context with `{value:?}`")
        }
        RuntimeEvalSignal::OwnerExit {
            owner_key,
            exit_name,
        } => format!(
            "runtime try qualifier `?` returned from unsupported owner exit `{exit_name}` for `{owner_key}`"
        ),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeExecutionState {
    next_local_handle: u64,
    captured_local_values: BTreeMap<RuntimeLocalHandle, RuntimeValue>,
    call_stack: Vec<String>,
    cleanup_footer_frames: Vec<RuntimeCleanupFooterFrame>,
    owners: BTreeMap<String, RuntimeOwnerState>,
    module_memory_specs: BTreeMap<String, RuntimeMemorySpecState>,
    next_entity_id: i64,
    live_entities: BTreeSet<i64>,
    component_slots: BTreeMap<Vec<String>, BTreeMap<i64, RuntimeValue>>,
    next_arena_handle: u64,
    arenas: BTreeMap<RuntimeArenaHandle, RuntimeArenaState>,
    next_frame_arena_handle: u64,
    frame_arenas: BTreeMap<RuntimeFrameArenaHandle, RuntimeFrameArenaState>,
    next_pool_arena_handle: u64,
    pool_arenas: BTreeMap<RuntimePoolArenaHandle, RuntimePoolArenaState>,
    next_temp_arena_handle: u64,
    temp_arenas: BTreeMap<RuntimeTempArenaHandle, RuntimeTempArenaState>,
    next_session_arena_handle: u64,
    session_arenas: BTreeMap<RuntimeSessionArenaHandle, RuntimeSessionArenaState>,
    next_ring_buffer_handle: u64,
    ring_buffers: BTreeMap<RuntimeRingBufferHandle, RuntimeRingBufferState>,
    next_slab_handle: u64,
    slabs: BTreeMap<RuntimeSlabHandle, RuntimeSlabState>,
    next_element_view_buffer_handle: u64,
    element_view_buffers: BTreeMap<RuntimeElementViewBufferHandle, RuntimeElementViewBufferState>,
    next_contiguous_view_id: u64,
    read_views: BTreeMap<RuntimeReadViewHandle, RuntimeReadViewState>,
    next_contiguous_edit_view_id: u64,
    edit_views: BTreeMap<RuntimeEditViewHandle, RuntimeEditViewState>,
    next_byte_view_buffer_handle: u64,
    byte_view_buffers: BTreeMap<RuntimeByteViewBufferHandle, RuntimeByteViewBufferState>,
    next_u8_view_id: u64,
    byte_views: BTreeMap<RuntimeByteViewHandle, RuntimeByteViewState>,
    next_u8_edit_view_id: u64,
    byte_edit_views: BTreeMap<RuntimeByteEditViewHandle, RuntimeByteEditViewState>,
    next_str_view_buffer_handle: u64,
    str_view_buffers: BTreeMap<RuntimeStrViewBufferHandle, RuntimeStrViewBufferState>,
    next_text_view_id: u64,
    str_views: BTreeMap<RuntimeStrViewHandle, RuntimeStrViewState>,
    next_task_handle: u64,
    tasks: BTreeMap<RuntimeTaskHandle, RuntimeTaskState>,
    next_thread_handle: u64,
    threads: BTreeMap<RuntimeThreadHandle, RuntimeThreadState>,
    next_lazy_handle: u64,
    lazy_values: BTreeMap<RuntimeLazyHandle, RuntimeTaskState>,
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
    exported_descriptor_counts: BTreeMap<RuntimeExportedDescriptorTarget, usize>,
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
    policy: RuntimeArenaPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeFrameArenaState {
    type_args: Vec<String>,
    next_slot: u64,
    generation: u64,
    slots: BTreeMap<u64, RuntimeValue>,
    policy: RuntimeFrameArenaPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimePoolArenaState {
    type_args: Vec<String>,
    next_slot: u64,
    free_slots: Vec<u64>,
    generations: BTreeMap<u64, u64>,
    slots: BTreeMap<u64, RuntimeValue>,
    policy: RuntimePoolArenaPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeTempArenaState {
    type_args: Vec<String>,
    next_slot: u64,
    generation: u64,
    slots: BTreeMap<u64, RuntimeValue>,
    policy: RuntimeTempArenaPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeSessionArenaState {
    type_args: Vec<String>,
    next_slot: u64,
    generation: u64,
    slots: BTreeMap<u64, RuntimeValue>,
    policy: RuntimeSessionArenaPolicy,
    sealed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeRingBufferState {
    type_args: Vec<String>,
    next_slot: u64,
    generations: BTreeMap<u64, u64>,
    slots: BTreeMap<u64, RuntimeValue>,
    order: VecDeque<u64>,
    policy: RuntimeRingBufferPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeSlabState {
    type_args: Vec<String>,
    next_slot: u64,
    free_slots: Vec<u64>,
    generations: BTreeMap<u64, u64>,
    slots: BTreeMap<u64, RuntimeValue>,
    policy: RuntimeSlabPolicy,
    sealed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RuntimeExportedDescriptorTarget {
    SessionArena(RuntimeSessionArenaHandle),
    Slab(RuntimeSlabHandle),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeElementViewBufferState {
    type_args: Vec<String>,
    values: Vec<RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeElementViewBacking {
    Buffer(RuntimeElementViewBufferHandle),
    Reference(RuntimeReferenceValue),
    RingWindow {
        arena: RuntimeRingBufferHandle,
        slots: Vec<u64>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeReadViewState {
    type_args: Vec<String>,
    backing: RuntimeElementViewBacking,
    start: usize,
    len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeEditViewState {
    type_args: Vec<String>,
    backing: RuntimeElementViewBacking,
    start: usize,
    len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeByteViewBufferState {
    values: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeForeignByteViewBacking {
    package_id: &'static str,
    handle: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeByteViewBacking {
    Buffer(RuntimeByteViewBufferHandle),
    Reference(RuntimeReferenceValue),
    Foreign(RuntimeForeignByteViewBacking),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeByteViewState {
    backing: RuntimeByteViewBacking,
    start: usize,
    len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeByteEditViewState {
    backing: RuntimeByteViewBacking,
    start: usize,
    len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeStrViewBufferState {
    text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeStrViewBacking {
    Buffer(RuntimeStrViewBufferHandle),
    Reference(RuntimeReferenceValue),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeStrViewState {
    backing: RuntimeStrViewBacking,
    start: usize,
    len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeCleanupFooterFrame {
    cleanup_footers: Vec<ParsedCleanupFooter>,
    current_package_id: String,
    current_module_id: String,
    owner_call_stack_depth: usize,
    owner_scope_depth: usize,
    activations: Vec<RuntimeTrackedCleanupBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeTrackedCleanupBinding {
    binding_id: u64,
    subject: String,
    binding: RuntimeLocalHandle,
    value: RuntimeValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeDeferredCall {
    callable: Vec<String>,
    resolved_routine: Option<String>,
    dynamic_dispatch: Option<ParsedDynamicDispatch>,
    current_package_id: String,
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
    current_package_id: String,
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

fn lower_owner(owner: &AotOwnerArtifact) -> RuntimeOwnerPlan {
    RuntimeOwnerPlan {
        package_id: if owner.package_id.is_empty() {
            owner.owner_path.first().cloned().unwrap_or_default()
        } else {
            owner.package_id.clone()
        },
        module_id: owner.module_id.clone(),
        owner_path: owner.owner_path.clone(),
        owner_name: owner.owner_name.clone(),
        context_type: owner.context_type.as_ref().map(IrRoutineType::render),
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
                retains: owner_exit.retains.clone(),
            })
            .collect(),
    }
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

fn parse_lang_item_row(row: &str) -> Result<(String, String, Vec<String>), String> {
    let payload = row
        .strip_prefix("module=")
        .ok_or_else(|| format!("malformed lang item row `{row}`"))?;
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() != 4 || parts[1] != "lang" {
        return Err(format!("malformed lang item row `{row}`"));
    }
    Ok((
        parts[0].to_string(),
        parts[2].to_string(),
        parts[3].split('.').map(ToString::to_string).collect(),
    ))
}

fn module_alias_scope_key(package_id: &str, module_id: &str) -> String {
    format!("{package_id}|{module_id}")
}

fn build_module_aliases(
    artifact: &AotPackageArtifact,
) -> Result<BTreeMap<String, BTreeMap<String, Vec<String>>>, String> {
    let mut aliases = BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
    for module in &artifact.modules {
        {
            let module_aliases = aliases
                .entry(module_alias_scope_key(
                    &module.package_id,
                    &module.module_id,
                ))
                .or_default();
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
            for row in &module.exported_surface_rows {
                if let Some(spec) = parse_memory_spec_surface_row(row)? {
                    let encoded = serde_json::to_string(&spec).map_err(|err| {
                        format!(
                            "failed to encode runtime memory spec `{}`: {err}",
                            spec.name
                        )
                    })?;
                    module_aliases.insert(memory_spec_alias_name(&spec.name), vec![encoded]);
                }
            }
        }
        for row in &module.lang_item_rows {
            let Some(layout) = parse_struct_bitfield_layout_row(row)? else {
                continue;
            };
            let encoded = serde_json::to_string(&layout).map_err(|err| {
                format!(
                    "failed to encode runtime struct bitfield layout `{}`: {err}",
                    layout.type_name
                )
            })?;
            aliases
                .entry(MODULE_BITFIELD_LAYOUT_SCOPE.to_string())
                .or_default()
                .insert(layout.type_name, vec![encoded]);
        }
    }
    Ok(aliases)
}

fn build_opaque_family_types(
    artifact: &AotPackageArtifact,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut families = BTreeMap::<String, Vec<String>>::new();
    for module in &artifact.modules {
        for row in &module.lang_item_rows {
            if parse_struct_bitfield_layout_row(row)?.is_some() {
                continue;
            }
            let (module_id, name, target) = parse_lang_item_row(row)?;
            if module_id != module.module_id {
                return Err(format!(
                    "lang item row `{row}` does not match containing module `{}`",
                    module.module_id
                ));
            }
            if !tracked_opaque_lang_item(&name) {
                continue;
            }
            let target_text = target.join(".");
            let entries = families.entry(name).or_default();
            if !entries.contains(&target_text) {
                entries.push(target_text);
            }
        }
    }
    for entries in families.values_mut() {
        entries.sort();
    }
    Ok(families)
}

pub fn plan_from_artifact(artifact: &AotPackageArtifact) -> Result<RuntimePackagePlan, String> {
    validate_package_artifact(artifact)?;
    let routines = artifact
        .routines
        .iter()
        .map(lower_routine)
        .collect::<Vec<_>>();
    let entrypoints = artifact
        .entrypoints
        .iter()
        .map(|entrypoint| lower_entrypoint(entrypoint, &routines))
        .collect::<Result<Vec<_>, String>>()?;
    let plan = RuntimePackagePlan {
        package_id: artifact.package_id.clone(),
        package_name: artifact.package_name.clone(),
        root_module_id: artifact.root_module_id.clone(),
        direct_deps: artifact.direct_deps.clone(),
        direct_dep_ids: artifact.direct_dep_ids.clone(),
        package_display_names: artifact.package_display_names.clone(),
        package_direct_dep_ids: artifact.package_direct_dep_ids.clone(),
        runtime_requirements: artifact.runtime_requirements.clone(),
        foreword_index: artifact.foreword_index.clone(),
        foreword_registrations: artifact.foreword_registrations.clone(),
        module_aliases: build_module_aliases(artifact)?,
        opaque_family_types: build_opaque_family_types(artifact)?,
        entrypoints,
        routines,
        native_callbacks: artifact
            .native_callbacks
            .iter()
            .map(lower_native_callback)
            .collect(),
        shackle_decls: artifact
            .shackle_decls
            .iter()
            .map(|decl| decl.surface_text.clone())
            .collect(),
        binding_layouts: artifact.binding_layouts.clone(),
        owners: artifact.owners.iter().map(lower_owner).collect(),
    };
    validate_runtime_cleanup_footer_handlers(&plan)?;
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

fn validate_runtime_cleanup_footer_handlers(plan: &RuntimePackagePlan) -> Result<(), String> {
    for routine in &plan.routines {
        for cleanup_footer in &routine.cleanup_footers {
            validate_runtime_cleanup_footer_handler(
                plan,
                &routine.package_id,
                &routine.module_id,
                cleanup_footer,
            )?;
        }
        validate_runtime_cleanup_footer_handlers_in_statements(
            plan,
            &routine.package_id,
            &routine.module_id,
            &routine.statements,
        )?;
    }
    Ok(())
}

fn validate_runtime_cleanup_footer_handlers_in_statements(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    statements: &[ParsedStmt],
) -> Result<(), String> {
    for statement in statements {
        match statement {
            ParsedStmt::Expr {
                cleanup_footers, ..
            } => {
                for cleanup_footer in cleanup_footers {
                    validate_runtime_cleanup_footer_handler(
                        plan,
                        current_package_id,
                        current_module_id,
                        cleanup_footer,
                    )?;
                }
            }
            ParsedStmt::If {
                then_branch,
                else_branch,
                cleanup_footers,
                ..
            } => {
                for cleanup_footer in cleanup_footers {
                    validate_runtime_cleanup_footer_handler(
                        plan,
                        current_package_id,
                        current_module_id,
                        cleanup_footer,
                    )?;
                }
                validate_runtime_cleanup_footer_handlers_in_statements(
                    plan,
                    current_package_id,
                    current_module_id,
                    then_branch,
                )?;
                validate_runtime_cleanup_footer_handlers_in_statements(
                    plan,
                    current_package_id,
                    current_module_id,
                    else_branch,
                )?;
            }
            ParsedStmt::While {
                body,
                cleanup_footers,
                ..
            }
            | ParsedStmt::For {
                body,
                cleanup_footers,
                ..
            } => {
                for cleanup_footer in cleanup_footers {
                    validate_runtime_cleanup_footer_handler(
                        plan,
                        current_package_id,
                        current_module_id,
                        cleanup_footer,
                    )?;
                }
                validate_runtime_cleanup_footer_handlers_in_statements(
                    plan,
                    current_package_id,
                    current_module_id,
                    body,
                )?;
            }
            ParsedStmt::Let { .. }
            | ParsedStmt::ReturnVoid
            | ParsedStmt::ReturnValue { .. }
            | ParsedStmt::ActivateOwner { .. }
            | ParsedStmt::Defer(_)
            | ParsedStmt::Break
            | ParsedStmt::Continue
            | ParsedStmt::Assign { .. }
            | ParsedStmt::Reclaim(_)
            | ParsedStmt::Recycle { .. }
            | ParsedStmt::Bind { .. }
            | ParsedStmt::Record(_)
            | ParsedStmt::Array(_)
            | ParsedStmt::Construct(_)
            | ParsedStmt::MemorySpec(_) => {}
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

fn runtime_type_root_name(type_name: &str) -> String {
    let base = type_name
        .split_once('[')
        .map(|(head, _)| head)
        .unwrap_or(type_name)
        .trim();
    base.rsplit('.').next().unwrap_or(base).to_string()
}

fn runtime_nominal_type_name(type_name: &str) -> String {
    type_name
        .split_once('[')
        .map(|(head, _)| head)
        .unwrap_or(type_name)
        .trim()
        .to_string()
}

fn runtime_bitfield_storage_key(storage_index: u16) -> String {
    format!("{RUNTIME_STRUCT_BITFIELD_STORAGE_PREFIX}{storage_index}")
}

pub(crate) fn runtime_is_hidden_bitfield_storage_field(name: &str) -> bool {
    name.starts_with(RUNTIME_STRUCT_BITFIELD_STORAGE_PREFIX)
}

fn runtime_integer_type_is_signed(name: &str) -> bool {
    matches!(name, "I8" | "I16" | "I32" | "I64")
}

fn runtime_lookup_struct_bitfield_layout(
    plan: &RuntimePackagePlan,
    type_name: &str,
) -> Result<Option<ExecStructBitfieldLayout>, String> {
    let normalized = runtime_nominal_type_name(type_name);
    let Some(scope) = plan.module_aliases.get(MODULE_BITFIELD_LAYOUT_SCOPE) else {
        return Ok(None);
    };
    let Some(encoded_rows) = scope.get(&normalized) else {
        return Ok(None);
    };
    let Some(encoded) = encoded_rows.first() else {
        return Ok(None);
    };
    serde_json::from_str(encoded).map(Some).map_err(|err| {
        format!("failed to parse runtime struct bitfield layout `{normalized}`: {err}")
    })
}

fn runtime_bitfield_mask(width: u16) -> u128 {
    if width >= 128 {
        u128::MAX
    } else {
        (1_u128 << width) - 1
    }
}

fn runtime_pack_bitfield_value(
    value: &RuntimeValue,
    field: &ExecStructBitfieldFieldLayout,
    context: &str,
) -> Result<u128, String> {
    let int_value = expect_int(value.clone(), context)?;
    let signed = runtime_integer_type_is_signed(&field.base_type);
    let width = u32::from(field.bit_width);
    if signed {
        let shift = width.saturating_sub(1);
        let min = -(1_i128 << shift);
        let max = (1_i128 << shift) - 1;
        let numeric = int_value as i128;
        if numeric < min || numeric > max {
            return Err(format!(
                "{context} value `{int_value}` is out of range for {}-bit signed bitfield `{}`",
                field.bit_width, field.name
            ));
        }
        Ok((numeric as u128) & runtime_bitfield_mask(field.bit_width))
    } else {
        if int_value < 0 {
            return Err(format!(
                "{context} value `{int_value}` must be non-negative for unsigned bitfield `{}`",
                field.name
            ));
        }
        let numeric = int_value as u128;
        let mask = runtime_bitfield_mask(field.bit_width);
        if numeric > mask {
            return Err(format!(
                "{context} value `{int_value}` is out of range for {}-bit unsigned bitfield `{}`",
                field.bit_width, field.name
            ));
        }
        Ok(numeric)
    }
}

fn runtime_decode_bitfield_value(
    storage_bits: u128,
    field: &ExecStructBitfieldFieldLayout,
) -> Result<RuntimeValue, String> {
    let raw = (storage_bits >> field.bit_offset) & runtime_bitfield_mask(field.bit_width);
    if runtime_integer_type_is_signed(&field.base_type) {
        let numeric = if field.bit_width == 64 {
            (raw as u64) as i64
        } else {
            let shift = 128_u32 - u32::from(field.bit_width);
            (((raw << shift) as i128) >> shift) as i64
        };
        Ok(RuntimeValue::Int(numeric))
    } else {
        let numeric = i64::try_from(raw as i128).map_err(|_| {
            format!(
                "runtime unsigned bitfield `{}` exceeds the current signed integer carrier",
                field.name
            )
        })?;
        Ok(RuntimeValue::Int(numeric))
    }
}

fn runtime_hidden_bitfield_storage_bits(
    fields: &BTreeMap<String, RuntimeValue>,
    storage_index: u16,
) -> Result<Option<u128>, String> {
    let Some(value) = fields
        .get(&runtime_bitfield_storage_key(storage_index))
        .cloned()
    else {
        return Ok(None);
    };
    Ok(Some(
        (expect_int(value, "struct bitfield storage")? as u64) as u128,
    ))
}

fn apply_runtime_struct_bitfield_layout(
    plan: &RuntimePackagePlan,
    type_name: &str,
    fields: &mut BTreeMap<String, RuntimeValue>,
) -> Result<(), String> {
    let Some(layout) = runtime_lookup_struct_bitfield_layout(plan, type_name)? else {
        return Ok(());
    };
    if layout.fields.is_empty() {
        return Ok(());
    }
    let mut storage_bits = BTreeMap::<u16, u128>::new();
    for field in &layout.fields {
        let value = if let Some(value) = fields.get(&field.name) {
            value.clone()
        } else if let Some(bits) =
            runtime_hidden_bitfield_storage_bits(fields, field.storage_index)?
        {
            runtime_decode_bitfield_value(bits, field)?
        } else {
            return Err(format!(
                "struct bitfield `{}` is missing field `{}` during runtime layout application",
                layout.type_name, field.name
            ));
        };
        let packed = runtime_pack_bitfield_value(
            &value,
            field,
            &format!("struct bitfield `{}`", field.name),
        )?;
        let entry = storage_bits.entry(field.storage_index).or_insert(0);
        let mask = runtime_bitfield_mask(field.bit_width) << field.bit_offset;
        *entry &= !mask;
        *entry |= packed << field.bit_offset;
    }
    for storage_index in layout
        .fields
        .iter()
        .map(|field| field.storage_index)
        .collect::<BTreeSet<_>>()
    {
        fields.remove(&runtime_bitfield_storage_key(storage_index));
    }
    for (storage_index, bits) in &storage_bits {
        fields.insert(
            runtime_bitfield_storage_key(*storage_index),
            RuntimeValue::Int((*bits as u64) as i64),
        );
    }
    for field in &layout.fields {
        let bits = storage_bits
            .get(&field.storage_index)
            .copied()
            .unwrap_or_default();
        fields.insert(
            field.name.clone(),
            runtime_decode_bitfield_value(bits, field)?,
        );
    }
    Ok(())
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
    state.call_stack.pop();
}

fn current_package_name<'a>(
    plan: &'a RuntimePackagePlan,
    current_package_id: &str,
) -> Option<&'a str> {
    plan.package_display_names
        .get(current_package_id)
        .map(String::as_str)
}

fn resolve_visible_package_id_for_root<'a>(
    plan: &'a RuntimePackagePlan,
    current_package_id: &'a str,
    root: &str,
) -> Option<&'a str> {
    if current_package_name(plan, current_package_id) == Some(root) {
        return Some(current_package_id);
    }
    if let Some(package_id) = plan
        .package_direct_dep_ids
        .get(current_package_id)
        .and_then(|deps| deps.get(root))
    {
        return Some(package_id.as_str());
    }
    None
}

fn resolve_routine_module_targets(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    callable_path: &[String],
) -> Vec<(String, String, String)> {
    let (module_id, symbol_name) = match callable_path {
        [] => return Vec::new(),
        [symbol_name] => {
            return vec![(
                current_package_id.to_string(),
                current_module_id.to_string(),
                symbol_name.clone(),
            )];
        }
        _ => (
            callable_path[..callable_path.len() - 1].join("."),
            callable_path.last().cloned().unwrap_or_default(),
        ),
    };

    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push_target = |package_id: &str, module_id: String, symbol_name: &str| {
        let target = (package_id.to_string(), module_id, symbol_name.to_string());
        if seen.insert(target.clone()) {
            targets.push(target);
        }
    };
    let root = callable_path
        .first()
        .map(String::as_str)
        .unwrap_or_default();
    if let Some(package_id) = resolve_visible_package_id_for_root(plan, current_package_id, root) {
        push_target(package_id, module_id.clone(), &symbol_name);
        if let Some(stripped_module) = module_id
            .strip_prefix(root)
            .and_then(|rest| rest.strip_prefix('.'))
            .filter(|module| !module.is_empty())
        {
            push_target(package_id, stripped_module.to_string(), &symbol_name);
        }
        return targets;
    }

    push_target(current_package_id, module_id.clone(), &symbol_name);

    if let Some(package_name) = current_package_name(plan, current_package_id) {
        let prefixed_module = if module_id == package_name
            || module_id.starts_with(&(package_name.to_string() + "."))
        {
            module_id.clone()
        } else {
            format!("{package_name}.{module_id}")
        };
        push_target(current_package_id, prefixed_module, &symbol_name);
    }

    targets
}

fn resolve_routine_index(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    callable_path: &[String],
) -> Option<usize> {
    for (package_id, module_id, symbol_name) in
        resolve_routine_module_targets(plan, current_package_id, current_module_id, callable_path)
    {
        if let Some(index) = plan.routines.iter().position(|routine| {
            routine.package_id == package_id
                && routine.module_id == module_id
                && routine.symbol_name == symbol_name
        }) {
            return Some(index);
        }
    }
    None
}

fn resolve_lowered_routine_index(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    routine_key: &str,
) -> Result<Option<usize>, String> {
    let mut candidate_keys = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push_candidate = |candidate: String| {
        if seen.insert(candidate.clone()) {
            candidate_keys.push(candidate);
        }
    };
    push_candidate(routine_key.to_string());
    if !routine_key.contains('|')
        && let Some((module_id, _)) = routine_key.split_once('#')
    {
        let root = module_id.split('.').next().unwrap_or(module_id);
        if let Some(package_id) =
            resolve_visible_package_id_for_root(plan, current_package_id, root)
        {
            push_candidate(format!("{package_id}|{routine_key}"));
        }
    }

    let filtered = plan
        .routines
        .iter()
        .enumerate()
        .filter_map(|(index, routine)| {
            candidate_keys
                .contains(&routine.routine_key)
                .then_some(index)
        })
        .collect::<Vec<_>>();
    match filtered.as_slice() {
        [] => Ok(None),
        [index] => Ok(Some(*index)),
        _ => Err(format!(
            "runtime lowered routine `{routine_key}` matched duplicate runtime routines"
        )),
    }
}

fn resolve_routine_candidate_indices(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    callable_path: &[String],
) -> Vec<usize> {
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    for (package_id, module_id, symbol_name) in
        resolve_routine_module_targets(plan, current_package_id, current_module_id, callable_path)
    {
        for (index, routine) in plan.routines.iter().enumerate() {
            if routine.package_id == package_id
                && routine.module_id == module_id
                && routine.symbol_name == symbol_name
                && seen.insert(index)
            {
                candidates.push(index);
            }
        }
    }
    candidates
}

fn resolve_method_candidate_indices_by_name(
    plan: &RuntimePackagePlan,
    symbol_name: &str,
) -> Vec<usize> {
    plan.routines
        .iter()
        .enumerate()
        .filter_map(|(index, routine)| {
            (routine.symbol_name == symbol_name && routine.impl_target_type.is_some())
                .then_some(index)
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

fn runtime_value_to_string(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Int(value) => value.to_string(),
        RuntimeValue::Float { text, .. } => text.clone(),
        RuntimeValue::Bool(value) => value.to_string(),
        RuntimeValue::Str(value) => value.clone(),
        RuntimeValue::Bytes(bytes) => format!(
            "Bytes[{}]",
            bytes
                .iter()
                .map(|byte| byte.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::ByteBuffer(bytes) => format!(
            "ByteBuffer[{}]",
            bytes
                .iter()
                .map(|byte| byte.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::Utf16(units) => format!(
            "Utf16[{}]",
            units
                .iter()
                .map(|unit| unit.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        RuntimeValue::Utf16Buffer(units) => format!(
            "Utf16Buffer[{}]",
            units
                .iter()
                .map(|unit| unit.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
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
        RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) => {
            format!("{}#{}", binding.type_name, binding.handle)
        }
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
            RuntimeReferenceTarget::TempSlot { id, members } => {
                if members.is_empty() {
                    format!("<ref:Temp:{}:{}:{}>", id.arena.0, id.slot, id.generation)
                } else {
                    format!(
                        "<ref:Temp:{}:{}:{}:{}>",
                        id.arena.0,
                        id.slot,
                        id.generation,
                        members.join(".")
                    )
                }
            }
            RuntimeReferenceTarget::SessionSlot { id, members } => {
                if members.is_empty() {
                    format!("<ref:Session:{}:{}:{}>", id.arena.0, id.slot, id.generation)
                } else {
                    format!(
                        "<ref:Session:{}:{}:{}:{}>",
                        id.arena.0,
                        id.slot,
                        id.generation,
                        members.join(".")
                    )
                }
            }
            RuntimeReferenceTarget::RingSlot { id, members } => {
                if members.is_empty() {
                    format!("<ref:Ring:{}:{}:{}>", id.arena.0, id.slot, id.generation)
                } else {
                    format!(
                        "<ref:Ring:{}:{}:{}:{}>",
                        id.arena.0,
                        id.slot,
                        id.generation,
                        members.join(".")
                    )
                }
            }
            RuntimeReferenceTarget::SlabSlot { id, members } => {
                if members.is_empty() {
                    format!("<ref:Slab:{}:{}:{}>", id.arena.0, id.slot, id.generation)
                } else {
                    format!(
                        "<ref:Slab:{}:{}:{}:{}>",
                        id.arena.0,
                        id.slot,
                        id.generation,
                        members.join(".")
                    )
                }
            }
        },
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
        RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(handle)) => {
            format!("<TempArena:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(id)) => {
            format!("<TempId:{}:{}:{}>", id.arena.0, id.slot, id.generation)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(handle)) => {
            format!("<SessionArena:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(id)) => {
            format!("<SessionId:{}:{}:{}>", id.arena.0, id.slot, id.generation)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)) => {
            format!("<RingBuffer:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(id)) => {
            format!("<RingId:{}:{}:{}>", id.arena.0, id.slot, id.generation)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle)) => {
            format!("<Slab:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id)) => {
            format!("<SlabId:{}:{}:{}>", id.arena.0, id.slot, id.generation)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
            format!("<ReadView:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => {
            format!("<EditView:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
            format!("<ByteView:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) => {
            format!("<ByteEditView:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
            format!("<StrView:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Task(handle)) => {
            format!("<Task:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(handle)) => {
            format!("<Thread:{}>", handle.0)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Lazy(handle)) => {
            format!("<Lazy:{}>", handle.0)
        }
        RuntimeValue::Record { name, fields } => format!(
            "{}{{{}}}",
            name,
            fields
                .iter()
                .filter(|(field, _)| !runtime_is_hidden_bitfield_storage_field(field))
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

fn format_runtime_float_text(value: f64) -> String {
    let mut text = value.to_string();
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        text.push_str(".0");
    }
    text
}

fn parse_runtime_float_text(text: &str, kind: ParsedFloatKind) -> Result<f64, String> {
    let normalized = text
        .strip_suffix("f32")
        .or_else(|| text.strip_suffix("f64"))
        .unwrap_or(text);
    match kind {
        ParsedFloatKind::F32 => normalized
            .parse::<f32>()
            .map(f64::from)
            .map_err(|err| format!("invalid F32 literal `{text}`: {err}")),
        ParsedFloatKind::F64 => normalized
            .parse::<f64>()
            .map_err(|err| format!("invalid F64 literal `{text}`: {err}")),
    }
}

fn make_runtime_float(kind: ParsedFloatKind, value: f64) -> RuntimeValue {
    match kind {
        ParsedFloatKind::F32 => {
            let value = value as f32;
            RuntimeValue::Float {
                text: format_runtime_float_text(f64::from(value)),
                kind,
            }
        }
        ParsedFloatKind::F64 => RuntimeValue::Float {
            text: format_runtime_float_text(value),
            kind,
        },
    }
}

fn lookup_local<'a>(scopes: &'a [RuntimeScope], name: &str) -> Option<&'a RuntimeLocal> {
    scopes.iter().rev().find_map(|scope| scope.locals.get(name))
}

fn lookup_memory_spec_in_scopes<'a>(
    scopes: &'a [RuntimeScope],
    name: &str,
) -> Option<&'a RuntimeMemorySpecState> {
    scopes
        .iter()
        .rev()
        .find_map(|scope| scope.memory_specs.get(name))
}

fn lookup_memory_spec_in_scopes_mut<'a>(
    scopes: &'a mut [RuntimeScope],
    name: &str,
) -> Option<&'a mut RuntimeMemorySpecState> {
    scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.memory_specs.get_mut(name))
}

fn read_runtime_local_value(
    scopes: &[RuntimeScope],
    state: &RuntimeExecutionState,
    name: &str,
) -> Result<RuntimeValue, String> {
    let local = lookup_local(scopes, name)
        .ok_or_else(|| format!("unsupported runtime value path `{name}`"))?;
    if local.moved {
        return Err(format!("use of moved local `{name}`"));
    }
    if local.take_reserved {
        return Err(format!(
            "local `{name}` is reserved by an active `&take` capability"
        ));
    }
    if local.held {
        return Err(format!(
            "local `{name}` is suspended by an active `&hold` capability"
        ));
    }
    Ok(state
        .captured_local_values
        .get(&local.handle)
        .cloned()
        .unwrap_or_else(|| local.value.clone()))
}

fn lookup_local_mut_by_handle(
    scopes: &mut [RuntimeScope],
    handle: RuntimeLocalHandle,
) -> Option<&mut RuntimeLocal> {
    scopes.iter_mut().rev().find_map(|scope| {
        scope
            .locals
            .values_mut()
            .find(|local| local.handle == handle)
    })
}

fn lookup_local_with_name_by_handle(
    scopes: &[RuntimeScope],
    handle: RuntimeLocalHandle,
) -> Option<(&str, &RuntimeLocal)> {
    scopes.iter().rev().find_map(|scope| {
        scope
            .locals
            .iter()
            .find_map(|(name, local)| (local.handle == handle).then_some((name.as_str(), local)))
    })
}

fn runtime_scope_local_summary(scopes: &[RuntimeScope]) -> String {
    let mut entries = Vec::new();
    for scope in scopes.iter().rev() {
        for (name, local) in &scope.locals {
            entries.push(format!("{name}#{}", local.handle.0));
        }
    }
    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

fn push_runtime_cleanup_footer_frame(
    state: &mut RuntimeExecutionState,
    cleanup_footers: &[ParsedCleanupFooter],
    scopes: &[RuntimeScope],
    current_package_id: &str,
    current_module_id: &str,
) {
    if cleanup_footers.is_empty() {
        return;
    }
    state.cleanup_footer_frames.push(RuntimeCleanupFooterFrame {
        cleanup_footers: cleanup_footers.to_vec(),
        current_package_id: current_package_id.to_string(),
        current_module_id: current_module_id.to_string(),
        owner_call_stack_depth: state.call_stack.len(),
        owner_scope_depth: scopes.len(),
        activations: Vec::new(),
    });
}

fn pop_runtime_cleanup_footer_frame(
    state: &mut RuntimeExecutionState,
) -> Option<RuntimeCleanupFooterFrame> {
    state.cleanup_footer_frames.pop()
}

fn activate_runtime_cleanup_footer_binding(
    state: &mut RuntimeExecutionState,
    scope_depth: usize,
    binding_id: u64,
    name: &str,
    handle: RuntimeLocalHandle,
    value: &RuntimeValue,
) {
    let current_call_stack_depth = state.call_stack.len();
    for frame in state.cleanup_footer_frames.iter_mut().rev() {
        if frame.owner_scope_depth != scope_depth
            || frame.owner_call_stack_depth != current_call_stack_depth
        {
            continue;
        }
        if frame.cleanup_footers.iter().any(|cleanup_footer| {
            (cleanup_footer.binding_id != 0 && cleanup_footer.binding_id == binding_id)
                || (cleanup_footer.binding_id == 0 && cleanup_footer.subject == name)
        }) {
            frame.activations.push(RuntimeTrackedCleanupBinding {
                binding_id,
                subject: name.to_string(),
                binding: handle,
                value: value.clone(),
            });
        }
    }
}

fn update_runtime_cleanup_footer_binding_value(
    state: &mut RuntimeExecutionState,
    handle: RuntimeLocalHandle,
    value: &RuntimeValue,
) {
    for frame in state.cleanup_footer_frames.iter_mut().rev() {
        for subject in frame.activations.iter_mut() {
            if subject.binding == handle {
                subject.value = value.clone();
            }
        }
    }
}

fn insert_runtime_local(
    state: &mut RuntimeExecutionState,
    scope_depth: usize,
    scope: &mut RuntimeScope,
    binding_id: u64,
    name: String,
    mutable: bool,
    value: RuntimeValue,
) {
    let handle = RuntimeLocalHandle(state.next_local_handle);
    state.next_local_handle += 1;
    activate_runtime_cleanup_footer_binding(state, scope_depth, binding_id, &name, handle, &value);
    scope.locals.insert(
        name,
        RuntimeLocal {
            handle,
            binding_id,
            mutable,
            moved: false,
            held: false,
            take_reserved: false,
            value,
        },
    );
}

fn apply_runtime_availability_attachments(
    scope: &mut RuntimeScope,
    attachments: &[ParsedAvailabilityAttachment],
) {
    for attachment in attachments {
        match attachment.kind {
            ParsedAvailabilityKind::Object => {
                scope
                    .attached_object_names
                    .insert(attachment.local_name.clone());
                scope.attached_objects.push(RuntimeAttachedObject {
                    object_path: attachment.path.clone(),
                    local_name: attachment.local_name.clone(),
                });
            }
            ParsedAvailabilityKind::Owner => {
                scope.attached_owners.push(RuntimeAttachedOwner {
                    owner_path: attachment.path.clone(),
                    local_name: attachment.local_name.clone(),
                });
            }
        }
    }
}

fn owner_state_key(package_id: &str, owner_path: &[String]) -> String {
    format!("{package_id}|{}", owner_path.join("."))
}

fn parse_owner_state_key(owner_key: &str) -> (String, Vec<String>) {
    if let Some((package_id, owner_path)) = owner_key.split_once('|') {
        return (
            package_id.to_string(),
            owner_path
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(ToString::to_string)
                .collect(),
        );
    }
    let owner_path = owner_key
        .split('.')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    (owner_path.first().cloned().unwrap_or_default(), owner_path)
}

fn lookup_runtime_owner_plan<'a>(
    plan: &'a RuntimePackagePlan,
    package_id: &str,
    owner_path: &[String],
) -> Option<&'a RuntimeOwnerPlan> {
    plan.owners
        .iter()
        .find(|owner| owner.package_id == package_id && owner.owner_path == owner_path)
}

fn resolve_visible_package_id_for_path<'a>(
    plan: &'a RuntimePackagePlan,
    current_package_id: &'a str,
    path: &[String],
) -> Option<&'a str> {
    let root = path.first().map(String::as_str)?;
    resolve_visible_package_id_for_root(plan, current_package_id, root)
}

fn runtime_module_exists(plan: &RuntimePackagePlan, package_id: &str, module_id: &str) -> bool {
    plan.module_aliases
        .contains_key(&module_alias_scope_key(package_id, module_id))
}

fn resolve_memory_spec_target(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    path: &[String],
) -> Option<(String, String, String)> {
    if path.is_empty() {
        return None;
    }
    let canonical = if path.len() == 1 {
        path.to_vec()
    } else if let Some(prefix) = aliases.get(&path[0]) {
        let mut resolved = prefix.clone();
        resolved.extend(path[1..].iter().cloned());
        resolved
    } else {
        path.to_vec()
    };
    let spec_name = canonical.last()?.clone();
    if canonical.len() == 1 {
        return Some((
            current_package_id.to_string(),
            current_module_id.to_string(),
            spec_name,
        ));
    }
    let package_id = if let Some(package_id) =
        resolve_visible_package_id_for_root(plan, current_package_id, &canonical[0])
    {
        package_id.to_string()
    } else {
        let current_package_name = current_package_name(plan, current_package_id)?;
        let relative = current_module_id
            .split('.')
            .skip(1)
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut module_segments = vec![current_package_name.to_string()];
        module_segments.extend(relative);
        module_segments.extend(canonical[..canonical.len() - 1].iter().cloned());
        let module_id = module_segments.join(".");
        return runtime_module_exists(plan, current_package_id, &module_id).then_some((
            current_package_id.to_string(),
            module_id,
            spec_name,
        ));
    };
    let module_id = canonical[..canonical.len() - 1].join(".");
    if runtime_module_exists(plan, &package_id, &module_id) {
        return Some((package_id, module_id, spec_name));
    }
    let stripped_module = module_id
        .strip_prefix(&canonical[0])
        .and_then(|rest| rest.strip_prefix('.'))
        .filter(|module| !module.is_empty())?
        .to_string();
    runtime_module_exists(plan, &package_id, &stripped_module).then_some((
        package_id,
        stripped_module,
        spec_name,
    ))
}

fn lookup_module_memory_spec_decl(
    plan: &RuntimePackagePlan,
    package_id: &str,
    module_id: &str,
    name: &str,
) -> Result<Option<ParsedMemorySpecDecl>, String> {
    let Some(module_aliases) = plan
        .module_aliases
        .get(&module_alias_scope_key(package_id, module_id))
    else {
        return Ok(None);
    };
    let Some(encoded) = module_aliases.get(&memory_spec_alias_name(name)) else {
        return Ok(None);
    };
    let [json] = encoded.as_slice() else {
        return Err(format!(
            "runtime memory spec alias `{name}` in `{module_id}` is malformed"
        ));
    };
    serde_json::from_str(json)
        .map(Some)
        .map_err(|err| format!("failed to decode runtime memory spec `{name}`: {err}"))
}

fn attached_object_is_visible(scopes: &[RuntimeScope], object_path: &[String], name: &str) -> bool {
    scopes
        .iter()
        .rev()
        .flat_map(|scope| scope.attached_objects.iter())
        .any(|object| object.local_name == name && object.object_path == object_path)
}

fn collect_active_owner_keys_from_scopes(scopes: &[RuntimeScope]) -> Vec<String> {
    let mut active = BTreeSet::new();
    for scope in scopes {
        for owner_key in &scope.inherited_active_owner_keys {
            active.insert(owner_key.clone());
        }
        for owner_key in &scope.activated_owner_keys {
            active.insert(owner_key.clone());
        }
        for local in scope.locals.values() {
            match &local.value {
                RuntimeValue::OwnerHandle(owner_key) => {
                    active.insert(owner_key.clone());
                }
                RuntimeValue::Ref(reference) => {
                    if let RuntimeReferenceTarget::OwnerObject { owner_key, .. } = &reference.target
                    {
                        active.insert(owner_key.clone());
                    }
                }
                _ => {}
            }
        }
    }
    active.into_iter().collect()
}

fn collect_active_owner_keys_from_state(state: &RuntimeExecutionState) -> Vec<String> {
    state
        .owners
        .iter()
        .filter_map(|(owner_key, owner_state)| {
            if owner_state.active_bindings > 0 {
                Some(owner_key.clone())
            } else {
                None
            }
        })
        .collect()
}

fn owner_key_active_on_execution_path(
    scopes: &[RuntimeScope],
    inherited_active_owner_keys: &[String],
    owner_key: &str,
) -> bool {
    scopes.iter().rev().any(|scope| {
        scope
            .activated_owner_keys
            .iter()
            .any(|active| active == owner_key)
            || scope
                .inherited_active_owner_keys
                .iter()
                .any(|active| active == owner_key)
    }) || inherited_active_owner_keys
        .iter()
        .any(|active| active == owner_key)
}

fn activate_attached_runtime_owners_for_current_scope(
    scopes: &mut Vec<RuntimeScope>,
    inherited_active_owner_keys: &[String],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    state: &mut RuntimeExecutionState,
) -> Result<(), String> {
    let attached_owners = scopes
        .last()
        .ok_or_else(|| "runtime scope stack is empty".to_string())?
        .attached_owners
        .clone();
    for attached_owner in attached_owners {
        let Some(owner_package_id) = resolve_visible_package_id_for_path(
            plan,
            current_package_id,
            &attached_owner.owner_path,
        ) else {
            continue;
        };
        let owner_key = owner_state_key(owner_package_id, &attached_owner.owner_path);
        if !owner_key_active_on_execution_path(scopes, inherited_active_owner_keys, &owner_key) {
            continue;
        }
        let owner = lookup_runtime_owner_plan(plan, owner_package_id, &attached_owner.owner_path)
            .ok_or_else(|| {
            format!(
                "runtime availability attachment `{}` resolves to an unknown owner",
                attached_owner.owner_path.join(".")
            )
        })?;
        let binding = if attached_owner.local_name == owner.owner_name {
            None
        } else {
            Some(attached_owner.local_name.as_str())
        };
        activate_owner_scope_binding(scopes, state, owner, &owner_key, binding, &[])?;
    }
    Ok(())
}

fn activate_attached_runtime_objects_for_current_scope(
    scopes: &mut Vec<RuntimeScope>,
    inherited_active_owner_keys: &[String],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    state: &mut RuntimeExecutionState,
) -> Result<(), String> {
    let attached_objects = scopes
        .last()
        .ok_or_else(|| "runtime scope stack is empty".to_string())?
        .attached_objects
        .clone();
    for attached_object in attached_objects {
        let attached_package_id = resolve_visible_package_id_for_path(
            plan,
            current_package_id,
            &attached_object.object_path,
        );
        let matches = plan
            .owners
            .iter()
            .filter_map(|owner| {
                if attached_package_id != Some(owner.package_id.as_str()) {
                    return None;
                }
                let owner_key = owner_state_key(&owner.package_id, &owner.owner_path);
                if !owner_key_active_on_execution_path(
                    scopes,
                    inherited_active_owner_keys,
                    &owner_key,
                ) {
                    return None;
                }
                owner
                    .objects
                    .iter()
                    .find(|object| {
                        object.local_name == attached_object.local_name
                            && object.type_path == attached_object.object_path
                    })
                    .map(|object| (owner_key, object.local_name.clone()))
            })
            .collect::<Vec<_>>();
        if matches.len() > 1 {
            return Err(format!(
                "attached object `{}` is ambiguous across active owners",
                attached_object.local_name
            ));
        }
        let Some((owner_key, object_name)) = matches.into_iter().next() else {
            continue;
        };
        let scope_depth = scopes.len().saturating_sub(1);
        let inherited_value =
            inherited_attached_object_value(scopes, plan, current_package_id, &attached_object)?;
        let scope = scopes
            .last_mut()
            .ok_or_else(|| "runtime scope stack is empty".to_string())?;
        if scope.locals.contains_key(&attached_object.local_name) {
            continue;
        }
        if let Some(value) = inherited_value {
            insert_runtime_local(
                state,
                scope_depth,
                scope,
                0,
                attached_object.local_name,
                true,
                value,
            );
            continue;
        }
        insert_runtime_local(
            state,
            scope_depth,
            scope,
            0,
            attached_object.local_name,
            true,
            make_owner_object_reference(&owner_key, &object_name),
        );
    }
    Ok(())
}

fn owner_object_matches_attachment(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    owner_key: &str,
    object_name: &str,
    attached_object: &RuntimeAttachedObject,
) -> Result<bool, String> {
    let (owner_package_id, owner_path) = parse_owner_state_key(owner_key);
    let attached_package_id =
        resolve_visible_package_id_for_path(plan, current_package_id, &attached_object.object_path);
    if attached_package_id != Some(owner_package_id.as_str()) {
        return Ok(false);
    }
    let owner =
        lookup_runtime_owner_plan(plan, &owner_package_id, &owner_path).ok_or_else(|| {
            format!("runtime owner `{owner_key}` is not declared in the package plan")
        })?;
    let object = lookup_runtime_owner_object_plan(owner, object_name)?;
    Ok(object.local_name == attached_object.local_name
        && object.type_path == attached_object.object_path)
}

fn inherited_attached_object_value(
    scopes: &[RuntimeScope],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    attached_object: &RuntimeAttachedObject,
) -> Result<Option<RuntimeValue>, String> {
    let mut matches = Vec::new();
    for scope in scopes.iter().rev().skip(1) {
        let Some(local) = scope.locals.get(&attached_object.local_name) else {
            continue;
        };
        let RuntimeValue::Ref(reference) = &local.value else {
            continue;
        };
        let RuntimeReferenceTarget::OwnerObject {
            owner_key,
            object_name,
            members,
        } = &reference.target
        else {
            continue;
        };
        if !members.is_empty() {
            continue;
        }
        if owner_object_matches_attachment(
            plan,
            current_package_id,
            owner_key,
            object_name,
            attached_object,
        )? {
            let pair = (owner_key.clone(), object_name.clone(), local.value.clone());
            if !matches.iter().any(|(seen_owner, seen_object, _)| {
                seen_owner == owner_key && seen_object == object_name
            }) {
                matches.push(pair);
            }
        }
    }
    if matches.len() > 1 {
        return Err(format!(
            "attached object `{}` is ambiguous across active owners",
            attached_object.local_name
        ));
    }
    Ok(matches.into_iter().next().map(|(_, _, value)| value))
}

fn make_owner_object_reference(owner_key: &str, object_name: &str) -> RuntimeValue {
    RuntimeValue::Ref(RuntimeReferenceValue {
        mode: RuntimeReferenceMode::Edit,
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
    owner_package_id: &str,
    owner_path: &[String],
    object_name: &str,
) -> Result<RuntimeValue, String> {
    let owner = lookup_runtime_owner_plan(plan, owner_package_id, owner_path).ok_or_else(|| {
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
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    owner_key: &str,
    object_name: &str,
) -> Result<(), String> {
    let (owner_package_id, owner_path) = parse_owner_state_key(owner_key);
    let owner =
        lookup_runtime_owner_plan(plan, &owner_package_id, &owner_path).ok_or_else(|| {
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
                let value =
                    realize_owner_object_value(plan, &owner_package_id, &owner_path, object_name)?;
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
            &collect_active_owner_keys_from_state(state),
            None,
            None,
            None,
            None,
            None,
            state,
            host,
            false,
        )
        .map_err(runtime_eval_message)?;
        if let Some(control) = outcome.control {
            return Err(runtime_eval_message(match control {
                FlowSignal::OwnerExit {
                    owner_key,
                    exit_name,
                } => RuntimeEvalSignal::OwnerExit {
                    owner_key,
                    exit_name,
                },
                other => RuntimeEvalSignal::Message(format!(
                    "unsupported lifecycle hook control flow `{other:?}`"
                )),
            }));
        }
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
        let owner_state = state.owners.entry(owner_key.to_string()).or_default();
        owner_state
            .objects
            .insert(object_name.to_string(), updated_value);
        owner_state.pending_init.remove(object_name);
        owner_state.pending_resume.remove(object_name);
        let owner_keys = vec![owner_key.to_string()];
        evaluate_owner_exit_checkpoints(
            &owner_keys,
            plan,
            current_package_id,
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
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    owner_key: &str,
    object_name: &str,
) -> Result<RuntimeValue, String> {
    let (_owner_package_id, owner_path) = parse_owner_state_key(owner_key);
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
        current_package_id,
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
        0,
        owner.owner_name.clone(),
        false,
        RuntimeValue::OwnerHandle(owner_key.to_string()),
    );
    for object in &owner.objects {
        insert_runtime_local(
            state,
            0,
            &mut scope,
            0,
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

fn apply_explicit_owner_exit(
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    owner_key: &str,
    exit_name: &str,
    scopes: Option<&mut Vec<RuntimeScope>>,
) -> Result<(), String> {
    let (owner_package_id, owner_path) = parse_owner_state_key(owner_key);
    let owner =
        lookup_runtime_owner_plan(plan, &owner_package_id, &owner_path).ok_or_else(|| {
            format!(
                "runtime owner `{}` is not declared in the package plan",
                owner_key
            )
        })?;
    let owner_exit = owner
        .exits
        .iter()
        .find(|owner_exit| owner_exit.name == exit_name)
        .ok_or_else(|| {
            format!(
                "owner `{}` does not declare exit `{exit_name}`",
                owner_path.join(".")
            )
        })?;
    {
        let owner_state = state.owners.entry(owner_key.to_string()).or_default();
        owner_state
            .objects
            .retain(|name, _| owner_exit.retains.iter().any(|retain| retain == name));
        owner_state.pending_init.clear();
        owner_state.pending_resume.clear();
        owner_state.activation_context = None;
        owner_state.active_bindings = 0;
    }
    runtime_reset_owner_exit_module_memory_specs(state, owner_key)?;
    if let Some(scopes) = scopes {
        runtime_reset_owner_exit_memory_specs_in_scopes(scopes, state, owner_key)?;
        invalidate_owner_activations_in_scopes(scopes, owner_key);
    }
    Ok(())
}

fn resolve_named_owner_exit_target(
    plan: &RuntimePackagePlan,
    scopes: &[RuntimeScope],
    exit_name: &str,
) -> Result<String, String> {
    let matches = collect_active_owner_keys_from_scopes(scopes)
        .into_iter()
        .filter(|owner_key| {
            let (package_id, owner_path) = parse_owner_state_key(owner_key);
            lookup_runtime_owner_plan(plan, &package_id, &owner_path)
                .map(|owner| {
                    owner
                        .exits
                        .iter()
                        .any(|owner_exit| owner_exit.name == exit_name)
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(format!(
            "named recycle exit `{exit_name}` is not active on this path"
        )),
        [owner_key] => Ok(owner_key.clone()),
        _ => Err(format!(
            "named recycle exit `{exit_name}` is ambiguous across active owners"
        )),
    }
}

fn evaluate_owner_exit_checkpoints(
    owner_keys: &[String],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    mut scopes: Option<&mut Vec<RuntimeScope>>,
) -> Result<(), String> {
    let mut unique_keys = BTreeSet::new();
    for owner_key in owner_keys {
        if !unique_keys.insert(owner_key.clone()) {
            continue;
        }
        let (owner_package_id, owner_path) = parse_owner_state_key(owner_key);
        let Some(owner) = lookup_runtime_owner_plan(plan, &owner_package_id, &owner_path).cloned()
        else {
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
                current_package_id,
                current_module_id,
                &mut exit_scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
            .map_err(runtime_eval_message)?;
            let condition = read_runtime_value_if_ref(
                condition,
                &mut exit_scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )?;
            if expect_bool(condition, "owner exit condition")? {
                selected_exit = Some(owner_exit.clone());
                break;
            }
        }
        if let Some(owner_exit) = selected_exit {
            {
                let owner_state = state.owners.entry(owner_key.clone()).or_default();
                owner_state
                    .objects
                    .retain(|name, _| owner_exit.retains.iter().any(|retain| retain == name));
                owner_state.pending_init.clear();
                owner_state.pending_resume.clear();
                owner_state.activation_context = None;
                owner_state.active_bindings = 0;
            }
            runtime_reset_owner_exit_module_memory_specs(state, owner_key)?;
            if let Some(scopes) = scopes.as_deref_mut() {
                runtime_reset_owner_exit_memory_specs_in_scopes(scopes, state, owner_key)?;
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
    object_binding_ids: &[ParsedNamedBindingId],
) -> Result<(), String> {
    let scope_depth = scopes.len().saturating_sub(1);
    let visible_objects = owner
        .objects
        .iter()
        .filter(|object| attached_object_is_visible(scopes, &object.type_path, &object.local_name))
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
        0,
        owner.owner_name.clone(),
        false,
        RuntimeValue::OwnerHandle(owner_key.to_string()),
    );
    if let Some(binding) = binding {
        insert_runtime_local(
            state,
            scope_depth,
            scope,
            0,
            binding.to_string(),
            false,
            RuntimeValue::OwnerHandle(owner_key.to_string()),
        );
    }
    for object_name in visible_objects {
        let binding_id = object_binding_ids
            .iter()
            .find(|entry| entry.name == object_name)
            .map(|entry| entry.binding_id)
            .unwrap_or(0);
        insert_runtime_local(
            state,
            scope_depth,
            scope,
            binding_id,
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
        RuntimeReferenceTarget::TempSlot { id, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::TempSlot {
                id: *id,
                members: next_members,
            }
        }
        RuntimeReferenceTarget::SessionSlot { id, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::SessionSlot {
                id: *id,
                members: next_members,
            }
        }
        RuntimeReferenceTarget::RingSlot { id, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::RingSlot {
                id: *id,
                members: next_members,
            }
        }
        RuntimeReferenceTarget::SlabSlot { id, members } => {
            let mut next_members = members.clone();
            next_members.push(member);
            RuntimeReferenceTarget::SlabSlot {
                id: *id,
                members: next_members,
            }
        }
    }
}

fn runtime_local_value_ref<'a>(
    scopes: &'a [RuntimeScope],
    state: &'a RuntimeExecutionState,
    handle: RuntimeLocalHandle,
) -> Option<(&'a str, &'a RuntimeValue, bool, bool, bool)> {
    if let Some((name, runtime_local)) = lookup_local_with_name_by_handle(scopes, handle) {
        if let Some(captured) = state.captured_local_values.get(&handle) {
            return Some((
                name,
                captured,
                runtime_local.moved,
                runtime_local.held,
                runtime_local.take_reserved,
            ));
        }
        return Some((
            name,
            &runtime_local.value,
            runtime_local.moved,
            runtime_local.held,
            runtime_local.take_reserved,
        ));
    }
    state
        .captured_local_values
        .get(&handle)
        .map(|value| ("<captured>", value, false, false, false))
}

fn runtime_member_value_ref<'a>(
    value: &'a RuntimeValue,
    member: &str,
) -> Result<Option<&'a RuntimeValue>, String> {
    match value {
        RuntimeValue::Pair(left, right) => match member {
            "0" => Ok(Some(left.as_ref())),
            "1" => Ok(Some(right.as_ref())),
            _ => Err(format!("pair has no member `.{member}`")),
        },
        RuntimeValue::Record { name, fields } => fields
            .get(member)
            .map(Some)
            .ok_or_else(|| format!("record `{name}` has no field `.{member}`")),
        RuntimeValue::Variant { .. } | RuntimeValue::OwnerHandle(_) => Ok(None),
        other => Err(format!(
            "unsupported runtime member access `.{member}` on `{other:?}`"
        )),
    }
}

fn runtime_reference_root_value_ref<'a>(
    scopes: &'a [RuntimeScope],
    state: &'a RuntimeExecutionState,
    target: &RuntimeReferenceTarget,
    access_mode: RuntimeReferenceMode,
) -> Result<Option<&'a RuntimeValue>, String> {
    match target {
        RuntimeReferenceTarget::Local { local, .. } => {
            let Some((name, value, moved, held, take_reserved)) =
                runtime_local_value_ref(scopes, state, *local)
            else {
                return Err(format!(
                    "runtime reference local `{}` is unresolved; visible locals: {}",
                    local.0,
                    runtime_scope_local_summary(scopes)
                ));
            };
            if moved {
                return Err(format!("use of moved local `{name}`"));
            }
            if take_reserved && access_mode != RuntimeReferenceMode::Take {
                return Err(format!(
                    "local `{name}` is reserved by an active `&take` capability"
                ));
            }
            if held && access_mode != RuntimeReferenceMode::Hold {
                return Err(format!(
                    "local `{name}` is suspended by an active `&hold` capability"
                ));
            }
            Ok(Some(value))
        }
        RuntimeReferenceTarget::OwnerObject { .. } => Ok(None),
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
            Ok(arena.slots.get(&id.slot))
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
            Ok(arena.slots.get(&id.slot))
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
            Ok(arena.slots.get(&id.slot))
        }
        RuntimeReferenceTarget::TempSlot { id, .. } => {
            let arena = state
                .temp_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", id.arena.0))?;
            if !temp_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid TempId `{}` for TempArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(*id))),
                    id.arena.0
                ));
            }
            Ok(arena.slots.get(&id.slot))
        }
        RuntimeReferenceTarget::SessionSlot { id, .. } => {
            let arena = state
                .session_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", id.arena.0))?;
            if !session_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SessionId `{}` for SessionArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            Ok(arena.slots.get(&id.slot))
        }
        RuntimeReferenceTarget::RingSlot { id, .. } => {
            let arena = state
                .ring_buffers
                .get(&id.arena)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", id.arena.0))?;
            if !ring_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid RingId `{}` for RingBuffer `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(*id))),
                    id.arena.0
                ));
            }
            Ok(arena.slots.get(&id.slot))
        }
        RuntimeReferenceTarget::SlabSlot { id, .. } => {
            let arena = state
                .slabs
                .get(&id.arena)
                .ok_or_else(|| format!("invalid Slab handle `{}`", id.arena.0))?;
            if !slab_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(*id))),
                    id.arena.0
                ));
            }
            Ok(arena.slots.get(&id.slot))
        }
    }
}

fn runtime_reference_value_ref<'a>(
    scopes: &'a [RuntimeScope],
    state: &'a RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
) -> Result<Option<&'a RuntimeValue>, String> {
    let Some(mut value) =
        runtime_reference_root_value_ref(scopes, state, &reference.target, reference.mode.clone())?
    else {
        return Ok(None);
    };
    for member in runtime_reference_members(&reference.target) {
        let Some(next) = runtime_member_value_ref(value, member)? else {
            return Ok(None);
        };
        value = next;
    }
    Ok(Some(value))
}

fn runtime_reference_array_len(
    scopes: &[RuntimeScope],
    state: &RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    context: &str,
) -> Result<Option<usize>, String> {
    let Some(value) = runtime_reference_value_ref(scopes, state, reference)? else {
        return Ok(None);
    };
    match value {
        RuntimeValue::Array(values) => Ok(Some(values.len())),
        RuntimeValue::Bytes(bytes) => Ok(Some(bytes.len())),
        RuntimeValue::ByteBuffer(bytes) => Ok(Some(bytes.len())),
        _ => Err(format!("{context} expected byte-compatible array or Bytes")),
    }
}

fn runtime_reference_array_byte_at(
    scopes: &[RuntimeScope],
    state: &RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    index: usize,
    context: &str,
) -> Result<Option<u8>, String> {
    let Some(value) = runtime_reference_value_ref(scopes, state, reference)? else {
        return Ok(None);
    };
    match value {
        RuntimeValue::Array(values) => {
            let value = values
                .get(index)
                .cloned()
                .ok_or_else(|| format!("{context} index `{index}` is out of bounds"))?;
            let value = expect_int(value, context)?;
            if !(0..=255).contains(&value) {
                return Err(format!(
                    "{context} byte `{value}` is out of range `0..=255`"
                ));
            }
            Ok(Some(value as u8))
        }
        RuntimeValue::Bytes(bytes) => bytes
            .get(index)
            .copied()
            .map(Some)
            .ok_or_else(|| format!("{context} index `{index}` is out of bounds")),
        RuntimeValue::ByteBuffer(bytes) => bytes
            .get(index)
            .copied()
            .map(Some)
            .ok_or_else(|| format!("{context} index `{index}` is out of bounds")),
        _ => Err(format!("{context} expected byte-compatible array or Bytes")),
    }
}

fn runtime_reference_members(target: &RuntimeReferenceTarget) -> &[String] {
    match target {
        RuntimeReferenceTarget::Local { members, .. }
        | RuntimeReferenceTarget::OwnerObject { members, .. }
        | RuntimeReferenceTarget::ArenaSlot { members, .. }
        | RuntimeReferenceTarget::FrameSlot { members, .. }
        | RuntimeReferenceTarget::PoolSlot { members, .. }
        | RuntimeReferenceTarget::TempSlot { members, .. }
        | RuntimeReferenceTarget::SessionSlot { members, .. }
        | RuntimeReferenceTarget::RingSlot { members, .. }
        | RuntimeReferenceTarget::SlabSlot { members, .. } => members,
    }
}

fn runtime_any_live_element_view_reference(
    state: &RuntimeExecutionState,
    predicate: impl Fn(&RuntimeReferenceValue) -> bool,
) -> bool {
    state.read_views.values().any(|view| {
        matches!(
            &view.backing,
            RuntimeElementViewBacking::Reference(reference) if predicate(reference)
        )
    }) || state.edit_views.values().any(|view| {
        matches!(
            &view.backing,
            RuntimeElementViewBacking::Reference(reference) if predicate(reference)
        )
    })
}

fn runtime_reference_targets_arena(
    reference: &RuntimeReferenceValue,
    handle: RuntimeArenaHandle,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::ArenaSlot { id, .. } if id.arena == handle
    )
}

fn runtime_reference_targets_arena_id(
    reference: &RuntimeReferenceValue,
    id: RuntimeArenaIdValue,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::ArenaSlot {
            id: reference_id,
            ..
        } if *reference_id == id
    )
}

fn runtime_reference_targets_frame_arena(
    reference: &RuntimeReferenceValue,
    handle: RuntimeFrameArenaHandle,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::FrameSlot { id, .. } if id.arena == handle
    )
}

fn runtime_reference_targets_pool_arena(
    reference: &RuntimeReferenceValue,
    handle: RuntimePoolArenaHandle,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::PoolSlot { id, .. } if id.arena == handle
    )
}

fn runtime_reference_targets_pool_id(
    reference: &RuntimeReferenceValue,
    id: RuntimePoolIdValue,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::PoolSlot {
            id: reference_id,
            ..
        } if *reference_id == id
    )
}

fn runtime_reference_targets_temp_arena(
    reference: &RuntimeReferenceValue,
    handle: RuntimeTempArenaHandle,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::TempSlot { id, .. } if id.arena == handle
    )
}

fn runtime_reference_targets_session_arena(
    reference: &RuntimeReferenceValue,
    handle: RuntimeSessionArenaHandle,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::SessionSlot { id, .. } if id.arena == handle
    )
}

fn runtime_reference_targets_ring_arena(
    reference: &RuntimeReferenceValue,
    handle: RuntimeRingBufferHandle,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::RingSlot { id, .. } if id.arena == handle
    )
}

fn runtime_reference_targets_ring_id(
    reference: &RuntimeReferenceValue,
    id: RuntimeRingIdValue,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::RingSlot {
            id: reference_id,
            ..
        } if *reference_id == id
    )
}

fn runtime_reference_targets_slab_arena(
    reference: &RuntimeReferenceValue,
    handle: RuntimeSlabHandle,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::SlabSlot { id, .. } if id.arena == handle
    )
}

fn runtime_reference_targets_slab_id(
    reference: &RuntimeReferenceValue,
    id: RuntimeSlabIdValue,
) -> bool {
    matches!(
        &reference.target,
        RuntimeReferenceTarget::SlabSlot {
            id: reference_id,
            ..
        } if *reference_id == id
    )
}

fn runtime_reject_live_view_conflict(
    state: &RuntimeExecutionState,
    predicate: impl Fn(&RuntimeReferenceValue) -> bool,
    message: String,
) -> Result<(), String> {
    if runtime_any_live_element_view_reference(state, predicate) {
        return Err(message);
    }
    Ok(())
}

fn runtime_opaque_matches_reference_predicate(
    opaque: &RuntimeOpaqueValue,
    state: &RuntimeExecutionState,
    predicate: &impl Fn(&RuntimeReferenceValue) -> bool,
) -> bool {
    match opaque {
        RuntimeOpaqueValue::ReadView(handle) => state.read_views.get(handle).is_some_and(|view| {
            matches!(
                &view.backing,
                RuntimeElementViewBacking::Reference(reference) if predicate(reference)
            )
        }),
        RuntimeOpaqueValue::EditView(handle) => state.edit_views.get(handle).is_some_and(|view| {
            matches!(
                &view.backing,
                RuntimeElementViewBacking::Reference(reference) if predicate(reference)
            )
        }),
        RuntimeOpaqueValue::ByteView(handle) => state.byte_views.get(handle).is_some_and(|view| {
            matches!(
                &view.backing,
                RuntimeByteViewBacking::Reference(reference) if predicate(reference)
            )
        }),
        RuntimeOpaqueValue::ByteEditView(handle) => {
            state.byte_edit_views.get(handle).is_some_and(|view| {
                matches!(
                    &view.backing,
                    RuntimeByteViewBacking::Reference(reference) if predicate(reference)
                )
            })
        }
        RuntimeOpaqueValue::StrView(handle) => state.str_views.get(handle).is_some_and(|view| {
            matches!(
                &view.backing,
                RuntimeStrViewBacking::Reference(reference) if predicate(reference)
            )
        }),
        _ => false,
    }
}

fn runtime_opaque_matches_element_buffer(
    opaque: &RuntimeOpaqueValue,
    state: &RuntimeExecutionState,
    buffer: RuntimeElementViewBufferHandle,
) -> bool {
    match opaque {
        RuntimeOpaqueValue::ReadView(handle) => state.read_views.get(handle).is_some_and(|view| {
            matches!(&view.backing, RuntimeElementViewBacking::Buffer(backing) if *backing == buffer)
        }),
        RuntimeOpaqueValue::EditView(handle) => state.edit_views.get(handle).is_some_and(|view| {
            matches!(&view.backing, RuntimeElementViewBacking::Buffer(backing) if *backing == buffer)
        }),
        _ => false,
    }
}

fn runtime_ring_window_active_slots(slots: &[u64], start: usize, len: usize) -> Option<&[u64]> {
    slots.get(start..start.checked_add(len)?)
}

fn runtime_ring_window_backing_matches_predicate(
    backing: &RuntimeElementViewBacking,
    start: usize,
    len: usize,
    predicate: &impl Fn(RuntimeRingBufferHandle, &[u64]) -> bool,
) -> bool {
    match backing {
        RuntimeElementViewBacking::RingWindow { arena, slots } => {
            runtime_ring_window_active_slots(slots, start, len)
                .is_some_and(|active| predicate(*arena, active))
        }
        RuntimeElementViewBacking::Buffer(_) | RuntimeElementViewBacking::Reference(_) => false,
    }
}

fn runtime_opaque_matches_ring_window_predicate(
    opaque: &RuntimeOpaqueValue,
    state: &RuntimeExecutionState,
    predicate: &impl Fn(RuntimeRingBufferHandle, &[u64]) -> bool,
) -> bool {
    match opaque {
        RuntimeOpaqueValue::ReadView(handle) => state.read_views.get(handle).is_some_and(|view| {
            runtime_ring_window_backing_matches_predicate(
                &view.backing,
                view.start,
                view.len,
                predicate,
            )
        }),
        RuntimeOpaqueValue::EditView(handle) => state.edit_views.get(handle).is_some_and(|view| {
            runtime_ring_window_backing_matches_predicate(
                &view.backing,
                view.start,
                view.len,
                predicate,
            )
        }),
        _ => false,
    }
}

fn runtime_ring_window_overlaps_slots(
    arena: RuntimeRingBufferHandle,
    slots: &[u64],
    candidate_arena: RuntimeRingBufferHandle,
    candidate_slots: &[u64],
) -> bool {
    arena == candidate_arena
        && candidate_slots
            .iter()
            .any(|slot| slots.iter().any(|candidate| candidate == slot))
}

fn runtime_opaque_matches_byte_buffer(
    opaque: &RuntimeOpaqueValue,
    state: &RuntimeExecutionState,
    buffer: RuntimeByteViewBufferHandle,
) -> bool {
    match opaque {
        RuntimeOpaqueValue::ByteView(handle) => {
            state.byte_views.get(handle).is_some_and(|view| {
                matches!(&view.backing, RuntimeByteViewBacking::Buffer(backing) if *backing == buffer)
            })
        }
        RuntimeOpaqueValue::ByteEditView(handle) => {
            state.byte_edit_views.get(handle).is_some_and(|view| {
                matches!(&view.backing, RuntimeByteViewBacking::Buffer(backing) if *backing == buffer)
            })
        }
        _ => false,
    }
}

fn runtime_opaque_matches_foreign_byte_handle(
    opaque: &RuntimeOpaqueValue,
    state: &RuntimeExecutionState,
    package_id: &str,
    handle: u64,
) -> bool {
    match opaque {
        RuntimeOpaqueValue::ByteView(view_handle) => {
            state.byte_views.get(view_handle).is_some_and(|view| {
                matches!(
                    &view.backing,
                    RuntimeByteViewBacking::Foreign(backing)
                        if backing.package_id == package_id && backing.handle == handle
                )
            })
        }
        RuntimeOpaqueValue::ByteEditView(view_handle) => {
            state.byte_edit_views.get(view_handle).is_some_and(|view| {
                matches!(
                    &view.backing,
                    RuntimeByteViewBacking::Foreign(backing)
                        if backing.package_id == package_id && backing.handle == handle
                )
            })
        }
        _ => false,
    }
}

fn runtime_value_contains_reference_or_opaque_conflict(
    value: &RuntimeValue,
    state: &RuntimeExecutionState,
    reference_predicate: &impl Fn(&RuntimeReferenceValue) -> bool,
    opaque_predicate: &impl Fn(&RuntimeOpaqueValue, &RuntimeExecutionState) -> bool,
    ignored_reference: Option<&RuntimeReferenceValue>,
    ignored_opaque: Option<RuntimeOpaqueValue>,
) -> bool {
    match value {
        RuntimeValue::Bytes(_)
        | RuntimeValue::ByteBuffer(_)
        | RuntimeValue::Utf16(_)
        | RuntimeValue::Utf16Buffer(_) => false,
        RuntimeValue::Ref(reference) => {
            !ignored_reference.is_some_and(|ignored| ignored == reference)
                && reference_predicate(reference)
        }
        RuntimeValue::Opaque(opaque) => {
            ignored_opaque != Some(*opaque) && opaque_predicate(opaque, state)
        }
        RuntimeValue::Pair(left, right) => {
            runtime_value_contains_reference_or_opaque_conflict(
                left,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            ) || runtime_value_contains_reference_or_opaque_conflict(
                right,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        }
        RuntimeValue::Array(values) | RuntimeValue::List(values) => values.iter().any(|value| {
            runtime_value_contains_reference_or_opaque_conflict(
                value,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        }),
        RuntimeValue::Map(entries) => entries.iter().any(|(key, value)| {
            runtime_value_contains_reference_or_opaque_conflict(
                key,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            ) || runtime_value_contains_reference_or_opaque_conflict(
                value,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        }),
        RuntimeValue::Record { fields, .. } => fields.values().any(|value| {
            runtime_value_contains_reference_or_opaque_conflict(
                value,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        }),
        RuntimeValue::Variant { payload, .. } => payload.iter().any(|value| {
            runtime_value_contains_reference_or_opaque_conflict(
                value,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        }),
        RuntimeValue::Int(_)
        | RuntimeValue::Float { .. }
        | RuntimeValue::Bool(_)
        | RuntimeValue::Str(_)
        | RuntimeValue::OwnerHandle(_)
        | RuntimeValue::Range { .. }
        | RuntimeValue::Unit => false,
    }
}

fn runtime_scopes_contain_reference_or_opaque_conflict(
    scopes: &[RuntimeScope],
    state: &RuntimeExecutionState,
    reference_predicate: &impl Fn(&RuntimeReferenceValue) -> bool,
    opaque_predicate: &impl Fn(&RuntimeOpaqueValue, &RuntimeExecutionState) -> bool,
    ignored_reference: Option<&RuntimeReferenceValue>,
    ignored_opaque: Option<RuntimeOpaqueValue>,
    final_args: Option<&[RuntimeValue]>,
) -> bool {
    scopes.iter().any(|scope| {
        scope.locals.values().any(|local| {
            runtime_value_contains_reference_or_opaque_conflict(
                &local.value,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        })
    }) || final_args.is_some_and(|args| {
        args.iter().any(|value| {
            runtime_value_contains_reference_or_opaque_conflict(
                value,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        })
    }) || state.captured_local_values.values().any(|value| {
        runtime_value_contains_reference_or_opaque_conflict(
            value,
            state,
            reference_predicate,
            opaque_predicate,
            ignored_reference,
            ignored_opaque,
        )
    }) || state.cleanup_footer_frames.iter().any(|frame| {
        frame.activations.iter().any(|binding| {
            runtime_value_contains_reference_or_opaque_conflict(
                &binding.value,
                state,
                reference_predicate,
                opaque_predicate,
                ignored_reference,
                ignored_opaque,
            )
        })
    })
}

fn runtime_reject_live_reference_or_opaque_conflict(
    scopes: Option<&[RuntimeScope]>,
    final_args: Option<&[RuntimeValue]>,
    state: &RuntimeExecutionState,
    reference_predicate: impl Fn(&RuntimeReferenceValue) -> bool,
    opaque_predicate: impl Fn(&RuntimeOpaqueValue, &RuntimeExecutionState) -> bool,
    ignored_reference: Option<&RuntimeReferenceValue>,
    ignored_opaque: Option<RuntimeOpaqueValue>,
    fallback_conflict: impl Fn(&RuntimeExecutionState) -> bool,
    message: String,
) -> Result<(), String> {
    if let Some(scopes) = scopes
        && runtime_scopes_contain_reference_or_opaque_conflict(
            scopes,
            state,
            &reference_predicate,
            &opaque_predicate,
            ignored_reference,
            ignored_opaque,
            final_args,
        )
    {
        return Err(message);
    }
    if fallback_conflict(state) {
        return Err(message);
    }
    Ok(())
}

fn runtime_reference_descriptor_export_target(
    reference: &RuntimeReferenceValue,
    state: &RuntimeExecutionState,
) -> Result<Option<RuntimeExportedDescriptorTarget>, String> {
    match &reference.target {
        RuntimeReferenceTarget::SessionSlot { id, .. } => {
            let arena = state
                .session_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", id.arena.0))?;
            if !session_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SessionId `{}` for SessionArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            Ok(arena
                .sealed
                .then_some(RuntimeExportedDescriptorTarget::SessionArena(id.arena)))
        }
        RuntimeReferenceTarget::SlabSlot { id, .. } => {
            let arena = state
                .slabs
                .get(&id.arena)
                .ok_or_else(|| format!("invalid Slab handle `{}`", id.arena.0))?;
            if !slab_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(*id))),
                    id.arena.0
                ));
            }
            Ok(arena
                .sealed
                .then_some(RuntimeExportedDescriptorTarget::Slab(id.arena)))
        }
        _ => Ok(None),
    }
}

fn runtime_reference_backed_descriptor_view_allowed(
    reference: &RuntimeReferenceValue,
    state: &RuntimeExecutionState,
) -> Result<bool, String> {
    Ok(runtime_reference_descriptor_export_target(reference, state)?.is_some())
}

fn runtime_reference_root_value(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    target: &RuntimeReferenceTarget,
    access_mode: RuntimeReferenceMode,
) -> Result<RuntimeValue, String> {
    match target {
        RuntimeReferenceTarget::Local { local, .. } => {
            if let Some((name, runtime_local)) = lookup_local_with_name_by_handle(scopes, *local) {
                if runtime_local.moved {
                    return Err(format!("use of moved local `{name}`"));
                }
                if runtime_local.take_reserved && access_mode != RuntimeReferenceMode::Take {
                    return Err(format!(
                        "local `{name}` is reserved by an active `&take` capability"
                    ));
                }
                if runtime_local.held && access_mode != RuntimeReferenceMode::Hold {
                    return Err(format!(
                        "local `{name}` is suspended by an active `&hold` capability"
                    ));
                }
                return Ok(state
                    .captured_local_values
                    .get(local)
                    .cloned()
                    .unwrap_or_else(|| runtime_local.value.clone()));
            }
            state
                .captured_local_values
                .get(local)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "runtime reference local `{}` is unresolved; visible locals: {}",
                        local.0,
                        runtime_scope_local_summary(scopes)
                    )
                })
        }
        RuntimeReferenceTarget::OwnerObject {
            owner_key,
            object_name,
            ..
        } => owner_object_root_value(
            scopes,
            plan,
            current_package_id,
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
        RuntimeReferenceTarget::TempSlot { id, .. } => {
            let arena = state
                .temp_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", id.arena.0))?;
            if !temp_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid TempId `{}` for TempArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(*id))),
                    id.arena.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("TempArena slot `{}` is missing", id.slot))
        }
        RuntimeReferenceTarget::SessionSlot { id, .. } => {
            let arena = state
                .session_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", id.arena.0))?;
            if !session_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SessionId `{}` for SessionArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("SessionArena slot `{}` is missing", id.slot))
        }
        RuntimeReferenceTarget::RingSlot { id, .. } => {
            let arena = state
                .ring_buffers
                .get(&id.arena)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", id.arena.0))?;
            if !ring_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid RingId `{}` for RingBuffer `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(*id))),
                    id.arena.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("RingBuffer slot `{}` is missing", id.slot))
        }
        RuntimeReferenceTarget::SlabSlot { id, .. } => {
            let arena = state
                .slabs
                .get(&id.arena)
                .ok_or_else(|| format!("invalid Slab handle `{}`", id.arena.0))?;
            if !slab_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(*id))),
                    id.arena.0
                ));
            }
            arena
                .slots
                .get(&id.slot)
                .cloned()
                .ok_or_else(|| format!("Slab slot `{}` is missing", id.slot))
        }
    }
}

fn assign_member_chain(
    plan: &RuntimePackagePlan,
    base: RuntimeValue,
    members: &[String],
    value: RuntimeValue,
) -> Result<RuntimeValue, String> {
    let Some((member, rest)) = members.split_first() else {
        return Ok(value);
    };
    if rest.is_empty() {
        return assign_record_member(plan, base, member, value);
    }
    let child = eval_member_value(base.clone(), member)?;
    let updated_child = assign_member_chain(plan, child, rest, value)?;
    assign_record_member(plan, base, member, updated_child)
}

fn read_runtime_reference(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    if let Some(value) = runtime_reference_value_ref(scopes.as_slice(), state, reference)? {
        return Ok(value.clone());
    }
    let mut value = runtime_reference_root_value(
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
        &reference.target,
        reference.mode.clone(),
    )?;
    for member in runtime_reference_members(&reference.target) {
        value = eval_member_value(value, member)?;
    }
    Ok(value)
}

fn write_runtime_reference(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    value: RuntimeValue,
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    if !reference.mode.allows_write() {
        return Err(format!(
            "runtime reference mode `{}` does not allow mutation",
            reference.mode.as_str()
        ));
    }
    let members = runtime_reference_members(&reference.target);
    let updated_root = if members.is_empty() {
        if matches!(reference.target, RuntimeReferenceTarget::OwnerObject { .. }) {
            let _ = runtime_reference_root_value(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
                &reference.target,
                reference.mode.clone(),
            )?;
        }
        value
    } else {
        let root = runtime_reference_root_value(
            scopes,
            plan,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
            &reference.target,
            reference.mode.clone(),
        )?;
        assign_member_chain(plan, root, members, value)?
    };
    match &reference.target {
        RuntimeReferenceTarget::Local { local, .. } => {
            let visible_locals = runtime_scope_local_summary(scopes);
            if let Some(runtime_local) = lookup_local_mut_by_handle(scopes, *local) {
                runtime_local.moved = false;
                runtime_local.value = updated_root.clone();
                update_runtime_cleanup_footer_binding_value(state, *local, &runtime_local.value);
                state.captured_local_values.insert(*local, updated_root);
                Ok(())
            } else if let Some(captured) = state.captured_local_values.get_mut(local) {
                *captured = updated_root;
                Ok(())
            } else {
                Err(format!(
                    "runtime reference local `{}` is unresolved; visible locals: {}",
                    local.0, visible_locals
                ))
            }
        }
        RuntimeReferenceTarget::OwnerObject {
            owner_key,
            object_name,
            ..
        } => {
            let owner_state = state.owners.entry(owner_key.clone()).or_default();
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
                current_package_id,
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
        RuntimeReferenceTarget::TempSlot { id, .. } => {
            let arena = state
                .temp_arenas
                .get_mut(&id.arena)
                .ok_or_else(|| format!("invalid TempArena handle `{}`", id.arena.0))?;
            if !temp_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid TempId `{}` for TempArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(*id))),
                    id.arena.0
                ));
            }
            arena.slots.insert(id.slot, updated_root);
            Ok(())
        }
        RuntimeReferenceTarget::SessionSlot { id, .. } => {
            let arena = state
                .session_arenas
                .get_mut(&id.arena)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", id.arena.0))?;
            if !session_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SessionId `{}` for SessionArena `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(
                        *id
                    ))),
                    id.arena.0
                ));
            }
            arena.slots.insert(id.slot, updated_root);
            Ok(())
        }
        RuntimeReferenceTarget::RingSlot { id, .. } => {
            let arena = state
                .ring_buffers
                .get_mut(&id.arena)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", id.arena.0))?;
            if !ring_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid RingId `{}` for RingBuffer `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(*id))),
                    id.arena.0
                ));
            }
            arena.slots.insert(id.slot, updated_root);
            Ok(())
        }
        RuntimeReferenceTarget::SlabSlot { id, .. } => {
            let arena = state
                .slabs
                .get_mut(&id.arena)
                .ok_or_else(|| format!("invalid Slab handle `{}`", id.arena.0))?;
            if !slab_id_is_live(id.arena, arena, *id) {
                return Err(format!(
                    "stale or invalid SlabId `{}` for Slab `{}`",
                    runtime_value_to_string(&RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(*id))),
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

fn expect_float(value: RuntimeValue, context: &str) -> Result<(ParsedFloatKind, f64), String> {
    match value {
        RuntimeValue::Float { text, kind } => Ok((kind, parse_runtime_float_text(&text, kind)?)),
        other => Err(format!("{context} expected float, got `{other:?}`")),
    }
}

fn expect_same_float_operands(
    left: RuntimeValue,
    right: RuntimeValue,
    context: &str,
) -> Result<(ParsedFloatKind, f64, f64), String> {
    let (left_kind, left_value) = expect_float(left, context)?;
    let (right_kind, right_value) = expect_float(right, context)?;
    if left_kind != right_kind {
        return Err(format!(
            "{context} expected float operands of the same type, found `{left_kind:?}` and `{right_kind:?}`"
        ));
    }
    Ok((left_kind, left_value, right_value))
}

fn runtime_values_equal(left: &RuntimeValue, right: &RuntimeValue) -> bool {
    match (left, right) {
        (
            RuntimeValue::Float {
                text: left_text,
                kind: left_kind,
            },
            RuntimeValue::Float {
                text: right_text,
                kind: right_kind,
            },
        ) if left_kind == right_kind => parse_runtime_float_text(left_text, *left_kind)
            .ok()
            .zip(parse_runtime_float_text(right_text, *right_kind).ok())
            .is_some_and(|(left_value, right_value)| left_value == right_value),
        _ => left == right,
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
    match value {
        RuntimeValue::Bytes(bytes) => Ok(bytes),
        RuntimeValue::ByteBuffer(bytes) => Ok(bytes),
        RuntimeValue::Array(values) => values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let byte = expect_int(value, context)?;
                u8::try_from(byte).map_err(|_| {
                    format!("{context} byte index `{index}` is out of range 0..255: `{byte}`")
                })
            })
            .collect(),
        _other => Err(format!("{context} expected Bytes")),
    }
}

fn expect_utf16_units(value: RuntimeValue, context: &str) -> Result<Vec<u16>, String> {
    match value {
        RuntimeValue::Utf16(units) => Ok(units),
        RuntimeValue::Utf16Buffer(units) => Ok(units),
        RuntimeValue::Array(values) => values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let unit = expect_int(value, context)?;
                u16::try_from(unit).map_err(|_| {
                    format!("{context} utf16 index `{index}` is out of range 0..65535: `{unit}`")
                })
            })
            .collect(),
        _other => Err(format!("{context} expected Utf16")),
    }
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

fn std_types_core_monotonic_time_ms_record(value: i64) -> RuntimeValue {
    let mut fields = BTreeMap::new();
    fields.insert("value".to_string(), RuntimeValue::Int(value));
    RuntimeValue::Record {
        name: "std.types.core.MonotonicTimeMs".to_string(),
        fields,
    }
}

fn runtime_bundle_manifest_candidates(bundle_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let primary = bundle_dir.join("arcana.bundle.toml");
    if primary.is_file() {
        return Ok(vec![primary]);
    }
    let mut manifests = fs::read_dir(bundle_dir)
        .map_err(|e| {
            format!(
                "failed to read bundle directory `{}`: {e}",
                bundle_dir.display()
            )
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".arcana-bundle.toml"))
        })
        .collect::<Vec<_>>();
    manifests.sort();
    Ok(manifests)
}

fn runtime_resolve_manifest_override_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        return Ok(normalize_lexical_path(&path));
    }
    let cwd = std::env::current_dir()
        .map_err(|err| format!("failed to resolve current directory: {err}"))?;
    Ok(normalize_lexical_path(
        &normalize_lexical_path(&cwd).join(path),
    ))
}

fn load_runtime_native_products() -> Result<RuntimeNativeProductCatalog, String> {
    let manifest_override = std::env::var(ARCANA_NATIVE_BUNDLE_MANIFEST_ENV).unwrap_or_default();
    if !manifest_override.is_empty() {
        let manifest_path = runtime_resolve_manifest_override_path(&manifest_override)?;
        if !manifest_path.is_file() {
            return Err(format!(
                "native bundle manifest override `{}` does not exist",
                manifest_path.display()
            ));
        }
        return load_bundle_native_products_from_manifest_path(&manifest_path);
    }
    let bundle_override = std::env::var(ARCANA_NATIVE_BUNDLE_DIR_ENV).unwrap_or_default();
    if !bundle_override.is_empty() {
        let bundle_dir = runtime_resolve_manifest_override_path(&bundle_override)?;
        let manifests = runtime_bundle_manifest_candidates(&bundle_dir)?;
        return match manifests.as_slice() {
            [manifest] => load_bundle_native_products_from_manifest_path(manifest),
            [] => load_bundle_native_products(&bundle_dir),
            _ => Err(format!(
                "bundle override `{}` has multiple candidate manifests; set `{ARCANA_NATIVE_BUNDLE_MANIFEST_ENV}` explicitly",
                bundle_dir.display()
            )),
        };
    }
    let cwd = std::env::current_dir()
        .map(|path| normalize_lexical_path(&path))
        .map_err(|err| format!("failed to resolve current directory: {err}"))?;
    let cwd_manifests = runtime_bundle_manifest_candidates(&cwd)?;
    match cwd_manifests.as_slice() {
        [manifest] => load_bundle_native_products_from_manifest_path(manifest),
        [] => load_current_bundle_native_products(),
        _ => Err(format!(
            "working directory `{}` has multiple candidate native bundle manifests; set `{ARCANA_NATIVE_BUNDLE_MANIFEST_ENV}` explicitly",
            cwd.display()
        )),
    }
}

fn reset_runtime_native_products_cache() {
    ACTIVE_RUNTIME_NATIVE_PRODUCTS.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn with_runtime_native_products<R>(
    action: impl FnOnce(&mut RuntimeNativeProductCatalog) -> Result<R, String>,
) -> Result<R, String> {
    ACTIVE_RUNTIME_NATIVE_PRODUCTS.with(|slot| {
        if slot.borrow().is_none() {
            let catalog = load_runtime_native_products()?;
            *slot.borrow_mut() = Some(catalog);
        }
        let mut borrow = slot.borrow_mut();
        let catalog = borrow
            .as_mut()
            .ok_or_else(|| "runtime native product catalog is not available".to_string())?;
        action(catalog)
    })
}

fn runtime_short_package_id_hash(package_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"arcana_package_id_v1\n");
    hasher.update(package_id.as_bytes());
    format!("{:x}", hasher.finalize())
        .chars()
        .take(12)
        .collect()
}

fn runtime_default_package_asset_root(package_id: &str) -> String {
    format!(
        "package-assets/{}",
        runtime_short_package_id_hash(package_id)
    )
}

fn runtime_current_package_asset_root(current_package_id: &str) -> Result<PathBuf, String> {
    match with_runtime_native_products(|catalog| {
        if let Some(root) = catalog.package_asset_root(current_package_id) {
            let path = catalog.bundle_dir().join(root);
            if path.exists() {
                return Ok(normalize_lexical_path(&path));
            }
        }
        let fallback_bundle = normalize_lexical_path(
            &catalog
                .bundle_dir()
                .join(runtime_default_package_asset_root(current_package_id)),
        );
        if fallback_bundle.exists() {
            return Ok(fallback_bundle);
        }
        Err(String::new())
    }) {
        Ok(path) => return Ok(path),
        Err(err) if !err.is_empty() => return Err(err),
        Err(_) => {}
    }
    let cwd = std::env::current_dir()
        .map(|path| normalize_lexical_path(&path))
        .map_err(|err| format!("failed to resolve current directory: {err}"))?;
    let fallback =
        normalize_lexical_path(&cwd.join(runtime_default_package_asset_root(current_package_id)));
    if fallback.exists() {
        return Ok(fallback);
    }
    Err(format!(
        "package assets for `{current_package_id}` are not staged under `{}`",
        fallback.display()
    ))
}

fn runtime_process_file_stream_value(handle: u64) -> RuntimeValue {
    RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(RuntimeBindingOpaqueValue {
        package_id: "arcana_winapi",
        type_name: "arcana_winapi.process_handles.FileStream",
        handle,
    }))
}

fn expect_process_file_stream_handle(value: RuntimeValue, context: &str) -> Result<u64, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) = value else {
        return Err(format!(
            "{context} expected arcana_winapi.process_handles.FileStream"
        ));
    };
    if binding.type_name != "arcana_winapi.process_handles.FileStream" {
        return Err(format!(
            "{context} expected arcana_winapi.process_handles.FileStream, got `{}`",
            binding.type_name
        ));
    }
    if binding.handle == 0 {
        return Err("FileStream handle must not be 0".to_string());
    }
    Ok(binding.handle)
}

fn bind_runtime_direct_call_args(
    callable: &[String],
    call_args: &[RuntimeCallArg],
    take_arg_indices: &[usize],
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<Vec<RuntimeCallArg>, String> {
    let mut bound = bind_call_args_for_intrinsic(callable, call_args.to_vec())?;
    consume_take_call_args(scopes, take_arg_indices, &bound)?;
    for arg in &mut bound {
        arg.value = read_runtime_value_if_ref(
            arg.value.clone(),
            scopes,
            plan,
            current_package_id,
            current_module_id,
            &BTreeMap::new(),
            &BTreeMap::new(),
            state,
            host,
        )?;
    }
    Ok(bound)
}

fn runtime_exec_capture_record(capture: RuntimeProcessCapture) -> RuntimeValue {
    let mut fields = BTreeMap::new();
    fields.insert("status".to_string(), RuntimeValue::Int(capture.status));
    fields.insert(
        "output".to_string(),
        make_pair(
            RuntimeValue::Bytes(capture.stdout),
            RuntimeValue::Bytes(capture.stderr),
        ),
    );
    fields.insert(
        "utf8".to_string(),
        make_pair(
            RuntimeValue::Bool(capture.stdout_utf8),
            RuntimeValue::Bool(capture.stderr_utf8),
        ),
    );
    RuntimeValue::Record {
        name: "arcana_process.process.ExecCapture".to_string(),
        fields,
    }
}

fn runtime_arcana_owned_callable_key(callable: &[String]) -> Option<String> {
    let key = match callable {
        [std_name, io_name, name] if std_name == "std" && io_name == "io" => {
            format!("std.io.{name}")
        }
        [module, name]
            if matches!(
                module.as_str(),
                "io" | "args" | "env" | "path" | "fs" | "process"
            ) =>
        {
            format!("arcana_process.{module}.{name}")
        }
        _ => callable.join("."),
    };
    matches!(
        key.as_str(),
        "std.io.print"
            | "std.io.eprint"
            | "std.io.flush_stdout"
            | "std.io.flush_stderr"
            | "std.io.read_line"
            | "arcana_process.io.print"
            | "arcana_process.io.print_line"
            | "arcana_process.io.eprint"
            | "arcana_process.io.eprint_line"
            | "arcana_process.io.flush_stdout"
            | "arcana_process.io.flush_stderr"
            | "arcana_process.io.read_line"
            | "arcana_process.args.count"
            | "arcana_process.args.get"
            | "arcana_process.env.has"
            | "arcana_process.env.get"
            | "arcana_process.env.get_or"
            | "arcana_process.path.cwd"
            | "arcana_process.path.join"
            | "arcana_process.path.normalize"
            | "arcana_process.path.parent"
            | "arcana_process.path.file_name"
            | "arcana_process.path.ext"
            | "arcana_process.path.is_absolute"
            | "arcana_process.path.stem"
            | "arcana_process.path.with_ext"
            | "arcana_process.path.relative_to"
            | "arcana_process.path.canonicalize"
            | "arcana_process.path.strip_prefix"
            | "arcana_process.fs.exists"
            | "arcana_process.fs.is_file"
            | "arcana_process.fs.is_dir"
            | "arcana_process.fs.read_text"
            | "arcana_process.fs.read_bytes"
            | "arcana_process.fs.write_text"
            | "arcana_process.fs.write_bytes"
            | "arcana_process.fs.stream_open_read"
            | "arcana_process.fs.stream_open_write"
            | "arcana_process.fs.stream_read"
            | "arcana_process.fs.stream_write"
            | "arcana_process.fs.stream_eof"
            | "arcana_process.fs.stream_close"
            | "arcana_process.fs.list_dir"
            | "arcana_process.fs.mkdir_all"
            | "arcana_process.fs.create_dir"
            | "arcana_process.fs.remove_file"
            | "arcana_process.fs.remove_dir"
            | "arcana_process.fs.remove_dir_all"
            | "arcana_process.fs.copy_file"
            | "arcana_process.fs.rename"
            | "arcana_process.fs.file_size"
            | "arcana_process.fs.modified_unix_ms"
            | "arcana_process.process.exec_status"
            | "arcana_process.process.exec_capture"
    )
    .then_some(key)
}

fn try_execute_arcana_owned_api_call(
    callable: &[String],
    call_args: &[RuntimeCallArg],
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    _aliases: &BTreeMap<String, Vec<String>>,
    _type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<Option<RuntimeValue>, String> {
    let Some(key) = runtime_arcana_owned_callable_key(callable) else {
        return Ok(None);
    };
    let value = match key.as_str() {
        "std.io.print" | "arcana_process.io.print" | "arcana_process.io.print_line" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            host.print(&runtime_value_to_string(&args[0].value))?;
            if key == "arcana_process.io.print_line" {
                host.print("\n")?;
            }
            RuntimeValue::Unit
        }
        "std.io.eprint" | "arcana_process.io.eprint" | "arcana_process.io.eprint_line" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            host.eprint(&runtime_value_to_string(&args[0].value))?;
            if key == "arcana_process.io.eprint_line" {
                host.eprint("\n")?;
            }
            RuntimeValue::Unit
        }
        "std.io.flush_stdout" | "arcana_process.io.flush_stdout" => {
            bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            host.flush_stdout()?;
            RuntimeValue::Unit
        }
        "std.io.flush_stderr" | "arcana_process.io.flush_stderr" => {
            bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            host.flush_stderr()?;
            RuntimeValue::Unit
        }
        "std.io.read_line" | "arcana_process.io.read_line" => {
            bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            match host.stdin_read_line() {
                Ok(line) => ok_variant(RuntimeValue::Str(line)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.args.count" => {
            bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            RuntimeValue::Int(host.runtime_arg_count()?)
        }
        "arcana_process.args.get" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            RuntimeValue::Str(host.runtime_arg_get(expect_int(
                args[0].value.clone(),
                "arcana_process.args.get",
            )?)?)
        }
        "arcana_process.env.has" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            RuntimeValue::Bool(host.runtime_env_has(&expect_str(
                args[0].value.clone(),
                "arcana_process.env.has",
            )?)?)
        }
        "arcana_process.env.get" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            RuntimeValue::Str(host.runtime_env_get(&expect_str(
                args[0].value.clone(),
                "arcana_process.env.get",
            )?)?)
        }
        "arcana_process.env.get_or" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let name = expect_str(args[0].value.clone(), "arcana_process.env.get_or")?;
            let fallback = expect_str(args[1].value.clone(), "arcana_process.env.get_or")?;
            RuntimeValue::Str(if host.runtime_env_has(&name)? {
                host.runtime_env_get(&name)?
            } else {
                fallback
            })
        }
        "arcana_process.path.cwd" => {
            bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            RuntimeValue::Str(runtime_path_string(&host.runtime_current_working_dir()?))
        }
        "arcana_process.path.join" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let a = expect_str(args[0].value.clone(), "arcana_process.path.join")?;
            let b = expect_str(args[1].value.clone(), "arcana_process.path.join")?;
            RuntimeValue::Str(runtime_path_string(&normalize_lexical_path(
                &Path::new(&a).join(b),
            )))
        }
        "arcana_process.path.normalize" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.normalize")?;
            RuntimeValue::Str(runtime_path_string(&normalize_lexical_path(Path::new(
                &path,
            ))))
        }
        "arcana_process.path.parent" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.parent")?;
            RuntimeValue::Str(
                Path::new(&path)
                    .parent()
                    .map(runtime_path_string)
                    .unwrap_or_default(),
            )
        }
        "arcana_process.path.file_name" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.file_name")?;
            RuntimeValue::Str(
                Path::new(&path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
            )
        }
        "arcana_process.path.ext" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.ext")?;
            RuntimeValue::Str(
                Path::new(&path)
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string())
                    .unwrap_or_default(),
            )
        }
        "arcana_process.path.is_absolute" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.is_absolute")?;
            RuntimeValue::Bool(Path::new(&path).is_absolute())
        }
        "arcana_process.path.stem" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.stem")?;
            match Path::new(&path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .ok_or_else(|| format!("path `{path}` has no stem"))
            {
                Ok(value) => ok_variant(RuntimeValue::Str(value)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.path.with_ext" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.with_ext")?;
            let ext = expect_str(args[1].value.clone(), "arcana_process.path.with_ext")?;
            let mut updated = PathBuf::from(path);
            updated.set_extension(ext);
            RuntimeValue::Str(runtime_path_string(&updated))
        }
        "arcana_process.path.relative_to" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.relative_to")?;
            let base = expect_str(args[1].value.clone(), "arcana_process.path.relative_to")?;
            match runtime_relative_path(Path::new(&path), Path::new(&base))
                .map(|value| runtime_path_string(&value))
            {
                Ok(value) => ok_variant(RuntimeValue::Str(value)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.path.canonicalize" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), "arcana_process.path.canonicalize")?;
            match host.runtime_path_canonicalize(&path) {
                Ok(value) => ok_variant(RuntimeValue::Str(value)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.path.strip_prefix" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = normalize_lexical_path(Path::new(&expect_str(
                args[0].value.clone(),
                "arcana_process.path.strip_prefix",
            )?));
            let prefix = normalize_lexical_path(Path::new(&expect_str(
                args[1].value.clone(),
                "arcana_process.path.strip_prefix",
            )?));
            match path
                .strip_prefix(&prefix)
                .map(runtime_path_string)
                .map_err(|_| {
                    format!(
                        "path `{}` does not start with `{}`",
                        runtime_path_string(&path),
                        runtime_path_string(&prefix)
                    )
                }) {
                Ok(value) => ok_variant(RuntimeValue::Str(value)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.exists" | "arcana_process.fs.is_file" | "arcana_process.fs.is_dir" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            let result = match key.as_str() {
                "arcana_process.fs.exists" => host.runtime_fs_exists(&path),
                "arcana_process.fs.is_file" => host.runtime_fs_is_file(&path),
                _ => host.runtime_fs_is_dir(&path),
            }?;
            RuntimeValue::Bool(result)
        }
        "arcana_process.fs.read_text" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            match host.runtime_fs_read_text(&path) {
                Ok(text) => ok_variant(RuntimeValue::Str(text)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.read_bytes" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            match host.runtime_fs_read_bytes(&path) {
                Ok(bytes) => ok_variant(RuntimeValue::Bytes(bytes)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.write_text" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            let text = expect_str(args[1].value.clone(), key.as_str())?;
            match host.runtime_fs_write_text(&path, &text) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.write_bytes" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            let bytes = expect_byte_array(args[1].value.clone(), key.as_str())?;
            match host.runtime_fs_write_bytes(&path, &bytes) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.stream_open_read" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            match host.runtime_fs_stream_open_read(&path) {
                Ok(handle) => {
                    if handle == 0 {
                        err_variant(
                            "runtime core host returned invalid FileStream handle `0`".to_string(),
                        )
                    } else {
                        ok_variant(runtime_process_file_stream_value(handle))
                    }
                }
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.stream_open_write" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            let append = expect_bool(args[1].value.clone(), key.as_str())?;
            match host.runtime_fs_stream_open_write(&path, append) {
                Ok(handle) => {
                    if handle == 0 {
                        err_variant(
                            "runtime core host returned invalid FileStream handle `0`".to_string(),
                        )
                    } else {
                        ok_variant(runtime_process_file_stream_value(handle))
                    }
                }
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.stream_read" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let handle = expect_process_file_stream_handle(
                args[0].value.clone(),
                "arcana_process.fs.stream_read",
            )?;
            let max_bytes = expect_int(args[1].value.clone(), key.as_str())?;
            let result = (|| -> Result<Vec<u8>, String> {
                if max_bytes < 0 {
                    return Err("fs_stream_read max_bytes must be non-negative".to_string());
                }
                host.runtime_fs_stream_read(handle, max_bytes as usize)
            })();
            match result {
                Ok(bytes) => ok_variant(RuntimeValue::Bytes(bytes)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.stream_write" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let handle = expect_process_file_stream_handle(
                args[0].value.clone(),
                "arcana_process.fs.stream_write",
            )?;
            let bytes = expect_byte_array(args[1].value.clone(), key.as_str())?;
            let result = host
                .runtime_fs_stream_write(handle, &bytes)
                .and_then(|written| {
                    i64::try_from(written).map_err(|_| {
                        "fs_stream_write wrote byte count that does not fit Int".to_string()
                    })
                });
            match result {
                Ok(written) => ok_variant(RuntimeValue::Int(written)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.stream_eof" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let handle = expect_process_file_stream_handle(
                args[0].value.clone(),
                "arcana_process.fs.stream_eof",
            )?;
            let result = host.runtime_fs_stream_eof(handle);
            match result {
                Ok(value) => ok_variant(RuntimeValue::Bool(value)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.stream_close" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[0],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let handle = expect_process_file_stream_handle(
                args[0].value.clone(),
                "arcana_process.fs.stream_close",
            )?;
            match host.runtime_fs_stream_close(handle) {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.list_dir" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            match host.runtime_fs_list_dir(&path) {
                Ok(entries) => ok_variant(RuntimeValue::List(
                    entries.into_iter().map(RuntimeValue::Str).collect(),
                )),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.mkdir_all"
        | "arcana_process.fs.create_dir"
        | "arcana_process.fs.remove_file"
        | "arcana_process.fs.remove_dir"
        | "arcana_process.fs.remove_dir_all" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            let result = match key.as_str() {
                "arcana_process.fs.mkdir_all" => host.runtime_fs_mkdir_all(&path),
                "arcana_process.fs.create_dir" => host.runtime_fs_create_dir(&path),
                "arcana_process.fs.remove_file" => host.runtime_fs_remove_file(&path),
                "arcana_process.fs.remove_dir" => host.runtime_fs_remove_dir(&path),
                _ => host.runtime_fs_remove_dir_all(&path),
            };
            match result {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.copy_file" | "arcana_process.fs.rename" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let from = expect_str(args[0].value.clone(), key.as_str())?;
            let to = expect_str(args[1].value.clone(), key.as_str())?;
            let result = match key.as_str() {
                "arcana_process.fs.copy_file" => host.runtime_fs_copy_file(&from, &to),
                _ => host.runtime_fs_rename(&from, &to),
            };
            match result {
                Ok(()) => ok_variant(RuntimeValue::Unit),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.fs.file_size" | "arcana_process.fs.modified_unix_ms" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let path = expect_str(args[0].value.clone(), key.as_str())?;
            let result = match key.as_str() {
                "arcana_process.fs.file_size" => host.runtime_fs_file_size(&path),
                _ => host.runtime_fs_modified_unix_ms(&path),
            };
            match result {
                Ok(value) => ok_variant(RuntimeValue::Int(value)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.process.exec_status" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let program = expect_str(args[0].value.clone(), key.as_str())?;
            let argv = expect_string_list(args[1].value.clone(), key.as_str())?;
            let result = host.runtime_process_exec_status(&program, &argv);
            match result {
                Ok(status) => ok_variant(RuntimeValue::Int(status)),
                Err(err) => err_variant(err),
            }
        }
        "arcana_process.process.exec_capture" => {
            let args = bind_runtime_direct_call_args(
                callable,
                call_args,
                &[],
                scopes,
                plan,
                current_package_id,
                current_module_id,
                state,
                host,
            )?;
            let program = expect_str(args[0].value.clone(), key.as_str())?;
            let argv = expect_string_list(args[1].value.clone(), key.as_str())?;
            let result = host.runtime_process_exec_capture(&program, &argv);
            match result {
                Ok(capture) => ok_variant(runtime_exec_capture_record(capture)),
                Err(err) => err_variant(err),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
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

fn expect_temp_arena(value: RuntimeValue, context: &str) -> Result<RuntimeTempArenaHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(handle)) = value else {
        return Err(format!("{context} expected TempArena"));
    };
    Ok(handle)
}

fn expect_temp_id(value: RuntimeValue, context: &str) -> Result<RuntimeTempIdValue, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(id)) = value else {
        return Err(format!("{context} expected TempId"));
    };
    Ok(id)
}

fn expect_session_arena(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeSessionArenaHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(handle)) = value else {
        return Err(format!("{context} expected SessionArena"));
    };
    Ok(handle)
}

fn expect_session_id(value: RuntimeValue, context: &str) -> Result<RuntimeSessionIdValue, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(id)) = value else {
        return Err(format!("{context} expected SessionId"));
    };
    Ok(id)
}

fn expect_ring_buffer(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeRingBufferHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)) = value else {
        return Err(format!("{context} expected RingBuffer"));
    };
    Ok(handle)
}

fn expect_ring_id(value: RuntimeValue, context: &str) -> Result<RuntimeRingIdValue, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(id)) = value else {
        return Err(format!("{context} expected RingId"));
    };
    Ok(id)
}

fn expect_slab(value: RuntimeValue, context: &str) -> Result<RuntimeSlabHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle)) = value else {
        return Err(format!("{context} expected Slab"));
    };
    Ok(handle)
}

fn expect_slab_id(value: RuntimeValue, context: &str) -> Result<RuntimeSlabIdValue, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id)) = value else {
        return Err(format!("{context} expected SlabId"));
    };
    Ok(id)
}

fn expect_edit_view(value: RuntimeValue, context: &str) -> Result<RuntimeEditViewHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) = value else {
        return Err(format!("{context} expected EditView"));
    };
    Ok(handle)
}

fn expect_byte_view(value: RuntimeValue, context: &str) -> Result<RuntimeByteViewHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) = value else {
        return Err(format!("{context} expected ByteView"));
    };
    Ok(handle)
}

fn expect_byte_edit_view(
    value: RuntimeValue,
    context: &str,
) -> Result<RuntimeByteEditViewHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(handle)) = value else {
        return Err(format!("{context} expected ByteEditView"));
    };
    Ok(handle)
}

fn expect_str_view(value: RuntimeValue, context: &str) -> Result<RuntimeStrViewHandle, String> {
    let RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) = value else {
        return Err(format!("{context} expected StrView"));
    };
    Ok(handle)
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

fn bytes_to_runtime_value(bytes: impl IntoIterator<Item = u8>) -> RuntimeValue {
    RuntimeValue::Bytes(bytes.into_iter().collect())
}

fn bytes_to_runtime_array(bytes: impl IntoIterator<Item = u8>) -> RuntimeValue {
    RuntimeValue::Array(
        bytes
            .into_iter()
            .map(|byte| RuntimeValue::Int(i64::from(byte)))
            .collect(),
    )
}

fn expect_runtime_array(value: RuntimeValue, context: &str) -> Result<Vec<RuntimeValue>, String> {
    let RuntimeValue::Array(values) = value else {
        return Err(format!("{context} expected Array"));
    };
    Ok(values)
}

fn runtime_string_slice(
    text: &str,
    start: usize,
    end: usize,
    context: &str,
) -> Result<String, String> {
    if start > end || end > text.len() {
        return Err(format!(
            "{context} slice `{start}..{end}` is out of bounds for {} bytes",
            text.len()
        ));
    }
    text.get(start..end)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{context} slice `{start}..{end}` is not on UTF-8 boundaries"))
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
    policy: RuntimeArenaPolicy,
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
            policy,
        },
    );
    handle
}

fn insert_runtime_frame_arena(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    policy: RuntimeFrameArenaPolicy,
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
            policy,
        },
    );
    handle
}

fn insert_runtime_pool_arena(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    policy: RuntimePoolArenaPolicy,
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
            policy,
        },
    );
    handle
}

fn insert_runtime_temp_arena(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    policy: RuntimeTempArenaPolicy,
) -> RuntimeTempArenaHandle {
    let handle = RuntimeTempArenaHandle(state.next_temp_arena_handle);
    state.next_temp_arena_handle += 1;
    state.temp_arenas.insert(
        handle,
        RuntimeTempArenaState {
            type_args: type_args.to_vec(),
            next_slot: 0,
            generation: 0,
            slots: BTreeMap::new(),
            policy,
        },
    );
    handle
}

fn insert_runtime_session_arena(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    policy: RuntimeSessionArenaPolicy,
) -> RuntimeSessionArenaHandle {
    let handle = RuntimeSessionArenaHandle(state.next_session_arena_handle);
    state.next_session_arena_handle += 1;
    state.session_arenas.insert(
        handle,
        RuntimeSessionArenaState {
            type_args: type_args.to_vec(),
            next_slot: 0,
            generation: 0,
            slots: BTreeMap::new(),
            policy,
            sealed: false,
        },
    );
    handle
}

fn insert_runtime_ring_buffer(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    policy: RuntimeRingBufferPolicy,
) -> RuntimeRingBufferHandle {
    let handle = RuntimeRingBufferHandle(state.next_ring_buffer_handle);
    state.next_ring_buffer_handle += 1;
    state.ring_buffers.insert(
        handle,
        RuntimeRingBufferState {
            type_args: type_args.to_vec(),
            next_slot: 0,
            generations: BTreeMap::new(),
            slots: BTreeMap::new(),
            order: VecDeque::new(),
            policy,
        },
    );
    handle
}

fn insert_runtime_slab(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    policy: RuntimeSlabPolicy,
) -> RuntimeSlabHandle {
    let handle = RuntimeSlabHandle(state.next_slab_handle);
    state.next_slab_handle += 1;
    state.slabs.insert(
        handle,
        RuntimeSlabState {
            type_args: type_args.to_vec(),
            next_slot: 0,
            free_slots: Vec::new(),
            generations: BTreeMap::new(),
            slots: BTreeMap::new(),
            policy,
            sealed: false,
        },
    );
    handle
}

fn insert_runtime_element_view_buffer(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    values: Vec<RuntimeValue>,
) -> RuntimeElementViewBufferHandle {
    let handle = RuntimeElementViewBufferHandle(state.next_element_view_buffer_handle);
    state.next_element_view_buffer_handle += 1;
    state.element_view_buffers.insert(
        handle,
        RuntimeElementViewBufferState {
            type_args: type_args.to_vec(),
            values,
        },
    );
    handle
}

fn insert_runtime_read_view_from_buffer(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    backing: RuntimeElementViewBufferHandle,
    start: usize,
    len: usize,
) -> RuntimeReadViewHandle {
    let handle = RuntimeReadViewHandle(state.next_contiguous_view_id);
    state.next_contiguous_view_id += 1;
    state.read_views.insert(
        handle,
        RuntimeReadViewState {
            type_args: type_args.to_vec(),
            backing: RuntimeElementViewBacking::Buffer(backing),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_edit_view_from_buffer(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    backing: RuntimeElementViewBufferHandle,
    start: usize,
    len: usize,
) -> RuntimeEditViewHandle {
    let handle = RuntimeEditViewHandle(state.next_contiguous_edit_view_id);
    state.next_contiguous_edit_view_id += 1;
    state.edit_views.insert(
        handle,
        RuntimeEditViewState {
            type_args: type_args.to_vec(),
            backing: RuntimeElementViewBacking::Buffer(backing),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_read_view_from_reference(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    reference: RuntimeReferenceValue,
    start: usize,
    len: usize,
) -> RuntimeReadViewHandle {
    let handle = RuntimeReadViewHandle(state.next_contiguous_view_id);
    state.next_contiguous_view_id += 1;
    state.read_views.insert(
        handle,
        RuntimeReadViewState {
            type_args: type_args.to_vec(),
            backing: RuntimeElementViewBacking::Reference(reference),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_read_view_from_ring_window(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    arena: RuntimeRingBufferHandle,
    slots: Vec<u64>,
    start: usize,
    len: usize,
) -> RuntimeReadViewHandle {
    let handle = RuntimeReadViewHandle(state.next_contiguous_view_id);
    state.next_contiguous_view_id += 1;
    state.read_views.insert(
        handle,
        RuntimeReadViewState {
            type_args: type_args.to_vec(),
            backing: RuntimeElementViewBacking::RingWindow { arena, slots },
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_edit_view_from_reference(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    reference: RuntimeReferenceValue,
    start: usize,
    len: usize,
) -> RuntimeEditViewHandle {
    let handle = RuntimeEditViewHandle(state.next_contiguous_edit_view_id);
    state.next_contiguous_edit_view_id += 1;
    state.edit_views.insert(
        handle,
        RuntimeEditViewState {
            type_args: type_args.to_vec(),
            backing: RuntimeElementViewBacking::Reference(reference),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_edit_view_from_ring_window(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    arena: RuntimeRingBufferHandle,
    slots: Vec<u64>,
    start: usize,
    len: usize,
) -> RuntimeEditViewHandle {
    let handle = RuntimeEditViewHandle(state.next_contiguous_edit_view_id);
    state.next_contiguous_edit_view_id += 1;
    state.edit_views.insert(
        handle,
        RuntimeEditViewState {
            type_args: type_args.to_vec(),
            backing: RuntimeElementViewBacking::RingWindow { arena, slots },
            start,
            len,
        },
    );
    handle
}

pub(crate) fn insert_runtime_read_view(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    values: Vec<RuntimeValue>,
) -> RuntimeReadViewHandle {
    let len = values.len();
    let backing = insert_runtime_element_view_buffer(state, type_args, values);
    insert_runtime_read_view_from_buffer(state, type_args, backing, 0, len)
}

pub(crate) fn insert_runtime_edit_view(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    values: Vec<RuntimeValue>,
) -> RuntimeEditViewHandle {
    let len = values.len();
    let backing = insert_runtime_element_view_buffer(state, type_args, values);
    insert_runtime_edit_view_from_buffer(state, type_args, backing, 0, len)
}

fn insert_runtime_read_view_from_backing(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    backing: RuntimeElementViewBacking,
    start: usize,
    len: usize,
) -> RuntimeReadViewHandle {
    match backing {
        RuntimeElementViewBacking::Buffer(buffer) => {
            insert_runtime_read_view_from_buffer(state, type_args, buffer, start, len)
        }
        RuntimeElementViewBacking::Reference(reference) => {
            insert_runtime_read_view_from_reference(state, type_args, reference, start, len)
        }
        RuntimeElementViewBacking::RingWindow { arena, slots } => {
            insert_runtime_read_view_from_ring_window(state, type_args, arena, slots, start, len)
        }
    }
}

fn insert_runtime_byte_view_buffer(
    state: &mut RuntimeExecutionState,
    values: Vec<u8>,
) -> RuntimeByteViewBufferHandle {
    let handle = RuntimeByteViewBufferHandle(state.next_byte_view_buffer_handle);
    state.next_byte_view_buffer_handle += 1;
    state
        .byte_view_buffers
        .insert(handle, RuntimeByteViewBufferState { values });
    handle
}

fn insert_runtime_byte_view_from_buffer(
    state: &mut RuntimeExecutionState,
    backing: RuntimeByteViewBufferHandle,
    start: usize,
    len: usize,
) -> RuntimeByteViewHandle {
    let handle = RuntimeByteViewHandle(state.next_u8_view_id);
    state.next_u8_view_id += 1;
    state.byte_views.insert(
        handle,
        RuntimeByteViewState {
            backing: RuntimeByteViewBacking::Buffer(backing),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_byte_edit_view_from_buffer(
    state: &mut RuntimeExecutionState,
    backing: RuntimeByteViewBufferHandle,
    start: usize,
    len: usize,
) -> RuntimeByteEditViewHandle {
    let handle = RuntimeByteEditViewHandle(state.next_u8_edit_view_id);
    state.next_u8_edit_view_id += 1;
    state.byte_edit_views.insert(
        handle,
        RuntimeByteEditViewState {
            backing: RuntimeByteViewBacking::Buffer(backing),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_byte_view_from_reference(
    state: &mut RuntimeExecutionState,
    reference: RuntimeReferenceValue,
    start: usize,
    len: usize,
) -> RuntimeByteViewHandle {
    let handle = RuntimeByteViewHandle(state.next_u8_view_id);
    state.next_u8_view_id += 1;
    state.byte_views.insert(
        handle,
        RuntimeByteViewState {
            backing: RuntimeByteViewBacking::Reference(reference),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_byte_edit_view_from_reference(
    state: &mut RuntimeExecutionState,
    reference: RuntimeReferenceValue,
    start: usize,
    len: usize,
) -> RuntimeByteEditViewHandle {
    let handle = RuntimeByteEditViewHandle(state.next_u8_edit_view_id);
    state.next_u8_edit_view_id += 1;
    state.byte_edit_views.insert(
        handle,
        RuntimeByteEditViewState {
            backing: RuntimeByteViewBacking::Reference(reference),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_byte_view_from_foreign(
    state: &mut RuntimeExecutionState,
    package_id: &'static str,
    handle_value: u64,
    start: usize,
    len: usize,
) -> RuntimeByteViewHandle {
    let handle = RuntimeByteViewHandle(state.next_u8_view_id);
    state.next_u8_view_id += 1;
    state.byte_views.insert(
        handle,
        RuntimeByteViewState {
            backing: RuntimeByteViewBacking::Foreign(RuntimeForeignByteViewBacking {
                package_id,
                handle: handle_value,
            }),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_byte_edit_view_from_foreign(
    state: &mut RuntimeExecutionState,
    package_id: &'static str,
    handle_value: u64,
    start: usize,
    len: usize,
) -> RuntimeByteEditViewHandle {
    let handle = RuntimeByteEditViewHandle(state.next_u8_edit_view_id);
    state.next_u8_edit_view_id += 1;
    state.byte_edit_views.insert(
        handle,
        RuntimeByteEditViewState {
            backing: RuntimeByteViewBacking::Foreign(RuntimeForeignByteViewBacking {
                package_id,
                handle: handle_value,
            }),
            start,
            len,
        },
    );
    handle
}

#[allow(dead_code)]
pub(crate) fn insert_runtime_byte_edit_view(
    state: &mut RuntimeExecutionState,
    values: Vec<u8>,
) -> RuntimeByteEditViewHandle {
    let len = values.len();
    let backing = insert_runtime_byte_view_buffer(state, values);
    insert_runtime_byte_edit_view_from_buffer(state, backing, 0, len)
}

fn insert_runtime_str_view_buffer(
    state: &mut RuntimeExecutionState,
    text: String,
) -> RuntimeStrViewBufferHandle {
    let handle = RuntimeStrViewBufferHandle(state.next_str_view_buffer_handle);
    state.next_str_view_buffer_handle += 1;
    state
        .str_view_buffers
        .insert(handle, RuntimeStrViewBufferState { text });
    handle
}

fn insert_runtime_str_view_from_buffer(
    state: &mut RuntimeExecutionState,
    backing: RuntimeStrViewBufferHandle,
    start: usize,
    len: usize,
) -> RuntimeStrViewHandle {
    let handle = RuntimeStrViewHandle(state.next_text_view_id);
    state.next_text_view_id += 1;
    state.str_views.insert(
        handle,
        RuntimeStrViewState {
            backing: RuntimeStrViewBacking::Buffer(backing),
            start,
            len,
        },
    );
    handle
}

fn insert_runtime_str_view_from_reference(
    state: &mut RuntimeExecutionState,
    reference: RuntimeReferenceValue,
    start: usize,
    len: usize,
) -> RuntimeStrViewHandle {
    let handle = RuntimeStrViewHandle(state.next_text_view_id);
    state.next_text_view_id += 1;
    state.str_views.insert(
        handle,
        RuntimeStrViewState {
            backing: RuntimeStrViewBacking::Reference(reference),
            start,
            len,
        },
    );
    handle
}

#[cfg(test)]
pub(crate) fn runtime_read_view_snapshot(
    state: &RuntimeExecutionState,
    handle: RuntimeReadViewHandle,
) -> Option<Vec<RuntimeValue>> {
    let view = state.read_views.get(&handle)?;
    match &view.backing {
        RuntimeElementViewBacking::Buffer(buffer) => {
            let values = &state.element_view_buffers.get(buffer)?.values;
            values
                .get(view.start..view.start + view.len)
                .map(|slice| slice.to_vec())
        }
        RuntimeElementViewBacking::Reference(_) => None,
        RuntimeElementViewBacking::RingWindow { arena, slots } => {
            let ring = state.ring_buffers.get(arena)?;
            let slot_slice = slots.get(view.start..view.start + view.len)?;
            slot_slice
                .iter()
                .map(|slot| ring.slots.get(slot).cloned())
                .collect()
        }
    }
}

fn default_runtime_arena_policy(capacity: usize) -> RuntimeArenaPolicy {
    RuntimeArenaPolicy {
        base_capacity: capacity,
        current_limit: capacity,
        growth_step: 0,
        pressure: RuntimeMemoryPressurePolicy::Bounded,
        handle: RuntimeMemoryHandlePolicy::Stable,
    }
}

fn default_runtime_frame_policy(capacity: usize) -> RuntimeFrameArenaPolicy {
    RuntimeFrameArenaPolicy {
        base_capacity: capacity,
        current_limit: capacity,
        growth_step: 0,
        pressure: RuntimeMemoryPressurePolicy::Bounded,
        recycle: RuntimeFrameRecyclePolicy::Manual,
        reset_on: RuntimeResetOnPolicy::Manual,
    }
}

fn default_runtime_pool_policy(capacity: usize) -> RuntimePoolArenaPolicy {
    RuntimePoolArenaPolicy {
        base_capacity: capacity,
        current_limit: capacity,
        growth_step: 0,
        pressure: RuntimeMemoryPressurePolicy::Bounded,
        recycle: RuntimePoolRecyclePolicy::Strict,
        handle: RuntimeMemoryHandlePolicy::Stable,
    }
}

fn default_runtime_temp_policy(capacity: usize) -> RuntimeTempArenaPolicy {
    RuntimeTempArenaPolicy {
        base_capacity: capacity,
        current_limit: capacity,
        growth_step: 0,
        pressure: RuntimeMemoryPressurePolicy::Bounded,
        reset_on: RuntimeResetOnPolicy::Manual,
    }
}

fn default_runtime_session_policy(capacity: usize) -> RuntimeSessionArenaPolicy {
    RuntimeSessionArenaPolicy {
        base_capacity: capacity,
        current_limit: capacity,
        growth_step: 0,
        pressure: RuntimeMemoryPressurePolicy::Bounded,
        handle: RuntimeMemoryHandlePolicy::Stable,
    }
}

fn default_runtime_ring_policy(capacity: usize) -> RuntimeRingBufferPolicy {
    RuntimeRingBufferPolicy {
        base_capacity: capacity,
        current_limit: capacity,
        growth_step: 0,
        pressure: RuntimeMemoryPressurePolicy::Bounded,
        overwrite: RuntimeRingOverwritePolicy::Oldest,
        window: capacity,
    }
}

fn default_runtime_slab_policy(capacity: usize) -> RuntimeSlabPolicy {
    RuntimeSlabPolicy {
        base_capacity: capacity,
        current_limit: capacity,
        growth_step: 0,
        pressure: RuntimeMemoryPressurePolicy::Bounded,
        handle: RuntimeMemoryHandlePolicy::Stable,
        page: capacity.max(1),
    }
}

fn ensure_runtime_arena_capacity(arena: &mut RuntimeArenaState) -> Result<(), String> {
    if arena.slots.len() < arena.policy.current_limit {
        return Ok(());
    }
    if matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic)
        && runtime_try_grow_limit(&mut arena.policy.current_limit, arena.policy.growth_step)
    {
        return Ok(());
    }
    Err(format!(
        "arena capacity exhausted at {}; growth={} pressure={:?}",
        arena.policy.current_limit, arena.policy.growth_step, arena.policy.pressure
    ))
}

fn ensure_runtime_frame_capacity(arena: &mut RuntimeFrameArenaState) -> Result<(), String> {
    if arena.slots.len() < arena.policy.current_limit {
        return Ok(());
    }
    if matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic)
        && runtime_try_grow_limit(&mut arena.policy.current_limit, arena.policy.growth_step)
    {
        return Ok(());
    }
    if matches!(arena.policy.recycle, RuntimeFrameRecyclePolicy::Frame)
        || matches!(arena.policy.reset_on, RuntimeResetOnPolicy::Frame)
    {
        arena.generation += 1;
        arena.next_slot = 0;
        arena.slots.clear();
        arena.policy.current_limit = arena.policy.base_capacity;
        return Ok(());
    }
    Err(format!(
        "frame arena capacity exhausted at {}; growth={} pressure={:?} recycle={:?} reset_on={:?}",
        arena.policy.current_limit,
        arena.policy.growth_step,
        arena.policy.pressure,
        arena.policy.recycle,
        arena.policy.reset_on
    ))
}

fn ensure_runtime_temp_capacity(arena: &mut RuntimeTempArenaState) -> Result<(), String> {
    if arena.slots.len() < arena.policy.current_limit {
        return Ok(());
    }
    if matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic)
        && runtime_try_grow_limit(&mut arena.policy.current_limit, arena.policy.growth_step)
    {
        return Ok(());
    }
    if matches!(arena.policy.reset_on, RuntimeResetOnPolicy::Frame) {
        arena.generation += 1;
        arena.next_slot = 0;
        arena.slots.clear();
        arena.policy.current_limit = arena.policy.base_capacity;
        return Ok(());
    }
    Err(format!(
        "temp arena capacity exhausted at {}; growth={} pressure={:?} reset_on={:?}",
        arena.policy.current_limit,
        arena.policy.growth_step,
        arena.policy.pressure,
        arena.policy.reset_on
    ))
}

fn ensure_runtime_session_capacity(arena: &mut RuntimeSessionArenaState) -> Result<(), String> {
    if arena.slots.len() < arena.policy.current_limit {
        return Ok(());
    }
    if matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic)
        && runtime_try_grow_limit(&mut arena.policy.current_limit, arena.policy.growth_step)
    {
        return Ok(());
    }
    Err(format!(
        "session arena capacity exhausted at {}; growth={} pressure={:?}",
        arena.policy.current_limit, arena.policy.growth_step, arena.policy.pressure
    ))
}

fn ensure_runtime_ring_capacity(arena: &mut RuntimeRingBufferState) -> Result<(), String> {
    if arena.slots.len() < arena.policy.current_limit {
        return Ok(());
    }
    if matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic)
        && runtime_try_grow_limit(&mut arena.policy.current_limit, arena.policy.growth_step)
    {
        return Ok(());
    }
    if matches!(arena.policy.overwrite, RuntimeRingOverwritePolicy::Oldest)
        && let Some(oldest_slot) = arena.order.pop_front()
    {
        arena.slots.remove(&oldest_slot);
        *arena.generations.entry(oldest_slot).or_insert(0) += 1;
        return Ok(());
    }
    Err(format!(
        "ring capacity exhausted at {}; growth={} pressure={:?} overwrite={:?}",
        arena.policy.current_limit,
        arena.policy.growth_step,
        arena.policy.pressure,
        arena.policy.overwrite
    ))
}

fn ensure_runtime_slab_capacity(arena: &mut RuntimeSlabState) -> Result<(), String> {
    if arena.slots.len() < arena.policy.current_limit {
        return Ok(());
    }
    let growth_step = arena.policy.growth_step.max(arena.policy.page);
    if matches!(arena.policy.pressure, RuntimeMemoryPressurePolicy::Elastic)
        && runtime_try_grow_limit_by(&mut arena.policy.current_limit, growth_step)
    {
        return Ok(());
    }
    Err(format!(
        "slab capacity exhausted at {}; growth={} pressure={:?} page={}",
        arena.policy.current_limit,
        arena.policy.growth_step,
        arena.policy.pressure,
        arena.policy.page
    ))
}

fn runtime_reset_frame_arena_handle(
    state: &mut RuntimeExecutionState,
    handle: RuntimeFrameArenaHandle,
    context: &str,
) -> Result<(), String> {
    runtime_reject_live_view_conflict(
        state,
        |reference| runtime_reference_targets_frame_arena(reference, handle),
        format!(
            "{context} rejects invalidation while borrowed views for FrameArena `{}` are live",
            handle.0
        ),
    )?;
    let arena = state
        .frame_arenas
        .get_mut(&handle)
        .ok_or_else(|| format!("invalid FrameArena handle `{}`", handle.0))?;
    arena.generation += 1;
    arena.next_slot = 0;
    arena.slots.clear();
    arena.policy.current_limit = arena.policy.base_capacity;
    Ok(())
}

fn runtime_reset_temp_arena_handle(
    state: &mut RuntimeExecutionState,
    handle: RuntimeTempArenaHandle,
    context: &str,
) -> Result<(), String> {
    runtime_reject_live_view_conflict(
        state,
        |reference| runtime_reference_targets_temp_arena(reference, handle),
        format!(
            "{context} rejects invalidation while borrowed views for TempArena `{}` are live",
            handle.0
        ),
    )?;
    let arena = state
        .temp_arenas
        .get_mut(&handle)
        .ok_or_else(|| format!("invalid TempArena handle `{}`", handle.0))?;
    arena.generation += 1;
    arena.next_slot = 0;
    arena.slots.clear();
    arena.policy.current_limit = arena.policy.base_capacity;
    Ok(())
}

fn runtime_reset_owner_exit_memory_specs_in_scopes(
    scopes: &mut [RuntimeScope],
    state: &mut RuntimeExecutionState,
    owner_key: &str,
) -> Result<(), String> {
    for scope in scopes {
        for spec_state in scope.memory_specs.values_mut() {
            if !spec_state
                .owner_keys
                .iter()
                .any(|active| active == owner_key)
            {
                continue;
            }
            let Some(handle) = spec_state.handle.clone() else {
                continue;
            };
            match handle {
                RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle)) => {
                    let should_reset = state.frame_arenas.get(&handle).is_some_and(|arena| {
                        matches!(arena.policy.reset_on, RuntimeResetOnPolicy::OwnerExit)
                    });
                    if should_reset {
                        runtime_reset_frame_arena_handle(state, handle, "owner_exit reset")?;
                    }
                }
                RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(handle)) => {
                    let should_reset = state.temp_arenas.get(&handle).is_some_and(|arena| {
                        matches!(arena.policy.reset_on, RuntimeResetOnPolicy::OwnerExit)
                    });
                    if should_reset {
                        runtime_reset_temp_arena_handle(state, handle, "owner_exit reset")?;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn runtime_reset_owner_exit_module_memory_specs(
    state: &mut RuntimeExecutionState,
    owner_key: &str,
) -> Result<(), String> {
    let handles = state
        .module_memory_specs
        .values()
        .filter(|spec_state| {
            spec_state
                .owner_keys
                .iter()
                .any(|active| active == owner_key)
        })
        .filter_map(|spec_state| spec_state.handle.clone())
        .collect::<Vec<_>>();
    for handle in handles {
        match handle {
            RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(handle)) => {
                let should_reset = state.frame_arenas.get(&handle).is_some_and(|arena| {
                    matches!(arena.policy.reset_on, RuntimeResetOnPolicy::OwnerExit)
                });
                if should_reset {
                    runtime_reset_frame_arena_handle(state, handle, "owner_exit reset")?;
                }
            }
            RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(handle)) => {
                let should_reset = state.temp_arenas.get(&handle).is_some_and(|arena| {
                    matches!(arena.policy.reset_on, RuntimeResetOnPolicy::OwnerExit)
                });
                if should_reset {
                    runtime_reset_temp_arena_handle(state, handle, "owner_exit reset")?;
                }
            }
            _ => {}
        }
    }
    Ok(())
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

fn insert_runtime_lazy(
    state: &mut RuntimeExecutionState,
    type_args: &[String],
    pending: RuntimePendingState,
) -> RuntimeLazyHandle {
    let handle = RuntimeLazyHandle(state.next_lazy_handle);
    state.next_lazy_handle += 1;
    state.lazy_values.insert(
        handle,
        RuntimeTaskState {
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

fn temp_id_is_live(
    handle: RuntimeTempArenaHandle,
    arena: &RuntimeTempArenaState,
    id: RuntimeTempIdValue,
) -> bool {
    id.arena == handle && id.generation == arena.generation && arena.slots.contains_key(&id.slot)
}

fn session_id_is_live(
    handle: RuntimeSessionArenaHandle,
    arena: &RuntimeSessionArenaState,
    id: RuntimeSessionIdValue,
) -> bool {
    id.arena == handle && id.generation == arena.generation && arena.slots.contains_key(&id.slot)
}

fn ring_slot_generation(ring: &RuntimeRingBufferState, slot: u64) -> u64 {
    ring.generations.get(&slot).copied().unwrap_or(0)
}

fn runtime_ring_ids_for_slots(
    state: &RuntimeExecutionState,
    arena: RuntimeRingBufferHandle,
    slots: &[u64],
) -> Result<Vec<RuntimeRingIdValue>, String> {
    let ring = state
        .ring_buffers
        .get(&arena)
        .ok_or_else(|| format!("invalid RingBuffer handle `{}`", arena.0))?;
    Ok(slots
        .iter()
        .copied()
        .map(|slot| RuntimeRingIdValue {
            arena,
            slot,
            generation: ring_slot_generation(ring, slot),
        })
        .collect())
}

fn ring_id_is_live(
    handle: RuntimeRingBufferHandle,
    arena: &RuntimeRingBufferState,
    id: RuntimeRingIdValue,
) -> bool {
    id.arena == handle
        && id.generation == ring_slot_generation(arena, id.slot)
        && arena.slots.contains_key(&id.slot)
}

fn slab_slot_generation(slab: &RuntimeSlabState, slot: u64) -> u64 {
    slab.generations.get(&slot).copied().unwrap_or(0)
}

fn slab_id_is_live(
    handle: RuntimeSlabHandle,
    arena: &RuntimeSlabState,
    id: RuntimeSlabIdValue,
) -> bool {
    id.arena == handle
        && id.generation == slab_slot_generation(arena, id.slot)
        && arena.slots.contains_key(&id.slot)
}

fn ecs_entity_exists(state: &RuntimeExecutionState, entity: i64) -> bool {
    entity == 0 || state.live_entities.contains(&entity)
}

fn runtime_behavior_step(
    plan: &RuntimePackagePlan,
    phase: &str,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
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

fn runtime_text_is_space_byte(byte: i64) -> bool {
    matches!(byte, 32 | 9 | 10 | 13)
}

fn runtime_text_len_bytes(text: &str) -> Result<i64, String> {
    i64::try_from(text.len()).map_err(|_| "text length does not fit in i64".to_string())
}

fn runtime_text_byte_at(text: &str, index: usize) -> Result<i64, String> {
    let byte = text
        .as_bytes()
        .get(index)
        .copied()
        .ok_or_else(|| format!("text byte index `{index}` is out of bounds"))?;
    Ok(i64::from(byte))
}

fn runtime_text_slice_bytes(text: &str, start: usize, end: usize) -> Result<String, String> {
    let slice = text
        .as_bytes()
        .get(start..end)
        .ok_or_else(|| format!("text byte slice `{start}..{end}` is out of bounds"))?;
    std::str::from_utf8(slice)
        .map(|slice| slice.to_string())
        .map_err(|_| format!("text byte slice `{start}..{end}` is not valid UTF-8"))
}

fn runtime_text_starts_with(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

fn runtime_text_ends_with(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

fn runtime_text_split_lines(text: &str) -> Vec<String> {
    text.lines().map(ToString::to_string).collect()
}

fn runtime_text_from_int(value: i64) -> String {
    value.to_string()
}

fn runtime_bytes_from_str_utf8(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

fn runtime_bytes_to_str_utf8(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn runtime_bytes_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn runtime_utf16_from_str(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn runtime_utf16_to_str(units: &[u16]) -> Result<String, String> {
    String::from_utf16(units).map_err(|err| err.to_string())
}

fn runtime_text_find(text: &str, start: i64, needle: &str) -> Result<i64, String> {
    let total = runtime_text_len_bytes(text)?;
    let needle_len = runtime_text_len_bytes(needle)?;
    let mut index = start.max(0);
    if needle_len == 0 {
        return Ok(index.min(total));
    }
    while index + needle_len <= total {
        if runtime_text_slice_bytes(text, index as usize, (index + needle_len) as usize)? == needle
        {
            return Ok(index);
        }
        index += 1;
    }
    Ok(-1)
}

fn runtime_text_trim_start(text: &str) -> Result<String, String> {
    let total = runtime_text_len_bytes(text)?;
    let mut index = 0;
    while index < total && runtime_text_is_space_byte(runtime_text_byte_at(text, index as usize)?) {
        index += 1;
    }
    runtime_text_slice_bytes(text, index as usize, total as usize)
}

fn runtime_text_trim_end(text: &str) -> Result<String, String> {
    let mut end = runtime_text_len_bytes(text)?;
    while end > 0 && runtime_text_is_space_byte(runtime_text_byte_at(text, (end - 1) as usize)?) {
        end -= 1;
    }
    runtime_text_slice_bytes(text, 0, end as usize)
}

fn runtime_text_trim(text: &str) -> Result<String, String> {
    let trimmed_start = runtime_text_trim_start(text)?;
    runtime_text_trim_end(&trimmed_start)
}

fn runtime_text_split(text: &str, delim: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let total = runtime_text_len_bytes(text)?;
    let delim_len = runtime_text_len_bytes(delim)?;
    if delim_len == 0 {
        out.push(runtime_text_slice_bytes(text, 0, total as usize)?);
        return Ok(out);
    }
    let mut start = 0;
    while start <= total {
        let next = runtime_text_find(text, start, delim)?;
        if next < 0 {
            out.push(runtime_text_slice_bytes(
                text,
                start as usize,
                total as usize,
            )?);
            return Ok(out);
        }
        out.push(runtime_text_slice_bytes(
            text,
            start as usize,
            next as usize,
        )?);
        start = next + delim_len;
    }
    Ok(out)
}

fn runtime_text_join(parts: Vec<String>, delim: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    for part in parts {
        if first {
            out = part;
            first = false;
        } else {
            out.push_str(delim);
            out.push_str(&part);
        }
    }
    out
}

fn runtime_text_repeat(text: &str, count: i64) -> String {
    if count <= 0 {
        return String::new();
    }
    let mut out = String::new();
    for _ in 0..count {
        out.push_str(text);
    }
    out
}

fn runtime_text_to_int(text: &str) -> Result<Result<i64, String>, String> {
    let value = runtime_text_trim(text)?;
    let total = runtime_text_len_bytes(&value)?;
    if total == 0 {
        return Ok(Err("expected integer text".to_string()));
    }
    let mut sign = 1;
    let mut index = 0;
    let first = runtime_text_byte_at(&value, 0)?;
    if first == 45 {
        sign = -1;
        index = 1;
    } else if first == 43 {
        index = 1;
    }
    if index >= total {
        return Ok(Err("expected integer digits".to_string()));
    }
    let mut out = 0i64;
    while index < total {
        let byte = runtime_text_byte_at(&value, index as usize)?;
        if !(48..=57).contains(&byte) {
            return Ok(Err("invalid digit in integer text".to_string()));
        }
        out = out * 10 + (byte - 48);
        index += 1;
    }
    Ok(Ok(out * sign))
}

fn expect_single_arg(mut args: Vec<RuntimeValue>, name: &str) -> Result<RuntimeValue, String> {
    if args.len() != 1 {
        return Err(format!("{name} expects one argument"));
    }
    Ok(args.remove(0))
}

const RUNTIME_DIRECT_CALLABLE_SIGNATURE_SOURCES: &[(&str, &str)] = &[
    (
        "std.behaviors",
        include_str!("../../../std/src/behaviors.arc"),
    ),
    (
        "std.concurrent",
        include_str!("../../../std/src/concurrent.arc"),
    ),
    ("std.ecs", include_str!("../../../std/src/ecs.arc")),
    ("std.memory", include_str!("../../../std/src/memory.arc")),
    ("std.package", include_str!("../../../std/src/package.arc")),
    ("std.text", include_str!("../../../std/src/text.arc")),
    ("std.time", include_str!("../../../std/src/time.arc")),
    (
        "arcana_process.args",
        include_str!("../../../grimoires/arcana/process/src/args.arc"),
    ),
    (
        "arcana_process.env",
        include_str!("../../../grimoires/arcana/process/src/env.arc"),
    ),
    (
        "arcana_process.fs",
        include_str!("../../../grimoires/arcana/process/src/fs.arc"),
    ),
    (
        "arcana_process.io",
        include_str!("../../../grimoires/arcana/process/src/io.arc"),
    ),
    (
        "arcana_process.path",
        include_str!("../../../grimoires/arcana/process/src/path.arc"),
    ),
    (
        "arcana_process.process",
        include_str!("../../../grimoires/arcana/process/src/process.arc"),
    ),
    (
        "arcana_winapi.helpers.window",
        include_str!("../../../grimoires/arcana/winapi/src/helpers/window.arc"),
    ),
    (
        "arcana_winapi.helpers.input",
        include_str!("../../../grimoires/arcana/winapi/src/helpers/input.arc"),
    ),
    (
        "arcana_winapi.helpers.events",
        include_str!("../../../grimoires/arcana/winapi/src/helpers/events.arc"),
    ),
    (
        "arcana_winapi.helpers.clipboard",
        include_str!("../../../grimoires/arcana/winapi/src/helpers/clipboard.arc"),
    ),
    (
        "arcana_winapi.helpers.text_input",
        include_str!("../../../grimoires/arcana/winapi/src/helpers/text_input.arc"),
    ),
    (
        "arcana_winapi.helpers.audio",
        include_str!("../../../grimoires/arcana/winapi/src/helpers/audio.arc"),
    ),
    (
        "std.collections.array",
        include_str!("../../../std/src/collections/array.arc"),
    ),
    (
        "std.collections.list",
        include_str!("../../../std/src/collections/list.arc"),
    ),
    (
        "std.collections.map",
        include_str!("../../../std/src/collections/map.arc"),
    ),
    (
        "std.collections.set",
        include_str!("../../../std/src/collections/set.arc"),
    ),
    (
        "std.kernel.collections",
        include_str!("../../../std/src/kernel/collections.arc"),
    ),
    (
        "std.kernel.concurrency",
        include_str!("../../../std/src/kernel/concurrency.arc"),
    ),
    (
        "std.kernel.ecs",
        include_str!("../../../std/src/kernel/ecs.arc"),
    ),
    (
        "std.kernel.io",
        include_str!("../../../std/src/kernel/io.arc"),
    ),
    (
        "std.kernel.memory",
        include_str!("../../../std/src/kernel/memory.arc"),
    ),
    (
        "std.kernel.package",
        include_str!("../../../std/src/kernel/package.arc"),
    ),
    (
        "std.kernel.text",
        include_str!("../../../std/src/kernel/text.arc"),
    ),
    (
        "std.kernel.time",
        include_str!("../../../std/src/kernel/time.arc"),
    ),
];

fn runtime_direct_callable_param_names() -> &'static BTreeMap<String, Vec<String>> {
    static PARAMS: OnceLock<BTreeMap<String, Vec<String>>> = OnceLock::new();
    PARAMS.get_or_init(|| {
        let mut params = BTreeMap::new();
        for (module_path, source) in RUNTIME_DIRECT_CALLABLE_SIGNATURE_SOURCES {
            collect_runtime_direct_callable_signatures(module_path, source, &mut params);
        }
        for (path, names) in [
            ("Option.None", Vec::<&str>::new()),
            ("Option.Some", vec!["value"]),
            ("Result.Ok", vec!["value"]),
            ("Result.Err", vec!["error"]),
            ("std.option.Option.None", Vec::<&str>::new()),
            ("std.option.Option.Some", vec!["value"]),
            ("std.result.Result.Ok", vec!["value"]),
            ("std.result.Result.Err", vec!["error"]),
            ("std.option.is_some", vec!["self"]),
            ("std.option.is_none", vec!["self"]),
            ("std.option.unwrap_or", vec!["self", "fallback"]),
            ("std.result.is_ok", vec!["self"]),
            ("std.result.is_err", vec!["self"]),
            ("std.result.unwrap_or", vec!["self", "fallback"]),
        ] {
            params.insert(
                path.to_string(),
                names.into_iter().map(str::to_string).collect(),
            );
        }
        params
    })
}

fn collect_runtime_direct_callable_signatures(
    module_path: &str,
    source: &str,
    params: &mut BTreeMap<String, Vec<String>>,
) {
    for line in source.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let trimmed = line.trim();
        let Some((name, param_names)) = parse_runtime_direct_callable_signature(trimmed) else {
            continue;
        };
        params.insert(format!("{module_path}.{name}"), param_names);
    }
}

fn parse_runtime_direct_callable_signature(line: &str) -> Option<(String, Vec<String>)> {
    let mut rest = line
        .strip_prefix("export ")
        .map(str::trim_start)
        .unwrap_or(line);
    rest = if let Some(next) = rest.strip_prefix("intrinsic fn ") {
        next
    } else if let Some(next) = rest.strip_prefix("async fn ") {
        next
    } else if let Some(next) = rest.strip_prefix("fn ") {
        next
    } else {
        return None;
    };
    let open = find_runtime_signature_open_paren(rest)?;
    let name_text = rest[..open].trim();
    let name = name_text.split('[').next()?.trim();
    let close = find_runtime_signature_matching_paren(&rest[open..])?;
    let params_text = &rest[open + 1..open + close];
    Some((
        name.to_string(),
        split_runtime_signature_params(params_text)
            .into_iter()
            .filter_map(|param| parse_runtime_signature_param_name(param.trim()))
            .collect(),
    ))
}

fn find_runtime_signature_open_paren(text: &str) -> Option<usize> {
    let mut bracket_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' if bracket_depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn find_runtime_signature_matching_paren(text: &str) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                if paren_depth == 0 && bracket_depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_runtime_signature_params(text: &str) -> Vec<&str> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                out.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    out.push(text[start..].trim());
    out
}

fn parse_runtime_signature_param_name(param: &str) -> Option<String> {
    if param.is_empty() {
        return None;
    }
    let param = param
        .strip_prefix("read ")
        .or_else(|| param.strip_prefix("edit "))
        .or_else(|| param.strip_prefix("take "))
        .unwrap_or(param);
    let (name, _) = param.split_once(':')?;
    Some(name.trim().to_string())
}

fn collect_call_args(
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<Vec<RuntimeCallArg>> {
    let mut values = args
        .iter()
        .map(|arg| {
            Ok(RuntimeCallArg {
                name: arg.name.clone(),
                source_expr: arg.value.clone(),
                value: force_runtime_value(
                    eval_expr(
                        &arg.value,
                        plan,
                        current_package_id,
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
                )?,
            })
        })
        .collect::<RuntimeEvalResult<Vec<_>>>()?;
    for attachment in attached {
        let (name, value) = match attachment {
            ParsedHeaderAttachment::Named { name, value } => (
                Some(name.clone()),
                force_runtime_value(
                    eval_expr(
                        value,
                        plan,
                        current_package_id,
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
                )?,
            ),
            ParsedHeaderAttachment::Chain { expr } => (
                None,
                force_runtime_value(
                    eval_expr(
                        expr,
                        plan,
                        current_package_id,
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

fn bind_call_args_for_intrinsic(
    callable: &[String],
    args: Vec<RuntimeCallArg>,
) -> Result<Vec<RuntimeCallArg>, String> {
    let callable_key = callable.join(".");
    let Some(param_names) = runtime_direct_callable_param_names().get(&callable_key) else {
        if args.iter().any(|arg| arg.name.is_some()) {
            return Err(format!(
                "runtime intrinsic `{callable_key}` does not expose named-argument metadata"
            ));
        }
        return Ok(args
            .into_iter()
            .map(|mut arg| {
                arg.name = None;
                arg
            })
            .collect());
    };
    let mut bound = vec![None; param_names.len()];
    let mut next_positional = 0usize;
    for mut arg in args {
        let index = if let Some(name) = arg.name.as_deref() {
            param_names
                .iter()
                .position(|param_name| param_name == name)
                .ok_or_else(|| {
                    format!("runtime intrinsic `{callable_key}` has no parameter `{name}`")
                })?
        } else {
            let Some(index) =
                (next_positional..param_names.len()).find(|index| bound[*index].is_none())
            else {
                return Err(format!(
                    "runtime intrinsic `{callable_key}` received too many arguments"
                ));
            };
            next_positional = index + 1;
            index
        };
        if bound[index].is_some() {
            return Err(format!(
                "runtime intrinsic `{callable_key}` received duplicate argument for `{}`",
                param_names[index]
            ));
        }
        arg.name = None;
        bound[index] = Some(arg);
    }
    let missing = param_names
        .iter()
        .zip(bound.iter())
        .filter_map(|(name, value)| value.is_none().then_some(name.clone()))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        let unit_ok_omission = missing.len() == 1
            && missing[0] == "value"
            && param_names.len() == 1
            && matches!(callable_key.as_str(), "Result.Ok" | "std.result.Result.Ok");
        if unit_ok_omission {
            return Ok(Vec::new());
        }
        return Err(format!(
            "runtime intrinsic `{callable_key}` is missing arguments for {}",
            missing.join(", ")
        ));
    }
    Ok(bound
        .into_iter()
        .map(|value| value.expect("missing args should have returned"))
        .collect())
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

fn runtime_execution_arg_for_bound_param(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    param: &RuntimeParamPlan,
    arg: &BoundRuntimeArg,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    match param.mode.as_deref() {
        Some("edit") => {}
        Some("take") | Some("hold") => return Ok(arg.value.clone()),
        _ => {
            return read_runtime_value_if_ref(
                arg.value.clone(),
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            );
        }
    }
    if !matches!(
        arg.value,
        RuntimeValue::Str(_)
            | RuntimeValue::Pair(_, _)
            | RuntimeValue::Array(_)
            | RuntimeValue::List(_)
            | RuntimeValue::Map(_)
            | RuntimeValue::Record { .. }
            | RuntimeValue::Variant { .. }
    ) {
        return Ok(arg.value.clone());
    }
    let Some(target) = expr_to_assign_target(&arg.source_expr) else {
        return Ok(arg.value.clone());
    };
    let Ok(place) = resolve_assign_target_place(scopes, &target) else {
        return Ok(arg.value.clone());
    };
    if let RuntimeReferenceTarget::Local { local, .. } = &place.target
        && let Some((_, runtime_local)) = lookup_local_with_name_by_handle(scopes, *local)
    {
        state
            .captured_local_values
            .entry(*local)
            .or_insert_with(|| runtime_local.value.clone());
    }
    Ok(RuntimeValue::Ref(RuntimeReferenceValue {
        mode: place.mode,
        target: place.target,
    }))
}

fn ok_variant(value: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Variant {
        name: "Result.Ok".to_string(),
        payload: vec![value],
    }
}

fn some_variant(value: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Variant {
        name: "Option.Some".to_string(),
        payload: vec![value],
    }
}

fn none_variant() -> RuntimeValue {
    RuntimeValue::Variant {
        name: "Option.None".to_string(),
        payload: Vec::new(),
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
        RuntimeOpaqueValue::Binding(value) => value.type_name,
        _ => RuntimeOpaqueFamily::from_opaque_value(value).canonical_type_name(),
    }
}

fn runtime_array_projection_type_args(
    values: &[RuntimeValue],
    state: &RuntimeExecutionState,
) -> Vec<String> {
    runtime_uniform_value_type(values, Some(state))
        .map(|ty| vec![ty.render()])
        .unwrap_or_default()
}

fn runtime_element_view_family_name(backing: &RuntimeElementViewBacking) -> &'static str {
    match backing {
        RuntimeElementViewBacking::RingWindow { .. } => "Strided",
        RuntimeElementViewBacking::Buffer(_) | RuntimeElementViewBacking::Reference(_) => {
            "Contiguous"
        }
    }
}

fn runtime_byte_view_family_name(backing: &RuntimeByteViewBacking) -> &'static str {
    match backing {
        RuntimeByteViewBacking::Foreign(_) => "Mapped",
        RuntimeByteViewBacking::Buffer(_) | RuntimeByteViewBacking::Reference(_) => "Contiguous",
    }
}

fn runtime_opaque_view_type_name(
    value: &RuntimeOpaqueValue,
    state: &RuntimeExecutionState,
) -> Option<String> {
    match value {
        RuntimeOpaqueValue::ReadView(handle) => state.read_views.get(handle).map(|view| {
            let item = view
                .type_args
                .first()
                .cloned()
                .unwrap_or_else(|| "Int".to_string());
            format!(
                "View[{item}, {}]",
                runtime_element_view_family_name(&view.backing)
            )
        }),
        RuntimeOpaqueValue::EditView(handle) => state.edit_views.get(handle).map(|view| {
            let item = view
                .type_args
                .first()
                .cloned()
                .unwrap_or_else(|| "Int".to_string());
            format!(
                "View[{item}, {}]",
                runtime_element_view_family_name(&view.backing)
            )
        }),
        RuntimeOpaqueValue::ByteView(handle) => state
            .byte_views
            .get(handle)
            .map(|view| format!("View[U8, {}]", runtime_byte_view_family_name(&view.backing))),
        RuntimeOpaqueValue::ByteEditView(handle) => state
            .byte_edit_views
            .get(handle)
            .map(|view| format!("View[U8, {}]", runtime_byte_view_family_name(&view.backing))),
        RuntimeOpaqueValue::StrView(_) => Some("View[U8, Contiguous]".to_string()),
        _ => None,
    }
}

fn runtime_opaque_type_name_with_state(
    value: &RuntimeOpaqueValue,
    state: Option<&RuntimeExecutionState>,
) -> String {
    if let Some(state) = state
        && let Some(view_type) = runtime_opaque_view_type_name(value, state)
    {
        return view_type;
    }
    opaque_type_name(value).to_string()
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
        RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(handle)) => state
            .temp_arenas
            .get(handle)
            .map(|arena| arena.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(handle)) => state
            .session_arenas
            .get(handle)
            .map(|arena| arena.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(handle)) => state
            .ring_buffers
            .get(handle)
            .map(|arena| arena.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle)) => state
            .slabs
            .get(handle)
            .map(|arena| arena.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => state
            .read_views
            .get(handle)
            .map(|view| view.type_args.clone())
            .unwrap_or_default(),
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(handle)) => state
            .edit_views
            .get(handle)
            .map(|view| view.type_args.clone())
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
        RuntimeValue::Ref(reference) => {
            runtime_reference_inner_value_type_from_state(reference, state)
                .map(|ty| parse_runtime_value_type_args(&ty.render()))
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn runtime_reference_inner_value_type_from_state(
    reference: &RuntimeReferenceValue,
    state: &RuntimeExecutionState,
) -> Option<IrRoutineType> {
    let root = match &reference.target {
        RuntimeReferenceTarget::Local { .. } | RuntimeReferenceTarget::OwnerObject { .. } => None,
        RuntimeReferenceTarget::ArenaSlot { id, .. } => {
            let arena = state.arenas.get(&id.arena)?;
            if !arena_id_is_live(id.arena, arena, *id) {
                return None;
            }
            arena.slots.get(&id.slot)
        }
        RuntimeReferenceTarget::FrameSlot { id, .. } => {
            let arena = state.frame_arenas.get(&id.arena)?;
            if !frame_id_is_live(id.arena, arena, *id) {
                return None;
            }
            arena.slots.get(&id.slot)
        }
        RuntimeReferenceTarget::PoolSlot { id, .. } => {
            let arena = state.pool_arenas.get(&id.arena)?;
            if !pool_id_is_live(id.arena, arena, *id) {
                return None;
            }
            arena.slots.get(&id.slot)
        }
        RuntimeReferenceTarget::TempSlot { id, .. } => {
            let arena = state.temp_arenas.get(&id.arena)?;
            if !temp_id_is_live(id.arena, arena, *id) {
                return None;
            }
            arena.slots.get(&id.slot)
        }
        RuntimeReferenceTarget::SessionSlot { id, .. } => {
            let arena = state.session_arenas.get(&id.arena)?;
            if !session_id_is_live(id.arena, arena, *id) {
                return None;
            }
            arena.slots.get(&id.slot)
        }
        RuntimeReferenceTarget::RingSlot { id, .. } => {
            let arena = state.ring_buffers.get(&id.arena)?;
            if !ring_id_is_live(id.arena, arena, *id) {
                return None;
            }
            arena.slots.get(&id.slot)
        }
        RuntimeReferenceTarget::SlabSlot { id, .. } => {
            let arena = state.slabs.get(&id.arena)?;
            if !slab_id_is_live(id.arena, arena, *id) {
                return None;
            }
            arena.slots.get(&id.slot)
        }
    }?;
    let mut current = root;
    for member in runtime_reference_members(&reference.target) {
        current = runtime_member_value_ref(current, member).ok().flatten()?;
    }
    runtime_value_type_from_state(current, Some(state))
}

fn runtime_simple_type(name: &str) -> Option<IrRoutineType> {
    parse_routine_type_text(name).ok()
}

fn runtime_synthetic_owner_type(owner_key: &str) -> IrRoutineType {
    let mut ty = runtime_simple_type("Owner").expect("synthetic Owner type should parse");
    if let IrRoutineTypeKind::Path(path) = &mut ty.kind {
        path.segments[0] = format!("Owner<{owner_key}>");
    }
    ty
}

fn runtime_type_with_args(base: &str, type_args: &[String]) -> Option<IrRoutineType> {
    if type_args.is_empty() {
        return runtime_simple_type(base);
    }
    let base = runtime_simple_type(base)?;
    let IrRoutineTypeKind::Path(base_path) = base.kind else {
        return None;
    };
    let args = type_args
        .iter()
        .map(|arg| parse_routine_type_text(arg).ok())
        .collect::<Option<Vec<_>>>()?;
    Some(IrRoutineType {
        kind: IrRoutineTypeKind::Apply {
            base: base_path,
            args,
        },
    })
}

fn runtime_uniform_value_type(
    values: &[RuntimeValue],
    state: Option<&RuntimeExecutionState>,
) -> Option<IrRoutineType> {
    let first = values
        .first()
        .and_then(|value| runtime_value_type_from_state(value, state))?;
    let rendered = first.render();
    values
        .iter()
        .skip(1)
        .all(|value| {
            runtime_value_type_from_state(value, state)
                .map(|ty| ty.render() == rendered)
                .unwrap_or(false)
        })
        .then_some(first)
}

fn runtime_map_entry_types(
    entries: &[(RuntimeValue, RuntimeValue)],
    state: Option<&RuntimeExecutionState>,
) -> Option<(IrRoutineType, IrRoutineType)> {
    let (first_key, first_value) = entries.first().and_then(|(key, value)| {
        Some((
            runtime_value_type_from_state(key, state)?,
            runtime_value_type_from_state(value, state)?,
        ))
    })?;
    let key_rendered = first_key.render();
    let value_rendered = first_value.render();
    entries
        .iter()
        .skip(1)
        .all(|(key, value)| {
            runtime_value_type_from_state(key, state)
                .map(|ty| ty.render() == key_rendered)
                .unwrap_or(false)
                && runtime_value_type_from_state(value, state)
                    .map(|ty| ty.render() == value_rendered)
                    .unwrap_or(false)
        })
        .then_some((first_key, first_value))
}

fn runtime_value_type_from_state(
    receiver: &RuntimeValue,
    state: Option<&RuntimeExecutionState>,
) -> Option<IrRoutineType> {
    match receiver {
        RuntimeValue::OwnerHandle(owner_key) => Some(runtime_synthetic_owner_type(owner_key)),
        RuntimeValue::Ref(reference) => state.and_then(|state| {
            let inner = runtime_reference_inner_value_type_from_state(reference, state)?;
            Some(IrRoutineType {
                kind: IrRoutineTypeKind::Ref {
                    mode: reference.mode.as_str().to_string(),
                    lifetime: None,
                    inner: Box::new(inner),
                },
            })
        }),
        RuntimeValue::Pair(left, right) => {
            let left = runtime_value_type_from_state(left, state)?;
            let right = runtime_value_type_from_state(right, state)?;
            runtime_type_with_args("Pair", &[left.render(), right.render()])
        }
        RuntimeValue::Array(values) => {
            if let Some(element) = runtime_uniform_value_type(values, state) {
                runtime_type_with_args("std.collections.array.Array", &[element.render()])
                    .or_else(|| runtime_simple_type("Array"))
            } else {
                runtime_simple_type("std.collections.array.Array")
                    .or_else(|| runtime_simple_type("Array"))
            }
        }
        RuntimeValue::List(values) => {
            if let Some(element) = runtime_uniform_value_type(values, state) {
                runtime_type_with_args("std.collections.list.List", &[element.render()])
                    .or_else(|| runtime_simple_type("List"))
            } else {
                runtime_simple_type("std.collections.list.List")
                    .or_else(|| runtime_simple_type("List"))
            }
        }
        RuntimeValue::Map(entries) => {
            if let Some((key, value)) = runtime_map_entry_types(entries, state) {
                runtime_type_with_args("std.collections.map.Map", &[key.render(), value.render()])
                    .or_else(|| runtime_simple_type("Map"))
            } else {
                runtime_simple_type("std.collections.map.Map")
                    .or_else(|| runtime_simple_type("Map"))
            }
        }
        RuntimeValue::Opaque(
            RuntimeOpaqueValue::Channel(_)
            | RuntimeOpaqueValue::Mutex(_)
            | RuntimeOpaqueValue::Arena(_)
            | RuntimeOpaqueValue::FrameArena(_)
            | RuntimeOpaqueValue::PoolArena(_)
            | RuntimeOpaqueValue::TempArena(_)
            | RuntimeOpaqueValue::SessionArena(_)
            | RuntimeOpaqueValue::RingBuffer(_)
            | RuntimeOpaqueValue::Slab(_)
            | RuntimeOpaqueValue::ReadView(_)
            | RuntimeOpaqueValue::EditView(_)
            | RuntimeOpaqueValue::ByteView(_)
            | RuntimeOpaqueValue::ByteEditView(_)
            | RuntimeOpaqueValue::StrView(_)
            | RuntimeOpaqueValue::Task(_)
            | RuntimeOpaqueValue::Thread(_),
        ) => state.and_then(|state| match receiver {
            RuntimeValue::Opaque(
                value @ (RuntimeOpaqueValue::ReadView(_)
                | RuntimeOpaqueValue::EditView(_)
                | RuntimeOpaqueValue::ByteView(_)
                | RuntimeOpaqueValue::ByteEditView(_)
                | RuntimeOpaqueValue::StrView(_)),
            ) => parse_routine_type_text(&runtime_opaque_type_name_with_state(value, Some(state)))
                .ok(),
            RuntimeValue::Opaque(value) => runtime_type_with_args(
                opaque_type_name(value),
                &runtime_receiver_type_args(receiver, state),
            ),
            _ => unreachable!(),
        }),
        RuntimeValue::Record { name, .. } => parse_routine_type_text(name)
            .ok()
            .or_else(|| runtime_simple_type(runtime_type_root_name(name).as_str())),
        RuntimeValue::Variant { name, .. } => {
            let enum_name = runtime_variant_enum_name(name);
            parse_routine_type_text(&enum_name)
                .ok()
                .or_else(|| runtime_simple_type(runtime_type_root_name(&enum_name).as_str()))
        }
        _ => runtime_value_type_root(receiver).and_then(|name| runtime_simple_type(&name)),
    }
}

fn runtime_value_type_without_state(receiver: &RuntimeValue) -> Option<IrRoutineType> {
    match receiver {
        RuntimeValue::OwnerHandle(owner_key) => Some(runtime_synthetic_owner_type(owner_key)),
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(_)) => runtime_simple_type("View"),
        RuntimeValue::Opaque(value) => runtime_simple_type(opaque_type_name(value)),
        _ => runtime_value_type_from_state(receiver, None),
    }
}

fn runtime_value_type(
    receiver: &RuntimeValue,
    state: &RuntimeExecutionState,
) -> Option<IrRoutineType> {
    runtime_value_type_from_state(receiver, Some(state))
}

fn runtime_simple_root_fallback_allowed(ty: &IrRoutineType) -> bool {
    match &ty.kind {
        IrRoutineTypeKind::Path(path) => {
            path.segments.len() == 1
                && path
                    .segments
                    .last()
                    .is_some_and(|segment| !segment.contains('<'))
        }
        _ => false,
    }
}

fn runtime_opaque_family_for_type_name(
    plan: &RuntimePackagePlan,
    type_name: &str,
) -> Option<RuntimeOpaqueFamily> {
    RuntimeOpaqueFamily::ALL.into_iter().find(|family| {
        family.canonical_type_name() == type_name
            || plan
                .opaque_family_types
                .get(family.lang_item_name())
                .is_some_and(|entries| entries.iter().any(|entry| entry == type_name))
    })
}

fn runtime_opaque_family_for_type(
    plan: &RuntimePackagePlan,
    ty: &IrRoutineType,
) -> Option<RuntimeOpaqueFamily> {
    let type_name = ty.base_path().map(|path| path.render())?;
    runtime_opaque_family_for_type_name(plan, &type_name)
}

fn runtime_opaque_family_type_args(ty: &IrRoutineType) -> Option<&[IrRoutineType]> {
    match &ty.kind {
        IrRoutineTypeKind::Path(_) => Some(&[]),
        IrRoutineTypeKind::Apply { args, .. } => Some(args.as_slice()),
        _ => None,
    }
}

fn runtime_opaque_family_matches(
    plan: &RuntimePackagePlan,
    declared: &IrRoutineType,
    actual: &IrRoutineType,
    type_params: &[String],
) -> bool {
    let Some(declared_family) = runtime_opaque_family_for_type(plan, declared) else {
        return false;
    };
    let Some(actual_family) = runtime_opaque_family_for_type(plan, actual) else {
        return false;
    };
    if declared_family != actual_family {
        return false;
    }
    let Some(declared_args) = runtime_opaque_family_type_args(declared) else {
        return false;
    };
    let Some(actual_args) = runtime_opaque_family_type_args(actual) else {
        return false;
    };
    declared_args.len() == actual_args.len()
        && declared_args
            .iter()
            .zip(actual_args)
            .all(|(declared_arg, actual_arg)| {
                IrRoutineType::matches_declared(declared_arg, actual_arg, type_params)
            })
}

fn runtime_receiver_matches_declared_type(
    plan: &RuntimePackagePlan,
    declared: &IrRoutineType,
    actual: &IrRoutineType,
    type_params: &[String],
    receiver_root: &str,
) -> bool {
    IrRoutineType::matches_declared(declared, actual, type_params)
        || match &actual.kind {
            IrRoutineTypeKind::Ref { inner, .. } => {
                IrRoutineType::matches_declared(declared, inner, type_params)
                    || runtime_opaque_family_matches(plan, declared, inner, type_params)
                    || runtime_simple_root_fallback_allowed(inner)
                        && runtime_simple_root_fallback_allowed(declared)
                        && declared.root_name() == inner.root_name()
            }
            _ => false,
        }
        || runtime_opaque_family_matches(plan, declared, actual, type_params)
        || runtime_simple_root_fallback_allowed(actual)
            && runtime_simple_root_fallback_allowed(declared)
            && declared.root_name() == Some(receiver_root)
        || declared.root_name() == Some(receiver_root)
            && match &actual.kind {
                IrRoutineTypeKind::Path(_) => runtime_simple_root_fallback_allowed(actual),
                IrRoutineTypeKind::Ref { inner, .. } => runtime_simple_root_fallback_allowed(inner),
                _ => false,
            }
}

fn runtime_value_type_root(receiver: &RuntimeValue) -> Option<String> {
    match receiver {
        RuntimeValue::Int(_) => Some("Int".to_string()),
        RuntimeValue::Float { kind, .. } => Some(
            match kind {
                ParsedFloatKind::F32 => "F32",
                ParsedFloatKind::F64 => "F64",
            }
            .to_string(),
        ),
        RuntimeValue::Bool(_) => Some("Bool".to_string()),
        RuntimeValue::Str(_) => Some("Str".to_string()),
        RuntimeValue::Bytes(_) => Some("Bytes".to_string()),
        RuntimeValue::ByteBuffer(_) => Some("ByteBuffer".to_string()),
        RuntimeValue::Utf16(_) => Some("Utf16".to_string()),
        RuntimeValue::Utf16Buffer(_) => Some("Utf16Buffer".to_string()),
        RuntimeValue::Pair(_, _) => Some("Pair".to_string()),
        RuntimeValue::Array(_) => Some("Array".to_string()),
        RuntimeValue::List(_) => Some("List".to_string()),
        RuntimeValue::Map(_) => Some("Map".to_string()),
        RuntimeValue::Range { .. } => Some("RangeInt".to_string()),
        RuntimeValue::OwnerHandle(_) => Some("Owner".to_string()),
        RuntimeValue::Ref(_) => Some("Ref".to_string()),
        RuntimeValue::Opaque(
            RuntimeOpaqueValue::ReadView(_)
            | RuntimeOpaqueValue::EditView(_)
            | RuntimeOpaqueValue::ByteView(_)
            | RuntimeOpaqueValue::ByteEditView(_)
            | RuntimeOpaqueValue::StrView(_),
        ) => Some("View".to_string()),
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
        | RuntimeValue::Float { .. }
        | RuntimeValue::Bool(_)
        | RuntimeValue::Range { .. }
        | RuntimeValue::OwnerHandle(_)
        | RuntimeValue::Ref(_)
        | RuntimeValue::Unit => true,
        RuntimeValue::Pair(left, right) => {
            runtime_value_is_copy(left) && runtime_value_is_copy(right)
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(binding)) => {
            matches!(
                binding.type_name,
                "arcana_winapi.types.WakeHandle" | "arcana_winapi.desktop_handles.WakeHandle"
            )
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicInt(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicBool(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(_)) => true,
        RuntimeValue::Str(_)
        | RuntimeValue::Bytes(_)
        | RuntimeValue::ByteBuffer(_)
        | RuntimeValue::Utf16(_)
        | RuntimeValue::Utf16Buffer(_)
        | RuntimeValue::Array(_)
        | RuntimeValue::List(_)
        | RuntimeValue::Map(_)
        | RuntimeValue::Record { .. }
        | RuntimeValue::Variant { .. } => false,
        RuntimeValue::Opaque(_) => false,
    }
}

fn runtime_validate_split_value(
    value: &RuntimeValue,
    state: &RuntimeExecutionState,
    context: &str,
) -> Result<(), String> {
    runtime_validate_split_value_with_ring_move(value, state, context, false)
}

fn runtime_validate_split_value_with_ring_move(
    value: &RuntimeValue,
    state: &RuntimeExecutionState,
    context: &str,
    allow_ring_move: bool,
) -> Result<(), String> {
    match value {
        RuntimeValue::Int(_)
        | RuntimeValue::Float { .. }
        | RuntimeValue::Bool(_)
        | RuntimeValue::Str(_)
        | RuntimeValue::Bytes(_)
        | RuntimeValue::ByteBuffer(_)
        | RuntimeValue::Utf16(_)
        | RuntimeValue::Utf16Buffer(_)
        | RuntimeValue::Unit
        | RuntimeValue::OwnerHandle(_)
        | RuntimeValue::Range { .. } => Ok(()),
        RuntimeValue::Ref(_) => Err(format!(
            "{context} cannot capture Ref values across split workers"
        )),
        RuntimeValue::Pair(left, right) => {
            runtime_validate_split_value_with_ring_move(left, state, context, allow_ring_move)?;
            runtime_validate_split_value_with_ring_move(right, state, context, allow_ring_move)
        }
        RuntimeValue::Array(values) | RuntimeValue::List(values) => {
            values.iter().try_for_each(|value| {
                runtime_validate_split_value_with_ring_move(value, state, context, allow_ring_move)
            })
        }
        RuntimeValue::Map(entries) => entries.iter().try_for_each(|(key, value)| {
            runtime_validate_split_value_with_ring_move(key, state, context, allow_ring_move)?;
            runtime_validate_split_value_with_ring_move(value, state, context, allow_ring_move)
        }),
        RuntimeValue::Record { fields, .. } => fields.values().try_for_each(|value| {
            runtime_validate_split_value_with_ring_move(value, state, context, allow_ring_move)
        }),
        RuntimeValue::Variant { payload, .. } => payload.iter().try_for_each(|value| {
            runtime_validate_split_value_with_ring_move(value, state, context, allow_ring_move)
        }),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ArenaId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::FrameId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::PoolId(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::TempId(_)) => Err(format!(
            "{context} cannot capture local memory families across split workers"
        )),
        RuntimeValue::Opaque(RuntimeOpaqueValue::Lazy(_)) => Err(format!(
            "{context} cannot capture lazy values across split workers; force them before split"
        )),
        RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(handle)) => {
            let arena = state
                .session_arenas
                .get(handle)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", handle.0))?;
            if arena.sealed {
                Ok(())
            } else {
                Err(format!(
                    "{context} can only capture SessionArena `{}` across split workers while sealed",
                    handle.0
                ))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::SessionId(id)) => {
            let arena = state
                .session_arenas
                .get(&id.arena)
                .ok_or_else(|| format!("invalid SessionArena handle `{}`", id.arena.0))?;
            if arena.sealed && session_id_is_live(id.arena, arena, *id) {
                Ok(())
            } else {
                Err(format!(
                    "{context} can only capture SessionId `{}` across split workers while the backing SessionArena is sealed and live",
                    runtime_value_to_string(value)
                ))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(handle)) => {
            let arena = state
                .slabs
                .get(handle)
                .ok_or_else(|| format!("invalid Slab handle `{}`", handle.0))?;
            if arena.sealed {
                Ok(())
            } else {
                Err(format!(
                    "{context} can only capture Slab `{}` across split workers while sealed",
                    handle.0
                ))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::SlabId(id)) => {
            let arena = state
                .slabs
                .get(&id.arena)
                .ok_or_else(|| format!("invalid Slab handle `{}`", id.arena.0))?;
            if arena.sealed && slab_id_is_live(id.arena, arena, *id) {
                Ok(())
            } else {
                Err(format!(
                    "{context} can only capture SlabId `{}` across split workers while the backing Slab is sealed and live",
                    runtime_value_to_string(value)
                ))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::RingId(_)) => {
            if allow_ring_move {
                Ok(())
            } else {
                Err(format!(
                    "{context} cannot capture ring memory across split workers without explicit move"
                ))
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::ReadView(handle)) => {
            let view = state
                .read_views
                .get(handle)
                .ok_or_else(|| format!("invalid ReadView handle `{}`", handle.0))?;
            match &view.backing {
                RuntimeElementViewBacking::Buffer(_) => Ok(()),
                RuntimeElementViewBacking::Reference(reference) => {
                    if runtime_reference_backed_descriptor_view_allowed(reference, state)? {
                        Ok(())
                    } else {
                        Err(format!(
                            "{context} can only capture reference-backed ReadView values across split workers when backed by sealed SessionArena/Slab storage"
                        ))
                    }
                }
                RuntimeElementViewBacking::RingWindow { .. } => Err(format!(
                    "{context} cannot capture RingBuffer-backed ReadView values across split workers; RingBuffer is move-only"
                )),
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::EditView(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::ByteEditView(_)) => Err(format!(
            "{context} cannot capture editable views across split workers"
        )),
        RuntimeValue::Opaque(RuntimeOpaqueValue::ByteView(handle)) => {
            let view = state
                .byte_views
                .get(handle)
                .ok_or_else(|| format!("invalid ByteView handle `{}`", handle.0))?;
            match &view.backing {
                RuntimeByteViewBacking::Buffer(_) => Ok(()),
                RuntimeByteViewBacking::Reference(reference) => {
                    if runtime_reference_backed_descriptor_view_allowed(reference, state)? {
                        Ok(())
                    } else {
                        Err(format!(
                            "{context} can only capture reference-backed ByteView values across split workers when backed by sealed SessionArena/Slab storage"
                        ))
                    }
                }
                RuntimeByteViewBacking::Foreign(_) => Err(format!(
                    "{context} cannot capture foreign-backed ByteView values across split workers"
                )),
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::StrView(handle)) => {
            let view = state
                .str_views
                .get(handle)
                .ok_or_else(|| format!("invalid StrView handle `{}`", handle.0))?;
            match &view.backing {
                RuntimeStrViewBacking::Buffer(_) => Ok(()),
                RuntimeStrViewBacking::Reference(reference) => {
                    if runtime_reference_backed_descriptor_view_allowed(reference, state)? {
                        Ok(())
                    } else {
                        Err(format!(
                            "{context} can only capture reference-backed StrView values across split workers when backed by sealed SessionArena/Slab storage"
                        ))
                    }
                }
            }
        }
        RuntimeValue::Opaque(RuntimeOpaqueValue::Task(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Binding(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Channel(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Mutex(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicInt(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::AtomicBool(_)) => Ok(()),
    }
}

fn runtime_split_arg_allows_ring_move(arg: &RuntimeCallArg, scopes: &[RuntimeScope]) -> bool {
    match expr_root_local_name(&arg.source_expr) {
        Some(name) => lookup_local(scopes, name).is_some_and(|local| local.moved),
        None => true,
    }
}

fn runtime_validate_split_scope_capture(
    scopes: &[RuntimeScope],
    call_args: &[RuntimeCallArg],
    state: &RuntimeExecutionState,
    context: &str,
) -> Result<(), String> {
    for scope in scopes {
        for (name, local) in &scope.locals {
            if local.moved {
                continue;
            }
            runtime_validate_split_value(
                &local.value,
                state,
                &format!("{context} local `{name}`"),
            )?;
        }
    }
    for (index, arg) in call_args.iter().enumerate() {
        runtime_validate_split_value_with_ring_move(
            &arg.value,
            state,
            &format!("{context} arg {}", index + 1),
            runtime_split_arg_allows_ring_move(arg, scopes),
        )?;
    }
    Ok(())
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
        ParsedExpr::Unary {
            op:
                ParsedUnaryOp::CapabilityRead
                | ParsedUnaryOp::CapabilityEdit
                | ParsedUnaryOp::CapabilityTake
                | ParsedUnaryOp::CapabilityHold
                | ParsedUnaryOp::Deref,
            expr,
        } => expr_to_assign_target(expr),
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

fn reserve_take_capability_root_local(
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
    if local.held {
        return Err(format!(
            "local `{name}` is suspended by an active `&hold` capability"
        ));
    }
    if local.take_reserved {
        return Err(format!(
            "local `{name}` is already reserved by an active `&take` capability"
        ));
    }
    local.take_reserved = true;
    Ok(())
}

fn reserve_hold_capability_root_local(
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
    if local.take_reserved {
        return Err(format!(
            "local `{name}` is reserved by an active `&take` capability"
        ));
    }
    if local.held {
        return Err(format!(
            "local `{name}` is already suspended by an active `&hold` capability"
        ));
    }
    local.held = true;
    Ok(())
}

fn reclaim_hold_capability_root_local(
    scopes: &mut [RuntimeScope],
    source_expr: &ParsedExpr,
) -> Result<(), String> {
    let Some(name) = expr_root_local_name(source_expr) else {
        return Err("`reclaim` expects a local `&hold[...]` capability binding".to_string());
    };
    let Some(local) = scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.locals.get_mut(name))
    else {
        return Err(format!(
            "`reclaim` capability binding `{name}` is unresolved at runtime"
        ));
    };
    local.moved = true;
    local.value = RuntimeValue::Unit;
    Ok(())
}

fn reclaim_held_target_local(
    scopes: &mut [RuntimeScope],
    target: &RuntimeReferenceTarget,
) -> Result<(), String> {
    let RuntimeReferenceTarget::Local { local, .. } = target else {
        return Err("`reclaim` requires a hold capability rooted in a local place".to_string());
    };
    let Some(runtime_local) = lookup_local_mut_by_handle(scopes, *local) else {
        return Err("`reclaim` target local is unresolved".to_string());
    };
    if !runtime_local.held {
        return Err("`reclaim` target is not currently held".to_string());
    }
    runtime_local.held = false;
    Ok(())
}

fn reserve_hold_arg_root_local(
    scopes: &mut [RuntimeScope],
    source_expr: &ParsedExpr,
) -> Result<Option<RuntimeLocalHandle>, String> {
    let Some(name) = expr_root_local_name(source_expr) else {
        return Ok(None);
    };
    let Some(local) = scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.locals.get_mut(name))
    else {
        return Ok(None);
    };
    if local.moved {
        return Err(format!("use of moved local `{name}`"));
    }
    if local.take_reserved {
        return Err(format!(
            "local `{name}` is reserved by an active `&take` capability"
        ));
    }
    if local.held {
        return Err(format!(
            "local `{name}` is already suspended by an active `&hold` capability"
        ));
    }
    local.held = true;
    Ok(Some(local.handle))
}

fn reserve_hold_bound_args(
    scopes: &mut [RuntimeScope],
    routine: &RuntimeRoutinePlan,
    args: &[BoundRuntimeArg],
) -> Result<Vec<RuntimeLocalHandle>, String> {
    let mut reserved = Vec::new();
    for (param, bound_arg) in routine.params.iter().zip(args) {
        if param.mode.as_deref() != Some("hold") {
            continue;
        }
        if let Some(handle) = reserve_hold_arg_root_local(scopes, &bound_arg.source_expr)? {
            reserved.push(handle);
        }
    }
    Ok(reserved)
}

fn release_reserved_hold_locals(scopes: &mut [RuntimeScope], handles: &[RuntimeLocalHandle]) {
    for handle in handles {
        if let Some(local) = lookup_local_mut_by_handle(scopes, *handle) {
            local.held = false;
        }
    }
}

fn validate_scope_hold_tokens(scope: &RuntimeScope) -> Result<(), String> {
    for (name, local) in &scope.locals {
        if local.moved {
            continue;
        }
        if let RuntimeValue::Ref(RuntimeReferenceValue {
            mode: RuntimeReferenceMode::Hold,
            target: RuntimeReferenceTarget::Local { local: target, .. },
        }) = &local.value
        {
            if scope
                .locals
                .values()
                .any(|candidate| candidate.handle == *target && !candidate.held)
            {
                continue;
            }
            return Err(format!(
                "local `{name}` holds an unreclaimed `&hold` capability at scope exit"
            ));
        }
    }
    Ok(())
}

fn runtime_reference_root_local_handle(
    target: &RuntimeReferenceTarget,
) -> Option<RuntimeLocalHandle> {
    match target {
        RuntimeReferenceTarget::Local { local, .. } => Some(*local),
        _ => None,
    }
}

fn redeem_take_reference(
    scopes: &mut [RuntimeScope],
    reference: &RuntimeReferenceValue,
) -> Result<(), String> {
    let Some(local) = runtime_reference_root_local_handle(&reference.target) else {
        return Ok(());
    };
    let Some(runtime_local) = lookup_local_mut_by_handle(scopes, local) else {
        return Err("`&take` capability target is unresolved".to_string());
    };
    if runtime_local.moved {
        return Err("`&take` capability was already redeemed".to_string());
    }
    if !runtime_local.take_reserved {
        return Err("`&take` capability is no longer active".to_string());
    }
    runtime_local.take_reserved = false;
    runtime_local.moved = true;
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

fn detach_moved_split_call_args(scopes: &[RuntimeScope], args: &mut [RuntimeCallArg]) {
    for arg in args {
        let Some(name) = expr_root_local_name(&arg.source_expr) else {
            continue;
        };
        if lookup_local(scopes, name).is_some_and(|local| local.moved) {
            arg.source_expr = ParsedExpr::Bool(true);
        }
    }
}

fn assign_record_member(
    plan: &RuntimePackagePlan,
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
            apply_runtime_struct_bitfield_layout(plan, &name, &mut fields)?;
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
            if local.take_reserved {
                return Err(format!(
                    "local `{name}` is reserved by an active `&take` capability"
                ));
            }
            if local.held {
                return Err(format!(
                    "local `{name}` is suspended by an active `&hold` capability"
                ));
            }
            match &local.value {
                RuntimeValue::Ref(reference) => Ok(RuntimeResolvedPlace {
                    mode: reference.mode.clone(),
                    target: reference.target.clone(),
                }),
                _ => Ok(RuntimeResolvedPlace {
                    mode: runtime_reference_mode_for_place(local.mutable),
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
                mode: place.mode,
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
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    target: &ParsedAssignTarget,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let place = resolve_assign_target_place(scopes, target)?;
    read_runtime_reference(
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        &RuntimeReferenceValue {
            mode: place.mode,
            target: place.target,
        },
        host,
    )
}

fn write_assign_target_value(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    target: &ParsedAssignTarget,
    value: RuntimeValue,
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    let place = resolve_assign_target_place(scopes, target)?;
    if !place.mode.allows_write() {
        let detail = match target {
            ParsedAssignTarget::Name(name) => match lookup_local(scopes, name) {
                Some(local) => match &local.value {
                    RuntimeValue::Ref(reference) => format!(
                        "local `{name}` mutable={}, moved={}, held={}, take_reserved={}, value=Ref(mode={})",
                        local.mutable,
                        local.moved,
                        local.held,
                        local.take_reserved,
                        reference.mode.as_str()
                    ),
                    other => format!(
                        "local `{name}` mutable={}, moved={}, held={}, take_reserved={}, value={}",
                        local.mutable,
                        local.moved,
                        local.held,
                        local.take_reserved,
                        runtime_value_to_string(other)
                    ),
                },
                None => format!("local `{name}` is unresolved"),
            },
            _ => "non-name assignment target".to_string(),
        };
        let call_stack = if state.call_stack.is_empty() {
            "<empty>".to_string()
        } else {
            state.call_stack.join(" -> ")
        };
        return Err(format!(
            "runtime assignment target `{target:?}` is not mutable ({detail}); call_stack={call_stack}"
        ));
    }
    write_runtime_reference(
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        &RuntimeReferenceValue {
            mode: place.mode,
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
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    let base = read_runtime_value_if_ref(
        base,
        scopes,
        plan,
        current_package_id,
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
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    match target {
        ParsedAssignTarget::Index { target, index } => eval_runtime_index_value(
            read_assign_target_value_runtime(
                target,
                plan,
                current_package_id,
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
                current_package_id,
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
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            host,
        ),
        _ => read_assign_target_value(
            scopes,
            plan,
            current_package_id,
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
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
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
                    current_package_id,
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
                    current_package_id,
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
                current_package_id,
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
                current_package_id,
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
            current_package_id,
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
        ParsedAssignOp::SubAssign => match (&current, &value) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (kind, left, right) = expect_same_float_operands(current, value, "-=")?;
                make_runtime_float(kind, left - right)
            }
            _ => RuntimeValue::Int(expect_int(current, "-=")? - expect_int(value, "-=")?),
        },
        ParsedAssignOp::MulAssign => match (&current, &value) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (kind, left, right) = expect_same_float_operands(current, value, "*=")?;
                make_runtime_float(kind, left * right)
            }
            _ => RuntimeValue::Int(expect_int(current, "*=")? * expect_int(value, "*=")?),
        },
        ParsedAssignOp::DivAssign => match (&current, &value) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (kind, left, right) = expect_same_float_operands(current, value, "/=")?;
                make_runtime_float(kind, left / right)
            }
            _ => RuntimeValue::Int(expect_int(current, "/=")? / expect_int(value, "/=")?),
        },
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
        (
            RuntimeValue::Float {
                text: left_text,
                kind: left_kind,
            },
            RuntimeValue::Float {
                text: right_text,
                kind: right_kind,
            },
        ) if left_kind == right_kind => {
            let left_value = parse_runtime_float_text(&left_text, left_kind)?;
            let right_value = parse_runtime_float_text(&right_text, right_kind)?;
            Ok(make_runtime_float(left_kind, left_value + right_value))
        }
        (RuntimeValue::Str(mut left), RuntimeValue::Str(right)) => {
            left.push_str(&right);
            Ok(RuntimeValue::Str(left))
        }
        (left, right) => Err(format!(
            "{context} expected Int, float, or Str operands of the same type, got `{left:?}` and `{right:?}`"
        )),
    }
}

fn write_back_call_args(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    skip_edit_arg_indices: &BTreeSet<usize>,
    edit_arg_indices: &[usize],
    call_args: &[RuntimeCallArg],
    final_args: &[RuntimeValue],
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    for &index in edit_arg_indices {
        if skip_edit_arg_indices.contains(&index) {
            continue;
        }
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
            current_package_id,
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
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    skip_edit_arg_indices: &BTreeSet<usize>,
    routine: &RuntimeRoutinePlan,
    args: &[BoundRuntimeArg],
    final_args: &[RuntimeValue],
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    for (index, param) in routine.params.iter().enumerate() {
        if param.mode.as_deref() != Some("edit") {
            continue;
        }
        if skip_edit_arg_indices.contains(&index) {
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
            current_package_id,
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

fn eval_runtime_member_value(base: RuntimeValue, member: &str) -> Result<RuntimeValue, String> {
    match base {
        RuntimeValue::Ref(reference) => Ok(RuntimeValue::Ref(RuntimeReferenceValue {
            mode: reference.mode,
            target: runtime_reference_with_member(&reference.target, member.to_string()),
        })),
        other => eval_member_value(other, member),
    }
}

fn read_runtime_value_if_ref(
    value: RuntimeValue,
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    match value {
        RuntimeValue::Ref(reference) => {
            let value = read_runtime_reference(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                &reference,
                host,
            )?;
            read_runtime_value_if_ref(
                force_runtime_value(value, plan, state, host)?,
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )
        }
        RuntimeValue::Record { name, fields } => {
            let mut materialized = BTreeMap::new();
            for (key, value) in fields {
                materialized.insert(
                    key,
                    read_runtime_value_if_ref(
                        value,
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?,
                );
            }
            Ok(RuntimeValue::Record {
                name,
                fields: materialized,
            })
        }
        RuntimeValue::Pair(left, right) => Ok(RuntimeValue::Pair(
            Box::new(read_runtime_value_if_ref(
                *left,
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )?),
            Box::new(read_runtime_value_if_ref(
                *right,
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                host,
            )?),
        )),
        RuntimeValue::List(values) => Ok(RuntimeValue::List(
            values
                .into_iter()
                .map(|value| {
                    read_runtime_value_if_ref(
                        value,
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        RuntimeValue::Array(values) => Ok(RuntimeValue::Array(
            values
                .into_iter()
                .map(|value| {
                    read_runtime_value_if_ref(
                        value,
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        RuntimeValue::Variant { name, payload } => Ok(RuntimeValue::Variant {
            name,
            payload: payload
                .into_iter()
                .map(|value| {
                    read_runtime_value_if_ref(
                        value,
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        }),
        other => force_runtime_value(other, plan, state, host),
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

fn runtime_view_bounds(
    start: i64,
    end: i64,
    len: usize,
    context: &str,
) -> Result<(usize, usize), String> {
    if start < 0 {
        return Err(format!("{context} start must be non-negative"));
    }
    if end < 0 {
        return Err(format!("{context} end must be non-negative"));
    }
    let start = usize::try_from(start)
        .map_err(|_| format!("{context} start `{start}` does not fit in usize"))?;
    let end =
        usize::try_from(end).map_err(|_| format!("{context} end `{end}` does not fit in usize"))?;
    if start > end {
        return Err(format!(
            "{context} start `{start}` must be less than or equal to end `{end}`"
        ));
    }
    if end > len {
        return Err(format!(
            "{context} slice `{start}..{end}` is out of bounds for length `{len}`"
        ));
    }
    Ok((start, end))
}

fn runtime_view_range(
    start: usize,
    len: usize,
    total_len: usize,
    context: &str,
) -> Result<(usize, usize), String> {
    let end = start
        .checked_add(len)
        .ok_or_else(|| format!("{context} range overflowed"))?;
    if end > total_len {
        return Err(format!(
            "{context} `{start}..{end}` is out of bounds for length `{total_len}`"
        ));
    }
    Ok((start, end))
}

fn runtime_reference_array_values(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<Vec<RuntimeValue>, String> {
    expect_runtime_array(
        read_runtime_reference(
            scopes,
            plan,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            reference,
            host,
        )?,
        context,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeByteReferenceCarrierKind {
    Array,
    ByteBuffer,
    Bytes,
}

fn runtime_reference_byte_values(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<Vec<u8>, String> {
    expect_byte_array(
        read_runtime_reference(
            scopes,
            plan,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            reference,
            host,
        )?,
        context,
    )
}

fn runtime_reference_byte_carrier(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<(RuntimeByteReferenceCarrierKind, Vec<u8>), String> {
    match read_runtime_reference(
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        reference,
        host,
    )? {
        RuntimeValue::Array(values) => Ok((
            RuntimeByteReferenceCarrierKind::Array,
            expect_byte_array(RuntimeValue::Array(values), context)?,
        )),
        RuntimeValue::ByteBuffer(values) => {
            Ok((RuntimeByteReferenceCarrierKind::ByteBuffer, values))
        }
        RuntimeValue::Bytes(values) => Ok((RuntimeByteReferenceCarrierKind::Bytes, values)),
        other => Err(format!(
            "{context} expected Array, Bytes, or ByteBuffer backing, got `{other:?}`"
        )),
    }
}

fn runtime_write_byte_reference(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    carrier: RuntimeByteReferenceCarrierKind,
    values: Vec<u8>,
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    let value = match carrier {
        RuntimeByteReferenceCarrierKind::Array => bytes_to_runtime_array(values),
        RuntimeByteReferenceCarrierKind::ByteBuffer => RuntimeValue::ByteBuffer(values),
        RuntimeByteReferenceCarrierKind::Bytes => {
            return Err("cannot write through an immutable Bytes projection".to_string());
        }
    };
    write_runtime_reference(
        scopes,
        plan,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        reference,
        value,
        host,
    )
}

fn runtime_reference_text_value(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    reference: &RuntimeReferenceValue,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<String, String> {
    expect_str(
        read_runtime_reference(
            scopes,
            plan,
            current_package_id,
            current_module_id,
            aliases,
            type_bindings,
            state,
            reference,
            host,
        )?,
        context,
    )
}

fn runtime_read_view_values(
    scopes: &mut Vec<RuntimeScope>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    view: &RuntimeReadViewState,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> Result<Vec<RuntimeValue>, String> {
    match &view.backing {
        RuntimeElementViewBacking::Buffer(buffer) => {
            let values = &state
                .element_view_buffers
                .get(buffer)
                .ok_or_else(|| format!("invalid element view buffer `{}`", buffer.0))?
                .values;
            let (start, end) = runtime_view_range(view.start, view.len, values.len(), context)?;
            Ok(values[start..end].to_vec())
        }
        RuntimeElementViewBacking::Reference(reference) => {
            let values = runtime_reference_array_values(
                scopes,
                plan,
                current_package_id,
                current_module_id,
                aliases,
                type_bindings,
                state,
                reference,
                host,
                context,
            )?;
            let (start, end) = runtime_view_range(view.start, view.len, values.len(), context)?;
            Ok(values[start..end].to_vec())
        }
        RuntimeElementViewBacking::RingWindow { arena, slots } => {
            let active = runtime_ring_window_active_slots(slots, view.start, view.len)
                .ok_or_else(|| {
                    format!(
                        "{context} view range `{}`..`{}` is out of bounds for ring window length `{}`",
                        view.start,
                        view.start + view.len,
                        slots.len()
                    )
                })?;
            let ring = state
                .ring_buffers
                .get(arena)
                .ok_or_else(|| format!("invalid RingBuffer handle `{}`", arena.0))?;
            active
                .iter()
                .map(|slot| {
                    ring.slots
                        .get(slot)
                        .cloned()
                        .ok_or_else(|| format!("RingBuffer slot `{slot}` is missing"))
                })
                .collect()
        }
    }
}

fn eval_optional_runtime_int_expr(
    expr: Option<&ParsedExpr>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    context: &str,
) -> RuntimeEvalResult<Option<i64>> {
    expr.map(|expr| {
        expect_int(
            eval_expr(
                expr,
                plan,
                current_package_id,
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
    .transpose()
}

fn eval_match_expr(
    subject: &ParsedExpr,
    arms: &[ParsedMatchArm],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let subject = read_runtime_value_if_ref(
        eval_expr(
            subject,
            plan,
            current_package_id,
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
        current_package_id,
        current_module_id,
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
                insert_runtime_local(state, scopes.len(), &mut scope, 0, name, false, value);
            }
            scopes.push(scope);
            let result = eval_expr(
                &arm.value,
                plan,
                current_package_id,
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
    Err(format!("runtime match expression had no matching arm for subject `{subject:?}`").into())
}

fn err_variant(message: String) -> RuntimeValue {
    RuntimeValue::Variant {
        name: "Result.Err".to_string(),
        payload: vec![RuntimeValue::Str(message)],
    }
}

fn expect_cleanup_outcome(value: RuntimeValue) -> Result<(), String> {
    match value {
        RuntimeValue::Unit => Ok(()),
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Ok") => {
            match payload.as_slice() {
                [RuntimeValue::Unit] => Ok(()),
                _ => Err(format!(
                    "cleanup footer expected Result.Ok(Unit), got `{name}`"
                )),
            }
        }
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Err") => {
            match payload.as_slice() {
                [RuntimeValue::Str(message)] => Err(message.clone()),
                [other] => Err(format!(
                    "cleanup footer expected Result.Err(Str), got `{other:?}`"
                )),
                _ => Err(format!(
                    "cleanup footer expected Result.Err(Str), got `{name}`"
                )),
            }
        }
        other => Err(format!(
            "cleanup footer expected Result[Unit, Str], got `{other:?}`"
        )),
    }
}

fn try_construct_record_value(
    callable: &[String],
    resolved_type_args: &[String],
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
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
                current_package_id,
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
    apply_runtime_struct_bitfield_layout(plan, &name, &mut fields)
        .map_err(RuntimeEvalSignal::from)?;
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
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
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
            current_package_id,
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

fn truncate_float_to_int(value: f64, context: &str) -> Result<i64, String> {
    if !value.is_finite() {
        return Err(format!("{context} requires a finite float value"));
    }
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return Err(format!(
            "{context} float value `{value}` is out of i64 range"
        ));
    }
    Ok(value.trunc() as i64)
}

fn validate_int_width(value: i64, bits: u32, signed: bool, context: &str) -> Result<i64, String> {
    let valid = if signed {
        let shift = bits.saturating_sub(1);
        let min = -(1_i128 << shift);
        let max = (1_i128 << shift) - 1;
        (value as i128) >= min && (value as i128) <= max
    } else {
        value >= 0 && (value as u128) <= ((1_u128 << bits) - 1)
    };
    if valid {
        Ok(value)
    } else {
        Err(format!(
            "{context} value `{value}` is out of range for {}-bit {} integer",
            bits,
            if signed { "signed" } else { "unsigned" }
        ))
    }
}

fn convert_runtime_numeric_value(
    target: &str,
    value: RuntimeValue,
) -> Result<RuntimeValue, String> {
    match target {
        "F32" => match value {
            RuntimeValue::Int(value) => Ok(make_runtime_float(ParsedFloatKind::F32, value as f64)),
            RuntimeValue::Float { text, kind } => Ok(make_runtime_float(
                ParsedFloatKind::F32,
                parse_runtime_float_text(&text, kind)?,
            )),
            other => Err(format!(
                "F32 conversion expected numeric input, got `{other:?}`"
            )),
        },
        "F64" => match value {
            RuntimeValue::Int(value) => Ok(make_runtime_float(ParsedFloatKind::F64, value as f64)),
            RuntimeValue::Float { text, kind } => Ok(make_runtime_float(
                ParsedFloatKind::F64,
                parse_runtime_float_text(&text, kind)?,
            )),
            other => Err(format!(
                "F64 conversion expected numeric input, got `{other:?}`"
            )),
        },
        "Int" | "I8" | "U8" | "I16" | "U16" | "I32" | "U32" | "I64" | "U64" | "ISize" | "USize" => {
            let context = format!("{target} conversion");
            let int_value = match value {
                RuntimeValue::Int(value) => value,
                RuntimeValue::Float { text, kind } => {
                    truncate_float_to_int(parse_runtime_float_text(&text, kind)?, &context)?
                }
                other => {
                    return Err(format!(
                        "{target} conversion expected numeric input, got `{other:?}`"
                    ));
                }
            };
            let checked = match target {
                "I8" => validate_int_width(int_value, 8, true, &context)?,
                "U8" => validate_int_width(int_value, 8, false, &context)?,
                "I16" => validate_int_width(int_value, 16, true, &context)?,
                "U16" => validate_int_width(int_value, 16, false, &context)?,
                "I32" => validate_int_width(int_value, 32, true, &context)?,
                "U32" => validate_int_width(int_value, 32, false, &context)?,
                "I64" | "ISize" | "Int" => int_value,
                "U64" | "USize" => {
                    if int_value < 0 {
                        return Err(format!(
                            "{context} value `{int_value}` must be non-negative"
                        ));
                    }
                    int_value
                }
                _ => unreachable!(),
            };
            Ok(RuntimeValue::Int(checked))
        }
        _ => Err(format!("unsupported numeric conversion target `{target}`")),
    }
}

fn try_execute_builtin_numeric_conversion(
    callable: &[String],
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
    let [target] = callable else {
        return Ok(None);
    };
    if !matches!(
        target.as_str(),
        "Int"
            | "I8"
            | "U8"
            | "I16"
            | "U16"
            | "I32"
            | "U32"
            | "I64"
            | "U64"
            | "ISize"
            | "USize"
            | "F32"
            | "F64"
    ) {
        return Ok(None);
    }
    if !attached.is_empty() {
        return Err(format!("`{target}` conversion does not support attached blocks").into());
    }
    if args.len() != 1 || args[0].name.is_some() {
        return Err(
            format!("`{target}` conversion expects exactly one positional argument").into(),
        );
    }
    let value = eval_expr(
        &args[0].value,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    Ok(Some(
        convert_runtime_numeric_value(target, value).map_err(RuntimeEvalSignal::from)?,
    ))
}

fn try_construct_array_value(
    _callable: &[String],
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
    if !attached.is_empty() || args.iter().any(|arg| arg.name.is_some()) {
        return Ok(None);
    }
    let values = args
        .iter()
        .map(|arg| {
            eval_expr(
                &arg.value,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(RuntimeValue::Array(values)))
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
    current_package_id: &str,
    current_module_id: &str,
    callable: &[String],
    call_args: &[RuntimeCallArg],
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    allow_receiver_root_fallback: bool,
    state: Option<&RuntimeExecutionState>,
) -> Result<Option<usize>, String> {
    if let Some(routine_key) = resolved_routine
        && let Some(index) = resolve_lowered_routine_index(plan, current_package_id, routine_key)?
    {
        return Ok(Some(index));
    }
    let bare_receiver_lookup =
        dynamic_dispatch.is_none() && allow_receiver_root_fallback && callable.len() == 1;
    let candidates = match dynamic_dispatch {
        Some(ParsedDynamicDispatch::TraitMethod { trait_path }) => {
            let Some(method_name) = callable.last() else {
                return Ok(None);
            };
            resolve_dynamic_method_candidate_indices(plan, method_name, trait_path)
        }
        None if bare_receiver_lookup => {
            resolve_method_candidate_indices_by_name(plan, &callable[0])
        }
        None => {
            resolve_routine_candidate_indices(plan, current_package_id, current_module_id, callable)
        }
    };
    if candidates.is_empty() {
        return Ok(None);
    }
    if dynamic_dispatch.is_none() && candidates.len() == 1 && !bare_receiver_lookup {
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
    let receiver_type = state
        .and_then(|state| runtime_value_type(receiver, state))
        .or_else(|| runtime_value_type_without_state(receiver));
    let Some(receiver_root) = receiver_type
        .as_ref()
        .and_then(IrRoutineType::root_name)
        .map(str::to_string)
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
                        .as_ref()
                        .or_else(|| routine.params.first().map(|param| &param.ty));
                    let Some(declared) = declared else {
                        return false;
                    };
                    receiver_type
                        .as_ref()
                        .map(|actual| {
                            runtime_receiver_matches_declared_type(
                                plan,
                                declared,
                                actual,
                                &routine.type_params,
                                &receiver_root,
                            )
                        })
                        .unwrap_or_else(|| declared.root_name() == Some(receiver_root.as_str()))
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

fn resolve_runtime_receiver_intrinsic_fallback(
    callable: &[String],
    call_args: &[RuntimeCallArg],
) -> Option<RuntimeIntrinsic> {
    if callable.len() != 1 {
        return None;
    }
    let method = callable[0].as_str();
    let receiver = &call_args.first()?.value;
    match receiver {
        RuntimeValue::Bytes(_) => match method {
            "len" => Some(RuntimeIntrinsic::BytesLen),
            "at" => Some(RuntimeIntrinsic::BytesAt),
            "slice" => Some(RuntimeIntrinsic::BytesSlice),
            "sha256_hex" => Some(RuntimeIntrinsic::BytesSha256Hex),
            "thaw" => Some(RuntimeIntrinsic::BytesThaw),
            _ => None,
        },
        RuntimeValue::ByteBuffer(_) => match method {
            "len" => Some(RuntimeIntrinsic::ByteBufferLen),
            "at" => Some(RuntimeIntrinsic::ByteBufferAt),
            "set" => Some(RuntimeIntrinsic::ByteBufferSet),
            "push" => Some(RuntimeIntrinsic::ByteBufferPush),
            "freeze" => Some(RuntimeIntrinsic::ByteBufferFreeze),
            _ => None,
        },
        RuntimeValue::Utf16(_) => match method {
            "len" => Some(RuntimeIntrinsic::Utf16Len),
            "at" => Some(RuntimeIntrinsic::Utf16At),
            "slice" => Some(RuntimeIntrinsic::Utf16Slice),
            "thaw" => Some(RuntimeIntrinsic::Utf16Thaw),
            _ => None,
        },
        RuntimeValue::Utf16Buffer(_) => match method {
            "len" => Some(RuntimeIntrinsic::Utf16BufferLen),
            "at" => Some(RuntimeIntrinsic::Utf16BufferAt),
            "set" => Some(RuntimeIntrinsic::Utf16BufferSet),
            "push" => Some(RuntimeIntrinsic::Utf16BufferPush),
            "freeze" => Some(RuntimeIntrinsic::Utf16BufferFreeze),
            _ => None,
        },
        _ => None,
    }
}

fn resolve_cleanup_footer_handler_callable_path(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    handler_path: &[String],
) -> Vec<String> {
    let Some((first, suffix)) = handler_path.split_first() else {
        return Vec::new();
    };
    if let Some(prefix) = plan
        .module_aliases
        .get(&module_alias_scope_key(
            current_package_id,
            current_module_id,
        ))
        .and_then(|aliases| aliases.get(first))
    {
        let mut callable = prefix.clone();
        callable.extend(suffix.iter().cloned());
        return callable;
    }
    handler_path.to_vec()
}

fn cleanup_footer_return_type_is_valid(return_type: Option<&IrRoutineType>) -> bool {
    let Some(return_type) = return_type else {
        return false;
    };
    let IrRoutineTypeKind::Apply { base, args } = &return_type.kind else {
        return false;
    };
    if base.root_name() != Some("Result") || args.len() != 2 {
        return false;
    }
    matches!(
        (&args[0].kind, &args[1].kind),
        (
            IrRoutineTypeKind::Path(unit_path),
            IrRoutineTypeKind::Path(str_path),
        ) if unit_path.root_name() == Some("Unit") && str_path.root_name() == Some("Str")
    )
}

fn validate_cleanup_footer_handler_routine_plan(
    routine: &RuntimeRoutinePlan,
    handler_label: &str,
) -> Result<(), String> {
    if routine.is_async {
        return Err(format!(
            "cleanup footer handler `{handler_label}` cannot be async in v1"
        ));
    }
    if routine.params.len() != 1 {
        return Err(format!(
            "cleanup footer handler `{handler_label}` must accept exactly one parameter in v1"
        ));
    }
    if routine.params[0].mode.as_deref() != Some("take") {
        return Err(format!(
            "cleanup footer handler `{handler_label}` must take its target parameter in v1"
        ));
    }
    if !cleanup_footer_return_type_is_valid(routine.return_type.as_ref()) {
        return Err(format!(
            "cleanup footer handler `{handler_label}` must return `Result[Unit, Str]` in v1"
        ));
    }
    Ok(())
}

fn validate_runtime_cleanup_footer_handler(
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    cleanup_footer: &ParsedCleanupFooter,
) -> Result<Vec<String>, String> {
    if cleanup_footer.handler_path == [String::from("cleanup")]
        && let Some(routine_key) = cleanup_footer.resolved_routine.as_deref()
    {
        let routine = plan
            .routines
            .iter()
            .find(|routine| routine.routine_key == routine_key)
            .ok_or_else(|| {
                format!(
                    "cleanup footer handler `cleanup` resolved to missing routine `{routine_key}`"
                )
            })?;
        validate_cleanup_footer_handler_routine_plan(routine, "cleanup")?;
        return Ok(cleanup_footer.handler_path.clone());
    }
    let handler_path = &cleanup_footer.handler_path;
    let callable = resolve_cleanup_footer_handler_callable_path(
        plan,
        current_package_id,
        current_module_id,
        handler_path,
    );
    if let Some(routine_index) =
        resolve_routine_index(plan, current_package_id, current_module_id, &callable)
    {
        let routine = plan.routines.get(routine_index).ok_or_else(|| {
            format!(
                "cleanup footer handler `{}` resolved to invalid routine index `{routine_index}`",
                handler_path.join(".")
            )
        })?;
        validate_cleanup_footer_handler_routine_plan(routine, &handler_path.join("."))?;
        return Ok(callable);
    }
    Err(format!(
        "cleanup footer handler `{}` does not resolve to a callable path",
        handler_path.join(".")
    ))
}

fn execute_call_by_path(
    callable: &[String],
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    current_package_id: &str,
    current_module_id: &str,
    type_args: Vec<String>,
    call_args: Vec<RuntimeCallArg>,
    allow_receiver_root_fallback: bool,
    plan: &RuntimePackagePlan,
    scopes: &mut Vec<RuntimeScope>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    allow_async: bool,
) -> RuntimeEvalResult<RuntimeValue> {
    if let Some(value) = try_execute_arcana_owned_api_call(
        callable,
        &call_args,
        scopes,
        plan,
        current_package_id,
        current_module_id,
        &BTreeMap::new(),
        &BTreeMap::new(),
        state,
        host,
    )? {
        return Ok(value);
    }
    let receiver_fallback_intrinsic = allow_receiver_root_fallback
        .then(|| resolve_runtime_receiver_intrinsic_fallback(callable, &call_args))
        .flatten();
    let routine_index = match resolve_routine_index_for_call(
        plan,
        current_package_id,
        current_module_id,
        callable,
        &call_args,
        resolved_routine,
        dynamic_dispatch,
        allow_receiver_root_fallback,
        Some(state),
    ) {
        Ok(index) => index,
        Err(err)
            if call_args
                .first()
                .is_some_and(|arg| matches!(arg.value, RuntimeValue::Ref(_)))
                && err.contains("has no overload matching receiver `Ref`") =>
        {
            let mut probe_args = call_args.clone();
            if let Some(receiver) = probe_args.first_mut() {
                receiver.value = read_runtime_value_if_ref(
                    receiver.value.clone(),
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    state,
                    host,
                )?;
            }
            match resolve_routine_index_for_call(
                plan,
                current_package_id,
                current_module_id,
                callable,
                &probe_args,
                resolved_routine,
                dynamic_dispatch,
                allow_receiver_root_fallback,
                Some(state),
            ) {
                Ok(index) => index,
                Err(retry_err)
                    if receiver_fallback_intrinsic.is_some()
                        && retry_err.contains("has no overload matching receiver") =>
                {
                    None
                }
                Err(retry_err) => return Err(retry_err.into()),
            }
        }
        Err(err)
            if receiver_fallback_intrinsic.is_some()
                && err.contains("has no overload matching receiver") =>
        {
            None
        }
        Err(err) => return Err(err.into()),
    };
    if let Some(routine_index) = routine_index {
        let routine = plan
            .routines
            .get(routine_index)
            .ok_or_else(|| format!("invalid routine index `{routine_index}`"))?;
        let bound_args = bind_call_args_for_routine(routine, call_args)?;
        consume_take_bound_args(scopes, routine, &bound_args)?;
        let held_locals = reserve_hold_bound_args(scopes, routine, &bound_args)?;
        let values = bound_args
            .iter()
            .map(|arg| arg.value.clone())
            .collect::<Vec<_>>();
        let execution_args = routine
            .params
            .iter()
            .zip(bound_args.iter())
            .map(|(param, arg)| {
                runtime_execution_arg_for_bound_param(
                    scopes,
                    plan,
                    current_package_id,
                    current_module_id,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    state,
                    param,
                    arg,
                    host,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let outcome = (|| -> RuntimeEvalResult<RoutineExecutionOutcome> {
            if let Some(intrinsic_impl) = &routine.intrinsic_impl {
                let intrinsic =
                    resolve_runtime_intrinsic_impl(intrinsic_impl).ok_or_else(|| {
                        format!(
                            "unsupported runtime intrinsic `{intrinsic_impl}` for `{}`",
                            routine.symbol_name
                        )
                    })?;
                let mut final_args = values;
                for (index, param) in routine.params.iter().enumerate() {
                    if param.mode.as_deref() == Some("edit") {
                        continue;
                    }
                    final_args[index] = read_runtime_value_if_ref(
                        final_args[index].clone(),
                        scopes,
                        plan,
                        current_package_id,
                        current_module_id,
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        state,
                        host,
                    )?;
                }
                let value = execute_runtime_intrinsic(
                    intrinsic,
                    &type_args,
                    &mut final_args,
                    plan,
                    Some(scopes),
                    Some(current_package_id),
                    Some(current_module_id),
                    Some(&BTreeMap::new()),
                    Some(&BTreeMap::new()),
                    state,
                    host,
                )?;
                Ok(RoutineExecutionOutcome {
                    value,
                    final_args,
                    skip_write_back_edit_indices: BTreeSet::new(),
                    control: None,
                })
            } else if let Some(native_impl) = &routine.native_impl {
                execute_runtime_native_binding_import(
                    plan,
                    routine,
                    native_impl,
                    &values,
                    scopes,
                    current_package_id,
                    current_module_id,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    state,
                    host,
                )
                .map_err(RuntimeEvalSignal::from)
            } else {
                execute_routine_call_with_state(
                    plan,
                    routine_index,
                    type_args,
                    execution_args,
                    &collect_active_owner_keys_from_scopes(scopes),
                    None,
                    None,
                    None,
                    None,
                    None,
                    state,
                    host,
                    allow_async,
                )
            }
        })();
        release_reserved_hold_locals(scopes, &held_locals);
        let outcome = outcome?;
        write_back_bound_args(
            scopes,
            plan,
            current_package_id,
            current_module_id,
            &BTreeMap::new(),
            &BTreeMap::new(),
            state,
            &outcome.skip_write_back_edit_indices,
            routine,
            &bound_args,
            &outcome.final_args,
            host,
        )?;
        if let Some(FlowSignal::OwnerExit {
            owner_key,
            exit_name,
        }) = outcome.control
        {
            return Err(RuntimeEvalSignal::OwnerExit {
                owner_key,
                exit_name,
            });
        }
        return Ok(outcome.value);
    }
    let intrinsic = receiver_fallback_intrinsic
        .or_else(|| resolve_runtime_intrinsic_path(callable))
        .ok_or_else(|| format!("unsupported runtime callable `{}`", callable.join(".")))?;
    let call_args = bind_call_args_for_intrinsic(callable, call_args)?;
    consume_take_call_args(scopes, take_arg_indices(intrinsic), &call_args)?;
    let mut values = call_args
        .iter()
        .map(|arg| arg.value.clone())
        .collect::<Vec<_>>();
    let edit_indices = edit_arg_indices(intrinsic);
    for (index, value) in values.iter_mut().enumerate() {
        if edit_indices.contains(&index) {
            continue;
        }
        *value = read_runtime_value_if_ref(
            value.clone(),
            scopes,
            plan,
            current_package_id,
            current_module_id,
            &BTreeMap::new(),
            &BTreeMap::new(),
            state,
            host,
        )?;
    }
    let value = execute_runtime_intrinsic(
        intrinsic,
        &type_args,
        &mut values,
        plan,
        Some(scopes),
        Some(current_package_id),
        Some(current_module_id),
        Some(&BTreeMap::new()),
        Some(&BTreeMap::new()),
        state,
        host,
    )?;
    let skip_edit_arg_indices = BTreeSet::new();
    write_back_call_args(
        scopes,
        plan,
        current_package_id,
        current_module_id,
        &BTreeMap::new(),
        &BTreeMap::new(),
        state,
        &skip_edit_arg_indices,
        edit_arg_indices(intrinsic),
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
    qualifier_type_args: &[String],
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
    allow_async: bool,
) -> RuntimeEvalResult<RuntimeValue> {
    let callable = resolved_callable
        .map(|path| path.to_vec())
        .or_else(|| resolve_callable_path(subject, aliases))
        .ok_or_else(|| format!("unsupported runtime callable `{subject:?}`"))?;
    let type_args = if qualifier_type_args.is_empty() {
        resolve_runtime_type_args(&extract_generic_type_args(subject), type_bindings)
    } else {
        qualifier_type_args.to_vec()
    };
    if let Some(value) = try_execute_builtin_numeric_conversion(
        &callable,
        args,
        attached,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )? {
        return Ok(value);
    }
    let has_lowered_runtime_routine = match resolved_routine {
        Some(routine_key) => resolve_lowered_routine_index(plan, current_package_id, routine_key)
            .map_err(RuntimeEvalSignal::from)?
            .is_some(),
        None => false,
    };
    let has_runtime_routine = has_lowered_runtime_routine
        || resolve_routine_index(plan, current_package_id, current_module_id, &callable).is_some();
    let has_runtime_intrinsic = resolve_runtime_intrinsic_path(&callable).is_some()
        || runtime_arcana_owned_callable_key(&callable).is_some();
    if !has_runtime_routine && !has_runtime_intrinsic {
        // Constructor fallback is only valid when lowering did not identify a routine call.
        if let Some(record) = try_construct_record_value(
            &callable,
            &type_args,
            args,
            attached,
            plan,
            current_package_id,
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
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )? {
            return Ok(variant);
        }
        if let Some(array) = try_construct_array_value(
            &callable,
            args,
            attached,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )? {
            return Ok(array);
        }
        return Err(format!("unsupported runtime callable `{}`", callable.join(".")).into());
    }
    let call_args = collect_call_args(
        args,
        attached,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    execute_call_by_path(
        &callable,
        resolved_routine,
        None,
        current_package_id,
        current_module_id,
        type_args,
        call_args,
        false,
        plan,
        scopes,
        state,
        host,
        allow_async,
    )
}

fn execute_runtime_method_call_with_type_args(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier: &str,
    qualifier_type_args: &[String],
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let receiver = eval_expr(
        subject,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let callable = resolved_callable
        .map(|callable| callable.to_vec())
        .unwrap_or_else(|| vec![qualifier.to_string()]);
    let type_args = if qualifier_type_args.is_empty() {
        runtime_receiver_type_args(&receiver, state)
    } else {
        qualifier_type_args.to_vec()
    };
    let mut call_args = vec![RuntimeCallArg {
        name: None,
        value: receiver,
        source_expr: subject.clone(),
    }];
    call_args.extend(collect_call_args(
        args,
        attached,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?);
    execute_call_by_path(
        &callable,
        resolved_routine,
        dynamic_dispatch,
        current_package_id,
        current_module_id,
        type_args,
        call_args,
        resolved_routine.is_none(),
        plan,
        scopes,
        state,
        host,
        runtime_async_calls_allowed(state),
    )
}

fn execute_runtime_named_qualifier_call(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier: &str,
    qualifier_type_args: &[String],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let receiver = eval_expr(
        subject,
        plan,
        current_package_id,
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
    let type_args = if qualifier_type_args.is_empty() {
        resolve_runtime_type_args(&extract_generic_type_args(&callable_expr), type_bindings)
    } else {
        qualifier_type_args.to_vec()
    };
    let mut call_args = vec![RuntimeCallArg {
        name: None,
        value: receiver,
        source_expr: subject.clone(),
    }];
    call_args.extend(collect_call_args(
        args,
        attached,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?);
    execute_call_by_path(
        &callable,
        None,
        None,
        current_package_id,
        current_module_id,
        type_args,
        call_args,
        false,
        plan,
        scopes,
        state,
        host,
        runtime_async_calls_allowed(state),
    )
}

fn eval_qualifier(
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier: &str,
    qualifier_type_args: &[String],
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    execute_runtime_method_call_with_type_args(
        subject,
        args,
        attached,
        qualifier,
        qualifier_type_args,
        resolved_callable,
        resolved_routine,
        dynamic_dispatch,
        plan,
        current_package_id,
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
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
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
        current_package_id,
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
    scopes: Option<&mut Vec<RuntimeScope>>,
    current_package_id: Option<&str>,
    current_module_id: Option<&str>,
    aliases: Option<&BTreeMap<String, Vec<String>>>,
    type_bindings: Option<&RuntimeTypeBindings>,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    execute_runtime_core_intrinsic(
        intrinsic,
        type_args,
        final_args,
        plan,
        scopes,
        current_package_id,
        current_module_id,
        aliases,
        type_bindings,
        state,
        host,
    )
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
            RuntimeValue::Float { text: value, kind } => parse_runtime_float_text(text, *kind)
                .ok()
                .zip(parse_runtime_float_text(value, *kind).ok())
                .is_some_and(|(literal, actual)| literal == actual),
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
        ParsedBinaryOp::EqEq => Ok(RuntimeValue::Bool(runtime_values_equal(&left, &right))),
        ParsedBinaryOp::NotEq => Ok(RuntimeValue::Bool(!runtime_values_equal(&left, &right))),
        ParsedBinaryOp::Lt => match (&left, &right) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (_, left, right) = expect_same_float_operands(left, right, "<")?;
                Ok(RuntimeValue::Bool(left < right))
            }
            _ => Ok(RuntimeValue::Bool(
                expect_int(left, "<")? < expect_int(right, "<")?,
            )),
        },
        ParsedBinaryOp::LtEq => match (&left, &right) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (_, left, right) = expect_same_float_operands(left, right, "<=")?;
                Ok(RuntimeValue::Bool(left <= right))
            }
            _ => Ok(RuntimeValue::Bool(
                expect_int(left, "<=")? <= expect_int(right, "<=")?,
            )),
        },
        ParsedBinaryOp::Gt => match (&left, &right) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (_, left, right) = expect_same_float_operands(left, right, ">")?;
                Ok(RuntimeValue::Bool(left > right))
            }
            _ => Ok(RuntimeValue::Bool(
                expect_int(left, ">")? > expect_int(right, ">")?,
            )),
        },
        ParsedBinaryOp::GtEq => match (&left, &right) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (_, left, right) = expect_same_float_operands(left, right, ">=")?;
                Ok(RuntimeValue::Bool(left >= right))
            }
            _ => Ok(RuntimeValue::Bool(
                expect_int(left, ">=")? >= expect_int(right, ">=")?,
            )),
        },
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
        ParsedBinaryOp::Sub => match (&left, &right) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (kind, left, right) = expect_same_float_operands(left, right, "-")?;
                Ok(make_runtime_float(kind, left - right))
            }
            _ => Ok(RuntimeValue::Int(
                expect_int(left, "-")? - expect_int(right, "-")?,
            )),
        },
        ParsedBinaryOp::Mul => match (&left, &right) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (kind, left, right) = expect_same_float_operands(left, right, "*")?;
                Ok(make_runtime_float(kind, left * right))
            }
            _ => Ok(RuntimeValue::Int(
                expect_int(left, "*")? * expect_int(right, "*")?,
            )),
        },
        ParsedBinaryOp::Div => match (&left, &right) {
            (RuntimeValue::Float { .. }, RuntimeValue::Float { .. }) => {
                let (kind, left, right) = expect_same_float_operands(left, right, "/")?;
                Ok(make_runtime_float(kind, left / right))
            }
            _ => Ok(RuntimeValue::Int(
                expect_int(left, "/")? / expect_int(right, "/")?,
            )),
        },
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
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
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
                current_package_id,
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

fn call_uses_linear_mutation_modes(
    callable: &[String],
    current_package_id: &str,
    current_module_id: &str,
    call_args: &[RuntimeCallArg],
    plan: &RuntimePackagePlan,
) -> RuntimeEvalResult<bool> {
    if let Some(routine_index) = resolve_routine_index_for_call(
        plan,
        current_package_id,
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
        return Ok(routine
            .params
            .iter()
            .any(|param| matches!(param.mode.as_deref(), Some("edit") | Some("hold"))));
    }
    let intrinsic = resolve_runtime_intrinsic_path(callable)
        .ok_or_else(|| format!("unsupported runtime callable `{}`", callable.join(".")))?;
    Ok(!edit_arg_indices(intrinsic).is_empty())
}

fn validate_spawned_call_capabilities(
    op: ParsedUnaryOp,
    callable: &[String],
    current_package_id: &str,
    current_module_id: &str,
    call_args: &[RuntimeCallArg],
    plan: &RuntimePackagePlan,
    context: &str,
) -> RuntimeEvalResult<()> {
    if matches!(op, ParsedUnaryOp::Split)
        && call_uses_linear_mutation_modes(
            callable,
            current_package_id,
            current_module_id,
            call_args,
            plan,
        )?
    {
        return Err(format!(
            "{context} `{}` does not yet support `edit`/`hold` parameters or intrinsic arguments across split/thread boundaries",
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
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let (callable, type_args, call_args) = build_runtime_call_args_from_chain_stage(
        stage,
        input,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    execute_call_by_path(
        &callable,
        None,
        None,
        current_package_id,
        current_module_id,
        type_args,
        call_args,
        false,
        plan,
        scopes,
        state,
        host,
        allow_async,
    )
}

fn spawn_runtime_chain_stage(
    op: ParsedUnaryOp,
    stage: &ParsedChainStep,
    input: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let (callable, type_args, mut call_args) = build_runtime_call_args_from_chain_stage(
        stage,
        Some(input),
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    validate_spawned_call_capabilities(
        op,
        &callable,
        current_package_id,
        current_module_id,
        &call_args,
        plan,
        "chain stage",
    )?;
    if let Some(routine_index) = resolve_routine_index_for_call(
        plan,
        current_package_id,
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
        call_args = bind_call_args_for_intrinsic(&callable, call_args)?;
        consume_take_call_args(scopes, take_arg_indices(intrinsic), &call_args)?;
    }
    let thread_id = match op {
        ParsedUnaryOp::Weave => state.current_thread_id,
        ParsedUnaryOp::Split => allocate_scheduler_thread_id(state),
        _ => unreachable!(),
    };
    if matches!(op, ParsedUnaryOp::Split) {
        runtime_validate_split_scope_capture(scopes, &call_args, state, "split capture")?;
    }
    if matches!(op, ParsedUnaryOp::Split) {
        detach_moved_split_call_args(scopes, &mut call_args);
    }
    let pending = RuntimePendingState::Pending(RuntimeDeferredWork::Call(RuntimeDeferredCall {
        callable,
        resolved_routine: None,
        dynamic_dispatch: None,
        current_package_id: current_package_id.to_string(),
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

fn auto_await_runtime_value(
    value: RuntimeValue,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    match value {
        RuntimeValue::Opaque(RuntimeOpaqueValue::Task(_))
        | RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(_)) => {
            Ok(await_runtime_value(value, plan, state, host)?)
        }
        other => Ok(other),
    }
}

fn build_runtime_lazy_chain_value(
    introducer: ParsedChainIntroducer,
    steps: &[ParsedChainStep],
    current_package_id: &str,
    current_module_id: &str,
    scopes: &[RuntimeScope],
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
) -> RuntimeValue {
    let pending = RuntimePendingState::Pending(RuntimeDeferredWork::Expr(RuntimeDeferredExpr {
        expr: ParsedExpr::Chain {
            style: "forward".to_string(),
            introducer,
            steps: steps.to_vec(),
        },
        current_package_id: current_package_id.to_string(),
        current_module_id: current_module_id.to_string(),
        aliases: aliases.clone(),
        type_bindings: type_bindings.clone(),
        scopes: scopes.to_vec(),
        thread_id: state.current_thread_id,
        allow_async: true,
    }));
    RuntimeValue::Opaque(RuntimeOpaqueValue::Lazy(insert_runtime_lazy(
        state,
        &[],
        pending,
    )))
}

fn choose_parallel_chain_stage_op(
    stage: &ParsedChainStep,
    seed: &RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> ParsedUnaryOp {
    let Ok((callable, _, call_args)) = build_runtime_call_args_from_chain_stage(
        stage,
        Some(seed.clone()),
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    ) else {
        return ParsedUnaryOp::Split;
    };
    let is_async_stage = resolve_routine_index_for_call(
        plan,
        current_package_id,
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
    .and_then(|routine_index| plan.routines.get(routine_index))
    .map(|routine| routine.is_async)
    .unwrap_or(false);
    if is_async_stage
        || call_uses_linear_mutation_modes(
            &callable,
            current_package_id,
            current_module_id,
            &call_args,
            plan,
        )
        .unwrap_or(false)
    {
        ParsedUnaryOp::Weave
    } else {
        ParsedUnaryOp::Split
    }
}

fn eval_forward_runtime_chain(
    ordered: &[&ParsedChainStep],
    seed: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let mut current = seed;
    for stage in ordered.iter().skip(1) {
        current = execute_runtime_chain_stage(
            stage,
            Some(current),
            true,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
    }
    Ok(current)
}

fn eval_async_runtime_chain(
    ordered: &[&ParsedChainStep],
    seed: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let mut current = auto_await_runtime_value(seed, plan, state, host)?;
    for stage in ordered.iter().skip(1) {
        current = execute_runtime_chain_stage(
            stage,
            Some(current),
            true,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        current = auto_await_runtime_value(current, plan, state, host)?;
    }
    Ok(current)
}

fn eval_broadcast_runtime_chain(
    ordered: &[&ParsedChainStep],
    seed: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let mut values = Vec::new();
    for stage in ordered.iter().skip(1) {
        values.push(execute_runtime_chain_stage(
            stage,
            Some(seed.clone()),
            true,
            plan,
            current_package_id,
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

fn eval_collect_runtime_chain(
    ordered: &[&ParsedChainStep],
    seed: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let mut current = seed;
    let mut values = Vec::new();
    for stage in ordered.iter().skip(1) {
        current = execute_runtime_chain_stage(
            stage,
            Some(current),
            true,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        values.push(current.clone());
    }
    Ok(RuntimeValue::List(values))
}

fn eval_parallel_runtime_chain(
    ordered: &[&ParsedChainStep],
    seed: RuntimeValue,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let mut spawned = Vec::new();
    for stage in ordered.iter().skip(1) {
        let op = choose_parallel_chain_stage_op(
            stage,
            &seed,
            plan,
            current_package_id,
            current_module_id,
            &mut scopes.clone(),
            aliases,
            type_bindings,
            state,
            host,
        );
        spawned.push(spawn_runtime_chain_stage(
            op,
            stage,
            seed.clone(),
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?);
    }
    let mut values = Vec::new();
    for value in spawned {
        values.push(await_runtime_value(value, plan, state, host)?);
    }
    Ok(RuntimeValue::List(values))
}

fn eval_runtime_chain_expr(
    style: &str,
    introducer: ParsedChainIntroducer,
    steps: &[ParsedChainStep],
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    if steps.is_empty() {
        return Err("runtime chain expression must contain at least one stage"
            .to_string()
            .into());
    }
    if style == "lazy" {
        return Ok(build_runtime_lazy_chain_value(
            introducer,
            steps,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
        ));
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
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    if style == "plan" {
        return Ok(seed);
    }
    if ordered.len() == 1 {
        return Ok(seed);
    }
    match style {
        "forward" => eval_forward_runtime_chain(
            &ordered,
            seed,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        "async" => eval_async_runtime_chain(
            &ordered,
            seed,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        "broadcast" => eval_broadcast_runtime_chain(
            &ordered,
            seed,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        "collect" => eval_collect_runtime_chain(
            &ordered,
            seed,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
        "parallel" => eval_parallel_runtime_chain(
            &ordered,
            seed,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        ),
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
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    match pending {
        RuntimeDeferredWork::Call(pending) => {
            let RuntimeDeferredCall {
                callable,
                resolved_routine,
                dynamic_dispatch,
                current_package_id,
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
                &current_package_id,
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
            result.map_err(runtime_eval_message)
        }
        RuntimeDeferredWork::Expr(pending) => {
            let RuntimeDeferredExpr {
                expr,
                current_package_id,
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
                &current_package_id,
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
    host: &mut dyn RuntimeCoreHost,
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
    host: &mut dyn RuntimeCoreHost,
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

fn drive_runtime_lazy(
    handle: RuntimeLazyHandle,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<(), String> {
    let pending = {
        let lazy = state
            .lazy_values
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid Lazy handle `{}`", handle.0))?;
        match std::mem::replace(&mut lazy.state, RuntimePendingState::Running) {
            RuntimePendingState::Pending(pending) => pending,
            RuntimePendingState::Completed(value) => {
                lazy.state = RuntimePendingState::Completed(value);
                return Ok(());
            }
            RuntimePendingState::Failed(message) => {
                lazy.state = RuntimePendingState::Failed(message);
                return Ok(());
            }
            RuntimePendingState::Running => {
                return Err(format!("Lazy `{}` is already running", handle.0));
            }
        }
    };
    let next_state = match execute_deferred_work(pending, plan, state, host) {
        Ok(value) => RuntimePendingState::Completed(value),
        Err(message) => RuntimePendingState::Failed(message),
    };
    state
        .lazy_values
        .get_mut(&handle)
        .ok_or_else(|| format!("invalid Lazy handle `{}`", handle.0))?
        .state = next_state;
    Ok(())
}

fn force_runtime_value(
    value: RuntimeValue,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> Result<RuntimeValue, String> {
    match value {
        RuntimeValue::Opaque(RuntimeOpaqueValue::Lazy(handle)) => {
            drive_runtime_lazy(handle, plan, state, host)?;
            let lazy = state
                .lazy_values
                .get(&handle)
                .ok_or_else(|| format!("invalid Lazy handle `{}`", handle.0))?;
            pending_state_value(&lazy.state, &format!("Lazy `{}`", handle.0))
        }
        other => Ok(other),
    }
}

fn capture_spawned_phrase_call(
    op: ParsedUnaryOp,
    subject: &ParsedExpr,
    args: &[ParsedPhraseArg],
    attached: &[ParsedHeaderAttachment],
    qualifier_kind: ParsedPhraseQualifierKind,
    qualifier: &str,
    qualifier_type_args: &[String],
    resolved_callable: Option<&[String]>,
    resolved_routine: Option<&str>,
    dynamic_dispatch: Option<&ParsedDynamicDispatch>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
    let (callable, type_args, mut call_args, call_routine, call_dynamic_dispatch) =
        match qualifier_kind {
            ParsedPhraseQualifierKind::Call
            | ParsedPhraseQualifierKind::Apply
            | ParsedPhraseQualifierKind::Weave
            | ParsedPhraseQualifierKind::Split => {
                if !matches!(
                    qualifier_kind,
                    ParsedPhraseQualifierKind::Apply
                        | ParsedPhraseQualifierKind::Weave
                        | ParsedPhraseQualifierKind::Split
                ) && qualifier != "call"
                {
                    return Ok(None);
                }
                let callable = resolved_callable
                    .map(|path| path.to_vec())
                    .or_else(|| resolve_callable_path(subject, aliases))
                    .ok_or_else(|| format!("unsupported runtime callable `{subject:?}`"))?;
                let type_args = if matches!(qualifier_kind, ParsedPhraseQualifierKind::Apply)
                    || qualifier_type_args.is_empty()
                {
                    resolve_runtime_type_args(&extract_generic_type_args(subject), type_bindings)
                } else {
                    qualifier_type_args.to_vec()
                };
                let call_args = collect_call_args(
                    args,
                    attached,
                    plan,
                    current_package_id,
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
                    current_package_id,
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
                let type_args = if qualifier_type_args.is_empty() {
                    resolve_runtime_type_args(
                        &extract_generic_type_args(&callable_expr),
                        type_bindings,
                    )
                } else {
                    qualifier_type_args.to_vec()
                };
                let mut call_args = vec![RuntimeCallArg {
                    name: None,
                    value: receiver,
                    source_expr: subject.clone(),
                }];
                call_args.extend(collect_call_args(
                    args,
                    attached,
                    plan,
                    current_package_id,
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
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let callable = resolved_callable
                    .map(|callable| callable.to_vec())
                    .unwrap_or_else(|| vec![qualifier.to_string()]);
                let type_args = if qualifier_type_args.is_empty() {
                    runtime_receiver_type_args(&receiver, state)
                } else {
                    qualifier_type_args.to_vec()
                };
                let mut call_args = vec![RuntimeCallArg {
                    name: None,
                    value: receiver,
                    source_expr: subject.clone(),
                }];
                call_args.extend(collect_call_args(
                    args,
                    attached,
                    plan,
                    current_package_id,
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
            ParsedPhraseQualifierKind::Try
            | ParsedPhraseQualifierKind::AwaitApply
            | ParsedPhraseQualifierKind::Await
            | ParsedPhraseQualifierKind::Must
            | ParsedPhraseQualifierKind::Fallback => return Ok(None),
        };

    if let Some(routine_index) = resolve_routine_index_for_call(
        plan,
        current_package_id,
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
        validate_spawned_call_capabilities(
            op,
            &callable,
            current_package_id,
            current_module_id,
            &call_args,
            plan,
            "spawned runtime call",
        )?;
        consume_take_bound_args(scopes, routine, &bound_args)?;
    } else {
        let intrinsic = resolve_runtime_intrinsic_path(&callable)
            .ok_or_else(|| format!("unsupported runtime callable `{}`", callable.join(".")))?;
        call_args = bind_call_args_for_intrinsic(&callable, call_args)?;
        validate_spawned_call_capabilities(
            op,
            &callable,
            current_package_id,
            current_module_id,
            &call_args,
            plan,
            "spawned runtime intrinsic call",
        )?;
        consume_take_call_args(scopes, take_arg_indices(intrinsic), &call_args)?;
    }

    let thread_id = match op {
        ParsedUnaryOp::Weave => state.current_thread_id,
        ParsedUnaryOp::Split => allocate_scheduler_thread_id(state),
        _ => unreachable!(),
    };
    if matches!(op, ParsedUnaryOp::Split) {
        runtime_validate_split_scope_capture(scopes, &call_args, state, "split capture")?;
    }
    if matches!(op, ParsedUnaryOp::Split) {
        detach_moved_split_call_args(scopes, &mut call_args);
    }
    let pending = RuntimePendingState::Pending(RuntimeDeferredWork::Call(RuntimeDeferredCall {
        callable,
        resolved_routine: call_routine,
        dynamic_dispatch: call_dynamic_dispatch,
        current_package_id: current_package_id.to_string(),
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
    current_package_id: &str,
    current_module_id: &str,
    scopes: &[RuntimeScope],
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    if matches!(op, ParsedUnaryOp::Split) {
        runtime_validate_split_scope_capture(scopes, &[], state, "split capture")
            .map_err(RuntimeEvalSignal::from)?;
    }
    let thread_id = match op {
        ParsedUnaryOp::Weave => state.current_thread_id,
        ParsedUnaryOp::Split => allocate_scheduler_thread_id(state),
        _ => unreachable!(),
    };
    let pending = RuntimePendingState::Pending(RuntimeDeferredWork::Expr(RuntimeDeferredExpr {
        expr: expr.clone(),
        current_package_id: current_package_id.to_string(),
        current_module_id: current_module_id.to_string(),
        aliases: aliases.clone(),
        type_bindings: type_bindings.clone(),
        scopes: scopes.to_vec(),
        thread_id,
        allow_async: true,
    }));
    Ok(match op {
        ParsedUnaryOp::Weave => RuntimeValue::Opaque(RuntimeOpaqueValue::Task(
            insert_runtime_task(state, &[], pending),
        )),
        ParsedUnaryOp::Split => RuntimeValue::Opaque(RuntimeOpaqueValue::Thread(
            insert_runtime_thread(state, &[], pending),
        )),
        _ => unreachable!(),
    })
}

fn await_runtime_value(
    value: RuntimeValue,
    plan: &RuntimePackagePlan,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
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

fn must_unwrap_runtime_value(value: RuntimeValue) -> RuntimeEvalResult<RuntimeValue> {
    match value {
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Option.Some") => {
            match payload.as_slice() {
                [value] => Ok(value.clone()),
                _ => Err(format!(
                    "runtime must qualifier `must` expected Option.Some with one payload value, got `{name}`"
                )
                .into()),
            }
        }
        RuntimeValue::Variant { ref name, .. } if variant_name_matches(name, "Option.None") => {
            Err("runtime must qualifier `must` encountered Option.None"
                .to_string()
                .into())
        }
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Ok") => {
            match payload.as_slice() {
                [value] => Ok(value.clone()),
                _ => Err(format!(
                    "runtime must qualifier `must` expected Result.Ok with one payload value, got `{name}`"
                )
                .into()),
            }
        }
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Err") => {
            match payload.as_slice() {
                [RuntimeValue::Str(message)] => Err(message.clone().into()),
                [other] => Err(format!(
                    "runtime must qualifier `must` expected Result.Err(Str), got `{other:?}`"
                )
                .into()),
                _ => Err(format!(
                    "runtime must qualifier `must` expected Result.Err with one payload value, got `{name}`"
                )
                .into()),
            }
        }
        other => Err(format!(
            "runtime must qualifier `must` expects Option-shape or Result[T, Str], got `{other:?}`"
        )
        .into()),
    }
}

fn fallback_runtime_value(
    value: RuntimeValue,
    fallback: RuntimeValue,
) -> RuntimeEvalResult<RuntimeValue> {
    match value {
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Option.Some") => {
            match payload.as_slice() {
                [value] => Ok(value.clone()),
                _ => Err(format!(
                    "runtime fallback qualifier `fallback` expected Option.Some with one payload value, got `{name}`"
                )
                .into()),
            }
        }
        RuntimeValue::Variant { ref name, .. } if variant_name_matches(name, "Option.None") => {
            Ok(fallback)
        }
        RuntimeValue::Variant { name, payload } if variant_name_matches(&name, "Result.Ok") => {
            match payload.as_slice() {
                [value] => Ok(value.clone()),
                _ => Err(format!(
                    "runtime fallback qualifier `fallback` expected Result.Ok with one payload value, got `{name}`"
                )
                .into()),
            }
        }
        RuntimeValue::Variant { ref name, .. } if variant_name_matches(name, "Result.Err") => {
            Ok(fallback)
        }
        other => Err(format!(
            "runtime fallback qualifier `fallback` expects Option-shape or Result[T, Str], got `{other:?}`"
        )
        .into()),
    }
}

fn eval_spawn_expr(
    op: ParsedUnaryOp,
    expr: &ParsedExpr,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    if let ParsedExpr::Phrase {
        subject,
        args,
        qualifier_kind,
        qualifier,
        qualifier_type_args,
        resolved_callable,
        resolved_routine,
        dynamic_dispatch,
        attached,
    } = expr
        && let Some(spawned) = capture_spawned_phrase_call(
            op,
            subject,
            args,
            attached,
            *qualifier_kind,
            qualifier,
            qualifier_type_args,
            resolved_callable.as_deref(),
            resolved_routine.as_deref(),
            dynamic_dispatch.as_ref(),
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?
    {
        return Ok(spawned);
    }
    spawn_runtime_expr(
        op,
        expr,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
    )
}

fn runtime_expr_path_name(expr: &ParsedExpr) -> Option<String> {
    match expr {
        ParsedExpr::Path(segments) => Some(segments.join(".")),
        ParsedExpr::Generic { expr, .. } => runtime_expr_path_name(expr),
        ParsedExpr::Member { expr, member } => {
            runtime_expr_path_name(expr).map(|base| format!("{base}.{member}"))
        }
        _ => None,
    }
}

fn runtime_gate_outcome(
    value: RuntimeValue,
    context: &str,
) -> Result<Result<Option<RuntimeValue>, RuntimeValue>, String> {
    match value {
        RuntimeValue::Bool(true) => Ok(Ok(None)),
        RuntimeValue::Bool(false) => Ok(Err(RuntimeValue::Bool(false))),
        RuntimeValue::Variant { name, mut payload } => {
            if variant_name_matches(&name, "Option.Some") && payload.len() == 1 {
                Ok(Ok(Some(payload.remove(0))))
            } else if variant_name_matches(&name, "Option.None") && payload.is_empty() {
                Ok(Err(none_variant()))
            } else if variant_name_matches(&name, "Result.Ok") && payload.len() == 1 {
                Ok(Ok(Some(payload.remove(0))))
            } else if variant_name_matches(&name, "Result.Err") && payload.len() == 1 {
                Ok(Err(RuntimeValue::Variant { name, payload }))
            } else {
                Err(format!("{context} expects Bool, Option, or Result"))
            }
        }
        other => Err(format!(
            "{context} expects Bool, Option, or Result, found {other:?}"
        )),
    }
}

fn eval_headed_modifier_payload(
    payload: Option<&ParsedExpr>,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
    payload
        .map(|expr| {
            eval_expr(
                expr,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )
        })
        .transpose()
}

fn memory_family_from_text(text: &str) -> Result<MemoryFamily, String> {
    MemoryFamily::parse(text).ok_or_else(|| format!("unknown memory family `{text}`"))
}

fn memory_detail_key_from_text(text: &str) -> Result<MemoryDetailKey, String> {
    MemoryDetailKey::parse(text).ok_or_else(|| format!("unknown memory detail key `{text}`"))
}

fn memory_detail_atom(value: RuntimeValue, context: &str) -> Result<String, String> {
    match value {
        RuntimeValue::Str(text) => Ok(text),
        RuntimeValue::Variant { name, payload } if payload.is_empty() => {
            Ok(name.rsplit('.').next().unwrap_or(&name).to_string())
        }
        RuntimeValue::Record { name, fields } if fields.is_empty() => {
            Ok(name.rsplit('.').next().unwrap_or(&name).to_string())
        }
        RuntimeValue::OwnerHandle(name) => Ok(name),
        other => Err(format!(
            "{context} requires an identifier atom, found {other:?}"
        )),
    }
}

fn build_runtime_memory_spec_materialization(
    spec: &ParsedMemorySpecDecl,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeMemorySpecMaterialization> {
    let family = memory_family_from_text(&spec.family).map_err(RuntimeEvalSignal::from)?;
    let descriptor = memory_family_descriptor(family);
    let default_strategy = spec
        .default_modifier
        .as_ref()
        .map(runtime_memory_strategy_from_modifier)
        .transpose()
        .map_err(RuntimeEvalSignal::from)?
        .unwrap_or(RuntimeMemoryStrategy::Alloc);
    let mut budget_strategy = default_strategy;
    let mut recycle_strategy = default_strategy;
    let mut handle_strategy = default_strategy;
    let mut capacity = 0usize;
    let mut growth = None;
    let mut pressure = None;
    let mut handle_policy = None;
    let mut frame_recycle = None;
    let mut pool_recycle = None;
    let mut reset_on = None;
    let mut ring_overwrite = None;
    let mut ring_window = None;
    let mut slab_page = None;
    for detail in &spec.details {
        let key = memory_detail_key_from_text(&detail.key).map_err(RuntimeEvalSignal::from)?;
        let descriptor = memory_detail_descriptor(family, key).ok_or_else(|| {
            RuntimeEvalSignal::from(format!(
                "memory detail `{}` is not supported for family `{}`",
                detail.key, spec.family
            ))
        })?;
        let detail_strategy = detail
            .modifier
            .as_ref()
            .map(runtime_memory_strategy_from_modifier)
            .transpose()
            .map_err(RuntimeEvalSignal::from)?;
        match descriptor.value_kind {
            MemoryDetailValueKind::IntExpr => {
                let value = eval_expr(
                    &detail.value,
                    plan,
                    current_package_id,
                    current_module_id,
                    scopes,
                    aliases,
                    type_bindings,
                    state,
                    host,
                )?;
                let int_value = expect_int(value, &format!("Memory.{}", detail.key))
                    .map_err(RuntimeEvalSignal::from)?;
                let normalized =
                    runtime_non_negative_usize(int_value, &format!("Memory.{}", detail.key))
                        .map_err(RuntimeEvalSignal::from)?;
                if matches!(
                    key,
                    MemoryDetailKey::Capacity
                        | MemoryDetailKey::Growth
                        | MemoryDetailKey::Pressure
                        | MemoryDetailKey::Page
                        | MemoryDetailKey::Window
                ) && let Some(strategy) = detail_strategy
                {
                    budget_strategy = strategy;
                }
                match key {
                    MemoryDetailKey::Capacity => capacity = normalized,
                    MemoryDetailKey::Growth => growth = Some(normalized),
                    MemoryDetailKey::Page => slab_page = Some(normalized),
                    MemoryDetailKey::Window => ring_window = Some(normalized),
                    _ => unreachable!("only int memory detail keys reach this branch"),
                }
            }
            MemoryDetailValueKind::Atom => {
                let atom = match &detail.value {
                    ParsedExpr::Path(segments) if !segments.is_empty() => {
                        segments.last().cloned().unwrap_or_default()
                    }
                    _ => {
                        let value = eval_expr(
                            &detail.value,
                            plan,
                            current_package_id,
                            current_module_id,
                            scopes,
                            aliases,
                            type_bindings,
                            state,
                            host,
                        )?;
                        memory_detail_atom(value, &format!("Memory.{}", detail.key))
                            .map_err(RuntimeEvalSignal::from)?
                    }
                };
                if !descriptor.atoms.iter().any(|allowed| *allowed == atom) {
                    return Err(format!(
                        "memory detail `{}` for family `{}` rejects atom `{}`; allowed: {}",
                        detail.key,
                        spec.family,
                        atom,
                        descriptor.atoms.join(", ")
                    )
                    .into());
                }
                match key {
                    MemoryDetailKey::Pressure => {
                        if let Some(strategy) = detail_strategy {
                            budget_strategy = strategy;
                        }
                        pressure = Some(
                            runtime_memory_pressure_from_atom(&atom)
                                .map_err(RuntimeEvalSignal::from)?,
                        );
                    }
                    MemoryDetailKey::Handle => {
                        if let Some(strategy) = detail_strategy {
                            handle_strategy = strategy;
                        }
                        handle_policy = Some(
                            runtime_memory_handle_policy_from_atom(&atom)
                                .map_err(RuntimeEvalSignal::from)?,
                        );
                    }
                    MemoryDetailKey::Recycle => {
                        if let Some(strategy) = detail_strategy {
                            recycle_strategy = strategy;
                        }
                        match family {
                            MemoryFamily::Arena => {
                                return Err(RuntimeEvalSignal::from(
                                    "arena does not support recycle atoms".to_string(),
                                ));
                            }
                            MemoryFamily::Frame => {
                                frame_recycle = Some(
                                    runtime_frame_recycle_policy_from_atom(&atom)
                                        .map_err(RuntimeEvalSignal::from)?,
                                );
                            }
                            MemoryFamily::Pool => {
                                pool_recycle = Some(
                                    runtime_pool_recycle_policy_from_atom(&atom)
                                        .map_err(RuntimeEvalSignal::from)?,
                                );
                            }
                            MemoryFamily::Temp
                            | MemoryFamily::Session
                            | MemoryFamily::Ring
                            | MemoryFamily::Slab => {
                                return Err(RuntimeEvalSignal::from(format!(
                                    "{} does not support recycle atoms",
                                    spec.family
                                )));
                            }
                        }
                    }
                    MemoryDetailKey::ResetOn => {
                        reset_on = Some(
                            runtime_reset_on_policy_from_atom(&atom)
                                .map_err(RuntimeEvalSignal::from)?,
                        );
                    }
                    MemoryDetailKey::Overwrite => {
                        ring_overwrite = Some(
                            runtime_ring_overwrite_policy_from_atom(&atom)
                                .map_err(RuntimeEvalSignal::from)?,
                        );
                    }
                    _ => unreachable!("only atom memory detail keys reach this branch"),
                }
            }
        }
    }
    let (resolved_handle_policy, kind) = match family {
        MemoryFamily::Arena => {
            let policy = RuntimeArenaPolicy {
                base_capacity: capacity,
                current_limit: capacity,
                growth_step: growth
                    .unwrap_or_else(|| runtime_default_growth_step(budget_strategy, capacity)),
                pressure: pressure
                    .unwrap_or_else(|| runtime_default_memory_pressure(budget_strategy)),
                handle: handle_policy
                    .unwrap_or_else(|| runtime_default_memory_handle_policy(handle_strategy)),
            };
            (
                policy.handle,
                RuntimeMemorySpecMaterializationKind::Arena(policy),
            )
        }
        MemoryFamily::Frame => (
            RuntimeMemoryHandlePolicy::Stable,
            RuntimeMemorySpecMaterializationKind::Frame(RuntimeFrameArenaPolicy {
                base_capacity: capacity,
                current_limit: capacity,
                growth_step: growth
                    .unwrap_or_else(|| runtime_default_growth_step(budget_strategy, capacity)),
                pressure: pressure
                    .unwrap_or_else(|| runtime_default_memory_pressure(budget_strategy)),
                recycle: frame_recycle
                    .unwrap_or_else(|| runtime_default_frame_recycle_policy(recycle_strategy)),
                reset_on: reset_on
                    .unwrap_or_else(|| runtime_default_reset_on_policy(recycle_strategy)),
            }),
        ),
        MemoryFamily::Pool => {
            let policy = RuntimePoolArenaPolicy {
                base_capacity: capacity,
                current_limit: capacity,
                growth_step: growth
                    .unwrap_or_else(|| runtime_default_growth_step(budget_strategy, capacity)),
                pressure: pressure
                    .unwrap_or_else(|| runtime_default_memory_pressure(budget_strategy)),
                recycle: pool_recycle
                    .unwrap_or_else(|| runtime_default_pool_recycle_policy(recycle_strategy)),
                handle: handle_policy
                    .unwrap_or_else(|| runtime_default_memory_handle_policy(handle_strategy)),
            };
            (
                policy.handle,
                RuntimeMemorySpecMaterializationKind::Pool(policy),
            )
        }
        MemoryFamily::Temp => (
            RuntimeMemoryHandlePolicy::Stable,
            RuntimeMemorySpecMaterializationKind::Temp(RuntimeTempArenaPolicy {
                base_capacity: capacity,
                current_limit: capacity,
                growth_step: growth
                    .unwrap_or_else(|| runtime_default_growth_step(budget_strategy, capacity)),
                pressure: pressure
                    .unwrap_or_else(|| runtime_default_memory_pressure(budget_strategy)),
                reset_on: reset_on
                    .unwrap_or_else(|| runtime_default_reset_on_policy(recycle_strategy)),
            }),
        ),
        MemoryFamily::Session => {
            let policy = RuntimeSessionArenaPolicy {
                base_capacity: capacity,
                current_limit: capacity,
                growth_step: growth
                    .unwrap_or_else(|| runtime_default_growth_step(budget_strategy, capacity)),
                pressure: pressure
                    .unwrap_or_else(|| runtime_default_memory_pressure(budget_strategy)),
                handle: handle_policy.unwrap_or(RuntimeMemoryHandlePolicy::Stable),
            };
            (
                policy.handle,
                RuntimeMemorySpecMaterializationKind::Session(policy),
            )
        }
        MemoryFamily::Ring => (
            RuntimeMemoryHandlePolicy::Stable,
            RuntimeMemorySpecMaterializationKind::Ring(RuntimeRingBufferPolicy {
                base_capacity: capacity,
                current_limit: capacity,
                growth_step: growth
                    .unwrap_or_else(|| runtime_default_growth_step(budget_strategy, capacity)),
                pressure: pressure
                    .unwrap_or_else(|| runtime_default_memory_pressure(budget_strategy)),
                overwrite: ring_overwrite.unwrap_or(RuntimeRingOverwritePolicy::Oldest),
                window: ring_window.unwrap_or(capacity.max(1)),
            }),
        ),
        MemoryFamily::Slab => {
            let policy = RuntimeSlabPolicy {
                base_capacity: capacity,
                current_limit: capacity,
                growth_step: growth
                    .unwrap_or_else(|| runtime_default_growth_step(budget_strategy, capacity)),
                pressure: pressure
                    .unwrap_or_else(|| runtime_default_memory_pressure(budget_strategy)),
                handle: handle_policy.unwrap_or(RuntimeMemoryHandlePolicy::Stable),
                page: slab_page.unwrap_or(capacity.max(1)),
            };
            (
                policy.handle,
                RuntimeMemorySpecMaterializationKind::Slab(policy),
            )
        }
    };
    Ok(RuntimeMemorySpecMaterialization {
        hook_id: descriptor.lazy_materialization_hook_id,
        handle_policy: resolved_handle_policy,
        kind,
    })
}

struct RuntimeMemoryMaterializationHook {
    id: &'static str,
    materialize: fn(
        &RuntimeMemorySpecMaterializationKind,
        &mut RuntimeExecutionState,
    ) -> RuntimeEvalResult<RuntimeValue>,
}

fn materialize_runtime_arena_spec_hook(
    kind: &RuntimeMemorySpecMaterializationKind,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let RuntimeMemorySpecMaterializationKind::Arena(policy) = kind else {
        return Err(RuntimeEvalSignal::from(
            "runtime memory materialization hook `arena_new` received a non-arena policy"
                .to_string(),
        ));
    };
    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Arena(
        insert_runtime_arena(state, &[], policy.clone()),
    )))
}

fn materialize_runtime_frame_spec_hook(
    kind: &RuntimeMemorySpecMaterializationKind,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let RuntimeMemorySpecMaterializationKind::Frame(policy) = kind else {
        return Err(RuntimeEvalSignal::from(
            "runtime memory materialization hook `frame_new` received a non-frame policy"
                .to_string(),
        ));
    };
    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::FrameArena(
        insert_runtime_frame_arena(state, &[], policy.clone()),
    )))
}

fn materialize_runtime_pool_spec_hook(
    kind: &RuntimeMemorySpecMaterializationKind,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let RuntimeMemorySpecMaterializationKind::Pool(policy) = kind else {
        return Err(RuntimeEvalSignal::from(
            "runtime memory materialization hook `pool_new` received a non-pool policy".to_string(),
        ));
    };
    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::PoolArena(
        insert_runtime_pool_arena(state, &[], policy.clone()),
    )))
}

fn materialize_runtime_temp_spec_hook(
    kind: &RuntimeMemorySpecMaterializationKind,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let RuntimeMemorySpecMaterializationKind::Temp(policy) = kind else {
        return Err(RuntimeEvalSignal::from(
            "runtime memory materialization hook `temp_new` received a non-temp policy".to_string(),
        ));
    };
    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::TempArena(
        insert_runtime_temp_arena(state, &[], policy.clone()),
    )))
}

fn materialize_runtime_session_spec_hook(
    kind: &RuntimeMemorySpecMaterializationKind,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let RuntimeMemorySpecMaterializationKind::Session(policy) = kind else {
        return Err(RuntimeEvalSignal::from(
            "runtime memory materialization hook `session_new` received a non-session policy"
                .to_string(),
        ));
    };
    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::SessionArena(
        insert_runtime_session_arena(state, &[], policy.clone()),
    )))
}

fn materialize_runtime_ring_spec_hook(
    kind: &RuntimeMemorySpecMaterializationKind,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let RuntimeMemorySpecMaterializationKind::Ring(policy) = kind else {
        return Err(RuntimeEvalSignal::from(
            "runtime memory materialization hook `ring_new` received a non-ring policy".to_string(),
        ));
    };
    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::RingBuffer(
        insert_runtime_ring_buffer(state, &[], policy.clone()),
    )))
}

fn materialize_runtime_slab_spec_hook(
    kind: &RuntimeMemorySpecMaterializationKind,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let RuntimeMemorySpecMaterializationKind::Slab(policy) = kind else {
        return Err(RuntimeEvalSignal::from(
            "runtime memory materialization hook `slab_new` received a non-slab policy".to_string(),
        ));
    };
    Ok(RuntimeValue::Opaque(RuntimeOpaqueValue::Slab(
        insert_runtime_slab(state, &[], policy.clone()),
    )))
}

const RUNTIME_MEMORY_MATERIALIZATION_HOOKS: &[RuntimeMemoryMaterializationHook] = &[
    RuntimeMemoryMaterializationHook {
        id: "arena_new",
        materialize: materialize_runtime_arena_spec_hook,
    },
    RuntimeMemoryMaterializationHook {
        id: "frame_new",
        materialize: materialize_runtime_frame_spec_hook,
    },
    RuntimeMemoryMaterializationHook {
        id: "pool_new",
        materialize: materialize_runtime_pool_spec_hook,
    },
    RuntimeMemoryMaterializationHook {
        id: "temp_new",
        materialize: materialize_runtime_temp_spec_hook,
    },
    RuntimeMemoryMaterializationHook {
        id: "session_new",
        materialize: materialize_runtime_session_spec_hook,
    },
    RuntimeMemoryMaterializationHook {
        id: "ring_new",
        materialize: materialize_runtime_ring_spec_hook,
    },
    RuntimeMemoryMaterializationHook {
        id: "slab_new",
        materialize: materialize_runtime_slab_spec_hook,
    },
];

fn materialize_runtime_memory_spec_handle(
    materialization: &RuntimeMemorySpecMaterialization,
    state: &mut RuntimeExecutionState,
) -> RuntimeEvalResult<RuntimeValue> {
    let hook = RUNTIME_MEMORY_MATERIALIZATION_HOOKS
        .iter()
        .find(|hook| hook.id == materialization.hook_id)
        .ok_or_else(|| {
            RuntimeEvalSignal::from(format!(
                "unsupported runtime memory materialization hook `{}`",
                materialization.hook_id
            ))
        })?;
    (hook.materialize)(&materialization.kind, state)
}

fn materialize_runtime_memory_spec(
    spec: &ParsedMemorySpecDecl,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<(RuntimeValue, RuntimeMemoryHandlePolicy)> {
    let materialization = build_runtime_memory_spec_materialization(
        spec,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    let handle_policy = materialization.handle_policy;
    let value = materialize_runtime_memory_spec_handle(&materialization, state)?;
    Ok((value, handle_policy))
}

fn resolve_runtime_memory_phrase_instance(
    family: &str,
    arena: &ParsedExpr,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let Some(path) = runtime_expr_path_name(arena)
        .map(|text| text.split('.').map(ToString::to_string).collect::<Vec<_>>())
    else {
        return eval_expr(
            arena,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        );
    };
    if path.len() == 1 && lookup_local(scopes, &path[0]).is_some() {
        return eval_expr(
            arena,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        );
    }
    if path.len() == 1
        && let Some(spec_state) = lookup_memory_spec_in_scopes(scopes, &path[0]).cloned()
    {
        if spec_state.spec.family != family {
            return Err(format!(
                "memory spec `{}` is family `{}` but phrase requires `{family}`",
                path[0], spec_state.spec.family
            )
            .into());
        }
        if spec_state.handle_policy == Some(RuntimeMemoryHandlePolicy::Stable)
            && let Some(handle) = spec_state.handle
        {
            return Ok(handle);
        }
        let (handle, handle_policy) = materialize_runtime_memory_spec(
            &spec_state.spec,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        if let Some(spec_state) = lookup_memory_spec_in_scopes_mut(scopes, &path[0]) {
            spec_state.handle_policy = Some(handle_policy);
            spec_state.handle =
                matches!(handle_policy, RuntimeMemoryHandlePolicy::Stable).then(|| handle.clone());
        }
        return Ok(handle);
    }
    if let Some((package_id, module_id, spec_name)) =
        resolve_memory_spec_target(plan, current_package_id, current_module_id, aliases, &path)
    {
        let key = memory_spec_state_key(&package_id, &module_id, &spec_name);
        let existing_state = state.module_memory_specs.get(&key).cloned();
        let spec = if let Some(existing) = &existing_state {
            existing.spec.clone()
        } else if let Some(spec) =
            lookup_module_memory_spec_decl(plan, &package_id, &module_id, &spec_name)
                .map_err(RuntimeEvalSignal::from)?
        {
            if spec.family != family {
                return Err(format!(
                    "memory spec `{}` is family `{}` but phrase requires `{family}`",
                    path.join("."),
                    spec.family
                )
                .into());
            }
            state.module_memory_specs.insert(
                key.clone(),
                RuntimeMemorySpecState {
                    spec: spec.clone(),
                    handle: None,
                    handle_policy: None,
                    owner_keys: collect_active_owner_keys_from_scopes(scopes),
                },
            );
            spec
        } else {
            return eval_expr(
                arena,
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            );
        };
        if let Some(existing) = existing_state
            && existing.handle_policy == Some(RuntimeMemoryHandlePolicy::Stable)
            && let Some(handle) = existing.handle
        {
            let active_owner_keys = collect_active_owner_keys_from_scopes(scopes);
            if !active_owner_keys.is_empty() {
                state
                    .module_memory_specs
                    .entry(key.clone())
                    .and_modify(|spec_state| {
                        for owner_key in &active_owner_keys {
                            if !spec_state
                                .owner_keys
                                .iter()
                                .any(|active| active == owner_key)
                            {
                                spec_state.owner_keys.push(owner_key.clone());
                            }
                        }
                    });
            }
            return Ok(handle);
        }
        let (handle, handle_policy) = materialize_runtime_memory_spec(
            &spec,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        state
            .module_memory_specs
            .entry(key.clone())
            .and_modify(|spec_state| {
                spec_state.handle_policy = Some(handle_policy);
                spec_state.handle = matches!(handle_policy, RuntimeMemoryHandlePolicy::Stable)
                    .then(|| handle.clone());
                let active_owner_keys = collect_active_owner_keys_from_scopes(scopes);
                for owner_key in active_owner_keys {
                    if !spec_state
                        .owner_keys
                        .iter()
                        .any(|active| active == &owner_key)
                    {
                        spec_state.owner_keys.push(owner_key);
                    }
                }
            })
            .or_insert(RuntimeMemorySpecState {
                spec,
                handle: matches!(handle_policy, RuntimeMemoryHandlePolicy::Stable)
                    .then(|| handle.clone()),
                handle_policy: Some(handle_policy),
                owner_keys: collect_active_owner_keys_from_scopes(scopes),
            });
        return Ok(handle);
    }
    eval_expr(
        arena,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )
}

fn eval_construct_contribution_value(
    line: &ParsedConstructLine,
    default_modifier: Option<&ParsedHeadedModifier>,
    context: &str,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<Option<RuntimeValue>> {
    let value = eval_expr(
        &line.value,
        plan,
        current_package_id,
        current_module_id,
        scopes,
        aliases,
        type_bindings,
        state,
        host,
    )?;
    if matches!(line.mode, ParsedConstructContributionMode::Direct) {
        return Ok(Some(value));
    }
    let modifier = line.modifier.as_ref().or(default_modifier);
    let Some(modifier) = modifier else {
        return Err(format!("{context} acquisition failure requires an explicit modifier").into());
    };
    match runtime_construct_contribution_outcome(value, line.mode, context)? {
        Ok(payload) => Ok(Some(payload)),
        Err(failure) => match modifier.kind.as_str() {
            "return" => {
                if let Some(payload) = &modifier.payload {
                    let value = eval_expr(
                        payload,
                        plan,
                        current_package_id,
                        current_module_id,
                        scopes,
                        aliases,
                        type_bindings,
                        state,
                        host,
                    )?;
                    Err(RuntimeEvalSignal::Return(value))
                } else if let RuntimeValue::Variant { name, .. } = &failure {
                    if variant_name_matches(name, "Result.Err") {
                        Err(RuntimeEvalSignal::Return(failure))
                    } else {
                        Err(format!("bare `-return` on {context} requires Result failure").into())
                    }
                } else {
                    Err(format!("bare `-return` on {context} requires Result failure").into())
                }
            }
            "default" => Ok(eval_headed_modifier_payload(
                modifier.payload.as_ref(),
                plan,
                current_package_id,
                current_module_id,
                scopes,
                aliases,
                type_bindings,
                state,
                host,
            )?),
            "skip" => Ok(Some(none_variant())),
            other => Err(format!("unsupported {context} modifier `{other}`").into()),
        },
    }
}

fn runtime_construct_contribution_outcome(
    value: RuntimeValue,
    mode: ParsedConstructContributionMode,
    context: &str,
) -> Result<Result<RuntimeValue, RuntimeValue>, String> {
    match mode {
        ParsedConstructContributionMode::Direct => Ok(Ok(value)),
        ParsedConstructContributionMode::OptionPayload => match value {
            RuntimeValue::Variant { name, mut payload }
                if variant_name_matches(&name, "Option.Some") && payload.len() == 1 =>
            {
                Ok(Ok(payload.remove(0)))
            }
            RuntimeValue::Variant { name, payload }
                if variant_name_matches(&name, "Option.None") && payload.is_empty() =>
            {
                Ok(Err(none_variant()))
            }
            other => Err(format!(
                "{context} acquisition expects Option payload, found {other:?}"
            )),
        },
        ParsedConstructContributionMode::ResultPayload => match value {
            RuntimeValue::Variant { name, mut payload }
                if variant_name_matches(&name, "Result.Ok") && payload.len() == 1 =>
            {
                Ok(Ok(payload.remove(0)))
            }
            RuntimeValue::Variant { name, payload }
                if variant_name_matches(&name, "Result.Err") && payload.len() == 1 =>
            {
                Ok(Err(RuntimeValue::Variant { name, payload }))
            }
            other => Err(format!(
                "{context} acquisition expects Result payload, found {other:?}"
            )),
        },
    }
}

fn eval_record_region_value(
    region: &ParsedRecordRegion,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let target_name = runtime_expr_path_name(&region.target)
        .ok_or_else(|| "record target must be a path-like record reference".to_string())?;
    let mut fields = BTreeMap::new();
    if let Some(base_expr) = &region.base {
        let base_value = eval_expr(
            base_expr,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        let RuntimeValue::Record {
            fields: base_fields,
            ..
        } = base_value
        else {
            return Err("record base must evaluate to a record value"
                .to_string()
                .into());
        };
        for field_name in &region.copied_fields {
            let value = base_fields
                .get(field_name)
                .cloned()
                .ok_or_else(|| format!("record base is missing copied field `{field_name}`"))?;
            fields.insert(field_name.clone(), value);
        }
    }
    for line in &region.lines {
        if let Some(value) = eval_construct_contribution_value(
            line,
            region.default_modifier.as_ref(),
            "record",
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )? {
            fields.insert(line.name.clone(), value);
        }
    }
    apply_runtime_struct_bitfield_layout(plan, &target_name, &mut fields)
        .map_err(RuntimeEvalSignal::from)?;
    Ok(RuntimeValue::Record {
        name: target_name,
        fields,
    })
}

fn eval_array_region_value(
    region: &ParsedArrayRegion,
    plan: &RuntimePackagePlan,
    current_package_id: &str,
    current_module_id: &str,
    scopes: &mut Vec<RuntimeScope>,
    aliases: &BTreeMap<String, Vec<String>>,
    type_bindings: &RuntimeTypeBindings,
    state: &mut RuntimeExecutionState,
    host: &mut dyn RuntimeCoreHost,
) -> RuntimeEvalResult<RuntimeValue> {
    let mut values = if let Some(base_expr) = &region.base {
        let base_value = eval_expr(
            base_expr,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        match base_value {
            RuntimeValue::Array(values) => values,
            other => {
                return Err(
                    format!("array base must evaluate to an array value, found {other:?}").into(),
                );
            }
        }
    } else {
        Vec::new()
    };
    for line in &region.lines {
        let value = eval_expr(
            &line.value,
            plan,
            current_package_id,
            current_module_id,
            scopes,
            aliases,
            type_bindings,
            state,
            host,
        )?;
        if values.len() <= line.index {
            values.resize(line.index + 1, RuntimeValue::Unit);
        }
        values[line.index] = value;
    }
    Ok(RuntimeValue::Array(values))
}

#[cfg(test)]
mod test_parse;
#[cfg(test)]
pub(crate) use test_parse::{parse_cleanup_footer_row, parse_stmt};
#[cfg(test)]
mod tests;
