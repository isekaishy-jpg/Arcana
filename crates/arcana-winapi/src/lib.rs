#![cfg(windows)]

use arcana_cabi::{
    ARCANA_CABI_CONTRACT_VERSION_V1,
    ArcanaCabiBindingCallbackEntryV1, ArcanaCabiBindingCallbackFn,
    ArcanaCabiBindingImportEntryV1, ArcanaCabiBindingOpsV1, ArcanaCabiBindingPayloadV1,
    ArcanaCabiBindingRegisterCallbackFn, ArcanaCabiBindingUnregisterCallbackFn,
    ArcanaCabiBindingValueTag, ArcanaCabiBindingValueV1, ArcanaCabiCreateInstanceFn,
    ArcanaCabiDestroyInstanceFn, ArcanaCabiExportParamV1, ArcanaCabiInstanceOpsV1,
    ArcanaCabiLastErrorAllocFn, ArcanaCabiOwnedBytesFreeFn, ArcanaCabiOwnedStrFreeFn,
    ArcanaCabiProductApiV1, ArcanaOwnedStr,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{CStr, c_char, c_void};
use std::ptr::{self, null, null_mut};
use std::sync::OnceLock;
use winapi::Interface;
use winapi::ctypes::c_void as winapi_c_void;
use winapi::shared::minwindef::{BOOL, FALSE};
use winapi::shared::winerror::RPC_E_CHANGED_MODE;
use winapi::um::dwrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory, IDWriteFont,
    IDWriteFontCollection, IDWriteFontFamily, IDWriteFontFile, IDWriteFontFileLoader,
    IDWriteFontFace, IDWriteLocalizedStrings, IDWriteLocalFontFileLoader,
};
use winapi::um::unknwnbase::IUnknown;
use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, GetLastError, HMODULE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GWLP_USERDATA, GetWindowLongPtrW, MSG, PM_REMOVE, PeekMessageW, PostMessageW, RegisterClassW,
    SetWindowLongPtrW, TranslateMessage, WM_APP, WM_CLOSE, WM_NCCREATE, WNDCLASSW, WS_OVERLAPPED,
};

static PACKAGE_NAME: &[u8] = b"arcana_winapi\0";
static PRODUCT_NAME: &[u8] = b"default\0";
static ROLE_NAME: &[u8] = b"binding\0";
static CONTRACT_ID: &[u8] = b"arcana.cabi.binding.v1\0";

static IMPORT_NAME_FOUNDATION_CURRENT_MODULE: &[u8] = b"foundation.current_module\0";
static IMPORT_SYMBOL_FOUNDATION_CURRENT_MODULE: &[u8] =
    b"arcana_winapi_import_foundation_current_module\0";
static IMPORT_NAME_FOUNDATION_MODULE_IS_NULL: &[u8] = b"foundation.module_is_null\0";
static IMPORT_SYMBOL_FOUNDATION_MODULE_IS_NULL: &[u8] =
    b"arcana_winapi_import_foundation_module_is_null\0";
static IMPORT_NAME_FOUNDATION_MODULE_PATH: &[u8] = b"foundation.module_path\0";
static IMPORT_SYMBOL_FOUNDATION_MODULE_PATH: &[u8] =
    b"arcana_winapi_import_foundation_module_path\0";
static IMPORT_NAME_FOUNDATION_UTF16_LEN: &[u8] = b"foundation.utf16_len\0";
static IMPORT_SYMBOL_FOUNDATION_UTF16_LEN: &[u8] =
    b"arcana_winapi_import_foundation_utf16_len\0";
static IMPORT_NAME_FOUNDATION_FAIL_SAMPLE: &[u8] = b"foundation.fail_sample\0";
static IMPORT_SYMBOL_FOUNDATION_FAIL_SAMPLE: &[u8] =
    b"arcana_winapi_import_foundation_fail_sample\0";

static IMPORT_NAME_FONTS_SYSTEM_FONT_CATALOG: &[u8] = b"fonts.system_font_catalog\0";
static IMPORT_SYMBOL_FONTS_SYSTEM_FONT_CATALOG: &[u8] =
    b"arcana_winapi_import_fonts_system_font_catalog\0";
static IMPORT_NAME_FONTS_CATALOG_COUNT: &[u8] = b"fonts.catalog_count\0";
static IMPORT_SYMBOL_FONTS_CATALOG_COUNT: &[u8] =
    b"arcana_winapi_import_fonts_catalog_count\0";
static IMPORT_NAME_FONTS_CATALOG_FAMILY_NAME: &[u8] = b"fonts.catalog_family_name\0";
static IMPORT_SYMBOL_FONTS_CATALOG_FAMILY_NAME: &[u8] =
    b"arcana_winapi_import_fonts_catalog_family_name\0";
static IMPORT_NAME_FONTS_CATALOG_FACE_NAME: &[u8] = b"fonts.catalog_face_name\0";
static IMPORT_SYMBOL_FONTS_CATALOG_FACE_NAME: &[u8] =
    b"arcana_winapi_import_fonts_catalog_face_name\0";
static IMPORT_NAME_FONTS_CATALOG_FULL_NAME: &[u8] = b"fonts.catalog_full_name\0";
static IMPORT_SYMBOL_FONTS_CATALOG_FULL_NAME: &[u8] =
    b"arcana_winapi_import_fonts_catalog_full_name\0";
static IMPORT_NAME_FONTS_CATALOG_POSTSCRIPT_NAME: &[u8] = b"fonts.catalog_postscript_name\0";
static IMPORT_SYMBOL_FONTS_CATALOG_POSTSCRIPT_NAME: &[u8] =
    b"arcana_winapi_import_fonts_catalog_postscript_name\0";
static IMPORT_NAME_FONTS_CATALOG_PATH: &[u8] = b"fonts.catalog_path\0";
static IMPORT_SYMBOL_FONTS_CATALOG_PATH: &[u8] = b"arcana_winapi_import_fonts_catalog_path\0";
static IMPORT_NAME_FONTS_CATALOG_DESTROY: &[u8] = b"fonts.catalog_destroy\0";
static IMPORT_SYMBOL_FONTS_CATALOG_DESTROY: &[u8] =
    b"arcana_winapi_import_fonts_catalog_destroy\0";

static IMPORT_NAME_WINDOWS_CREATE_HIDDEN_WINDOW: &[u8] = b"windows.create_hidden_window\0";
static IMPORT_SYMBOL_WINDOWS_CREATE_HIDDEN_WINDOW: &[u8] =
    b"arcana_winapi_import_windows_create_hidden_window\0";
