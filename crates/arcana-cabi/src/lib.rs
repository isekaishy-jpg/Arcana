use std::collections::BTreeMap;
use std::ffi::{c_char, c_void};

use serde::{Deserialize, Serialize};

pub const ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL: &str = "arcana_cabi_get_product_api_v1";
pub const ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL: &str = "arcana_cabi_last_error_alloc_v1";
pub const ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL: &str = "arcana_cabi_owned_bytes_free_v1";
pub const ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL: &str = "arcana_cabi_owned_str_free_v1";

pub const ARCANA_CABI_EXPORT_CONTRACT_ID: &str = "arcana.cabi.export.v1";
pub const ARCANA_CABI_CHILD_CONTRACT_ID: &str = "arcana.cabi.child.v1";
pub const ARCANA_CABI_PLUGIN_CONTRACT_ID: &str = "arcana.cabi.plugin.v1";
pub const ARCANA_CABI_BINDING_CONTRACT_ID: &str = "arcana.cabi.binding.v1";
pub const ARCANA_CABI_CONTRACT_VERSION_V1: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArcanaCabiProductRole {
    Export,
    Child,
    Plugin,
    Binding,
}

impl ArcanaCabiProductRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Child => "child",
            Self::Plugin => "plugin",
            Self::Binding => "binding",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "export" => Ok(Self::Export),
            "child" => Ok(Self::Child),
            "plugin" => Ok(Self::Plugin),
            "binding" => Ok(Self::Binding),
            other => Err(format!(
                "`role` must be \"export\", \"child\", \"plugin\", or \"binding\" (found `{other}`)"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArcanaCabiParamSourceMode {
    Read,
    Take,
    Edit,
}

impl ArcanaCabiParamSourceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Take => "take",
            Self::Edit => "edit",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "read" => Ok(Self::Read),
            "take" => Ok(Self::Take),
            "edit" => Ok(Self::Edit),
            other => Err(format!("unsupported native param source mode `{other}`")),
        }
    }

    pub fn from_param_mode_text(mode: Option<&str>) -> Result<Self, String> {
        match mode {
            None | Some("read") => Ok(Self::Read),
            Some("take") => Ok(Self::Take),
            Some("edit") => Ok(Self::Edit),
            Some(other) => Err(format!("unsupported native param source mode `{other}`")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcanaCabiPassMode {
    In,
    InWithWriteBack,
}

impl ArcanaCabiPassMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::InWithWriteBack => "in_with_write_back",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "in" => Ok(Self::In),
            "in_with_write_back" => Ok(Self::InWithWriteBack),
            other => Err(format!("unsupported native pass mode `{other}`")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArcanaCabiType {
    Int,
    Bool,
    Str,
    Bytes,
    Opaque(String),
    Pair(Box<ArcanaCabiType>, Box<ArcanaCabiType>),
    Unit,
}

impl ArcanaCabiType {
    pub fn render(&self) -> String {
        match self {
            Self::Int => "Int".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::Str => "Str".to_string(),
            Self::Bytes => "Array[Int]".to_string(),
            Self::Opaque(name) => name.clone(),
            Self::Unit => "Unit".to_string(),
            Self::Pair(left, right) => format!("Pair[{}, {}]", left.render(), right.render()),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("cabi type cannot be empty".to_string());
        }
        match trimmed {
            "Int" => Ok(Self::Int),
            "Bool" => Ok(Self::Bool),
            "Str" => Ok(Self::Str),
            "Array[Int]" => Ok(Self::Bytes),
            "Unit" => Ok(Self::Unit),
            _ if trimmed.starts_with("Pair[") && trimmed.ends_with(']') => {
                let inner = &trimmed["Pair[".len()..trimmed.len() - 1];
                let (left, right) = split_top_level_pair_args(inner)?;
                Ok(Self::Pair(
                    Box::new(Self::parse(left)?),
                    Box::new(Self::parse(right)?),
                ))
            }
            _ if trimmed.contains(['[', ']', ',']) => {
                Err(format!("unsupported cabi type syntax `{trimmed}`"))
            }
            _ => Ok(Self::Opaque(trimmed.to_string())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArcanaCabiBindingScalarType {
    Int,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    ISize,
    USize,
    F32,
    F64,
}

impl ArcanaCabiBindingScalarType {
    pub fn render(self) -> &'static str {
        match self {
            Self::Int => "Int",
            Self::Bool => "Bool",
            Self::I8 => "I8",
            Self::U8 => "U8",
            Self::I16 => "I16",
            Self::U16 => "U16",
            Self::I32 => "I32",
            Self::U32 => "U32",
            Self::I64 => "I64",
            Self::U64 => "U64",
            Self::ISize => "ISize",
            Self::USize => "USize",
            Self::F32 => "F32",
            Self::F64 => "F64",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "Int" => Some(Self::Int),
            "Bool" => Some(Self::Bool),
            "I8" | "i8" => Some(Self::I8),
            "U8" | "u8" => Some(Self::U8),
            "I16" | "i16" => Some(Self::I16),
            "U16" | "u16" => Some(Self::U16),
            "I32" | "i32" => Some(Self::I32),
            "U32" | "u32" => Some(Self::U32),
            "I64" | "i64" => Some(Self::I64),
            "U64" | "u64" => Some(Self::U64),
            "ISize" | "isize" => Some(Self::ISize),
            "USize" | "usize" => Some(Self::USize),
            "F32" | "f32" => Some(Self::F32),
            "F64" | "f64" => Some(Self::F64),
            _ => None,
        }
    }

    pub fn size_bytes(self) -> usize {
        match self {
            Self::Int | Self::I64 | Self::U64 | Self::F64 => 8,
            Self::ISize | Self::USize => std::mem::size_of::<usize>(),
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I16 | Self::U16 => 2,
            Self::I8 | Self::U8 | Self::Bool => 1,
        }
    }

    pub fn align_bytes(self) -> usize {
        self.size_bytes()
    }

    pub fn is_integer(self) -> bool {
        !matches!(self, Self::F32 | Self::F64)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArcanaCabiBindingType {
    Int,
    Bool,
    Str,
    Bytes,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    ISize,
    USize,
    F32,
    F64,
    Named(String),
    Unit,
}

impl ArcanaCabiBindingType {
    pub fn render(&self) -> String {
        match self {
            Self::Int => "Int".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::Str => "Str".to_string(),
            Self::Bytes => "Array[Int]".to_string(),
            Self::I8 => "I8".to_string(),
            Self::U8 => "U8".to_string(),
            Self::I16 => "I16".to_string(),
            Self::U16 => "U16".to_string(),
            Self::I32 => "I32".to_string(),
            Self::U32 => "U32".to_string(),
            Self::I64 => "I64".to_string(),
            Self::U64 => "U64".to_string(),
            Self::ISize => "ISize".to_string(),
            Self::USize => "USize".to_string(),
            Self::F32 => "F32".to_string(),
            Self::F64 => "F64".to_string(),
            Self::Named(name) => name.clone(),
            Self::Unit => "Unit".to_string(),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err("binding cabi type cannot be empty".to_string());
        }
        Ok(match trimmed {
            "Int" => Self::Int,
            "Bool" => Self::Bool,
            "Str" => Self::Str,
            "Array[Int]" => Self::Bytes,
            "I8" => Self::I8,
            "U8" => Self::U8,
            "I16" => Self::I16,
            "U16" => Self::U16,
            "I32" => Self::I32,
            "U32" => Self::U32,
            "I64" => Self::I64,
            "U64" => Self::U64,
            "ISize" => Self::ISize,
            "USize" => Self::USize,
            "F32" => Self::F32,
            "F64" => Self::F64,
            "Unit" => Self::Unit,
            _ if trimmed.contains(['[', ']', ',']) => {
                return Err(format!("unsupported binding cabi type syntax `{trimmed}`"));
            }
            _ => Self::Named(trimmed.to_string()),
        })
    }

    pub fn scalar(self) -> Option<ArcanaCabiBindingScalarType> {
        match self {
            Self::Int => Some(ArcanaCabiBindingScalarType::Int),
            Self::Bool => Some(ArcanaCabiBindingScalarType::Bool),
            Self::I8 => Some(ArcanaCabiBindingScalarType::I8),
            Self::U8 => Some(ArcanaCabiBindingScalarType::U8),
            Self::I16 => Some(ArcanaCabiBindingScalarType::I16),
            Self::U16 => Some(ArcanaCabiBindingScalarType::U16),
            Self::I32 => Some(ArcanaCabiBindingScalarType::I32),
            Self::U32 => Some(ArcanaCabiBindingScalarType::U32),
            Self::I64 => Some(ArcanaCabiBindingScalarType::I64),
            Self::U64 => Some(ArcanaCabiBindingScalarType::U64),
            Self::ISize => Some(ArcanaCabiBindingScalarType::ISize),
            Self::USize => Some(ArcanaCabiBindingScalarType::USize),
            Self::F32 => Some(ArcanaCabiBindingScalarType::F32),
            Self::F64 => Some(ArcanaCabiBindingScalarType::F64),
            Self::Str | Self::Bytes | Self::Named(_) | Self::Unit => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcanaCabiBindingRawType {
    Void,
    Scalar(ArcanaCabiBindingScalarType),
    Named(String),
    Pointer {
        mutable: bool,
        inner: Box<ArcanaCabiBindingRawType>,
    },
    FunctionPointer {
        abi: String,
        nullable: bool,
        params: Vec<ArcanaCabiBindingRawType>,
        return_type: Box<ArcanaCabiBindingRawType>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiBindingLayoutField {
    pub name: String,
    pub ty: ArcanaCabiBindingRawType,
    pub offset: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_width: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_offset: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiBindingLayoutEnumVariant {
    pub name: String,
    pub value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArcanaCabiBindingLayoutKind {
    Alias {
        target: ArcanaCabiBindingRawType,
    },
    Struct {
        fields: Vec<ArcanaCabiBindingLayoutField>,
    },
    Union {
        fields: Vec<ArcanaCabiBindingLayoutField>,
    },
    Array {
        element_type: ArcanaCabiBindingRawType,
        len: usize,
    },
    Enum {
        repr: ArcanaCabiBindingScalarType,
        variants: Vec<ArcanaCabiBindingLayoutEnumVariant>,
    },
    Flags {
        repr: ArcanaCabiBindingScalarType,
    },
    Callback {
        abi: String,
        params: Vec<ArcanaCabiBindingRawType>,
        return_type: ArcanaCabiBindingRawType,
    },
    Interface {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        iid: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vtable_layout_id: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiBindingLayout {
    pub layout_id: String,
    pub size: usize,
    pub align: usize,
    pub kind: ArcanaCabiBindingLayoutKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiExportParam {
    pub name: String,
    pub source_mode: ArcanaCabiParamSourceMode,
    pub pass_mode: ArcanaCabiPassMode,
    pub input_type: ArcanaCabiType,
    pub write_back_type: Option<ArcanaCabiType>,
}

impl ArcanaCabiExportParam {
    pub fn binding(
        name: impl Into<String>,
        source_mode: ArcanaCabiParamSourceMode,
        input_type: ArcanaCabiType,
    ) -> Self {
        let write_back_type =
            matches!(source_mode, ArcanaCabiParamSourceMode::Edit).then(|| input_type.clone());
        Self {
            name: name.into(),
            source_mode,
            pass_mode: match source_mode {
                ArcanaCabiParamSourceMode::Edit => ArcanaCabiPassMode::InWithWriteBack,
                ArcanaCabiParamSourceMode::Read | ArcanaCabiParamSourceMode::Take => {
                    ArcanaCabiPassMode::In
                }
            },
            input_type,
            write_back_type,
        }
    }

    pub fn requires_write_back(&self) -> bool {
        self.write_back_type.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiExport {
    pub export_name: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub return_type: ArcanaCabiType,
    pub params: Vec<ArcanaCabiExportParam>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiBindingParam {
    pub name: String,
    pub source_mode: ArcanaCabiParamSourceMode,
    pub pass_mode: ArcanaCabiPassMode,
    pub input_type: ArcanaCabiBindingType,
    pub write_back_type: Option<ArcanaCabiBindingType>,
}

impl ArcanaCabiBindingParam {
    pub fn binding(
        name: impl Into<String>,
        source_mode: ArcanaCabiParamSourceMode,
        input_type: ArcanaCabiBindingType,
    ) -> Self {
        let write_back_type =
            matches!(source_mode, ArcanaCabiParamSourceMode::Edit).then(|| input_type.clone());
        Self {
            name: name.into(),
            source_mode,
            pass_mode: match source_mode {
                ArcanaCabiParamSourceMode::Edit => ArcanaCabiPassMode::InWithWriteBack,
                ArcanaCabiParamSourceMode::Read | ArcanaCabiParamSourceMode::Take => {
                    ArcanaCabiPassMode::In
                }
            },
            input_type,
            write_back_type,
        }
    }

    pub fn requires_write_back(&self) -> bool {
        self.write_back_type.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiBindingImport {
    pub name: String,
    pub symbol_name: String,
    pub return_type: ArcanaCabiBindingType,
    pub params: Vec<ArcanaCabiBindingParam>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiBindingCallback {
    pub name: String,
    pub return_type: ArcanaCabiBindingType,
    pub params: Vec<ArcanaCabiBindingParam>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcanaCabiBindingSignatureKind {
    Import,
    Callback,
}

impl ArcanaCabiBindingSignatureKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Import => "binding import",
            Self::Callback => "binding callback",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArcanaCabiBindingSignature {
    pub name: String,
    pub return_type: ArcanaCabiBindingType,
    pub params: Vec<ArcanaCabiBindingParam>,
}

impl ArcanaCabiBindingImport {
    pub fn signature(&self) -> ArcanaCabiBindingSignature {
        ArcanaCabiBindingSignature {
            name: self.name.clone(),
            return_type: self.return_type.clone(),
            params: self.params.clone(),
        }
    }
}

impl ArcanaCabiBindingCallback {
    pub fn signature(&self) -> ArcanaCabiBindingSignature {
        ArcanaCabiBindingSignature {
            name: self.name.clone(),
            return_type: self.return_type.clone(),
            params: self.params.clone(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaBytesView {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaStrView {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaOwnedBytes {
    pub ptr: *mut u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcanaOwnedStr {
    pub ptr: *mut u8,
    pub len: usize,
}

pub fn into_owned_bytes(mut bytes: Vec<u8>) -> ArcanaOwnedBytes {
    let owned = ArcanaOwnedBytes {
        ptr: bytes.as_mut_ptr(),
        len: bytes.len(),
    };
    std::mem::forget(bytes);
    owned
}

pub fn into_owned_str(text: String) -> ArcanaOwnedStr {
    let mut bytes = text.into_bytes();
    let owned = ArcanaOwnedStr {
        ptr: bytes.as_mut_ptr(),
        len: bytes.len(),
    };
    std::mem::forget(bytes);
    owned
}

/// # Safety
///
/// `ptr` and `len` must come from `into_owned_bytes` in the same binary.
pub unsafe fn free_owned_bytes(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// # Safety
///
/// `ptr` and `len` must come from `into_owned_str` in the same binary.
pub unsafe fn free_owned_str(ptr: *mut u8, len: usize) {
    unsafe {
        free_owned_bytes(ptr, len);
    }
}

pub type ArcanaCabiLastErrorAllocFn = unsafe extern "system" fn(out_len: *mut usize) -> *mut u8;
pub type ArcanaCabiOwnedBytesFreeFn = unsafe extern "system" fn(ptr: *mut u8, len: usize);
pub type ArcanaCabiOwnedStrFreeFn = unsafe extern "system" fn(ptr: *mut u8, len: usize);
pub type ArcanaCabiChildRunEntrypointFn = unsafe extern "system" fn(
    instance: *mut c_void,
    package_image_ptr: *const u8,
    package_image_len: usize,
    main_routine_key: *const c_char,
    out_exit_code: *mut i32,
) -> i32;
pub type ArcanaCabiPluginDescribeInstanceFn =
    unsafe extern "system" fn(instance: *mut c_void, out_len: *mut usize) -> *mut u8;
pub type ArcanaCabiPluginUseInstanceFn = unsafe extern "system" fn(
    instance: *mut c_void,
    request_ptr: *const u8,
    request_len: usize,
    out_len: *mut usize,
) -> *mut u8;
pub type ArcanaCabiBindingImportFn = unsafe extern "system" fn(
    instance: *mut c_void,
    args: *const ArcanaCabiBindingValueV1,
    arg_count: usize,
    out_write_backs: *mut ArcanaCabiBindingValueV1,
    out_result: *mut ArcanaCabiBindingValueV1,
) -> i32;
pub type ArcanaCabiBindingCallbackFn = unsafe extern "system" fn(
    user_data: *mut c_void,
    args: *const ArcanaCabiBindingValueV1,
    arg_count: usize,
    out_write_backs: *mut ArcanaCabiBindingValueV1,
    out_result: *mut ArcanaCabiBindingValueV1,
) -> i32;
pub type ArcanaCabiBindingRegisterCallbackFn = unsafe extern "system" fn(
    instance: *mut c_void,
    callback_name: *const c_char,
    callback: ArcanaCabiBindingCallbackFn,
    callback_owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    callback_owned_str_free: ArcanaCabiOwnedStrFreeFn,
    user_data: *mut c_void,
    out_handle: *mut u64,
) -> i32;
pub type ArcanaCabiBindingUnregisterCallbackFn =
    unsafe extern "system" fn(instance: *mut c_void, handle: u64) -> i32;
pub type ArcanaCabiBindingInvokeImportFn = unsafe extern "system" fn(
    import_name: *const c_char,
    instance: *mut c_void,
    args: *const ArcanaCabiBindingValueV1,
    arg_count: usize,
    out_write_backs: *mut ArcanaCabiBindingValueV1,
    out_result: *mut ArcanaCabiBindingValueV1,
) -> i32;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiProductApiV1 {
    pub descriptor_size: usize,
    pub package_name: *const c_char,
    pub product_name: *const c_char,
    pub role: *const c_char,
    pub contract_id: *const c_char,
    pub contract_version: u32,
    pub role_ops: *const c_void,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiProductApiV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiExportParamV1 {
    pub name: *const c_char,
    pub source_mode: *const c_char,
    pub pass_mode: *const c_char,
    pub input_type: *const c_char,
    pub write_back_type: *const c_char,
}
unsafe impl Sync for ArcanaCabiExportParamV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiExportEntryV1 {
    pub export_name: *const c_char,
    pub routine_key: *const c_char,
    pub symbol_name: *const c_char,
    pub return_type: *const c_char,
    pub params: *const ArcanaCabiExportParamV1,
    pub param_count: usize,
}
unsafe impl Sync for ArcanaCabiExportEntryV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiExportOpsV1 {
    pub ops_size: usize,
    pub exports: *const ArcanaCabiExportEntryV1,
    pub export_count: usize,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub owned_str_free: ArcanaCabiOwnedStrFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiExportOpsV1 {}

pub type ArcanaCabiCreateInstanceFn = unsafe extern "system" fn() -> *mut c_void;
pub type ArcanaCabiDestroyInstanceFn = unsafe extern "system" fn(instance: *mut c_void);

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiInstanceOpsV1 {
    pub ops_size: usize,
    pub create_instance: ArcanaCabiCreateInstanceFn,
    pub destroy_instance: ArcanaCabiDestroyInstanceFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiInstanceOpsV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiChildOpsV1 {
    pub base: ArcanaCabiInstanceOpsV1,
    pub run_entrypoint: ArcanaCabiChildRunEntrypointFn,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiChildOpsV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiPluginOpsV1 {
    pub base: ArcanaCabiInstanceOpsV1,
    pub describe_instance: ArcanaCabiPluginDescribeInstanceFn,
    pub use_instance: ArcanaCabiPluginUseInstanceFn,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiPluginOpsV1 {}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcanaCabiBindingValueTag {
    Int = 1,
    Bool = 2,
    Str = 3,
    Bytes = 4,
    Opaque = 5,
    Unit = 6,
    I8 = 7,
    U8 = 8,
    I16 = 9,
    U16 = 10,
    I32 = 11,
    U32 = 12,
    I64 = 13,
    U64 = 14,
    ISize = 15,
    USize = 16,
    F32 = 17,
    F64 = 18,
    Layout = 19,
}

impl TryFrom<u32> for ArcanaCabiBindingValueTag {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            tag if tag == Self::Int as u32 => Ok(Self::Int),
            tag if tag == Self::Bool as u32 => Ok(Self::Bool),
            tag if tag == Self::Str as u32 => Ok(Self::Str),
            tag if tag == Self::Bytes as u32 => Ok(Self::Bytes),
            tag if tag == Self::Opaque as u32 => Ok(Self::Opaque),
            tag if tag == Self::Unit as u32 => Ok(Self::Unit),
            tag if tag == Self::I8 as u32 => Ok(Self::I8),
            tag if tag == Self::U8 as u32 => Ok(Self::U8),
            tag if tag == Self::I16 as u32 => Ok(Self::I16),
            tag if tag == Self::U16 as u32 => Ok(Self::U16),
            tag if tag == Self::I32 as u32 => Ok(Self::I32),
            tag if tag == Self::U32 as u32 => Ok(Self::U32),
            tag if tag == Self::I64 as u32 => Ok(Self::I64),
            tag if tag == Self::U64 as u32 => Ok(Self::U64),
            tag if tag == Self::ISize as u32 => Ok(Self::ISize),
            tag if tag == Self::USize as u32 => Ok(Self::USize),
            tag if tag == Self::F32 as u32 => Ok(Self::F32),
            tag if tag == Self::F64 as u32 => Ok(Self::F64),
            tag if tag == Self::Layout as u32 => Ok(Self::Layout),
            other => Err(format!("unsupported native binding value tag `{other}`")),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ArcanaCabiBindingPayloadV1 {
    pub int_value: i64,
    pub bool_value: u8,
    pub i8_value: i8,
    pub u8_value: u8,
    pub i16_value: i16,
    pub u16_value: u16,
    pub i32_value: i32,
    pub u32_value: u32,
    pub i64_value: i64,
    pub u64_value: u64,
    pub isize_value: isize,
    pub usize_value: usize,
    pub f32_value: f32,
    pub f64_value: f64,
    pub str_value: ArcanaStrView,
    pub bytes_value: ArcanaBytesView,
    pub opaque_value: u64,
    pub owned_str_value: ArcanaOwnedStr,
    pub owned_bytes_value: ArcanaOwnedBytes,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArcanaCabiBindingValueV1 {
    pub tag: u32,
    pub reserved0: u32,
    pub reserved1: u64,
    pub payload: ArcanaCabiBindingPayloadV1,
}

impl Default for ArcanaCabiBindingValueV1 {
    fn default() -> Self {
        Self {
            tag: ArcanaCabiBindingValueTag::Unit as u32,
            reserved0: 0,
            reserved1: 0,
            payload: ArcanaCabiBindingPayloadV1 { int_value: 0 },
        }
    }
}

impl ArcanaCabiBindingValueV1 {
    pub fn tag(&self) -> Result<ArcanaCabiBindingValueTag, String> {
        self.tag.try_into()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiBindingImportEntryV1 {
    pub name: *const c_char,
    pub symbol_name: *const c_char,
    pub return_type: *const c_char,
    pub params: *const ArcanaCabiExportParamV1,
    pub param_count: usize,
}
unsafe impl Sync for ArcanaCabiBindingImportEntryV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiBindingCallbackEntryV1 {
    pub name: *const c_char,
    pub return_type: *const c_char,
    pub params: *const ArcanaCabiExportParamV1,
    pub param_count: usize,
}
unsafe impl Sync for ArcanaCabiBindingCallbackEntryV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiBindingLayoutEntryV1 {
    pub layout_id: *const c_char,
    pub detail_json: *const c_char,
}
unsafe impl Sync for ArcanaCabiBindingLayoutEntryV1 {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ArcanaCabiBindingOpsV1 {
    pub base: ArcanaCabiInstanceOpsV1,
    pub imports: *const ArcanaCabiBindingImportEntryV1,
    pub import_count: usize,
    pub callbacks: *const ArcanaCabiBindingCallbackEntryV1,
    pub callback_count: usize,
    pub layouts: *const ArcanaCabiBindingLayoutEntryV1,
    pub layout_count: usize,
    pub register_callback: ArcanaCabiBindingRegisterCallbackFn,
    pub unregister_callback: ArcanaCabiBindingUnregisterCallbackFn,
    pub last_error_alloc: ArcanaCabiLastErrorAllocFn,
    pub owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    pub owned_str_free: ArcanaCabiOwnedStrFreeFn,
    pub reserved0: *const c_void,
    pub reserved1: *const c_void,
}
unsafe impl Sync for ArcanaCabiBindingOpsV1 {}

pub fn validate_binding_transport_type(ty: &ArcanaCabiBindingType) -> Result<(), String> {
    match ty {
        ArcanaCabiBindingType::Int
        | ArcanaCabiBindingType::Bool
        | ArcanaCabiBindingType::Str
        | ArcanaCabiBindingType::Bytes
        | ArcanaCabiBindingType::I8
        | ArcanaCabiBindingType::U8
        | ArcanaCabiBindingType::I16
        | ArcanaCabiBindingType::U16
        | ArcanaCabiBindingType::I32
        | ArcanaCabiBindingType::U32
        | ArcanaCabiBindingType::I64
        | ArcanaCabiBindingType::U64
        | ArcanaCabiBindingType::ISize
        | ArcanaCabiBindingType::USize
        | ArcanaCabiBindingType::F32
        | ArcanaCabiBindingType::F64
        | ArcanaCabiBindingType::Unit => Ok(()),
        ArcanaCabiBindingType::Named(name) if !name.trim().is_empty() => Ok(()),
        ArcanaCabiBindingType::Named(_) => Err("binding named type id cannot be empty".to_string()),
    }
}

pub fn validate_binding_param(param: &ArcanaCabiBindingParam) -> Result<(), String> {
    if param.name.trim().is_empty() {
        return Err("binding param name cannot be empty".to_string());
    }
    validate_binding_transport_type(&param.input_type)?;
    match param.source_mode {
        ArcanaCabiParamSourceMode::Edit => {
            if param.pass_mode != ArcanaCabiPassMode::InWithWriteBack {
                return Err(format!(
                    "binding param `{}` uses source_mode `edit` but pass_mode is `{}` instead of `in_with_write_back`",
                    param.name,
                    param.pass_mode.as_str()
                ));
            }
            if param.write_back_type.as_ref() != Some(&param.input_type) {
                return Err(format!(
                    "binding param `{}` uses source_mode `edit` but write_back_type does not match input_type `{}`",
                    param.name,
                    param.input_type.render()
                ));
            }
        }
        ArcanaCabiParamSourceMode::Read | ArcanaCabiParamSourceMode::Take => {
            if param.pass_mode != ArcanaCabiPassMode::In {
                return Err(format!(
                    "binding param `{}` uses source_mode `{}` but pass_mode is `{}` instead of `in`",
                    param.name,
                    param.source_mode.as_str(),
                    param.pass_mode.as_str()
                ));
            }
            if param.write_back_type.is_some() {
                return Err(format!(
                    "binding param `{}` uses source_mode `{}` but still declares write_back_type",
                    param.name,
                    param.source_mode.as_str()
                ));
            }
        }
    }
    if let Some(write_back_type) = &param.write_back_type {
        validate_binding_transport_type(write_back_type)?;
    }
    Ok(())
}

pub fn validate_binding_imports(imports: &[ArcanaCabiBindingImport]) -> Result<(), String> {
    validate_binding_named_entries(
        ArcanaCabiBindingSignatureKind::Import,
        imports
            .iter()
            .map(|import| {
                if import.symbol_name.trim().is_empty() {
                    return Err(format!(
                        "binding import `{}` symbol_name cannot be empty",
                        import.name
                    ));
                }
                Ok(import.signature())
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
}

pub fn validate_binding_callbacks(callbacks: &[ArcanaCabiBindingCallback]) -> Result<(), String> {
    validate_binding_named_entries(
        ArcanaCabiBindingSignatureKind::Callback,
        callbacks
            .iter()
            .map(ArcanaCabiBindingCallback::signature)
            .collect(),
    )
}

pub fn compare_binding_signatures(
    kind: ArcanaCabiBindingSignatureKind,
    expected: &[ArcanaCabiBindingSignature],
    actual: &[ArcanaCabiBindingSignature],
) -> Result<(), String> {
    let expected_by_name = binding_signatures_by_name(kind, expected)?;
    let actual_by_name = binding_signatures_by_name(kind, actual)?;
    for name in expected_by_name.keys() {
        if !actual_by_name.contains_key(name) {
            return Err(format!(
                "{} `{}` is missing from the loaded metadata",
                kind.label(),
                name
            ));
        }
    }
    for name in actual_by_name.keys() {
        if !expected_by_name.contains_key(name) {
            return Err(format!(
                "loaded metadata declares unexpected {} `{}`",
                kind.label(),
                name
            ));
        }
    }
    for (name, expected_signature) in expected_by_name {
        let actual_signature = actual_by_name.get(name).ok_or_else(|| {
            format!(
                "{} `{name}` is missing from the loaded metadata",
                kind.label()
            )
        })?;
        if expected_signature.return_type != actual_signature.return_type {
            return Err(format!(
                "{} `{}` return type mismatch: expected `{}`, got `{}`",
                kind.label(),
                name,
                expected_signature.return_type.render(),
                actual_signature.return_type.render()
            ));
        }
        if expected_signature.params.len() != actual_signature.params.len() {
            return Err(format!(
                "{} `{}` param count mismatch: expected {}, got {}",
                kind.label(),
                name,
                expected_signature.params.len(),
                actual_signature.params.len()
            ));
        }
        for (index, (expected_param, actual_param)) in expected_signature
            .params
            .iter()
            .zip(actual_signature.params.iter())
            .enumerate()
        {
            if expected_param != actual_param {
                return Err(format!(
                    "{} `{}` param {} mismatch: expected `{} {}: {} / {} / {}`, got `{} {}: {} / {} / {}`",
                    kind.label(),
                    name,
                    index,
                    expected_param.source_mode.as_str(),
                    expected_param.name,
                    expected_param.input_type.render(),
                    expected_param.pass_mode.as_str(),
                    expected_param
                        .write_back_type
                        .as_ref()
                        .map(ArcanaCabiBindingType::render)
                        .unwrap_or_else(|| "none".to_string()),
                    actual_param.source_mode.as_str(),
                    actual_param.name,
                    actual_param.input_type.render(),
                    actual_param.pass_mode.as_str(),
                    actual_param
                        .write_back_type
                        .as_ref()
                        .map(ArcanaCabiBindingType::render)
                        .unwrap_or_else(|| "none".to_string()),
                ));
            }
        }
    }
    Ok(())
}

pub fn binding_write_back_slots(
    params: &[ArcanaCabiBindingParam],
) -> Vec<ArcanaCabiBindingValueV1> {
    params
        .iter()
        .map(|_| ArcanaCabiBindingValueV1::default())
        .collect()
}

pub fn validate_binding_write_backs(
    params: &[ArcanaCabiBindingParam],
    write_backs: &[ArcanaCabiBindingValueV1],
) -> Result<(), String> {
    if params.len() != write_backs.len() {
        return Err(format!(
            "binding write-back slot count mismatch: expected {}, got {}",
            params.len(),
            write_backs.len()
        ));
    }
    for (index, (param, value)) in params.iter().zip(write_backs.iter()).enumerate() {
        if param.requires_write_back() {
            continue;
        }
        if value.tag()? != ArcanaCabiBindingValueTag::Unit {
            return Err(format!(
                "binding write-back slot {} for param `{}` must be Unit because the param does not declare write-back semantics",
                index, param.name
            ));
        }
    }
    Ok(())
}

pub fn validate_binding_layouts(layouts: &[ArcanaCabiBindingLayout]) -> Result<(), String> {
    let mut ids = BTreeMap::<&str, &ArcanaCabiBindingLayout>::new();
    for layout in layouts {
        if layout.layout_id.trim().is_empty() {
            return Err("binding layout id cannot be empty".to_string());
        }
        if ids.insert(&layout.layout_id, layout).is_some() {
            return Err(format!(
                "binding layout table declares duplicate layout id `{}`",
                layout.layout_id
            ));
        }
        if layout.align == 0 {
            return Err(format!(
                "binding layout `{}` must have non-zero alignment",
                layout.layout_id
            ));
        }
        if !layout.align.is_power_of_two() {
            return Err(format!(
                "binding layout `{}` alignment {} must be a power of two",
                layout.layout_id, layout.align
            ));
        }
    }
    for layout in layouts {
        validate_binding_layout_kind(layout, &ids)?;
    }
    Ok(())
}

fn validate_binding_layout_kind(
    layout: &ArcanaCabiBindingLayout,
    ids: &BTreeMap<&str, &ArcanaCabiBindingLayout>,
) -> Result<(), String> {
    match &layout.kind {
        ArcanaCabiBindingLayoutKind::Alias { target } => {
            validate_binding_raw_type(target, ids, &layout.layout_id)?;
        }
        ArcanaCabiBindingLayoutKind::Struct { fields } => {
            validate_binding_layout_fields(layout, ids, fields, false)?;
        }
        ArcanaCabiBindingLayoutKind::Union { fields } => {
            validate_binding_layout_fields(layout, ids, fields, true)?;
        }
        ArcanaCabiBindingLayoutKind::Array { element_type, len } => {
            if *len == 0 {
                return Err(format!(
                    "binding array layout `{}` must declare a non-zero length",
                    layout.layout_id
                ));
            }
            validate_binding_raw_type(element_type, ids, &layout.layout_id)?;
        }
        ArcanaCabiBindingLayoutKind::Enum { repr, variants } => {
            if !repr.is_integer() {
                return Err(format!(
                    "binding enum layout `{}` repr `{}` must be an integer scalar",
                    layout.layout_id,
                    repr.render()
                ));
            }
            let mut names = BTreeMap::<&str, usize>::new();
            let mut values = BTreeMap::<i64, usize>::new();
            for (index, variant) in variants.iter().enumerate() {
                if variant.name.trim().is_empty() {
                    return Err(format!(
                        "binding enum layout `{}` has an empty variant name at index {}",
                        layout.layout_id, index
                    ));
                }
                if let Some(previous) = names.insert(variant.name.as_str(), index) {
                    return Err(format!(
                        "binding enum layout `{}` declares duplicate variant `{}` at indices {} and {}",
                        layout.layout_id, variant.name, previous, index
                    ));
                }
                if let Some(previous) = values.insert(variant.value, index) {
                    return Err(format!(
                        "binding enum layout `{}` declares duplicate repr value `{}` at indices {} and {}",
                        layout.layout_id, variant.value, previous, index
                    ));
                }
            }
        }
        ArcanaCabiBindingLayoutKind::Flags { repr } => {
            if !repr.is_integer() {
                return Err(format!(
                    "binding flags layout `{}` repr `{}` must be an integer scalar",
                    layout.layout_id,
                    repr.render()
                ));
            }
        }
        ArcanaCabiBindingLayoutKind::Callback {
            abi,
            params,
            return_type,
        } => {
            if abi.trim().is_empty() {
                return Err(format!(
                    "binding callback layout `{}` must declare a non-empty ABI string",
                    layout.layout_id
                ));
            }
            let pointer_size = std::mem::size_of::<usize>();
            if layout.size != pointer_size || layout.align != pointer_size {
                return Err(format!(
                    "binding callback layout `{}` must use pointer-sized storage (size {}, align {})",
                    layout.layout_id, pointer_size, pointer_size
                ));
            }
            for param in params {
                validate_binding_raw_type(param, ids, &layout.layout_id)?;
            }
            validate_binding_raw_type(return_type, ids, &layout.layout_id)?;
        }
        ArcanaCabiBindingLayoutKind::Interface {
            vtable_layout_id, ..
        } => {
            let pointer_size = std::mem::size_of::<usize>();
            if layout.size != pointer_size || layout.align != pointer_size {
                return Err(format!(
                    "binding interface layout `{}` must use pointer-sized storage (size {}, align {})",
                    layout.layout_id, pointer_size, pointer_size
                ));
            }
            if let Some(vtable_layout_id) = vtable_layout_id
                && !ids.contains_key(vtable_layout_id.as_str())
            {
                return Err(format!(
                    "binding interface layout `{}` references missing vtable layout `{}`",
                    layout.layout_id, vtable_layout_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_binding_layout_fields(
    layout: &ArcanaCabiBindingLayout,
    ids: &BTreeMap<&str, &ArcanaCabiBindingLayout>,
    fields: &[ArcanaCabiBindingLayoutField],
    union_layout: bool,
) -> Result<(), String> {
    let mut names = BTreeMap::<&str, usize>::new();
    for (index, field) in fields.iter().enumerate() {
        if field.name.trim().is_empty() {
            return Err(format!(
                "binding layout `{}` has an empty field name at index {}",
                layout.layout_id, index
            ));
        }
        if let Some(previous) = names.insert(field.name.as_str(), index) {
            return Err(format!(
                "binding layout `{}` declares duplicate field `{}` at indices {} and {}",
                layout.layout_id, field.name, previous, index
            ));
        }
        validate_binding_raw_type(
            &field.ty,
            ids,
            &format!("{}::{}", layout.layout_id, field.name),
        )?;
        let field_size = binding_raw_type_size(&field.ty, ids)?;
        if union_layout && field.offset != 0 {
            return Err(format!(
                "binding union layout `{}` field `{}` must have offset 0",
                layout.layout_id, field.name
            ));
        }
        if field.offset + field_size > layout.size {
            return Err(format!(
                "binding layout `{}` field `{}` exceeds layout size {}",
                layout.layout_id, field.name, layout.size
            ));
        }
        match (field.bit_width, field.bit_offset) {
            (Some(bit_width), Some(bit_offset)) => {
                let ArcanaCabiBindingRawType::Scalar(scalar) = &field.ty else {
                    return Err(format!(
                        "binding layout `{}` bitfield `{}` must use a scalar base type",
                        layout.layout_id, field.name
                    ));
                };
                if !scalar.is_integer() || matches!(scalar, ArcanaCabiBindingScalarType::Bool) {
                    return Err(format!(
                        "binding layout `{}` bitfield `{}` must use a fixed-width integer base type",
                        layout.layout_id, field.name
                    ));
                }
                if bit_width == 0 {
                    return Err(format!(
                        "binding layout `{}` bitfield `{}` must not use zero width",
                        layout.layout_id, field.name
                    ));
                }
                let storage_bits = scalar.size_bytes() * 8;
                let bit_width = usize::from(bit_width);
                let bit_offset = usize::from(bit_offset);
                if bit_width > storage_bits || bit_offset + bit_width > storage_bits {
                    return Err(format!(
                        "binding layout `{}` bitfield `{}` exceeds its storage unit",
                        layout.layout_id, field.name
                    ));
                }
            }
            (None, None) => {}
            _ => {
                return Err(format!(
                    "binding layout `{}` field `{}` must set both bit_width and bit_offset together",
                    layout.layout_id, field.name
                ));
            }
        }
    }
    Ok(())
}

fn validate_binding_raw_type(
    ty: &ArcanaCabiBindingRawType,
    ids: &BTreeMap<&str, &ArcanaCabiBindingLayout>,
    context: &str,
) -> Result<(), String> {
    match ty {
        ArcanaCabiBindingRawType::Void => Ok(()),
        ArcanaCabiBindingRawType::Scalar(_) => Ok(()),
        ArcanaCabiBindingRawType::Named(layout_id) => {
            if layout_id.trim().is_empty() {
                return Err(format!("{context} references an empty binding layout id"));
            }
            if !ids.contains_key(layout_id.as_str()) {
                return Err(format!(
                    "{context} references missing binding layout `{layout_id}`"
                ));
            }
            Ok(())
        }
        ArcanaCabiBindingRawType::Pointer { inner, .. } => {
            validate_binding_raw_type(inner, ids, context)
        }
        ArcanaCabiBindingRawType::FunctionPointer {
            abi,
            params,
            return_type,
            ..
        } => {
            if abi.trim().is_empty() {
                return Err(format!(
                    "{context} declares a function pointer with an empty ABI"
                ));
            }
            for param in params {
                validate_binding_raw_type(param, ids, context)?;
            }
            validate_binding_raw_type(return_type, ids, context)
        }
    }
}

fn binding_raw_type_size(
    ty: &ArcanaCabiBindingRawType,
    ids: &BTreeMap<&str, &ArcanaCabiBindingLayout>,
) -> Result<usize, String> {
    Ok(match ty {
        ArcanaCabiBindingRawType::Void => 0,
        ArcanaCabiBindingRawType::Scalar(scalar) => scalar.size_bytes(),
        ArcanaCabiBindingRawType::Named(layout_id) => {
            ids.get(layout_id.as_str())
                .ok_or_else(|| format!("missing binding layout `{layout_id}`"))?
                .size
        }
        ArcanaCabiBindingRawType::Pointer { .. }
        | ArcanaCabiBindingRawType::FunctionPointer { .. } => std::mem::size_of::<usize>(),
    })
}

pub fn compare_binding_layouts(
    expected: &[ArcanaCabiBindingLayout],
    actual: &[ArcanaCabiBindingLayout],
) -> Result<(), String> {
    validate_binding_layouts(expected)?;
    validate_binding_layouts(actual)?;
    let expected_by_id = expected
        .iter()
        .map(|layout| (layout.layout_id.as_str(), layout))
        .collect::<BTreeMap<_, _>>();
    let actual_by_id = actual
        .iter()
        .map(|layout| (layout.layout_id.as_str(), layout))
        .collect::<BTreeMap<_, _>>();
    for layout_id in expected_by_id.keys() {
        if !actual_by_id.contains_key(layout_id) {
            return Err(format!(
                "binding layout `{layout_id}` is missing from the loaded metadata"
            ));
        }
    }
    for layout_id in actual_by_id.keys() {
        if !expected_by_id.contains_key(layout_id) {
            return Err(format!(
                "loaded metadata declares unexpected binding layout `{layout_id}`"
            ));
        }
    }
    for (layout_id, expected_layout) in expected_by_id {
        let actual_layout = actual_by_id.get(layout_id).ok_or_else(|| {
            format!("binding layout `{layout_id}` is missing from the loaded metadata")
        })?;
        if expected_layout != *actual_layout {
            return Err(format!(
                "binding layout `{layout_id}` does not match the expected typed raw metadata"
            ));
        }
    }
    Ok(())
}

pub fn clone_owned_binding_bytes(
    owned: ArcanaOwnedBytes,
    free: ArcanaCabiOwnedBytesFreeFn,
) -> Result<Vec<u8>, String> {
    if owned.ptr.is_null() {
        if owned.len == 0 {
            return Ok(Vec::new());
        }
        return Err(format!(
            "native binding returned null owned bytes with non-zero length {}",
            owned.len
        ));
    }
    let bytes = unsafe { std::slice::from_raw_parts(owned.ptr, owned.len) }.to_vec();
    unsafe {
        free(owned.ptr, owned.len);
    }
    Ok(bytes)
}

pub fn clone_owned_binding_str(
    owned: ArcanaOwnedStr,
    free: ArcanaCabiOwnedStrFreeFn,
) -> Result<String, String> {
    let bytes = clone_owned_binding_bytes(
        ArcanaOwnedBytes {
            ptr: owned.ptr,
            len: owned.len,
        },
        free as ArcanaCabiOwnedBytesFreeFn,
    )?;
    String::from_utf8(bytes).map_err(|err| format!("native binding string is not utf-8: {err}"))
}

pub fn release_binding_output_value(
    value: ArcanaCabiBindingValueV1,
    owned_bytes_free: ArcanaCabiOwnedBytesFreeFn,
    owned_str_free: ArcanaCabiOwnedStrFreeFn,
) -> Result<(), String> {
    match value.tag()? {
        ArcanaCabiBindingValueTag::Bytes => {
            let owned = unsafe { value.payload.owned_bytes_value };
            unsafe {
                owned_bytes_free(owned.ptr, owned.len);
            }
            Ok(())
        }
        ArcanaCabiBindingValueTag::Layout => {
            let owned = unsafe { value.payload.owned_bytes_value };
            unsafe {
                owned_bytes_free(owned.ptr, owned.len);
            }
            Ok(())
        }
        ArcanaCabiBindingValueTag::Str => {
            let owned = unsafe { value.payload.owned_str_value };
            unsafe {
                owned_str_free(owned.ptr, owned.len);
            }
            Ok(())
        }
        ArcanaCabiBindingValueTag::Int
        | ArcanaCabiBindingValueTag::Bool
        | ArcanaCabiBindingValueTag::I8
        | ArcanaCabiBindingValueTag::U8
        | ArcanaCabiBindingValueTag::I16
        | ArcanaCabiBindingValueTag::U16
        | ArcanaCabiBindingValueTag::I32
        | ArcanaCabiBindingValueTag::U32
        | ArcanaCabiBindingValueTag::I64
        | ArcanaCabiBindingValueTag::U64
        | ArcanaCabiBindingValueTag::ISize
        | ArcanaCabiBindingValueTag::USize
        | ArcanaCabiBindingValueTag::F32
        | ArcanaCabiBindingValueTag::F64
        | ArcanaCabiBindingValueTag::Opaque
        | ArcanaCabiBindingValueTag::Unit => Ok(()),
    }
}

pub fn render_c_value_type_defs() -> String {
    concat!(
        "typedef struct ArcanaBytesView {\n",
        "    const uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaBytesView;\n\n",
        "typedef struct ArcanaStrView {\n",
        "    const uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaStrView;\n\n",
        "typedef struct ArcanaOwnedBytes {\n",
        "    uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaOwnedBytes;\n\n",
        "typedef struct ArcanaOwnedStr {\n",
        "    uint8_t* ptr;\n",
        "    size_t len;\n",
        "} ArcanaOwnedStr;\n\n",
        "typedef union ArcanaCabiBindingPayloadV1 {\n",
        "    int64_t int_value;\n",
        "    uint8_t bool_value;\n",
        "    int8_t i8_value;\n",
        "    uint8_t u8_value;\n",
        "    int16_t i16_value;\n",
        "    uint16_t u16_value;\n",
        "    int32_t i32_value;\n",
        "    uint32_t u32_value;\n",
        "    int64_t i64_value;\n",
        "    uint64_t u64_value;\n",
        "    intptr_t isize_value;\n",
        "    uintptr_t usize_value;\n",
        "    float f32_value;\n",
        "    double f64_value;\n",
        "    ArcanaStrView str_value;\n",
        "    ArcanaBytesView bytes_value;\n",
        "    uint64_t opaque_value;\n",
        "    ArcanaOwnedStr owned_str_value;\n",
        "    ArcanaOwnedBytes owned_bytes_value;\n",
        "} ArcanaCabiBindingPayloadV1;\n\n",
        "typedef struct ArcanaCabiBindingValueV1 {\n",
        "    uint32_t tag;\n",
        "    uint32_t reserved0;\n",
        "    uint64_t reserved1;\n",
        "    ArcanaCabiBindingPayloadV1 payload;\n",
        "} ArcanaCabiBindingValueV1;\n\n",
    )
    .to_string()
}

pub fn render_c_descriptor_type_defs() -> String {
    concat!(
        "typedef struct ArcanaCabiProductApiV1 {\n",
        "    size_t descriptor_size;\n",
        "    const char* package_name;\n",
        "    const char* product_name;\n",
        "    const char* role;\n",
        "    const char* contract_id;\n",
        "    uint32_t contract_version;\n",
        "    const void* role_ops;\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiProductApiV1;\n\n",
        "typedef struct ArcanaCabiExportParamV1 {\n",
        "    const char* name;\n",
        "    const char* source_mode;\n",
        "    const char* pass_mode;\n",
        "    const char* input_type;\n",
        "    const char* write_back_type;\n",
        "} ArcanaCabiExportParamV1;\n\n",
        "typedef struct ArcanaCabiExportEntryV1 {\n",
        "    const char* export_name;\n",
        "    const char* routine_key;\n",
        "    const char* symbol_name;\n",
        "    const char* return_type;\n",
        "    const ArcanaCabiExportParamV1* params;\n",
        "    size_t param_count;\n",
        "} ArcanaCabiExportEntryV1;\n\n",
        "typedef struct ArcanaCabiExportOpsV1 {\n",
        "    size_t ops_size;\n",
        "    const ArcanaCabiExportEntryV1* exports;\n",
        "    size_t export_count;\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    void (*owned_str_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiExportOpsV1;\n\n",
        "typedef struct ArcanaCabiInstanceOpsV1 {\n",
        "    size_t ops_size;\n",
        "    void* (*create_instance)(void);\n",
        "    void (*destroy_instance)(void* instance);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiInstanceOpsV1;\n\n",
        "typedef struct ArcanaCabiChildOpsV1 {\n",
        "    ArcanaCabiInstanceOpsV1 base;\n",
        "    int32_t (*run_entrypoint)(void* instance, const uint8_t* package_image_ptr, size_t package_image_len, const char* main_routine_key, int32_t* out_exit_code);\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiChildOpsV1;\n\n",
        "typedef struct ArcanaCabiPluginOpsV1 {\n",
        "    ArcanaCabiInstanceOpsV1 base;\n",
        "    uint8_t* (*describe_instance)(void* instance, size_t* out_len);\n",
        "    uint8_t* (*use_instance)(void* instance, const uint8_t* request_ptr, size_t request_len, size_t* out_len);\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiPluginOpsV1;\n\n",
        "typedef struct ArcanaCabiBindingImportEntryV1 {\n",
        "    const char* name;\n",
        "    const char* symbol_name;\n",
        "    const char* return_type;\n",
        "    const ArcanaCabiExportParamV1* params;\n",
        "    size_t param_count;\n",
        "} ArcanaCabiBindingImportEntryV1;\n\n",
        "typedef struct ArcanaCabiBindingCallbackEntryV1 {\n",
        "    const char* name;\n",
        "    const char* return_type;\n",
        "    const ArcanaCabiExportParamV1* params;\n",
        "    size_t param_count;\n",
        "} ArcanaCabiBindingCallbackEntryV1;\n\n",
        "typedef struct ArcanaCabiBindingLayoutEntryV1 {\n",
        "    const char* layout_id;\n",
        "    const char* detail_json;\n",
        "} ArcanaCabiBindingLayoutEntryV1;\n\n",
        "typedef struct ArcanaCabiBindingOpsV1 {\n",
        "    ArcanaCabiInstanceOpsV1 base;\n",
        "    const ArcanaCabiBindingImportEntryV1* imports;\n",
        "    size_t import_count;\n",
        "    const ArcanaCabiBindingCallbackEntryV1* callbacks;\n",
        "    size_t callback_count;\n",
        "    const ArcanaCabiBindingLayoutEntryV1* layouts;\n",
        "    size_t layout_count;\n",
        "    int32_t (*register_callback)(void* instance, const char* callback_name, int32_t (*callback)(void* user_data, const ArcanaCabiBindingValueV1* args, size_t arg_count, ArcanaCabiBindingValueV1* out_write_backs, ArcanaCabiBindingValueV1* out_result), void (*callback_owned_bytes_free)(uint8_t* ptr, size_t len), void (*callback_owned_str_free)(uint8_t* ptr, size_t len), void* user_data, uint64_t* out_handle);\n",
        "    int32_t (*unregister_callback)(void* instance, uint64_t handle);\n",
        "    uint8_t* (*last_error_alloc)(size_t* out_len);\n",
        "    void (*owned_bytes_free)(uint8_t* ptr, size_t len);\n",
        "    void (*owned_str_free)(uint8_t* ptr, size_t len);\n",
        "    const void* reserved0;\n",
        "    const void* reserved1;\n",
        "} ArcanaCabiBindingOpsV1;\n\n",
    )
    .to_string()
}

fn split_top_level_pair_args(text: &str) -> Result<(&str, &str), String> {
    let mut depth = 0usize;
    let mut split = None;
    let mut extra_split = None;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                if depth == 0 {
                    return Err(format!("invalid cabi type syntax `Pair[{text}]`"));
                }
                depth -= 1;
            }
            ',' if depth == 0 => {
                if split.is_none() {
                    split = Some(index);
                } else {
                    extra_split = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(split_index) = split else {
        return Err(format!("invalid cabi type syntax `Pair[{text}]`"));
    };
    if extra_split.is_some() {
        return Err(format!("invalid cabi type syntax `Pair[{text}]`"));
    }
    let left = text[..split_index].trim();
    let right = text[split_index + 1..].trim();
    if left.is_empty() || right.is_empty() {
        return Err(format!("invalid cabi type syntax `Pair[{text}]`"));
    }
    Ok((left, right))
}

fn validate_binding_named_entries(
    kind: ArcanaCabiBindingSignatureKind,
    entries: Vec<ArcanaCabiBindingSignature>,
) -> Result<(), String> {
    let by_name = binding_signatures_by_name(kind, &entries)?;
    for (name, signature) in by_name {
        if name.trim().is_empty() {
            return Err(format!("{} name cannot be empty", kind.label()));
        }
        validate_binding_transport_type(&signature.return_type)?;
        let mut seen_param_names = BTreeMap::<&str, usize>::new();
        for (index, param) in signature.params.iter().enumerate() {
            validate_binding_param(param)?;
            if let Some(previous) = seen_param_names.insert(param.name.as_str(), index) {
                return Err(format!(
                    "{} `{}` declares duplicate param `{}` at indices {} and {}",
                    kind.label(),
                    signature.name,
                    param.name,
                    previous,
                    index
                ));
            }
        }
    }
    Ok(())
}

fn binding_signatures_by_name(
    kind: ArcanaCabiBindingSignatureKind,
    entries: &[ArcanaCabiBindingSignature],
) -> Result<BTreeMap<&str, &ArcanaCabiBindingSignature>, String> {
    let mut by_name = BTreeMap::new();
    for entry in entries {
        if let Some(existing) = by_name.insert(entry.name.as_str(), entry) {
            return Err(format!(
                "{} `{}` is declared more than once",
                kind.label(),
                existing.name
            ));
        }
    }
    Ok(by_name)
}

#[cfg(test)]
mod tests {
    use super::{
        ArcanaCabiBindingCallback, ArcanaCabiBindingLayout, ArcanaCabiBindingLayoutField,
        ArcanaCabiBindingLayoutKind, ArcanaCabiBindingParam, ArcanaCabiBindingRawType,
        ArcanaCabiBindingScalarType, ArcanaCabiBindingSignature, ArcanaCabiBindingSignatureKind,
        ArcanaCabiBindingType, ArcanaCabiBindingValueTag, ArcanaCabiBindingValueV1,
        ArcanaCabiParamSourceMode, ArcanaStrView, binding_write_back_slots,
        clone_owned_binding_bytes, clone_owned_binding_str, compare_binding_layouts,
        compare_binding_signatures, free_owned_bytes, free_owned_str, into_owned_bytes,
        into_owned_str, release_binding_output_value, render_c_descriptor_type_defs,
        render_c_value_type_defs, validate_binding_callbacks, validate_binding_layouts,
        validate_binding_write_backs,
    };

    unsafe extern "system" fn test_free_owned_bytes(ptr: *mut u8, len: usize) {
        unsafe {
            free_owned_bytes(ptr, len);
        }
    }

    unsafe extern "system" fn test_free_owned_str(ptr: *mut u8, len: usize) {
        unsafe {
            free_owned_str(ptr, len);
        }
    }

    fn binding_int(value: i64) -> ArcanaCabiBindingValueV1 {
        ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Int as u32,
            reserved0: 0,
            reserved1: 0,
            payload: super::ArcanaCabiBindingPayloadV1 { int_value: value },
        }
    }

    fn binding_owned_bytes(bytes: &[u8]) -> ArcanaCabiBindingValueV1 {
        ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Bytes as u32,
            reserved0: 0,
            reserved1: 0,
            payload: super::ArcanaCabiBindingPayloadV1 {
                owned_bytes_value: into_owned_bytes(bytes.to_vec()),
            },
        }
    }

    fn binding_owned_str(text: &str) -> ArcanaCabiBindingValueV1 {
        ArcanaCabiBindingValueV1 {
            tag: ArcanaCabiBindingValueTag::Str as u32,
            reserved0: 0,
            reserved1: 0,
            payload: super::ArcanaCabiBindingPayloadV1 {
                owned_str_value: into_owned_str(text.to_string()),
            },
        }
    }

    unsafe extern "system" fn fixture_callback(
        _user_data: *mut std::ffi::c_void,
        args: *const ArcanaCabiBindingValueV1,
        arg_count: usize,
        out_write_backs: *mut ArcanaCabiBindingValueV1,
        out_result: *mut ArcanaCabiBindingValueV1,
    ) -> i32 {
        if args.is_null() || out_write_backs.is_null() || out_result.is_null() || arg_count != 2 {
            return 0;
        }
        let args = unsafe { std::slice::from_raw_parts(args, arg_count) };
        let slots = unsafe { std::slice::from_raw_parts_mut(out_write_backs, arg_count) };
        slots[0] = ArcanaCabiBindingValueV1::default();
        slots[1] = binding_owned_str("edited");
        unsafe {
            *out_result = binding_owned_bytes(match args[0].tag() {
                Ok(ArcanaCabiBindingValueTag::Str) => b"callback",
                _ => b"unexpected",
            });
        }
        1
    }

    #[test]
    fn render_c_value_type_defs_includes_owned_and_view_buffers() {
        let text = render_c_value_type_defs();
        assert!(text.contains("typedef struct ArcanaBytesView"));
        assert!(text.contains("typedef struct ArcanaOwnedStr"));
        assert!(text.contains("typedef struct ArcanaCabiBindingValueV1"));
    }

    #[test]
    fn render_c_descriptor_type_defs_includes_export_and_instance_ops() {
        let text = render_c_descriptor_type_defs();
        assert!(text.contains("typedef struct ArcanaCabiProductApiV1"));
        assert!(text.contains("typedef struct ArcanaCabiExportOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiInstanceOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiChildOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiPluginOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiBindingOpsV1"));
        assert!(text.contains("use_instance"));
        assert!(text.contains("owned_str_free"));
        assert!(text.contains("ArcanaCabiBindingValueV1* out_write_backs"));
        assert!(text.contains("callback_owned_bytes_free"));
        assert!(!text.contains("typedef struct ArcanaCabiProviderOpsV1"));
    }

    #[test]
    fn binding_write_back_slots_default_to_unit_and_validate_non_edit_rows() {
        let params = vec![
            ArcanaCabiBindingParam::binding(
                "source",
                ArcanaCabiParamSourceMode::Read,
                ArcanaCabiBindingType::Str,
            ),
            ArcanaCabiBindingParam::binding(
                "target",
                ArcanaCabiParamSourceMode::Edit,
                ArcanaCabiBindingType::Str,
            ),
        ];
        let mut slots = binding_write_back_slots(&params);
        assert_eq!(slots.len(), 2);
        assert_eq!(
            slots[0].tag().expect("tag should parse"),
            ArcanaCabiBindingValueTag::Unit
        );
        assert_eq!(
            slots[1].tag().expect("tag should parse"),
            ArcanaCabiBindingValueTag::Unit
        );

        slots[1] = binding_owned_str("mutated");
        validate_binding_write_backs(&params, &slots).expect("edit write-back should validate");

        slots[0] = binding_int(9);
        let err = validate_binding_write_backs(&params, &slots)
            .expect_err("non-edit write-back must fail");
        assert!(err.contains("must be Unit"), "{err}");

        release_binding_output_value(slots[1], test_free_owned_bytes, test_free_owned_str)
            .expect("owned slot should release");
    }

    #[test]
    fn owned_buffer_helpers_round_trip_strings_and_bytes() {
        let bytes = into_owned_bytes(vec![1, 2, 3]);
        assert_eq!(
            clone_owned_binding_bytes(bytes, test_free_owned_bytes).expect("bytes should clone"),
            vec![1, 2, 3]
        );

        let text = into_owned_str("arcana".to_string());
        assert_eq!(
            clone_owned_binding_str(text, test_free_owned_str).expect("str should clone"),
            "arcana"
        );

        let bytes = into_owned_bytes(vec![4, 5, 6]);
        unsafe {
            test_free_owned_bytes(bytes.ptr, bytes.len);
        }
        let text = into_owned_str("free".to_string());
        unsafe {
            test_free_owned_str(text.ptr, text.len);
        }
    }

    #[test]
    fn generic_callback_fixture_round_trips_owned_result_and_edit_write_back() {
        let params = [
            ArcanaCabiBindingValueV1 {
                tag: ArcanaCabiBindingValueTag::Str as u32,
                reserved0: 0,
                reserved1: 0,
                payload: super::ArcanaCabiBindingPayloadV1 {
                    str_value: ArcanaStrView {
                        ptr: b"arcana".as_ptr(),
                        len: "arcana".len(),
                    },
                },
            },
            ArcanaCabiBindingValueV1::default(),
        ];
        let mut write_backs = [
            ArcanaCabiBindingValueV1::default(),
            ArcanaCabiBindingValueV1::default(),
        ];
        let mut result = ArcanaCabiBindingValueV1::default();
        let ok = unsafe {
            fixture_callback(
                std::ptr::null_mut(),
                params.as_ptr(),
                params.len(),
                write_backs.as_mut_ptr(),
                &mut result,
            )
        };
        assert_eq!(ok, 1);
        assert_eq!(
            clone_owned_binding_bytes(
                unsafe { result.payload.owned_bytes_value },
                test_free_owned_bytes,
            )
            .expect("result bytes should clone"),
            b"callback"
        );
        assert_eq!(
            clone_owned_binding_str(
                unsafe { write_backs[1].payload.owned_str_value },
                test_free_owned_str,
            )
            .expect("write-back str should clone"),
            "edited"
        );
    }

    #[test]
    fn binding_signature_validation_rejects_pair_types_and_mismatches() {
        let callbacks = vec![ArcanaCabiBindingCallback {
            name: "cb".to_string(),
            return_type: ArcanaCabiBindingType::Unit,
            params: vec![ArcanaCabiBindingParam {
                name: "pair".to_string(),
                source_mode: ArcanaCabiParamSourceMode::Read,
                pass_mode: super::ArcanaCabiPassMode::In,
                input_type: ArcanaCabiBindingType::Named("Pair[Int, Bool]".to_string()),
                write_back_type: None,
            }],
        }];
        validate_binding_callbacks(&callbacks)
            .expect("named layout transport should validate even before the layout exists");

        let expected = vec![ArcanaCabiBindingSignature {
            name: "cb".to_string(),
            return_type: ArcanaCabiBindingType::Int,
            params: vec![ArcanaCabiBindingParam::binding(
                "value",
                ArcanaCabiParamSourceMode::Edit,
                ArcanaCabiBindingType::Str,
            )],
        }];
        let actual = vec![ArcanaCabiBindingSignature {
            name: "cb".to_string(),
            return_type: ArcanaCabiBindingType::Bool,
            params: vec![ArcanaCabiBindingParam::binding(
                "value",
                ArcanaCabiParamSourceMode::Edit,
                ArcanaCabiBindingType::Str,
            )],
        }];
        let err = compare_binding_signatures(
            ArcanaCabiBindingSignatureKind::Callback,
            &expected,
            &actual,
        )
        .expect_err("return mismatch should fail");
        assert!(err.contains("return type mismatch"), "{err}");
    }

    #[test]
    fn binding_layout_validation_accepts_struct_array_callback_and_interface_shapes() {
        let layouts = vec![
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.Rect".to_string(),
                size: 12,
                align: 4,
                kind: ArcanaCabiBindingLayoutKind::Struct {
                    fields: vec![
                        ArcanaCabiBindingLayoutField {
                            name: "x".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                            offset: 0,
                            bit_width: None,
                            bit_offset: None,
                        },
                        ArcanaCabiBindingLayoutField {
                            name: "y".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                            offset: 4,
                            bit_width: None,
                            bit_offset: None,
                        },
                        ArcanaCabiBindingLayoutField {
                            name: "flags".to_string(),
                            ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U32),
                            offset: 8,
                            bit_width: Some(3),
                            bit_offset: Some(0),
                        },
                    ],
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.PointArray".to_string(),
                size: 24,
                align: 4,
                kind: ArcanaCabiBindingLayoutKind::Array {
                    element_type: ArcanaCabiBindingRawType::Named("hostapi.raw.Rect".to_string()),
                    len: 2,
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.Mode".to_string(),
                size: 4,
                align: 4,
                kind: ArcanaCabiBindingLayoutKind::Enum {
                    repr: ArcanaCabiBindingScalarType::U32,
                    variants: vec![
                        crate::ArcanaCabiBindingLayoutEnumVariant {
                            name: "Idle".to_string(),
                            value: 0,
                        },
                        crate::ArcanaCabiBindingLayoutEnumVariant {
                            name: "Busy".to_string(),
                            value: 1,
                        },
                    ],
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.WindowProc".to_string(),
                size: std::mem::size_of::<usize>(),
                align: std::mem::size_of::<usize>(),
                kind: ArcanaCabiBindingLayoutKind::Callback {
                    abi: "system".to_string(),
                    params: vec![
                        ArcanaCabiBindingRawType::Pointer {
                            mutable: false,
                            inner: Box::new(ArcanaCabiBindingRawType::Void),
                        },
                        ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U32),
                    ],
                    return_type: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::I32),
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.IUnknownVTable".to_string(),
                size: std::mem::size_of::<usize>() * 3,
                align: std::mem::size_of::<usize>(),
                kind: ArcanaCabiBindingLayoutKind::Struct {
                    fields: vec![ArcanaCabiBindingLayoutField {
                        name: "query_interface".to_string(),
                        ty: ArcanaCabiBindingRawType::FunctionPointer {
                            abi: "system".to_string(),
                            nullable: false,
                            params: vec![ArcanaCabiBindingRawType::Pointer {
                                mutable: false,
                                inner: Box::new(ArcanaCabiBindingRawType::Void),
                            }],
                            return_type: Box::new(ArcanaCabiBindingRawType::Scalar(
                                ArcanaCabiBindingScalarType::I32,
                            )),
                        },
                        offset: 0,
                        bit_width: None,
                        bit_offset: None,
                    }],
                },
            },
            ArcanaCabiBindingLayout {
                layout_id: "hostapi.raw.IUnknown".to_string(),
                size: std::mem::size_of::<usize>(),
                align: std::mem::size_of::<usize>(),
                kind: ArcanaCabiBindingLayoutKind::Interface {
                    iid: Some("00000000-0000-0000-C000-000000000046".to_string()),
                    vtable_layout_id: Some("hostapi.raw.IUnknownVTable".to_string()),
                },
            },
        ];

        validate_binding_layouts(&layouts).expect("raw binding layouts should validate");
        compare_binding_layouts(&layouts, &layouts).expect("identical layout tables should match");
    }

    #[test]
    fn binding_layout_validation_rejects_missing_refs_and_bad_bitfields() {
        let layouts = vec![ArcanaCabiBindingLayout {
            layout_id: "hostapi.raw.Bad".to_string(),
            size: 4,
            align: 4,
            kind: ArcanaCabiBindingLayoutKind::Struct {
                fields: vec![ArcanaCabiBindingLayoutField {
                    name: "missing".to_string(),
                    ty: ArcanaCabiBindingRawType::Named("hostapi.raw.Missing".to_string()),
                    offset: 0,
                    bit_width: None,
                    bit_offset: None,
                }],
            },
        }];
        let err = validate_binding_layouts(&layouts)
            .expect_err("missing raw layout refs should fail validation");
        assert!(err.contains("Missing"), "{err}");

        let layouts = vec![ArcanaCabiBindingLayout {
            layout_id: "hostapi.raw.Flags".to_string(),
            size: 4,
            align: 4,
            kind: ArcanaCabiBindingLayoutKind::Struct {
                fields: vec![ArcanaCabiBindingLayoutField {
                    name: "bad".to_string(),
                    ty: ArcanaCabiBindingRawType::Scalar(ArcanaCabiBindingScalarType::U32),
                    offset: 0,
                    bit_width: Some(33),
                    bit_offset: Some(0),
                }],
            },
        }];
        let err = validate_binding_layouts(&layouts).expect_err("oversized bitfields should fail");
        assert!(err.contains("exceeds its storage unit"), "{err}");
    }
}
