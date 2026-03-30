use std::path::{Path, PathBuf};

use crate::emit::AotRuntimeBinding;
use crate::native_abi::{NativeAbiParam, NativeAbiType, NativeExport};
use crate::native_layout::{NativeAbiRole, NativeLayoutCatalog};
use crate::native_lowering::{
    NativeCleanupAction, NativeDirectBlock, NativeDirectExpr, NativeDirectIntBinaryOp,
    NativeDirectIntCompareOp, NativeDirectRoutine, NativeDirectStmt, NativeExportLowering,
    NativeLaunchLowering, NativeLoweringPlan, NativeRoutineLowering,
};
use crate::native_manifest::{
    native_bundle_manifest_file_name, render_native_bundle_manifest,
    render_windows_dll_definition_file, windows_dll_definition_file_name,
    windows_dll_header_file_name,
};
use crate::native_plan::{NativeLaunchPlan, NativePackagePlan};
use arcana_cabi::{
    ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL, ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL,
    ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL, ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL,
    ArcanaCabiPassMode, render_c_descriptor_type_defs,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustNativeProject {
    pub project_dir: PathBuf,
    pub output_name: String,
    pub artifact_text: String,
    pub support_files: Vec<(String, Vec<u8>)>,
    pub cargo_toml: String,
    pub build_rs: Option<String>,
    pub lib_rs: Option<String>,
    pub main_rs: Option<String>,
}

pub fn generate_windows_exe_project(
    project_dir: &Path,
    plan: &NativePackagePlan,
    lowering: &NativeLoweringPlan,
) -> Result<RustNativeProject, String> {
    let output_stem = native_output_stem(&plan.root_artifact_file_name);
    let NativeLaunchPlan::Executable { main_routine_key } = &plan.launch else {
        panic!("windows exe project generation requires an executable native plan");
    };
    let NativeLaunchLowering::Executable {
        main_routine_key: lowered_routine_key,
        lowering: main_lowering,
    } = &lowering.launch
    else {
        return Err(
            "windows exe project generation requires an executable lowering plan".to_string(),
        );
    };
    if lowered_routine_key != main_routine_key {
        return Err(format!(
            "native lowering main routine `{lowered_routine_key}` did not match native plan main routine `{main_routine_key}`"
        ));
    }
    Ok(RustNativeProject {
        project_dir: project_dir.to_path_buf(),
        output_name: plan.root_artifact_file_name.clone(),
        artifact_text: plan.artifact_text.clone(),
        support_files: vec![(
            native_bundle_manifest_file_name(&plan.root_artifact_file_name),
            render_native_bundle_manifest(plan)?.into_bytes(),
        )],
        cargo_toml: render_exe_cargo_toml(
            plan.artifact.package_name.as_str(),
            &output_stem,
            plan.runtime_binding,
        ),
        build_rs: Some(render_native_build_rs(None)),
        lib_rs: None,
        main_rs: Some(render_exe_main_rs(
            main_routine_key,
            main_lowering,
            lowering,
        )),
    })
}

pub fn generate_windows_dll_project(
    project_dir: &Path,
    plan: &NativePackagePlan,
    lowering: &NativeLoweringPlan,
) -> Result<RustNativeProject, String> {
    let NativeLaunchPlan::DynamicLibrary { exports } = &plan.launch else {
        return Err(
            "windows dll project generation requires a dynamic-library native plan".to_string(),
        );
    };
    let NativeLaunchLowering::DynamicLibrary {
        exports: lowered_exports,
    } = &lowering.launch
    else {
        return Err(
            "windows dll project generation requires a dynamic-library lowering plan".to_string(),
        );
    };
    if lowered_exports.len() != exports.len() {
        return Err(format!(
            "native lowering export count {} did not match native plan export count {}",
            lowered_exports.len(),
            exports.len()
        ));
    }
    let layout = NativeLayoutCatalog::from_exports(exports);
    let output_stem = native_output_stem(&plan.root_artifact_file_name);
    let definition_text = render_windows_dll_definition_file(plan)?;
    Ok(RustNativeProject {
        project_dir: project_dir.to_path_buf(),
        output_name: plan.root_artifact_file_name.clone(),
        artifact_text: plan.artifact_text.clone(),
        support_files: vec![
            (
                windows_dll_header_file_name(&plan.root_artifact_file_name),
                render_dll_header(exports, &layout).into_bytes(),
            ),
            (
                windows_dll_definition_file_name(&plan.root_artifact_file_name),
                definition_text.clone().into_bytes(),
            ),
            (
                native_bundle_manifest_file_name(&plan.root_artifact_file_name),
                render_native_bundle_manifest(plan)?.into_bytes(),
            ),
        ],
        cargo_toml: render_dll_cargo_toml(
            plan.artifact.package_name.as_str(),
            &output_stem,
            plan.runtime_binding,
        ),
        build_rs: Some(render_native_build_rs(Some(&definition_text))),
        lib_rs: Some(render_dll_lib_rs(plan, lowered_exports, &layout, lowering)),
        main_rs: None,
    })
}

fn render_exe_cargo_toml(
    crate_name: &str,
    output_stem: &str,
    runtime_binding: AotRuntimeBinding,
) -> String {
    let repo_root = repo_root();
    let runtime_dependency = render_runtime_dependency(runtime_binding, &repo_root);
    let cabi_dependency = escape_toml(
        &repo_root
            .join("crates")
            .join("arcana-cabi")
            .display()
            .to_string(),
    );
    format!(
        concat!(
            "[package]\n",
            "name = \"{}\"\n",
            "version = \"0.0.0\"\n",
            "edition = \"2024\"\n\n",
            "[[bin]]\n",
            "name = \"{}\"\n",
            "path = \"src/main.rs\"\n\n",
            "[dependencies]\n",
            "arcana_cabi = {{ package = \"arcana-cabi\", path = \"{}\" }}\n",
            "arcana_runtime = {{ {} }}\n",
            "\n[build-dependencies]\n",
            "arcana-aot = {{ path = \"{}\" }}\n",
            "arcana_runtime = {{ {} }}\n",
            "\n[workspace]\n",
        ),
        sanitize_crate_name(crate_name),
        escape_toml(output_stem),
        cabi_dependency,
        runtime_dependency,
        escape_toml(
            &repo_root
                .join("crates")
                .join("arcana-aot")
                .display()
                .to_string()
        ),
        runtime_dependency,
    )
}

fn render_dll_cargo_toml(
    crate_name: &str,
    output_stem: &str,
    runtime_binding: AotRuntimeBinding,
) -> String {
    let repo_root = repo_root();
    let runtime_dependency = render_runtime_dependency(runtime_binding, &repo_root);
    let cabi_dependency = escape_toml(
        &repo_root
            .join("crates")
            .join("arcana-cabi")
            .display()
            .to_string(),
    );
    format!(
        concat!(
            "[package]\n",
            "name = \"{}\"\n",
            "version = \"0.0.0\"\n",
            "edition = \"2024\"\n\n",
            "[lib]\n",
            "name = \"{}\"\n",
            "crate-type = [\"cdylib\"]\n\n",
            "[dependencies]\n",
            "arcana_cabi = {{ package = \"arcana-cabi\", path = \"{}\" }}\n",
            "arcana_runtime = {{ {} }}\n",
            "\n[build-dependencies]\n",
            "arcana-aot = {{ path = \"{}\" }}\n",
            "arcana_runtime = {{ {} }}\n",
            "\n[workspace]\n",
        ),
        sanitize_crate_name(crate_name),
        escape_toml(output_stem),
        cabi_dependency,
        runtime_dependency,
        escape_toml(
            &repo_root
                .join("crates")
                .join("arcana-aot")
                .display()
                .to_string()
        ),
        runtime_dependency,
    )
}

fn render_runtime_dependency(runtime_binding: AotRuntimeBinding, repo_root: &Path) -> String {
    let _ = runtime_binding;
    let dependency_path = repo_root.join("crates").join("arcana-runtime");
    let package_name = "arcana-runtime";
    format!(
        "package = \"{}\", path = \"{}\"",
        escape_toml(package_name),
        escape_toml(&dependency_path.display().to_string())
    )
}

fn render_exe_main_rs(
    main_routine_key: &str,
    lowering: &NativeRoutineLowering,
    plan: &NativeLoweringPlan,
) -> String {
    match lowering {
        NativeRoutineLowering::Direct { routine_key } => format!(
            concat!(
                "#![windows_subsystem = \"windows\"]\n\n",
                "use arcana_runtime::{{RuntimeAbiValue, RuntimeExecutionState, RuntimePackagePlan, activate_current_bundle_native_products, current_process_runtime_host, parse_runtime_package_image}};\n\n",
                "static PACKAGE_IMAGE_TEXT: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/runtime-package.json\"));\n\n",
                "static MAIN_ROUTINE_KEY: &str = __ARCANA_MAIN_ROUTINE_KEY__;\n\n",
                "{}",
                "fn load_plan() -> Result<RuntimePackagePlan, String> {{\n",
                "    parse_runtime_package_image(PACKAGE_IMAGE_TEXT)\n",
                "}}\n\n",
                "fn main() {{\n",
                "    let code = match run() {{\n",
                "        Ok(code) => code,\n",
                "        Err(err) => {{\n",
                "            eprintln!(\"{{err}}\");\n",
                "            1\n",
                "        }}\n",
                "    }};\n",
                "    std::process::exit(code);\n",
                "}}\n\n",
                "fn run() -> Result<i32, String> {{\n",
                "    let mut native_products = activate_current_bundle_native_products()?;\n",
                "    if let Some(code) = native_products.run_child_entrypoint(PACKAGE_IMAGE_TEXT, MAIN_ROUTINE_KEY)? {{\n",
                "        return Ok(code);\n",
                "    }}\n",
                "    let plan = load_plan()?;\n",
                "    let mut host = current_process_runtime_host()?;\n",
                "    let mut state = RuntimeExecutionState::default();\n",
                "    let result = {}?;\n",
                "    match result {{\n",
                "        RuntimeAbiValue::Int(code) => i32::try_from(code)\n",
                "            .map_err(|_| format!(\"main routine `{}` returned Int outside i32 range: {{code}}\")),\n",
                "        RuntimeAbiValue::Unit => Ok(0),\n",
                "        _ => Err(\"direct native main must return Int or Unit\".to_string()),\n",
                "    }}\n",
                "}}\n",
            ),
            render_direct_routine_helpers(plan),
            render_direct_routine_call_from_values(
                routine_key,
                &[],
                &[],
                Some("&plan"),
                Some("&mut state"),
                Some("host.as_mut()"),
            ),
            main_routine_key,
        )
        .replace(
            "__ARCANA_MAIN_ROUTINE_KEY__",
            &format!("{main_routine_key:?}"),
        ),
        NativeRoutineLowering::RuntimeDispatch => {
            let template = concat!(
                "#![windows_subsystem = \"windows\"]\n\n",
                "use arcana_runtime::{RuntimeAbiValue, RuntimeExecutionState, RuntimePackagePlan, execute_current_bundle_entrypoint};\n\n",
                "static PACKAGE_IMAGE_TEXT: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/runtime-package.json\"));\n\n",
                "static MAIN_ROUTINE_KEY: &str = __ARCANA_MAIN_ROUTINE_KEY__;\n\n",
                "fn main() {\n",
                "    let code = match run() {\n",
                "        Ok(code) => code,\n",
                "        Err(err) => {\n",
                "            eprintln!(\"{err}\");\n",
                "            1\n",
                "        }\n",
                "    };\n",
                "    std::process::exit(code);\n",
                "}\n\n",
                "fn run() -> Result<i32, String> {\n",
                "    execute_current_bundle_entrypoint(PACKAGE_IMAGE_TEXT, MAIN_ROUTINE_KEY)\n",
                "}\n\n",
            );
            let mut out = String::new();
            out.push_str(&render_direct_routine_helpers(plan));
            out.push_str(&template.replace(
                "__ARCANA_MAIN_ROUTINE_KEY__",
                &format!("{main_routine_key:?}"),
            ));
            out
        }
    }
}

fn render_native_build_rs(dll_definition_text: Option<&str>) -> String {
    let mut out = String::from(concat!(
        "use std::fs;\n",
        "use std::path::PathBuf;\n\n",
        "use arcana_aot::parse_package_artifact;\n",
        "use arcana_runtime::{plan_from_artifact, render_runtime_package_image};\n\n",
        "fn main() {\n",
        "    if let Err(err) = build_runtime_package_image() {\n",
        "        panic!(\"failed to build runtime package image: {err}\");\n",
        "    }\n",
        "}\n\n",
        "fn build_runtime_package_image() -> Result<(), String> {\n",
        "    let manifest_dir = std::env::var(\"CARGO_MANIFEST_DIR\")\n",
        "        .map(PathBuf::from)\n",
        "        .map_err(|e| format!(\"missing CARGO_MANIFEST_DIR: {e}\"))?;\n",
        "    let out_dir = std::env::var(\"OUT_DIR\")\n",
        "        .map(PathBuf::from)\n",
        "        .map_err(|e| format!(\"missing OUT_DIR: {e}\"))?;\n",
        "    let artifact_path = manifest_dir.join(\"src\").join(\"artifact.toml\");\n",
        "    let artifact_text = fs::read_to_string(&artifact_path)\n",
        "        .map_err(|e| format!(\"failed to read {}: {e}\", artifact_path.display()))?;\n",
        "    let artifact = parse_package_artifact(&artifact_text)?;\n",
        "    let plan = plan_from_artifact(&artifact)?;\n",
        "    let image_text = render_runtime_package_image(&plan)?;\n",
        "    fs::write(out_dir.join(\"runtime-package.json\"), image_text)\n",
        "        .map_err(|e| format!(\"failed to write runtime package image: {e}\"))?;\n",
    ));
    if dll_definition_text.is_none() {
        out.push_str(concat!(
            "    if std::env::var(\"CARGO_CFG_TARGET_OS\").as_deref() == Ok(\"windows\") {\n",
            "        if std::env::var(\"CARGO_CFG_TARGET_ENV\").as_deref() == Ok(\"msvc\") {\n",
            "            println!(\"cargo:rustc-link-arg=/STACK:8388608\");\n",
            "        } else if std::env::var(\"CARGO_CFG_TARGET_ENV\").as_deref() == Ok(\"gnu\") {\n",
            "            println!(\"cargo:rustc-link-arg=-Wl,--stack,8388608\");\n",
            "        }\n",
            "    }\n",
        ));
    }
    if let Some(definition_text) = dll_definition_text {
        out.push_str("    let definition_text: &str = ");
        out.push_str(&format!("{definition_text:?}"));
        out.push_str(";\n");
        out.push_str(concat!(
            "    let definition_path = out_dir.join(\"arcana-exports.def\");\n",
            "    fs::write(&definition_path, definition_text)\n",
            "        .map_err(|e| format!(\"failed to write dll definition file: {e}\"))?;\n",
            "    if std::env::var(\"CARGO_CFG_TARGET_ENV\").as_deref() == Ok(\"msvc\") {\n",
            "        println!(\"cargo:rustc-cdylib-link-arg=/DEF:{}\", definition_path.display());\n",
            "    }\n",
        ));
    }
    out.push_str(concat!(
        "    println!(\"cargo:rerun-if-changed={}\", artifact_path.display());\n",
        "    Ok(())\n",
        "}\n",
    ));
    out
}

fn render_dll_lib_rs(
    plan: &NativePackagePlan,
    exports: &[NativeExportLowering],
    layout: &NativeLayoutCatalog,
    lowering: &NativeLoweringPlan,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        concat!(
            "#![allow(dead_code, unused_imports)]\n\n",
            "use std::cell::RefCell;\n",
            "use std::ffi::{{c_char, c_void}};\n",
            "use std::ptr;\n",
            "use std::sync::OnceLock;\n\n",
            "use arcana_cabi::{{ArcanaBytesView, ArcanaCabiExportEntryV1, ArcanaCabiExportOpsV1, ArcanaCabiExportParamV1, ArcanaCabiProductApiV1, ArcanaOwnedBytes, ArcanaOwnedStr, ArcanaStrView}};\n",
            "use arcana_runtime::{{RuntimeAbiExportOutcome, RuntimeAbiValue, RuntimeAbiWriteBack, RuntimeExecutionState, RuntimePackagePlan, current_process_runtime_host, execute_exported_abi_routine, parse_runtime_package_image}};\n\n",
            "thread_local! {{\n",
            "    static LAST_ERROR: RefCell<Vec<u8>> = const {{ RefCell::new(Vec::new()) }};\n",
            "}}\n\n",
            "static PLAN: OnceLock<Result<RuntimePackagePlan, String>> = OnceLock::new();\n",
            "static PACKAGE_IMAGE_TEXT: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/runtime-package.json\"));\n\n",
            "#[unsafe(no_mangle)]\n",
            "pub extern \"system\" fn {last_error_symbol}(out_len: *mut usize) -> *mut u8 {{\n",
            "    let bytes = LAST_ERROR.with(|slot| slot.borrow().clone());\n",
            "    write_allocated_bytes(bytes, out_len)\n",
            "}}\n\n",
            "#[unsafe(no_mangle)]\n",
            "pub extern \"system\" fn {owned_bytes_free_symbol}(ptr: *mut u8, len: usize) {{\n",
            "    if ptr.is_null() || len == 0 {{ return; }}\n",
            "    unsafe {{ drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len))); }}\n",
            "}}\n\n",
            "#[unsafe(no_mangle)]\n",
            "pub extern \"system\" fn {owned_str_free_symbol}(ptr: *mut u8, len: usize) {{\n",
            "    {owned_bytes_free_symbol}(ptr, len);\n",
            "}}\n\n",
            "fn load_plan() -> Result<&'static RuntimePackagePlan, String> {{\n",
            "    match PLAN.get_or_init(|| {{\n",
            "        parse_runtime_package_image(PACKAGE_IMAGE_TEXT)\n",
            "    }}) {{\n",
            "        Ok(plan) => Ok(plan),\n",
            "        Err(err) => Err(err.clone()),\n",
            "    }}\n",
            "}}\n\n",
            "fn default_host() -> Result<Box<dyn arcana_runtime::RuntimeHost>, String> {{\n",
            "    current_process_runtime_host()\n",
            "}}\n\n",
            "fn set_last_error(err: String) {{\n",
            "    LAST_ERROR.with(|slot| *slot.borrow_mut() = err.into_bytes());\n",
            "}}\n\n",
            "fn bytes_from_view(view: ArcanaBytesView, context: &str) -> Result<Vec<u8>, String> {{\n",
            "    if view.ptr.is_null() {{\n",
            "        if view.len == 0 {{\n",
            "            return Ok(Vec::new());\n",
            "        }}\n",
            "        return Err(format!(\"{{context}} received null bytes pointer with len {{}}\", view.len));\n",
            "    }}\n",
            "    Ok(unsafe {{ std::slice::from_raw_parts(view.ptr, view.len) }}.to_vec())\n",
            "}}\n\n",
            "fn string_from_view(view: ArcanaStrView, context: &str) -> Result<String, String> {{\n",
            "    let bytes = bytes_from_view(ArcanaBytesView {{ ptr: view.ptr, len: view.len }}, context)?;\n",
            "    String::from_utf8(bytes).map_err(|e| format!(\"{{context}} received invalid utf8: {{e}}\"))\n",
            "}}\n\n",
            "fn allocated_bytes_parts(bytes: Vec<u8>) -> (*mut u8, usize) {{\n",
            "    if bytes.is_empty() {{\n",
            "        return (ptr::null_mut(), 0);\n",
            "    }}\n",
            "    let len = bytes.len();\n",
            "    (Box::into_raw(bytes.into_boxed_slice()) as *mut u8, len)\n",
            "}}\n\n",
            "fn owned_bytes_from_vec(bytes: Vec<u8>) -> ArcanaOwnedBytes {{\n",
            "    let (ptr, len) = allocated_bytes_parts(bytes);\n",
            "    ArcanaOwnedBytes {{ ptr, len }}\n",
            "}}\n\n",
            "fn owned_str_from_string(text: String) -> ArcanaOwnedStr {{\n",
            "    let (ptr, len) = allocated_bytes_parts(text.into_bytes());\n",
            "    ArcanaOwnedStr {{ ptr, len }}\n",
            "}}\n\n",
            "fn write_allocated_bytes(bytes: Vec<u8>, out_len: *mut usize) -> *mut u8 {{\n",
            "    let (ptr, len) = allocated_bytes_parts(bytes);\n",
            "    if !out_len.is_null() {{ unsafe {{ *out_len = len; }} }}\n",
            "    ptr\n",
            "}}\n\n"
        ),
        last_error_symbol = ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL,
        owned_bytes_free_symbol = ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL,
        owned_str_free_symbol = ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL,
    ));
    out.push_str(&layout.render_rust_pair_type_defs());
    out.push_str(&render_direct_routine_helpers(lowering));

    for export in exports {
        out.push_str(&render_export_fn(export, layout));
    }
    out.push_str(&render_export_descriptor(plan, exports));
    out
}