static IMPORT_NAME_WINDOWS_POST_PING: &[u8] = b"windows.post_ping\0";
static IMPORT_SYMBOL_WINDOWS_POST_PING: &[u8] = b"arcana_winapi_import_windows_post_ping\0";
static IMPORT_NAME_WINDOWS_PUMP_MESSAGES: &[u8] = b"windows.pump_messages\0";
static IMPORT_SYMBOL_WINDOWS_PUMP_MESSAGES: &[u8] =
    b"arcana_winapi_import_windows_pump_messages\0";
static IMPORT_NAME_WINDOWS_TAKE_LAST_CALLBACK_CODE: &[u8] =
    b"windows.take_last_callback_code\0";
static IMPORT_SYMBOL_WINDOWS_TAKE_LAST_CALLBACK_CODE: &[u8] =
    b"arcana_winapi_import_windows_take_last_callback_code\0";
static IMPORT_NAME_WINDOWS_DESTROY_HIDDEN_WINDOW: &[u8] = b"windows.destroy_hidden_window\0";
static IMPORT_SYMBOL_WINDOWS_DESTROY_HIDDEN_WINDOW: &[u8] =
    b"arcana_winapi_import_windows_destroy_hidden_window\0";

static CALLBACK_NAME_WINDOW_PROC: &[u8] = b"window_proc\0";

static TYPE_INT: &[u8] = b"Int\0";
static TYPE_BOOL: &[u8] = b"Bool\0";
static TYPE_STR: &[u8] = b"Str\0";
static TYPE_UNIT: &[u8] = b"Unit\0";
static TYPE_MODULE_HANDLE: &[u8] = b"arcana_winapi.types.ModuleHandle\0";
static TYPE_SYSTEM_FONT_CATALOG: &[u8] = b"arcana_winapi.types.SystemFontCatalog\0";
static TYPE_HIDDEN_WINDOW: &[u8] = b"arcana_winapi.types.HiddenWindow\0";
static SOURCE_MODE_READ: &[u8] = b"read\0";
static SOURCE_MODE_TAKE: &[u8] = b"take\0";
static PASS_MODE_IN: &[u8] = b"in\0";

