use std::ffi::{c_char, c_void};

use serde::{Deserialize, Serialize};

pub const ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL: &str = "arcana_cabi_get_product_api_v1";
pub const ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL: &str = "arcana_cabi_last_error_alloc_v1";
pub const ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL: &str = "arcana_cabi_owned_bytes_free_v1";
pub const ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL: &str = "arcana_cabi_owned_str_free_v1";

pub const ARCANA_CABI_EXPORT_CONTRACT_ID: &str = "arcana.cabi.export.v1";
pub const ARCANA_CABI_CHILD_CONTRACT_ID: &str = "arcana.cabi.child.v1";
pub const ARCANA_CABI_PLUGIN_CONTRACT_ID: &str = "arcana.cabi.plugin.v1";
pub const ARCANA_CABI_CONTRACT_VERSION_V1: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArcanaCabiProductRole {
    Export,
    Child,
    Plugin,
}

impl ArcanaCabiProductRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Child => "child",
            Self::Plugin => "plugin",
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "export" => Ok(Self::Export),
            "child" => Ok(Self::Child),
            "plugin" => Ok(Self::Plugin),
            other => Err(format!(
                "`role` must be \"export\", \"child\", or \"plugin\" (found `{other}`)"
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
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArcanaCabiType {
    Int,
    Bool,
    Str,
    Bytes,
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
            Self::Unit => "Unit".to_string(),
            Self::Pair(left, right) => format!("Pair[{}, {}]", left.render(), right.render()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiExportParam {
    pub name: String,
    pub source_mode: ArcanaCabiParamSourceMode,
    pub pass_mode: ArcanaCabiPassMode,
    pub input_type: ArcanaCabiType,
    pub write_back_type: Option<ArcanaCabiType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcanaCabiExport {
    pub export_name: String,
    pub routine_key: String,
    pub symbol_name: String,
    pub return_type: ArcanaCabiType,
    pub params: Vec<ArcanaCabiExportParam>,
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
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{render_c_descriptor_type_defs, render_c_value_type_defs};

    #[test]
    fn render_c_value_type_defs_includes_owned_and_view_buffers() {
        let text = render_c_value_type_defs();
        assert!(text.contains("typedef struct ArcanaBytesView"));
        assert!(text.contains("typedef struct ArcanaOwnedStr"));
    }

    #[test]
    fn render_c_descriptor_type_defs_includes_export_and_instance_ops() {
        let text = render_c_descriptor_type_defs();
        assert!(text.contains("typedef struct ArcanaCabiProductApiV1"));
        assert!(text.contains("typedef struct ArcanaCabiExportOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiInstanceOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiChildOpsV1"));
        assert!(text.contains("typedef struct ArcanaCabiPluginOpsV1"));
        assert!(text.contains("use_instance"));
        assert!(text.contains("owned_str_free"));
    }
}