fn render_export_fn(export: &NativeExportLowering, layout: &NativeLayoutCatalog) -> String {
    let api = &export.export;
    let needs_write_backs = api
        .params
        .iter()
        .any(|param| matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack));
    let mut params = api
        .params
        .iter()
        .map(|param| {
            format!(
                "{}: {}",
                param.name,
                layout.rust_type_ref(&param.input_type, NativeAbiRole::Param)
            )
        })
        .collect::<Vec<_>>();
    for param in &api.params {
        if matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack)
            && !matches!(param.input_type, NativeAbiType::Unit)
        {
            params.push(format!(
                "out_{}: *mut {}",
                param.name,
                layout.rust_type_ref(&param.input_type, NativeAbiRole::Return)
            ));
        }
    }
    if let Some(out_ty) = layout.rust_out_type_ref(&api.return_type) {
        params.push(format!("out_result: *mut {out_ty}"));
    }
    let mut body = String::new();
    for param in &api.params {
        body.push_str(&render_native_param_binding(api, param, layout));
    }
    match &export.lowering {
        NativeRoutineLowering::Direct { routine_key } => {
            body.push_str("    let plan = match load_plan() {\n");
            body.push_str("        Ok(plan) => plan,\n");
            body.push_str("        Err(err) => { set_last_error(err); return 0; }\n");
            body.push_str("    };\n");
            body.push_str("    let mut host = match default_host() {\n");
            body.push_str("        Ok(host) => host,\n");
            body.push_str("        Err(err) => { set_last_error(err); return 0; }\n");
            body.push_str("    };\n");
            body.push_str("    let mut state = RuntimeExecutionState::default();\n");
            body.push_str("    let result = match ");
            body.push_str(&render_direct_routine_call_from_values(
                routine_key,
                &api.params,
                &api.params
                    .iter()
                    .map(|param| format!("{}_value", param.name))
                    .collect::<Vec<_>>(),
                Some("plan"),
                Some("&mut state"),
                Some("host.as_mut()"),
            ));
            body.push_str(" {\n");
            body.push_str("        Ok(value) => value,\n");
            body.push_str("        Err(err) => { set_last_error(err); return 0; }\n");
            body.push_str("    };\n");
        }
        NativeRoutineLowering::RuntimeDispatch => {
            body.push_str("    let plan = match load_plan() {\n");
            body.push_str("        Ok(plan) => plan,\n");
            body.push_str("        Err(err) => { set_last_error(err); return 0; }\n");
            body.push_str("    };\n");
            body.push_str("    let mut host = match default_host() {\n");
            body.push_str("        Ok(host) => host,\n");
            body.push_str("        Err(err) => { set_last_error(err); return 0; }\n");
            body.push_str("    };\n");
            body.push_str("    let outcome = match execute_exported_abi_routine(plan, ");
            body.push_str(&format!("{:?}", api.routine_key));
            body.push_str(", vec![");
            body.push_str(
                &api.params
                    .iter()
                    .map(|param| format!("{}_value", param.name))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            body.push_str("], host.as_mut()) {\n");
            body.push_str("        Ok(value) => value,\n");
            body.push_str("        Err(err) => { set_last_error(err); return 0; }\n");
            body.push_str("    };\n");
            body.push_str("    let result = outcome.result;\n");
            if needs_write_backs {
                body.push_str("    let write_backs = outcome.write_backs;\n");
            }
        }
    }
    for (index, param) in api.params.iter().enumerate() {
        if !matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack)
            || matches!(param.input_type, NativeAbiType::Unit)
        {
            continue;
        }
        let out_name = format!("out_{}", param.name);
        let source_value_name = format!("{}_write_back", param.name);
        let out_value_name = format!("{}_out_value", param.name);
        body.push_str(&format!(
            "    if {out_name}.is_null() {{ set_last_error(\"null {out_name}\".to_string()); return 0; }}\n"
        ));
        match &export.lowering {
            NativeRoutineLowering::Direct { .. } => {
                body.push_str(&format!(
                    "    let {source_value_name} = {}_value.clone();\n",
                    param.name
                ));
            }
            NativeRoutineLowering::RuntimeDispatch => {
                body.push_str(&format!(
                    "    let {source_value_name} = match write_backs.iter().find(|write_back| write_back.index == {index}) {{\n"
                ));
                body.push_str("        Some(write_back) => write_back.value.clone(),\n");
                body.push_str(&format!(
                    "        None => {{ set_last_error(\"missing write-back `{}`\".to_string()); return 0; }}\n",
                    param.name
                ));
                body.push_str("    };\n");
            }
        }
        body.push_str(&format!(
            "    let {out_value_name}: {};\n",
            layout.rust_type_ref(&param.input_type, NativeAbiRole::Return)
        ));
        body.push_str(&render_store_runtime_abi_value(
            &param.input_type,
            &source_value_name,
            &out_value_name,
            layout,
        ));
        body.push_str(&format!(
            "    unsafe {{ *{out_name} = {out_value_name}; }}\n"
        ));
    }
    if let Some(out_ty) = layout.rust_out_type_ref(&api.return_type) {
        body.push_str("    if out_result.is_null() { set_last_error(\"null out_result\".to_string()); return 0; }\n");
        body.push_str(&format!("    let out_value: {out_ty};\n"));
        body.push_str(&render_store_runtime_abi_value(
            &api.return_type,
            "result",
            "out_value",
            layout,
        ));
        body.push_str("    unsafe { *out_result = out_value; }\n");
    } else {
        body.push_str("    let RuntimeAbiValue::Unit = result else { set_last_error(\"abi return type mismatch\".to_string()); return 0; };\n");
    }
    body.push_str("    1\n");

    format!(
        "#[unsafe(no_mangle)]\npub extern \"system\" fn {}({}) -> u8 {{\n{}}}\n\n",
        api.export_name,
        params.join(", "),
        body
    )
}