static PARAM_MODULE_READ: [ArcanaCabiExportParamV1; 1] = [ArcanaCabiExportParamV1 {
    name: b"module\0".as_ptr().cast(),
    source_mode: SOURCE_MODE_READ.as_ptr().cast(),
    pass_mode: PASS_MODE_IN.as_ptr().cast(),
    input_type: TYPE_MODULE_HANDLE.as_ptr().cast(),
    write_back_type: ptr::null(),
}];
static PARAM_TEXT_READ: [ArcanaCabiExportParamV1; 1] = [ArcanaCabiExportParamV1 {
    name: b"text\0".as_ptr().cast(),
    source_mode: SOURCE_MODE_READ.as_ptr().cast(),
    pass_mode: PASS_MODE_IN.as_ptr().cast(),
    input_type: TYPE_STR.as_ptr().cast(),
    write_back_type: ptr::null(),
}];
static PARAM_CATALOG_READ: [ArcanaCabiExportParamV1; 1] = [ArcanaCabiExportParamV1 {
    name: b"catalog\0".as_ptr().cast(),
    source_mode: SOURCE_MODE_READ.as_ptr().cast(),
    pass_mode: PASS_MODE_IN.as_ptr().cast(),
    input_type: TYPE_SYSTEM_FONT_CATALOG.as_ptr().cast(),
    write_back_type: ptr::null(),
}];
static PARAM_CATALOG_TAKE: [ArcanaCabiExportParamV1; 1] = [ArcanaCabiExportParamV1 {
    name: b"catalog\0".as_ptr().cast(),
    source_mode: SOURCE_MODE_TAKE.as_ptr().cast(),
    pass_mode: PASS_MODE_IN.as_ptr().cast(),
    input_type: TYPE_SYSTEM_FONT_CATALOG.as_ptr().cast(),
    write_back_type: ptr::null(),
}];
static PARAM_CATALOG_INDEX: [ArcanaCabiExportParamV1; 2] = [
    ArcanaCabiExportParamV1 {
        name: b"catalog\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_SYSTEM_FONT_CATALOG.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
    ArcanaCabiExportParamV1 {
        name: b"index\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_INT.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
];
static PARAM_WINDOW_TAKE: [ArcanaCabiExportParamV1; 1] = [ArcanaCabiExportParamV1 {
    name: b"window\0".as_ptr().cast(),
    source_mode: SOURCE_MODE_TAKE.as_ptr().cast(),
    pass_mode: PASS_MODE_IN.as_ptr().cast(),
    input_type: TYPE_HIDDEN_WINDOW.as_ptr().cast(),
    write_back_type: ptr::null(),
}];
static PARAM_WINDOW_CODE: [ArcanaCabiExportParamV1; 2] = [
    ArcanaCabiExportParamV1 {
        name: b"window\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_HIDDEN_WINDOW.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
    ArcanaCabiExportParamV1 {
        name: b"code\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_INT.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
];
static CALLBACK_PARAMS_WINDOW_PROC: [ArcanaCabiExportParamV1; 4] = [
    ArcanaCabiExportParamV1 {
        name: b"window\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_HIDDEN_WINDOW.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
    ArcanaCabiExportParamV1 {
        name: b"message\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_INT.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
    ArcanaCabiExportParamV1 {
        name: b"wparam\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_INT.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
    ArcanaCabiExportParamV1 {
        name: b"lparam\0".as_ptr().cast(),
        source_mode: SOURCE_MODE_READ.as_ptr().cast(),
        pass_mode: PASS_MODE_IN.as_ptr().cast(),
        input_type: TYPE_INT.as_ptr().cast(),
        write_back_type: ptr::null(),
    },
];

thread_local! {
    static LAST_ERROR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Default)]
struct BindingInstance {
    callbacks_by_name: BTreeMap<String, RegisteredCallback>,
    handles_to_name: BTreeMap<u64, String>,
    next_handle: u64,
    last_callback_code: i64,
    com_initialized: bool,
}

#[derive(Clone, Copy, Debug)]
struct RegisteredCallback {
    callback: ArcanaCabiBindingCallbackFn,
    user_data: *mut c_void,
}

#[derive(Debug)]
struct SystemFontCatalogState {
    entries: Vec<SystemFontEntry>,
}

#[derive(Clone, Debug)]
struct SystemFontEntry {
    family_name: String,
    face_name: String,
    full_name: String,
    postscript_name: String,
    path: String,
}

#[derive(Debug)]
struct ComPtr<T>(*mut T);

impl<T> ComPtr<T> {
    fn null() -> Self {
        Self(null_mut())
    }

    fn as_ptr(&self) -> *mut T {
        self.0
    }

    fn as_mut_ptr(&mut self) -> *mut *mut T {
        &mut self.0
    }

    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl<T> Drop for ComPtr<T> {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                (*(self.0 as *mut IUnknown)).Release();
            }
        }
    }
}

static BINDING_IMPORTS: [ArcanaCabiBindingImportEntryV1; 18] = [
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FOUNDATION_CURRENT_MODULE.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FOUNDATION_CURRENT_MODULE.as_ptr().cast(),
        return_type: TYPE_MODULE_HANDLE.as_ptr().cast(),
        params: ptr::null(),
        param_count: 0,
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FOUNDATION_MODULE_IS_NULL.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FOUNDATION_MODULE_IS_NULL.as_ptr().cast(),
        return_type: TYPE_BOOL.as_ptr().cast(),
        params: PARAM_MODULE_READ.as_ptr(),
        param_count: PARAM_MODULE_READ.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FOUNDATION_MODULE_PATH.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FOUNDATION_MODULE_PATH.as_ptr().cast(),
        return_type: TYPE_STR.as_ptr().cast(),
        params: PARAM_MODULE_READ.as_ptr(),
        param_count: PARAM_MODULE_READ.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FOUNDATION_UTF16_LEN.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FOUNDATION_UTF16_LEN.as_ptr().cast(),
        return_type: TYPE_INT.as_ptr().cast(),
        params: PARAM_TEXT_READ.as_ptr(),
        param_count: PARAM_TEXT_READ.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FOUNDATION_FAIL_SAMPLE.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FOUNDATION_FAIL_SAMPLE.as_ptr().cast(),
        return_type: TYPE_INT.as_ptr().cast(),
        params: PARAM_TEXT_READ.as_ptr(),
        param_count: PARAM_TEXT_READ.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_SYSTEM_FONT_CATALOG.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_SYSTEM_FONT_CATALOG.as_ptr().cast(),
        return_type: TYPE_SYSTEM_FONT_CATALOG.as_ptr().cast(),
        params: ptr::null(),
        param_count: 0,
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_CATALOG_COUNT.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_CATALOG_COUNT.as_ptr().cast(),
        return_type: TYPE_INT.as_ptr().cast(),
        params: PARAM_CATALOG_READ.as_ptr(),
        param_count: PARAM_CATALOG_READ.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_CATALOG_FAMILY_NAME.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_CATALOG_FAMILY_NAME.as_ptr().cast(),
        return_type: TYPE_STR.as_ptr().cast(),
        params: PARAM_CATALOG_INDEX.as_ptr(),
        param_count: PARAM_CATALOG_INDEX.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_CATALOG_FACE_NAME.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_CATALOG_FACE_NAME.as_ptr().cast(),
        return_type: TYPE_STR.as_ptr().cast(),
        params: PARAM_CATALOG_INDEX.as_ptr(),
        param_count: PARAM_CATALOG_INDEX.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_CATALOG_FULL_NAME.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_CATALOG_FULL_NAME.as_ptr().cast(),
        return_type: TYPE_STR.as_ptr().cast(),
        params: PARAM_CATALOG_INDEX.as_ptr(),
        param_count: PARAM_CATALOG_INDEX.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_CATALOG_POSTSCRIPT_NAME.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_CATALOG_POSTSCRIPT_NAME.as_ptr().cast(),
        return_type: TYPE_STR.as_ptr().cast(),
        params: PARAM_CATALOG_INDEX.as_ptr(),
        param_count: PARAM_CATALOG_INDEX.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_CATALOG_PATH.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_CATALOG_PATH.as_ptr().cast(),
        return_type: TYPE_STR.as_ptr().cast(),
        params: PARAM_CATALOG_INDEX.as_ptr(),
        param_count: PARAM_CATALOG_INDEX.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_FONTS_CATALOG_DESTROY.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_FONTS_CATALOG_DESTROY.as_ptr().cast(),
        return_type: TYPE_UNIT.as_ptr().cast(),
        params: PARAM_CATALOG_TAKE.as_ptr(),
        param_count: PARAM_CATALOG_TAKE.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_WINDOWS_CREATE_HIDDEN_WINDOW.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_WINDOWS_CREATE_HIDDEN_WINDOW.as_ptr().cast(),
        return_type: TYPE_HIDDEN_WINDOW.as_ptr().cast(),
        params: ptr::null(),
        param_count: 0,
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_WINDOWS_POST_PING.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_WINDOWS_POST_PING.as_ptr().cast(),
        return_type: TYPE_UNIT.as_ptr().cast(),
        params: PARAM_WINDOW_CODE.as_ptr(),
        param_count: PARAM_WINDOW_CODE.len(),
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_WINDOWS_PUMP_MESSAGES.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_WINDOWS_PUMP_MESSAGES.as_ptr().cast(),
        return_type: TYPE_INT.as_ptr().cast(),
        params: ptr::null(),
        param_count: 0,
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_WINDOWS_TAKE_LAST_CALLBACK_CODE.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_WINDOWS_TAKE_LAST_CALLBACK_CODE.as_ptr().cast(),
        return_type: TYPE_INT.as_ptr().cast(),
        params: ptr::null(),
        param_count: 0,
    },
    ArcanaCabiBindingImportEntryV1 {
        name: IMPORT_NAME_WINDOWS_DESTROY_HIDDEN_WINDOW.as_ptr().cast(),
        symbol_name: IMPORT_SYMBOL_WINDOWS_DESTROY_HIDDEN_WINDOW.as_ptr().cast(),
        return_type: TYPE_UNIT.as_ptr().cast(),
        params: PARAM_WINDOW_TAKE.as_ptr(),
        param_count: PARAM_WINDOW_TAKE.len(),
    },
];

static BINDING_CALLBACKS: [ArcanaCabiBindingCallbackEntryV1; 1] = [ArcanaCabiBindingCallbackEntryV1 {
    name: CALLBACK_NAME_WINDOW_PROC.as_ptr().cast(),
    return_type: TYPE_INT.as_ptr().cast(),
    params: CALLBACK_PARAMS_WINDOW_PROC.as_ptr(),
    param_count: CALLBACK_PARAMS_WINDOW_PROC.len(),
}];

static BINDING_OPS: ArcanaCabiBindingOpsV1 = ArcanaCabiBindingOpsV1 {
    base: ArcanaCabiInstanceOpsV1 {
        ops_size: std::mem::size_of::<ArcanaCabiInstanceOpsV1>(),
        create_instance: create_instance as ArcanaCabiCreateInstanceFn,
        destroy_instance: destroy_instance as ArcanaCabiDestroyInstanceFn,
        reserved0: ptr::null(),
        reserved1: ptr::null(),
    },
    imports: BINDING_IMPORTS.as_ptr(),
    import_count: BINDING_IMPORTS.len(),
    callbacks: BINDING_CALLBACKS.as_ptr(),
    callback_count: BINDING_CALLBACKS.len(),
    register_callback: register_callback as ArcanaCabiBindingRegisterCallbackFn,
    unregister_callback: unregister_callback as ArcanaCabiBindingUnregisterCallbackFn,
    last_error_alloc: arcana_cabi_last_error_alloc_v1 as ArcanaCabiLastErrorAllocFn,
    owned_bytes_free: arcana_cabi_owned_bytes_free_v1 as ArcanaCabiOwnedBytesFreeFn,
    owned_str_free: arcana_cabi_owned_str_free_v1 as ArcanaCabiOwnedStrFreeFn,
    reserved0: ptr::null(),
    reserved1: ptr::null(),
};

static PRODUCT_API: ArcanaCabiProductApiV1 = ArcanaCabiProductApiV1 {
    descriptor_size: std::mem::size_of::<ArcanaCabiProductApiV1>(),
    package_name: PACKAGE_NAME.as_ptr().cast(),
    product_name: PRODUCT_NAME.as_ptr().cast(),
    role: ROLE_NAME.as_ptr().cast(),
    contract_id: CONTRACT_ID.as_ptr().cast(),
    contract_version: ARCANA_CABI_CONTRACT_VERSION_V1,
    role_ops: &BINDING_OPS as *const ArcanaCabiBindingOpsV1 as *const c_void,
    reserved0: ptr::null(),
    reserved1: ptr::null(),
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn debug_font_trace(message: impl AsRef<str>) {
    if std::env::var_os("ARCANA_WINAPI_DEBUG_FONTS").is_some() {
        eprintln!("[arcana-winapi/fonts] {}", message.as_ref());
    }
}

fn check_hresult(hr: i32, context: &str) -> Result<(), String> {
    if hr < 0 {
        return Err(format!("{context} failed with HRESULT 0x{:08X}", hr as u32));
    }
    Ok(())
}

fn set_last_error(err: impl Into<String>) {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = err.into().into_bytes();
    });
}

fn owned_bytes_parts(bytes: Vec<u8>) -> (*mut u8, usize) {
    if bytes.is_empty() {
        return (ptr::null_mut(), 0);
    }
    let len = bytes.len();
    let ptr = Box::into_raw(bytes.into_boxed_slice()) as *mut u8;
    (ptr, len)
}

fn binding_opaque(handle: u64) -> ArcanaCabiBindingValueV1 {
    ArcanaCabiBindingValueV1 {
        tag: ArcanaCabiBindingValueTag::Opaque as u32,
        reserved0: 0,
        reserved1: 0,
        payload: ArcanaCabiBindingPayloadV1 { opaque_value: handle },
    }
}

fn binding_int(value: i64) -> ArcanaCabiBindingValueV1 {
    ArcanaCabiBindingValueV1 {
        tag: ArcanaCabiBindingValueTag::Int as u32,
        reserved0: 0,
        reserved1: 0,
        payload: ArcanaCabiBindingPayloadV1 { int_value: value },
    }
}

fn binding_bool(value: bool) -> ArcanaCabiBindingValueV1 {
    ArcanaCabiBindingValueV1 {
        tag: ArcanaCabiBindingValueTag::Bool as u32,
        reserved0: 0,
        reserved1: 0,
        payload: ArcanaCabiBindingPayloadV1 {
            bool_value: if value { 1 } else { 0 },
        },
    }
}

fn binding_owned_str(text: String) -> ArcanaCabiBindingValueV1 {
    let (ptr, len) = owned_bytes_parts(text.into_bytes());
    ArcanaCabiBindingValueV1 {
        tag: ArcanaCabiBindingValueTag::Str as u32,
        reserved0: 0,
        reserved1: 0,
        payload: ArcanaCabiBindingPayloadV1 {
            owned_str_value: ArcanaOwnedStr { ptr, len },
        },
    }
}

fn binding_unit() -> ArcanaCabiBindingValueV1 {
    ArcanaCabiBindingValueV1::default()
}

fn binding_tag(value: &ArcanaCabiBindingValueV1) -> Result<ArcanaCabiBindingValueTag, String> {
    match value.tag {
        tag if tag == ArcanaCabiBindingValueTag::Int as u32 => Ok(ArcanaCabiBindingValueTag::Int),
        tag if tag == ArcanaCabiBindingValueTag::Bool as u32 => Ok(ArcanaCabiBindingValueTag::Bool),
        tag if tag == ArcanaCabiBindingValueTag::Str as u32 => Ok(ArcanaCabiBindingValueTag::Str),
        tag if tag == ArcanaCabiBindingValueTag::Opaque as u32 => {
            Ok(ArcanaCabiBindingValueTag::Opaque)
        }
        tag if tag == ArcanaCabiBindingValueTag::Unit as u32 => {
            Ok(ArcanaCabiBindingValueTag::Unit)
        }
        tag if tag == ArcanaCabiBindingValueTag::Bytes as u32 => {
            Ok(ArcanaCabiBindingValueTag::Bytes)
        }
        other => Err(format!("unsupported binding value tag `{other}`")),
    }
}

fn read_utf8_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<String, String> {
    if binding_tag(value)? != ArcanaCabiBindingValueTag::Str {
        return Err(format!("binding arg `{name}` expected `Str`"));
    }
    let view = unsafe { value.payload.str_value };
    if view.ptr.is_null() {
        if view.len == 0 {
            return Ok(String::new());
        }
        return Err(format!(
            "binding arg `{name}` carried a null string pointer with non-zero length"
        ));
    }
    let bytes = unsafe { std::slice::from_raw_parts(view.ptr, view.len) };
    std::str::from_utf8(bytes)
        .map(|text| text.to_string())
        .map_err(|err| format!("binding arg `{name}` is not utf-8: {err}"))
}

fn read_int_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<i64, String> {
    if binding_tag(value)? != ArcanaCabiBindingValueTag::Int {
        return Err(format!("binding arg `{name}` expected `Int`"));
    }
    Ok(unsafe { value.payload.int_value })
}

fn read_opaque_arg(value: &ArcanaCabiBindingValueV1, name: &str) -> Result<u64, String> {
    if binding_tag(value)? != ArcanaCabiBindingValueTag::Opaque {
        return Err(format!("binding arg `{name}` expected opaque handle"));
    }
    Ok(unsafe { value.payload.opaque_value })
}

fn require_arg_count(actual: usize, expected: usize, name: &str) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "binding import `{name}` expected {expected} args, got {actual}"
        ));
    }
    Ok(())
}