fn render_dll_header(exports: &[NativeExport], layout: &NativeLayoutCatalog) -> String {
    let mut out = String::from(concat!(
        "#ifndef ARCANA_EXPORTS_H\n#define ARCANA_EXPORTS_H\n\n",
        "#include <stdint.h>\n#include <stddef.h>\n\n",
    ));
    out.push_str(&layout.render_c_type_defs());
    out.push_str(&render_c_descriptor_type_defs());
    out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");
    out.push_str(&format!(
        "uint8_t* {}(size_t* out_len);\n",
        ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL
    ));
    out.push_str(&format!(
        "void {}(uint8_t* ptr, size_t len);\n",
        ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL
    ));
    out.push_str(&format!(
        "void {}(uint8_t* ptr, size_t len);\n",
        ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL
    ));
    out.push_str(&format!(
        "const ArcanaCabiProductApiV1* {}(void);\n\n",
        ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL
    ));
    for export in exports {
        let mut params = export
            .params
            .iter()
            .map(|param| {
                format!(
                    "{} {}",
                    layout.c_type_ref(&param.input_type, NativeAbiRole::Param),
                    param.name
                )
            })
            .collect::<Vec<_>>();
        for param in &export.params {
            if matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack)
                && !matches!(param.input_type, NativeAbiType::Unit)
            {
                params.push(format!(
                    "{}* out_{}",
                    layout.c_type_ref(&param.input_type, NativeAbiRole::Return),
                    param.name
                ));
            }
        }
        if let Some(c_out_ty) = layout.c_out_type_ref(&export.return_type) {
            params.push(format!("{c_out_ty}* out_result"));
        }
        out.push_str(&format!(
            "uint8_t {}({});\n",
            export.export_name,
            params.join(", ")
        ));
    }
    out.push_str("\n#ifdef __cplusplus\n}\n#endif\n\n#endif\n");
    out
}

fn render_export_descriptor(plan: &NativePackagePlan, exports: &[NativeExportLowering]) -> String {
    let Some(product) = &plan.native_product else {
        return String::new();
    };
    let mut out = String::new();
    for (export_index, export) in exports.iter().enumerate() {
        out.push_str(&format!(
            "static ARCANA_EXPORT_{export_index}_PARAMS: [ArcanaCabiExportParamV1; {}] = [\n",
            export.export.params.len()
        ));
        for param in &export.export.params {
            out.push_str(&format!(
                concat!(
                    "    ArcanaCabiExportParamV1 {{\n",
                    "        name: {name},\n",
                    "        source_mode: {source_mode},\n",
                    "        pass_mode: {pass_mode},\n",
                    "        input_type: {input_type},\n",
                    "        write_back_type: {write_back_type},\n",
                    "    }},\n"
                ),
                name = render_c_string_ptr(&param.name),
                source_mode = render_c_string_ptr(param.source_mode.as_str()),
                pass_mode = render_c_string_ptr(param.pass_mode.as_str()),
                input_type = render_c_string_ptr(&param.input_type.render()),
                write_back_type = param
                    .write_back_type
                    .as_ref()
                    .map(|ty| render_c_string_ptr(&ty.render()))
                    .unwrap_or_else(|| "ptr::null()".to_string()),
            ));
        }
        out.push_str("];\n\n");
    }
    out.push_str(&format!(
        "static ARCANA_EXPORT_ENTRIES: [ArcanaCabiExportEntryV1; {}] = [\n",
        exports.len()
    ));
    for (export_index, export) in exports.iter().enumerate() {
        out.push_str(&format!(
            concat!(
                "    ArcanaCabiExportEntryV1 {{\n",
                "        export_name: {export_name},\n",
                "        routine_key: {routine_key},\n",
                "        symbol_name: {symbol_name},\n",
                "        return_type: {return_type},\n",
                "        params: ARCANA_EXPORT_{export_index}_PARAMS.as_ptr(),\n",
                "        param_count: {param_count},\n",
                "    }},\n"
            ),
            export_name = render_c_string_ptr(&export.export.export_name),
            routine_key = render_c_string_ptr(&export.export.routine_key),
            symbol_name = render_c_string_ptr(&export.export.symbol_name),
            return_type = render_c_string_ptr(&export.export.return_type.render()),
            export_index = export_index,
            param_count = export.export.params.len(),
        ));
    }
    out.push_str("];\n\n");
    out.push_str(&format!(
        concat!(
            "static ARCANA_EXPORT_OPS: ArcanaCabiExportOpsV1 = ArcanaCabiExportOpsV1 {{\n",
            "    ops_size: std::mem::size_of::<ArcanaCabiExportOpsV1>(),\n",
            "    exports: ARCANA_EXPORT_ENTRIES.as_ptr(),\n",
            "    export_count: {},\n",
            "    last_error_alloc: {},\n",
            "    owned_bytes_free: {},\n",
            "    owned_str_free: {},\n",
            "    reserved0: ptr::null(),\n",
            "    reserved1: ptr::null(),\n",
            "}};\n\n",
            "static ARCANA_PRODUCT_API: ArcanaCabiProductApiV1 = ArcanaCabiProductApiV1 {{\n",
            "    descriptor_size: std::mem::size_of::<ArcanaCabiProductApiV1>(),\n",
            "    package_name: {},\n",
            "    product_name: {},\n",
            "    role: {},\n",
            "    contract_id: {},\n",
            "    contract_version: {},\n",
            "    role_ops: &ARCANA_EXPORT_OPS as *const _ as *const c_void,\n",
            "    reserved0: ptr::null(),\n",
            "    reserved1: ptr::null(),\n",
            "}};\n\n",
            "#[unsafe(no_mangle)]\n",
            "pub extern \"system\" fn {}() -> *const ArcanaCabiProductApiV1 {{\n",
            "    &ARCANA_PRODUCT_API\n",
            "}}\n\n"
        ),
        exports.len(),
        ARCANA_CABI_LAST_ERROR_ALLOC_V1_SYMBOL,
        ARCANA_CABI_OWNED_BYTES_FREE_V1_SYMBOL,
        ARCANA_CABI_OWNED_STR_FREE_V1_SYMBOL,
        render_c_string_ptr(&plan.artifact.package_name),
        render_c_string_ptr(&product.name),
        render_c_string_ptr(product.role.as_str()),
        render_c_string_ptr(&product.contract_id),
        product.contract_version,
        ARCANA_CABI_GET_PRODUCT_API_V1_SYMBOL,
    ));
    out
}