unsafe fn instance_ptr(instance: *mut c_void) -> Result<*mut BindingInstance, String> {
    if instance.is_null() {
        return Err("binding instance must not be null".to_string());
    }
    Ok(instance.cast())
}

fn current_module_handle_for_address(address: *const c_void) -> Result<HMODULE, String> {
    let mut module: HMODULE = null_mut();
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            address.cast::<u16>(),
            &mut module,
        )
    };
    if ok == 0 {
        return Err(format!(
            "GetModuleHandleExW failed with Win32 error {}",
            unsafe { GetLastError() }
        ));
    }
    Ok(module)
}

fn module_path_text(module: HMODULE) -> Result<String, String> {
    let mut buffer = vec![0u16; 260];
    loop {
        let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) };
        if len == 0 {
            return Err(format!(
                "GetModuleFileNameW failed with Win32 error {}",
                unsafe { GetLastError() }
            ));
        }
        let len = len as usize;
        if len + 1 < buffer.len() {
            buffer.truncate(len);
            return String::from_utf16(&buffer)
                .map_err(|err| format!("module path is not utf-16: {err}"));
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

fn localized_string(strings: *mut IDWriteLocalizedStrings) -> Result<String, String> {
    if strings.is_null() {
        return Ok(String::new());
    }
    if unsafe { (*strings).GetCount() } == 0 {
        return Ok(String::new());
    }
    let mut index = 0u32;
    let mut exists: BOOL = FALSE;
    let en_us = wide("en-us");
    let hr = unsafe { (*strings).FindLocaleName(en_us.as_ptr(), &mut index, &mut exists) };
    if hr < 0 || exists == FALSE {
        let en = wide("en");
        let hr = unsafe { (*strings).FindLocaleName(en.as_ptr(), &mut index, &mut exists) };
        if hr < 0 || exists == FALSE {
            index = 0;
        }
    }
    let mut length = 0u32;
    check_hresult(
        unsafe { (*strings).GetStringLength(index, &mut length) },
        "IDWriteLocalizedStrings::GetStringLength",
    )?;
    let mut buffer = vec![0u16; length as usize + 1];
    check_hresult(
        unsafe { (*strings).GetString(index, buffer.as_mut_ptr(), buffer.len() as u32) },
        "IDWriteLocalizedStrings::GetString",
    )?;
    buffer.truncate(length as usize);
    String::from_utf16(&buffer).map_err(|err| format!("localized string is not utf-16: {err}"))
}

fn synthesize_full_name(family_name: &str, face_name: &str) -> String {
    let trimmed_face = face_name.trim();
    if trimmed_face.is_empty() || trimmed_face.eq_ignore_ascii_case("regular") {
        family_name.to_string()
    } else {
        format!("{family_name} {trimmed_face}")
    }
}

fn synthesize_postscript_name(family_name: &str, face_name: &str) -> String {
    synthesize_full_name(family_name, face_name)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn file_path_from_font(font: *mut IDWriteFont) -> Result<String, String> {
    let mut face = ComPtr::<IDWriteFontFace>::null();
    check_hresult(
        unsafe { (*font).CreateFontFace(face.as_mut_ptr()) },
        "IDWriteFont::CreateFontFace",
    )?;
    let mut count = 0u32;
    check_hresult(
        unsafe { (*face.as_ptr()).GetFiles(&mut count, null_mut()) },
        "IDWriteFontFace::GetFiles(count)",
    )?;
    if count == 0 {
        return Ok(String::new());
    }
    let mut files = vec![null_mut::<IDWriteFontFile>(); count as usize];
    check_hresult(
        unsafe { (*face.as_ptr()).GetFiles(&mut count, files.as_mut_ptr()) },
        "IDWriteFontFace::GetFiles(files)",
    )?;
    let mut path = String::new();
    for file_ptr in files {
        let file = ComPtr(file_ptr);
        if file.is_null() {
            continue;
        }
        let mut key_ptr: *const winapi_c_void = null();
        let mut key_len: u32 = 0;
        check_hresult(
            unsafe { (*file.as_ptr()).GetReferenceKey(&mut key_ptr, &mut key_len) },
            "IDWriteFontFile::GetReferenceKey",
        )?;
        let mut loader = ComPtr::<IDWriteFontFileLoader>::null();
        check_hresult(
            unsafe { (*file.as_ptr()).GetLoader(loader.as_mut_ptr()) },
            "IDWriteFontFile::GetLoader",
        )?;
        let mut local_loader = ComPtr::<IDWriteLocalFontFileLoader>::null();
        let iid = <IDWriteLocalFontFileLoader as Interface>::uuidof();
        let hr = unsafe {
            (*(loader.as_ptr() as *mut IUnknown)).QueryInterface(
                &iid,
                local_loader.as_mut_ptr() as *mut *mut _ as *mut *mut winapi_c_void,
            )
        };
        if hr < 0 || local_loader.is_null() {
            continue;
        }
        let mut length = 0u32;
        check_hresult(
            unsafe {
                (*local_loader.as_ptr()).GetFilePathLengthFromKey(key_ptr, key_len, &mut length)
            },
            "IDWriteLocalFontFileLoader::GetFilePathLengthFromKey",
        )?;
        let mut buffer = vec![0u16; length as usize + 1];
        check_hresult(
            unsafe {
                (*local_loader.as_ptr()).GetFilePathFromKey(
                    key_ptr,
                    key_len,
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                )
            },
            "IDWriteLocalFontFileLoader::GetFilePathFromKey",
        )?;
        buffer.truncate(length as usize);
        path = String::from_utf16(&buffer)
            .map_err(|err| format!("font file path is not utf-16: {err}"))?;
        if !path.is_empty() {
            break;
        }
    }
    Ok(path)
}

fn build_system_font_catalog() -> Result<SystemFontCatalogState, String> {
    debug_font_trace("create factory");
    let mut factory_unknown: *mut IUnknown = null_mut();
    let iid = <IDWriteFactory as Interface>::uuidof();
    check_hresult(
        unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &iid, &mut factory_unknown) },
        "DWriteCreateFactory",
    )?;
    let factory = ComPtr(factory_unknown as *mut IDWriteFactory);
    debug_font_trace("get system font collection");
    let mut collection = ComPtr::<IDWriteFontCollection>::null();
    check_hresult(
        unsafe { (*factory.as_ptr()).GetSystemFontCollection(collection.as_mut_ptr(), FALSE) },
        "IDWriteFactory::GetSystemFontCollection",
    )?;
    let family_count = unsafe { (*collection.as_ptr()).GetFontFamilyCount() };
    debug_font_trace(format!("family_count={family_count}"));
    let mut entries = Vec::new();
    for family_index in 0..family_count {
        debug_font_trace(format!("family[{family_index}]"));
        let mut family = ComPtr::<IDWriteFontFamily>::null();
        check_hresult(
            unsafe { (*collection.as_ptr()).GetFontFamily(family_index, family.as_mut_ptr()) },
            "IDWriteFontCollection::GetFontFamily",
        )?;
        let mut family_names = ComPtr::<IDWriteLocalizedStrings>::null();
        check_hresult(
            unsafe { (*family.as_ptr()).GetFamilyNames(family_names.as_mut_ptr()) },
            "IDWriteFontFamily::GetFamilyNames",
        )?;
        let family_name = localized_string(family_names.as_ptr())?;
        let font_list = family.as_ptr() as *mut winapi::um::dwrite::IDWriteFontList;
        let font_count = unsafe { (*font_list).GetFontCount() };
        debug_font_trace(format!("family[{family_index}] font_count={font_count}"));
        for font_index in 0..font_count {
            debug_font_trace(format!("family[{family_index}] font[{font_index}]"));
            let mut font = ComPtr::<IDWriteFont>::null();
            check_hresult(
                unsafe { (*font_list).GetFont(font_index, font.as_mut_ptr()) },
                "IDWriteFontList::GetFont",
            )?;
            debug_font_trace(format!("family[{family_index}] font[{font_index}] face_names"));
            let mut face_names = ComPtr::<IDWriteLocalizedStrings>::null();
            check_hresult(
                unsafe { (*font.as_ptr()).GetFaceNames(face_names.as_mut_ptr()) },
                "IDWriteFont::GetFaceNames",
            )?;
            let face_name = localized_string(face_names.as_ptr())?;
            debug_font_trace(format!("family[{family_index}] font[{font_index}] path"));
            let path = file_path_from_font(font.as_ptr())?;
            debug_font_trace(format!("family[{family_index}] font[{font_index}] names"));
            let full_name = synthesize_full_name(&family_name, &face_name);
            let postscript_name = synthesize_postscript_name(&family_name, &face_name);
            entries.push(SystemFontEntry {
                family_name: family_name.clone(),
                face_name,
                full_name,
                postscript_name,
                path,
            });
        }
    }
    Ok(SystemFontCatalogState { entries })
}