fn render_c_string_ptr(text: &str) -> String {
    format!(
        "b\"{}\\0\".as_ptr() as *const c_char",
        escape_rust_byte_string(text)
    )
}

fn escape_rust_byte_string(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            0x20..=0x7e => out.push(char::from(byte)),
            _ => out.push_str(&format!("\\x{byte:02x}")),
        }
    }
    out
}

fn render_native_param_binding(
    export: &NativeExport,
    param: &NativeAbiParam,
    _layout: &NativeLayoutCatalog,
) -> String {
    let context = format!("{} parameter `{}`", export.export_name, param.name);
    let mut_kw = if matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack) {
        "mut "
    } else {
        ""
    };
    format!(
        "    let {mut_kw}{name}_value = match {expr} {{\n        Ok(value) => value,\n        Err(err) => {{ set_last_error(err); return 0; }}\n    }};\n",
        mut_kw = mut_kw,
        name = param.name,
        expr = render_runtime_abi_expr_from_native(&param.input_type, &param.name, &context),
    )
}

fn render_runtime_abi_expr_from_native(ty: &NativeAbiType, expr: &str, context: &str) -> String {
    match ty {
        NativeAbiType::Int => format!("Ok(RuntimeAbiValue::Int({expr}))"),
        NativeAbiType::Bool => format!("Ok(RuntimeAbiValue::Bool({expr} != 0))"),
        NativeAbiType::Str => {
            format!("string_from_view({expr}, {context:?}).map(RuntimeAbiValue::Str)")
        }
        NativeAbiType::Bytes => {
            format!("bytes_from_view({expr}, {context:?}).map(RuntimeAbiValue::Bytes)")
        }
        NativeAbiType::Pair(left, right) => format!(
            concat!(
                "{{\n",
                "        match {} {{\n",
                "            Ok(left) => match {} {{\n",
                "                Ok(right) => Ok(RuntimeAbiValue::Pair(Box::new(left), Box::new(right))),\n",
                "                Err(err) => Err(err),\n",
                "            }},\n",
                "            Err(err) => Err(err),\n",
                "        }}\n",
                "    }}"
            ),
            render_runtime_abi_expr_from_native(
                left,
                &format!("{expr}.left"),
                &format!("{context} left")
            ),
            render_runtime_abi_expr_from_native(
                right,
                &format!("{expr}.right"),
                &format!("{context} right")
            ),
        ),
        NativeAbiType::Unit => "Ok(RuntimeAbiValue::Unit)".to_string(),
    }
}

fn render_lowered_runtime_abi_expr(expr: &NativeDirectExpr, indent_level: usize) -> String {
    match expr {
        NativeDirectExpr::Int(value) => {
            format!("Result::<RuntimeAbiValue, String>::Ok(RuntimeAbiValue::Int({value}))")
        }
        NativeDirectExpr::Bool(value) => {
            format!("Result::<RuntimeAbiValue, String>::Ok(RuntimeAbiValue::Bool({value}))")
        }
        NativeDirectExpr::Unit => {
            "Result::<RuntimeAbiValue, String>::Ok(RuntimeAbiValue::Unit)".to_string()
        }
        NativeDirectExpr::Str(value) => {
            format!(
                "Result::<RuntimeAbiValue, String>::Ok(RuntimeAbiValue::Str({value:?}.to_string()))"
            )
        }
        NativeDirectExpr::Bytes(bytes) => {
            format!("Result::<RuntimeAbiValue, String>::Ok(RuntimeAbiValue::Bytes(vec!{bytes:?}))")
        }
        NativeDirectExpr::Binding(name) => {
            format!("Result::<RuntimeAbiValue, String>::Ok({name}_value.clone())")
        }
        NativeDirectExpr::IntBinary { op, left, right } => {
            render_direct_int_binary_expr(*op, left, right, indent_level)
        }
        NativeDirectExpr::IntCompare { op, left, right } => {
            render_direct_int_compare_expr(*op, left, right, indent_level)
        }
        NativeDirectExpr::Call {
            routine_key,
            params,
            args,
        } => render_direct_routine_call_expr(routine_key, params, args, indent_level),
        NativeDirectExpr::Pair { left, right } => format!(
            concat!(
                "{{\n",
                "        match {} {{\n",
                "            Ok(left) => match {} {{\n",
                "                Ok(right) => Ok(RuntimeAbiValue::Pair(Box::new(left), Box::new(right))),\n",
                "                Err(err) => Err(err),\n",
                "            }},\n",
                "            Err(err) => Err(err),\n",
                "        }}\n",
                "    }}"
            ),
            render_lowered_runtime_abi_expr(left, indent_level + 2),
            render_lowered_runtime_abi_expr(right, indent_level + 2),
        ),
        NativeDirectExpr::StringConcat { left, right } => format!(
            concat!(
                "{{\n",
                "        match {} {{\n",
                "            Ok(RuntimeAbiValue::Str(mut left)) => match {} {{\n",
                "                Ok(RuntimeAbiValue::Str(right)) => {{\n",
                "                    left.push_str(&right);\n",
                "                    Ok(RuntimeAbiValue::Str(left))\n",
                "                }}\n",
                "                Ok(_) => Err(\"direct string concat expected Str rhs\".to_string()),\n",
                "                Err(err) => Err(err),\n",
                "            }},\n",
                "            Ok(_) => Err(\"direct string concat expected Str lhs\".to_string()),\n",
                "            Err(err) => Err(err),\n",
                "        }}\n",
                "    }}"
            ),
            render_lowered_runtime_abi_expr(left, indent_level + 2),
            render_lowered_runtime_abi_expr(right, indent_level + 2),
        ),
        NativeDirectExpr::If {
            condition,
            then_block,
            else_block,
        } => {
            let base_indent = indent(indent_level);
            let match_indent = indent(indent_level + 1);
            let arm_indent = indent(indent_level + 2);
            format!(
                concat!(
                    "{{\n",
                    "{match_indent}match {condition} {{\n",
                    "{arm_indent}Ok(RuntimeAbiValue::Bool(true)) => {then_block},\n",
                    "{arm_indent}Ok(RuntimeAbiValue::Bool(false)) => {else_block},\n",
                    "{arm_indent}Ok(_) => Err(\"direct if expected Bool condition\".to_string()),\n",
                    "{arm_indent}Err(err) => Err(err),\n",
                    "{match_indent}}}\n",
                    "{base_indent}}}"
                ),
                match_indent = match_indent,
                arm_indent = arm_indent,
                base_indent = base_indent,
                condition = render_lowered_runtime_abi_expr(condition, indent_level + 2),
                then_block = render_direct_block_expr(then_block, indent_level + 2),
                else_block = render_direct_block_expr(else_block, indent_level + 2),
            )
        }
    }
}

fn render_direct_routine_helpers(plan: &NativeLoweringPlan) -> String {
    let mut out = String::new();
    for routine in &plan.direct_routines {
        out.push_str(&render_direct_routine_helper(routine));
    }
    out
}