fn window_class_name() -> &'static [u16] {
    static NAME: OnceLock<Vec<u16>> = OnceLock::new();
    NAME.get_or_init(|| wide("ArcanaWinApiHiddenWindow"))
}

fn register_hidden_window_class() -> Result<(), String> {
    let module = current_module_handle_for_address(arcana_winapi_hidden_window_proc as *const c_void)?;
    let class_name = window_class_name();
    let class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(arcana_winapi_hidden_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: module,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };
    let atom = unsafe { RegisterClassW(&class) };
    if atom == 0 {
        let err = unsafe { GetLastError() };
        if err != ERROR_CLASS_ALREADY_EXISTS {
            return Err(format!("RegisterClassW failed with Win32 error {err}"));
        }
    }
    Ok(())
}

fn catalog_ref(handle: u64) -> Result<&'static SystemFontCatalogState, String> {
    let ptr = handle as usize as *const SystemFontCatalogState;
    if ptr.is_null() {
        return Err("SystemFontCatalog handle must not be null".to_string());
    }
    Ok(unsafe { &*ptr })
}

fn destroy_catalog_handle(handle: u64) {
    let ptr = handle as usize as *mut SystemFontCatalogState;
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

fn catalog_entry(handle: u64, index: i64) -> Result<&'static SystemFontEntry, String> {
    if index < 0 {
        return Err(format!("font catalog index `{index}` must be >= 0"));
    }
    let catalog = catalog_ref(handle)?;
    catalog
        .entries
        .get(index as usize)
        .ok_or_else(|| format!("font catalog index `{index}` is out of range"))
}