fn render_direct_routine_helper(routine: &NativeDirectRoutine) -> String {
    let params = routine
        .params
        .iter()
        .map(|param| match param.pass_mode {
            ArcanaCabiPassMode::In => format!("{}_value: RuntimeAbiValue", param.name),
            ArcanaCabiPassMode::InWithWriteBack => {
                format!("{}_slot: &mut RuntimeAbiValue", param.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let params = if params.is_empty() {
        "plan: &RuntimePackagePlan, state: &mut RuntimeExecutionState, host: &mut dyn arcana_runtime::RuntimeHost".to_string()
    } else {
        format!(
            "plan: &RuntimePackagePlan, state: &mut RuntimeExecutionState, host: &mut dyn arcana_runtime::RuntimeHost, {params}"
        )
    };
    let mut prelude = String::new();
    prelude.push_str("    let _ = (&*plan, &mut *state, &mut *host);\n");
    for param in &routine.params {
        if matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack) {
            prelude.push_str(&format!(
                "    let mut {}_value = {}_slot.clone();\n",
                param.name, param.name
            ));
        }
    }
    let body = render_direct_block_body(&routine.body, 2);
    let mut write_backs = String::new();
    for param in &routine.params {
        if matches!(param.pass_mode, ArcanaCabiPassMode::InWithWriteBack) {
            write_backs.push_str(&format!(
                "    *{}_slot = {}_value;\n",
                param.name, param.name
            ));
        }
    }
    format!(
        concat!(
            "fn {}({}) -> Result<RuntimeAbiValue, String> {{\n",
            "{}",
            "    let result = (|| -> Result<RuntimeAbiValue, String> {{\n",
            "{}",
            "    }})();\n",
            "{}",
            "    result\n",
            "}}\n\n"
        ),
        direct_routine_fn_name(&routine.routine_key),
        params,
        prelude,
        body,
        write_backs,
    )
}

fn render_direct_block_body(block: &NativeDirectBlock, indent_level: usize) -> String {
    let mut out = String::new();
    for stmt in &block.statements {
        out.push_str(&render_direct_routine_stmt(stmt, indent_level));
    }
    out.push_str(&indent(indent_level));
    out.push_str(&render_lowered_runtime_abi_expr(
        &block.return_expr,
        indent_level,
    ));
    out.push('\n');
    out
}

fn render_direct_block_expr(block: &NativeDirectBlock, indent_level: usize) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&render_direct_block_body(block, indent_level + 1));
    out.push_str(&indent(indent_level));
    out.push('}');
    out
}

fn render_direct_routine_stmt(stmt: &NativeDirectStmt, indent_level: usize) -> String {
    match stmt {
        NativeDirectStmt::Let {
            mutable,
            name,
            value,
        } => {
            let mut_kw = if *mutable { "mut " } else { "" };
            format!(
                concat!(
                    "{indent}let {mut_kw}{}_value = match {} {{\n",
                    "{match_indent}Ok(value) => value,\n",
                    "{match_indent}Err(err) => return Err(err),\n",
                    "{indent}}};\n"
                ),
                name,
                render_lowered_runtime_abi_expr(value, indent_level + 1),
                indent = indent(indent_level),
                match_indent = indent(indent_level + 1),
                mut_kw = mut_kw,
            )
        }
        NativeDirectStmt::Expr { value } => format!(
            concat!(
                "{indent}match {} {{\n",
                "{match_indent}Ok(_) => {{}}\n",
                "{match_indent}Err(err) => return Err(err),\n",
                "{indent}}};\n"
            ),
            render_lowered_runtime_abi_expr(value, indent_level + 1),
            indent = indent(indent_level),
            match_indent = indent(indent_level + 1),
        ),
        NativeDirectStmt::Cleanup { action } => render_cleanup_action_stmt(action, indent_level),
        NativeDirectStmt::Assign { name, value } => format!(
            concat!(
                "{indent}{}_value = match {} {{\n",
                "{match_indent}Ok(value) => value,\n",
                "{match_indent}Err(err) => return Err(err),\n",
                "{indent}}};\n"
            ),
            name,
            render_lowered_runtime_abi_expr(value, indent_level + 1),
            indent = indent(indent_level),
            match_indent = indent(indent_level + 1),
        ),
        NativeDirectStmt::Return { value } => format!(
            "{}return {};\n",
            indent(indent_level),
            render_lowered_runtime_abi_expr(value, indent_level)
        ),
        NativeDirectStmt::If {
            condition,
            then_body,
            else_body,
        } => format!(
            concat!(
                "{indent}match {} {{\n",
                "{arm_indent}Ok(RuntimeAbiValue::Bool(true)) => {{\n",
                "{}",
                "{arm_indent}}},\n",
                "{arm_indent}Ok(RuntimeAbiValue::Bool(false)) => {{\n",
                "{}",
                "{arm_indent}}},\n",
                "{arm_indent}Ok(_) => return Err(\"direct if expected Bool condition\".to_string()),\n",
                "{arm_indent}Err(err) => return Err(err),\n",
                "{indent}}}\n"
            ),
            render_lowered_runtime_abi_expr(condition, indent_level + 1),
            render_direct_stmt_body(then_body, indent_level + 2),
            render_direct_stmt_body(else_body, indent_level + 2),
            indent = indent(indent_level),
            arm_indent = indent(indent_level + 1),
        ),
        NativeDirectStmt::While { condition, body } => format!(
            concat!(
                "{indent}loop {{\n",
                "{match_indent}match {} {{\n",
                "{arm_indent}Ok(RuntimeAbiValue::Bool(true)) => {{\n",
                "{}",
                "{arm_indent}}},\n",
                "{arm_indent}Ok(RuntimeAbiValue::Bool(false)) => break,\n",
                "{arm_indent}Ok(_) => return Err(\"direct while expected Bool condition\".to_string()),\n",
                "{arm_indent}Err(err) => return Err(err),\n",
                "{match_indent}}}\n",
                "{indent}}}\n"
            ),
            render_lowered_runtime_abi_expr(condition, indent_level + 2),
            render_direct_stmt_body(body, indent_level + 2),
            indent = indent(indent_level),
            match_indent = indent(indent_level + 1),
            arm_indent = indent(indent_level + 2),
        ),
        NativeDirectStmt::Break => format!("{}break;\n", indent(indent_level)),
        NativeDirectStmt::Continue => format!("{}continue;\n", indent(indent_level)),
    }
}

fn render_direct_stmt_body(statements: &[NativeDirectStmt], indent_level: usize) -> String {
    statements
        .iter()
        .map(|stmt| render_direct_routine_stmt(stmt, indent_level))
        .collect()
}

fn render_cleanup_action_stmt(action: &NativeCleanupAction, indent_level: usize) -> String {
    match action {
        NativeCleanupAction::Direct { value } => format!(
            concat!(
                "{indent}match {} {{\n",
                "{arm_indent}Ok(RuntimeAbiValue::Unit) => {{}}\n",
                "{arm_indent}Ok(other) => return Err(format!(\"cleanup footer expected Unit, got `{{:?}}`\", other)),\n",
                "{arm_indent}Err(err) => return Err(err),\n",
                "{indent}}};\n"
            ),
            render_lowered_runtime_abi_expr(value, indent_level + 1),
            indent = indent(indent_level),
            arm_indent = indent(indent_level + 1),
        ),
        NativeCleanupAction::RuntimeDispatch { routine_key, arg } => format!(
            concat!(
                "{indent}{{\n",
                "{bind_indent}let cleanup_arg = match {} {{\n",
                "{arg_indent}Ok(value) => value,\n",
                "{arg_indent}Err(err) => return Err(err),\n",
                "{bind_indent}}};\n",
                "{bind_indent}if let Err(err) = arcana_runtime::execute_cleanup_runtime_abi_routine(plan, {:?}, cleanup_arg, state, host) {{\n",
                "{arg_indent}return Err(err);\n",
                "{bind_indent}}}\n",
                "{indent}}}\n"
            ),
            render_lowered_runtime_abi_expr(arg, indent_level + 2),
            routine_key,
            indent = indent(indent_level),
            bind_indent = indent(indent_level + 1),
            arg_indent = indent(indent_level + 2),
        ),
    }
}

fn render_direct_routine_call_from_values(
    routine_key: &str,
    params: &[NativeAbiParam],
    values: &[String],
    plan_expr: Option<&str>,
    state_expr: Option<&str>,
    host_expr: Option<&str>,
) -> String {
    let rendered = params
        .iter()
        .zip(values)
        .map(|(param, value)| match param.pass_mode {
            ArcanaCabiPassMode::In => value.clone(),
            ArcanaCabiPassMode::InWithWriteBack => format!("&mut {value}"),
        })
        .collect::<Vec<_>>();
    let mut args = Vec::new();
    if let Some(plan_expr) = plan_expr {
        args.push(plan_expr.to_string());
    }
    if let Some(state_expr) = state_expr {
        args.push(state_expr.to_string());
    }
    if let Some(host_expr) = host_expr {
        args.push(host_expr.to_string());
    }
    args.extend(rendered);
    format!(
        "{}({})",
        direct_routine_fn_name(routine_key),
        args.join(", ")
    )
}

fn render_direct_routine_call_expr(
    routine_key: &str,
    params: &[NativeAbiParam],
    args: &[NativeDirectExpr],
    indent_level: usize,
) -> String {
    render_direct_routine_call_match_chain(
        routine_key,
        params,
        args,
        0,
        &mut Vec::new(),
        indent_level,
    )
}

fn render_direct_routine_call_match_chain(
    routine_key: &str,
    params: &[NativeAbiParam],
    args: &[NativeDirectExpr],
    index: usize,
    bound_args: &mut Vec<String>,
    indent_level: usize,
) -> String {
    if index == args.len() {
        return render_direct_routine_call_from_values(
            routine_key,
            params,
            bound_args,
            Some("plan"),
            Some("state"),
            Some("host"),
        );
    }
    if matches!(params[index].pass_mode, ArcanaCabiPassMode::InWithWriteBack) {
        let NativeDirectExpr::Binding(name) = &args[index] else {
            return "Err(\"direct edit call expected binding argument\".to_string())".to_string();
        };
        bound_args.push(format!("{name}_value"));
        let next = render_direct_routine_call_match_chain(
            routine_key,
            params,
            args,
            index + 1,
            bound_args,
            indent_level,
        );
        bound_args.pop();
        return next;
    }
    let binding = format!("arg_{index}");
    bound_args.push(binding.clone());
    let next = render_direct_routine_call_match_chain(
        routine_key,
        params,
        args,
        index + 1,
        bound_args,
        indent_level + 1,
    );
    bound_args.pop();
    format!(
        concat!(
            "{{\n",
            "{match_indent}match {} {{\n",
            "{arm_indent}Ok({}) => {},\n",
            "{arm_indent}Err(err) => Err(err),\n",
            "{match_indent}}}\n",
            "{indent}}}"
        ),
        render_lowered_runtime_abi_expr(&args[index], indent_level + 2),
        binding,
        next,
        indent = indent(indent_level),
        match_indent = indent(indent_level + 1),
        arm_indent = indent(indent_level + 2),
    )
}

fn render_direct_int_binary_expr(
    op: NativeDirectIntBinaryOp,
    left: &NativeDirectExpr,
    right: &NativeDirectExpr,
    indent_level: usize,
) -> String {
    let op_text = match op {
        NativeDirectIntBinaryOp::Add => "+",
        NativeDirectIntBinaryOp::Sub => "-",
        NativeDirectIntBinaryOp::Mul => "*",
        NativeDirectIntBinaryOp::Div => "/",
        NativeDirectIntBinaryOp::Mod => "%",
    };
    render_direct_int_expr(
        left,
        right,
        indent_level,
        format!("Ok(RuntimeAbiValue::Int(left {op_text} right))"),
        "direct Int op expected Int lhs",
        "direct Int op expected Int rhs",
    )
}

fn render_direct_int_compare_expr(
    op: NativeDirectIntCompareOp,
    left: &NativeDirectExpr,
    right: &NativeDirectExpr,
    indent_level: usize,
) -> String {
    let op_text = match op {
        NativeDirectIntCompareOp::Eq => "==",
        NativeDirectIntCompareOp::NotEq => "!=",
        NativeDirectIntCompareOp::Lt => "<",
        NativeDirectIntCompareOp::LtEq => "<=",
        NativeDirectIntCompareOp::Gt => ">",
        NativeDirectIntCompareOp::GtEq => ">=",
    };
    render_direct_int_expr(
        left,
        right,
        indent_level,
        format!("Ok(RuntimeAbiValue::Bool(left {op_text} right))"),
        "direct Int compare expected Int lhs",
        "direct Int compare expected Int rhs",
    )
}

fn render_direct_int_expr(
    left: &NativeDirectExpr,
    right: &NativeDirectExpr,
    indent_level: usize,
    success_expr: String,
    lhs_message: &str,
    rhs_message: &str,
) -> String {
    format!(
        concat!(
            "{{\n",
            "{match_indent}match {} {{\n",
            "{arm_indent}Ok(RuntimeAbiValue::Int(left)) => match {} {{\n",
            "{deep_indent}Ok(RuntimeAbiValue::Int(right)) => {},\n",
            "{deep_indent}Ok(_) => Err({rhs_message:?}.to_string()),\n",
            "{deep_indent}Err(err) => Err(err),\n",
            "{arm_indent}}},\n",
            "{arm_indent}Ok(_) => Err({lhs_message:?}.to_string()),\n",
            "{arm_indent}Err(err) => Err(err),\n",
            "{match_indent}}}\n",
            "{indent}}}"
        ),
        render_lowered_runtime_abi_expr(left, indent_level + 2),
        render_lowered_runtime_abi_expr(right, indent_level + 3),
        success_expr,
        indent = indent(indent_level),
        match_indent = indent(indent_level + 1),
        arm_indent = indent(indent_level + 2),
        deep_indent = indent(indent_level + 3),
        lhs_message = lhs_message,
        rhs_message = rhs_message,
    )
}

fn render_store_runtime_abi_value(
    ty: &NativeAbiType,
    value_expr: &str,
    target_expr: &str,
    layout: &NativeLayoutCatalog,
) -> String {
    match ty {
        NativeAbiType::Int => format!(
            concat!(
                "    let RuntimeAbiValue::Int(value) = {} else {{ set_last_error(\"abi return type mismatch\".to_string()); return 0; }};\n",
                "    {} = value;\n"
            ),
            value_expr, target_expr
        ),
        NativeAbiType::Bool => format!(
            concat!(
                "    let RuntimeAbiValue::Bool(value) = {} else {{ set_last_error(\"abi return type mismatch\".to_string()); return 0; }};\n",
                "    {} = if value {{ 1 }} else {{ 0 }};\n"
            ),
            value_expr, target_expr
        ),
        NativeAbiType::Str => format!(
            concat!(
                "    let RuntimeAbiValue::Str(value) = {} else {{ set_last_error(\"abi return type mismatch\".to_string()); return 0; }};\n",
                "    {} = owned_str_from_string(value);\n"
            ),
            value_expr, target_expr
        ),
        NativeAbiType::Bytes => format!(
            concat!(
                "    let RuntimeAbiValue::Bytes(value) = {} else {{ set_last_error(\"abi return type mismatch\".to_string()); return 0; }};\n",
                "    {} = owned_bytes_from_vec(value);\n"
            ),
            value_expr, target_expr
        ),
        NativeAbiType::Pair(left, right) => {
            let pair_ty = layout.rust_type_ref(ty, NativeAbiRole::Return);
            format!(
                concat!(
                    "    let RuntimeAbiValue::Pair(left, right) = {} else {{ set_last_error(\"abi return type mismatch\".to_string()); return 0; }};\n",
                    "    let mut pair_value: {} = Default::default();\n",
                    "{}",
                    "{}",
                    "    {} = pair_value;\n"
                ),
                value_expr,
                pair_ty,
                render_store_runtime_abi_value(left, "*left", "pair_value.left", layout),
                render_store_runtime_abi_value(right, "*right", "pair_value.right", layout),
                target_expr
            )
        }
        NativeAbiType::Unit => {
            format!(
                "    let RuntimeAbiValue::Unit = {} else {{ set_last_error(\"abi return type mismatch\".to_string()); return 0; }};\n",
                value_expr
            )
        }
    }
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root should exist")
        .to_path_buf()
}

fn sanitize_crate_name(name: &str) -> String {
    let mut out = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    if out.is_empty() {
        out.push_str("arcana_native");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn direct_routine_fn_name(routine_key: &str) -> String {
    format!("arcana_direct_{}", sanitize_crate_name(routine_key))
}

fn native_output_stem(output_name: &str) -> String {
    Path::new(output_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("arcana_output")
        .to_string()
}

fn escape_toml(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