fn invoke_window_callback(
    instance: *mut BindingInstance,
    window: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> Result<LRESULT, String> {
    let callback = unsafe {
        (*instance)
            .callbacks_by_name
            .get("window_proc")
            .copied()
    }
    .ok_or_else(|| "no registered `window_proc` callback is active".to_string())?;
    let args = [
        binding_opaque(window as usize as u64),
        binding_int(message as i64),
        binding_int(wparam as i64),
        binding_int(lparam as i64),
    ];
    let mut out = ArcanaCabiBindingValueV1::default();
    let ok = unsafe { (callback.callback)(callback.user_data, args.as_ptr(), args.len(), &mut out) };
    if ok == 0 {
        return Err("registered `window_proc` callback returned failure".to_string());
    }
    if binding_tag(&out)? != ArcanaCabiBindingValueTag::Int {
        return Err("registered `window_proc` callback returned a non-Int result".to_string());
    }
    let result = unsafe { out.payload.int_value };
    unsafe {
        (*instance).last_callback_code = result;
    }
    Ok(result as LRESULT)
}

unsafe extern "system" fn arcana_winapi_hidden_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam as *const CREATESTRUCTW;
        if !create.is_null() {
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize);
            }
        }
        return 1;
    }
    let instance = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut BindingInstance };
    if message == WM_APP + 1 && !instance.is_null() {
        match invoke_window_callback(instance, hwnd, message, wparam, lparam) {
            Ok(result) => return result,
            Err(err) => {
                set_last_error(err);
                return 0;
            }
        }
    }
    if message == WM_CLOSE {
        unsafe {
            DestroyWindow(hwnd);
        }
        return 0;
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

unsafe extern "system" fn create_instance() -> *mut c_void {
    let mut instance = BindingInstance {
        next_handle: 1,
        ..Default::default()
    };
    let hr = unsafe { CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED as u32) };
    if hr >= 0 {
        instance.com_initialized = true;
    } else if hr as u32 != RPC_E_CHANGED_MODE as u32 {
        set_last_error(format!("CoInitializeEx failed with HRESULT 0x{:08X}", hr as u32));
        return null_mut();
    }
    Box::into_raw(Box::new(instance)).cast()
}

unsafe extern "system" fn destroy_instance(instance: *mut c_void) {
    if instance.is_null() {
        return;
    }
    let instance = unsafe { Box::from_raw(instance as *mut BindingInstance) };
    if instance.com_initialized {
        unsafe {
            CoUninitialize();
        }
    }
}

unsafe extern "system" fn register_callback(
    instance: *mut c_void,
    callback_name: *const c_char,
    callback: ArcanaCabiBindingCallbackFn,
    user_data: *mut c_void,
    out_handle: *mut u64,
) -> i32 {
    let result = (|| -> Result<(), String> {
        let instance = unsafe { instance_ptr(instance)? };
        if callback_name.is_null() {
            return Err("binding callback name must not be null".to_string());
        }
        let name = unsafe { CStr::from_ptr(callback_name) }
            .to_str()
            .map_err(|err| format!("binding callback name is not utf-8: {err}"))?
            .to_string();
        let handle = unsafe {
            let current = (*instance).next_handle;
            (*instance).next_handle += 1;
            (*instance).callbacks_by_name.insert(
                name.clone(),
                RegisteredCallback { callback, user_data },
            );
            (*instance).handles_to_name.insert(current, name);
            current
        };
        if !out_handle.is_null() {
            unsafe {
                *out_handle = handle;
            }
        }
        Ok(())
    })();
    match result {
        Ok(()) => 1,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

unsafe extern "system" fn unregister_callback(instance: *mut c_void, handle: u64) -> i32 {
    let result = (|| -> Result<(), String> {
        let instance = unsafe { instance_ptr(instance)? };
        let name = unsafe { (*instance).handles_to_name.remove(&handle) }
            .ok_or_else(|| format!("binding callback handle `{handle}` is not active"))?;
        unsafe {
            (*instance).callbacks_by_name.remove(&name);
        }
        Ok(())
    })();
    match result {
        Ok(()) => 1,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

unsafe fn run_import(
    import_name: &str,
    instance: *mut c_void,
    args: *const ArcanaCabiBindingValueV1,
    arg_count: usize,
    out_result: *mut ArcanaCabiBindingValueV1,
    handler: impl FnOnce(*mut BindingInstance, &[ArcanaCabiBindingValueV1]) -> Result<ArcanaCabiBindingValueV1, String>,
) -> i32 {
    let result = (|| -> Result<(), String> {
        let instance = unsafe { instance_ptr(instance)? };
        if out_result.is_null() {
            return Err(format!("binding import `{import_name}` requires non-null out_result"));
        }
        if args.is_null() && arg_count != 0 {
            return Err(format!(
                "binding import `{import_name}` received null args with non-zero count"
            ));
        }
        let args = if arg_count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(args, arg_count) }
        };
        let value = handler(instance, args)?;
        unsafe {
            *out_result = value;
        }
        Ok(())
    })();
    match result {
        Ok(()) => 1,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

macro_rules! binding_import {
    ($symbol:ident, $name:literal, $body:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "system" fn $symbol(
            instance: *mut c_void,
            args: *const ArcanaCabiBindingValueV1,
            arg_count: usize,
            _out_write_backs: *mut ArcanaCabiBindingValueV1,
            out_result: *mut ArcanaCabiBindingValueV1,
        ) -> i32 {
            unsafe { run_import($name, instance, args, arg_count, out_result, $body) }
        }
    };
}

binding_import!(
    arcana_winapi_import_foundation_current_module,
    "foundation.current_module",
    |_instance, args| {
        require_arg_count(args.len(), 0, "foundation.current_module")?;
        Ok(binding_opaque(
            current_module_handle_for_address(
                arcana_winapi_import_foundation_current_module as *const c_void,
            )? as usize as u64,
        ))
    }
);
binding_import!(
    arcana_winapi_import_foundation_module_is_null,
    "foundation.module_is_null",
    |_instance, args| {
        require_arg_count(args.len(), 1, "foundation.module_is_null")?;
        Ok(binding_bool(read_opaque_arg(&args[0], "module")? == 0))
    }
);
binding_import!(
    arcana_winapi_import_foundation_module_path,
    "foundation.module_path",
    |_instance, args| {
        require_arg_count(args.len(), 1, "foundation.module_path")?;
        let module = read_opaque_arg(&args[0], "module")?;
        Ok(binding_owned_str(module_path_text(module as usize as HMODULE)?))
    }
);
binding_import!(
    arcana_winapi_import_foundation_utf16_len,
    "foundation.utf16_len",
    |_instance, args| {
        require_arg_count(args.len(), 1, "foundation.utf16_len")?;
        let text = read_utf8_arg(&args[0], "text")?;
        Ok(binding_int(text.encode_utf16().count() as i64))
    }
);
binding_import!(
    arcana_winapi_import_foundation_fail_sample,
    "foundation.fail_sample",
    |_instance, args| {
        require_arg_count(args.len(), 1, "foundation.fail_sample")?;
        let message = read_utf8_arg(&args[0], "message")?;
        Err(format!("arcana_winapi sample failure: {message}"))
    }
);
binding_import!(
    arcana_winapi_import_fonts_system_font_catalog,
    "fonts.system_font_catalog",
    |_instance, args| {
        require_arg_count(args.len(), 0, "fonts.system_font_catalog")?;
        let catalog = Box::new(build_system_font_catalog()?);
        Ok(binding_opaque(Box::into_raw(catalog) as usize as u64))
    }
);
binding_import!(
    arcana_winapi_import_fonts_catalog_count,
    "fonts.catalog_count",
    |_instance, args| {
        require_arg_count(args.len(), 1, "fonts.catalog_count")?;
        let handle = read_opaque_arg(&args[0], "catalog")?;
        Ok(binding_int(catalog_ref(handle)?.entries.len() as i64))
    }
);

macro_rules! catalog_string_import {
    ($symbol:ident, $name:literal, $field:ident) => {
        binding_import!($symbol, $name, |_instance, args| {
            require_arg_count(args.len(), 2, $name)?;
            let handle = read_opaque_arg(&args[0], "catalog")?;
            let index = read_int_arg(&args[1], "index")?;
            Ok(binding_owned_str(catalog_entry(handle, index)?.$field.clone()))
        });
    };
}

catalog_string_import!(
    arcana_winapi_import_fonts_catalog_family_name,
    "fonts.catalog_family_name",
    family_name
);
catalog_string_import!(
    arcana_winapi_import_fonts_catalog_face_name,
    "fonts.catalog_face_name",
    face_name
);
catalog_string_import!(
    arcana_winapi_import_fonts_catalog_full_name,
    "fonts.catalog_full_name",
    full_name
);
catalog_string_import!(
    arcana_winapi_import_fonts_catalog_postscript_name,
    "fonts.catalog_postscript_name",
    postscript_name
);
catalog_string_import!(
    arcana_winapi_import_fonts_catalog_path,
    "fonts.catalog_path",
    path
);

binding_import!(
    arcana_winapi_import_fonts_catalog_destroy,
    "fonts.catalog_destroy",
    |_instance, args| {
        require_arg_count(args.len(), 1, "fonts.catalog_destroy")?;
        destroy_catalog_handle(read_opaque_arg(&args[0], "catalog")?);
        Ok(binding_unit())
    }
);
binding_import!(
    arcana_winapi_import_windows_create_hidden_window,
    "windows.create_hidden_window",
    |instance, args| {
        require_arg_count(args.len(), 0, "windows.create_hidden_window")?;
        register_hidden_window_class()?;
        let module =
            current_module_handle_for_address(arcana_winapi_hidden_window_proc as *const c_void)?;
        let class_name = window_class_name();
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            null_mut(),
            null_mut(),
            module,
            instance.cast(),
        );
        if hwnd.is_null() {
            return Err(format!(
                "CreateWindowExW failed with Win32 error {}",
                GetLastError()
            ));
        }
        Ok(binding_opaque(hwnd as usize as u64))
    }
);
binding_import!(
    arcana_winapi_import_windows_post_ping,
    "windows.post_ping",
    |_instance, args| {
        require_arg_count(args.len(), 2, "windows.post_ping")?;
        let hwnd = read_opaque_arg(&args[0], "window")? as HWND;
        let code = read_int_arg(&args[1], "code")?;
        let ok = PostMessageW(hwnd, WM_APP + 1, code as usize, 0);
        if ok == 0 {
            return Err(format!(
                "PostMessageW failed with Win32 error {}",
                GetLastError()
            ));
        }
        Ok(binding_unit())
    }
);
binding_import!(
    arcana_winapi_import_windows_pump_messages,
    "windows.pump_messages",
    |_instance, args| {
        require_arg_count(args.len(), 0, "windows.pump_messages")?;
        let mut msg = std::mem::zeroed::<MSG>();
        let mut count = 0i64;
        while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
            count += 1;
        }
        Ok(binding_int(count))
    }
);
binding_import!(
    arcana_winapi_import_windows_take_last_callback_code,
    "windows.take_last_callback_code",
    |instance, args| {
        require_arg_count(args.len(), 0, "windows.take_last_callback_code")?;
        let value = (*instance).last_callback_code;
        (*instance).last_callback_code = 0;
        Ok(binding_int(value))
    }
);
binding_import!(
    arcana_winapi_import_windows_destroy_hidden_window,
    "windows.destroy_hidden_window",
    |_instance, args| {
        require_arg_count(args.len(), 1, "windows.destroy_hidden_window")?;
        let hwnd = read_opaque_arg(&args[0], "window")? as HWND;
        let ok = DestroyWindow(hwnd);
        if ok == 0 {
            return Err(format!(
                "DestroyWindow failed with Win32 error {}",
                GetLastError()
            ));
        }
        Ok(binding_unit())
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::atomic::{AtomicI64, Ordering};

    static LAST_WPARAM: AtomicI64 = AtomicI64::new(0);

    unsafe extern "system" fn test_window_proc(
        _user_data: *mut c_void,
        args: *const ArcanaCabiBindingValueV1,
        arg_count: usize,
        out_result: *mut ArcanaCabiBindingValueV1,
    ) -> i32 {
        if args.is_null() || out_result.is_null() || arg_count != 4 {
            return 0;
        }
        let args = unsafe { std::slice::from_raw_parts(args, arg_count) };
        let wparam = match read_int_arg(&args[2], "wparam") {
            Ok(value) => value,
            Err(_) => return 0,
        };
        LAST_WPARAM.store(wparam, Ordering::SeqCst);
        unsafe {
            *out_result = binding_int(wparam + 1);
        }
        1
    }

    #[test]
    fn foundation_imports_work_without_runtime() {
        unsafe {
            let instance = create_instance();
            assert!(!instance.is_null(), "binding instance should create");

            let mut module = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_foundation_current_module(
                    instance,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    &mut module,
                ),
                1
            );
            assert_eq!(binding_tag(&module).unwrap(), ArcanaCabiBindingValueTag::Opaque);

            let mut is_null = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_foundation_module_is_null(
                    instance,
                    &module,
                    1,
                    ptr::null_mut(),
                    &mut is_null,
                ),
                1
            );
            assert_eq!(is_null.payload.bool_value, 0);

            destroy_instance(instance);
        }
    }

    #[test]
    fn hidden_window_round_trip_works_without_runtime() {
        unsafe {
            let instance = create_instance();
            assert!(!instance.is_null(), "binding instance should create");

            let callback_name = CString::new("window_proc").expect("callback name");
            let mut callback_handle = 0u64;
            assert_eq!(
                register_callback(
                    instance,
                    callback_name.as_ptr(),
                    test_window_proc,
                    ptr::null_mut(),
                    &mut callback_handle,
                ),
                1
            );
            assert_ne!(callback_handle, 0, "callback registration should return a handle");

            let mut window = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_windows_create_hidden_window(
                    instance,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    &mut window,
                ),
                1
            );

            let args = [window, binding_int(41)];
            let mut post = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_windows_post_ping(
                    instance,
                    args.as_ptr(),
                    args.len(),
                    ptr::null_mut(),
                    &mut post,
                ),
                1
            );

            let mut pumped = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_windows_pump_messages(
                    instance,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    &mut pumped,
                ),
                1
            );
            assert!(pumped.payload.int_value > 0);

            let mut callback_code = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_windows_take_last_callback_code(
                    instance,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    &mut callback_code,
                ),
                1
            );
            assert_eq!(callback_code.payload.int_value, 42);
            assert_eq!(LAST_WPARAM.load(Ordering::SeqCst), 41);

            let mut destroy = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_windows_destroy_hidden_window(
                    instance,
                    &window,
                    1,
                    ptr::null_mut(),
                    &mut destroy,
                ),
                1
            );
            assert_eq!(unregister_callback(instance, callback_handle), 1);
            destroy_instance(instance);
        }
    }

    #[test]
    fn system_font_catalog_import_works_without_runtime() {
        unsafe {
            let instance = create_instance();
            assert!(!instance.is_null(), "binding instance should create");

            let mut catalog = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_fonts_system_font_catalog(
                    instance,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    &mut catalog,
                ),
                1
            );
            assert_eq!(binding_tag(&catalog).unwrap(), ArcanaCabiBindingValueTag::Opaque);

            let mut count = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_fonts_catalog_count(
                    instance,
                    &catalog,
                    1,
                    ptr::null_mut(),
                    &mut count,
                ),
                1
            );
            assert!(count.payload.int_value > 0, "system font catalog should not be empty");

            let args = [catalog, binding_int(0)];
            let mut family = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_fonts_catalog_family_name(
                    instance,
                    args.as_ptr(),
                    args.len(),
                    ptr::null_mut(),
                    &mut family,
                ),
                1
            );
            assert_eq!(binding_tag(&family).unwrap(), ArcanaCabiBindingValueTag::Str);

            let mut destroy = ArcanaCabiBindingValueV1::default();
            assert_eq!(
                arcana_winapi_import_fonts_catalog_destroy(
                    instance,
                    &catalog,
                    1,
                    ptr::null_mut(),
                    &mut destroy,
                ),
                1
            );

            destroy_instance(instance);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn arcana_cabi_get_product_api_v1() -> *const ArcanaCabiProductApiV1 {
    &PRODUCT_API
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn arcana_cabi_last_error_alloc_v1(out_len: *mut usize) -> *mut u8 {
    let bytes = LAST_ERROR.with(|slot| slot.borrow().clone());
    let (ptr, len) = owned_bytes_parts(bytes);
    if !out_len.is_null() {
        unsafe {
            *out_len = len;
        }
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn arcana_cabi_owned_bytes_free_v1(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    unsafe {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn arcana_cabi_owned_str_free_v1(ptr: *mut u8, len: usize) {
    unsafe {
        arcana_cabi_owned_bytes_free_v1(ptr, len);
    }
}
